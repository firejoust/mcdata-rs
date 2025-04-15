use crate::error::McDataError;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Loads and deserializes JSON data from a given file path.
pub fn load_data_from_path<T: DeserializeOwned>(path: &Path) -> Result<T, McDataError> {
    let file_content = fs::read_to_string(path).map_err(|e| McDataError::IoError {
        path: path.to_path_buf(),
        source: e,
    })?;

    serde_json::from_str(&file_content).map_err(|e| McDataError::JsonParseError {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Loads data by resolving the path using dataPaths.json first.
///
/// Uses the major version string (e.g., "1.18") to look up the specific path suffix.
pub fn load_data<T: DeserializeOwned>(
    edition: crate::version::Edition,
    version: &str, // Major version string (e.g., "1.18")
    data_key: &str,
) -> Result<T, McDataError> {
    let path = crate::paths::get_full_data_path(edition, version, data_key)?;
    load_data_from_path(&path)
}
