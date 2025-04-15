use crate::data_source;
use crate::error::{Edition, McDataError};
use crate::loader::load_data_from_path;
use crate::structs::DataPaths;
use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Arc;

// Cache for loaded DataPaths.
// The OnceCell stores the Result of the loading operation itself,
// ensuring that we only attempt to load dataPaths.json once.
static LOADED_DATA_PATHS: OnceCell<Result<Arc<DataPaths>, McDataError>> = OnceCell::new();

/// Loads (or retrieves cached) dataPaths.json.
///
/// This function handles caching and ensures that the loading process
/// (including potentially accessing the file system) happens only once.
/// Subsequent calls will return the cached result (either the loaded data or the error).
fn get_data_paths() -> Result<Arc<DataPaths>, McDataError> {
    let stored_result_ref = LOADED_DATA_PATHS.get_or_try_init(|| {
        // This closure is executed only on the first call or if initialization failed previously.
        log::debug!("Attempting to load dataPaths.json for the first time...");

        // Handle potential error from getting the data root path.
        let data_root = data_source::get_data_root()?;
        let path = data_root.join("dataPaths.json");

        // Load and parse the JSON file.
        Ok(load_data_from_path::<DataPaths>(&path).map(Arc::new)) // Returns Result<Arc<DataPaths>, McDataError>
    });

    // Handle the nested Result structure returned by get_or_try_init.
    match stored_result_ref {
        // Case 1: Initialization succeeded, and the stored value is Ok(arc_data).
        Ok(Ok(arc_data_ref)) => Ok(arc_data_ref.clone()), // Clone the Arc from the cached reference.

        // Case 2: Initialization succeeded, but the stored value is an Err(inner_error).
        // This means a previous attempt to load dataPaths.json failed.
        Ok(Err(inner_error_ref)) => {
            // Return a new error indicating that the cached operation failed.
            Err(McDataError::CachedError(format!(
                "Previously failed to load dataPaths.json: {}",
                inner_error_ref
            )))
        }

        // Case 3: Initialization itself failed (e.g., get_data_root failed inside the closure).
        Err(init_error) => {
            // Return the error that occurred during the initialization attempt.
            Err(init_error)
        }
    }
}

/// Gets the relative path suffix (like "pc/1.18") for a given data key and version.
///
/// This path is retrieved from the loaded dataPaths.json.
fn get_path_suffix(edition: Edition, version: &str, data_key: &str) -> Result<String, McDataError> {
    let data_paths = get_data_paths()?; // Load/get cached dataPaths

    let edition_paths = match edition {
        Edition::Pc => &data_paths.pc,
        Edition::Bedrock => &data_paths.bedrock,
    };

    edition_paths
        .get(version) // Look up by major version (e.g., "1.18")
        .and_then(|version_paths| version_paths.get(data_key)) // Look up by data key (e.g., "blocks")
        .cloned() // Clone the String path suffix if found
        .ok_or_else(|| McDataError::DataPathNotFound {
            mc_version: version.to_string(),
            edition,
            data_key: data_key.to_string(),
        })
}

/// Constructs the full, absolute path to a data file (e.g., blocks.json) within the cache.
///
/// It uses the suffix from `dataPaths.json` and searches for a file matching the `data_key`
/// within the corresponding directory.
pub fn get_full_data_path(
    edition: Edition,
    version: &str,
    data_key: &str,
) -> Result<PathBuf, McDataError> {
    let suffix = get_path_suffix(edition, version, data_key)?; // e.g., "pc/1.18"
    let base_path = data_source::get_data_root()?; // e.g., ~/.cache/mcdata-rs/minecraft-data/data

    // The suffix from dataPaths.json gives the directory relative to the base data path.
    let relative_path = PathBuf::from(suffix);
    let dir_path = base_path.join(&relative_path); // e.g., ~/.cache/mcdata-rs/minecraft-data/data/pc/1.18

    // The data_key corresponds to the file stem (e.g., "blocks" for "blocks.json").
    let file_stem = data_key;

    log::trace!(
        "Searching for file stem '{}' in directory {}",
        file_stem,
        dir_path.display()
    );

    match std::fs::read_dir(&dir_path) {
        Ok(entries) => {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            if stem == file_stem {
                                // Found the matching file (e.g., blocks.json).
                                log::trace!("Found data file: {}", path.display());
                                return Ok(path);
                            }
                        }
                    }
                } else {
                    log::warn!(
                        "Error reading directory entry in {}: {:?}",
                        dir_path.display(),
                        entry_result.err()
                    );
                }
            }
            // If the loop finishes without finding the file.
            log::warn!(
                "Data file with stem '{}' not found in directory {}",
                file_stem,
                dir_path.display()
            );
            Err(McDataError::DataFileNotFound {
                data_key: data_key.to_string(),
                path: dir_path.join(format!("{}.*", file_stem)), // Indicate the expected pattern.
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // The directory itself (e.g., data/pc/1.18) was not found.
            log::warn!("Data directory not found: {}", dir_path.display());
            Err(McDataError::DataFileNotFound {
                data_key: data_key.to_string(),
                path: dir_path, // Indicate the directory that was missing.
            })
        }
        Err(e) => {
            // Other I/O error reading the directory.
            log::error!("I/O error reading directory {}: {}", dir_path.display(), e);
            Err(McDataError::IoError {
                path: dir_path,
                source: e,
            })
        }
    }
}
