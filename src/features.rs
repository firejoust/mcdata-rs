use crate::error::{McDataError, Edition};
use crate::structs::Feature;
use crate::version::{self, Version};
use crate::loader::load_data_from_path;
use crate::data_source; // Use data_source
use once_cell::sync::{Lazy, OnceCell}; // Use OnceCell
use std::collections::HashMap;
use std::sync::{Arc, RwLock}; // Keep RwLock for version cache
use serde_json::Value;

// Cache loaded features per edition
static LOADED_FEATURES: OnceCell<HashMap<Edition, Result<Arc<Vec<Feature>>, McDataError>>> = OnceCell::new();

fn load_features_for_edition(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
    log::debug!("Attempting to load features.json for {:?}...", edition);
    let data_root = data_source::get_data_root()?;
    let path_str = format!("{}/common/features.json", edition.path_prefix());
    let path = data_root.join(path_str);
    load_data_from_path(&path).map(Arc::new)
}

fn get_features(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
    let cache = LOADED_FEATURES.get_or_init(|| {
        log::debug!("Initializing features cache map");
        let mut map = HashMap::new();
        map.insert(Edition::Pc, load_features_for_edition(Edition::Pc));
        map.insert(Edition::Bedrock, load_features_for_edition(Edition::Bedrock));
        map
    });

    // Get the result for the requested edition
    match cache.get(&edition) {
        Some(Ok(arc_data)) => Ok(arc_data.clone()),
        Some(Err(original_error)) => Err(McDataError::Internal(format!(
            "Failed to load features.json for {:?} during initialization: {}",
            edition, original_error
        ))),
        None => Err(McDataError::Internal(format!( // Should not happen
            "Features data for edition {:?} unexpectedly missing from cache.", edition
        ))),
    }
}

// Cache only successfully resolved versions
static RESOLVED_VERSION_CACHE: Lazy<RwLock<HashMap<(Edition, String), Version>>> =
    Lazy::new(Default::default);

// Resolve version string to Version struct, using cache
fn resolve_cached_version(edition: Edition, version_str: &str) -> Result<Version, McDataError> {
    let cache_key = (edition, version_str.to_string());
    // Check read cache
    {
        let cache = RESOLVED_VERSION_CACHE.read().map_err(|_| McDataError::Internal("Version cache read lock poisoned".to_string()))?;
        if let Some(cached_version) = cache.get(&cache_key) {
            log::trace!("Cache hit for resolved version: {:?}", cache_key);
            return Ok(cached_version.clone()); // Version is Clone
        }
    } // Release read lock

    // Resolve (using the main version resolution logic)
    // We need the full string potentially (e.g. "pc_1.18.2" or just "1.18.2")
    // Let the main resolve_version handle prefix logic.
    let resolved_result = crate::version::resolve_version(version_str);

    // If successful, acquire write lock and insert
    if let Ok(ref version) = resolved_result {
         // Ensure the resolved version matches the edition we expected
         if version.edition == edition {
            let mut cache = RESOLVED_VERSION_CACHE.write().map_err(|_| McDataError::Internal("Version cache write lock poisoned".to_string()))?;
            // Insert the successfully resolved Version (clone it)
            // Use entry API for efficiency and to avoid double-checking
            log::trace!("Cache miss, inserting resolved version: {:?}", cache_key);
            cache.entry(cache_key).or_insert_with(|| version.clone());
         } else {
              log::warn!("Resolved version {} has edition {:?}, but expected {:?}", version_str, version.edition, edition);
              // Return error or the resolved version anyway? Let's return error for consistency.
              return Err(McDataError::Internal(format!("Resolved version {} edition mismatch (got {:?}, expected {:?})", version_str, version.edition, edition)));
         }
    } // Release write lock implicitly

    resolved_result // Return the original result (Ok or Err)
}


// --- Updated is_version_in_range ---

fn is_version_in_range(target_version: &Version, min_ver_str: &str, max_ver_str: &str) -> Result<bool, McDataError> {
    let edition = target_version.edition;
    log::trace!("Checking if {:?} {} is in range [{}, {}]", edition, target_version.minecraft_version, min_ver_str, max_ver_str);

    // --- Handle min_ver_str ---
    let min_ver = if let Some(base_major) = min_ver_str.strip_suffix("_major") {
        // Handle _major suffix: Find the OLDEST version in that major range
        log::trace!("Resolving min_ver {}_major", base_major);
        let version_data = version::get_version_data(edition)?; // Get protocol versions data
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.last()) // Get the last element (oldest)
            .cloned() // Clone the Version struct
            .ok_or_else(|| McDataError::InvalidVersion(format!("Could not find oldest version for major '{}_{}'", edition.path_prefix(), base_major)))?
    } else {
        // Resolve normally using cache
        log::trace!("Resolving min_ver {}", min_ver_str);
        resolve_cached_version(edition, min_ver_str)?
    };

    // --- Handle max_ver_str ---
    let max_ver = if max_ver_str == "latest" {
        // Handle "latest": Find the absolute newest version for the edition
        log::trace!("Resolving max_ver 'latest'");
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_minecraft_version // Use this map as it contains all versions
            .values()
            .max() // Find the version with the highest data_version
            .cloned()
            .ok_or_else(|| McDataError::Internal(format!("Could not determine latest version for {:?}", edition)))?
    } else if let Some(base_major) = max_ver_str.strip_suffix("_major") {
        // Handle _major suffix: Find the NEWEST version in that major range
        log::trace!("Resolving max_ver {}_major", base_major);
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.first()) // Get the first element (newest)
            .cloned()
            .ok_or_else(|| McDataError::InvalidVersion(format!("Could not find newest version for major '{}_{}'", edition.path_prefix(), base_major)))?
    } else {
        // Resolve normally using cache
        log::trace!("Resolving max_ver {}", max_ver_str);
        resolve_cached_version(edition, max_ver_str)?
    };

    // --- Perform Comparison ---
    let result = target_version >= &min_ver && target_version <= &max_ver;
    log::trace!("Range check: {} >= {} && {} <= {} -> {}", target_version.data_version, min_ver.data_version, target_version.data_version, max_ver.data_version, result);
    Ok(result)
}


// --- get_feature_support remains the same ---
pub fn get_feature_support(target_version: &Version, feature_name: &str) -> Result<Value, McDataError> {
    log::debug!("Checking feature support for '{}' in version {}", feature_name, target_version.minecraft_version);
    let features = get_features(target_version.edition)?;

    // Find the feature by name (last one wins in case of duplicates, like node-minecraft-data)
    if let Some(feature) = features.iter().rev().find(|f| f.name == feature_name) {
        log::trace!("Found feature entry: {:?}", feature);
        if !feature.values.is_empty() {
            // Check values array (last matching range wins)
            log::trace!("Checking feature.values array ({} entries)", feature.values.len());
            for fv in feature.values.iter().rev() {
                let in_range = if let Some(v_str) = &fv.version {
                    is_version_in_range(target_version, v_str, v_str)?
                } else if fv.versions.len() == 2 {
                    is_version_in_range(target_version, &fv.versions[0], &fv.versions[1])?
                } else {
                    log::warn!("Invalid version range definition in feature '{}' value: {:?}", feature_name, fv);
                    false // Invalid range definition
                };
                if in_range {
                    log::debug!("Feature '{}' supported via values array, value: {}", feature_name, fv.value);
                    return Ok(fv.value.clone());
                }
            }
            log::trace!("No matching range found in feature.values");
        } else if let Some(v_str) = &feature.version {
            // Check single version string
            log::trace!("Checking feature.version string: {}", v_str);
            if is_version_in_range(target_version, v_str, v_str)? {
                 log::debug!("Feature '{}' supported via version string (implicit true)", feature_name);
                 return Ok(Value::Bool(true));
            }
        } else if feature.versions.len() == 2 {
            // Check versions array [min, max]
            log::trace!("Checking feature.versions array: [{}, {}]", feature.versions[0], feature.versions[1]);
             if is_version_in_range(target_version, &feature.versions[0], &feature.versions[1])? {
                 log::debug!("Feature '{}' supported via versions array (implicit true)", feature_name);
                 return Ok(Value::Bool(true));
             }
        } else {
             log::trace!("Feature '{}' found but has no version/versions/values definition, assuming false", feature_name);
        }
    } else {
         log::trace!("Feature '{}' not found in features.json", feature_name);
    }

    // Default to false if feature not found or no range matched
    log::debug!("Feature '{}' determined to be unsupported (defaulting to false)", feature_name);
    Ok(Value::Bool(false))
}