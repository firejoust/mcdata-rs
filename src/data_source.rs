use crate::error::McDataError;
use once_cell::sync::{Lazy, OnceCell};
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Constants for downloading data from the PrismarineJS/minecraft-data repository.
const REPO_URL: &str = "https://github.com/PrismarineJS/minecraft-data";
const BRANCH: &str = "master";
// The path prefix within the downloaded zip archive where the actual data resides.
const DATA_PREFIX_IN_ZIP: &str = "minecraft-data-master/data/";
// Subdirectory within the system cache directory for this library's data.
const CACHE_SUBDIR: &str = "mcdata-rs";
// Name of the directory within CACHE_SUBDIR that holds the extracted repository content.
const DATA_DIR_NAME: &str = "minecraft-data";

// A Mutex to ensure only one thread attempts the download/extraction process at a time,
// preventing redundant downloads and potential race conditions during extraction.
static DOWNLOAD_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
// OnceCell to store the successfully determined path to the extracted 'data' directory.
// This avoids repeated path checks and ensures consistency.
static DATA_PATH: OnceCell<PathBuf> = OnceCell::new();

/// Returns the path to the directory containing the minecraft-data files
/// (e.g., `~/.cache/mcdata-rs/minecraft-data/data`).
///
/// On the first call (or if the data is not found locally), it attempts to
/// download the data repository from GitHub, extract the relevant 'data' directory,
/// and store it in the appropriate cache location. Subsequent calls will return
/// the cached path directly. This function handles locking to prevent concurrent downloads.
///
/// # Errors
/// Returns `McDataError` if:
/// *   The cache directory cannot be determined.
/// *   Network errors occur during download.
/// *   Filesystem errors occur during extraction or verification.
/// *   The downloaded archive is invalid or corrupt.
/// *   The download lock cannot be acquired.
pub fn get_data_root() -> Result<&'static Path, McDataError> {
    DATA_PATH.get_or_try_init(|| {
        // Acquire a lock *before* checking path existence. This prevents a race condition
        // where multiple threads might simultaneously find the path missing and all attempt download.
        let _lock = DOWNLOAD_LOCK.lock().map_err(|_| {
            McDataError::Internal("Failed to acquire download lock".to_string())
        })?;

        // After acquiring the lock, check again if another thread might have already
        // completed the initialization while this thread was waiting.
        if let Some(path) = DATA_PATH.get() {
            log::trace!("Data path already initialized by another thread: {}", path.display());
            return Ok(path.clone());
        }

        // Determine the target cache directory structure.
        let base_cache_dir = dirs_next::cache_dir()
            .ok_or_else(|| McDataError::CacheDirNotFound)?
            .join(CACHE_SUBDIR); // e.g., ~/.cache/mcdata-rs
        let target_repo_dir = base_cache_dir.join(DATA_DIR_NAME); // e.g., ~/.cache/mcdata-rs/minecraft-data
        let target_data_dir = target_repo_dir.join("data"); // The final target: ~/.cache/mcdata-rs/minecraft-data/data

        // Check if the data seems to be present and valid (e.g., dataPaths.json exists).
        let check_file = target_data_dir.join("dataPaths.json");
        if target_data_dir.is_dir() && check_file.is_file() {
            log::info!("Found existing minecraft-data at: {}", target_data_dir.display());
            Ok(target_data_dir)
        } else {
            // Data not found or incomplete, proceed with download and extraction.
            log::info!(
                "minecraft-data not found or incomplete at {}. Downloading...",
                target_data_dir.display()
            );
            // Ensure the parent directory exists (e.g., ~/.cache/mcdata-rs).
            fs::create_dir_all(&base_cache_dir).map_err(|e| McDataError::IoError {
                 path: base_cache_dir.clone(),
                 source: e,
             })?;

            // Perform the download and extraction into the target repository directory.
            download_and_extract(&target_repo_dir)?;

            // Verify that the extraction was successful and the data directory now exists.
            if target_data_dir.is_dir() && check_file.is_file() {
                log::info!("Successfully downloaded and extracted data to {}", target_data_dir.display());
                Ok(target_data_dir)
            } else {
                log::error!("Verification failed after download. Expected data directory not found or incomplete: {}", target_data_dir.display());
                Err(McDataError::DownloadVerificationFailed(target_data_dir))
            }
        }
    // `get_or_try_init` returns `Result<&PathBuf, McDataError>`. We map it to `Result<&Path, McDataError>`.
    }).map(|p| p.as_path())
}

/// Downloads the minecraft-data repository zip archive and extracts the `data` directory
/// into the specified `target_base_dir`.
fn download_and_extract(target_base_dir: &Path) -> Result<(), McDataError> {
    let download_url = format!("{}/archive/refs/heads/{}.zip", REPO_URL, BRANCH);
    log::debug!("Downloading from {}", download_url);

    // --- Download Phase ---
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // Set a reasonable timeout.
        .user_agent(format!("mcdata-rs/{}", env!("CARGO_PKG_VERSION"))) // Identify the client.
        .build()
        .map_err(|e| McDataError::DownloadError(e.to_string()))?;

    let response = client
        .get(&download_url)
        .send()
        .map_err(|e| McDataError::DownloadError(format!("Request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(McDataError::DownloadError(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    // Read the entire response body into memory. For very large repositories,
    // streaming to a temporary file might be more memory-efficient.
    let zip_data = response
        .bytes()
        .map_err(|e| McDataError::DownloadError(format!("Failed to read response bytes: {}", e)))?;
    log::debug!(
        "Download complete ({} bytes). Extracting...",
        zip_data.len()
    );

    // --- Extraction Phase ---
    let reader = Cursor::new(zip_data); // Read from the in-memory bytes.
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| McDataError::ArchiveError(format!("Failed to open zip archive: {}", e)))?;

    // Clear the target directory before extraction to ensure a clean state.
    // The `target_base_dir` should be the directory intended to hold the repo contents
    // (e.g., .../minecraft-data), not the final 'data' directory itself.
    if target_base_dir.exists() {
        log::debug!("Removing existing directory: {}", target_base_dir.display());
        fs::remove_dir_all(target_base_dir).map_err(|e| McDataError::IoError {
            path: target_base_dir.to_path_buf(),
            source: e,
        })?;
    }
    // Recreate the base directory.
    fs::create_dir_all(target_base_dir).map_err(|e| McDataError::IoError {
        path: target_base_dir.to_path_buf(),
        source: e,
    })?;

    // Iterate through each file in the zip archive.
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            McDataError::ArchiveError(format!("Failed to get file at index {}: {}", i, e))
        })?;

        // Get the path of the file *inside* the zip archive.
        let full_path_in_zip = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                log::warn!(
                    "Skipping entry with potentially unsafe path in zip: {}",
                    file.name()
                );
                continue; // Skip potentially malicious paths (e.g., "../..").
            }
        };

        // We are only interested in files located under the specific data prefix within the zip.
        if !full_path_in_zip.starts_with(DATA_PREFIX_IN_ZIP) {
            continue;
        }

        // Determine the relative path *within* the 'data' directory.
        let relative_path_str = full_path_in_zip.to_str().ok_or_else(|| {
            McDataError::Internal(format!(
                "Non-UTF8 path in zip: {}",
                full_path_in_zip.display()
            ))
        })?;
        // This check should always pass due to the `starts_with` check above, but it's safe to keep.
        if !relative_path_str.starts_with(DATA_PREFIX_IN_ZIP) {
            log::warn!(
                "Path {} does not start with expected prefix {}, skipping.",
                relative_path_str,
                DATA_PREFIX_IN_ZIP
            );
            continue;
        }
        // Strip the prefix to get the path relative to the 'data' root (e.g., "pc/1.18/blocks.json").
        let relative_path = Path::new(&relative_path_str[DATA_PREFIX_IN_ZIP.len()..]);

        // Construct the final output path in the filesystem cache.
        // This joins `target_base_dir` / "data" / `relative_path`.
        let outpath = target_base_dir.join("data").join(relative_path);

        if file.name().ends_with('/') {
            // Create the directory if it's a directory entry.
            log::trace!("Creating directory {}", outpath.display());
            fs::create_dir_all(&outpath).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
        } else {
            // Extract the file content.
            log::trace!("Extracting file to {}", outpath.display());
            // Ensure the parent directory exists before creating the file.
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| McDataError::IoError {
                        path: p.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            // Create the output file and copy data from the zip entry.
            let mut outfile = File::create(&outpath).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
            io::copy(&mut file, &mut outfile).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
        }

        // Set file permissions on Unix-like systems, preserving original permissions if possible.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                // Ensure the mode is valid before applying.
                if mode != 0 {
                    if let Err(e) = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                    {
                        // Log a warning but don't fail the entire extraction for permission errors.
                        log::warn!("Failed to set permissions on {}: {}", outpath.display(), e);
                    }
                }
            }
        }
    }
    log::debug!("Extraction complete.");
    Ok(())
}
