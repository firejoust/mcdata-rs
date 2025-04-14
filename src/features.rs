use crate::error::{McDataError, Edition};
use crate::structs::Feature;
use crate::version::{self, Version};
use crate::loader::load_data_from_path;
use crate::constants::MINECRAFT_DATA_SUBMODULE_PATH;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use serde_json::Value;

static PC_FEATURES: Lazy<Result<Arc<Vec<Feature>>, McDataError>> = Lazy::new(|| {
    load_features(Edition::Pc)
});

static BEDROCK_FEATURES: Lazy<Result<Arc<Vec<Feature>>, McDataError>> = Lazy::new(|| {
    load_features(Edition::Bedrock)
});

fn load_features(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
    let path_str = format!("{}/data/{}/common/features.json", MINECRAFT_DATA_SUBMODULE_PATH, edition.path_prefix());
    let path = Path::new(&path_str);
    load_data_from_path(path).map(Arc::new)
}

fn get_features(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
  let lazy_ref = match edition {
      Edition::Pc => &PC_FEATURES,
      Edition::Bedrock => &BEDROCK_FEATURES,
  };
  // Dereference Lazy, handle Result, clone Arc if Ok, create new error if Err
  match **lazy_ref {
      Ok(ref arc_data) => Ok(arc_data.clone()),
      Err(ref original_error) => Err(McDataError::Internal(format!(
          "Failed to load features.json during static initialization: {}",
          original_error
      ))),
  }
}

// Cache only successfully resolved versions
static RESOLVED_VERSION_CACHE: Lazy<std::sync::RwLock<HashMap<(Edition, String), Version>>> =
    Lazy::new(Default::default);

// Keep the existing resolve_cached_version for non-_major strings
fn resolve_cached_version(edition: Edition, version_str: &str) -> Result<Version, McDataError> {
    // (Implementation remains the same as your last version)
    let cache_key = (edition, version_str.to_string());
    // Check read cache
    {
        let cache = RESOLVED_VERSION_CACHE.read().unwrap();
        if let Some(cached_version) = cache.get(&cache_key) {
            return Ok(cached_version.clone()); // Version is Clone
        }
    } // Release read lock

    // Resolve
    let full_version_str = if version_str.starts_with("pc_") || version_str.starts_with("bedrock_") {
        version_str.to_string()
    } else {
        format!("{}_{}", edition.path_prefix(), version_str)
    };
    let resolved_result = crate::version::resolve_version(&full_version_str);

    // If successful, acquire write lock and insert
    if let Ok(ref version) = resolved_result {
        let mut cache = RESOLVED_VERSION_CACHE.write().unwrap();
        // Insert the successfully resolved Version (clone it)
        // Use entry API for efficiency and to avoid double-checking
        cache.entry(cache_key).or_insert_with(|| version.clone());
    } // Release write lock implicitly

    resolved_result // Return the original result (Ok or Err)
}


// --- Updated is_version_in_range ---

fn is_version_in_range(target_version: &Version, min_ver_str: &str, max_ver_str: &str) -> Result<bool, McDataError> {
    let edition = target_version.edition;

    // --- Handle min_ver_str ---
    let min_ver = if let Some(base_major) = min_ver_str.strip_suffix("_major") {
        // Handle _major suffix: Find the OLDEST version in that major range
        let version_data = version::get_version_data(edition)?; // Get protocol versions data
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.last()) // Get the last element (oldest)
            .cloned() // Clone the Version struct
            .ok_or_else(|| McDataError::InvalidVersion(format!("{}_{}", edition.path_prefix(), min_ver_str)))?
    } else {
        // Resolve normally
        resolve_cached_version(edition, min_ver_str)?
    };

    // --- Handle max_ver_str ---
    let max_ver = if max_ver_str == "latest" {
        // Handle "latest": Find the absolute newest version for the edition
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_minecraft_version // Use this map as it contains all versions
            .values()
            .max() // Find the version with the highest data_version
            .cloned()
            .ok_or_else(|| McDataError::Internal("Could not determine latest version".to_string()))?
    } else if let Some(base_major) = max_ver_str.strip_suffix("_major") {
        // Handle _major suffix: Find the NEWEST version in that major range
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.first()) // Get the first element (newest)
            .cloned()
            .ok_or_else(|| McDataError::InvalidVersion(format!("{}_{}", edition.path_prefix(), max_ver_str)))?
    } else {
        // Resolve normally
        resolve_cached_version(edition, max_ver_str)?
    };

    // --- Perform Comparison ---
    Ok(target_version >= &min_ver && target_version <= &max_ver)
}


// --- get_feature_support remains the same ---
pub fn get_feature_support(target_version: &Version, feature_name: &str) -> Result<Value, McDataError> {
    // (Implementation remains the same as your last version)
    let features = get_features(target_version.edition)?;

    // Find the feature by name (last one wins in case of duplicates, like node-minecraft-data)
    if let Some(feature) = features.iter().rev().find(|f| f.name == feature_name) {
        if !feature.values.is_empty() {
            // Check values array (last matching range wins)
            for fv in feature.values.iter().rev() {
                let in_range = if let Some(v_str) = &fv.version {
                    is_version_in_range(target_version, v_str, v_str)?
                } else if fv.versions.len() == 2 {
                    is_version_in_range(target_version, &fv.versions[0], &fv.versions[1])?
                } else {
                    false // Invalid range definition
                };
                if in_range {
                    return Ok(fv.value.clone());
                }
            }
        } else if let Some(v_str) = &feature.version {
            // Check single version string
            if is_version_in_range(target_version, v_str, v_str)? {
                return Ok(Value::Bool(true));
            }
        } else if feature.versions.len() == 2 {
            // Check versions array [min, max]
             if is_version_in_range(target_version, &feature.versions[0], &feature.versions[1])? {
                 return Ok(Value::Bool(true));
             }
        }
    }

    // Default to false if feature not found or no range matched
    Ok(Value::Bool(false))
}