# mcdata-rs

[![Crates.io](https://img.shields.io/crates/v/mcdata-rs.svg)](https://crates.io/crates/mcdata-rs)
[![Docs.rs](https://docs.rs/mcdata-rs/badge.svg)](https://docs.rs/mcdata-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
<!-- Add build status badge if you have CI setup -->

A Rust library providing easy access to Minecraft data sourced from the comprehensive [PrismarineJS/minecraft-data](https://github.com/PrismarineJS/minecraft-data) repository. It handles automatic downloading, caching, and indexing of data for various Minecraft versions.

## Features

*   Access to indexed Minecraft data (Blocks, Items, Entities, Biomes, Effects, Foods, etc.) by ID and name.
*   Automatic download and caching of `minecraft-data` files on first use.
*   Helper functions for version comparison (`is_newer_or_equal_to`, `is_older_than`).
*   Feature checking based on `features.json` (`support_feature`).
*   Lazy loading and caching of data per version for efficient memory usage.

## Installation

Add `mcdata-rs` to your `Cargo.toml`:

```toml
[dependencies]
mcdata-rs = "0.1.0" # Replace with the actual latest version from crates.io
```

## Data Cache

The library automatically downloads the necessary `minecraft-data` files on the first run for a given version (or if the cache is missing/corrupted). This data is stored in your system's standard cache directory:

*   **Linux:** `~/.cache/mcdata-rs/minecraft-data`
*   **macOS:** `~/Library/Caches/mcdata-rs/minecraft-data`
*   **Windows:** `%LOCALAPPDATA%\mcdata-rs\minecraft-data`

The initial download might take a moment depending on your network connection. Subsequent runs using the same version will load data instantly from the cache.

*(Optional)*: For debugging download or cache issues, enable logging by setting the `RUST_LOG` environment variable (e.g., `RUST_LOG=mcdata_rs=debug cargo run`).

## API and Usage Examples

The main entry point is the `mc_data(&str)` function, which takes a version string and returns a `Result<Arc<IndexedData>, McDataError>`. The `IndexedData` struct contains all the loaded and indexed data for that version, wrapped in `Arc` for efficient sharing.

```rust
use mcdata_rs::*; // Import necessary items
use std::sync::Arc;

fn main() -> Result<(), McDataError> {
    // --- Get Data for a Specific Version ---
    // Accepts version strings like "1.18.2", "pc_1.16.5", "1.19" (latest release), etc.
    // This might download data on the first run for this version.
    let data_1_18_2: Arc<IndexedData> = mc_data("1.18.2")?;
    println!("Loaded data for Minecraft PC {}", data_1_18_2.version.minecraft_version);

    // --- Accessing Indexed Data ---

    // By Name (most common for blocks, items, entities, etc.)
    if let Some(stone) = data_1_18_2.blocks_by_name.get("stone") {
        println!("Stone Info:");
        println!("  ID: {}", stone.id);
        println!("  Display Name: {}", stone.display_name);
        println!("  Hardness: {:?}", stone.hardness);
        println!("  Diggable: {}", stone.diggable);
    }

    if let Some(stick) = data_1_18_2.items_by_name.get("stick") {
        println!("Stick stack size: {}", stick.stack_size);
    }

    if let Some(zombie) = data_1_18_2.entities_by_name.get("zombie") {
        println!("Zombie category: {:?}", zombie.category);
    }

    // By ID
    if let Some(block_id_1) = data_1_18_2.blocks_by_id.get(&1) {
        // Note: Block ID 1 is typically stone in many versions, but not guaranteed.
        println!("Block with ID 1: {}", block_id_1.name);
    }

    // By State ID (for blocks >= 1.13)
    let stone_block = data_1_18_2.blocks_by_name.get("stone").unwrap(); // Assume stone exists
    if let Some(block_from_state) = data_1_18_2.blocks_by_state_id.get(&stone_block.default_state) {
         println!("Block for default state {}: {}", stone_block.default_state, block_from_state.name);
    }

    // Accessing Arrays (less common, but available)
    println!("First loaded block: {}", data_1_18_2.blocks_array[0].name);
    println!("Total loaded items: {}", data_1_18_2.items_array.len());

    // --- Using Helper Functions ---

    // Version Comparison
    let data_1_16_5 = mc_data("1.16.5")?;
    assert!(data_1_18_2.is_newer_or_equal_to("1.16.5")?);
    assert!(data_1_16_5.is_older_than("1.18.2")?);
    assert!(!data_1_18_2.is_older_than("1.18.2")?);

    // Feature Checking (based on features.json)
    // Check if dimensions were represented as an Integer in 1.15.2
    let data_1_15_2 = mc_data("1.15.2")?;
    let dim_is_int_1_15 = data_1_15_2.support_feature("dimensionIsAnInt")?;
    assert_eq!(dim_is_int_1_15, serde_json::Value::Bool(true));

    // Check the same feature in 1.18.2
    let dim_is_int_1_18 = data_1_18_2.support_feature("dimensionIsAnInt")?;
    assert_eq!(dim_is_int_1_18, serde_json::Value::Bool(false));

    // Check a feature with a value
    let metadata_index = data_1_18_2.support_feature("metadataIxOfItem")?;
    assert_eq!(metadata_index, serde_json::Value::Number(8.into())); // Value might change in data updates

    // --- Listing Supported Versions ---
    let pc_versions = supported_versions(Edition::Pc)?;
    println!("\nSupported PC Versions (Oldest to Newest):");
    // Print first 5 and last 5 for brevity
    for v in pc_versions.iter().take(5) {
        println!(" - {}", v);
    }
    println!("...");
    for v in pc_versions.iter().rev().take(5).rev() {
         println!(" - {}", v);
    }

    // let bedrock_versions = supported_versions(Edition::Bedrock)?;
    // println!("\nSupported Bedrock Versions: {:?}", bedrock_versions);


    // --- Accessing Raw Data (Example: Recipes) ---
    if let Some(recipes) = data_1_18_2.recipes.as_ref() {
        // recipes is a serde_json::Value, access it as needed
        if let Some(crafting_table_recipes) = recipes.get("minecraft:crafting_table") {
             println!("\nFound {} recipes for crafting_table", crafting_table_recipes.as_array().map_or(0, |a| a.len()));
        }
    }

    Ok(())
}

```

## License

Licensed under the MIT License. See the [LICENSE](LICENSE) file for details.