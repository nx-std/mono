//! The validated DNS [`Hostname`] resolver input newtype.
//!
//! A `Hostname` is checked once at construction; every layer below trusts it
//! without re-validation. See the crate-root documentation for how the
//! validated input types fit the three-layer design.

use alloc::string::{
    String,
    ToString,
};
use core::str::FromStr;

/// Maximum length, in bytes, of a [`Hostname`].
///
/// A DNS name is at most 255 octets on the wire; anything longer cannot name
/// a real host, so it is rejected at construction rather than handed to the
/// resolver.
pub const MAX_HOSTNAME_LEN: usize = 255;

/// A validated DNS hostname.
///
/// Guaranteed non-empty and no longer than [`MAX_HOSTNAME_LEN`] bytes. The
/// resolver and wire codec consume this type directly and never re-check it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hostname(String);

impl Hostname {
    /// Returns the hostname as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the hostname as a byte slice (UTF-8, without a NUL terminator).
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Validates that `name` is a usable hostname.
    fn validate(name: &str) -> Result<(), HostnameError> {
        if name.is_empty() {
            return Err(HostnameError::Empty);
        }
        if name.len() > MAX_HOSTNAME_LEN {
            return Err(HostnameError::TooLong);
        }
        Ok(())
    }
}

impl FromStr for Hostname {
    type Err = HostnameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s)?;
        Ok(Self(s.to_string()))
    }
}

/// Errors produced when validating a [`Hostname`].
#[derive(Debug, thiserror::Error)]
pub enum HostnameError {
    /// The supplied hostname was empty.
    ///
    /// An empty string cannot name a host; a caller meaning "no node" must
    /// pass an absent value rather than an empty one.
    #[error("hostname must not be empty")]
    Empty,

    /// The supplied hostname exceeded [`MAX_HOSTNAME_LEN`] bytes.
    ///
    /// A DNS name longer than 255 octets is not representable on the wire.
    #[error("hostname exceeds the maximum length of 255 bytes")]
    TooLong,
}
