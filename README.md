# mcdata-rs

A Rust library for accessing Minecraft data (blocks, items, entities, biomes, etc.) for various Java Edition (PC) and Bedrock Edition versions. It provides indexed data structures for efficient lookups and handles version resolution and caching.

The data is vendored directly from the comprehensive [`PrismarineJS/minecraft-data`](https://github.com/PrismarineJS/minecraft-data) repository via the build script.

## Features

*   Access to indexed Minecraft data (blocks, items, entities, biomes, effects, sounds, particles, foods, enchantments, etc.).
*   Support for multiple Minecraft versions (currently focused on PC/Java Edition).
*   Automatic data vendoring and updating from `PrismarineJS/minecraft-data` via a build script.
*   Version resolution (e.g., "1.19" resolves to the latest release like "1.19.4", "1.8.8" resolves directly).
*   Efficient data access through HashMaps (indexed by ID, name, state ID, etc.).
*   Caching of loaded version data for performance. Subsequent requests for the same version are near-instant.
*   Feature checking mechanism similar to `node-minecraft-data` to determine version capabilities.
*   Typed data structures for most common data types (defined in `structs.rs`).
*   Access to less structured or highly variable data (like recipes, protocol details, materials) as raw `serde_json::Value`.

## API Overview

The primary way to interact with the library is through the `mc_data` function and the resulting `IndexedData` struct.

### Main Entry Point

*   `mc_data(version_str: &str) -> Result<Arc<IndexedData>, McDataError>`
    *   Loads (or retrieves from cache) all available Minecraft data for the specified version string.
    *   Accepts version strings like `"1.18.2"`, `"pc_1.16.5"`, `"1.8"`, etc.
    *   Handles version resolution (e.g., `"1.18"` might resolve to `"1.18.2"`).
    *   Returns a thread-safe `Arc` containing the indexed data upon success.
    *   Returns an `McDataError` if the version is invalid, data files are missing/corrupt, or other loading issues occur.

### `IndexedData` Struct

This struct holds all the loaded and indexed data for a specific Minecraft version. It is wrapped in an `Arc` for cheap cloning and sharing across threads. Key fields include:

*   `version: Version`: Contains detailed information about the resolved canonical version (Minecraft version string, protocol version, data version, edition, etc.).
*   **Indexed Data Maps:** Provides fast lookups using `Arc<HashMap<...>>`:
    *   `blocks_by_id`, `blocks_by_name`, `blocks_by_state_id`
    *   `items_by_id`, `items_by_name`
    *   `biomes_by_id`, `biomes_by_name`
    *   `effects_by_id`, `effects_by_name`
    *   `entities_by_id`, `entities_by_name`, `mobs_by_id`, `objects_by_id`
    *   `sounds_by_id`, `sounds_by_name`
    *   `particles_by_id`, `particles_by_name`
    *   `attributes_by_name`, `attributes_by_resource`
    *   `instruments_by_id`, `instruments_by_name`
    *   `foods_by_id`, `foods_by_name`
    *   `enchantments_by_id`, `enchantments_by_name`
    *   `map_icons_by_id`, `map_icons_by_name`
    *   `windows_by_id`, `windows_by_name`
    *   `block_loot_by_name`
    *   `entity_loot_by_name`
    *   `block_shapes_by_state_id`, `block_shapes_by_name` (mapping block state IDs or default block names to collision shape bounding boxes `Vec<[f64; 6]>`)
*   **Data Arrays:** Provides access to the original loaded data as `Arc<Vec<...>>`:
    *   `blocks_array`, `items_array`, `biomes_array`, `effects_array`, `entities_array`, `sounds_array`, `particles_array`, `attributes_array`, `instruments_array`, `foods_array`, `enchantments_array`, `map_icons_array`, `windows_array`, `block_loot_array`, `entity_loot_array`
*   **Other Structured Data:**
    *   `language: Arc<HashMap<String, String>>`: Language translations.
    *   `tints: Arc<Option<Tints>>`: Biome color tinting data.
    *   `legacy: Arc<Option<Legacy>>`: Legacy block/item ID mappings.
    *   `block_collision_shapes_raw: Arc<Option<BlockCollisionShapes>>`: The raw, unindexed block collision shape data.
*   **Raw JSON Values:** For data that varies significantly between versions or lacks a stable structure (`Arc<Option<serde_json::Value>>`):
    *   `recipes`
    *   `materials`
    *   `commands`
    *   `protocol` (from `protocol.json`)
    *   `protocol_comments` (from `protocolComments.json`)
    *   `login_packet` (from `loginPacket.json`)
*   **Methods:**
    *   `is_newer_or_equal_to(&self, other_version_str: &str) -> Result<bool, McDataError>`: Compares the current data's version against another version string.
    *   `is_older_than(&self, other_version_str: &str) -> Result<bool, McDataError>`: Compares the current data's version against another version string.
    *   `support_feature(&self, feature_name: &str) -> Result<serde_json::Value, McDataError>`: Checks if the current version supports a given feature (from `features.json`) and returns its value (often boolean, but can be other JSON types).

### Helper Functions

*   `supported_versions(edition: Edition) -> Result<Vec<String>, McDataError>`
    *   Returns a list of Minecraft version strings known to the library for the specified `Edition` (e.g., `Edition::Pc`).

### Core Structs and Enums

*   `structs::*`: Contains the definitions for data types like `Block`, `Item`, `Entity`, `Biome`, `Feature`, `VersionInfo`, etc. These are re-exported at the crate root.
*   `Edition`: Enum representing the Minecraft edition (`Pc` or `Bedrock`).
*   `McDataError`: Enum representing all possible errors returned by the library functions.

## Error Handling

Most public functions return `Result<T, McDataError>`. The `McDataError` enum covers various failure scenarios:

*   `InvalidVersion`: The provided version string could not be resolved.
*   `VersionNotFound`: A specific resolved version's data path mapping is missing.
*   `DataPathNotFound`: A specific data key (like "blocks") is missing for a version in `dataPaths.json`.
*   `IoError`: An error occurred during file reading.
*   `JsonParseError`: An error occurred during JSON deserialization.
*   `McDataDirNotFound`: The vendored data directory was not found (build script issue).
*   `DataFileNotFound`: A specific data file (e.g., `blocks.json`) was not found at the expected path.
*   `Internal`: An unexpected internal error occurred.
*   `CachedError`: An operation failed previously and the error state was cached (original error details might be lost).

## Versioning

The library resolves version strings based on the `protocolVersions.json` file for each edition.

*   Specific versions like `"1.18.2"` or `"pc_1.16.5"` are resolved directly.
*   Major versions like `"1.19"` are resolved to the *latest known release* within that major version (e.g., `"1.19.4"`).
*   Prefixes (`pc_`, `bedrock_`) specify the edition. If no prefix is given, `pc` (Java Edition) is assumed.

The resolved version information is available in the `IndexedData.version` field (`Version` struct).

## Data Source

The Minecraft data is sourced from the [`PrismarineJS/minecraft-data`](https://github.com/PrismarineJS/minecraft-data) repository. The `build.rs` script clones this repository (or pulls updates if already cloned) into the `target` directory during the build process and copies the relevant `data` directory into `src/minecraft_data_vendored`.

## Building

*   **Git:** You must have `git` installed and available in your system's `PATH` for the build script to clone/update the `minecraft-data` repository.
*   **Network:** The build script requires network access to clone/pull from GitHub.
*   **Dependencies:** The build script uses the `fs_extra` crate.

The first build will take longer as it needs to clone the repository. Subsequent builds will be faster, only pulling changes if the remote repository has been updated. The vendored data is included directly in the crate's source tree after the build script runs.