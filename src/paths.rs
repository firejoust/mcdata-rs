// src/paths.rs

use crate::error::{McDataError, Edition};
use crate::structs::DataPaths;
use crate::loader::load_data_from_path;
use crate::constants::VENDORED_MINECRAFT_DATA_PATH;
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static DATA_PATHS: Lazy<Result<Arc<DataPaths>, McDataError>> = Lazy::new(|| {
    let path = Path::new(VENDORED_MINECRAFT_DATA_PATH).join("dataPaths.json");
    load_data_from_path(&path).map(Arc::new)
});

/// Gets the relative path suffix (like "pc/1.18.2") for a given data key.
fn get_path_suffix(edition: Edition, version: &str, data_key: &str) -> Result<String, McDataError> {
  // Dereference Lazy once, then use as_ref() to borrow the content of the Result
  let data_paths_result: &Result<Arc<DataPaths>, McDataError> = &*DATA_PATHS; // Deref Lazy

  let data_paths: &Arc<DataPaths> = match data_paths_result.as_ref() { // Borrow Result content
      Ok(arc_data) => arc_data, // arc_data is &Arc<DataPaths>
      Err(original_error) => return Err(McDataError::Internal(format!(
          "Failed to load dataPaths.json during static initialization: {}",
          original_error
      ))),
  };
  // No need to clone the Arc here, we can just use the reference

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

// ... rest of paths.rs remains the same
/// Constructs the full, absolute path to a data file.
pub fn get_full_data_path(edition: Edition, version: &str, data_key: &str) -> Result<PathBuf, McDataError> {
    let suffix = get_path_suffix(edition, version, data_key)?;
    let base_path = PathBuf::from(VENDORED_MINECRAFT_DATA_PATH);

    // The suffix itself contains the edition, e.g., "pc/1.18.2"
    let relative_path = PathBuf::from(suffix);

    // Find the actual file with extension (.json, .yml, etc.)
    let dir_path = base_path.join(&relative_path);
    let file_stem = data_key; // e.g., "blocks", "items"

    match std::fs::read_dir(&dir_path) {
        Ok(entries) => {
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    let path = entry.path();
                    if path.is_file() {
                         if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                             if stem == file_stem {
                                 // Found the file (e.g., blocks.json)
                                 return Ok(path);
                             }
                         }
                    }
                }
            }
             Err(McDataError::DataFileNotFound {
                data_key: data_key.to_string(),
                path: dir_path.join(format!("{}.*", file_stem)), // Show expected pattern
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
             Err(McDataError::DataFileNotFound{
                 data_key: data_key.to_string(),
                 path: dir_path,
             })
        }
        Err(e) => Err(McDataError::IoError { path: dir_path, source: e }),
    }
}