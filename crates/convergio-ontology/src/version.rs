use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Strict SemVer for ontology schema versions.
///
/// This is intentionally minimal (no pre-release/build metadata) so it can be
/// used as a stable SQLite key and in deterministic exports.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct SchemaVersion {
    /// Semver major.
    pub major: u64,
    /// Semver minor.
    pub minor: u64,
    /// Semver patch.
    pub patch: u64,
}

impl SchemaVersion {
    /// Construct a new version.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the exact next patch bump.
    pub fn next_patch(self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Returns the exact next minor bump (patch resets to 0).
    pub fn next_minor(self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Returns the exact next major bump (minor/patch reset to 0).
    pub fn next_major(self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SchemaVersion {
    type Err = SchemaVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(SchemaVersionParseError::NotSemver { raw: s.to_string() });
        }

        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| SchemaVersionParseError::NotSemver { raw: s.to_string() })?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| SchemaVersionParseError::NotSemver { raw: s.to_string() })?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| SchemaVersionParseError::NotSemver { raw: s.to_string() })?;

        Ok(Self::new(major, minor, patch))
    }
}

/// Parse failures for [`SchemaVersion`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SchemaVersionParseError {
    /// The string did not match `MAJOR.MINOR.PATCH`.
    #[error("invalid schema version '{raw}': expected MAJOR.MINOR.PATCH")]
    NotSemver {
        /// The input string.
        raw: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_semver() {
        let v: SchemaVersion = "1.2.3".parse().unwrap();
        assert_eq!(v, SchemaVersion::new(1, 2, 3));
    }

    #[test]
    fn rejects_non_semver() {
        let err = "1.2".parse::<SchemaVersion>().unwrap_err();
        assert_eq!(
            err,
            SchemaVersionParseError::NotSemver {
                raw: "1.2".to_string()
            }
        );
    }

    #[test]
    fn ordering_is_semver_ordering() {
        assert!(SchemaVersion::new(1, 0, 0) > SchemaVersion::new(0, 99, 99));
    }
}
