use crate::data_source;
use crate::error::McDataError;
use crate::features;
use crate::indexer;
use crate::loader;
use crate::structs::*;
use crate::version::Version;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Holds all loaded and indexed Minecraft data for a specific version.
///
/// Instances of this struct are cached globally by the `mc_data` function.
/// Fields are wrapped in `Arc` to allow cheap cloning when retrieving from the cache.
#[derive(Debug, Clone)]
pub struct IndexedData {
    /// The canonical `Version` struct this data corresponds to.
    pub version: Version,

    // Indexed data structures for quick lookups.

    // Blocks
    pub blocks_array: Arc<Vec<Block>>,
    pub blocks_by_id: Arc<HashMap<u32, Block>>,
    pub blocks_by_name: Arc<HashMap<String, Block>>,
    pub blocks_by_state_id: Arc<HashMap<u32, Block>>,

    // Items
    pub items_array: Arc<Vec<Item>>,
    pub items_by_id: Arc<HashMap<u32, Item>>,
    pub items_by_name: Arc<HashMap<String, Item>>,

    // Biomes
    pub biomes_array: Arc<Vec<Biome>>,
    pub biomes_by_id: Arc<HashMap<u32, Biome>>,
    pub biomes_by_name: Arc<HashMap<String, Biome>>,

    // Effects (Status Effects)
    pub effects_array: Arc<Vec<Effect>>,
    pub effects_by_id: Arc<HashMap<u32, Effect>>,
    pub effects_by_name: Arc<HashMap<String, Effect>>,

    // Entities
    pub entities_array: Arc<Vec<Entity>>,
    pub entities_by_id: Arc<HashMap<u32, Entity>>,
    pub entities_by_name: Arc<HashMap<String, Entity>>,
    pub mobs_by_id: Arc<HashMap<u32, Entity>>, // Filtered index for entities of type "mob"
    pub objects_by_id: Arc<HashMap<u32, Entity>>, // Filtered index for entities of type "object"

    // Sounds
    pub sounds_array: Arc<Vec<Sound>>,
    pub sounds_by_id: Arc<HashMap<u32, Sound>>,
    pub sounds_by_name: Arc<HashMap<String, Sound>>,

    // Particles
    pub particles_array: Arc<Vec<Particle>>,
    pub particles_by_id: Arc<HashMap<u32, Particle>>,
    pub particles_by_name: Arc<HashMap<String, Particle>>,

    // Attributes
    pub attributes_array: Arc<Vec<Attribute>>,
    pub attributes_by_name: Arc<HashMap<String, Attribute>>,
    pub attributes_by_resource: Arc<HashMap<String, Attribute>>, // Index by namespaced key

    // Instruments (Note Block sounds)
    pub instruments_array: Arc<Vec<Instrument>>,
    pub instruments_by_id: Arc<HashMap<u32, Instrument>>,
    pub instruments_by_name: Arc<HashMap<String, Instrument>>,

    // Foods
    pub foods_array: Arc<Vec<Food>>,
    pub foods_by_id: Arc<HashMap<u32, Food>>,
    pub foods_by_name: Arc<HashMap<String, Food>>,

    // Enchantments
    pub enchantments_array: Arc<Vec<Enchantment>>,
    pub enchantments_by_id: Arc<HashMap<u32, Enchantment>>,
    pub enchantments_by_name: Arc<HashMap<String, Enchantment>>,

    // Map Icons
    pub map_icons_array: Arc<Vec<MapIcon>>,
    pub map_icons_by_id: Arc<HashMap<u32, MapIcon>>,
    pub map_icons_by_name: Arc<HashMap<String, MapIcon>>,

    // Windows (Containers/GUIs)
    pub windows_array: Arc<Vec<Window>>,
    pub windows_by_id: Arc<HashMap<String, Window>>, // Index by ID (string, potentially namespaced)
    pub windows_by_name: Arc<HashMap<String, Window>>,

    // Block Loot Tables
    pub block_loot_array: Arc<Vec<BlockLoot>>,
    pub block_loot_by_name: Arc<HashMap<String, BlockLoot>>, // Index by block name

    // Entity Loot Tables
    pub entity_loot_array: Arc<Vec<EntityLoot>>,
    pub entity_loot_by_name: Arc<HashMap<String, EntityLoot>>, // Index by entity name

    // Indexed Block Collision Shapes
    pub block_shapes_by_state_id: Arc<HashMap<u32, Vec<[f64; 6]>>>, // Map stateId -> BoundingBoxes
    pub block_shapes_by_name: Arc<HashMap<String, Vec<[f64; 6]>>>, // Map blockName -> Default State BoundingBoxes

    // Less structured or version-dependent data.
    /// Raw data from blockCollisionShapes.json, if available for the version.
    pub block_collision_shapes_raw: Arc<Option<BlockCollisionShapes>>,
    /// Data from tints.json, if available.
    pub tints: Arc<Option<Tints>>,
    /// Data from language.json (typically en_us), if available.
    pub language: Arc<HashMap<String, String>>,
    /// Data from legacy.json (mapping old IDs to new), if available.
    pub legacy: Arc<Option<Legacy>>,

    // Raw JSON values for data types that vary significantly across versions
    // or are too complex to represent with stable structs easily.
    pub recipes: Arc<Option<Value>>,
    pub materials: Arc<Option<Value>>,
    pub commands: Arc<Option<Value>>,
    pub protocol: Arc<Option<Value>>, // Raw protocol.json content
    pub protocol_comments: Arc<Option<Value>>, // Raw protocolComments.json content
    pub login_packet: Arc<Option<Value>>, // Raw loginPacket.json content
}

impl IndexedData {
    /// Loads all required and optional data files for the given canonical version,
    /// then indexes them into the `IndexedData` struct fields.
    pub fn load(version: Version) -> Result<Self, McDataError> {
        log::info!(
            "Loading and indexing data for version: {} ({:?})",
            version.minecraft_version,
            version.edition
        );
        let major_version_str = &version.major_version; // Use major version for path lookups
        let edition = version.edition;

        // Helper macro to load optional data of a specific type.
        // Handles "file not found" errors gracefully by returning None.
        // Propagates other errors (e.g., parse errors).
        macro_rules! load_optional {
            ($key:expr, $type:ty) => {
                match loader::load_data::<$type>(edition, major_version_str, $key) {
                    Ok(data) => {
                        log::trace!("Successfully loaded optional data for key '{}'", $key);
                        Some(data)
                    }
                    // Treat path/file not found as expected for optional data.
                    Err(McDataError::DataPathNotFound { .. })
                    | Err(McDataError::DataFileNotFound { .. }) => {
                        log::trace!("Optional data key '{}' not found for this version.", $key);
                        None
                    }
                    // Propagate other errors (I/O, JSON parsing, etc.).
                    Err(e) => {
                        log::error!("Error loading optional data for key '{}': {}", $key, e);
                        return Err(e);
                    }
                }
            };
        }
        // Helper macro similar to load_optional!, but for loading raw `serde_json::Value`.
        macro_rules! load_optional_value {
            ($key:expr) => {
                match loader::load_data::<Value>(edition, major_version_str, $key) {
                    Ok(data) => {
                        log::trace!("Successfully loaded optional value for key '{}'", $key);
                        Some(data)
                    }
                    Err(McDataError::DataPathNotFound { .. })
                    | Err(McDataError::DataFileNotFound { .. }) => {
                        log::trace!("Optional value key '{}' not found for this version.", $key);
                        None
                    }
                    Err(e) => {
                        log::error!("Error loading optional value for key '{}': {}", $key, e);
                        return Err(e);
                    }
                }
            };
        }

        // --- Load Raw Data Arrays/Maps ---
        // Load required data types (expect them to exist for any valid version).
        let blocks: Vec<Block> = loader::load_data(edition, major_version_str, "blocks")?;
        let items: Vec<Item> = loader::load_data(edition, major_version_str, "items")?;

        // Load optional data types using the helper macro. Use `unwrap_or_default` for Vec/HashMap.
        let biomes: Vec<Biome> = load_optional!("biomes", Vec<Biome>).unwrap_or_default();
        let effects: Vec<Effect> = load_optional!("effects", Vec<Effect>).unwrap_or_default();
        let entities: Vec<Entity> = load_optional!("entities", Vec<Entity>).unwrap_or_default();
        let sounds: Vec<Sound> = load_optional!("sounds", Vec<Sound>).unwrap_or_default();
        let particles: Vec<Particle> =
            load_optional!("particles", Vec<Particle>).unwrap_or_default();
        let attributes: Vec<Attribute> =
            load_optional!("attributes", Vec<Attribute>).unwrap_or_default();
        let instruments: Vec<Instrument> =
            load_optional!("instruments", Vec<Instrument>).unwrap_or_default();
        let foods: Vec<Food> = load_optional!("foods", Vec<Food>).unwrap_or_default();
        let enchantments: Vec<Enchantment> =
            load_optional!("enchantments", Vec<Enchantment>).unwrap_or_default();
        let map_icons: Vec<MapIcon> = load_optional!("mapIcons", Vec<MapIcon>).unwrap_or_default();
        let windows: Vec<Window> = load_optional!("windows", Vec<Window>).unwrap_or_default();
        let block_loot: Vec<BlockLoot> =
            load_optional!("blockLoot", Vec<BlockLoot>).unwrap_or_default();
        let entity_loot: Vec<EntityLoot> =
            load_optional!("entityLoot", Vec<EntityLoot>).unwrap_or_default();
        let block_collision_shapes_raw: Option<BlockCollisionShapes> =
            load_optional!("blockCollisionShapes", BlockCollisionShapes);
        let tints: Option<Tints> = load_optional!("tints", Tints);
        let language: HashMap<String, String> =
            load_optional!("language", HashMap<String, String>).unwrap_or_default();

        // Load optional raw JSON values.
        let recipes: Option<Value> = load_optional_value!("recipes");
        let materials: Option<Value> = load_optional_value!("materials");
        let commands: Option<Value> = load_optional_value!("commands");
        let protocol: Option<Value> = load_optional_value!("protocol");
        let protocol_comments: Option<Value> = load_optional_value!("protocolComments");
        let login_packet: Option<Value> = load_optional_value!("loginPacket");

        // Load legacy.json (common data, path constructed differently from versioned data).
        let legacy: Option<Legacy> = {
            match data_source::get_data_root() {
                Ok(data_root) => {
                    let legacy_path =
                        data_root.join(format!("{}/common/legacy.json", edition.path_prefix()));
                    match loader::load_data_from_path(&legacy_path) {
                        Ok(data) => {
                            log::trace!("Successfully loaded legacy.json for {:?}", edition);
                            Some(data)
                        }
                        // File not found is expected if legacy.json doesn't exist for the edition.
                        Err(McDataError::IoError { source, .. })
                            if source.kind() == std::io::ErrorKind::NotFound =>
                        {
                            log::trace!("legacy.json not found for {:?}", edition);
                            None
                        }
                        // Log other errors but treat them as non-fatal for legacy data.
                        Err(e) => {
                            log::warn!("Failed to load legacy.json for {:?}: {}", edition, e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Could not get data root to load legacy.json: {}", e);
                    None // Cannot load legacy if data root isn't available.
                }
            }
        };

        // --- Index Loaded Data ---
        log::debug!("Indexing loaded data...");
        let (blocks_by_id, blocks_by_name, blocks_by_state_id) = indexer::index_blocks(&blocks);
        let (items_by_id, items_by_name) = indexer::index_items(&items);
        let (biomes_by_id, biomes_by_name) = indexer::index_biomes(&biomes);
        let (effects_by_id, effects_by_name) = indexer::index_effects(&effects);
        let (entities_by_id, entities_by_name, mobs_by_id, objects_by_id) =
            indexer::index_entities(&entities);
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

        // Index block collision shapes if the raw data was loaded successfully.
        let (block_shapes_by_state_id, block_shapes_by_name) = if let Some(ref collision_data) =
            block_collision_shapes_raw
        {
            indexer::index_block_shapes(&blocks_by_state_id, &blocks_by_name, collision_data)
        } else {
            // Return empty maps if collision data doesn't exist for this version.
            log::debug!(
                "No blockCollisionShapes data found for this version, block shapes will be empty."
            );
            (HashMap::new(), HashMap::new())
        };

        log::info!(
            "Finished loading and indexing data for {} ({:?})",
            version.minecraft_version,
            version.edition
        );

        // Construct the final IndexedData struct, wrapping fields in Arc.
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
            // Indexed Shapes
            block_shapes_by_state_id: Arc::new(block_shapes_by_state_id),
            block_shapes_by_name: Arc::new(block_shapes_by_name),
            // Other Data
            block_collision_shapes_raw: Arc::new(block_collision_shapes_raw),
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

    /// Checks if the current data's version is newer than or equal to another version string.
    ///
    /// Resolves the `other_version_str` and compares using the `Version` struct's `Ord` implementation.
    ///
    /// # Errors
    /// Returns `McDataError::InvalidVersion` if `other_version_str` is invalid.
    /// Returns `McDataError::Internal` if attempting to compare versions from different editions.
    pub fn is_newer_or_equal_to(&self, other_version_str: &str) -> Result<bool, McDataError> {
        let other_version = crate::version::resolve_version(other_version_str)?;
        // Ensure comparison happens only within the same edition.
        if self.version.edition == other_version.edition {
            Ok(self.version >= other_version) // Uses the Ord implementation for Version
        } else {
            Err(McDataError::Internal(format!(
                "Cannot compare versions from different editions: {:?} ({}) and {:?} ({})",
                self.version.edition,
                self.version.minecraft_version,
                other_version.edition,
                other_version.minecraft_version
            )))
        }
    }

    /// Checks if the current data's version is strictly older than another version string.
    ///
    /// Resolves the `other_version_str` and compares using the `Version` struct's `Ord` implementation.
    ///
    /// # Errors
    /// Returns `McDataError::InvalidVersion` if `other_version_str` is invalid.
    /// Returns `McDataError::Internal` if attempting to compare versions from different editions.
    pub fn is_older_than(&self, other_version_str: &str) -> Result<bool, McDataError> {
        let other_version = crate::version::resolve_version(other_version_str)?;
        // Ensure comparison happens only within the same edition.
        if self.version.edition == other_version.edition {
            Ok(self.version < other_version) // Uses the Ord implementation for Version
        } else {
            Err(McDataError::Internal(format!(
                "Cannot compare versions from different editions: {:?} ({}) and {:?} ({})",
                self.version.edition,
                self.version.minecraft_version,
                other_version.edition,
                other_version.minecraft_version
            )))
        }
    }

    /// Checks support for a named feature based on the current data's version.
    ///
    /// Consults the `features.json` data and returns the feature's value (often boolean,
    /// but can be other JSON types) if supported for this version, or `Value::Bool(false)` otherwise.
    ///
    /// # Errors
    /// Returns `McDataError` if feature data or version information cannot be loaded or resolved.
    pub fn support_feature(&self, feature_name: &str) -> Result<Value, McDataError> {
        features::get_feature_support(&self.version, feature_name)
    }
}
