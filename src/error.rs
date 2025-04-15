use std::path::PathBuf;
use thiserror::Error;
pub use crate::version::Edition; // Keep this

#[derive(Error, Debug)]
pub enum McDataError {
    // --- Versioning Errors ---
    #[error("Version string '{0}' is invalid or unsupported")]
    InvalidVersion(String),

    #[error("Version '{mc_version}' (major: {major_version}) not found for edition {edition:?}")]
    VersionNotFound {
        mc_version: String,
        major_version: String,
        edition: Edition,
    },

    // --- Data Loading Errors ---
    #[error("Data key '{data_key}' not found in dataPaths.json for version {mc_version} ({edition:?})")]
    DataPathNotFound {
        mc_version: String,
        edition: Edition,
        data_key: String,
    },

    #[error("Data file not found for key '{data_key}' at expected path pattern: {path:?}")]
    DataFileNotFound { data_key: String, path: PathBuf },

    #[error("I/O error accessing path {path:?}: {source}")]
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

    // --- Runtime Download/Cache Errors ---
    #[error("Could not determine a valid cache directory for application data")]
    CacheDirNotFound,

    #[error("Failed to download minecraft-data: {0}")]
    DownloadError(String), // Wrap reqwest/IO errors

    #[error("Failed to process downloaded archive: {0}")]
    ArchiveError(String), // Wrap zip/IO errors

    #[error("Failed to verify data after download/extraction in {0:?}")]
    DownloadVerificationFailed(PathBuf),

    // --- Internal/Other Errors ---
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Cached operation failed previously: {0}")]
    CachedError(String), // Keep if used for other caching
}