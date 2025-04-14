// tests/integration_test.rs

use mcdata_rs::*; // Import items from your library's root
use serde_json::Value;
use std::sync::Arc;

// Helper to initialize logging for tests
// Run tests with `RUST_LOG=debug cargo test -- --nocapture` to see logs
fn setup() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn load_specific_pc_version() {
    setup();
    let version = "1.18.2";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, version);
    assert_eq!(data.version.edition, Edition::Pc);

    // Check block indexing
    let stone = data.blocks_by_name.get("stone").expect("Stone block not found by name");
    assert_eq!(stone.id, 1);
    assert_eq!(data.blocks_by_id.get(&1).unwrap().name, "stone");
    // State IDs might vary slightly depending on exact generation, but default should exist
    assert!(data.blocks_by_state_id.contains_key(&stone.default_state), "Default state ID for stone not found");
    let stone_from_state = data.blocks_by_state_id.get(&stone.default_state).unwrap();
    assert_eq!(stone_from_state.name, "stone");


    // Check item indexing
    let stick = data.items_by_name.get("stick").expect("Stick item not found by name");
    let stick_id = stick.id; // Get ID dynamically
    assert_eq!(data.items_by_id.get(&stick_id).unwrap().name, "stick");

    // Check other data types (add more as needed)
    assert!(!data.biomes_by_name.is_empty(), "Biomes map is empty");
    assert!(!data.effects_by_name.is_empty(), "Effects map is empty");
    assert!(!data.entities_by_name.is_empty(), "Entities map is empty");
    assert!(data.language.contains_key("block.minecraft.stone"), "Language map missing key");
}

#[test]
fn load_prefixed_pc_version() {
    setup();
    let version = "pc_1.16.5";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, "1.16.5");
    assert_eq!(data.version.edition, Edition::Pc);
    assert!(data.blocks_by_name.contains_key("netherite_block"));
}

#[test]
fn load_major_pc_version() {
    setup();
    let version = "1.19"; // Should resolve to latest release in 1.19 (e.g., 1.19.4 at time of writing)
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert!(data.version.minecraft_version.starts_with("1.19"));
    assert_eq!(data.version.edition, Edition::Pc);
    assert!(data.blocks_by_name.contains_key("mangrove_log")); // Block added in 1.19
    assert!(data.entities_by_name.contains_key("warden")); // Entity added in 1.19
}

#[test]
fn load_older_pc_version() {
    setup();
    let version = "1.8.8";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, "1.8.8");
    assert_eq!(data.version.edition, Edition::Pc);
    assert!(data.blocks_by_name.contains_key("stone"));
    assert!(!data.blocks_by_name.contains_key("shulker_box")); // Doesn't exist in 1.8
}

#[test]
fn version_comparison() {
    setup();
    let data_1_18 = mc_data("1.18.2").unwrap();
    let data_1_16 = mc_data("1.16.5").unwrap();

    // is_newer_or_equal_to
    assert!(data_1_18.is_newer_or_equal_to("1.16.5").unwrap());
    assert!(data_1_18.is_newer_or_equal_to("1.18.2").unwrap());
    assert!(data_1_18.is_newer_or_equal_to("1.16").unwrap()); // Compare against major
    assert!(!data_1_16.is_newer_or_equal_to("1.18.2").unwrap());
    // Test against a potentially non-existent version (should resolve via major)
    // This depends on 1.19 being present in your protocolVersions
    // assert!(!data_1_18.is_newer_or_equal_to("1.19").unwrap());

    // is_older_than
    assert!(data_1_16.is_older_than("1.18.2").unwrap());
    assert!(data_1_16.is_older_than("1.17.1").unwrap());
    assert!(!data_1_16.is_older_than("1.16.5").unwrap());
    assert!(!data_1_16.is_older_than("1.15.2").unwrap());
    assert!(!data_1_18.is_older_than("1.18.2").unwrap());
}

#[test]
fn feature_support() {
    setup();
    let data_1_18 = mc_data("1.18.2").unwrap();
    let data_1_15 = mc_data("1.15.2").unwrap();
    let data_1_8 = mc_data("1.8.8").unwrap();

    // Boolean feature check
    let dim_int_115 = data_1_15.support_feature("dimensionIsAnInt").unwrap();
    assert_eq!(dim_int_115, Value::Bool(true), "dimensionIsAnInt should be true for 1.15.2");

    let dim_int_118 = data_1_18.support_feature("dimensionIsAnInt").unwrap();
    assert_eq!(dim_int_118, Value::Bool(false), "dimensionIsAnInt should be false for 1.18.2");

    // Valued feature check (using values from node-minecraft-data/test/load.js)
    let meta_ix_118 = data_1_18.support_feature("metadataIxOfItem").unwrap();
    assert_eq!(meta_ix_118, Value::Number(8.into()), "metadataIxOfItem mismatch for 1.18.2");

    let meta_ix_115 = data_1_15.support_feature("metadataIxOfItem").unwrap();
    assert_eq!(meta_ix_115, Value::Number(7.into()), "metadataIxOfItem mismatch for 1.15.2");

    let meta_ix_18 = data_1_8.support_feature("metadataIxOfItem").unwrap();
     assert_eq!(meta_ix_18, Value::Number(8.into()), "metadataIxOfItem mismatch for 1.8.8"); // Note: 1.8 had a different structure

    // Check a feature that doesn't exist for a version
    let non_existent_feature = data_1_8.support_feature("someRandomFeatureName").unwrap();
    assert_eq!(non_existent_feature, Value::Bool(false), "Non-existent feature should return false");
}

#[test]
fn cache_hit() {
    setup();
    let version = "1.17.1"; // Choose a version not loaded by other tests if possible
    log::info!("Loading {} for the first time", version);
    let data1 = mc_data(version).expect("Load 1 failed");
    log::info!("Loading {} for the second time", version);
    let data2 = mc_data(version).expect("Load 2 failed");

    // Check if they point to the same Arc allocation (cache hit)
    assert!(Arc::ptr_eq(&data1, &data2), "Cache miss: Arcs point to different data");

    // Also test with prefix
    log::info!("Loading pc_{} for the third time", version);
    let data3 = mc_data(&format!("pc_{}", version)).expect("Load 3 failed");
    assert!(Arc::ptr_eq(&data1, &data3), "Cache miss: Prefixed version loaded different data");
}

#[test]
fn invalid_version_error() {
    setup();
    let version = "invalid_version_string";
    let result = mc_data(version);

    assert!(result.is_err(), "Expected an error for invalid version '{}'", version);
    match result.err().unwrap() {
        McDataError::InvalidVersion(v) => assert_eq!(v, version),
        e => panic!("Expected InvalidVersion error, got {:?}", e),
    }
}

#[test]
fn supported_versions_list() {
    setup();
    let pc_versions = supported_versions(Edition::Pc).expect("Failed to get PC versions");
    assert!(!pc_versions.is_empty(), "PC versions list is empty");
    assert!(pc_versions.contains(&"1.18.2".to_string()));
    assert!(pc_versions.contains(&"1.8.8".to_string()));

    // Add similar check for Bedrock when supported
    // let bedrock_versions = supported_versions(Edition::Bedrock).expect("Failed to get Bedrock versions");
    // assert!(!bedrock_versions.is_empty(), "Bedrock versions list is empty");
}

// TODO: Add tests for Bedrock edition once implemented.
// TODO: Add tests for specific data points in various versions (e.g., recipe shapes, entity properties).
// TODO: Consider tests for edge cases like snapshot versions if needed.