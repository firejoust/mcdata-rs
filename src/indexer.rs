// src/indexer.rs
use crate::structs::{
    Block, Item, Biome, Effect, Entity, Sound, Particle, Attribute, Instrument, Food, Enchantment,
    MapIcon, Window, BlockLoot, EntityLoot, BlockCollisionShapes, BlockShapeRef,
};
use std::collections::HashMap;
use log;

// --- Helper Macro for Indexing ---
// ... (index_by_field macro remains the same) ...
macro_rules! index_by_field {
    ($data:expr, $field:ident, $key_type:ty) => {
        $data.iter().map(|item| (item.$field.clone() as $key_type, item.clone())).collect()
    };
     ($data:expr, $field:ident) => { // Default key type based on field name
        {
            // This logic is a bit simplistic, might need refinement if keys aren't u32 or String
            // let key_type = if stringify!($field) == "id" { u32::default() } else { String::default() };
             $data.iter().map(|item| (item.$field.clone(), item.clone())).collect::<HashMap<_,_>>()
        }
    };
}


// --- Indexing Functions ---

pub fn index_blocks(blocks: &[Block]) -> (HashMap<u32, Block>, HashMap<String, Block>, HashMap<u32, Block>) {
    // ... (implementation remains the same) ...
    let mut blocks_with_states = Vec::with_capacity(blocks.len());
    let mut blocks_by_state_id = HashMap::new();

    for block in blocks {
        let mut processed_block = block.clone();

        // Calculate state IDs if they seem default (0) and ID is non-zero
        // This matches the logic in node-minecraft-data/lib/indexes.js
        if processed_block.id != 0 && processed_block.min_state_id == 0 && processed_block.max_state_id == 0 {
            processed_block.min_state_id = block.id << 4;
            processed_block.max_state_id = processed_block.min_state_id + 15; // Assume 16 states per block ID if not specified
            processed_block.default_state = processed_block.min_state_id; // Assume first state is default
        }

        for state_id in processed_block.min_state_id..=processed_block.max_state_id {
            // TODO: Potentially create unique Block instances per state if variations exist
            // For now, just map all states to the base block definition
            blocks_by_state_id.insert(state_id, processed_block.clone());
        }
        blocks_with_states.push(processed_block);
    }

    let blocks_by_id: HashMap<u32, Block> = index_by_field!(blocks_with_states, id, u32);
    let blocks_by_name: HashMap<String, Block> = index_by_field!(blocks_with_states, name, String);

    (blocks_by_id, blocks_by_name, blocks_by_state_id)
}

// ... (other index functions remain the same) ...
pub fn index_items(items: &[Item]) -> (HashMap<u32, Item>, HashMap<String, Item>) {
    let items_by_id: HashMap<u32, Item> = index_by_field!(items, id, u32);
    let items_by_name: HashMap<String, Item> = index_by_field!(items, name, String);
    (items_by_id, items_by_name)
}

pub fn index_biomes(biomes: &[Biome]) -> (HashMap<u32, Biome>, HashMap<String, Biome>) {
     let by_id: HashMap<u32, Biome> = index_by_field!(biomes, id, u32);
     let by_name: HashMap<String, Biome> = index_by_field!(biomes, name, String);
     (by_id, by_name)
 }

 pub fn index_effects(effects: &[Effect]) -> (HashMap<u32, Effect>, HashMap<String, Effect>) {
     let by_id: HashMap<u32, Effect> = index_by_field!(effects, id, u32);
     let by_name: HashMap<String, Effect> = index_by_field!(effects, name, String);
     (by_id, by_name)
 }

 pub fn index_entities(entities: &[Entity]) -> (HashMap<u32, Entity>, HashMap<String, Entity>, HashMap<u32, Entity>, HashMap<u32, Entity>) {
     let by_id: HashMap<u32, Entity> = index_by_field!(entities, id, u32);
     let by_name: HashMap<String, Entity> = index_by_field!(entities, name, String);

     let mobs_by_id = entities.iter()
         .filter(|e| e.entity_type == "mob") // Simple check, might need refinement
         .map(|e| (e.id, e.clone()))
         .collect();

     let objects_by_id = entities.iter()
         .filter(|e| e.entity_type == "object") // Simple check
         .map(|e| (e.id, e.clone()))
         .collect();

     (by_id, by_name, mobs_by_id, objects_by_id)
 }

 pub fn index_sounds(sounds: &[Sound]) -> (HashMap<u32, Sound>, HashMap<String, Sound>) {
    let by_id: HashMap<u32, Sound> = index_by_field!(sounds, id, u32);
    let by_name: HashMap<String, Sound> = index_by_field!(sounds, name, String);
    (by_id, by_name)
}

pub fn index_particles(particles: &[Particle]) -> (HashMap<u32, Particle>, HashMap<String, Particle>) {
    let by_id: HashMap<u32, Particle> = index_by_field!(particles, id, u32);
    let by_name: HashMap<String, Particle> = index_by_field!(particles, name, String);
    (by_id, by_name)
}

pub fn index_attributes(attributes: &[Attribute]) -> (HashMap<String, Attribute>, HashMap<String, Attribute>) {
    let by_name: HashMap<String, Attribute> = index_by_field!(attributes, name, String);
    let by_resource: HashMap<String, Attribute> = index_by_field!(attributes, resource, String);
    (by_name, by_resource)
}

pub fn index_instruments(instruments: &[Instrument]) -> (HashMap<u32, Instrument>, HashMap<String, Instrument>) {
    let by_id: HashMap<u32, Instrument> = index_by_field!(instruments, id, u32);
    let by_name: HashMap<String, Instrument> = index_by_field!(instruments, name, String);
    (by_id, by_name)
}

pub fn index_foods(foods: &[Food]) -> (HashMap<u32, Food>, HashMap<String, Food>) {
    let by_id: HashMap<u32, Food> = index_by_field!(foods, id, u32);
    let by_name: HashMap<String, Food> = index_by_field!(foods, name, String);
    (by_id, by_name)
}

pub fn index_enchantments(enchantments: &[Enchantment]) -> (HashMap<u32, Enchantment>, HashMap<String, Enchantment>) {
    let by_id: HashMap<u32, Enchantment> = index_by_field!(enchantments, id, u32);
    let by_name: HashMap<String, Enchantment> = index_by_field!(enchantments, name, String);
    (by_id, by_name)
}

pub fn index_map_icons(map_icons: &[MapIcon]) -> (HashMap<u32, MapIcon>, HashMap<String, MapIcon>) {
    let by_id: HashMap<u32, MapIcon> = index_by_field!(map_icons, id, u32);
    let by_name: HashMap<String, MapIcon> = index_by_field!(map_icons, name, String);
    (by_id, by_name)
}

pub fn index_windows(windows: &[Window]) -> (HashMap<String, Window>, HashMap<String, Window>) {
    let by_id: HashMap<String, Window> = index_by_field!(windows, id, String);
    let by_name: HashMap<String, Window> = index_by_field!(windows, name, String);
    (by_id, by_name)
}

pub fn index_block_loot(block_loot: &[BlockLoot]) -> HashMap<String, BlockLoot> {
    index_by_field!(block_loot, block, String)
}

pub fn index_entity_loot(entity_loot: &[EntityLoot]) -> HashMap<String, EntityLoot> {
    index_by_field!(entity_loot, entity, String)
}


/// Creates HashMaps mapping state IDs and block names (default state) to their collision shapes.
pub fn index_block_shapes(
    blocks_by_state_id: &HashMap<u32, Block>,
    blocks_by_name: &HashMap<String, Block>,
    collision_data: &BlockCollisionShapes,
) -> (HashMap<u32, Vec<[f64; 6]>>, HashMap<String, Vec<[f64; 6]>>) {
    log::debug!("Indexing block shapes...");
    let mut shapes_by_state_id = HashMap::new();
    let mut shapes_by_name = HashMap::new();

    for (state_id, block) in blocks_by_state_id.iter() {
        if let Some(shape_ref) = collision_data.blocks.get(&block.name) {
            let shape_index_result: Option<u32> = match shape_ref {
                BlockShapeRef::Single(index) => Some(*index),
                BlockShapeRef::Multiple(indices) => {
                    let offset_res = state_id.checked_sub(block.min_state_id);
                    if offset_res.is_none() {
                         log::warn!("State ID {} < minStateId {} for block {}", state_id, block.min_state_id, block.name);
                    }
                    offset_res.and_then(|offset| {
                        let shape_idx = indices.get(offset as usize).copied();
                        if shape_idx.is_none() {
                             log::warn!("Offset {} out of bounds (len {}) for block {} state {}", offset, indices.len(), block.name, state_id);
                        }
                        shape_idx
                    })
                }
            };

            if let Some(shape_index) = shape_index_result {
                if shape_index == 0 { // Shape 0 means no collision box
                    continue;
                }
                if let Some(shape_vec) = collision_data.shapes.get(&shape_index.to_string()) {
                    shapes_by_state_id.insert(*state_id, shape_vec.clone());
                } else {
                     log::warn!("Shape index {} found for block {} state {}, but not found in shapes map.", shape_index, block.name, state_id);
                }
            } // else: Shape index calculation failed (logged above) or block not in collision data
        } else if block.name != "air" { // Don't warn for air
             log::warn!("Block '{}' not found in blockCollisionShapes.blocks map.", block.name);
        }
    }

    log::debug!("Populating shapes_by_name map...");
    for (name, block) in blocks_by_name.iter() {
        if let Some(shape) = shapes_by_state_id.get(&block.default_state) {
             shapes_by_name.insert(name.clone(), shape.clone());
        } else {
            // Log only if the block wasn't expected to be shapeless (like air)
            let is_shapeless = collision_data.blocks.get(name)
                .map_or(true, |shape_ref| matches!(shape_ref, BlockShapeRef::Single(0))); // Check if shape ref is 0
            if !is_shapeless {
                 log::warn!("Default state shape not found for block '{}' (defaultState: {})", name, block.default_state);
            }
        }
    }
    log::debug!("Finished indexing block shapes.");

    (shapes_by_state_id, shapes_by_name)
}