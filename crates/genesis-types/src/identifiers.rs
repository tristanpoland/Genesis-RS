//! Type-safe identifiers and version types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use crate::errors::{GenesisError, Result};

/// A validated Genesis environment name.
///
/// Environment names must:
/// - Start with a lowercase letter or digit
/// - Contain only lowercase letters, digits, and hyphens
/// - Not start or end with a hyphen
///
/// # Example
///
/// ```
/// use genesis_types::EnvName;
///
/// let env = EnvName::new("us-west-prod").unwrap();
/// assert_eq!(env.as_str(), "us-west-prod");
///
/// // Invalid names are rejected
/// assert!(EnvName::new("Invalid-Name").is_err());
/// assert!(EnvName::new("-invalid").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvName(String);

impl EnvName {
    /// Create a new validated environment name.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't meet validation requirements.
    pub fn new(name: impl AsRef<str>) -> Result<Self> {
        let name = name.as_ref();
        if !Self::is_valid(name) {
            return Err(GenesisError::Validation(format!(
                "Invalid environment name '{}': must contain only lowercase letters, digits, and hyphens, \
                and must start with a letter or digit",
                name
            )));
        }
        Ok(Self(name.to_string()))
    }

    /// Check if a name is valid without allocating.
    pub fn is_valid(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let mut chars = name.chars();
        let first = chars.next().unwrap();

        // Must start with lowercase letter or digit
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return false;
        }

        // All characters must be lowercase, digits, or hyphens
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    /// Get the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the prefix components of an environment name.
    ///
    /// For example, "us-west-prod" returns ["us", "us-west", "us-west-prod"]
    pub fn prefixes(&self) -> Vec<String> {
        let parts: Vec<&str> = self.0.split('-').collect();
        let mut prefixes = Vec::new();

        for i in 1..=parts.len() {
            prefixes.push(parts[..i].join("-"));
        }

        prefixes
    }

    /// Extract environment name from a file path.
    ///
    /// Attempts to extract the environment name from a file path by:
    /// 1. Taking the file stem (filename without extension)
    /// 2. Validating it as an environment name
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't contain a valid environment name.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| GenesisError::Validation(
                format!("Cannot extract environment name from path: {:?}", path)
            ))?;

        Self::new(stem)
    }
}

impl fmt::Display for EnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for EnvName {
    type Err = GenesisError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// Semantic version type following semver 2.0.0 specification.
///
/// # Example
///
/// ```
/// use genesis_types::SemVer;
///
/// let v = SemVer::parse("1.2.3-beta.1+build.123").unwrap();
/// assert_eq!(v.major, 1);
/// assert_eq!(v.minor, 2);
/// assert_eq!(v.patch, 3);
/// assert_eq!(v.pre_release.as_deref(), Some("beta.1"));
/// assert_eq!(v.build.as_deref(), Some("build.123"));
///
/// // Version comparison
/// let v1 = SemVer::parse("1.2.3").unwrap();
/// let v2 = SemVer::parse("1.2.4").unwrap();
/// assert!(v1 < v2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    /// Major version number (incompatible API changes)
    pub major: u32,
    /// Minor version number (backwards-compatible features)
    pub minor: u32,
    /// Patch version number (backwards-compatible bug fixes)
    pub patch: u32,
    /// Pre-release version identifier
    pub pre_release: Option<String>,
    /// Build metadata
    pub build: Option<String>,
}

impl SemVer {
    /// Parse a semantic version string.
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is not valid semver.
    pub fn parse(version: &str) -> Result<Self> {
        // Basic semver parsing - in production, use semver crate
        let parts: Vec<&str> = version.split(&['.', '-', '+'][..]).collect();

        if parts.len() < 3 {
            return Err(GenesisError::Validation(format!(
                "Invalid semantic version '{}': expected format X.Y.Z",
                version
            )));
        }

        let major = parts[0].parse().map_err(|_| {
            GenesisError::Validation(format!("Invalid major version: {}", parts[0]))
        })?;

        let minor = parts[1].parse().map_err(|_| {
            GenesisError::Validation(format!("Invalid minor version: {}", parts[1]))
        })?;

        let patch = parts[2].parse().map_err(|_| {
            GenesisError::Validation(format!("Invalid patch version: {}", parts[2]))
        })?;

        // TODO: Properly parse pre-release and build metadata
        Ok(Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        })
    }

    /// Check if this version meets a minimum version requirement.
    pub fn meets_requirement(&self, min: &SemVer) -> bool {
        self >= min
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(self.pre_release.cmp(&other.pre_release))
    }
}

/// Kit identifier combining name and version.
///
/// # Example
///
/// ```
/// use genesis_types::{KitId, SemVer};
///
/// let kit = KitId {
///     name: "shield".to_string(),
///     version: SemVer::parse("1.2.3").unwrap(),
/// };
///
/// assert_eq!(kit.to_string(), "shield/1.2.3");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitId {
    /// Kit name
    pub name: String,
    /// Kit version
    pub version: SemVer,
}

impl fmt::Display for KitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_name_validation() {
        assert!(EnvName::new("valid-name").is_ok());
        assert!(EnvName::new("valid123").is_ok());
        assert!(EnvName::new("123valid").is_ok());

        assert!(EnvName::new("Invalid-Name").is_err());
        assert!(EnvName::new("-invalid").is_err());
        assert!(EnvName::new("invalid-").is_err());
        assert!(EnvName::new("").is_err());
        assert!(EnvName::new("invalid_name").is_err());
    }

    #[test]
    fn test_env_name_prefixes() {
        let env = EnvName::new("us-west-prod").unwrap();
        let prefixes = env.prefixes();
        assert_eq!(prefixes, vec!["us", "us-west", "us-west-prod"]);
    }

    #[test]
    fn test_semver_parsing() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_semver_comparison() {
        let v1 = SemVer::parse("1.2.3").unwrap();
        let v2 = SemVer::parse("1.2.4").unwrap();
        let v3 = SemVer::parse("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_kit_id_display() {
        let kit = KitId {
            name: "shield".to_string(),
            version: SemVer::parse("1.2.3").unwrap(),
        };
        assert_eq!(kit.to_string(), "shield/1.2.3");
    }
}
