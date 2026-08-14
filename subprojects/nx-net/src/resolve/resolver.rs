//! The musl-shaped resolver API — layer 2 of the crate.
//!
//! This module is the crate's primary public surface. Its function set and
//! naming mirror musl's `src/network` resolver family, but every entry point
//! takes and returns validated Rust types instead of raw C pointers: the
//! The C-ABI surface (the `ffi` module, compiled only with that feature) is
//! a consumer of this API, not a peer.
//!
//! ## Connecting
//!
//! Every resolver operation runs over a live `sfdnsres` session. Establishing
//! one is a two-step Horizon handshake — connect to the service manager, then
//! ask it for the `sfdnsres` service — performed once by [`connect`]. The
//! resulting [`SfdnsresService`] is injected into each per-operation function
//! so the session's lifetime is owned by the caller, not by hidden global
//! state; the C-ABI layer caches a single long-lived session of its own.
//!
//! ## Decoded results
//!
//! The `sfdnsres` wire-format codec lives in [`nx_service_sfdnsres`] itself:
//! its typed CMIF commands accept typed inputs and return owned, decoded
//! result types ([`HostEntry`], [`AddrInfoList`], [`NameInfo`]). This module
//! consumes those decoded types directly — it performs no wire decoding of
//! its own — and re-exports them so a caller of this API need not name the
//! backend crate.
//!
//! See the crate-root documentation for how this layer fits the three-layer
//! design.

use alloc::{
    borrow::Cow,
    string::ToString,
    vec::Vec,
};
use core::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

use nx_service_sfdnsres::{
    AddrFamily as SfAddrFamily,
    AddrInfoHints as SfAddrInfoHints,
    CommandError,
    ConnectCmifError,
    NameInfoFlags,
    Protocol as SfProtocol,
    SfdnsresService,
    SockType as SfSockType,
    netdb::{
        AddrInfoFailure,
        HostFailure,
    },
};
pub use nx_service_sfdnsres::{
    AddrInfoList,
    HostEntry,
    NameInfo,
    ResolvedAddr,
};
use nx_service_sm::SmService;

use super::{
    family::{
        AddrFamily,
        Protocol,
        SockType,
    },
    hints::AddrInfoHints,
    hostname::Hostname,
    service::ServiceSpec,
};

/// Acquires an `sfdnsres` resolver session over the service manager session `sm`.
///
/// `sm` is borrowed rather than opened here: a process gets one service-manager
/// session, and the runtime has taken it long before anything resolves a name.
/// Opening a second does not get a second session — it fails, and it fails
/// before `sfdnsres` has been asked for at all.
///
/// The returned session is passed by reference to the per-operation resolver
/// functions, which keeps session ownership with the caller.
pub fn connect(sm: &SmService) -> Result<SfdnsresService, ConnectError> {
    nx_service_sfdnsres::connect_cmif(sm).map_err(ConnectError)
}

/// Error returned when establishing the `sfdnsres` resolver connection.
///
/// Acquiring the `sfdnsres` session from the service manager failed.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to the sfdnsres service")]
pub struct ConnectError(#[source] pub ConnectCmifError);

/// Resolves a node name and/or service into a list of socket addresses.
///
/// This is the resolver's `getaddrinfo`: it asks the injected `sfdnsres`
/// session to perform the lookup against the typed `hints` template and
/// returns the owned [`AddrInfoList`] the service decoded.
///
/// At least one of `node` or `service` is expected to be present — the same
/// contract `getaddrinfo` itself imposes; an absent value is sent as a null
/// argument. The lookup is performed without a cancellation token and with
/// the network-service-discovery path disabled, matching the C resolver.
///
/// Two failure modes are distinguished:
///
/// - [`ResolveError::Ipc`] — the `sfdnsres` IPC round-trip failed, or its
///   response could not be decoded;
/// - [`ResolveError::Resolver`] — the round-trip succeeded but the resolver
///   reported a non-zero return code, carried verbatim for the caller.
pub fn lookup_addrinfo(
    svc: &SfdnsresService,
    node: Option<&Hostname>,
    service: Option<&ServiceSpec>,
    hints: &AddrInfoHints,
) -> Result<AddrInfoList, ResolveError> {
    // A service identifier is a port or a name; only the port has to be rendered,
    // so the borrow is kept for the name that is already text.
    let service_text = service.map(service_text);

    let result = svc
        .get_addr_info(
            None,
            false,
            node.map(Hostname::as_str),
            service_text.as_deref(),
            &to_sf_hints(hints),
        )
        .map_err(ResolveError::Ipc)?;

    if let Some(failure) = result.failure {
        return Err(ResolveError::Resolver(failure));
    }

    Ok(result.addrs)
}

/// Error returned by a `getaddrinfo` resolution.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// The `sfdnsres` `GetAddrInfo` IPC call failed.
    ///
    /// Covers both a failed IPC round-trip and a malformed response the
    /// `sfdnsres` codec could not decode — the decode failure arrives as a
    /// [`CommandError::Decode`].
    #[error("the sfdnsres getaddrinfo request failed")]
    Ipc(#[source] CommandError),

    /// The round-trip succeeded and the resolver refused the lookup.
    ///
    /// The verdict is the resolver's own, classified at the boundary that
    /// decoded it; a C surface can turn it back into an `EAI_*` code without
    /// this layer having handled one.
    #[error("the resolver refused the address lookup")]
    Resolver(#[source] AddrInfoFailure),
}

/// Resolves a host name into its decoded host entry.
///
/// This is the resolver's `gethostbyname` / `gethostbyname2`: it sends the
/// NUL-terminated `name` to the injected `sfdnsres` session and returns the
/// owned [`HostEntry`] the service decoded.
///
/// `family` narrows the result the way musl's `gethostbyname2` does: an
/// `AF_INET6` request keeps the entry's name and aliases but clears its
/// address list, since `sfdnsres` only ever returns IPv4 host records. Plain
/// `gethostbyname` passes [`AddrFamily::Inet`] (or [`AddrFamily::Unspec`]),
/// which keeps every resolved address.
///
/// The lookup is performed with the network-service-discovery path disabled,
/// matching the C resolver.
///
/// Two failure modes are distinguished:
///
/// - [`HostLookupError::ByName`] — the `sfdnsres` IPC round-trip failed, or
///   its response could not be decoded;
/// - [`HostLookupError::Resolver`] — the round-trip succeeded but the resolver
///   reported a non-zero `h_errno`, carried verbatim for the caller.
pub fn lookup_host_by_name(
    svc: &SfdnsresService,
    name: &Hostname,
    family: AddrFamily,
) -> Result<HostEntry, HostLookupError> {
    let result = svc
        .get_host_by_name(None, false, Some(name.as_str()))
        .map_err(HostLookupError::ByName)?;

    if let Some(failure) = result.failure {
        return Err(HostLookupError::Resolver(failure));
    }

    Ok(filter_by_family(result.host, family))
}

/// Reverse-resolves an IPv4 address into its decoded host entry.
///
/// This is the resolver's `gethostbyaddr`: it sends the address to the
/// injected `sfdnsres` session and returns the owned [`HostEntry`] the service
/// decoded.
///
/// `sfdnsres` performs reverse lookups for IPv4 addresses only, so the input
/// is an [`Ipv4Addr`]; its four octets travel to the service in network order.
///
/// The same two failure modes as [`lookup_host_by_name`] apply, with an IPC
/// transport failure surfaced as [`HostLookupError::ByAddr`].
pub fn lookup_host_by_addr(
    svc: &SfdnsresService,
    addr: Ipv4Addr,
) -> Result<HostEntry, HostLookupError> {
    let result = svc
        .get_host_by_addr(None, IpAddr::V4(addr))
        .map_err(HostLookupError::ByAddr)?;

    if let Some(failure) = result.failure {
        return Err(HostLookupError::Resolver(failure));
    }

    Ok(result.host)
}

/// Error returned by a `gethostbyname` / `gethostbyname2` / `gethostbyaddr`
/// host lookup.
#[derive(Debug, thiserror::Error)]
pub enum HostLookupError {
    /// The `sfdnsres` `GetHostByName` IPC call failed.
    ///
    /// Covers both a failed IPC round-trip and a malformed `hostent` response
    /// the `sfdnsres` codec could not decode.
    #[error("the sfdnsres gethostbyname request failed")]
    ByName(#[source] CommandError),

    /// The `sfdnsres` `GetHostByAddr` IPC call failed.
    ///
    /// Covers both a failed IPC round-trip and a malformed `hostent` response
    /// the `sfdnsres` codec could not decode.
    #[error("the sfdnsres gethostbyaddr request failed")]
    ByAddr(#[source] CommandError),

    /// The round-trip succeeded and the resolver refused the lookup.
    ///
    /// The verdict is the resolver's own, classified at the boundary that
    /// decoded it; a C surface can turn it back into an `h_errno` without this
    /// layer having handled one.
    #[error("the resolver refused the host lookup")]
    Resolver(#[source] HostFailure),
}

/// Reverse-resolves a socket address into its host and service names.
///
/// This is the resolver's `getnameinfo`: it asks the injected `sfdnsres`
/// session to translate `addr` and returns the owned [`NameInfo`] holding the
/// decoded host and service names.
///
/// `flags` is the bitwise-or of the `NI_*` constants; it is passed through to
/// the service unchanged, which is what selects numeric versus symbolic
/// output and whether an unresolved name is an error.
///
/// Two failure modes are distinguished:
///
/// - [`NameInfoError::Ipc`] — the `sfdnsres` IPC round-trip failed;
/// - [`NameInfoError::Resolver`] — the round-trip succeeded but the resolver
///   reported a non-zero return code, carried verbatim for the caller.
pub fn lookup_nameinfo(
    svc: &SfdnsresService,
    addr: &SocketAddr,
    flags: NameInfoFlags,
) -> Result<NameInfo, NameInfoError> {
    let result = svc
        .get_name_info(None, flags, addr)
        .map_err(NameInfoError::Ipc)?;

    if let Some(failure) = result.failure {
        return Err(NameInfoError::Resolver(failure));
    }

    Ok(result.name)
}

/// Error returned by a `getnameinfo` reverse lookup.
#[derive(Debug, thiserror::Error)]
pub enum NameInfoError {
    /// The `sfdnsres` `GetNameInfo` IPC call failed.
    #[error("the sfdnsres getnameinfo request failed")]
    Ipc(#[source] CommandError),

    /// The round-trip succeeded and the resolver refused the lookup.
    ///
    /// The verdict is the resolver's own, classified at the boundary that
    /// decoded it; a C surface can turn it back into an `EAI_*` code without
    /// this layer having handled one.
    #[error("the resolver refused the name lookup")]
    Resolver(#[source] AddrInfoFailure),
}

/// Restricts a decoded host entry to the requested address family.
///
/// `sfdnsres` only ever returns IPv4 host records, so an `AF_INET6` request
/// can match no address: the entry keeps its name and aliases but its address
/// list is cleared. `AF_UNSPEC` and `AF_INET` keep every address the service
/// listed.
fn filter_by_family(entry: HostEntry, family: AddrFamily) -> HostEntry {
    match family {
        AddrFamily::Unspec | AddrFamily::Inet => entry,
        AddrFamily::Inet6 => HostEntry::new(
            entry.name().to_string(),
            entry.aliases().to_vec(),
            Vec::new(),
        ),
    }
}

/// Converts the resolver's validated hints into the `sfdnsres` codec's hint
/// type.
///
/// `nx-net` keeps its own input selector enums — the FFI hard shell anchors
/// its `TryFrom<c_int>` parsers on them — so the typed hints are restated as
/// the equivalent [`nx_service_sfdnsres`] types the IPC command consumes. Each
/// selector maps one-to-one; the two enum families share their discriminants.
fn to_sf_hints(hints: &AddrInfoHints) -> SfAddrInfoHints {
    SfAddrInfoHints {
        flags: hints.flags,
        family: match hints.family {
            AddrFamily::Unspec => SfAddrFamily::Unspec,
            AddrFamily::Inet => SfAddrFamily::Inet,
            AddrFamily::Inet6 => SfAddrFamily::Inet6,
        },
        socktype: match hints.socktype {
            SockType::Any => SfSockType::Any,
            SockType::Stream => SfSockType::Stream,
            SockType::Dgram => SfSockType::Dgram,
            SockType::Raw => SfSockType::Raw,
            SockType::Rdm => SfSockType::Rdm,
            SockType::SeqPacket => SfSockType::SeqPacket,
        },
        protocol: match hints.protocol {
            Protocol::Unspec => SfProtocol::Unspec,
            Protocol::Icmp => SfProtocol::Icmp,
            Protocol::Tcp => SfProtocol::Tcp,
            Protocol::Udp => SfProtocol::Udp,
            Protocol::Ipv6 => SfProtocol::Ipv6,
            Protocol::Icmpv6 => SfProtocol::Icmpv6,
            Protocol::Raw => SfProtocol::Raw,
        },
    }
}

/// Renders a service identifier as the text `sfdnsres` parses.
///
/// A numeric port becomes its decimal text, which is the wire form the resolver
/// reads for both named and numeric services. A name is already that text, so it
/// is borrowed rather than copied.
fn service_text(service: &ServiceSpec) -> Cow<'_, str> {
    match service {
        ServiceSpec::Port(port) => Cow::Owned(port.to_string()),
        ServiceSpec::Name(name) => Cow::Borrowed(name),
    }
}
