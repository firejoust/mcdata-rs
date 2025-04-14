use crate::error::McDataError;
use crate::structs::ProtocolVersionInfo;
use crate::loader::load_data_from_path;
use crate::constants::VENDORED_MINECRAFT_DATA_PATH;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::path::Path;
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
        self.partial_cmp(other).unwrap_or(Ordering::Equal) // Panic? Or handle error? Let's assume same edition for Ord.
    }
}


// --- Static Loading of Protocol Versions ---

static PC_PROTOCOL_VERSIONS: Lazy<Result<Arc<VersionData>, McDataError>> = Lazy::new(|| {
    load_protocol_versions(Edition::Pc)
});

static BEDROCK_PROTOCOL_VERSIONS: Lazy<Result<Arc<VersionData>, McDataError>> = Lazy::new(|| {
    load_protocol_versions(Edition::Bedrock)
});

#[derive(Debug, Clone)]
pub struct VersionData {
    // minecraft_version -> Version
    pub by_minecraft_version: HashMap<String, Version>,
    // major_version -> Vec<Version> (newest first)
    pub by_major_version: HashMap<String, Vec<Version>>,
    // protocol_version -> Vec<Version> (newest first)
    pub by_protocol_version: HashMap<i32, Vec<Version>>,
}

fn load_protocol_versions(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
    let path_str = format!("{}/{}/common/protocolVersions.json", VENDORED_MINECRAFT_DATA_PATH, edition.path_prefix());
    let path = Path::new(&path_str);
    let mut raw_versions: Vec<ProtocolVersionInfo> = load_data_from_path(path)?;

    // Calculate dataVersion if missing (higher index = older = lower dataVersion)
    for (i, v) in raw_versions.iter_mut().enumerate() {
        if v.data_version.is_none() {
            v.data_version = Some(-(i as i32));
        }
    }

    let mut by_mc_ver = HashMap::new();
    let mut by_major = HashMap::<String, Vec<Version>>::new();
    let mut by_proto = HashMap::<i32, Vec<Version>>::new();

    for raw in raw_versions {
        let v = Version {
            minecraft_version: raw.minecraft_version.clone(),
            major_version: raw.major_version.clone(),
            version: raw.version,
            data_version: raw.data_version.unwrap(), // Should be populated now
            edition,
            release_type: raw.release_type.clone(),
        };

        // Index by minecraft_version (overwrite older entries if duplicate mc version exists)
        by_mc_ver.insert(raw.minecraft_version.clone(), v.clone());
        // Also index by major version if it ends in .0 (like node-minecraft-data)
        if raw.minecraft_version.ends_with(".0") {
             by_mc_ver.insert(raw.major_version.clone(), v.clone());
        }

        // Index by major_version (newest first)
        by_major.entry(raw.major_version).or_default().push(v.clone());

        // Index by protocol_version (newest first)
        by_proto.entry(raw.version).or_default().push(v);
    }

    Ok(Arc::new(VersionData {
        by_minecraft_version: by_mc_ver,
        by_major_version: by_major,
        by_protocol_version: by_proto,
    }))
}

pub fn get_version_data(edition: Edition) -> Result<Arc<VersionData>, McDataError> {
  let lazy_ref = match edition {
     Edition::Pc => &PC_PROTOCOL_VERSIONS,
     Edition::Bedrock => &BEDROCK_PROTOCOL_VERSIONS,
 };
 // Dereference the Lazy, then handle the Result<&Result<...>>
 match **lazy_ref {
     Ok(ref arc_data) => Ok(arc_data.clone()),
     // Cannot clone the error, so create a new one indicating the source
     Err(ref original_error) => Err(McDataError::Internal(format!(
         "Failed to load protocol versions during static initialization: {}",
         original_error
     ))),
 }
}

/// Resolves a version string (like "1.18.2", "pc_1.16.5", "bedrock_1.17.10")
/// into a canonical Version struct.
pub fn resolve_version(version_str: &str) -> Result<Version, McDataError> {
    let (edition, version_part) = parse_version_string(version_str)?;
    let version_data = get_version_data(edition)?;

    // 1. Try direct Minecraft version lookup
    if let Some(version) = version_data.by_minecraft_version.get(version_part) {
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
                return Ok(best_match.clone());
            }
        }
    }

    // 3. Try major version lookup (find newest for that major)
    if let Some(versions) = version_data.by_major_version.get(version_part) {
         if let Some(newest) = versions.first() { // Versions are sorted newest first
             // Need to look up the full version from the minecraft_version map
             // because the major version map might contain snapshots not in the mc map
             if let Some(canonical_version) = version_data.by_minecraft_version.get(&newest.minecraft_version) {
                 return Ok(canonical_version.clone());
             }
         }
    }


    Err(McDataError::InvalidVersion(version_str.to_string()))
}

// Helper to parse "edition_version" or just "version" (defaulting to PC)
fn parse_version_string(version_str: &str) -> Result<(Edition, &str), McDataError> {
    if let Some(stripped) = version_str.strip_prefix("pc_") {
        Ok((Edition::Pc, stripped))
    } else if let Some(stripped) = version_str.strip_prefix("bedrock_") {
        Ok((Edition::Bedrock, stripped))
    } else {
        // Assume PC if no prefix
        Ok((Edition::Pc, version_str))
    }
}

// Helper to get all known versions for an edition
pub fn get_supported_versions(edition: Edition) -> Result<Vec<String>, McDataError> {
    let version_data = get_version_data(edition)?;
    let mut versions: Vec<_> = version_data.by_minecraft_version.keys().cloned().collect();
    // Sort versions maybe? Or rely on HashMap iteration order? Let's sort for consistency.
    // This requires a more complex sort if versions aren't strictly semver.
    // For now, return as is. A proper semver sort or custom sort based on dataVersion
    // might be needed if order matters.
    versions.sort(); // Basic alphabetical sort
    Ok(versions)
}