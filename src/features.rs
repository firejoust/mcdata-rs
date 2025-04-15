use crate::data_source;
use crate::error::{Edition, McDataError};
use crate::loader::load_data_from_path;
use crate::structs::Feature;
use crate::version::{self, Version};
use once_cell::sync::{Lazy, OnceCell};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Cache for loaded features.json data, keyed by edition.
// Stores the Result to cache loading errors as well.
static LOADED_FEATURES: OnceCell<HashMap<Edition, Result<Arc<Vec<Feature>>, McDataError>>> =
    OnceCell::new();

/// Loads the features.json file for a specific edition.
fn load_features_for_edition(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
    log::debug!("Attempting to load features.json for {:?}...", edition);
    let data_root = data_source::get_data_root()?;
    let path_str = format!("{}/common/features.json", edition.path_prefix());
    let path = data_root.join(path_str);
    load_data_from_path(&path).map(Arc::new)
}

/// Retrieves the cached features data for the specified edition.
/// Loads and caches the data for both editions on the first call.
fn get_features(edition: Edition) -> Result<Arc<Vec<Feature>>, McDataError> {
    let cache = LOADED_FEATURES.get_or_init(|| {
        log::debug!("Initializing features cache map");
        let mut map = HashMap::new();
        // Pre-populate both editions to avoid multiple init attempts.
        map.insert(Edition::Pc, load_features_for_edition(Edition::Pc));
        map.insert(
            Edition::Bedrock,
            load_features_for_edition(Edition::Bedrock),
        );
        map
    });

    // Retrieve the result for the requested edition from the initialized cache.
    match cache.get(&edition) {
        Some(Ok(arc_data)) => Ok(arc_data.clone()),
        Some(Err(original_error)) => Err(McDataError::Internal(format!(
            "Failed to load features.json for {:?} during initialization: {}",
            edition, original_error
        ))),
        None => Err(McDataError::Internal(format!(
            "Features data for edition {:?} unexpectedly missing from cache.",
            edition
        ))),
    }
}

// Cache for successfully resolved Version structs, keyed by (Edition, version_string).
// Uses a RwLock for concurrent read access.
static RESOLVED_VERSION_CACHE: Lazy<RwLock<HashMap<(Edition, String), Version>>> =
    Lazy::new(Default::default);

/// Resolves a version string (like "1.18.2") to a canonical `Version` struct for a given edition,
/// utilizing a cache to avoid redundant resolutions.
fn resolve_cached_version(edition: Edition, version_str: &str) -> Result<Version, McDataError> {
    let cache_key = (edition, version_str.to_string());

    // Attempt to read from the cache first.
    {
        let cache = RESOLVED_VERSION_CACHE
            .read()
            .map_err(|_| McDataError::Internal("Version cache read lock poisoned".to_string()))?;
        if let Some(cached_version) = cache.get(&cache_key) {
            log::trace!("Cache hit for resolved version: {:?}", cache_key);
            return Ok(cached_version.clone());
        }
    } // Read lock is released here.

    // If not found in cache, perform the actual version resolution.
    let resolved_result = crate::version::resolve_version(version_str);

    // If resolution was successful, cache the result.
    if let Ok(ref version) = resolved_result {
        // Ensure the resolved version's edition matches the requested edition.
        if version.edition == edition {
            let mut cache = RESOLVED_VERSION_CACHE.write().map_err(|_| {
                McDataError::Internal("Version cache write lock poisoned".to_string())
            })?;
            // Use entry API to insert only if the key is not already present (handles potential race condition).
            log::trace!("Cache miss, inserting resolved version: {:?}", cache_key);
            cache.entry(cache_key).or_insert_with(|| version.clone());
        } else {
            // This indicates an inconsistency, likely in how the version string was provided or parsed.
            log::warn!(
                "Resolved version {} has edition {:?}, but expected {:?}",
                version_str,
                version.edition,
                edition
            );
            return Err(McDataError::Internal(format!(
                "Resolved version {} edition mismatch (got {:?}, expected {:?})",
                version_str, version.edition, edition
            )));
        }
    } // Write lock is released here.

    resolved_result // Return the original resolution result (Ok or Err).
}

/// Checks if a target `Version` falls within a specified version range.
///
/// The range can use specific version strings, "latest", or "_major" suffixes.
fn is_version_in_range(
    target_version: &Version,
    min_ver_str: &str,
    max_ver_str: &str,
) -> Result<bool, McDataError> {
    let edition = target_version.edition;
    log::trace!(
        "Checking if {:?} {} is in range [{}, {}]",
        edition,
        target_version.minecraft_version,
        min_ver_str,
        max_ver_str
    );

    // Resolve the minimum version boundary.
    let min_ver = if let Some(base_major) = min_ver_str.strip_suffix("_major") {
        // Handle `_major` suffix: Find the OLDEST version within that major release series.
        log::trace!("Resolving min_ver {}_major", base_major);
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.last()) // Get the last element (oldest in the sorted list).
            .cloned()
            .ok_or_else(|| {
                McDataError::InvalidVersion(format!(
                    "Could not find oldest version for major '{}_{}'",
                    edition.path_prefix(),
                    base_major
                ))
            })?
    } else {
        // Resolve a specific version string using the cache.
        log::trace!("Resolving min_ver {}", min_ver_str);
        resolve_cached_version(edition, min_ver_str)?
    };

    // Resolve the maximum version boundary.
    let max_ver = if max_ver_str == "latest" {
        // Handle "latest": Find the absolute newest version known for the edition.
        log::trace!("Resolving max_ver 'latest'");
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_minecraft_version // Use the map containing all versions.
            .values()
            .max() // Find the version with the highest data_version (newest).
            .cloned()
            .ok_or_else(|| {
                McDataError::Internal(format!(
                    "Could not determine latest version for {:?}",
                    edition
                ))
            })?
    } else if let Some(base_major) = max_ver_str.strip_suffix("_major") {
        // Handle `_major` suffix: Find the NEWEST version within that major release series.
        log::trace!("Resolving max_ver {}_major", base_major);
        let version_data = version::get_version_data(edition)?;
        version_data
            .by_major_version
            .get(base_major)
            .and_then(|versions| versions.first()) // Get the first element (newest in the sorted list).
            .cloned()
            .ok_or_else(|| {
                McDataError::InvalidVersion(format!(
                    "Could not find newest version for major '{}_{}'",
                    edition.path_prefix(),
                    base_major
                ))
            })?
    } else {
        // Resolve a specific version string using the cache.
        log::trace!("Resolving max_ver {}", max_ver_str);
        resolve_cached_version(edition, max_ver_str)?
    };

    // Perform the comparison using the resolved Version structs (which implement Ord).
    let result = target_version >= &min_ver && target_version <= &max_ver;
    log::trace!(
        "Range check: {} >= {} && {} <= {} -> {}",
        target_version.data_version,
        min_ver.data_version,
        target_version.data_version,
        max_ver.data_version,
        result
    );
    Ok(result)
}

/// Determines the support status and value of a feature for a given target version.
///
/// It checks the `features.json` data, considering version ranges defined within it.
/// The logic prioritizes the `values` array, then `version`, then `versions`.
/// If the feature is found and a range matches, it returns the associated value.
/// If the feature is found but no range matches, or if the feature is not found,
/// it defaults to `Value::Bool(false)`.
pub fn get_feature_support(
    target_version: &Version,
    feature_name: &str,
) -> Result<Value, McDataError> {
    log::debug!(
        "Checking feature support for '{}' in version {}",
        feature_name,
        target_version.minecraft_version
    );
    let features = get_features(target_version.edition)?;

    // Find the feature entry by name. Iterating in reverse mimics node-minecraft-data's behavior
    // where later definitions override earlier ones.
    if let Some(feature) = features.iter().rev().find(|f| f.name == feature_name) {
        log::trace!("Found feature entry: {:?}", feature);

        // Priority 1: Check the 'values' array if present.
        if !feature.values.is_empty() {
            log::trace!(
                "Checking feature.values array ({} entries)",
                feature.values.len()
            );
            // Iterate in reverse to match node-minecraft-data's priority (last matching range wins).
            for fv in feature.values.iter().rev() {
                let in_range = if let Some(v_str) = &fv.version {
                    // Single version string range.
                    is_version_in_range(target_version, v_str, v_str)?
                } else if fv.versions.len() == 2 {
                    // [min, max] version array range.
                    is_version_in_range(target_version, &fv.versions[0], &fv.versions[1])?
                } else {
                    log::warn!(
                        "Invalid version range definition in feature '{}' value: {:?}",
                        feature_name,
                        fv
                    );
                    false // Treat invalid range definitions as non-matching.
                };
                if in_range {
                    log::debug!(
                        "Feature '{}' supported via values array, value: {}",
                        feature_name,
                        fv.value
                    );
                    return Ok(fv.value.clone());
                }
            }
            log::trace!("No matching range found in feature.values");
        }
        // Priority 2: Check the single 'version' string if 'values' was empty or didn't match.
        else if let Some(v_str) = &feature.version {
            log::trace!("Checking feature.version string: {}", v_str);
            if is_version_in_range(target_version, v_str, v_str)? {
                log::debug!(
                    "Feature '{}' supported via version string (implicit true)",
                    feature_name
                );
                return Ok(Value::Bool(true)); // Implicitly true if range matches.
            }
        }
        // Priority 3: Check the 'versions' array [min, max] if others were absent or didn't match.
        else if feature.versions.len() == 2 {
            log::trace!(
                "Checking feature.versions array: [{}, {}]",
                feature.versions[0],
                feature.versions[1]
            );
            if is_version_in_range(target_version, &feature.versions[0], &feature.versions[1])? {
                log::debug!(
                    "Feature '{}' supported via versions array (implicit true)",
                    feature_name
                );
                return Ok(Value::Bool(true)); // Implicitly true if range matches.
            }
        } else {
            // Feature entry exists but has no valid version definition.
            log::trace!(
                "Feature '{}' found but has no version/versions/values definition, assuming false",
                feature_name
            );
        }
    } else {
        // Feature name was not found in the loaded features data.
        log::trace!("Feature '{}' not found in features.json", feature_name);
    }

    // Default to false if the feature wasn't found or no applicable version range matched.
    log::debug!(
        "Feature '{}' determined to be unsupported (defaulting to false)",
        feature_name
    );
    Ok(Value::Bool(false))
}
