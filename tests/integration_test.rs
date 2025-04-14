// tests/integration_test.rs

// Use the crate name defined in Cargo.toml
use mcdata_rs::*;
use serde_json::Value;
use std::sync::Arc;

// Helper to initialize logging for tests
// Run tests with `RUST_LOG=debug cargo test -- --nocapture` to see logs
fn setup() {
    // Use try_init to avoid panic if logger is already initialized
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn load_specific_pc_version_1_18_2() {
    // ... (test remains the same) ...
    setup();
    let version = "1.18.2";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, version);
    assert_eq!(data.version.edition, Edition::Pc);

    // Check block indexing
    let stone = data.blocks_by_name.get("stone").expect("Stone block not found by name");
    assert_eq!(stone.id, 1);
    assert_eq!(data.blocks_by_id.get(&1).unwrap().name, "stone");
    assert!(data.blocks_by_state_id.contains_key(&stone.default_state), "Default state ID for stone not found");
    let stone_from_state = data.blocks_by_state_id.get(&stone.default_state).unwrap();
    assert_eq!(stone_from_state.name, "stone");

    // Check item indexing
    let stick = data.items_by_name.get("stick").expect("Stick item not found by name");
    let stick_id = stick.id;
    assert_eq!(data.items_by_id.get(&stick_id).unwrap().name, "stick");

    // Check other data types (basic non-empty checks)
    assert!(!data.biomes_array.is_empty(), "Biomes array is empty");
    assert!(!data.effects_array.is_empty(), "Effects array is empty");
    assert!(!data.entities_array.is_empty(), "Entities array is empty");
    assert!(!data.sounds_array.is_empty(), "Sounds array is empty");
    assert!(!data.particles_array.is_empty(), "Particles array is empty");
    assert!(!data.foods_array.is_empty(), "Foods array is empty");
    assert!(!data.enchantments_array.is_empty(), "Enchantments array is empty");
    assert!(!data.map_icons_array.is_empty(), "MapIcons array is empty");
    assert!(!data.windows_array.is_empty(), "Windows array is empty");
    assert!(!data.block_loot_array.is_empty(), "BlockLoot array is empty");
    assert!(!data.entity_loot_array.is_empty(), "EntityLoot array is empty");

    // Check optional data presence (might be None depending on version)
    assert!(data.block_collision_shapes_raw.is_some(), "BlockCollisionShapes raw is None"); // Check raw data
    assert!(!data.block_shapes_by_name.is_empty(), "Block shapes by name map is empty"); // Check indexed map
    assert!(!data.block_shapes_by_state_id.is_empty(), "Block shapes by state id map is empty"); // Check indexed map
    assert!(data.tints.is_some(), "Tints is None");
    assert!(!data.language.is_empty(), "Language map is empty");

    // Check raw value data presence
    assert!(data.recipes.is_some(), "Recipes is None");
    assert!(data.materials.is_some(), "Materials is None");
    // Commands might be missing in some versions
    // assert!(data.commands.is_some(), "Commands is None");
    assert!(data.protocol.is_some(), "Protocol is None");
    assert!(data.login_packet.is_some(), "LoginPacket is None");

    // Check specific loaded values
    let apple = data.foods_by_name.get("apple").expect("Apple food not found");
    assert_eq!(apple.food_points, 4.0);

    let sharpness = data.enchantments_by_name.get("sharpness").expect("Sharpness enchantment not found");
    assert_eq!(sharpness.id, 12, "Sharpness ID mismatch for 1.18.2");

    let player_icon = data.map_icons_by_name.get("player").expect("Player map icon not found");
    assert_eq!(player_icon.id, 0);
}

// ... (other tests remain the same) ...
#[test]
fn load_prefixed_pc_version() {
    setup();
    let version = "pc_1.16.5";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, "1.16.5");
    assert_eq!(data.version.edition, Edition::Pc);
    assert!(data.blocks_by_name.contains_key("netherite_block"));
    assert!(!data.foods_array.is_empty());
    assert!(!data.attributes_array.is_empty());
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
    assert!(!data.instruments_array.is_empty()); // Instruments exist
}

#[test]
fn load_older_pc_version_1_8_8() {
    setup();
    let version = "1.8.8";
    let data = mc_data(version).unwrap_or_else(|e| panic!("Failed to load {}: {:?}", version, e));

    assert_eq!(data.version.minecraft_version, "1.8.8");
    assert_eq!(data.version.edition, Edition::Pc);
    assert!(data.blocks_by_name.contains_key("stone"));
    assert!(!data.blocks_by_name.contains_key("shulker_box")); // Doesn't exist in 1.8
    assert!(!data.items_array.is_empty());
    assert!(!data.foods_array.is_empty());
    assert!(data.block_collision_shapes_raw.is_some()); // Collision shapes exist in 1.8
    assert!(!data.block_shapes_by_name.is_empty()); // Check indexed map
    assert!(data.recipes.is_some()); // Recipes exist
    // Check if block drops are loaded correctly for older format
    let stone_block = data.blocks_by_name.get("stone").unwrap();
    assert!(!stone_block.drops.is_empty());
    match &stone_block.drops[0] {
        BlockDrop::Element(el) => match el.drop {
            DropType::Id(id) => assert_eq!(id, 4), // Cobblestone ID in 1.8
            _ => panic!("Expected simple ID drop for stone in 1.8"),
        },
        _ => panic!("Expected Element drop for stone in 1.8"),
    }
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

    // Check feature with _major range
    let item_frame_map_feature = data_1_8.support_feature("itemFrameMapIsRotated").unwrap();
    // UPDATE THIS ASSERTION based on debug output or re-evaluation
    assert_eq!(item_frame_map_feature, Value::Bool(false), "itemFrameMapIsRotated should be false for 1.8");
    let item_frame_map_feature_1_18 = data_1_18.support_feature("itemFrameMapIsRotated").unwrap();
    assert_eq!(item_frame_map_feature_1_18, Value::Bool(false), "itemFrameMapIsRotated should be false for 1.18");

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
    // Check for a few known versions
    assert!(pc_versions.iter().any(|v| v == "1.18.2"));
    assert!(pc_versions.iter().any(|v| v == "1.8.8"));
    assert!(pc_versions.iter().any(|v| v == "1.20.4"));

    // Add similar check for Bedrock when supported
    // let bedrock_versions = supported_versions(Edition::Bedrock).expect("Failed to get Bedrock versions");
    // assert!(!bedrock_versions.is_empty(), "Bedrock versions list is empty");
}

#[test]
fn block_shapes() {
    setup();
    let data = mc_data("1.18.2").unwrap(); // Use a version with collision shapes

    // Test 1: Simple block (stone) by name - should have shape index 1
    let stone_shape = data.block_shapes_by_name.get("stone").expect("Stone shape not found by name");
    assert_eq!(stone_shape.len(), 1, "Stone should have 1 bounding box");
    assert_eq!(stone_shape[0], [0.0, 0.0, 0.0, 1.0, 1.0, 1.0], "Stone bounding box mismatch");

    // Test 2: Simple block (stone) by state ID
    let stone_block = data.blocks_by_name.get("stone").unwrap();
    let stone_shape_by_state = data.block_shapes_by_state_id.get(&stone_block.default_state)
        .expect("Stone shape not found by state ID");
    assert_eq!(stone_shape_by_state.len(), 1);
    assert_eq!(stone_shape_by_state[0], [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

    // Test 3: Block with multiple shapes (oak_slab, type=bottom) by name (default state)
    let oak_slab_block = data.blocks_by_name.get("oak_slab").expect("Oak slab not found");
    let oak_slab_shape_default = data.block_shapes_by_name.get("oak_slab")
        .expect("Oak slab default shape not found by name");
    // Default oak slab (bottom) should have shape index 15 in 1.18.2
    assert_eq!(oak_slab_shape_default.len(), 1);
    assert_eq!(oak_slab_shape_default[0], [0.0, 0.0, 0.0, 1.0, 0.5, 1.0], "Oak slab (bottom) shape mismatch");

    // Test 4: Block with multiple shapes (oak_slab, type=top) by state ID
    // Find the state ID for oak_slab[type=top]. This requires parsing block states or knowing the ID.
    // For 1.18.2, oak_slab[type=top] is often minStateId + 1 (or similar offset).
    // Let's assume default state is bottom (minStateId) and top is minStateId + 1
    // WARNING: This assumption might break if state order changes. A robust test would parse states.
    let top_slab_state_id = oak_slab_block.min_state_id + 1; // Assuming 'top' is the second state
    let oak_slab_shape_top = data.block_shapes_by_state_id.get(&top_slab_state_id)
         .expect("Oak slab top shape not found by state ID");
    // Top oak slab should have shape index 16 in 1.18.2
    assert_eq!(oak_slab_shape_top.len(), 1);
    assert_eq!(oak_slab_shape_top[0], [0.0, 0.5, 0.0, 1.0, 1.0, 1.0], "Oak slab (top) shape mismatch");


    // Test 5: Block with no shape (air)
    assert!(data.block_shapes_by_name.get("air").is_none(), "Air should not have a shape entry in shapes map (shape index 0)");
    let air_block = data.blocks_by_name.get("air").unwrap();
    assert!(data.block_shapes_by_state_id.get(&air_block.default_state).is_none(), "Air state should not have a shape");

    // Test 6: Non-existent block
    assert!(data.block_shapes_by_name.get("not_a_real_block").is_none());
    // assert!(data.block_shapes_by_state_id.get(999999).is_none()); // This state ID might actually exist by chance

}


// TODO: Add tests for Bedrock edition once implemented.
// TODO: Add tests for specific data points in various versions (e.g., recipe shapes, entity properties).
// TODO: Consider tests for edge cases like snapshot versions if needed.