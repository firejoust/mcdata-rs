use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use log;

// Module declarations
mod error;
mod structs;
mod version;
mod paths;
mod loader;
mod indexer;
mod features;
mod cached_data;
mod data_source; // <-- Added

// Public exports
pub use error::{McDataError, Edition};
pub use version::Version;
pub use cached_data::IndexedData;
pub use structs::*; // Re-export common data structs

// --- Global Cache for IndexedData ---
// Key: String representation of the canonical Version (e.g., "pc_1.18.2")
// Value: Arc<IndexedData> to allow shared ownership
static DATA_CACHE: Lazy<RwLock<HashMap<String, Arc<IndexedData>>>> = Lazy::new(Default::default);

/// The main entry point to get Minecraft data for a specific version.
///
/// Accepts version strings like "1.18.2", "pc_1.16.5", "bedrock_1.17.10", "1.19".
/// Handles caching of loaded data automatically.
/// On first use (or if data is missing), it may download the required data files
/// from the internet and store them in a local cache directory.
///
/// # Errors
///
/// Returns `McDataError` if:
/// *   The version string is invalid or cannot be resolved.
/// *   Network errors occur during the initial data download.
/// *   Filesystem errors occur while accessing or writing to the cache.
/// *   Data files are missing or corrupt (e.g., JSON parsing errors).
pub fn mc_data(version_str: &str) -> Result<Arc<IndexedData>, McDataError> {
    // 1. Resolve version string to canonical Version struct
    // This step itself might trigger data download if version/feature info isn't cached yet
    let version = version::resolve_version(version_str)?;
    let cache_key = format!("{}_{}", version.edition.path_prefix(), version.minecraft_version);
    log::debug!("Requesting data for resolved version key: {}", cache_key);

    // 2. Check cache (read lock)
    {
        let cache = DATA_CACHE.read().map_err(|_| McDataError::Internal("Data cache read lock poisoned".to_string()))?;
        if let Some(data) = cache.get(&cache_key) {
            log::info!("Cache hit for version: {}", cache_key);
            return Ok(data.clone());
        }
    } // Read lock released

    // 3. Load data (cache miss) - This might involve I/O and parsing
    log::info!("Cache miss for version: {}. Loading...", cache_key);
    // The `IndexedData::load` function handles the actual loading of all necessary files
    // It uses functions that rely on `data_source::get_data_root()`, triggering download if needed.
    let loaded_data_result = IndexedData::load(version); // Load outside the write lock

    // Handle potential errors during loading *before* acquiring write lock
    let loaded_data = match loaded_data_result {
        Ok(data) => Arc::new(data),
        Err(e) => {
            log::error!("Failed to load data for {}: {}", cache_key, e);
            return Err(e); // Propagate the loading error
        }
    };

    // 4. Acquire write lock and insert (double-check)
    {
        let mut cache = DATA_CACHE.write().map_err(|_| McDataError::Internal("Data cache write lock poisoned".to_string()))?;
        // Check again in case another thread loaded it while we were loading
        if let Some(data) = cache.get(&cache_key) {
             log::info!("Cache hit after load race for version: {}", cache_key);
             return Ok(data.clone()); // Return the data loaded by the other thread
        }
        log::info!("Inserting loaded data into cache for version: {}", cache_key);
        cache.insert(cache_key.clone(), loaded_data.clone());
    } // Write lock released

    Ok(loaded_data)
}

/// Returns a list of supported Minecraft versions for the given edition,
/// sorted oldest to newest based on available data.
///
/// This may trigger data download on first call if version information isn't cached.
///
/// # Errors
/// Returns `McDataError` if version information cannot be loaded (e.g., download failure).
pub fn supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    version::get_supported_versions(edition)
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper to initialize logging for tests
    fn setup() {
        // Run `RUST_LOG=debug cargo test -- --nocapture` to see logs
         let _ = env_logger::builder().is_test(true).try_init();
         // Optionally clear cache before tests? Be careful with parallel tests.
         // clear_test_cache();
    }

    // Helper to find the cache directory used by the tests
    fn get_test_cache_dir() -> Option<PathBuf> {
        dirs_next::cache_dir()
            .map(|p| p.join("mcdata-rs").join("minecraft-data"))
    }

    // Example function to clear cache (Use with caution, especially in parallel tests)
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
        let stone = data.blocks_by_name.get("stone").expect("Stone block not found");
        assert_eq!(stone.id, 1); // In 1.18+, block IDs are less relevant, but stone is usually 1
        assert!(data.items_by_name.contains_key("stick"), "Stick item not found by name");
        assert!(!data.biomes_array.is_empty(), "Biomes empty");
        assert!(!data.entities_array.is_empty(), "Entities empty");
        assert!(data.block_collision_shapes_raw.is_some(), "Collision shapes missing");
        assert!(!data.block_shapes_by_name.is_empty(), "Indexed shapes empty");
    }

     #[test]
     fn load_pc_major_version() {
         setup();
         // Should resolve to the latest release within 1.19 (e.g., 1.19.4)
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
         let data_1_20 = mc_data("1.20.1").unwrap(); // Use a known newer version

         assert!(data_1_18.is_newer_or_equal_to("1.16.5").unwrap());
         assert!(data_1_18.is_newer_or_equal_to("1.18.2").unwrap());
         assert!(!data_1_18.is_newer_or_equal_to("1.20.1").unwrap());
         assert!(data_1_20.is_newer_or_equal_to("1.18.2").unwrap());

         assert!(data_1_16.is_older_than("1.18.2").unwrap());
         assert!(!data_1_16.is_older_than("1.16.5").unwrap());
         assert!(!data_1_16.is_older_than("1.15.2").unwrap()); // Assuming 1.15.2 data exists
         assert!(data_1_18.is_older_than("1.20.1").unwrap());
    }

     #[test]
     fn test_feature_support() {
         setup();
         let data_1_18 = mc_data("1.18.2").unwrap();
         let data_1_15 = mc_data("1.15.2").unwrap();

         // Example feature: 'dimensionIsAnInt' was true up to 1.15.2
         let dim_int_115 = data_1_15.support_feature("dimensionIsAnInt").unwrap();
         assert_eq!(dim_int_115, serde_json::Value::Bool(true));

         let dim_int_118 = data_1_18.support_feature("dimensionIsAnInt").unwrap();
         assert_eq!(dim_int_118, serde_json::Value::Bool(false));

         // Example feature with value: 'metadataIxOfItem'
         let meta_ix_118 = data_1_18.support_feature("metadataIxOfItem").unwrap();
         // Value depends on the exact features.json, check node-minecraft-data if this fails
         assert_eq!(meta_ix_118, serde_json::Value::Number(8.into()));

         let meta_ix_115 = data_1_15.support_feature("metadataIxOfItem").unwrap();
         assert_eq!(meta_ix_115, serde_json::Value::Number(7.into()));
     }

     #[test]
     fn test_cache() {
         setup();
         let version = "1.17.1"; // Use a version likely not loaded by other tests
         log::info!("CACHE TEST: Loading {} for the first time", version);
         let data1 = mc_data(version).expect("Load 1 failed");
         log::info!("CACHE TEST: Loading {} for the second time", version);
         let data2 = mc_data(version).expect("Load 2 failed");
         // Check if they point to the same Arc allocation (cache hit)
         assert!(Arc::ptr_eq(&data1, &data2), "Cache miss: Arcs point to different data for {}", version);

         // Also test with prefix resolves to the same cache entry
         let prefixed_version = format!("pc_{}", version);
         log::info!("CACHE TEST: Loading {} for the third time", prefixed_version);
         let data3 = mc_data(&prefixed_version).expect("Load 3 failed");
         assert!(Arc::ptr_eq(&data1, &data3), "Cache miss: Prefixed version {} loaded different data", prefixed_version);
     }

     #[test]
     fn test_supported_versions() {
         setup();
         let versions = supported_versions(Edition::Pc).expect("Failed to get supported PC versions");
         assert!(!versions.is_empty());
         // Check if some expected versions are present
         assert!(versions.iter().any(|v| v == "1.8.8"));
         assert!(versions.iter().any(|v| v == "1.16.5"));
         assert!(versions.iter().any(|v| v == "1.18.2"));
         assert!(versions.iter().any(|v| v == "1.20.1")); // Check a more recent one

         // Check sorting (basic check: 1.8.8 should come before 1.16.5)
         let index_1_8 = versions.iter().position(|v| v == "1.8.8");
         let index_1_16 = versions.iter().position(|v| v == "1.16.5");
         assert!(index_1_8.is_some());
         assert!(index_1_16.is_some());
         assert!(index_1_8 < index_1_16, "Versions should be sorted oldest to newest");
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

     // Add tests for Bedrock when data is available and confirmed working
     // #[test]
     // fn load_bedrock_version() {
     //     setup();
     //     // Find a known Bedrock version from protocolVersions.json
     //     let version = "bedrock_1.18.30"; // Example, check data for a valid one
     //     let data = mc_data(version).expect("Failed to load Bedrock data");
     //     assert_eq!(data.version.edition, Edition::Bedrock);
     //     assert!(data.version.minecraft_version.contains("1.18.30")); // Or exact match depending on resolution
     //     assert!(!data.blocks_array.is_empty());
     //     assert!(!data.items_array.is_empty());
     // }
}