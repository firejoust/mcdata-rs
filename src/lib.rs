use log;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Module definitions
mod cached_data;
mod data_source;
mod error;
mod features;
mod indexer;
mod loader;
mod paths;
mod structs;
mod version;

// Public API exports
pub use cached_data::IndexedData;
pub use error::{Edition, McDataError};
pub use structs::*;
pub use version::Version; // Re-export all data structs

// Global cache for loaded and indexed data, keyed by canonical version string (e.g., "pc_1.18.2").
// Uses a RwLock to allow concurrent reads while ensuring safe writes.
static DATA_CACHE: Lazy<RwLock<HashMap<String, Arc<IndexedData>>>> = Lazy::new(Default::default);

/// The main entry point to get Minecraft data for a specific version.
///
/// Accepts version strings like "1.18.2", "pc_1.16.5", "bedrock_1.17.10", "1.19".
/// Handles caching of loaded data automatically.
/// On first use for a specific version (or if data is missing from the local cache),
/// it may download the required data files from the PrismarineJS/minecraft-data repository
/// and store them in a local cache directory (typically within the system's cache location).
///
/// # Errors
///
/// Returns `McDataError` if:
/// *   The version string is invalid or cannot be resolved to a known Minecraft version.
/// *   Network errors occur during the initial data download.
/// *   Filesystem errors occur while accessing or writing to the cache directory.
/// *   Required data files are missing or corrupt (e.g., JSON parsing errors).
/// *   Internal errors occur (e.g., cache lock poisoning).
pub fn mc_data(version_str: &str) -> Result<Arc<IndexedData>, McDataError> {
    // 1. Resolve the input version string to a canonical `Version` struct.
    // This step might trigger initial download/loading of version metadata if not already cached.
    let version = version::resolve_version(version_str)?;
    let cache_key = format!(
        "{}_{}",
        version.edition.path_prefix(),
        version.minecraft_version
    );
    log::debug!("Requesting data for resolved version key: {}", cache_key);

    // 2. Check the cache for existing data using a read lock.
    {
        let cache = DATA_CACHE
            .read()
            .map_err(|_| McDataError::Internal("Data cache read lock poisoned".to_string()))?;
        if let Some(data) = cache.get(&cache_key) {
            log::info!("Cache hit for version: {}", cache_key);
            return Ok(data.clone()); // Return the cached Arc.
        }
    } // Read lock is released here.

    // 3. Cache miss: Load and index the data for this version.
    // This involves reading files, parsing JSON, and building index HashMaps.
    // This operation happens outside the write lock to avoid holding it during potentially long I/O.
    log::info!("Cache miss for version: {}. Loading...", cache_key);
    let loaded_data_result = IndexedData::load(version); // This function handles loading all necessary files.

    // Handle potential errors during the loading process before attempting to cache.
    let loaded_data = match loaded_data_result {
        Ok(data) => Arc::new(data),
        Err(e) => {
            log::error!("Failed to load data for {}: {}", cache_key, e);
            return Err(e); // Propagate the loading error.
        }
    };

    // 4. Acquire write lock to insert the newly loaded data into the cache.
    {
        let mut cache = DATA_CACHE
            .write()
            .map_err(|_| McDataError::Internal("Data cache write lock poisoned".to_string()))?;
        // Double-check: Another thread might have loaded and inserted the data
        // while this thread was performing the load operation.
        if let Some(data) = cache.get(&cache_key) {
            log::info!("Cache hit after load race for version: {}", cache_key);
            return Ok(data.clone()); // Return the data loaded by the other thread.
        }
        // Insert the data loaded by this thread.
        log::info!(
            "Inserting loaded data into cache for version: {}",
            cache_key
        );
        cache.insert(cache_key.clone(), loaded_data.clone());
    } // Write lock is released here.

    Ok(loaded_data)
}

/// Returns a list of supported Minecraft versions for the given edition,
/// sorted oldest to newest based on available data in `protocolVersions.json`.
///
/// This may trigger data download on the first call if version information isn't cached.
///
/// # Errors
/// Returns `McDataError` if version information cannot be loaded (e.g., download failure, file corruption).
pub fn supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    version::get_supported_versions(edition)
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Initializes logging for test output.
    fn setup() {
        // Use try_init to avoid panic if logger is already initialized by another test.
        let _ = env_logger::builder().is_test(true).try_init();
    }

    // Helper to get the expected cache directory path used during tests.
    fn get_test_cache_dir() -> Option<PathBuf> {
        dirs_next::cache_dir().map(|p| p.join("mcdata-rs").join("minecraft-data"))
    }

    // Utility function to clear the cache directory (use with caution, especially in parallel tests).
    #[allow(dead_code)]
    fn clear_test_cache() {
        if let Some(cache_dir) = get_test_cache_dir() {
            if cache_dir.exists() {
                log::warn!("Clearing test cache directory: {}", cache_dir.display());
                if let Err(e) = std::fs::remove_dir_all(&cache_dir) {
                    log::error!("Failed to clear test cache: {}", e);
                }
            }
        }
    }

    #[test]
    fn load_pc_1_18_2() {
        setup();
        let data = mc_data("1.18.2").expect("Failed to load 1.18.2 data");
        assert_eq!(data.version.minecraft_version, "1.18.2");
        assert_eq!(data.version.edition, Edition::Pc);
        let stone = data
            .blocks_by_name
            .get("stone")
            .expect("Stone block not found");
        assert_eq!(stone.id, 1);
        assert!(
            data.items_by_name.contains_key("stick"),
            "Stick item not found by name"
        );
        assert!(!data.biomes_array.is_empty(), "Biomes empty");
        assert!(!data.entities_array.is_empty(), "Entities empty");
        assert!(
            data.block_collision_shapes_raw.is_some(),
            "Collision shapes missing"
        );
        assert!(
            !data.block_shapes_by_name.is_empty(),
            "Indexed shapes empty"
        );
    }

    #[test]
    fn load_pc_major_version() {
        setup();
        // Should resolve to the latest release within the 1.19 major version.
        let data = mc_data("1.19").expect("Failed to load 1.19 data");
        assert!(data.version.minecraft_version.starts_with("1.19"));
        assert_eq!(data.version.edition, Edition::Pc);
        assert!(data.blocks_by_name.contains_key("mangrove_log"));
        assert!(data.entities_by_name.contains_key("warden"));
    }

    #[test]
    fn test_version_comparison() {
        setup();
        let data_1_18 = mc_data("1.18.2").unwrap();
        let data_1_16 = mc_data("1.16.5").unwrap();
        let data_1_20 = mc_data("1.20.1").unwrap(); // Assumes 1.20.1 data exists

        assert!(data_1_18.is_newer_or_equal_to("1.16.5").unwrap());
        assert!(data_1_18.is_newer_or_equal_to("1.18.2").unwrap());
        assert!(!data_1_18.is_newer_or_equal_to("1.20.1").unwrap());
        assert!(data_1_20.is_newer_or_equal_to("1.18.2").unwrap());

        assert!(data_1_16.is_older_than("1.18.2").unwrap());
        assert!(!data_1_16.is_older_than("1.16.5").unwrap());
        assert!(!data_1_16.is_older_than("1.15.2").unwrap()); // Assumes 1.15.2 data exists
        assert!(data_1_18.is_older_than("1.20.1").unwrap());
    }

    #[test]
    fn test_feature_support() {
        setup();
        let data_1_18 = mc_data("1.18.2").unwrap();
        let data_1_15 = mc_data("1.15.2").unwrap();

        // Check a boolean feature that changes across versions.
        let dim_int_115 = data_1_15.support_feature("dimensionIsAnInt").unwrap();
        assert_eq!(dim_int_115, serde_json::Value::Bool(true));

        let dim_int_118 = data_1_18.support_feature("dimensionIsAnInt").unwrap();
        assert_eq!(dim_int_118, serde_json::Value::Bool(false));

        // Check a feature with a numeric value that changes.
        let meta_ix_118 = data_1_18.support_feature("metadataIxOfItem").unwrap();
        assert_eq!(meta_ix_118, serde_json::Value::Number(8.into()));

        let meta_ix_115 = data_1_15.support_feature("metadataIxOfItem").unwrap();
        assert_eq!(meta_ix_115, serde_json::Value::Number(7.into()));
    }

    #[test]
    fn test_cache() {
        setup();
        let version = "1.17.1";
        log::info!("CACHE TEST: Loading {} for the first time", version);
        let data1 = mc_data(version).expect("Load 1 failed");
        log::info!("CACHE TEST: Loading {} for the second time", version);
        let data2 = mc_data(version).expect("Load 2 failed");
        // Check if both results point to the same Arc allocation (indicates cache hit).
        assert!(
            Arc::ptr_eq(&data1, &data2),
            "Cache miss: Arcs point to different data for {}",
            version
        );

        // Test that resolving a prefixed version hits the same cache entry.
        let prefixed_version = format!("pc_{}", version);
        log::info!(
            "CACHE TEST: Loading {} for the third time",
            prefixed_version
        );
        let data3 = mc_data(&prefixed_version).expect("Load 3 failed");
        assert!(
            Arc::ptr_eq(&data1, &data3),
            "Cache miss: Prefixed version {} loaded different data",
            prefixed_version
        );
    }

    #[test]
    fn test_supported_versions() {
        setup();
        let versions =
            supported_versions(Edition::Pc).expect("Failed to get supported PC versions");
        assert!(!versions.is_empty());
        // Check if some expected versions are present.
        assert!(versions.iter().any(|v| v == "1.8.8"));
        assert!(versions.iter().any(|v| v == "1.16.5"));
        assert!(versions.iter().any(|v| v == "1.18.2"));
        assert!(versions.iter().any(|v| v == "1.20.1"));

        // Check sorting (basic check: 1.8.8 should appear before 1.16.5).
        let index_1_8 = versions.iter().position(|v| v == "1.8.8");
        let index_1_16 = versions.iter().position(|v| v == "1.16.5");
        assert!(index_1_8.is_some());
        assert!(index_1_16.is_some());
        assert!(
            index_1_8 < index_1_16,
            "Versions should be sorted oldest to newest"
        );
    }

    #[test]
    fn test_invalid_version() {
        setup();
        let result = mc_data("invalid_version_string_1.2.3");
        assert!(result.is_err());
        match result.err().unwrap() {
            McDataError::InvalidVersion(s) => assert!(s.contains("invalid_version")),
            e => panic!("Expected InvalidVersion error, got {:?}", e),
        }
    }

    // Placeholder for Bedrock tests when data/support is confirmed.
    // #[test]
    // fn load_bedrock_version() {
    //     setup();
    //     let version = "bedrock_1.18.30"; // Use a known valid Bedrock version
    //     let data = mc_data(version).expect("Failed to load Bedrock data");
    //     assert_eq!(data.version.edition, Edition::Bedrock);
    //     assert!(data.version.minecraft_version.contains("1.18.30"));
    //     assert!(!data.blocks_array.is_empty());
    //     assert!(!data.items_array.is_empty());
    // }
}
