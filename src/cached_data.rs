// src/cached_data.rs

use crate::error::McDataError;
use crate::structs::*; // Import all structs
use crate::version::Version;
use crate::loader;
use crate::indexer;
use crate::features;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct IndexedData {
    pub version: Version, // The resolved canonical version

    // --- Indexed Data ---
    pub blocks_array: Arc<Vec<Block>>,
    pub blocks_by_id: Arc<HashMap<u32, Block>>,
    pub blocks_by_name: Arc<HashMap<String, Block>>,
    pub blocks_by_state_id: Arc<HashMap<u32, Block>>,

    pub items_array: Arc<Vec<Item>>,
    pub items_by_id: Arc<HashMap<u32, Item>>,
    pub items_by_name: Arc<HashMap<String, Item>>,

    pub biomes_array: Arc<Vec<Biome>>,
    pub biomes_by_id: Arc<HashMap<u32, Biome>>,
    pub biomes_by_name: Arc<HashMap<String, Biome>>,

    pub effects_array: Arc<Vec<Effect>>,
    pub effects_by_id: Arc<HashMap<u32, Effect>>,
    pub effects_by_name: Arc<HashMap<String, Effect>>,

    pub entities_array: Arc<Vec<Entity>>,
    pub entities_by_id: Arc<HashMap<u32, Entity>>,
    pub entities_by_name: Arc<HashMap<String, Entity>>,
    pub mobs_by_id: Arc<HashMap<u32, Entity>>,
    pub objects_by_id: Arc<HashMap<u32, Entity>>,

    pub sounds_array: Arc<Vec<Sound>>,
    pub sounds_by_id: Arc<HashMap<u32, Sound>>,
    pub sounds_by_name: Arc<HashMap<String, Sound>>,

    pub particles_array: Arc<Vec<Particle>>,
    pub particles_by_id: Arc<HashMap<u32, Particle>>,
    pub particles_by_name: Arc<HashMap<String, Particle>>,

    pub attributes_array: Arc<Vec<Attribute>>,
    pub attributes_by_name: Arc<HashMap<String, Attribute>>,
    pub attributes_by_resource: Arc<HashMap<String, Attribute>>,

    pub instruments_array: Arc<Vec<Instrument>>,
    pub instruments_by_id: Arc<HashMap<u32, Instrument>>,
    pub instruments_by_name: Arc<HashMap<String, Instrument>>,

    pub foods_array: Arc<Vec<Food>>,
    pub foods_by_id: Arc<HashMap<u32, Food>>,
    pub foods_by_name: Arc<HashMap<String, Food>>,

    pub enchantments_array: Arc<Vec<Enchantment>>,
    pub enchantments_by_id: Arc<HashMap<u32, Enchantment>>,
    pub enchantments_by_name: Arc<HashMap<String, Enchantment>>,

    pub map_icons_array: Arc<Vec<MapIcon>>,
    pub map_icons_by_id: Arc<HashMap<u32, MapIcon>>,
    pub map_icons_by_name: Arc<HashMap<String, MapIcon>>,

    pub windows_array: Arc<Vec<Window>>,
    pub windows_by_id: Arc<HashMap<String, Window>>,
    pub windows_by_name: Arc<HashMap<String, Window>>,

    pub block_loot_array: Arc<Vec<BlockLoot>>,
    pub block_loot_by_name: Arc<HashMap<String, BlockLoot>>,

    pub entity_loot_array: Arc<Vec<EntityLoot>>,
    pub entity_loot_by_name: Arc<HashMap<String, EntityLoot>>,

    // --- Less Structured Data ---
    pub block_collision_shapes: Arc<Option<BlockCollisionShapes>>,
    pub tints: Arc<Option<Tints>>,
    pub language: Arc<HashMap<String, String>>,
    pub legacy: Arc<Option<Legacy>>, // Common data

    // --- Raw JSON Values for Complex/Varying Data ---
    pub recipes: Arc<Option<Value>>,
    pub materials: Arc<Option<Value>>,
    pub commands: Arc<Option<Value>>,
    pub protocol: Arc<Option<Value>>, // For protocol.json
    pub protocol_comments: Arc<Option<Value>>, // For protocolComments.json
    pub login_packet: Arc<Option<Value>>, // For loginPacket.json
}

impl IndexedData {
    /// Loads and indexes all data for the given canonical version.
    pub fn load(version: Version) -> Result<Self, McDataError> {
        // Use major_version for loading paths as per node-minecraft-data logic
        let major_version_str = &version.major_version;
        let edition = version.edition;

        // Helper macro to load optional data, handling specific errors
        macro_rules! load_optional {
            ($key:expr, $type:ty) => {
                match loader::load_data::<$type>(edition, major_version_str, $key) {
                    Ok(data) => Some(data),
                    Err(McDataError::DataPathNotFound { .. }) | Err(McDataError::DataFileNotFound { .. }) => None,
                    Err(e) => return Err(e), // Propagate other errors
                }
            };
        }
        // Helper macro to load optional raw JSON Value
        macro_rules! load_optional_value {
            ($key:expr) => {
                match loader::load_data::<Value>(edition, major_version_str, $key) {
                    Ok(data) => Some(data),
                    Err(McDataError::DataPathNotFound { .. }) | Err(McDataError::DataFileNotFound { .. }) => None,
                    Err(e) => return Err(e), // Propagate other errors
                }
            };
        }


        // --- Load Raw Data ---
        // Mandatory (assuming these exist for most versions, adjust if needed)
        let blocks: Vec<Block> = loader::load_data(edition, major_version_str, "blocks")?;
        let items: Vec<Item> = loader::load_data(edition, major_version_str, "items")?;

        // Optional (provide default empty collections) - Use the macro
        let biomes: Vec<Biome> = load_optional!("biomes", Vec<Biome>).unwrap_or_default();
        let effects: Vec<Effect> = load_optional!("effects", Vec<Effect>).unwrap_or_default();
        let entities: Vec<Entity> = load_optional!("entities", Vec<Entity>).unwrap_or_default();
        let sounds: Vec<Sound> = load_optional!("sounds", Vec<Sound>).unwrap_or_default();
        let particles: Vec<Particle> = load_optional!("particles", Vec<Particle>).unwrap_or_default();
        let attributes: Vec<Attribute> = load_optional!("attributes", Vec<Attribute>).unwrap_or_default();
        let instruments: Vec<Instrument> = load_optional!("instruments", Vec<Instrument>).unwrap_or_default();
        let foods: Vec<Food> = load_optional!("foods", Vec<Food>).unwrap_or_default();
        let enchantments: Vec<Enchantment> = load_optional!("enchantments", Vec<Enchantment>).unwrap_or_default();
        let map_icons: Vec<MapIcon> = load_optional!("mapIcons", Vec<MapIcon>).unwrap_or_default();
        let windows: Vec<Window> = load_optional!("windows", Vec<Window>).unwrap_or_default();
        let block_loot: Vec<BlockLoot> = load_optional!("blockLoot", Vec<BlockLoot>).unwrap_or_default();
        let entity_loot: Vec<EntityLoot> = load_optional!("entityLoot", Vec<EntityLoot>).unwrap_or_default();

        // Optional structs/maps - Use the macro
        let block_collision_shapes: Option<BlockCollisionShapes> = load_optional!("blockCollisionShapes", BlockCollisionShapes);
        let tints: Option<Tints> = load_optional!("tints", Tints);
        let language: HashMap<String, String> = load_optional!("language", HashMap<String, String>).unwrap_or_default();

        // Optional raw values - Use the macro
        let recipes: Option<Value> = load_optional_value!("recipes");
        let materials: Option<Value> = load_optional_value!("materials");
        let commands: Option<Value> = load_optional_value!("commands");
        let protocol: Option<Value> = load_optional_value!("protocol");
        let protocol_comments: Option<Value> = load_optional_value!("protocolComments");
        let login_packet: Option<Value> = load_optional_value!("loginPacket");

        // Common data (loaded once, could be optimized but fine here for now)
        let legacy: Option<Legacy> = loader::load_data_from_path(
                &std::path::PathBuf::from(crate::constants::MINECRAFT_DATA_SUBMODULE_PATH)
                    .join(format!("data/{}/common/legacy.json", edition.path_prefix()))
            ).ok(); // Ignore errors for common data for now


        // --- Index Data ---
        let (blocks_by_id, blocks_by_name, blocks_by_state_id) = indexer::index_blocks(&blocks);
        let (items_by_id, items_by_name) = indexer::index_items(&items);
        let (biomes_by_id, biomes_by_name) = indexer::index_biomes(&biomes);
        let (effects_by_id, effects_by_name) = indexer::index_effects(&effects);
        let (entities_by_id, entities_by_name, mobs_by_id, objects_by_id) = indexer::index_entities(&entities);
        let (sounds_by_id, sounds_by_name) = indexer::index_sounds(&sounds);
        let (particles_by_id, particles_by_name) = indexer::index_particles(&particles);
        let (attributes_by_name, attributes_by_resource) = indexer::index_attributes(&attributes);
        let (instruments_by_id, instruments_by_name) = indexer::index_instruments(&instruments);
        let (foods_by_id, foods_by_name) = indexer::index_foods(&foods);
        let (enchantments_by_id, enchantments_by_name) = indexer::index_enchantments(&enchantments);
        let (map_icons_by_id, map_icons_by_name) = indexer::index_map_icons(&map_icons);
        let (windows_by_id, windows_by_name) = indexer::index_windows(&windows);
        let block_loot_by_name = indexer::index_block_loot(&block_loot);
        let entity_loot_by_name = indexer::index_entity_loot(&entity_loot);

        Ok(IndexedData {
            version,
            // Arrays
            blocks_array: Arc::new(blocks),
            items_array: Arc::new(items),
            biomes_array: Arc::new(biomes),
            effects_array: Arc::new(effects),
            entities_array: Arc::new(entities),
            sounds_array: Arc::new(sounds),
            particles_array: Arc::new(particles),
            attributes_array: Arc::new(attributes),
            instruments_array: Arc::new(instruments),
            foods_array: Arc::new(foods),
            enchantments_array: Arc::new(enchantments),
            map_icons_array: Arc::new(map_icons),
            windows_array: Arc::new(windows),
            block_loot_array: Arc::new(block_loot),
            entity_loot_array: Arc::new(entity_loot),
            // Indexed Maps
            blocks_by_id: Arc::new(blocks_by_id),
            blocks_by_name: Arc::new(blocks_by_name),
            blocks_by_state_id: Arc::new(blocks_by_state_id),
            items_by_id: Arc::new(items_by_id),
            items_by_name: Arc::new(items_by_name),
            biomes_by_id: Arc::new(biomes_by_id),
            biomes_by_name: Arc::new(biomes_by_name),
            effects_by_id: Arc::new(effects_by_id),
            effects_by_name: Arc::new(effects_by_name),
            entities_by_id: Arc::new(entities_by_id),
            entities_by_name: Arc::new(entities_by_name),
            mobs_by_id: Arc::new(mobs_by_id),
            objects_by_id: Arc::new(objects_by_id),
            sounds_by_id: Arc::new(sounds_by_id),
            sounds_by_name: Arc::new(sounds_by_name),
            particles_by_id: Arc::new(particles_by_id),
            particles_by_name: Arc::new(particles_by_name),
            attributes_by_name: Arc::new(attributes_by_name),
            attributes_by_resource: Arc::new(attributes_by_resource),
            instruments_by_id: Arc::new(instruments_by_id),
            instruments_by_name: Arc::new(instruments_by_name),
            foods_by_id: Arc::new(foods_by_id),
            foods_by_name: Arc::new(foods_by_name),
            enchantments_by_id: Arc::new(enchantments_by_id),
            enchantments_by_name: Arc::new(enchantments_by_name),
            map_icons_by_id: Arc::new(map_icons_by_id),
            map_icons_by_name: Arc::new(map_icons_by_name),
            windows_by_id: Arc::new(windows_by_id),
            windows_by_name: Arc::new(windows_by_name),
            block_loot_by_name: Arc::new(block_loot_by_name),
            entity_loot_by_name: Arc::new(entity_loot_by_name),
            // Other Data
            block_collision_shapes: Arc::new(block_collision_shapes),
            tints: Arc::new(tints),
            language: Arc::new(language),
            legacy: Arc::new(legacy),
            // Raw Values
            recipes: Arc::new(recipes),
            materials: Arc::new(materials),
            commands: Arc::new(commands),
            protocol: Arc::new(protocol),
            protocol_comments: Arc::new(protocol_comments),
            login_packet: Arc::new(login_packet),
        })
    }

    /// Checks if the current version is newer than or equal to the other version string.
    pub fn is_newer_or_equal_to(&self, other_version_str: &str) -> Result<bool, McDataError> {
        let other_version = crate::version::resolve_version(other_version_str)?;
        // Ensure comparison happens only within the same edition
        if self.version.edition == other_version.edition {
            Ok(self.version >= other_version)
        } else {
            Err(McDataError::Internal(format!(
                "Cannot compare versions from different editions: {:?} and {:?}",
                self.version.edition, other_version.edition
            )))
        }
    }

    /// Checks if the current version is older than the other version string.
     pub fn is_older_than(&self, other_version_str: &str) -> Result<bool, McDataError> {
         let other_version = crate::version::resolve_version(other_version_str)?;
          // Ensure comparison happens only within the same edition
         if self.version.edition == other_version.edition {
             Ok(self.version < other_version)
         } else {
             Err(McDataError::Internal(format!(
                 "Cannot compare versions from different editions: {:?} and {:?}",
                 self.version.edition, other_version.edition
             )))
         }
     }

    /// Checks support for a feature, returning the feature's value (often boolean).
    pub fn support_feature(&self, feature_name: &str) -> Result<Value, McDataError> {
        features::get_feature_support(&self.version, feature_name)
    }

    // Convenience methods to get specific data by name/id could be added here
    // e.g., pub fn block_by_name(&self, name: &str) -> Option<&Block> { self.blocks_by_name.get(name) }
    // pub fn food_by_name(&self, name: &str) -> Option<&Food> { self.foods_by_name.get(name) }
    // pub fn particle_by_id(&self, id: u32) -> Option<&Particle> { self.particles_by_id.get(&id) }
}