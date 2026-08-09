//! The [`AddrInfoHints`] address-lookup hint record.
//!
//! See the crate-root documentation for how the validated input types fit the
//! three-layer design.

use core::ffi::c_int;

use super::family::{
    AddrFamily,
    Protocol,
    SockType,
};

/// Resolver hints for an address lookup.
///
/// Mirrors the `hints` argument of `getaddrinfo`: it constrains the result
/// set by address family, socket type, and protocol, and carries the `AI_*`
/// flag bits. The wire codec serializes a value of this type into the
/// `sfdnsres` request buffer.
///
/// A `getaddrinfo` hints record never carries a socket address, so this type
/// has no address field — only the four selector values the service reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct AddrInfoHints {
    /// `AI_*` flag bits; zero requests the resolver default.
    pub flags: c_int,
    /// Restricts results to one address family, or [`AddrFamily::Unspec`].
    pub family: AddrFamily,
    /// Restricts results to one socket type, or [`SockType::Any`].
    pub socktype: SockType,
    /// Restricts results to one protocol, or [`Protocol::Unspec`].
    pub protocol: Protocol,
}
