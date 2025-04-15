use crate::data_source;
use crate::error::McDataError;
use crate::loader::load_data_from_path;
use crate::structs::ProtocolVersionInfo;
use once_cell::sync::OnceCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

/// Represents the Minecraft edition (PC/Java or Bedrock).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    Pc,
    Bedrock,
}

impl Edition {
    /// Returns the string prefix used for paths related to this edition.
    pub fn path_prefix(&self) -> &'static str {
        match self {
            Edition::Pc => "pc",
            Edition::Bedrock => "bedrock",
        }
    }
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path_prefix())
    }
}

/// Represents a specific Minecraft version with associated metadata.
///
/// This struct is used for version comparisons and lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    /// The user-facing Minecraft version string (e.g., "1.18.2").
    pub minecraft_version: String,
    /// The major version string (e.g., "1.18").
    pub major_version: String,
    /// The protocol version number.
    pub version: i32,
    /// The data version number, used for reliable comparisons between versions of the same edition.
    /// Higher data versions are newer. This is calculated if missing in source data.
    pub data_version: i32,
    /// The edition (PC or Bedrock).
    pub edition: Edition,
    /// The release type (e.g., "release", "snapshot").
    pub release_type: String,
}

// Implement comparison operators based on `data_version`.
// Versions from different editions are considered incomparable.
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.edition != other.edition {
            None // Cannot compare across editions.
        } else {
            // Compare based on data_version.
            self.data_version.partial_cmp(&other.data_version)
        }
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or_else(|| {
            // If editions differ, log a warning and treat them as equal for sorting purposes,
            // although semantically they are incomparable.
            log::warn!(
                "Comparing Version structs from different editions ({:?} vs {:?})",
                self.edition,
                other.edition
            );
            Ordering::Equal
        })
    }
}

// --- Static Loading and Caching of Version Data ---

// Cache for loaded and indexed version data, keyed by edition.
// Stores the Result to cache loading errors as well.
static LOADED_VERSIONS: OnceCell<HashMap<Edition, Result<Arc<VersionData>, McDataError>>> =
    OnceCell::new();

/// Holds indexed version data for efficient lookups.
#[derive(Debug, Clone)]
pub struct VersionData {
    /// Maps Minecraft version string (e.g., "1.18.2", "1.19") to the corresponding `Version`.
    /// Major version keys map to the latest *release* within that major version.
    pub by_minecraft_version: HashMap<String, Version>,
    /// Maps major version string (e.g., "1.18") to a list of all `Version`s within that major version,
    /// sorted newest first (by data_version).
    pub by_major_version: HashMap<String, Vec<Version>>,
    /// Maps protocol version number to a list of `Version`s sharing that protocol number,
    /// sorted newest first (by data_version).
    pub by_protocol_version: HashMap<i32, Vec<Version>>,
}

/// Loads `protocolVersions.json` for the given edition, calculates missing `data_version`s,
/// and indexes the data into a `VersionData` struct.
fn load_and_index_versions(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
    log::debug!(
        "Attempting to load protocolVersions.json for {:?}...",
        edition
    );
    let data_root = data_source::get_data_root()?;
    let path_str = format!("{}/common/protocolVersions.json", edition.path_prefix());
    let path = data_root.join(path_str);

    // Load the raw version info from the JSON file.
    let mut raw_versions: Vec<ProtocolVersionInfo> = load_data_from_path(&path)?;

    // Calculate `data_version` if missing. This is crucial for reliable comparisons.
    // We assign decreasing negative numbers based on reverse protocol version order.
    // Sort by protocol version descending first to ensure consistent assignment.
    raw_versions.sort_by(|a, b| b.version.cmp(&a.version));
    for (i, v) in raw_versions.iter_mut().enumerate() {
        if v.data_version.is_none() {
            // Assign a synthetic, negative data_version for older entries lacking one.
            v.data_version = Some(-(i as i32));
            log::trace!(
                "Assigned synthetic data_version {} to {}",
                v.data_version.unwrap(),
                v.minecraft_version
            );
        }
    }

    // Initialize index maps.
    let mut by_mc_ver = HashMap::new();
    let mut by_major = HashMap::<String, Vec<Version>>::new();
    let mut by_proto = HashMap::<i32, Vec<Version>>::new();

    // Process each raw version entry and populate the indexes.
    for raw in raw_versions {
        // Ensure data_version was present or calculated.
        let data_version = raw.data_version.ok_or_else(|| {
            McDataError::Internal(format!("Missing dataVersion for {}", raw.minecraft_version))
        })?;

        let v = Version {
            minecraft_version: raw.minecraft_version.clone(),
            major_version: raw.major_version.clone(),
            version: raw.version,
            data_version,
            edition,
            release_type: raw.release_type.clone(),
        };

        // Index by full Minecraft version string (e.g., "1.18.2"). Overwrite if duplicate.
        by_mc_ver.insert(raw.minecraft_version.clone(), v.clone());

        // Index by major version string (e.g., "1.18") to point to the *latest release* within that major version.
        // This allows resolving "1.18" to the newest actual release like "1.18.2".
        by_mc_ver
            .entry(raw.major_version.clone())
            .and_modify(|existing| {
                // Update only if the current version `v` is newer AND is a release,
                // OR if `v` is newer and the existing entry is not a release (prefer releases).
                if (v.data_version > existing.data_version && v.release_type == "release")
                    || (v.data_version > existing.data_version
                        && existing.release_type != "release")
                {
                    *existing = v.clone();
                }
            })
            .or_insert_with(|| v.clone()); // Insert if the major version key doesn't exist yet.

        // Index by major_version, collecting all versions (including snapshots) for that major.
        by_major
            .entry(raw.major_version)
            .or_default()
            .push(v.clone());

        // Index by protocol_version, collecting all versions sharing that protocol number.
        by_proto.entry(raw.version).or_default().push(v);
    }

    // Sort the vectors within the maps by data_version descending (newest first).
    for versions in by_major.values_mut() {
        versions.sort_unstable_by(|a, b| b.cmp(a));
    }
    for versions in by_proto.values_mut() {
        versions.sort_unstable_by(|a, b| b.cmp(a));
    }

    Ok(Arc::new(VersionData {
        by_minecraft_version: by_mc_ver,
        by_major_version: by_major,
        by_protocol_version: by_proto,
    }))
}

/// Gets the cached `VersionData` for the specified edition.
/// Loads and caches data for both editions on the first call.
pub fn get_version_data(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
    let cache = LOADED_VERSIONS.get_or_init(|| {
        log::debug!("Initializing version cache map");
        // Pre-populate both editions on first access to avoid multiple init attempts.
        let mut map = HashMap::new();
        map.insert(Edition::Pc, load_and_index_versions(Edition::Pc));
        map.insert(Edition::Bedrock, load_and_index_versions(Edition::Bedrock));
        map
    });

    // Retrieve the result for the requested edition from the initialized cache.
    match cache.get(&edition) {
        Some(Ok(arc_data)) => Ok(arc_data.clone()),
        Some(Err(original_error)) => Err(McDataError::Internal(format!(
            "Failed to load protocol versions for {:?} during initialization: {}",
            edition, original_error
        ))),
        None => Err(McDataError::Internal(format!(
            "Version data for edition {:?} unexpectedly missing from cache.",
            edition
        ))),
    }
}

/// Resolves a version string (like "1.18.2", "pc_1.16.5", "1.19", or a protocol number)
/// into a canonical `Version` struct.
///
/// It attempts resolution in the following order:
/// 1. Direct lookup by Minecraft version string (e.g., "1.18.2").
/// 2. Lookup by protocol version number (preferring release versions).
/// 3. Lookup by major version string (e.g., "1.19"), resolving to the latest release within that major version.
/// 4. Fallback lookup by major version string, resolving to the absolute newest version (including snapshots) if no release was found in step 3.
pub fn resolve_version(version_str: &str) -> Result<Version, McDataError> {
    log::debug!("Resolving version string: '{}'", version_str);
    let (edition, version_part) = parse_version_string(version_str)?;
    let version_data = get_version_data(edition)?;

    // 1. Try direct Minecraft version lookup (e.g., "1.18.2").
    if let Some(version) = version_data.by_minecraft_version.get(version_part) {
        // Check if the key used was the actual minecraft_version or a major version key.
        // If it was a major version key, the stored version should be the latest release.
        if version.minecraft_version == version_part || version.major_version == version_part {
            log::trace!(
                "Resolved '{}' via direct/major Minecraft version lookup to {}",
                version_str,
                version.minecraft_version
            );
            return Ok(version.clone());
        }
        // If the key matched but wasn't the exact mc version or major version,
        // it might be an older entry (e.g., a .0 version). Let subsequent steps handle it.
    }

    // 2. Try parsing as protocol version number.
    if let Ok(protocol_num) = version_part.parse::<i32>() {
        if let Some(versions) = version_data.by_protocol_version.get(&protocol_num) {
            // Find the best match: prefer the latest release version, otherwise take the absolute newest.
            if let Some(best_match) = versions
                .iter()
                .find(|v| v.release_type == "release") // Find latest release first
                .or_else(|| versions.first())
            // Fallback to newest overall if no release
            {
                log::trace!(
                    "Resolved '{}' via protocol number {} lookup to {}",
                    version_str,
                    protocol_num,
                    best_match.minecraft_version
                );
                return Ok(best_match.clone());
            }
        }
    }

    // 3. Try major version lookup again, specifically checking the by_major_version map.
    // This handles cases where the major version string itself wasn't a direct key in by_minecraft_version
    // or if we need the absolute newest entry (including snapshots) for that major.
    if let Some(versions) = version_data.by_major_version.get(version_part) {
        if let Some(newest) = versions.first() {
            // Versions are sorted newest first.
            log::trace!(
                "Resolved '{}' via by_major_version map (newest is {})",
                version_str,
                newest.minecraft_version
            );
            // Return the absolute newest version found for this major.
            return Ok(newest.clone());
        }
    }

    log::warn!(
        "Failed to resolve version string '{}' for edition {:?}",
        version_str,
        edition
    );
    Err(McDataError::InvalidVersion(version_str.to_string()))
}

/// Parses a version string, extracting the edition and the version part.
/// Defaults to PC edition if no prefix ("pc_" or "bedrock_") is found.
fn parse_version_string(version_str: &str) -> Result<(Edition, &str), McDataError> {
    if let Some(stripped) = version_str.strip_prefix("pc_") {
        Ok((Edition::Pc, stripped))
    } else if let Some(stripped) = version_str.strip_prefix("bedrock_") {
        Ok((Edition::Bedrock, stripped))
    } else {
        // Assume PC edition if no prefix is present.
        // The subsequent `resolve_version` logic will determine if the version part is valid for PC.
        log::trace!(
            "Assuming PC edition for version string '{}' (no prefix found)",
            version_str
        );
        Ok((Edition::Pc, version_str))
    }
}

/// Returns a sorted list of all known specific Minecraft version strings for an edition.
/// Versions are sorted chronologically (oldest first) based on a basic semver-like comparison.
pub fn get_supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    let version_data = get_version_data(edition)?;

    // Extract specific version strings (containing '.') from the indexed data.
    // This filters out major version keys like "1.18" that might be in the map.
    let mut versions: Vec<_> = version_data
        .by_minecraft_version
        .values()
        .filter(|v| v.minecraft_version.contains('.'))
        .map(|v| v.minecraft_version.clone())
        .collect();

    // Sort the versions. A simple numeric comparison of parts usually works well.
    versions.sort_by(|a, b| {
        let parts_a: Vec<Option<u32>> = a.split('.').map(|s| s.parse().ok()).collect();
        let parts_b: Vec<Option<u32>> = b.split('.').map(|s| s.parse().ok()).collect();
        let len = std::cmp::max(parts_a.len(), parts_b.len());
        for i in 0..len {
            // Treat missing parts or non-numeric parts as 0 for comparison.
            let val_a = parts_a.get(i).cloned().flatten().unwrap_or(0);
            let val_b = parts_b.get(i).cloned().flatten().unwrap_or(0);
            match val_a.cmp(&val_b) {
                Ordering::Equal => continue, // If parts are equal, compare the next part.
                other => return other,       // Otherwise, return the comparison result.
            }
        }
        Ordering::Equal // If all parts are equal, the versions are considered equal.
    });

    // Return sorted list (oldest first).
    Ok(versions)
}
