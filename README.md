# mcdata-rs

[![Crates.io](https://img.shields.io/crates/v/mcdata-rs.svg)](https://crates.io/crates/mcdata-rs) <!-- Replace with actual badge if published -->
[![Docs.rs](https://docs.rs/mcdata-rs/badge.svg)](https://docs.rs/mcdata-rs) <!-- Replace with actual badge if published -->

<!-- Add build status badge if you have CI -->

A Rust library providing easy access to Minecraft data for various versions. It aims to be a port of the core functionality found in the popular [`node-minecraft-data`](https://github.com/PrismarineJS/node-minecraft-data) library.

This library reads data directly from the [`minecraft-data`](https://github.com/PrismarineJS/minecraft-data) repository.

**(Work in Progress - Currently supports core PC data including blocks, items, biomes, effects, entities, sounds, particles, attributes, instruments, foods, enchantments, map icons, windows, block/entity loot, block collision shapes, tints, language, versioning, and feature checking. Bedrock support and structured parsing for recipes, materials, commands, and protocol are planned.)**

## Features

- Load data for specific Minecraft Java Edition versions (Bedrock planned).
- Indexed data for fast lookups (by ID, name, state ID, resource, etc.).
- Version comparison utilities (`is_newer_or_equal_to`, `is_older_than`).
- Feature checking (`support_feature`) based on `features.json`.
- Lazy loading and caching of version data for efficiency.
- Handles variations in data formats across different versions.

## Prerequisites

This library requires the `minecraft-data` repository to be available as a Git submodule in your project's root directory, located at `./minecraft-data`.

Initialize it in your project:

```bash
git submodule add https://github.com/PrismarineJS/minecraft-data.git
git submodule update --init --recursive
```

Keep the submodule updated (`git submodule update --remote`) to get the latest data from the upstream `minecraft-data` repository.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
mcdata-rs = "0.1.0" # Replace with the desired version or Git/path source
# Example using Git:
# mcdata-rs = { git = "https://github.com/your-username/mcdata-rs.git", branch = "main" }
log = "0.4" # Optional, for viewing debug logs
```

## Usage

```rust
use rs_minecraft_data::{mc_data, Edition}; // Use your crate name here
use std::sync::Arc;

fn main() -> Result<(), rs_minecraft_data::McDataError> {
    // Load data for Minecraft 1.20.4 (PC assumed by default)
    // Returns an Arc<IndexedData> which is cheap to clone.
    let data_1_20_4: Arc<rs_minecraft_data::IndexedData> = mc_data("1.20.4")?;

    // --- Accessing Data ---

    // Blocks by name
    if let Some(stone) = data_1_20_4.blocks_by_name.get("stone") {
        println!("Stone ID: {}", stone.id);
        println!("Stone Display Name: {}", stone.display_name);
        println!("Stone Hardness: {:?}", stone.hardness);
    }

    // Items by ID
    if let Some(stick) = data_1_20_4.items_by_id.get(&603) { // Stick ID in 1.20.4
        println!("Item ID {}: {}", stick.id, stick.name);
    }

    // Blocks by State ID
    let stone_default_state = data_1_20_4.blocks_by_name.get("stone").map(|b| b.default_state);
    if let Some(state_id) = stone_default_state {
         if let Some(block_state) = data_1_20_4.blocks_by_state_id.get(&state_id) {
              println!("Block for state ID {}: {}", state_id, block_state.name);
         }
    }

    // Foods by name
    if let Some(apple) = data_1_20_4.foods_by_name.get("apple") {
        println!("Apple food points: {}", apple.food_points);
        println!("Apple saturation: {}", apple.saturation);
    }

    // Enchantments by name
    if let Some(sharpness) = data_1_20_4.enchantments_by_name.get("sharpness") {
        println!("Sharpness ID: {}", sharpness.id); // ID is 12 in 1.20.4
        println!("Sharpness max level: {}", sharpness.max_level);
    }

    // Accessing arrays
    println!("First loaded particle: {}", data_1_20_4.particles_array[0].name);
    println!("Number of sounds loaded: {}", data_1_20_4.sounds_array.len());

    // Language
    println!("Stone in language map: {}", data_1_20_4.language.get("block.minecraft.stone").unwrap_or("N/A"));

    // Block Loot
    if let Some(stone_loot) = data_1_20_4.block_loot_by_name.get("stone") {
        println!("Stone drops: {:?}", stone_loot.drops);
    }

    // --- Utilities ---

    // Check feature support
    let supports_cherry_grove = data_1_20_4.support_feature("cherryGrove")?;
    println!("1.20.4 supports Cherry Grove: {}", supports_cherry_grove.as_bool().unwrap_or(false));

    let metadata_ix = data_1_20_4.support_feature("metadataIxOfItem")?;
    println!("metadataIxOfItem for 1.20.4: {}", metadata_ix.as_i64().unwrap_or(-1)); // Should be 8

    // Version comparison
    let data_1_8_8 = mc_data("1.8.8")?;
    println!("Is 1.20.4 >= 1.8.8? {}", data_1_20_4.is_newer_or_equal_to("1.8.8")?);
    println!("Is 1.8.8 < 1.20.4? {}", data_1_8_8.is_older_than("1.20.4")?);

    // Get list of supported versions (for PC in this case)
    let pc_versions = rs_minecraft_data::supported_versions(Edition::Pc)?;
    println!("Some supported PC versions: {:?}", &pc_versions[..5]);

    Ok(())
}
```

## API Overview

### Core Functions

- **`mc_data(version_str: &str) -> Result<Arc<IndexedData>, McDataError>`**
  - The main entry point. Loads data for a given version string (e.g., `"1.18.2"`, `"pc_1.16.5"`, `"bedrock_1.19.80"`).
  - Resolves version strings (including major versions like `"1.16"` to the latest release).
  - Returns a thread-safe, cheaply cloneable `Arc<IndexedData>`. Data is cached globally.
- **`supported_versions(edition: Edition) -> Result<Vec<String>, McDataError>`**
  - Returns a sorted list of supported Minecraft version strings for the given `Edition` (`Edition::Pc` or `Edition::Bedrock`).

### `struct IndexedData`

The primary struct holding all loaded and indexed data for a specific version. Key fields include:

- `version: Version`: Information about the resolved version (Minecraft version, protocol version, data version, edition, etc.).

- **Blocks:**
  - `blocks_array: Arc<Vec<Block>>`
  - `blocks_by_id: Arc<HashMap<u32, Block>>`
  - `blocks_by_name: Arc<HashMap<String, Block>>`
  - `blocks_by_state_id: Arc<HashMap<u32, Block>>`
- **Items:**
  - `items_array: Arc<Vec<Item>>`
  - `items_by_id: Arc<HashMap<u32, Item>>`
  - `items_by_name: Arc<HashMap<String, Item>>`
- **Biomes:**
  - `biomes_array: Arc<Vec<Biome>>`
  - `biomes_by_id: Arc<HashMap<u32, Biome>>`
  - `biomes_by_name: Arc<HashMap<String, Biome>>`
- **Effects:**
  - `effects_array: Arc<Vec<Effect>>`
  - `effects_by_id: Arc<HashMap<u32, Effect>>`
  - `effects_by_name: Arc<HashMap<String, Effect>>`
- **Entities:**
  - `entities_array: Arc<Vec<Entity>>`
  - `entities_by_id: Arc<HashMap<u32, Entity>>`
  - `entities_by_name: Arc<HashMap<String, Entity>>`
  - `mobs_by_id: Arc<HashMap<u32, Entity>>` (Filtered for type "mob")
  - `objects_by_id: Arc<HashMap<u32, Entity>>` (Filtered for type "object")
- **Sounds:**
  - `sounds_array: Arc<Vec<Sound>>`
  - `sounds_by_id: Arc<HashMap<u32, Sound>>`
  - `sounds_by_name: Arc<HashMap<String, Sound>>`
- **Particles:**
  - `particles_array: Arc<Vec<Particle>>`
  - `particles_by_id: Arc<HashMap<u32, Particle>>`
  - `particles_by_name: Arc<HashMap<String, Particle>>`
- **Attributes:**
  - `attributes_array: Arc<Vec<Attribute>>`
  - `attributes_by_name: Arc<HashMap<String, Attribute>>`
  - `attributes_by_resource: Arc<HashMap<String, Attribute>>`
- **Instruments:**
  - `instruments_array: Arc<Vec<Instrument>>`
  - `instruments_by_id: Arc<HashMap<u32, Instrument>>`
  - `instruments_by_name: Arc<HashMap<String, Instrument>>`
- **Foods:**
  - `foods_array: Arc<Vec<Food>>`
  - `foods_by_id: Arc<HashMap<u32, Food>>`
  - `foods_by_name: Arc<HashMap<String, Food>>`
- **Enchantments:**
  - `enchantments_array: Arc<Vec<Enchantment>>`
  - `enchantments_by_id: Arc<HashMap<u32, Enchantment>>`
  - `enchantments_by_name: Arc<HashMap<String, Enchantment>>`
- **Map Icons:**
  - `map_icons_array: Arc<Vec<MapIcon>>`
  - `map_icons_by_id: Arc<HashMap<u32, MapIcon>>`
  - `map_icons_by_name: Arc<HashMap<String, MapIcon>>`
- **Windows (Inventories):**
  - `windows_array: Arc<Vec<Window>>`
  - `windows_by_id: Arc<HashMap<String, Window>>`
  - `windows_by_name: Arc<HashMap<String, Window>>`
- **Loot Tables:**
  - `block_loot_array: Arc<Vec<BlockLoot>>`
  - `block_loot_by_name: Arc<HashMap<String, BlockLoot>>`
  - `entity_loot_array: Arc<Vec<EntityLoot>>`
  - `entity_loot_by_name: Arc<HashMap<String, EntityLoot>>`
- **Other Data:**
  - `block_collision_shapes: Arc<Option<BlockCollisionShapes>>`
  - `tints: Arc<Option<Tints>>`
  - `language: Arc<HashMap<String, String>>`
  - `legacy: Arc<Option<Legacy>>` (Common data, e.g., old block ID mappings)
- **Raw JSON Values (for complex/varying data):**
  - `recipes: Arc<Option<serde_json::Value>>`
  - `materials: Arc<Option<serde_json::Value>>`
  - `commands: Arc<Option<serde_json::Value>>`
  - `protocol: Arc<Option<serde_json::Value>>`
  - `protocol_comments: Arc<Option<serde_json::Value>>`
  - `login_packet: Arc<Option<serde_json::Value>>`

### `IndexedData` Methods

- `is_newer_or_equal_to(&self, other_version_str: &str) -> Result<bool, McDataError>`: Compares the loaded version against another version string (must be same edition).
- `is_older_than(&self, other_version_str: &str) -> Result<bool, McDataError>`: Compares the loaded version against another version string (must be same edition).
- `support_feature(&self, feature_name: &str) -> Result<serde_json::Value, McDataError>`: Checks if a feature (from `features.json`) is supported by this version. Returns the feature's value (often boolean, but can be string/number).

_(See the Rustdoc documentation for details on specific data structures like `Block`, `Item`, `Entity`, etc.)_

## Contributing

Contributions (bug reports, feature requests, pull requests) are welcome! Please check the issue tracker first.

## License

Licensed under the MIT license.