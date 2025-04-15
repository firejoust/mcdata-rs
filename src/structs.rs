use serde::Deserialize;
use std::collections::HashMap;

// Structs related to version information from protocolVersions.json.

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersionInfo {
    pub minecraft_version: String,
    pub version: i32,              // Protocol version number
    pub data_version: Option<i32>, // Data version number (used for comparisons)
    pub uses_netty: bool,
    pub major_version: String,
    #[serde(default = "default_release_type")]
    pub release_type: String, // e.g., "release", "snapshot"
}

fn default_release_type() -> String {
    "release".to_string()
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub version: i32,
    pub minecraft_version: String,
    pub major_version: String,
    #[serde(default = "default_release_type")]
    pub release_type: String,
}

// Structs representing various game data elements loaded from JSON files.

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub hardness: Option<f32>,
    pub resistance: f32,
    pub stack_size: u32,
    pub diggable: bool,
    pub bounding_box: String, // Typically "block" or "empty"
    pub material: Option<String>,
    #[serde(default)]
    pub harvest_tools: HashMap<String, bool>,
    #[serde(default)]
    pub variations: Option<Vec<BlockVariation>>,
    #[serde(default)]
    pub drops: Vec<BlockDrop>, // Handles simple ID drops or complex drop definitions
    #[serde(default)]
    pub emit_light: u8,
    #[serde(default)]
    pub filter_light: u8,
    #[serde(default)]
    pub transparent: bool,
    #[serde(default)]
    pub states: Vec<BlockStateDefinition>, // Block states (relevant for 1.13+)

    // State IDs might be calculated during indexing if not present in the source JSON.
    #[serde(default = "default_state_id")]
    pub min_state_id: u32,
    #[serde(default = "default_state_id")]
    pub max_state_id: u32,
    #[serde(default = "default_state_id")]
    pub default_state: u32,

    // Internal map added during indexing for quick state ID lookup.
    #[serde(skip)]
    pub state_id_map: Option<HashMap<u32, Block>>,
}

// Default value for state IDs before calculation.
fn default_state_id() -> u32 {
    0
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockVariation {
    pub metadata: u32,
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockStateDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String, // e.g., "bool", "enum", "int"
    pub num_values: Option<u32>,
    #[serde(default)]
    pub values: Vec<String>,
}

// Structs for handling block drops, especially in older formats (pre-1.13)
// where drops could be simple item IDs or more complex objects.

#[derive(Deserialize, Debug, Clone)]
pub struct DropItem {
    pub id: u32,
    pub metadata: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)] // Allows deserializing as either a simple u32 or a DropItem object.
pub enum DropType {
    Id(u32),
    Item(DropItem),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DropElement {
    pub drop: DropType, // The actual item dropped.
    pub min_count: Option<f32>,
    pub max_count: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)] // Allows deserializing as either a simple u32 ID or a DropElement object.
pub enum BlockDrop {
    Id(u32),
    Element(DropElement),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub stack_size: u32,
    #[serde(default)]
    pub enchant_categories: Option<Vec<String>>,
    #[serde(default)]
    pub repair_with: Option<Vec<String>>,
    #[serde(default)]
    pub max_durability: Option<u32>,
    #[serde(default)]
    pub variations: Option<Vec<ItemVariation>>, // Relevant for older versions with metadata variations.
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemVariation {
    pub metadata: u32,
    pub display_name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Biome {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub temperature: f32,
    pub precipitation: Option<String>, // e.g., "none", "rain", "snow"
    pub dimension: String,             // e.g., "overworld", "nether", "end"
    pub display_name: String,
    pub color: i32,
    pub rainfall: Option<f32>,
    #[serde(default)]
    pub depth: Option<f32>,
    #[serde(default)]
    pub has_precipitation: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub effect_type: String, // Typically "good" or "bad"
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: u32,
    pub internal_id: Option<u32>, // Used in some older versions.
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub entity_type: String, // e.g., "mob", "object", "projectile"
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub category: Option<String>,
    #[serde(default)]
    pub metadata_keys: Vec<String>, // Relevant for 1.20.2+
}

// Structs for feature checking from features.json.

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub values: Vec<FeatureValue>, // Prioritized if present.
    pub version: Option<String>, // Used if `values` is empty.
    #[serde(default)]
    pub versions: Vec<String>, // Used if `values` and `version` are empty; expected [min, max].
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeatureValue {
    pub value: serde_json::Value, // The actual feature value (bool, string, number).
    pub version: Option<String>,  // Single version applicability.
    #[serde(default)]
    pub versions: Vec<String>, // Version range applicability [min, max].
}

// Struct for dataPaths.json, mapping versions and keys to file paths.
#[derive(Deserialize, Debug, Clone)]
pub struct DataPaths {
    // Major Version -> Data Key -> Path Suffix
    pub pc: HashMap<String, HashMap<String, String>>,
    pub bedrock: HashMap<String, HashMap<String, String>>,
}

// Other miscellaneous data structs.

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)] // Allows deserializing as a single index or multiple indices.
pub enum BlockShapeRef {
    Single(u32),        // Single shape index for all states.
    Multiple(Vec<u32>), // Shape indices per state/metadata.
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockCollisionShapes {
    // Maps block name to its shape reference (single index or list of indices).
    pub blocks: HashMap<String, BlockShapeRef>,
    // Maps shape index (as string key) to an array of bounding boxes ([x1, y1, z1, x2, y2, z2]).
    pub shapes: HashMap<String, Vec<[f64; 6]>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Particle {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub name: String,
    pub resource: String, // Namespaced key
    #[serde(default)]
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockLoot {
    pub block: String, // Block name
    pub drops: Vec<BlockLootDrop>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockLootDrop {
    pub item: String, // Item name
    #[serde(default = "default_drop_chance")]
    pub drop_chance: f32,
    #[serde(default = "default_stack_size_range")]
    pub stack_size_range: Vec<Option<i32>>, // Range [min] or [min, max]
    #[serde(default)]
    pub silk_touch: Option<bool>,
    #[serde(default)]
    pub no_silk_touch: Option<bool>,
    #[serde(default)]
    pub block_age: Option<i32>,
}

fn default_drop_chance() -> f32 {
    1.0
}

// Default stack size range is [1].
fn default_stack_size_range() -> Vec<Option<i32>> {
    vec![Some(1)]
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String, // Can be numeric string or namespaced string (e.g., "minecraft:chest")
    pub name: String,
    #[serde(default)]
    pub slots: Vec<WindowSlot>,
    #[serde(default)]
    pub opened_with: Vec<WindowOpenedWith>,
    #[serde(default)]
    pub properties: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowSlot {
    pub name: String,
    pub index: u32,
    pub size: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowOpenedWith {
    #[serde(rename = "type")]
    pub opener_type: String, // e.g., "block", "entity"
    pub id: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityLoot {
    pub entity: String, // Entity name
    pub drops: Vec<EntityLootDrop>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityLootDrop {
    pub item: String, // Item name
    #[serde(default = "default_drop_chance")]
    pub drop_chance: f32,
    #[serde(default = "default_entity_stack_size_range")]
    pub stack_size_range: Vec<u32>, // Range [min] or [min, max]
    #[serde(default)]
    pub player_kill: Option<bool>,
}

// Default stack size range is [1].
fn default_entity_stack_size_range() -> Vec<u32> {
    vec![1]
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Food {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub stack_size: u32,
    pub food_points: f32,
    pub saturation: f32,
    pub effective_quality: f32,
    pub saturation_ratio: f32,
    #[serde(default)]
    pub variations: Option<Vec<ItemVariation>>, // Relevant for older versions.
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Enchantment {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    #[serde(rename = "maxLevel")]
    pub max_level: u32,
    #[serde(default)]
    pub min_cost: EnchantmentCost,
    #[serde(default)]
    pub max_cost: EnchantmentCost,
    #[serde(default)]
    pub treasure_only: bool,
    #[serde(default)]
    pub curse: bool,
    #[serde(default)]
    pub exclude: Vec<String>, // Names of mutually exclusive enchantments
    pub category: String, // e.g., "weapon", "armor"
    pub weight: u32,      // Rarity weight
    #[serde(default)]
    pub tradeable: bool,
    #[serde(default)]
    pub discoverable: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnchantmentCost {
    // Cost calculation parameters: cost = a * level + b
    pub a: i32,
    pub b: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MapIcon {
    pub id: u32,
    pub name: String,
    pub appearance: Option<String>,
    #[serde(default)]
    pub visible_in_item_frame: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tints {
    pub grass: TintData,
    pub foliage: TintData,
    pub water: TintData,
    pub redstone: TintData,
    pub constant: TintData,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TintData {
    #[serde(default)]
    pub default: Option<i32>, // Default color value
    pub data: Vec<TintDatum>, // List of specific tint rules
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TintDatum {
    // Keys can be biome names (string) or redstone levels (number).
    pub keys: Vec<serde_json::Value>,
    pub color: i32, // The tint color associated with these keys.
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Legacy {
    // Maps legacy numeric IDs (as strings) to modern namespaced IDs.
    pub blocks: HashMap<String, String>,
    pub items: HashMap<String, String>,
}

// Note: Commands and Materials are often loaded as raw `serde_json::Value`
// due to their high variability across versions.
