use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use log;

mod error;
mod structs;
mod version;
mod paths;
mod loader;
mod indexer;
mod features;
mod cached_data;
mod constants;

pub use error::{McDataError, Edition};
pub use version::Version;
pub use cached_data::IndexedData;
pub use structs::*; // Re-export common data structs

// --- Global Cache ---
// Key: String representation of the canonical Version (e.g., "pc_1.18.2")
// Value: Arc<IndexedData> to allow shared ownership
static DATA_CACHE: Lazy<RwLock<HashMap<String, Arc<IndexedData>>>> = Lazy::new(Default::default);

/// The main entry point to get Minecraft data for a specific version.
///
/// Accepts version strings like "1.18.2", "pc_1.16.5", "bedrock_1.17.10".
/// Handles caching automatically.
///
/// # Errors
///
/// Returns `McDataError` if the version is invalid, data files are missing/corrupt,
/// or other issues occur during loading.
pub fn mc_data(version_str: &str) -> Result<Arc<IndexedData>, McDataError> {
    let version = version::resolve_version(version_str)?;
    let cache_key = format!("{}_{}", version.edition.path_prefix(), version.minecraft_version);

    // 1. Check cache (read lock)
    {
        let cache = DATA_CACHE.read().expect("Cache read lock poisoned");
        if let Some(data) = cache.get(&cache_key) {
            log::debug!("Cache hit for version: {}", version_str);
            return Ok(data.clone());
        }
    } // Read lock released

    // 2. Load data and acquire write lock
    log::debug!("Cache miss for version: {}. Loading...", version_str);
    let loaded_data = Arc::new(IndexedData::load(version)?); // Load outside the lock if possible

    // 3. Acquire write lock and insert (double-check)
    {
        let mut cache = DATA_CACHE.write().expect("Cache write lock poisoned");
        // Check again in case another thread loaded it while we were loading
        if let Some(data) = cache.get(&cache_key) {
             log::debug!("Cache hit after load for version: {}", version_str);
             return Ok(data.clone()); // Return the data loaded by the other thread
        }
        log::debug!("Inserting loaded data into cache for version: {}", version_str);
        cache.insert(cache_key.clone(), loaded_data.clone());
    } // Write lock released

    Ok(loaded_data)
}

/// Returns a list of supported Minecraft versions for the given edition.
pub fn supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    version::get_supported_versions(edition)
}

// --- Example Usage (add tests instead) ---
#[cfg(test)]
mod tests {
    use super::*;
    // Make sure env_logger is in dev-dependencies
    // You might need to add `use env_logger;` here if setup() is more complex
    // Helper to initialize logging for tests
    fn setup() {
        // Run `RUST_LOG=debug cargo test` to see logs
         let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn load_pc_1_18_2() {
        setup();
        let data = mc_data("1.18.2").expect("Failed to load 1.18.2 data");
        assert_eq!(data.version.minecraft_version, "1.18.2");
        assert_eq!(data.version.edition, Edition::Pc);
        let stone = data.blocks_by_name.get("stone").expect("Stone block not found");
        assert_eq!(stone.id, 1);
        // Item IDs change, find a stable one or update this
        // Let's use name which is more stable
        assert!(data.items_by_name.contains_key("stick"), "Stick item not found by name");
        // assert_eq!(stick.id, 280); // Example ID for 1.18.2, adjust if needed
    }

     #[test]
     fn load_pc_major_version() {
         setup();
         // Should resolve to the latest release within 1.16 (e.g., 1.16.5)
         let data = mc_data("1.16").expect("Failed to load 1.16 data");
         assert!(data.version.minecraft_version.starts_with("1.16"));
         assert_eq!(data.version.edition, Edition::Pc);
         assert!(data.blocks_by_name.contains_key("netherite_block"));
     }

    #[test]
    fn test_version_comparison() {
         setup();
         let data_1_18 = mc_data("1.18.2").unwrap();
         let data_1_16 = mc_data("1.16.5").unwrap();

         assert!(data_1_18.is_newer_or_equal_to("1.16.5").unwrap());
         assert!(data_1_18.is_newer_or_equal_to("1.18.2").unwrap());
         // This test might fail if 1.19 data isn't present yet in your submodule
         // assert!(!data_1_18.is_newer_or_equal_to("1.19").unwrap());

         assert!(data_1_16.is_older_than("1.18.2").unwrap());
         assert!(!data_1_16.is_older_than("1.16.5").unwrap());
         assert!(!data_1_16.is_older_than("1.15.2").unwrap());
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
         assert_eq!(meta_ix_118, serde_json::Value::Number(8.into())); // Check node-minecraft-data/test/load.js

         let meta_ix_115 = data_1_15.support_feature("metadataIxOfItem").unwrap();
         assert_eq!(meta_ix_115, serde_json::Value::Number(7.into()));
     }

     #[test]
     fn test_cache() {
         setup();
         let data1 = mc_data("1.17.1").expect("Load 1 failed");
         let data2 = mc_data("pc_1.17.1").expect("Load 2 failed");
         // Check if they point to the same Arc allocation (cache hit)
         assert!(Arc::ptr_eq(&data1, &data2));
     }

     // Add tests for Bedrock when implemented
     // Add tests for error conditions (invalid version, missing files)
}