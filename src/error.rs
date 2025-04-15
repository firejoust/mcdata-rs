pub use crate::version::Edition;
use std::path::PathBuf;
use thiserror::Error; // Re-export Edition for convenience

/// Represents errors that can occur within the mcdata-rs library.
#[derive(Error, Debug)]
pub enum McDataError {
    // Errors related to version resolution.
    #[error("Version string '{0}' is invalid or unsupported")]
    InvalidVersion(String),

    #[error("Version '{mc_version}' (major: {major_version}) not found for edition {edition:?}")]
    VersionNotFound {
        mc_version: String,
        major_version: String,
        edition: Edition,
    },

    // Errors related to finding and loading data files.
    #[error(
        "Data key '{data_key}' not found in dataPaths.json for version {mc_version} ({edition:?})"
    )]
    DataPathNotFound {
        mc_version: String, // The major version used for lookup (e.g., "1.18")
        edition: Edition,
        data_key: String, // The data key (e.g., "blocks")
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

    // Errors related to the download and caching mechanism.
    #[error("Could not determine a valid cache directory for application data")]
    CacheDirNotFound,

    #[error("Failed to download minecraft-data: {0}")]
    DownloadError(String), // Wraps errors from reqwest or response handling.

    #[error("Failed to process downloaded archive: {0}")]
    ArchiveError(String), // Wraps errors from the zip library or I/O during extraction.

    #[error("Failed to verify data after download/extraction in {0:?}")]
    DownloadVerificationFailed(PathBuf), // Indicates expected files/dirs were missing post-extraction.

    // Other internal or unexpected errors.
    #[error("Internal error: {0}")]
    Internal(String), // For unexpected states or logic errors.

    #[error("Cached operation failed previously: {0}")]
    CachedError(String), // Indicates a cached OnceCell holds a previous error result.
}
