use crate::structs::{
    Attribute, Biome, Block, BlockCollisionShapes, BlockLoot, BlockShapeRef, Effect, Enchantment,
    Entity, EntityLoot, Food, Instrument, Item, MapIcon, Particle, Sound, Window,
};
use log;
use std::collections::HashMap;

// Helper macro to create a HashMap index from a slice of data structs.
// It takes the data slice, the field name to use as the key, and optionally the key type.
macro_rules! index_by_field {
    // Version with explicit key type.
    ($data:expr, $field:ident, $key_type:ty) => {
        $data
            .iter()
            .map(|item| (item.$field.clone() as $key_type, item.clone()))
            .collect()
    };
    // Version inferring key type (defaults to the field's type).
    ($data:expr, $field:ident) => {
        $data
            .iter()
            .map(|item| (item.$field.clone(), item.clone()))
            .collect::<HashMap<_, _>>()
    };
}

// Functions to create indexed HashMaps for various data types.

/// Indexes blocks by ID, name, and state ID.
/// Also calculates approximate state ID ranges if they are missing in the source data.
pub fn index_blocks(
    blocks: &[Block],
) -> (
    HashMap<u32, Block>,
    HashMap<String, Block>,
    HashMap<u32, Block>,
) {
    let mut blocks_with_calculated_states = Vec::with_capacity(blocks.len());
    let mut blocks_by_state_id = HashMap::new();

    for block in blocks {
        let mut processed_block = block.clone();

        // Heuristic: If state IDs seem uninitialized (min/max are 0) but the block ID is not 0,
        // calculate a default range based on the block ID. This mimics node-minecraft-data's
        // handling for older versions lacking explicit state IDs.
        if processed_block.id != 0
            && processed_block.min_state_id == 0
            && processed_block.max_state_id == 0
        {
            // Assume 16 states per block ID (<< 4 is equivalent to * 16).
            processed_block.min_state_id = block.id << 4;
            processed_block.max_state_id = processed_block.min_state_id + 15;
            // Assume the first state in the calculated range is the default.
            processed_block.default_state = processed_block.min_state_id;
        }

        // Map all state IDs within the block's range back to the block definition.
        // Note: This doesn't create unique Block instances per state; variations are handled separately if needed.
        for state_id in processed_block.min_state_id..=processed_block.max_state_id {
            blocks_by_state_id.insert(state_id, processed_block.clone());
        }
        blocks_with_calculated_states.push(processed_block);
    }

    // Create the primary indexes using the potentially updated block data.
    let blocks_by_id: HashMap<u32, Block> = index_by_field!(blocks_with_calculated_states, id, u32);
    let blocks_by_name: HashMap<String, Block> =
        index_by_field!(blocks_with_calculated_states, name, String);

    (blocks_by_id, blocks_by_name, blocks_by_state_id)
}

/// Indexes items by ID and name.
pub fn index_items(items: &[Item]) -> (HashMap<u32, Item>, HashMap<String, Item>) {
    let items_by_id: HashMap<u32, Item> = index_by_field!(items, id, u32);
    let items_by_name: HashMap<String, Item> = index_by_field!(items, name, String);
    (items_by_id, items_by_name)
}

/// Indexes biomes by ID and name.
pub fn index_biomes(biomes: &[Biome]) -> (HashMap<u32, Biome>, HashMap<String, Biome>) {
    let by_id: HashMap<u32, Biome> = index_by_field!(biomes, id, u32);
    let by_name: HashMap<String, Biome> = index_by_field!(biomes, name, String);
    (by_id, by_name)
}

/// Indexes effects by ID and name.
pub fn index_effects(effects: &[Effect]) -> (HashMap<u32, Effect>, HashMap<String, Effect>) {
    let by_id: HashMap<u32, Effect> = index_by_field!(effects, id, u32);
    let by_name: HashMap<String, Effect> = index_by_field!(effects, name, String);
    (by_id, by_name)
}

/// Indexes entities by ID and name, and also creates filtered indexes for mobs and objects.
pub fn index_entities(
    entities: &[Entity],
) -> (
    HashMap<u32, Entity>,
    HashMap<String, Entity>,
    HashMap<u32, Entity>,
    HashMap<u32, Entity>,
) {
    let by_id: HashMap<u32, Entity> = index_by_field!(entities, id, u32);
    let by_name: HashMap<String, Entity> = index_by_field!(entities, name, String);

    // Create a filtered map containing only entities classified as "mob".
    let mobs_by_id = entities
        .iter()
        .filter(|e| e.entity_type == "mob")
        .map(|e| (e.id, e.clone()))
        .collect();

    // Create a filtered map containing only entities classified as "object".
    let objects_by_id = entities
        .iter()
        .filter(|e| e.entity_type == "object")
        .map(|e| (e.id, e.clone()))
        .collect();

    (by_id, by_name, mobs_by_id, objects_by_id)
}

/// Indexes sounds by ID and name.
pub fn index_sounds(sounds: &[Sound]) -> (HashMap<u32, Sound>, HashMap<String, Sound>) {
    let by_id: HashMap<u32, Sound> = index_by_field!(sounds, id, u32);
    let by_name: HashMap<String, Sound> = index_by_field!(sounds, name, String);
    (by_id, by_name)
}

/// Indexes particles by ID and name.
pub fn index_particles(
    particles: &[Particle],
) -> (HashMap<u32, Particle>, HashMap<String, Particle>) {
    let by_id: HashMap<u32, Particle> = index_by_field!(particles, id, u32);
    let by_name: HashMap<String, Particle> = index_by_field!(particles, name, String);
    (by_id, by_name)
}

/// Indexes attributes by name and resource key.
pub fn index_attributes(
    attributes: &[Attribute],
) -> (HashMap<String, Attribute>, HashMap<String, Attribute>) {
    let by_name: HashMap<String, Attribute> = index_by_field!(attributes, name, String);
    let by_resource: HashMap<String, Attribute> = index_by_field!(attributes, resource, String);
    (by_name, by_resource)
}

/// Indexes instruments by ID and name.
pub fn index_instruments(
    instruments: &[Instrument],
) -> (HashMap<u32, Instrument>, HashMap<String, Instrument>) {
    let by_id: HashMap<u32, Instrument> = index_by_field!(instruments, id, u32);
    let by_name: HashMap<String, Instrument> = index_by_field!(instruments, name, String);
    (by_id, by_name)
}

/// Indexes foods by ID and name.
pub fn index_foods(foods: &[Food]) -> (HashMap<u32, Food>, HashMap<String, Food>) {
    let by_id: HashMap<u32, Food> = index_by_field!(foods, id, u32);
    let by_name: HashMap<String, Food> = index_by_field!(foods, name, String);
    (by_id, by_name)
}

/// Indexes enchantments by ID and name.
pub fn index_enchantments(
    enchantments: &[Enchantment],
) -> (HashMap<u32, Enchantment>, HashMap<String, Enchantment>) {
    let by_id: HashMap<u32, Enchantment> = index_by_field!(enchantments, id, u32);
    let by_name: HashMap<String, Enchantment> = index_by_field!(enchantments, name, String);
    (by_id, by_name)
}

/// Indexes map icons by ID and name.
pub fn index_map_icons(map_icons: &[MapIcon]) -> (HashMap<u32, MapIcon>, HashMap<String, MapIcon>) {
    let by_id: HashMap<u32, MapIcon> = index_by_field!(map_icons, id, u32);
    let by_name: HashMap<String, MapIcon> = index_by_field!(map_icons, name, String);
    (by_id, by_name)
}

/// Indexes windows (containers/GUIs) by ID and name.
pub fn index_windows(windows: &[Window]) -> (HashMap<String, Window>, HashMap<String, Window>) {
    let by_id: HashMap<String, Window> = index_by_field!(windows, id, String);
    let by_name: HashMap<String, Window> = index_by_field!(windows, name, String);
    (by_id, by_name)
}

/// Indexes block loot tables by block name.
pub fn index_block_loot(block_loot: &[BlockLoot]) -> HashMap<String, BlockLoot> {
    index_by_field!(block_loot, block, String)
}

/// Indexes entity loot tables by entity name.
pub fn index_entity_loot(entity_loot: &[EntityLoot]) -> HashMap<String, EntityLoot> {
    index_by_field!(entity_loot, entity, String)
}

/// Creates HashMaps mapping block state IDs and block names (for default state)
/// to their corresponding collision shape bounding boxes.
///
/// Uses the pre-indexed block maps and the raw collision shape data.
pub fn index_block_shapes(
    blocks_by_state_id: &HashMap<u32, Block>,
    blocks_by_name: &HashMap<String, Block>,
    collision_data: &BlockCollisionShapes,
) -> (HashMap<u32, Vec<[f64; 6]>>, HashMap<String, Vec<[f64; 6]>>) {
    log::debug!("Indexing block shapes...");
    let mut shapes_by_state_id = HashMap::new();
    let mut shapes_by_name = HashMap::new();

    // Iterate through all known block states.
    for (state_id, block) in blocks_by_state_id.iter() {
        // Find the shape reference for this block's name in the collision data.
        if let Some(shape_ref) = collision_data.blocks.get(&block.name) {
            // Determine the specific shape index for this state.
            let shape_index_result: Option<u32> = match shape_ref {
                // If only one shape index is defined, use it for all states.
                BlockShapeRef::Single(index) => Some(*index),
                // If multiple indices are defined, calculate the offset based on the state ID.
                BlockShapeRef::Multiple(indices) => {
                    // Calculate the offset from the block's minimum state ID.
                    let offset_res = state_id.checked_sub(block.min_state_id);
                    if offset_res.is_none() {
                        log::warn!(
                            "State ID {} < minStateId {} for block {}",
                            state_id,
                            block.min_state_id,
                            block.name
                        );
                    }
                    // Use the offset to get the shape index from the list.
                    offset_res.and_then(|offset| {
                        let shape_idx = indices.get(offset as usize).copied();
                        if shape_idx.is_none() {
                             log::warn!("Shape index offset {} out of bounds (len {}) for block {} state {}", offset, indices.len(), block.name, state_id);
                        }
                        shape_idx
                    })
                }
            };

            // If a valid shape index was determined...
            if let Some(shape_index) = shape_index_result {
                // Shape index 0 represents "no collision box" (like air). Skip it.
                if shape_index == 0 {
                    continue;
                }
                // Look up the actual bounding box array using the shape index.
                if let Some(shape_vec) = collision_data.shapes.get(&shape_index.to_string()) {
                    // Insert the shape data into the state ID index.
                    shapes_by_state_id.insert(*state_id, shape_vec.clone());
                } else {
                    // This indicates inconsistency in the source data.
                    log::warn!(
                        "Shape index {} found for block {} state {}, but not found in shapes map.",
                        shape_index,
                        block.name,
                        state_id
                    );
                }
            }
        } else if block.name != "air" {
            // Don't warn if 'air' is missing, it's expected.
            // This indicates a block exists but has no entry in the collision shape data.
            log::warn!(
                "Block '{}' not found in blockCollisionShapes.blocks map.",
                block.name
            );
        }
    }

    // Populate the index mapping block names to their default state's shape.
    log::debug!("Populating shapes_by_name map using default states...");
    for (name, block) in blocks_by_name.iter() {
        // Find the shape associated with the block's default state ID.
        if let Some(shape) = shapes_by_state_id.get(&block.default_state) {
            shapes_by_name.insert(name.clone(), shape.clone());
        } else {
            // Log a warning if a shape was expected but not found for the default state.
            // Avoid warning for blocks explicitly defined as shapeless (shape index 0).
            let is_explicitly_shapeless =
                collision_data.blocks.get(name).map_or(false, |shape_ref| {
                    matches!(shape_ref, BlockShapeRef::Single(0))
                });
            if name != "air" && !is_explicitly_shapeless {
                log::warn!(
                    "Default state shape not found for block '{}' (defaultState: {})",
                    name,
                    block.default_state
                );
            }
        }
    }
    log::debug!("Finished indexing block shapes.");

    (shapes_by_state_id, shapes_by_name)
}
