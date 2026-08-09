//! C-ABI mirrors of the BSD networking types and the resolver constants.
//!
//! Every struct here is laid out byte-for-byte identical to the corresponding
//! C BSD header (`netdb.h`, `sys/socket.h`, `netinet/in.h`) so the
//! `__nx_net__*` FFI can hand C callers result blocks that are
//! indistinguishable from what the original C resolver produced. The
//! `const_assert_eq!` checks at the end of the file pin every size; a layout
//! drift fails the build instead of silently corrupting a result.
//!
//! This module is part of the C-ABI surface: it lives under `ffi` and is
//! compiled only with the `ffi` feature. The idiomatic Rust API (the soft
//! core and the resolver) never references it — the validated input enums
//! carry their own discriminants, and every C-ABI conversion is confined to
//! the sibling `ffi` submodules.
//!
//! Field widths follow the Horizon `aarch64` target: `sa_family_t` is a
//! `u8`, `socklen_t` is a `u32`, and pointers are 8 bytes.

use core::ffi::{
    c_char,
    c_int,
};

use static_assertions::const_assert_eq;

/// Address family: unspecified — accept any family the resolver returns.
pub const AF_UNSPEC: c_int = 0;
/// Address family: IPv4.
pub const AF_INET: c_int = 2;
/// Address family: IPv6.
pub const AF_INET6: c_int = 28;

/// Socket type: reliable byte stream (TCP).
pub const SOCK_STREAM: c_int = 1;
/// Socket type: connectionless datagrams (UDP).
pub const SOCK_DGRAM: c_int = 2;
/// Socket type: raw protocol interface.
pub const SOCK_RAW: c_int = 3;
/// Socket type: reliably-delivered messages.
pub const SOCK_RDM: c_int = 4;
/// Socket type: sequenced packet stream.
pub const SOCK_SEQPACKET: c_int = 5;

/// IP protocol: unspecified — the resolver picks a default.
pub const IPPROTO_IP: c_int = 0;
/// IP protocol: ICMP.
pub const IPPROTO_ICMP: c_int = 1;
/// IP protocol: TCP.
pub const IPPROTO_TCP: c_int = 6;
/// IP protocol: UDP.
pub const IPPROTO_UDP: c_int = 17;
/// IP protocol: IPv6 header.
pub const IPPROTO_IPV6: c_int = 41;
/// IP protocol: ICMPv6.
pub const IPPROTO_ICMPV6: c_int = 58;
/// IP protocol: raw IP packet.
pub const IPPROTO_RAW: c_int = 255;

/// `getaddrinfo` hint flag: addresses are intended for `bind()`.
pub const AI_PASSIVE: c_int = 0x0000_0001;
/// `getaddrinfo` hint flag: fill in `ai_canonname`.
pub const AI_CANONNAME: c_int = 0x0000_0002;
/// `getaddrinfo` hint flag: treat the node as a numeric address only.
pub const AI_NUMERICHOST: c_int = 0x0000_0004;
/// `getaddrinfo` hint flag: treat the service as a numeric port only.
pub const AI_NUMERICSERV: c_int = 0x0000_0008;
/// `getaddrinfo` hint flag: return IPv6 and IPv4-mapped addresses.
pub const AI_ALL: c_int = 0x0000_0100;
/// `getaddrinfo` hint flag: accept IPv4-mapped if the kernel supports it.
pub const AI_V4MAPPED_CFG: c_int = 0x0000_0200;
/// `getaddrinfo` hint flag: only return a family if an address is assigned.
pub const AI_ADDRCONFIG: c_int = 0x0000_0400;
/// `getaddrinfo` hint flag: accept IPv4-mapped IPv6 addresses.
pub const AI_V4MAPPED: c_int = 0x0000_0800;

/// `getnameinfo` flag: return only the hostname portion of an FQDN.
pub const NI_NOFQDN: c_int = 0x0000_0001;
/// `getnameinfo` flag: return the address in numeric form.
pub const NI_NUMERICHOST: c_int = 0x0000_0002;
/// `getnameinfo` flag: fail if the hostname cannot be resolved.
pub const NI_NAMEREQD: c_int = 0x0000_0004;
/// `getnameinfo` flag: return the service in numeric form.
pub const NI_NUMERICSERV: c_int = 0x0000_0008;
/// `getnameinfo` flag: the service is datagram-based.
pub const NI_DGRAM: c_int = 0x0000_0010;
/// `getnameinfo` flag: return the scope ID in numeric form.
pub const NI_NUMERICSCOPE: c_int = 0x0000_0020;

/// `getaddrinfo` error: address family for the hostname is not supported.
pub const EAI_ADDRFAMILY: c_int = 1;
/// `getaddrinfo` error: temporary failure in name resolution.
pub const EAI_AGAIN: c_int = 2;
/// `getaddrinfo` error: invalid value for `ai_flags`.
pub const EAI_BADFLAGS: c_int = 3;
/// `getaddrinfo` error: non-recoverable failure in name resolution.
pub const EAI_FAIL: c_int = 4;
/// `getaddrinfo` error: `ai_family` is not supported.
pub const EAI_FAMILY: c_int = 5;
/// `getaddrinfo` error: memory allocation failure.
pub const EAI_MEMORY: c_int = 6;
/// `getaddrinfo` error: no address associated with the hostname.
pub const EAI_NODATA: c_int = 7;
/// `getaddrinfo` error: neither node nor service was provided, or unknown.
pub const EAI_NONAME: c_int = 8;
/// `getaddrinfo` error: the service is not supported for the socket type.
pub const EAI_SERVICE: c_int = 9;
/// `getaddrinfo` error: `ai_socktype` is not supported.
pub const EAI_SOCKTYPE: c_int = 10;
/// `getaddrinfo` error: a system error is reported in `errno`.
pub const EAI_SYSTEM: c_int = 11;
/// `getaddrinfo` error: invalid value in the hints structure.
pub const EAI_BADHINTS: c_int = 12;
/// `getaddrinfo` error: the resolved protocol is unknown.
pub const EAI_PROTOCOL: c_int = 13;
/// `getaddrinfo` error: an argument buffer overflowed.
pub const EAI_OVERFLOW: c_int = 14;
/// One past the highest defined `EAI_*` code.
pub const EAI_MAX: c_int = 15;

/// `h_errno` value: an internal error occurred; see `errno`.
pub const NETDB_INTERNAL: c_int = -1;
/// `h_errno` value: the lookup succeeded.
pub const NETDB_SUCCESS: c_int = 0;
/// `h_errno` value: the host is authoritatively not found.
pub const HOST_NOT_FOUND: c_int = 1;
/// `h_errno` value: a non-authoritative "not found", retry later.
pub const TRY_AGAIN: c_int = 2;
/// `h_errno` value: a non-recoverable server error.
pub const NO_RECOVERY: c_int = 3;
/// `h_errno` value: the name is valid but has no address record.
pub const NO_DATA: c_int = 4;
/// `h_errno` value: alias of [`NO_DATA`] — no address for the host.
pub const NO_ADDRESS: c_int = NO_DATA;

/// `errno` value: resource temporarily unavailable, retry.
pub const EAGAIN: c_int = 11;
/// `errno` value: out of memory.
pub const ENOMEM: c_int = 12;
/// `errno` value: a bad address was supplied.
pub const EFAULT: c_int = 14;
/// `errno` value: an invalid argument was supplied.
pub const EINVAL: c_int = 22;
/// `errno` value: broken pipe — used for unclassified IPC failures.
pub const EPIPE: c_int = 32;
/// `errno` value: the operation is not supported on the socket.
pub const EOPNOTSUPP: c_int = 95;
/// `errno` value: the address family is not supported by the protocol family.
pub const EAFNOSUPPORT: c_int = 106;

/// Generic socket address header (`struct sockaddr`).
///
/// The leading `sa_len`/`sa_family` pair is shared by every concrete
/// `sockaddr_*` type; pointer casts between them rely on this common prefix.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct sockaddr {
    /// Total length of the address, in bytes.
    pub sa_len: u8,
    /// Address family (`AF_*`).
    pub sa_family: u8,
    /// Family-specific address payload.
    pub sa_data: [c_char; 14],
}

/// IPv4 address (`struct in_addr`).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct in_addr {
    /// The 32-bit IPv4 address, in network byte order.
    pub s_addr: u32,
}

/// IPv6 address (`struct in6_addr`).
///
/// The C definition is a union whose widest member is a `u32`, which forces
/// 4-byte alignment; `align(4)` reproduces that so the enclosing
/// [`sockaddr_in6`] lays out identically.
#[derive(Debug, Clone, Copy)]
#[repr(C, align(4))]
pub struct in6_addr {
    /// The 128-bit IPv6 address, in network byte order.
    pub s6_addr: [u8; 16],
}

/// IPv4 socket address (`struct sockaddr_in`).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct sockaddr_in {
    /// Total length of the address, in bytes.
    pub sin_len: u8,
    /// Address family — always [`AF_INET`].
    pub sin_family: u8,
    /// Port number, in network byte order.
    pub sin_port: u16,
    /// IPv4 address.
    pub sin_addr: in_addr,
    /// Zero padding to the size of [`sockaddr`].
    pub sin_zero: [c_char; 8],
}

/// IPv6 socket address (`struct sockaddr_in6`).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct sockaddr_in6 {
    /// Address family — always [`AF_INET6`].
    pub sin6_family: u8,
    /// Port number, in network byte order.
    pub sin6_port: u16,
    /// IPv6 flow information.
    pub sin6_flowinfo: u32,
    /// IPv6 address.
    pub sin6_addr: in6_addr,
    /// Scope ID for link-local addresses.
    pub sin6_scope_id: u32,
}

/// Family-agnostic socket address storage (`struct sockaddr_storage`).
///
/// Large enough (128 bytes, 8-byte aligned) to hold any concrete
/// `sockaddr_*` value; the result-block allocator reserves one of these per
/// `addrinfo` node so the address fits regardless of family.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct sockaddr_storage {
    /// Total length of the stored address, in bytes.
    pub ss_len: u8,
    /// Address family (`AF_*`).
    pub ss_family: u8,
    /// Padding to the alignment field.
    pub __ss_pad1: [c_char; 6],
    /// Forces 8-byte struct alignment.
    pub __ss_align: i64,
    /// Padding to the full 128-byte size.
    pub __ss_pad2: [c_char; 112],
}

/// Resolved address record (`struct addrinfo`).
///
/// `getaddrinfo` returns a `ai_next`-linked chain of these; the FFI allocates
/// each node as a single block (node + `sockaddr_storage` + canonical name)
/// so `freeaddrinfo` can release it with one deallocation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct addrinfo {
    /// `AI_*` flags carried over from the request hints.
    pub ai_flags: c_int,
    /// Address family (`AF_*`).
    pub ai_family: c_int,
    /// Socket type (`SOCK_*`).
    pub ai_socktype: c_int,
    /// Protocol (`IPPROTO_*`).
    pub ai_protocol: c_int,
    /// Length of the address pointed to by `ai_addr`, in bytes.
    pub ai_addrlen: u32,
    /// Canonical hostname, or null when not requested.
    pub ai_canonname: *mut c_char,
    /// Binary socket address.
    pub ai_addr: *mut sockaddr,
    /// Next record in the chain, or null at the end.
    pub ai_next: *mut addrinfo,
}

/// Host lookup result (`struct hostent`).
///
/// Returned by `gethostbyname`/`gethostbyaddr`; the FFI allocates it as a
/// single block holding the struct plus its alias and address arrays so
/// `freehostent` can release everything with one deallocation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct hostent {
    /// Official, NUL-terminated host name.
    pub h_name: *mut c_char,
    /// Null-terminated array of alias name pointers.
    pub h_aliases: *mut *mut c_char,
    /// Address family of the entries in `h_addr_list` (`AF_*`).
    pub h_addrtype: c_int,
    /// Length of each address in `h_addr_list`, in bytes.
    pub h_length: c_int,
    /// Null-terminated array of address pointers.
    pub h_addr_list: *mut *mut c_char,
}

const_assert_eq!(core::mem::size_of::<sockaddr>(), 16);
const_assert_eq!(core::mem::size_of::<in_addr>(), 4);
const_assert_eq!(core::mem::size_of::<in6_addr>(), 16);
const_assert_eq!(core::mem::size_of::<sockaddr_in>(), 16);
const_assert_eq!(core::mem::size_of::<sockaddr_in6>(), 28);
const_assert_eq!(core::mem::size_of::<sockaddr_storage>(), 128);
const_assert_eq!(core::mem::size_of::<addrinfo>(), 48);
const_assert_eq!(core::mem::size_of::<hostent>(), 32);
