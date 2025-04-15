use crate::error::McDataError;
use crate::structs::ProtocolVersionInfo;
use crate::loader::load_data_from_path;
use crate::data_source; // Use data_source
use once_cell::sync::OnceCell; // Use OnceCell
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    Pc,
    Bedrock,
}

impl Edition {
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


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub minecraft_version: String,
    pub major_version: String,
    pub version: i32, // Protocol version
    pub data_version: i32, // Used for comparison
    pub edition: Edition,
    pub release_type: String,
}

// Implement comparison based on data_version
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.edition != other.edition {
            None // Cannot compare across editions
        } else {
            self.data_version.partial_cmp(&other.data_version)
        }
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or_else(|| {
            // Fallback if editions differ? Or panic? Let's treat different editions as incomparable (Equal).
            log::warn!("Comparing Version structs from different editions ({:?} vs {:?})", self.edition, other.edition);
            Ordering::Equal
        })
    }
}


// --- Static Loading of Protocol Versions ---

// Cache loaded version data per edition
static LOADED_VERSIONS: OnceCell<HashMap<Edition, Result<Arc<VersionData>, McDataError>>> = OnceCell::new();


#[derive(Debug, Clone)]
pub struct VersionData {
    // minecraft_version -> Version
    pub by_minecraft_version: HashMap<String, Version>,
    // major_version -> Vec<Version> (newest first)
    pub by_major_version: HashMap<String, Vec<Version>>,
    // protocol_version -> Vec<Version> (newest first)
    pub by_protocol_version: HashMap<i32, Vec<Version>>,
}

fn load_and_index_versions(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
    log::debug!("Attempting to load protocolVersions.json for {:?}...", edition);
    let data_root = data_source::get_data_root()?;
    let path_str = format!("{}/common/protocolVersions.json", edition.path_prefix());
    let path = data_root.join(path_str);

    let mut raw_versions: Vec<ProtocolVersionInfo> = load_data_from_path(&path)?;

    // Calculate dataVersion if missing (higher index = older = lower dataVersion)
    // Sort by protocol version descending first to ensure consistent dataVersion assignment
    // (Newer protocols should generally come first in the file, but let's not rely on it)
    raw_versions.sort_by(|a, b| b.version.cmp(&a.version));
    for (i, v) in raw_versions.iter_mut().enumerate() {
        if v.data_version.is_none() {
            // Assign decreasing negative numbers for older versions if dataVersion is missing
            v.data_version = Some(-(i as i32));
        }
    }

    let mut by_mc_ver = HashMap::new();
    let mut by_major = HashMap::<String, Vec<Version>>::new();
    let mut by_proto = HashMap::<i32, Vec<Version>>::new();

    for raw in raw_versions {
        // Ensure data_version was populated
        let data_version = raw.data_version.ok_or_else(|| McDataError::Internal(format!("Missing dataVersion for {}", raw.minecraft_version)))?;

        let v = Version {
            minecraft_version: raw.minecraft_version.clone(),
            major_version: raw.major_version.clone(),
            version: raw.version,
            data_version,
            edition,
            release_type: raw.release_type.clone(),
        };

        // Index by minecraft_version (overwrite older entries if duplicate mc version exists)
        by_mc_ver.insert(raw.minecraft_version.clone(), v.clone());
        // Also index by major version if it ends in .0 (like node-minecraft-data)
        // And index by major version directly (e.g., "1.18")
        if raw.minecraft_version.ends_with(".0") {
             by_mc_ver.entry(raw.major_version.clone()).or_insert_with(|| v.clone());
        }
        // Ensure major version itself maps to the latest release within it
        by_mc_ver.entry(raw.major_version.clone())
            .and_modify(|existing| {
                // Only update if the new version is newer (higher data_version) and is a release
                if v.data_version > existing.data_version && v.release_type == "release" {
                    *existing = v.clone();
                } else if v.data_version > existing.data_version && existing.release_type != "release" {
                    // Or if the new version is newer and the existing one isn't a release
                     *existing = v.clone();
                }
            })
            .or_insert_with(|| v.clone());


        // Index by major_version (newest first)
        by_major.entry(raw.major_version).or_default().push(v.clone());

        // Index by protocol_version (newest first)
        by_proto.entry(raw.version).or_default().push(v);
    }

    // Sort the vectors within the maps (newest first based on data_version)
    for versions in by_major.values_mut() {
        versions.sort_unstable_by(|a, b| b.cmp(a)); // Descending order (newest first)
    }
    for versions in by_proto.values_mut() {
        versions.sort_unstable_by(|a, b| b.cmp(a)); // Descending order (newest first)
    }


    Ok(Arc::new(VersionData {
        by_minecraft_version: by_mc_ver,
        by_major_version: by_major,
        by_protocol_version: by_proto,
    }))
}

/// Gets the cached version data for the specified edition. Loads if not already cached.
pub fn get_version_data(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
    let cache = LOADED_VERSIONS.get_or_init(|| {
        log::debug!("Initializing version cache map");
        // Pre-populate both editions on first access to avoid multiple init attempts
        let mut map = HashMap::new();
        map.insert(Edition::Pc, load_and_index_versions(Edition::Pc));
        map.insert(Edition::Bedrock, load_and_index_versions(Edition::Bedrock));
        map
    });

    // Get the result for the requested edition
    match cache.get(&edition) {
        Some(Ok(arc_data)) => Ok(arc_data.clone()),
        Some(Err(original_error)) => Err(McDataError::Internal(format!(
            "Failed to load protocol versions for {:?} during initialization: {}",
            edition, original_error
        ))),
        None => Err(McDataError::Internal(format!( // Should not happen if pre-populated
            "Version data for edition {:?} unexpectedly missing from cache.", edition
        ))),
    }
}

/// Resolves a version string (like "1.18.2", "pc_1.16.5", "bedrock_1.17.10", "1.19")
/// into a canonical Version struct.
pub fn resolve_version(version_str: &str) -> Result<Version, McDataError> {
    log::debug!("Resolving version string: '{}'", version_str);
    let (edition, version_part) = parse_version_string(version_str)?;
    let version_data = get_version_data(edition)?;

    // 1. Try direct Minecraft version lookup (e.g., "1.18.2")
    if let Some(version) = version_data.by_minecraft_version.get(version_part) {
        log::trace!("Resolved '{}' via direct Minecraft version lookup", version_str);
        return Ok(version.clone());
    }

    // 2. Try parsing as protocol version number (integer)
    if let Ok(protocol_num) = version_part.parse::<i32>() {
        if let Some(versions) = version_data.by_protocol_version.get(&protocol_num) {
            // Find the best match (prefer release, then newest)
            if let Some(best_match) = versions.iter()
                .find(|v| v.release_type == "release")
                .or_else(|| versions.first()) // Fallback to newest if no release
            {
                 log::trace!("Resolved '{}' via protocol number {} lookup", version_str, protocol_num);
                return Ok(best_match.clone());
            }
        }
    }

    // 3. Try major version lookup (find newest release for that major, e.g. "1.18")
    // This relies on the by_minecraft_version map having an entry for the major version string
    // which points to the latest release (handled during indexing).
    if let Some(version) = version_data.by_minecraft_version.get(version_part) {
         log::trace!("Resolved '{}' via major version lookup", version_str);
         return Ok(version.clone());
    }

    // 4. If major version lookup failed directly, check the by_major_version map
    // This might find snapshots if no release exists for that major string in by_minecraft_version
     if let Some(versions) = version_data.by_major_version.get(version_part) {
         if let Some(newest) = versions.first() { // Versions are sorted newest first
             log::trace!("Resolved '{}' via by_major_version map (newest is {})", version_str, newest.minecraft_version);
             // Return this newest one (could be a snapshot)
             return Ok(newest.clone());
         }
    }


    log::warn!("Failed to resolve version string '{}' for edition {:?}", version_str, edition);
    Err(McDataError::InvalidVersion(version_str.to_string()))
}

// Helper to parse "edition_version" or just "version" (defaulting to PC)
fn parse_version_string(version_str: &str) -> Result<(Edition, &str), McDataError> {
    if let Some(stripped) = version_str.strip_prefix("pc_") {
        Ok((Edition::Pc, stripped))
    } else if let Some(stripped) = version_str.strip_prefix("bedrock_") {
        Ok((Edition::Bedrock, stripped))
    } else {
        // Assume PC if no prefix and it looks like a PC version (e.g., starts with digit)
        // This is a heuristic, might need refinement for edge cases
        if version_str.chars().next().map_or(false, |c| c.is_ascii_digit()) {
             Ok((Edition::Pc, version_str))
        } else {
            // If it doesn't look like a PC version, maybe it's Bedrock without prefix?
            // Or just invalid. Let resolve_version handle the final determination.
            // For now, default to PC, but resolution might fail later.
            log::trace!("Assuming PC edition for version string '{}'", version_str);
            Ok((Edition::Pc, version_str))
            // Alternatively, could try resolving as Bedrock here too, or require prefixes.
            // Err(McDataError::InvalidVersion(format!("Ambiguous version string '{}', please prefix with 'pc_' or 'bedrock_'", version_str)))
        }
    }
}

// Helper to get all known versions for an edition
pub fn get_supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    let version_data = get_version_data(edition)?;
    // Filter out major versions used as keys (like "1.18") from the list
    let mut versions: Vec<_> = version_data.by_minecraft_version.values()
        .filter(|v| v.minecraft_version.contains('.')) // Simple filter: keep only specific versions
        .map(|v| v.minecraft_version.clone())
        .collect();

    // Sort versions naturally if possible, otherwise alphabetically
    versions.sort_by(|a, b| {
        // Basic semver-like comparison (split by '.', compare numerically)
        let parts_a: Vec<Option<u32>> = a.split('.').map(|s| s.parse().ok()).collect();
        let parts_b: Vec<Option<u32>> = b.split('.').map(|s| s.parse().ok()).collect();
        let len = std::cmp::max(parts_a.len(), parts_b.len());
        for i in 0..len {
            let val_a = parts_a.get(i).cloned().flatten().unwrap_or(0);
            let val_b = parts_b.get(i).cloned().flatten().unwrap_or(0);
            match val_a.cmp(&val_b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal // If all numeric parts are equal, consider them equal
    });
    // Optionally reverse to get newest first? Or keep oldest first? Let's keep oldest first.
    // versions.reverse();
    Ok(versions)
}