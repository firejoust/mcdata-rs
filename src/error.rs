use std::path::PathBuf;
use thiserror::Error;
pub use crate::version::Edition;

// Remove Clone from derive list
#[derive(Error, Debug)] // <--- REMOVED CLONE
pub enum McDataError {
    #[error("Version string '{0}' is invalid or unsupported")]
    InvalidVersion(String),

    #[error("Version '{mc_version}' (major: {major_version}) not found for edition {edition:?}")]
    VersionNotFound {
        mc_version: String,
        major_version: String,
        edition: Edition,
    },

    #[error("Data key '{data_key}' not found in dataPaths.json for version {mc_version} ({edition:?})")]
    DataPathNotFound {
        mc_version: String,
        edition: Edition,
        data_key: String,
    },

    #[error("I/O error accessing file {path:?}: {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse JSON file {path:?}: {source}")]
    JsonParseError {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("Minecraft-data directory not found at expected path: {0:?}")]
    McDataDirNotFound(PathBuf),

    #[error("Data file not found for key '{data_key}' at path: {path:?}")]
    DataFileNotFound { data_key: String, path: PathBuf },

    #[error("Internal error: {0}")]
    Internal(String),

    // Add a new variant to represent errors retrieved from cache where the original cannot be cloned
    #[error("Cached operation failed previously: {0}")]
    CachedError(String),
}