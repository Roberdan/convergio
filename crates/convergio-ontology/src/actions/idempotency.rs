//! Idempotency key primitives.

use std::fmt;

/// A validated idempotency key.
///
/// The key is treated as an opaque string by the framework, but we enforce a
/// conservative shape so it is safe to persist and to log.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Maximum key length.
    pub const MAX_LEN: usize = 200;

    /// Create a new idempotency key.
    ///
    /// Allowed characters: ASCII letters/digits plus `._:-/`.
    pub fn new(value: impl Into<String>) -> Result<Self, IdempotencyKeyError> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdempotencyKeyError::Empty);
        }
        if value.len() > Self::MAX_LEN {
            return Err(IdempotencyKeyError::TooLong {
                len: value.len(),
                max: Self::MAX_LEN,
            });
        }
        if !value.is_ascii() {
            return Err(IdempotencyKeyError::NonAscii);
        }
        let ok = value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-' | b'/'));
        if !ok {
            return Err(IdempotencyKeyError::InvalidChars);
        }
        Ok(Self(value))
    }

    /// Return the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validation errors for [`IdempotencyKey`].
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum IdempotencyKeyError {
    /// Key must not be empty.
    #[error("empty")]
    Empty,
    /// Key must be ASCII.
    #[error("non_ascii")]
    NonAscii,
    /// Key contains disallowed characters.
    #[error("invalid_chars")]
    InvalidChars,
    /// Key exceeds the maximum allowed length.
    #[error("too_long:{len}:{max}")]
    TooLong {
        /// Provided length.
        len: usize,
        /// Maximum allowed length.
        max: usize,
    },
}
