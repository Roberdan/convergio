use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Stable identifier for schema types (object/link/property).
///
/// Kept intentionally narrow so IDs are portable across:
/// - file paths (YAML definitions)
/// - CLI flags
/// - JSON keys
/// - future SQL primary keys
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct TypeId(String);

impl TypeId {
    /// Create a new [`TypeId`], validating the string.
    pub fn new(raw: impl Into<String>) -> Result<Self, TypeIdParseError> {
        let raw = raw.into();
        validate_id("type", &raw)?;
        Ok(Self(raw))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for TypeId {
    type Err = TypeIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Key for a property slot inside an [`ObjectType`](crate::ObjectType).
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct PropertyKey(String);

impl PropertyKey {
    /// Create a new [`PropertyKey`], validating the string.
    pub fn new(raw: impl Into<String>) -> Result<Self, TypeIdParseError> {
        let raw = raw.into();
        validate_id("property", &raw)?;
        Ok(Self(raw))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PropertyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PropertyKey {
    type Err = TypeIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

fn validate_id(label: &'static str, raw: &str) -> Result<(), TypeIdParseError> {
    if raw.is_empty() {
        return Err(TypeIdParseError::Empty { label });
    }
    if raw.len() > 128 {
        return Err(TypeIdParseError::TooLong {
            label,
            len: raw.len(),
        });
    }

    let mut chars = raw.chars();
    let first = chars.next().ok_or(TypeIdParseError::Empty { label })?;
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(TypeIdParseError::InvalidFirstChar { label, ch: first });
    }

    for ch in raw.chars() {
        let ok =
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-' | ':');
        if !ok {
            return Err(TypeIdParseError::InvalidChar { label, ch });
        }
    }

    Ok(())
}

/// Parse/validation failures for [`TypeId`] and [`PropertyKey`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TypeIdParseError {
    /// Empty identifier.
    #[error("{label} id is empty")]
    Empty {
        /// Which identifier kind failed.
        label: &'static str,
    },

    /// Identifier exceeded the max length.
    #[error("{label} id too long: {len} (max 128)")]
    TooLong {
        /// Which identifier kind failed.
        label: &'static str,
        /// Length in bytes.
        len: usize,
    },

    /// Identifier started with a forbidden character.
    #[error("{label} id must start with a lowercase letter or digit: got '{ch}'")]
    InvalidFirstChar {
        /// Which identifier kind failed.
        label: &'static str,
        /// First character.
        ch: char,
    },

    /// Identifier contained a forbidden character.
    #[error("{label} id contains invalid character '{ch}' (allowed: [a-z0-9._:-])")]
    InvalidChar {
        /// Which identifier kind failed.
        label: &'static str,
        /// Offending character.
        ch: char,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_id_rejects_empty() {
        let err = TypeId::new("").unwrap_err();
        assert_eq!(err, TypeIdParseError::Empty { label: "type" });
    }

    #[test]
    fn type_id_accepts_simple() {
        let id = TypeId::new("edu.student").unwrap();
        assert_eq!(id.as_str(), "edu.student");
    }

    #[test]
    fn property_key_rejects_uppercase() {
        let err = PropertyKey::new("StudentId").unwrap_err();
        assert_eq!(
            err,
            TypeIdParseError::InvalidFirstChar {
                label: "property",
                ch: 'S'
            }
        );
    }
}
