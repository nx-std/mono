//! The std-like resolver facade — the crate's recommended Rust API.
//!
//! Where [`resolver`] mirrors musl's C resolver function-for-function, this
//! module offers the shape a Rust caller expects: a [`lookup_host`] entry
//! point that yields [`core::net`] address types directly, with no
//! `AI_*`/`AF_*` hints to fill in and no owned `addrinfo`-chain type to walk.
//!
//! It is a thin convenience layer — every function here delegates to the
//! musl-shaped resolver and reshapes the result. It performs no IPC and no
//! wire decoding of its own, and it touches no C-ABI item; a caller wanting
//! the resolver's full detail (canonical names, per-record socket types)
//! reaches for [`resolver`] instead.
//!
//! This module is also the parent of the crate's soft core and public Rust
//! API: the validated input types ([`hostname`], [`service`], [`family`],
//! [`hints`]) and the musl-shaped [`resolver`] all live under it, so the
//! module tree mirrors the crate's layering. The `sfdnsres` wire-format codec
//! and the owned decoded result types live in [`nx_service_sfdnsres`]; the
//! [`resolver`] module consumes and re-exports them.
//!
//! See the crate-root documentation for how this layer fits the three-layer
//! design.

pub mod family;
pub mod hints;
pub mod hostname;
pub mod resolver;
pub mod service;
pub mod target;

use alloc::{
    vec,
    vec::Vec,
};
use core::net::{
    IpAddr,
    SocketAddr,
};

use nx_service_sfdnsres::SfdnsresService;

use self::{
    hints::AddrInfoHints,
    hostname::Hostname,
    resolver::{
        AddrInfoList,
        ResolveError,
        lookup_addrinfo,
    },
};

/// Resolves a hostname into the socket addresses it maps to.
///
/// This is the facade's headline entry point and the std-shaped counterpart
/// of `getaddrinfo`: it performs an address lookup for `host` over the
/// injected `sfdnsres` session — using the resolver's default hints, with no
/// service and no family/socket-type constraints — and yields each resolved
/// [`SocketAddr`].
///
/// Because no service was requested, every yielded address carries port `0`;
/// a caller that needs a port sets it on the returned addresses, or uses
/// [`resolver::lookup_addrinfo`] with a populated service argument.
///
/// The `sfdnsres` session is passed in rather than created here so the
/// caller owns its lifetime; [`resolver::connect`] establishes one.
///
/// A successful lookup that matched no address yields an empty iterator — it
/// is not an error. The failure modes are those of
/// [`resolver::lookup_addrinfo`].
pub fn lookup_host(host: &Hostname, svc: &SfdnsresService) -> Result<LookupHost, ResolveError> {
    let list = lookup_addrinfo(svc, Some(host), None, &AddrInfoHints::default())?;
    Ok(LookupHost(socket_addrs(&list).into_iter()))
}

/// An iterator over the socket addresses a hostname resolved to.
///
/// Returned by [`lookup_host`]. It is the std-shaped resolver result: a plain
/// sequence of [`SocketAddr`] values in the order the resolver listed them,
/// with none of the `addrinfo`-chain machinery the musl-shaped API exposes.
#[derive(Debug, Clone)]
pub struct LookupHost(vec::IntoIter<SocketAddr>);

impl Iterator for LookupHost {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for LookupHost {
    fn len(&self) -> usize {
        self.0.len()
    }
}

/// Resolves a hostname into the IP addresses it maps to.
///
/// The [`IpAddr`]-returning companion of [`lookup_host`]: it runs the same
/// lookup and yields each resolved address with its port dropped, for callers
/// that want a bare IP rather than a [`SocketAddr`]. The same failure modes
/// and empty-result semantics apply.
pub fn lookup_ip(
    host: &Hostname,
    svc: &SfdnsresService,
) -> Result<impl Iterator<Item = IpAddr>, ResolveError> {
    Ok(lookup_host(host, svc)?.map(|addr| addr.ip()))
}

/// Collects the socket addresses from a decoded `addrinfo` list, in order.
///
/// A resolved `addrinfo` record may carry no socket address (a name-only
/// hint echo, for instance); those records are skipped so the facade yields
/// only usable addresses.
fn socket_addrs(list: &AddrInfoList) -> Vec<SocketAddr> {
    let mut addrs = vec![];
    for record in list.records() {
        if let Some(addr) = record.socket_addr() {
            addrs.push(addr);
        }
    }
    addrs
}
