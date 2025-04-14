// src/structs.rs

use serde::Deserialize;
use std::collections::HashMap;

// --- Versioning Structs ---

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersionInfo {
    pub minecraft_version: String,
    pub version: i32, // Protocol version number
    pub data_version: Option<i32>, // Optional, calculated later if missing
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
    // data_version might be added dynamically if needed
}

// --- Game Data Structs ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub hardness: Option<f32>, // Made optional
    pub resistance: f32,
    pub stack_size: u32,
    pub diggable: bool,
    pub bounding_box: String, // "block" or "empty" - Consider Enum later if stable
    pub material: Option<String>, // Varies too much for a strict enum across versions
    #[serde(default)]
    pub harvest_tools: HashMap<String, bool>,
    #[serde(default)]
    pub variations: Option<Vec<BlockVariation>>,
    #[serde(default)]
    pub drops: Vec<BlockDrop>, // Use the enum to handle simple/complex drops
    #[serde(default)]
    pub emit_light: u8,
    #[serde(default)]
    pub filter_light: u8,
    #[serde(default)]
    pub transparent: bool,
    #[serde(default)]
    pub states: Vec<BlockStateDefinition>, // Added from 1.13+ blocks

    // These might be added dynamically for older versions or loaded if present
    #[serde(default = "default_state_id")]
    pub min_state_id: u32,
    #[serde(default = "default_state_id")]
    pub max_state_id: u32,
    #[serde(default = "default_state_id")]
    pub default_state: u32,

    // Added during indexing if needed
    #[serde(skip)]
    pub state_id_map: Option<HashMap<u32, Block>>,
}

fn default_state_id() -> u32 { 0 } // Placeholder, calculated later

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
    pub state_type: String, // "bool", "enum", "int"
    pub num_values: Option<u32>,
    #[serde(default)]
    pub values: Vec<String>,
}

// --- Drop Structs (for older block formats like 1.8, 1.12) ---
// Needed because `drops` can be `Vec<u32>` or `Vec<DropElement>`

#[derive(Deserialize, Debug, Clone)]
pub struct DropItem {
    pub id: u32,
    pub metadata: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)] // Allows deserializing as either a u32 or a DropItem object
pub enum DropType {
    Id(u32),
    Item(DropItem),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DropElement {
    pub drop: DropType, // The actual item dropped (can be simple ID or complex)
    pub min_count: Option<f32>, // Use f32 as seen in JSON, handle potential float values
    pub max_count: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)] // Allows deserializing as either a simple u32 ID or a DropElement object
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
    pub enchant_categories: Option<Vec<String>>, // Renamed from TS, varies
    #[serde(default)]
    pub repair_with: Option<Vec<String>>,
    #[serde(default)]
    pub max_durability: Option<u32>,
    #[serde(default)]
    pub variations: Option<Vec<ItemVariation>>, // Added for older versions
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
    pub precipitation: Option<String>, // Varies ("none", "rain", "snow")
    pub dimension: String, // "overworld", "nether", "end"
    pub display_name: String,
    pub color: i32,
    pub rainfall: Option<f32>, // Optional in some versions
    #[serde(default)]
    pub depth: Option<f32>, // Optional
    #[serde(default)]
    pub has_precipitation: Option<bool>, // Added in later versions
    // Removed child, climates, parent as they are less common/complex
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Effect {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub effect_type: String, // "good" or "bad"
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: u32,
    pub internal_id: Option<u32>, // Older versions might use this
    pub name: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub entity_type: String, // "mob", "object", "projectile", etc. - Consider Enum later
    pub width: Option<f32>, // Optional in some versions
    pub height: Option<f32>, // Optional in some versions
    pub category: Option<String>, // Optional
    #[serde(default)]
    pub metadata_keys: Vec<String>, // Added in 1.20.2+
}

// --- Feature Checking Structs ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub name: String,
    pub description: Option<String>, // Made optional
    #[serde(default)]
    pub values: Vec<FeatureValue>, // If present, use this
    pub version: Option<String>,   // If present (and values is empty), use this
    #[serde(default)]
    pub versions: Vec<String>, // If present (and others are empty), use this [min, max]
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeatureValue {
    pub value: serde_json::Value, // Can be bool, string, number
    pub version: Option<String>,
    #[serde(default)]
    pub versions: Vec<String>, // [min, max]
}

// --- Data Paths Struct ---
#[derive(Deserialize, Debug, Clone)]
pub struct DataPaths {
    pub pc: HashMap<String, HashMap<String, String>>,
    pub bedrock: HashMap<String, HashMap<String, String>>,
}

// --- New Structs ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockCollisionShapes {
    // Block name -> Shape index/indices
    pub blocks: HashMap<String, serde_json::Value>, // Use Value due to number | number[] variation
    // Shape index (as string) -> Array of bounding boxes ([x1, y1, z1, x2, y2, z2])
    pub shapes: HashMap<String, Vec<[f64; 6]>>, // Assuming f64 for precision
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
    pub resource: String,
    #[serde(default)] // Default might be missing in older versions
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
    pub block: String,
    pub drops: Vec<BlockLootDrop>,
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockLootDrop {
    pub item: String,
    #[serde(default = "default_drop_chance")]
    pub drop_chance: f32,
    // CHANGE THE INNER TYPE TO Option<i32>
    #[serde(default = "default_stack_size_range")]
    pub stack_size_range: Vec<Option<i32>>,
    #[serde(default)]
    pub silk_touch: Option<bool>,
    #[serde(default)]
    pub no_silk_touch: Option<bool>,
    #[serde(default)]
    pub block_age: Option<i32>, // Keep as i32
}

fn default_drop_chance() -> f32 { 1.0 }

// CHANGE THE RETURN TYPE TO Vec<Option<i32>>
fn default_stack_size_range() -> Vec<Option<i32>> {
    vec![Some(1)] // Return Some(1) which is valid for i32
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String, // Can be numeric string or namespaced string
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
    pub opener_type: String,
    pub id: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityLoot {
    pub entity: String,
    pub drops: Vec<EntityLootDrop>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntityLootDrop {
    pub item: String,
    #[serde(default = "default_drop_chance")]
    pub drop_chance: f32,
    // This field expects Vec<u32> based on TS definition
    #[serde(default = "default_entity_stack_size_range")] // Use a different default fn name
    pub stack_size_range: Vec<u32>,
    #[serde(default)]
    pub player_kill: Option<bool>,
}

// Corrected default function for EntityLootDrop
fn default_entity_stack_size_range() -> Vec<u32> {
    vec![1] // Return Vec<u32>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Food {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub stack_size: u32,
    pub food_points: f32, // Can be float
    pub saturation: f32, // Can be float
    pub effective_quality: f32, // Can be float
    pub saturation_ratio: f32, // Can be float
    #[serde(default)]
    pub variations: Option<Vec<ItemVariation>>, // Added for older versions
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Enchantment {
    pub id: u32, // ID is usually non-negative
    pub name: String,
    pub display_name: String,
    #[serde(rename = "maxLevel")]
    pub max_level: u32, // Max level is usually non-negative
    #[serde(default)]
    pub min_cost: EnchantmentCost, // Uses the struct below
    #[serde(default)]
    pub max_cost: EnchantmentCost, // Uses the struct below
    #[serde(default)]
    pub treasure_only: bool,
    #[serde(default)]
    pub curse: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
    pub category: String,
    pub weight: u32, // Weight seems likely to be positive, keep as u32 for now unless errors point here
    #[serde(default)]
    pub tradeable: bool,
    #[serde(default)]
    pub discoverable: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnchantmentCost {
    // CHANGE THESE TO i32
    pub a: i32,
    pub b: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MapIcon {
    pub id: u32,
    pub name: String,
    pub appearance: Option<String>, // Optional in some versions
    #[serde(default)]
    pub visible_in_item_frame: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tints {
    pub grass: TintData,
    pub foliage: TintData,
    pub water: TintData,
    pub redstone: TintData, // Structure varies, handle below
    pub constant: TintData,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TintData {
    #[serde(default)]
    pub default: Option<i32>, // Optional default color
    pub data: Vec<TintDatum>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TintDatum {
    // Keys can be string (biome names) or number (redstone level)
    pub keys: Vec<serde_json::Value>,
    pub color: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Legacy {
    pub blocks: HashMap<String, String>,
    pub items: HashMap<String, String>,
}

// Commands vary too much, load as raw Value
// pub type Commands = serde_json::Value;

// Materials vary too much, load as raw Value
// pub type Materials = serde_json::Value;