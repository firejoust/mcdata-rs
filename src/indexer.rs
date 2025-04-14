use std::collections::HashMap;

use crate::{Biome, Block, Effect, Entity, Item};

// --- Helper Macro for Indexing ---
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

// REMOVED index_option_by_field macro


// --- Indexing Functions ---

pub fn index_blocks(blocks: &[Block]) -> (HashMap<u32, Block>, HashMap<String, Block>, HashMap<u32, Block>) {
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
         .filter(|e| e.entity_type == "mob")
         .map(|e| (e.id, e.clone()))
         .collect();

     let objects_by_id = entities.iter()
         .filter(|e| e.entity_type == "object") // Or other non-mob types considered "objects"
         .map(|e| (e.id, e.clone()))
         .collect();

     (by_id, by_name, mobs_by_id, objects_by_id)
 }

// Add similar functions for enchantments, particles, windows, etc.