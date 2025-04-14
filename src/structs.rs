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
    pub hardness: Option<f32>,
    pub resistance: f32,
    pub stack_size: u32,
    pub diggable: bool,
    pub bounding_box: String, // "block" or "empty"
    pub material: Option<String>,
    #[serde(default)] // Default to empty vec if missing
    pub harvest_tools: HashMap<String, bool>,
    #[serde(default)]
    pub variations: Option<Vec<BlockVariation>>,
    #[serde(default)]
    pub drops: Vec<u32>,
    #[serde(default)]
    pub emit_light: u8,
    #[serde(default)]
    pub filter_light: u8,
    #[serde(default)]
    pub transparent: bool,

    // These might be added dynamically for older versions
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
pub struct Item {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub stack_size: u32,
    pub enchantments: Option<Vec<ItemEnchantment>>,
    pub repair_with: Option<Vec<String>>,
    pub max_durability: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemEnchantment {
    pub name: String,
    pub level: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Biome {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub temperature: f32,
    pub precipitation: String,
    pub dimension: String,
    pub display_name: String,
    pub color: i32,
    pub rainfall: f32,
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
    pub entity_type: String, // "mob", "object", "projectile", etc.
    pub width: f32,
    pub height: f32,
    pub category: Option<String>,
}

// --- Feature Checking Structs ---

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub name: String,
    pub description: String,
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
// Represents the structure of dataPaths.json
#[derive(Deserialize, Debug, Clone)]
pub struct DataPaths {
    pub pc: HashMap<String, HashMap<String, String>>,
    pub bedrock: HashMap<String, HashMap<String, String>>,
}