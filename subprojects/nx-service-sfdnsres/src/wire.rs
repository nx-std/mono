//! `sfdnsres` wire-format codec and decoded result types.
//!
//! The `sfdnsres` service exchanges resolver data as flat byte buffers with a
//! fixed serialized layout: big-endian (network order) integers,
//! NUL-terminated C strings, and inline sockaddr structures. This module owns
//! that layout — it encodes the typed `getaddrinfo` hints and the
//! `getnameinfo` request `sockaddr`, and decodes the serialized hostent /
//! addrinfo / getnameinfo responses into owned, structurally-valid Rust types
//! ([`HostEntry`], [`AddrInfoList`], [`NameInfo`]).
//!
//! The serialized layout is `sfdnsres`-specific knowledge, so it lives beside
//! the IPC commands that produce it rather than in a separate consumer crate.
//! Decoding is bounds-checked end to end: every read is range-checked against
//! the buffer, so a malformed or truncated response surfaces as a typed
//! [`WireError`] instead of a panic or out-of-bounds access. The command layer
//! turns that into a `CommandError::Decode`.

use alloc::{
    string::String,
    vec::Vec,
};
use core::{
    ffi::c_int,
    net::{
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
        SocketAddrV4,
        SocketAddrV6,
    },
};

/// Big-endian record magic that marks the start of a serialized `addrinfo`.
///
/// The serialized chain is a run of records each prefixed with this value; a
/// trailing `u32` zero where the next magic would be terminates the chain.
const ADDRINFO_MAGIC: u32 = 0xBEEF_CAFE;

/// Size, in bytes, of one serialized `getaddrinfo` hints record.
///
/// The encoding is a 24-byte big-endian header, the 4-byte zero address-slot
/// placeholder, the single-NUL absent canonical name, and the 4-byte chain
/// terminator — always 33 bytes for a hints record (which carries no address).
const HINTS_ENCODED_LEN: usize = 33;

/// Length, in bytes, of a serialized IPv4 host address.
///
/// `sfdnsres` host records carry only IPv4 addresses; [`decode_hostent`]
/// rejects any record whose `h_length` is not this value.
const IPV4_ADDR_LEN: u16 = 4;

/// Address family selector for an address lookup.
///
/// Restricts the result set to a single IP version, or accepts both with
/// [`AddrFamily::Unspec`]. Each variant's discriminant is its C `AF_*`
/// numeric value, so `family as c_int` yields the serialized representation
/// directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AddrFamily {
    /// Accept any address family the resolver returns (`AF_UNSPEC`).
    #[default]
    Unspec = 0,
    /// IPv4 only (`AF_INET`).
    Inet = 2,
    /// IPv6 only (`AF_INET6`).
    Inet6 = 28,
}

/// Socket type selector for an address lookup.
///
/// Each variant's discriminant is its C `SOCK_*` numeric value;
/// [`SockType::Any`] is the unspecified hint (`0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SockType {
    /// Accept any socket type the resolver returns (hint value `0`).
    #[default]
    Any = 0,
    /// Reliable byte stream (`SOCK_STREAM`, TCP).
    Stream = 1,
    /// Connectionless datagrams (`SOCK_DGRAM`, UDP).
    Dgram = 2,
    /// Raw protocol interface (`SOCK_RAW`).
    Raw = 3,
    /// Reliably-delivered messages (`SOCK_RDM`).
    Rdm = 4,
    /// Sequenced packet stream (`SOCK_SEQPACKET`).
    SeqPacket = 5,
}

/// Protocol selector for an address lookup.
///
/// Each variant's discriminant is its C `IPPROTO_*` numeric value;
/// [`Protocol::Unspec`] is the unspecified hint (`IPPROTO_IP`, `0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Protocol {
    /// Let the resolver pick a protocol (`IPPROTO_IP`).
    #[default]
    Unspec = 0,
    /// Internet Control Message Protocol (`IPPROTO_ICMP`).
    Icmp = 1,
    /// Transmission Control Protocol (`IPPROTO_TCP`).
    Tcp = 6,
    /// User Datagram Protocol (`IPPROTO_UDP`).
    Udp = 17,
    /// IPv6 header (`IPPROTO_IPV6`).
    Ipv6 = 41,
    /// ICMP for IPv6 (`IPPROTO_ICMPV6`).
    Icmpv6 = 58,
    /// Raw IP packet (`IPPROTO_RAW`).
    Raw = 255,
}

/// Resolver hints for an address lookup.
///
/// Mirrors the `hints` argument of `getaddrinfo`: it constrains the result
/// set by address family, socket type, and protocol, and carries the `AI_*`
/// flag bits. The encoder serializes a value of this type into the
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

/// Errors produced while decoding an `sfdnsres` response buffer.
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    /// A read needed more bytes than the buffer had remaining.
    #[error("unexpected end of buffer while reading {needed} byte(s)")]
    UnexpectedEof {
        /// Number of bytes the read required.
        needed: usize,
    },

    /// A C string ran to the end of the buffer without a NUL terminator.
    #[error("C string is not NUL-terminated")]
    UnterminatedString,

    /// A serialized `hostent` declared an address family the codec cannot
    /// decode.
    ///
    /// `sfdnsres` host lookups only ever return IPv4 records, so any
    /// `h_addrtype` other than `AF_INET` is a malformed record.
    #[error("serialized hostent has unsupported address family {family}")]
    UnsupportedHostAddrType {
        /// The `h_addrtype` value the buffer carried.
        family: u16,
    },

    /// A serialized `hostent` declared an address length the codec cannot
    /// decode.
    ///
    /// Only the 4-byte IPv4 address length is valid; any other `h_length` is
    /// a malformed record.
    #[error("serialized hostent has unsupported address length {length}")]
    UnsupportedHostAddrLen {
        /// The `h_length` value the buffer carried.
        length: u16,
    },

    /// A serialized `addrinfo` record carried an inline socket address for an
    /// address family the codec cannot decode.
    ///
    /// The codec only knows the `sockaddr_in` (`AF_INET`) and `sockaddr_in6`
    /// (`AF_INET6`) inline layouts; a record with any other `ai_family` and a
    /// non-empty address is malformed.
    #[error("serialized addrinfo record has unsupported address family {family}")]
    UnsupportedAddrInfoFamily {
        /// The `ai_family` value the record carried.
        family: c_int,
    },
}

/// Encodes resolver hints into the serialized `sfdnsres` request format.
///
/// `getaddrinfo` passes the lookup hints to the service as a one-record
/// serialized `addrinfo` chain. The record is a 24-byte big-endian header —
/// `magic`, `ai_flags`, `ai_family`, `ai_socktype`, `ai_protocol`,
/// `ai_addrlen` — then the address slot, then the canonical name, then the
/// chain terminator:
///
/// - **Address slot.** Hints never carry a socket address, so `ai_addrlen` is
///   zero and the slot is the 4-byte zero placeholder the service expects in
///   that case (it is *not* omitted).
/// - **Canonical name.** Absent on hints, encoded as a single NUL byte.
/// - **Terminator.** A trailing `u32` zero stands where the next record's
///   magic would be, ending the one-record chain.
///
/// The encoding is always [`HINTS_ENCODED_LEN`] bytes and cannot fail: the
/// fixed-size buffer is exactly the size the layout needs. Hints carry no
/// socket address, so the `htons`/`htonl` byte-swap quirk that applies to
/// inline addresses never triggers here.
pub(crate) fn encode_hints(hints: &AddrInfoHints) -> [u8; HINTS_ENCODED_LEN] {
    let mut buf = [0u8; HINTS_ENCODED_LEN];
    // 24-byte big-endian record header.
    buf[0..4].copy_from_slice(&ADDRINFO_MAGIC.to_be_bytes());
    buf[4..8].copy_from_slice(&hints.flags.to_be_bytes());
    buf[8..12].copy_from_slice(&(hints.family as c_int).to_be_bytes());
    buf[12..16].copy_from_slice(&(hints.socktype as c_int).to_be_bytes());
    buf[16..20].copy_from_slice(&(hints.protocol as c_int).to_be_bytes());
    // buf[20..24]  ai_addrlen = 0          (no address travels with hints)
    // buf[24..28]  4-byte zero address slot (present even when ai_addrlen == 0)
    // buf[28]      absent canonical name    (a single NUL byte)
    // buf[29..33]  trailing u32 zero        (terminates the one-record chain)
    buf
}

/// Serializes a socket address into the raw BSD `sockaddr` byte form the
/// `sfdnsres` `getnameinfo` command expects.
///
/// `getnameinfo` hands the address to the service verbatim: unlike the
/// serialized `addrinfo` chain, it carries no record header and is not subject
/// to the codec's double byte-swap quirk. The result is a `sockaddr_in`
/// (16 bytes) or `sockaddr_in6` (28 bytes) laid out exactly as the BSD headers
/// define it, with the port — and, for IPv4, the address — in network byte
/// order. The slice length doubles as the `salen` the service reads.
pub(crate) fn encode_sockaddr(addr: &SocketAddr) -> Vec<u8> {
    match addr {
        SocketAddr::V4(v4) => {
            let mut buf = Vec::with_capacity(16);
            buf.push(16); // sin_len
            buf.push(AddrFamily::Inet as u8); // sin_family
            buf.extend_from_slice(&v4.port().to_be_bytes()); // sin_port
            buf.extend_from_slice(&v4.ip().octets()); // sin_addr
            buf.extend_from_slice(&[0u8; 8]); // sin_zero
            buf
        }
        SocketAddr::V6(v6) => {
            let mut buf = Vec::with_capacity(28);
            buf.push(AddrFamily::Inet6 as u8); // sin6_family
            buf.push(0); // padding before sin6_port
            buf.extend_from_slice(&v6.port().to_be_bytes()); // sin6_port
            buf.extend_from_slice(&v6.flowinfo().to_ne_bytes()); // sin6_flowinfo
            buf.extend_from_slice(&v6.ip().octets()); // sin6_addr
            buf.extend_from_slice(&v6.scope_id().to_ne_bytes()); // sin6_scope_id
            buf
        }
    }
}

/// A decoded host lookup result.
///
/// The owned, structurally-valid form of a `gethostbyname` / `gethostbyaddr`
/// reply: an official host name, zero or more alias names, and the resolved
/// IPv4 addresses. The decoder produces a value of this type from the
/// serialized `sfdnsres` `hostent` buffer.
///
/// `sfdnsres` only ever returns IPv4 host records, so the address list is
/// always [`Ipv4Addr`]; a non-IPv4 reply is rejected by the decoder rather
/// than represented here.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostEntry {
    name: String,
    aliases: Vec<String>,
    addresses: Vec<Ipv4Addr>,
}

impl HostEntry {
    /// Creates a host entry from its decoded parts.
    pub fn new(name: String, aliases: Vec<String>, addresses: Vec<Ipv4Addr>) -> Self {
        Self {
            name,
            aliases,
            addresses,
        }
    }

    /// Returns the official host name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the host's alternate names, in the order the service listed
    /// them.
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Returns the resolved IPv4 addresses, in the order the service listed
    /// them.
    pub fn addresses(&self) -> &[Ipv4Addr] {
        &self.addresses
    }
}

/// Decodes a serialized `sfdnsres` `hostent` buffer into a [`HostEntry`].
///
/// `gethostbyname` / `gethostbyaddr` receive the host record as a flat byte
/// buffer with this layout:
///
/// - the official host name, a NUL-terminated C string;
/// - a big-endian `u32` alias count, followed by that many NUL-terminated
///   alias strings;
/// - a big-endian `u16` address family (`h_addrtype`) and a big-endian `u16`
///   address length (`h_length`);
/// - a big-endian `u32` address count, followed by that many addresses, each
///   `h_length` bytes wide.
///
/// `sfdnsres` only returns IPv4 host records, so the sole layout this decoder
/// accepts is `h_addrtype == AF_INET` with `h_length == 4`. Any other family
/// or length fails with [`WireError::UnsupportedHostAddrType`] or
/// [`WireError::UnsupportedHostAddrLen`].
///
/// **Byte order.** Each IPv4 address is stored on the wire with its `s_addr`
/// bytes reversed relative to network byte order; the decoder reverses them
/// back so the returned [`Ipv4Addr`] reads in the usual `a.b.c.d` order.
///
/// Host names are expected to be ASCII; any byte that is not valid UTF-8 is
/// replaced with the Unicode replacement character rather than failing the
/// decode.
pub(crate) fn decode_hostent(buf: &[u8]) -> Result<HostEntry, WireError> {
    let mut reader = Reader::new(buf);

    let name = decode_cstr(&mut reader)?;

    let alias_count = reader.read_u32_be()?;
    let mut aliases = Vec::new();
    for _ in 0..alias_count {
        aliases.push(decode_cstr(&mut reader)?);
    }

    let addr_type = reader.read_u16_be()?;
    if c_int::from(addr_type) != AddrFamily::Inet as c_int {
        return Err(WireError::UnsupportedHostAddrType { family: addr_type });
    }
    let addr_len = reader.read_u16_be()?;
    if addr_len != IPV4_ADDR_LEN {
        return Err(WireError::UnsupportedHostAddrLen { length: addr_len });
    }

    let addr_count = reader.read_u32_be()?;
    let mut addresses = Vec::new();
    for _ in 0..addr_count {
        let bytes = reader.read_bytes(usize::from(addr_len))?;
        // The wire stores s_addr with its bytes reversed relative to network
        // byte order; reverse them back to recover the a.b.c.d octets.
        addresses.push(Ipv4Addr::from([bytes[3], bytes[2], bytes[1], bytes[0]]));
    }

    Ok(HostEntry::new(name, aliases, addresses))
}

/// One decoded record from a serialized `addrinfo` response chain.
///
/// The owned, structurally-valid form of a single `getaddrinfo` result: the
/// `AI_*` / `AF_*` / `SOCK_*` / `IPPROTO_*` values the service assigned to the
/// record, the resolved socket address, and the canonical host name when the
/// service supplied one.
///
/// The selector values are kept in their raw C `int` form: they pass straight
/// through to the C `addrinfo` node a consumer's FFI builds, and no layer
/// below the codec branches on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddr {
    flags: c_int,
    family: c_int,
    socktype: c_int,
    protocol: c_int,
    socket_addr: Option<SocketAddr>,
    canonname: Option<String>,
}

impl ResolvedAddr {
    /// Creates a resolved address record from its decoded parts.
    pub fn new(
        flags: c_int,
        family: c_int,
        socktype: c_int,
        protocol: c_int,
        socket_addr: Option<SocketAddr>,
        canonname: Option<String>,
    ) -> Self {
        Self {
            flags,
            family,
            socktype,
            protocol,
            socket_addr,
            canonname,
        }
    }

    /// Returns the `AI_*` flag bits the service assigned to this record.
    pub fn flags(&self) -> c_int {
        self.flags
    }

    /// Returns the address family (`AF_*`) of this record.
    pub fn family(&self) -> c_int {
        self.family
    }

    /// Returns the socket type (`SOCK_*`) of this record.
    pub fn socktype(&self) -> c_int {
        self.socktype
    }

    /// Returns the protocol (`IPPROTO_*`) of this record.
    pub fn protocol(&self) -> c_int {
        self.protocol
    }

    /// Returns the resolved socket address, or `None` when the record carries
    /// no address.
    pub fn socket_addr(&self) -> Option<SocketAddr> {
        self.socket_addr
    }

    /// Returns the canonical host name, or `None` when the service supplied
    /// none.
    pub fn canonname(&self) -> Option<&str> {
        self.canonname.as_deref()
    }
}

/// A decoded `addrinfo` response: the ordered list of resolved address
/// records.
///
/// The owned, structurally-valid form of a full `getaddrinfo` reply.
/// The decoder produces this by walking each record of the
/// serialized `sfdnsres` response in chain order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddrInfoList(Vec<ResolvedAddr>);

impl AddrInfoList {
    /// Creates an address list from its decoded records, in chain order.
    pub fn new(records: Vec<ResolvedAddr>) -> Self {
        Self(records)
    }

    /// Returns the resolved address records, in the order the service listed
    /// them.
    pub fn records(&self) -> &[ResolvedAddr] {
        &self.0
    }

    /// Returns the number of resolved address records.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` when the service returned no address records.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Decodes a serialized `sfdnsres` `addrinfo` chain into an [`AddrInfoList`].
///
/// `getaddrinfo` receives its result as a flat byte buffer holding a run of
/// serialized `addrinfo` records written back-to-back. Each record is:
///
/// - a 24-byte big-endian header — `magic`, `ai_flags`, `ai_family`,
///   `ai_socktype`, `ai_protocol`, `ai_addrlen`;
/// - the inline socket address — `ai_addrlen` bytes when `ai_addrlen` is
///   non-zero, or a 4-byte zero placeholder when it is zero;
/// - the canonical name — a NUL-terminated C string, a single NUL byte when
///   absent.
///
/// Records run until the next big-endian `u32` is not the record magic: a
/// trailing `u32` zero (or the end of the buffer) terminates the chain.
///
/// **Byte order.** The header integers are plain network order. The inline
/// sockaddr is the quirky part: `sfdnsres` byte-swaps the sockaddr's numeric
/// fields a *second* time, on top of the network order they already carry, so
/// on the wire `sin_port` / `sin6_port`, `sin_addr`, `sin6_flowinfo` and
/// `sin6_scope_id` end up little-endian. The decoder reverses that — it reads
/// those fields little-endian and recovers each IPv4 `s_addr` by reversing its
/// four bytes — so the returned [`SocketAddr`] reads correctly. The IPv6
/// address bytes are not swapped and are taken verbatim.
///
/// A record whose `ai_family` is neither `AF_INET` nor `AF_INET6` but still
/// carries an inline address fails with
/// [`WireError::UnsupportedAddrInfoFamily`].
pub(crate) fn decode_addrinfo_list(buf: &[u8]) -> Result<AddrInfoList, WireError> {
    let mut reader = Reader::new(buf);
    let mut records = Vec::new();
    while matches!(reader.peek_u32_be(), Ok(ADDRINFO_MAGIC)) {
        records.push(decode_addrinfo_record(&mut reader)?);
    }
    Ok(AddrInfoList::new(records))
}

/// A decoded `getnameinfo` reverse-lookup result.
///
/// The owned, structurally-valid form of a `getnameinfo` reply: the host name
/// and the service name the resolver mapped a socket address to. Either string
/// is empty when the request's flags suppressed that half of the lookup or
/// when the resolver returned no text for it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NameInfo {
    host: String,
    service: String,
}

impl NameInfo {
    /// Creates a name-info result from its decoded host and service names.
    pub fn new(host: String, service: String) -> Self {
        Self { host, service }
    }

    /// Returns the resolved host name, or the empty string when none was
    /// returned.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the resolved service name, or the empty string when none was
    /// returned.
    pub fn service(&self) -> &str {
        &self.service
    }
}

/// Decodes the `getnameinfo` host and service reply buffers into a
/// [`NameInfo`].
///
/// The service writes each name into its own output buffer as a
/// NUL-terminated C string padded with trailing zero bytes; this clamps each
/// buffer to its first NUL and interprets the prefix as text. Names are
/// expected to be ASCII; any byte that is not valid UTF-8 is replaced with the
/// Unicode replacement character rather than failing.
pub(crate) fn decode_nameinfo(host: &[u8], service: &[u8]) -> NameInfo {
    NameInfo::new(decode_c_string(host), decode_c_string(service))
}

/// Decodes one serialized `addrinfo` record at the reader's cursor.
///
/// The caller must have confirmed the record magic with
/// [`Reader::peek_u32_be`]; this consumes the magic, the header, the inline
/// socket address, and the canonical name, leaving the cursor at the next
/// record or the chain terminator.
fn decode_addrinfo_record(reader: &mut Reader<'_>) -> Result<ResolvedAddr, WireError> {
    // The magic was confirmed by the caller's peek; consume it.
    reader.read_u32_be()?;
    let flags = reader.read_i32_be()?;
    let family = reader.read_i32_be()?;
    let socktype = reader.read_i32_be()?;
    let protocol = reader.read_i32_be()?;
    let addr_len = reader.read_u32_be()?;

    // A zero ai_addrlen still occupies a 4-byte zero placeholder slot.
    let blob_len = if addr_len == 0 { 4 } else { addr_len as usize };
    let blob = reader.read_bytes(blob_len)?;
    let socket_addr = if addr_len == 0 {
        None
    } else {
        Some(decode_inline_sockaddr(family, blob)?)
    };

    let canonname = decode_cstr(reader)?;
    let canonname = if canonname.is_empty() {
        None
    } else {
        Some(canonname)
    };

    Ok(ResolvedAddr::new(
        flags,
        family,
        socktype,
        protocol,
        socket_addr,
        canonname,
    ))
}

/// Decodes the inline socket address of a serialized `addrinfo` record.
///
/// `blob` is the record's `ai_addrlen` address bytes. The numeric fields are
/// read little-endian and each IPv4 `s_addr` is recovered by reversing its
/// bytes, undoing the `sfdnsres` double byte-swap documented on
/// [`decode_addrinfo_list`]; the IPv6 address bytes are taken verbatim.
fn decode_inline_sockaddr(family: c_int, blob: &[u8]) -> Result<SocketAddr, WireError> {
    const AF_INET: c_int = AddrFamily::Inet as c_int;
    const AF_INET6: c_int = AddrFamily::Inet6 as c_int;

    match family {
        AF_INET => {
            let mut reader = Reader::new(blob);
            // Skip sin_len and sin_family.
            reader.read_bytes(2)?;
            let port = reader.read_u16_le()?;
            let octets = reader.read_bytes(4)?;
            // s_addr is stored with its bytes reversed relative to the a.b.c.d
            // octet order; reverse them back.
            let ip = Ipv4Addr::from([octets[3], octets[2], octets[1], octets[0]]);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        AF_INET6 => {
            let mut reader = Reader::new(blob);
            // Skip sin6_family and the one byte of padding before sin6_port.
            reader.read_bytes(2)?;
            let port = reader.read_u16_le()?;
            let flowinfo = reader.read_u32_le()?;
            let addr_bytes = reader.read_bytes(16)?;
            let mut octets = [0u8; 16];
            octets.copy_from_slice(addr_bytes);
            let scope_id = reader.read_u32_le()?;
            let ip = Ipv6Addr::from(octets);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip, port, flowinfo, scope_id,
            )))
        }
        other => Err(WireError::UnsupportedAddrInfoFamily { family: other }),
    }
}

/// Reads a NUL-terminated wire string into an owned [`String`].
///
/// Wire strings are host names, expected to be ASCII; any byte that is not
/// valid UTF-8 is replaced with the Unicode replacement character so a stray
/// non-ASCII byte never fails an otherwise well-formed decode.
fn decode_cstr(reader: &mut Reader<'_>) -> Result<String, WireError> {
    let bytes = reader.read_cstr()?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// Interprets a fixed-size output buffer as a NUL-terminated C string.
///
/// The buffer is clamped to its first NUL byte — or kept whole when it carries
/// no terminator — and the prefix is decoded as lossy UTF-8.
fn decode_c_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// A bounds-checked, forward-only cursor over a borrowed byte buffer.
///
/// The reader decodes the big-endian integers and NUL-terminated strings of
/// the `sfdnsres` wire format. Each accessor advances the cursor past the
/// bytes it consumed; any read that would pass the end of the buffer fails
/// with [`WireError::UnexpectedEof`] and leaves the cursor unchanged.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Creates a reader positioned at the start of `buf`.
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Returns the number of unread bytes remaining in the buffer.
    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Reads the next `len` bytes and advances the cursor past them.
    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], WireError> {
        match self.pos.checked_add(len) {
            Some(end) if end <= self.buf.len() => {
                let bytes = &self.buf[self.pos..end];
                self.pos = end;
                Ok(bytes)
            }
            _ => Err(WireError::UnexpectedEof {
                needed: len - self.remaining(),
            }),
        }
    }

    /// Reads a big-endian `u16` and advances the cursor by two bytes.
    fn read_u16_be(&mut self) -> Result<u16, WireError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a big-endian `u32` and advances the cursor by four bytes.
    fn read_u32_be(&mut self) -> Result<u32, WireError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a big-endian `i32` and advances the cursor by four bytes.
    fn read_i32_be(&mut self) -> Result<i32, WireError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Reads a little-endian `u16` and advances the cursor by two bytes.
    ///
    /// The inline sockaddr fields of a serialized `addrinfo` record are stored
    /// little-endian — see [`decode_addrinfo_list`] for why.
    fn read_u16_le(&mut self) -> Result<u16, WireError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a little-endian `u32` and advances the cursor by four bytes.
    ///
    /// The inline sockaddr fields of a serialized `addrinfo` record are stored
    /// little-endian — see [`decode_addrinfo_list`] for why.
    fn read_u32_le(&mut self) -> Result<u32, WireError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Returns the next big-endian `u32` without advancing the cursor.
    ///
    /// Used by the `addrinfo` chain decoder to test for the record magic or
    /// the trailing zero terminator before committing to decode a record.
    fn peek_u32_be(&self) -> Result<u32, WireError> {
        let mut probe = Reader {
            buf: self.buf,
            pos: self.pos,
        };
        probe.read_u32_be()
    }

    /// Reads a NUL-terminated C string, returning the bytes before the NUL.
    ///
    /// The cursor advances past the terminating NUL. The returned slice is the
    /// raw string contents and never includes the NUL. A buffer that ends
    /// before a NUL is reached fails with [`WireError::UnterminatedString`].
    fn read_cstr(&mut self) -> Result<&'a [u8], WireError> {
        let tail = &self.buf[self.pos..];
        match tail.iter().position(|&b| b == 0) {
            Some(nul) => {
                let text = &tail[..nul];
                self.pos += nul + 1;
                Ok(text)
            }
            None => Err(WireError::UnterminatedString),
        }
    }
}
