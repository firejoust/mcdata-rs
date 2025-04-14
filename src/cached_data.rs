// src/cached_data.rs

use crate::error::McDataError;
use crate::structs::{Block, Item, Biome, Effect, Entity, /* add others */};
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

    // Indexed Data (add more fields as needed)
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

    // Raw Data (optional, if direct access is needed)
    // pub raw_recipes: Option<serde_json::Value>,
    // pub raw_protocol: Option<serde_json::Value>,

    // Language map
    pub language: Arc<HashMap<String, String>>,
}

impl IndexedData {
    /// Loads and indexes all data for the given canonical version.
    pub fn load(version: Version) -> Result<Self, McDataError> {
        // --- Load Raw Data ---
        // Use major_version for loading paths as per node-minecraft-data logic
        let major_version_str = &version.major_version;
        let edition = version.edition;

        // Load mandatory data
        let blocks: Vec<Block> = loader::load_data(edition, major_version_str, "blocks")?;
        let items: Vec<Item> = loader::load_data(edition, major_version_str, "items")?;

        // Load optional data directly, providing default values if not found
        let biomes: Vec<Biome> = loader::load_data(edition, major_version_str, "biomes")
            .or_else(|e| match e {
                McDataError::DataPathNotFound { .. } | McDataError::DataFileNotFound { .. } => Ok(Vec::new()), // Default empty vec
                _ => Err(e),
            })?;
        let effects: Vec<Effect> = loader::load_data(edition, major_version_str, "effects")
             .or_else(|e| match e {
                 McDataError::DataPathNotFound { .. } | McDataError::DataFileNotFound { .. } => Ok(Vec::new()), // Default empty vec
                 _ => Err(e),
             })?;
        let entities: Vec<Entity> = loader::load_data(edition, major_version_str, "entities")
             .or_else(|e| match e {
                 McDataError::DataPathNotFound { .. } | McDataError::DataFileNotFound { .. } => Ok(Vec::new()), // Default empty vec
                 _ => Err(e),
             })?;
        let language: HashMap<String, String> = loader::load_data(edition, major_version_str, "language")
             .or_else(|e| match e {
                 McDataError::DataPathNotFound { .. } | McDataError::DataFileNotFound { .. } => Ok(HashMap::new()), // Default empty map
                 _ => Err(e),
             })?;
        // Load others: recipes, protocol, foods, particles, etc. similarly

        // --- Index Data ---
        let (blocks_by_id, blocks_by_name, blocks_by_state_id) = indexer::index_blocks(&blocks);
        let (items_by_id, items_by_name) = indexer::index_items(&items);
        let (biomes_by_id, biomes_by_name) = indexer::index_biomes(&biomes);
        let (effects_by_id, effects_by_name) = indexer::index_effects(&effects);
        let (entities_by_id, entities_by_name, mobs_by_id, objects_by_id) = indexer::index_entities(&entities);
        // Index others...

        Ok(IndexedData {
            version,
            blocks_array: Arc::new(blocks),
            blocks_by_id: Arc::new(blocks_by_id),
            blocks_by_name: Arc::new(blocks_by_name),
            blocks_by_state_id: Arc::new(blocks_by_state_id),
            items_array: Arc::new(items),
            items_by_id: Arc::new(items_by_id),
            items_by_name: Arc::new(items_by_name),
            biomes_array: Arc::new(biomes),
            biomes_by_id: Arc::new(biomes_by_id),
            biomes_by_name: Arc::new(biomes_by_name),
            effects_array: Arc::new(effects),
            effects_by_id: Arc::new(effects_by_id),
            effects_by_name: Arc::new(effects_by_name),
            entities_array: Arc::new(entities),
            entities_by_id: Arc::new(entities_by_id),
            entities_by_name: Arc::new(entities_by_name),
            mobs_by_id: Arc::new(mobs_by_id),
            objects_by_id: Arc::new(objects_by_id),
            language: Arc::new(language),
            // Assign others...
        })
    }

    /// Checks if the current version is newer than or equal to the other version string.
    pub fn is_newer_or_equal_to(&self, other_version_str: &str) -> Result<bool, McDataError> {
        let other_version = crate::version::resolve_version(other_version_str)?;
        Ok(self.version >= other_version)
    }

    /// Checks if the current version is older than the other version string.
     pub fn is_older_than(&self, other_version_str: &str) -> Result<bool, McDataError> {
         let other_version = crate::version::resolve_version(other_version_str)?;
         Ok(self.version < other_version)
     }

    /// Checks support for a feature, returning the feature's value (often boolean).
    pub fn support_feature(&self, feature_name: &str) -> Result<Value, McDataError> {
        features::get_feature_support(&self.version, feature_name)
    }

    // Convenience methods to get specific data by name/id could be added here
    // e.g., pub fn block_by_name(&self, name: &str) -> Option<&Block> { self.blocks_by_name.get(name) }
}