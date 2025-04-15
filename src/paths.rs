use crate::error::{McDataError, Edition};
use crate::structs::DataPaths;
use crate::loader::load_data_from_path;
use crate::data_source; // Import the new module
use once_cell::sync::OnceCell; // Use OnceCell for caching loaded data
use std::path::PathBuf;
use std::sync::Arc; // Keep Arc for shared ownership of loaded data

// Cache for loaded DataPaths
// The OnceCell stores the result of the loading operation itself.
static LOADED_DATA_PATHS: OnceCell<Result<Arc<DataPaths>, McDataError>> = OnceCell::new();

/// Loads (or retrieves cached) dataPaths.json
fn get_data_paths() -> Result<Arc<DataPaths>, McDataError> {
    // get_or_try_init returns Result<&StoredValue, InitError>
    // StoredValue = Result<Arc<DataPaths>, McDataError>
    // InitError = McDataError (from the closure's potential '?' or explicit Err)
    let stored_result_ref: Result<&Result<Arc<DataPaths>, McDataError>, McDataError> =
        LOADED_DATA_PATHS.get_or_try_init(|| {
            // This closure must return Result<Arc<DataPaths>, McDataError>
            log::debug!("Attempting to load dataPaths.json for the first time...");

            // Handle error from get_data_root explicitly within the closure
            let data_root = match data_source::get_data_root() {
                 Ok(root) => root,
                 // If getting the data root fails, return that error as the result for this init attempt
                 Err(e) => return Err(e),
            };

            let path = data_root.join("dataPaths.json");
            // load_data_from_path already returns Result<T, McDataError>
            Ok(load_data_from_path(&path).map(Arc::new)) // Returns Result<Arc<DataPaths>, McDataError>
        });

    // Now, correctly handle the nested Result structure returned by get_or_try_init
    match stored_result_ref {
        // Case 1: Initialization succeeded, and the stored value is Ok(arc_data)
        Ok(Ok(arc_data_ref)) => Ok(arc_data_ref.clone()), // Clone the Arc from the reference

        // Case 2: Initialization succeeded, but the stored value is an Err(inner_error)
        Ok(Err(inner_error_ref)) => {
            // The error is stored behind a reference. We need to create a new error instance.
            // Since McDataError doesn't implement Clone, we format the stored error.
            // This indicates that a previous attempt failed.
            Err(McDataError::CachedError(format!(
                "Previously failed to load dataPaths.json: {}",
                inner_error_ref
            )))
        }

        // Case 3: Initialization itself failed (e.g., get_data_root failed inside the closure)
        Err(init_error) => {
             // The init_error is owned, so we can return it directly.
             Err(init_error)
        }
    }
}


/// Gets the relative path suffix (like "pc/1.18.2") for a given data key.
fn get_path_suffix(edition: Edition, version: &str, data_key: &str) -> Result<String, McDataError> {
    let data_paths = get_data_paths()?; // Load/get cached dataPaths

    let edition_paths = match edition {
        Edition::Pc => &data_paths.pc,
        Edition::Bedrock => &data_paths.bedrock,
    };

    edition_paths
        .get(version)
        .and_then(|version_paths| version_paths.get(data_key))
        .cloned() // Clones the String path suffix if found
        .ok_or_else(|| McDataError::DataPathNotFound {
            mc_version: version.to_string(),
            edition,
            data_key: data_key.to_string(),
        })
}

/// Constructs the full, absolute path to a data file within the cache.
pub fn get_full_data_path(edition: Edition, version: &str, data_key: &str) -> Result<PathBuf, McDataError> {
    let suffix = get_path_suffix(edition, version, data_key)?;
    let base_path = data_source::get_data_root()?; // Get the runtime data root

    // The suffix itself contains the edition, e.g., "pc/1.18.2"
    let relative_path = PathBuf::from(suffix);

    // Find the actual file with extension (.json, .yml, etc.)
    let dir_path = base_path.join(&relative_path);
    let file_stem = data_key; // e.g., "blocks", "items"

    log::trace!("Searching for file stem '{}' in directory {}", file_stem, dir_path.display());

    match std::fs::read_dir(&dir_path) {
        Ok(entries) => {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    if path.is_file() {
                         if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                             if stem == file_stem {
                                 // Found the file (e.g., blocks.json)
                                 log::trace!("Found data file: {}", path.display());
                                 return Ok(path);
                             }
                         }
                    }
                } else {
                    log::warn!("Error reading directory entry in {}: {:?}", dir_path.display(), entry_result.err());
                }
            }
             log::warn!("Data file with stem '{}' not found in directory {}", file_stem, dir_path.display());
             Err(McDataError::DataFileNotFound {
                data_key: data_key.to_string(),
                path: dir_path.join(format!("{}.*", file_stem)), // Show expected pattern
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
             log::warn!("Data directory not found: {}", dir_path.display());
             Err(McDataError::DataFileNotFound{
                 data_key: data_key.to_string(),
                 path: dir_path, // Show the directory that was not found
             })
        }
        Err(e) => {
            log::error!("I/O error reading directory {}: {}", dir_path.display(), e);
            Err(McDataError::IoError { path: dir_path, source: e })
        }
    }
}