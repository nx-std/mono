//! The [`ServiceSpec`] resolver input type — a numeric port or a service name.
//!
//! A `ServiceSpec` is parsed once at construction; every layer below trusts it
//! without re-validation. See the crate-root documentation for how the
//! validated input types fit the three-layer design.

use alloc::string::{
    String,
    ToString,
};
use core::{
    ffi::CStr,
    str::FromStr,
};

/// A resolver service identifier: either a numeric port or a service name.
///
/// Mirrors the `service` argument of `getaddrinfo` — `"80"` is a
/// [`ServiceSpec::Port`], `"http"` is a [`ServiceSpec::Name`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceSpec {
    /// A numeric port number in host byte order.
    Port(u16),

    /// A named service to be resolved (for example, `"http"`).
    Name(String),
}

impl FromStr for ServiceSpec {
    type Err = ServiceSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ServiceSpecError::Empty);
        }
        match s.parse::<u16>() {
            Ok(port) => Ok(Self::Port(port)),
            Err(_) => Ok(Self::Name(s.to_string())),
        }
    }
}

impl TryFrom<&CStr> for ServiceSpec {
    type Error = ServiceSpecError;

    fn try_from(value: &CStr) -> Result<Self, Self::Error> {
        let text = value.to_str().map_err(ServiceSpecError::NotUtf8)?;
        text.parse()
    }
}

/// Errors produced when parsing a [`ServiceSpec`].
#[derive(Debug, thiserror::Error)]
pub enum ServiceSpecError {
    /// The supplied service identifier was empty.
    ///
    /// A caller meaning "no service" must pass an absent value rather than an
    /// empty string.
    #[error("service identifier must not be empty")]
    Empty,

    /// A C-string service identifier contained bytes that are not valid UTF-8.
    #[error("service identifier is not valid UTF-8")]
    NotUtf8(#[source] core::str::Utf8Error),
}
