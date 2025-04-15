use crate::error::McDataError;
use once_cell::sync::{Lazy, OnceCell};
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const REPO_URL: &str = "https://github.com/PrismarineJS/minecraft-data";
const BRANCH: &str = "master";
// This is the path prefix *inside* the zip archive
const DATA_PREFIX_IN_ZIP: &str = "minecraft-data-master/data/";
const CACHE_SUBDIR: &str = "mcdata-rs"; // Subdirectory within the system cache dir
const DATA_DIR_NAME: &str = "minecraft-data"; // Name of the directory holding the extracted 'data'

// Use a Mutex to ensure only one thread attempts download/extraction at a time
static DOWNLOAD_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
// Use OnceCell to store the successfully determined data path
static DATA_PATH: OnceCell<PathBuf> = OnceCell::new();

/// Returns the path to the directory containing the minecraft-data files (e.g., .../cache/mcdata-rs/minecraft-data/data).
/// It will attempt to download and extract the data on the first call if not found locally.
pub fn get_data_root() -> Result<&'static Path, McDataError> {
    DATA_PATH.get_or_try_init(|| {
        // Acquire lock *before* checking path existence to prevent race conditions
        // where multiple threads see the path doesn't exist and all try to download.
        let _lock = DOWNLOAD_LOCK.lock().map_err(|_| {
            McDataError::Internal("Failed to acquire download lock".to_string())
        })?;

        // Check again inside the lock, in case another thread finished while waiting
        if let Some(path) = DATA_PATH.get() {
            log::trace!("Data path already initialized: {}", path.display());
            return Ok(path.clone());
        }

        // Determine cache directory
        let cache_dir = dirs_next::cache_dir()
            .ok_or_else(|| McDataError::CacheDirNotFound)?
            .join(CACHE_SUBDIR)
            .join(DATA_DIR_NAME); // e.g., ~/.cache/mcdata-rs/minecraft-data

        let data_dir = cache_dir.join("data"); // The actual target: ~/.cache/mcdata-rs/minecraft-data/data

        // Check if data seems present (e.g., dataPaths.json exists)
        let check_file = data_dir.join("dataPaths.json");
        if data_dir.is_dir() && check_file.is_file() {
            log::info!("Found existing minecraft-data at: {}", data_dir.display());
            Ok(data_dir)
        } else {
            log::info!(
                "minecraft-data not found or incomplete at {}. Downloading...",
                data_dir.display()
            );
            // Ensure the parent directory exists (e.g., ~/.cache/mcdata-rs)
            if let Some(parent) = data_dir.parent() {
                 fs::create_dir_all(parent).map_err(|e| McDataError::IoError {
                     path: parent.to_path_buf(),
                     source: e,
                 })?;
            } else {
                 // This case should be unlikely given how cache_dir is constructed
                 return Err(McDataError::Internal(format!("Could not determine parent directory for {}", data_dir.display())));
            }

            // Download and extract
            download_and_extract(&cache_dir)?; // Pass the parent dir (e.g., .../minecraft-data)

            // Verify again after download
            if data_dir.is_dir() && check_file.is_file() {
                log::info!("Successfully downloaded and extracted data to {}", data_dir.display());
                Ok(data_dir)
            } else {
                log::error!("Verification failed after download. Check directory: {}", data_dir.display());
                Err(McDataError::DownloadVerificationFailed(data_dir))
            }
        }
    }).map(|p| p.as_path()) // Convert Result<PathBuf, Error> to Result<&Path, Error>
}

fn download_and_extract(target_base_dir: &Path) -> Result<(), McDataError> {
    let download_url = format!("{}/archive/refs/heads/{}.zip", REPO_URL, BRANCH);
    log::debug!("Downloading from {}", download_url);

    // --- Download ---
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
        .user_agent(format!("mcdata-rs/{}", env!("CARGO_PKG_VERSION"))) // Be polite
        .build()
        .map_err(|e| McDataError::DownloadError(e.to_string()))?;

    let response = client.get(&download_url)
        .send()
        .map_err(|e| McDataError::DownloadError(format!("Request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(McDataError::DownloadError(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    // Read the response body into memory (consider streaming to temp file for huge repos)
    let zip_data = response.bytes()
        .map_err(|e| McDataError::DownloadError(format!("Failed to read response bytes: {}", e)))?;
    log::debug!("Download complete ({} bytes). Extracting...", zip_data.len());

    // --- Extraction ---
    let reader = Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| McDataError::ArchiveError(format!("Failed to open zip archive: {}", e)))?;

    // Clean the target directory before extraction (ensure it's the base like .../minecraft-data)
    if target_base_dir.exists() {
        log::debug!("Removing existing directory: {}", target_base_dir.display());
        fs::remove_dir_all(target_base_dir).map_err(|e| McDataError::IoError {
            path: target_base_dir.to_path_buf(),
            source: e,
        })?;
    }
    // Recreate the base directory
     fs::create_dir_all(target_base_dir).map_err(|e| McDataError::IoError {
         path: target_base_dir.to_path_buf(),
         source: e,
     })?;


    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            McDataError::ArchiveError(format!("Failed to get file at index {}: {}", i, e))
        })?;

        let full_path_in_zip = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => {
                log::warn!("Skipping entry with potentially unsafe path in zip: {}", file.name());
                continue;
            }
        };

        // We only care about files inside the 'data' directory within the zip
        if !full_path_in_zip.starts_with(DATA_PREFIX_IN_ZIP) {
            continue;
        }

        // Construct the final path in the cache directory
        // Strip the prefix (e.g., "minecraft-data-master/data/") to get the relative path
        // Need to handle potential errors if the prefix isn't exactly as expected
        let relative_path_str = full_path_in_zip.to_str().ok_or_else(|| McDataError::Internal(format!("Non-UTF8 path in zip: {}", full_path_in_zip.display())))?;
        if !relative_path_str.starts_with(DATA_PREFIX_IN_ZIP) {
             // Should not happen due to the check above, but belt-and-suspenders
             log::warn!("Path {} does not start with expected prefix {}, skipping.", relative_path_str, DATA_PREFIX_IN_ZIP);
             continue;
        }
        let relative_path = Path::new(&relative_path_str[DATA_PREFIX_IN_ZIP.len()..]);


        // The final path will be relative to target_base_dir/data
        let outpath = target_base_dir.join("data").join(relative_path);


        if file.name().ends_with('/') {
            // It's a directory
            log::trace!("Creating directory {}", outpath.display());
            fs::create_dir_all(&outpath).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
        } else {
            // It's a file
            log::trace!("Extracting file to {}", outpath.display());
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| McDataError::IoError {
                        path: p.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            let mut outfile = File::create(&outpath).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
            io::copy(&mut file, &mut outfile).map_err(|e| McDataError::IoError {
                path: outpath.clone(),
                source: e,
            })?;
        }

        // Set permissions on Unix systems if needed (optional but good practice)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                // Ensure mode is valid before applying
                if mode != 0 {
                    if let Err(e) = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)) {
                         log::warn!("Failed to set permissions on {}: {}", outpath.display(), e);
                         // Don't fail the whole process for a permissions error
                    }
                }
            }
        }
    }
    log::debug!("Extraction complete.");
    Ok(())
}