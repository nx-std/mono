//! Wire-format types, prefix decoding, and section writing for HIPC.
//!
//! The bitfield structs in this module describe the on-the-wire byte layouts
//! the kernel reads and writes. Higher-level views (`Request`, `Response`,
//! request and reply building) live in the sibling `request`/`response`
//! modules and consume these types.
//!
//! The two directions meet here. A request and a reply share the same prefix
//! shape, so [`parse_prefix`] decodes it once for both the inbound parser and
//! the response parser, and [`write_section`] emits a fixed-layout section for
//! both the request builder and the reply builder.
//!
//! Everything either direction needs and neither owns lives here for the same
//! reason: [`HipcPayload`] because both a request and a reply are parametric
//! over their data-words encoder, [`WriteError`] because both writers return
//! it, and [`HIPC_MAX_DESCRIPTORS`] because it is a property of the header
//! field rather than of a direction. Parking any of them in `request` would
//! make `response` import from its sibling to reach a fact neither one owns.

#![expect(clippy::identity_op)]

use core::mem::{
    size_of,
    size_of_val,
};

use modular_bitfield::prelude::*;
use static_assertions::const_assert_eq;

use crate::{
    cursor::Cursor,
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
};

/// Minimum buffer size required to decode the worst-case HIPC wire prefix:
/// [`Header`] (8) + [`SpecialHeader`] (4) + sender PID (`u64`, 8) = 20 bytes.
pub const MIN_PREFIX_BUF_SIZE: usize =
    size_of::<Header>() + size_of::<SpecialHeader>() + size_of::<u64>();

/// Maximum descriptors of any single kind that fit in an HIPC header
/// (each `num_*` field is 4 bits wide).
pub const HIPC_MAX_DESCRIPTORS: usize = 15;

/// Encoder for the in-band data-words region of an HIPC message.
///
/// HIPC owns the envelope (header, descriptors, handles); the payload owns
/// everything that goes into the data-words region. Higher-level protocols
/// (CMIF, TIPC) implement this trait for their wire-format bodies and attach
/// them to a request or a reply.
///
/// # Contract
///
/// The writer computes the data-words region as
/// `encoded_len().next_multiple_of(4)` and hands the impl a `dst` slice of
/// exactly that length. The region is **not** pre-zeroed: IPC is on the hot
/// path, and a global fill duplicates writes the impl already performs for its
/// sections. Bytes in `dst` that the impl does not overwrite (alignment slack,
/// trailing word padding) are transmitted as-is from the caller's TLS buffer;
/// well-behaved peers parse by structure layout and ignore them. Impls that
/// need deterministic wire bytes must zero those regions themselves. Encoding
/// is infallible: the destination slice is guaranteed large enough by
/// construction, and CMIF/TIPC wire-format bodies have no other failure modes.
pub trait HipcPayload {
    /// Byte length of the encoded payload, **unrounded**.
    ///
    /// HIPC rounds this up to the next 4-byte word boundary when sizing the
    /// data-words region.
    fn encoded_len(&self) -> usize;

    /// Writes the payload into the data-words region starting at `dst[0]`.
    ///
    /// `dst.len()` equals [`encoded_len`](Self::encoded_len) rounded up to a
    /// 4-byte multiple. The region is **not** pre-zeroed; see the trait-level
    /// [`Contract`](Self#contract) for the rules governing untouched bytes.
    fn write_to(&self, dst: &mut [u8]);
}

impl HipcPayload for () {
    #[inline]
    fn encoded_len(&self) -> usize {
        0
    }

    #[inline]
    fn write_to(&self, _: &mut [u8]) {}
}

impl HipcPayload for &[u8] {
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn write_to(&self, dst: &mut [u8]) {
        dst[..self.len()].copy_from_slice(self);
    }
}

/// Error returned when a destination buffer is too small to hold an encoded
/// message.
///
/// The layout is computed from the accumulated envelope (descriptors, handles,
/// recv-list, optional special header) plus the payload's data-words region
/// (sized as `payload.encoded_len().next_multiple_of(4)`). If that total
/// exceeds the caller-supplied destination buffer's `N` bytes, the writer
/// returns this instead of emitting a partial message. The fields report the
/// layout requirement and the buffer capacity, so a caller can either size its
/// IPC buffer to fit or drop descriptors and payload until it does.
///
/// Building a request or a reply is infallible: this only surfaces at
/// serialization time, when the destination buffer is known. Both directions
/// return it, which is why it lives here rather than beside either writer.
#[derive(Debug, thiserror::Error)]
#[error("message layout requires {needed} bytes, IPC buffer holds {limit}")]
pub struct WriteError {
    /// Total bytes the encoded layout requires.
    pub needed: usize,
    /// Capacity of the destination buffer.
    pub limit: usize,
}

impl ToResultCode for WriteError {
    fn to_rc(self) -> ResultCode {
        // Caught before the message went anywhere, so no peer saw it and there
        // is no service code to forward.
        GENERIC_ERROR
    }
}

/// 4-bit `recv_static_mode` wire encoding.
///
/// The Horizon kernel defines four cases for the receive-list field (yuzu/suyu
/// `ReceiveListCountType`):
///
/// - `0` - no recv-list; the server may not return Type-X pointer data.
/// - `1` - `ToMessageBuffer`: server places returned pointer data inside the
///   client's TLS message buffer; no wire slot is reserved.
/// - `2` - `ToSingleBuffer`: one wire slot the server may subdivide for all
///   returned pointer data.
/// - `2 + n` for `n ∈ 1..=13` - per-pointer recv-list of `n` entries; entry
///   `i` destinations the `i`-th out-pointer descriptor.
pub const RECV_LIST_WIRE_NONE: u8 = 0;
/// See [`RECV_LIST_WIRE_NONE`]. Wire mode `1`: returned pointer data goes into
/// the client's TLS message buffer, so no wire slot is reserved.
pub const RECV_LIST_WIRE_TO_MESSAGE_BUFFER: u8 = 1;
/// See [`RECV_LIST_WIRE_NONE`]. Wire mode `2` reserves one slot; `2 + n`
/// reserves `n`.
pub const RECV_LIST_WIRE_SINGLE_BUFFER: u8 = 2;

/// HIPC message header (8 bytes).
///
/// This is the first structure in every HIPC message and describes
/// the message type and the counts of various descriptors that follow.
///
/// # Bit layout
///
/// Fields are packed LSB-first across two little-endian 32-bit words:
///
/// ```text
/// Word 0 (bits 0..31)
///  0                             15 16       19 20       23 24       27 28       31
/// ╔══════════════════════════════╦═══════════╤═══════════╦═══════════╤═══════════╗
/// ║         message_type         ║   send    │   send    ║   recv    │   exch    ║
/// ║            (16)              ║  statics  │  buffers  ║  buffers  │  buffers  ║
/// ║                              ║    (4)    │    (4)    ║    (4)    │    (4)    ║
/// ╚══════════════════════════════╩═══════════╧═══════════╩═══════════╧═══════════╝
///
/// Word 1 (bits 32..63) - all internal field splits fall inside a byte
///  32                41 42        45 46            51 52                  62     63
/// ╔══════════════════╤════════════╤════════════════╤══════════════════════╤══════╗
/// ║  num_data_words  │  recv_st.  │    padding     │   recv_list_offset   │ `S`  ║
/// ║       (10)       │   mode (4) │      (6)       │     (11, unused)     │ (1)  ║
/// ║                  │            │                │                      │      ║
/// ╚══════════════════╧════════════╧════════════════╧══════════════════════╧══════╝
///   `S` = `has_special_header`
/// ```
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct Header {
    /// Message type. Command type for CMIF.
    pub message_type: B16,
    /// Number of send static descriptors.
    pub num_send_statics: B4,
    /// Number of send buffer descriptors.
    pub num_send_buffers: B4,
    /// Number of receive buffer descriptors.
    pub num_recv_buffers: B4,
    /// Number of exchange buffer descriptors.
    pub num_exch_buffers: B4,
    /// Number of data words in the message.
    pub num_data_words: B10,
    /// Receive static mode (0 = none, 2 = auto, 2+n = n entries).
    pub recv_static_mode: B4,
    /// Padding bits.
    #[skip]
    __padding: B6,
    /// Unused on current Horizon; written as 0.
    #[skip]
    __recv_list_offset: B11,
    /// Whether a special header follows.
    pub has_special_header: bool,
}

const_assert_eq!(size_of::<Header>(), 8);

/// HIPC special header (4 bytes).
///
/// Present when the message includes PID or handles.
///
/// # Bit layout
///
/// Fields are packed LSB-first across a single little-endian 32-bit word:
///
/// ```text
///  0    1              4 5            8 9                                             31
/// ╔═════╤══════════════╤══════════════╤════════════════════════════════════════════════╗
/// ║ `P` │  num_copy_   │  num_move_   │                    padding                     ║
/// ║     │   handles    │   handles    │                     (23)                       ║
/// ║ (1) │     (4)      │     (4)      │                                                ║
/// ╚═════╧══════════════╧══════════════╧════════════════════════════════════════════════╝
///   `P` = `send_pid`
/// ```
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct SpecialHeader {
    /// Whether to send the process ID.
    pub send_pid: bool,
    /// Number of copy handles.
    pub num_copy_handles: B4,
    /// Number of move handles.
    pub num_move_handles: B4,
    /// Padding bits.
    #[skip]
    __padding: B23,
}

const_assert_eq!(size_of::<SpecialHeader>(), 4);

/// Static descriptor for send/receive static pointers (8 bytes).
///
/// Used for small data transfers via static buffers.
/// The address is split across multiple fields for encoding.
///
/// # Bit layout
///
/// Fields are packed LSB-first across two little-endian 32-bit words:
///
/// ```text
/// Word 0 (bits 0..31)
///  0          5 6        11 12     15 16                            31
/// ╔════════════╤═══════════╤══════════╦═══════════════════════════════╗
/// ║   index    │ address_  │ address_ ║             size              ║
/// ║    (6)     │ high (6)  │ mid (4)  ║             (16)              ║
/// ╚════════════╧═══════════╧══════════╩═══════════════════════════════╝
///
/// Word 1 (bits 32..63)
///  32                                                                63
/// ╔═════════════════════════════════════════════════════════════════════╗
/// ║                            address_low                              ║
/// ║                                (32)                                 ║
/// ╚═════════════════════════════════════════════════════════════════════╝
///
/// The 64-bit buffer address is reassembled as:
///   `address = (address_high << 36) | (address_mid << 32) | address_low`
/// ```
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct StaticDescriptor {
    /// Index for matching send/receive pairs.
    pub index: B6,
    /// Address bits 36-41.
    pub address_high: B6,
    /// Address bits 32-35.
    pub address_mid: B4,
    /// Size of the buffer.
    pub size: B16,
    /// Address bits 0-31.
    pub address_low: B32,
}

const_assert_eq!(size_of::<StaticDescriptor>(), 8);

impl StaticDescriptor {
    /// Creates a static descriptor for sending data.
    pub(crate) fn new_send(buffer: *const u8, size: usize, index: u8) -> Self {
        let addr = buffer as usize;
        Self::new()
            .with_index(index & 0x3F)
            .with_address_low(addr as u32)
            .with_address_mid(((addr >> 32) & 0xF) as u8)
            .with_address_high(((addr >> 36) & 0x3F) as u8)
            .with_size(size as u16)
    }

    /// Reconstructs the full address from the split fields.
    pub fn address(&self) -> usize {
        self.address_low() as usize
            | ((self.address_mid() as usize) << 32)
            | ((self.address_high() as usize) << 36)
    }
}

/// Buffer descriptor for send/receive/exchange buffers (12 bytes).
///
/// Used for larger data transfers via mapped buffers.
/// Both address and size are split across multiple fields.
///
/// # Bit layout
///
/// Fields are packed LSB-first across three little-endian 32-bit words:
///
/// ```text
/// Word 0 (bits 0..31)
///  0                                                                  31
/// ╔═════════════════════════════════════════════════════════════════════╗
/// ║                              size_low                               ║
/// ║                                (32)                                 ║
/// ╚═════════════════════════════════════════════════════════════════════╝
///
/// Word 1 (bits 32..63)
///  32                                                                 63
/// ╔═════════════════════════════════════════════════════════════════════╗
/// ║                             address_low                             ║
/// ║                                (32)                                 ║
/// ╚═════════════════════════════════════════════════════════════════════╝
///
/// Word 2 (bits 64..95)
///  64 65 66                                  87 88     91 92         95
/// ╔═════╤══════════════════════════════════════╦═════════╤═════════════╗
/// ║mode │            address_high              ║size_high│ address_mid ║
/// ║ (2) │                (22)                  ║   (4)   │     (4)     ║
/// ╚═════╧══════════════════════════════════════╩═════════╧═════════════╝
///
/// The 64-bit buffer address and size are reassembled as:
///   `address = (address_high << 36) | (address_mid << 32) | address_low`
///   `size    = (size_high    << 32) | size_low`
/// ```
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BufferDescriptor {
    /// Size bits 0-31.
    pub size_low: B32,
    /// Address bits 0-31.
    pub address_low: B32,
    /// Buffer mode (Normal, NonSecure, etc.).
    pub mode: BufferMode,
    /// Address bits 36-57.
    pub address_high: B22,
    /// Size bits 32-35.
    pub size_high: B4,
    /// Address bits 32-35.
    pub address_mid: B4,
}

const_assert_eq!(size_of::<BufferDescriptor>(), 12);

impl BufferDescriptor {
    /// Creates a buffer descriptor with the given mode.
    pub(crate) fn new_buffer(buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        let addr = buffer as usize;
        Self::new()
            .with_mode(mode)
            .with_address_low(addr as u32)
            .with_address_mid(((addr >> 32) & 0xF) as u8)
            .with_address_high(((addr >> 36) & 0x3FFFFF) as u32)
            .with_size_low(size as u32)
            .with_size_high(((size >> 32) & 0xF) as u8)
    }

    /// Reconstructs the full address from the split fields.
    pub fn address(&self) -> usize {
        self.address_low() as usize
            | ((self.address_mid() as usize) << 32)
            | ((self.address_high() as usize) << 36)
    }

    /// Reconstructs the full size from the split fields.
    pub fn size(&self) -> usize {
        self.size_low() as usize | ((self.size_high() as usize) << 32)
    }
}

/// Buffer transfer mode for HIPC buffer descriptors.
///
/// Controls how the kernel maps the buffer between processes.
#[derive(BitfieldSpecifier, Debug, Clone, Copy, PartialEq, Eq)]
#[bits = 2]
pub enum BufferMode {
    /// Normal buffer mapping.
    Normal = 0,
    /// Non-secure memory area.
    NonSecure = 1,
    /// Invalid/device memory (cannot be mapped).
    Invalid = 2,
    /// Non-device memory area.
    NonDevice = 3,
}

/// Receive list entry for static receive buffers (8 bytes).
///
/// # Bit layout
///
/// Fields are packed LSB-first across two little-endian 32-bit words:
///
/// ```text
/// Word 0 (bits 0..31)
///  0                                                                  31
/// ╔═════════════════════════════════════════════════════════════════════╗
/// ║                             address_low                             ║
/// ║                                (32)                                 ║
/// ╚═════════════════════════════════════════════════════════════════════╝
///
/// Word 1 (bits 32..63)
///  32                              47 48                              63
/// ╔══════════════════════════════════╦══════════════════════════════════╗
/// ║           address_high           ║               size               ║
/// ║              (16)                ║              (16)                ║
/// ╚══════════════════════════════════╩══════════════════════════════════╝
///
/// The 48-bit buffer address is reassembled as:
///   `address = (address_high << 32) | address_low`
/// ```
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct RecvListEntry {
    /// Address bits 0-31.
    pub address_low: B32,
    /// Address bits 32-47.
    pub address_high: B16,
    /// Size of the buffer.
    pub size: B16,
}

const_assert_eq!(size_of::<RecvListEntry>(), 8);

impl RecvListEntry {
    /// Creates a _receive list entry_.
    pub(crate) fn new_recv(buffer: *mut u8, size: usize) -> Self {
        let addr = buffer as usize;
        Self::new()
            .with_address_low(addr as u32)
            .with_address_high(((addr >> 32) & 0xFFFF) as u16)
            .with_size(size as u16)
    }

    /// Reconstructs the full address from the split fields.
    pub fn address(&self) -> usize {
        self.address_low() as usize | ((self.address_high() as usize) << 32)
    }
}

/// Message type for HIPC requests.
///
/// This is a newtype wrapper around the raw 16-bit message type field.
/// Protocol-specific command types (CMIF, TIPC) implement `From` to convert
/// to this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MessageType(u16);

impl MessageType {
    /// Creates a message type from a raw value.
    #[inline]
    pub(crate) const fn from_raw(value: u16) -> Self {
        Self(value)
    }

    /// Returns the raw u16 value.
    #[inline]
    pub const fn to_raw(self) -> u16 {
        self.0
    }
}

/// Decodes the wire-level prefix (header + optional special header + optional PID)
/// from a fixed-size IPC buffer slice, returning the parsed [`Prefix`] and the
/// remainder of the buffer after the prefix bytes.
///
/// # Wire layout
///
/// Three valid prefix shapes, selected by two bits in the wire bytes:
///
/// ```text
/// offset:   0                  8              12             20
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape A:  │      Header      │ ⋯ payload ⋯
///           │     (8 bytes)    │
///           └──────────────────┘
///           has_special_header = 0
///
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape B:  │      Header      │ SpecialHdr   │ ⋯ payload ⋯
///           │     (8 bytes)    │  (4 bytes)   │
///           └──────────────────┴──────────────┘
///           has_special_header = 1, send_pid = 0
///
///           ┌──────────────────┬──────────────┬──────────────┐
/// shape C:  │      Header      │ SpecialHdr   │  ProcessId   │ ⋯ payload ⋯
///           │     (8 bytes)    │  (4 bytes)   │  (8 bytes)   │
///           └──────────────────┴──────────────┴──────────────┘
///           has_special_header = 1, send_pid = 1
/// ```
///
/// Infallible: the caller supplies a buffer at least [`MIN_PREFIX_BUF_SIZE`]
/// bytes long (the worst-case prefix), enforced at monomorphization via a
/// `const` assertion on `N`.
pub(crate) fn parse_prefix<const N: usize>(buf: &[u8; N]) -> (Prefix, &[u8]) {
    // Compile-time check: the buffer must fit the worst-case prefix
    const {
        assert!(
            N >= MIN_PREFIX_BUF_SIZE,
            "parse_prefix buffer must be at least MIN_PREFIX_BUF_SIZE bytes",
        );
    }

    let cursor = Cursor::new(buf);
    // SAFETY: the `const` block above rejects any `N` below
    // `MIN_PREFIX_BUF_SIZE`, which is `size_of::<Header>()` plus the two
    // optional sections. The cursor is at offset 0 and a `Header` is the
    // smallest of the three summands, so `N` bytes remain and the read fits.
    let (header, cursor) = cursor
        .read::<Header>()
        .expect("internal: TLR buffer fits HIPC header");
    if !header.has_special_header() {
        return (
            Prefix {
                header: *header,
                extras: None,
            },
            cursor.remaining(),
        );
    }

    // SAFETY: the `const` block above rejects any `N` below
    // `MIN_PREFIX_BUF_SIZE`, which counts a `Header` and a `SpecialHeader` and
    // a PID slot. Exactly one `Header` has been read, so at least the latter
    // two remain and this read fits.
    let (special_hdr, cursor) = cursor
        .read::<SpecialHeader>()
        .expect("internal: TLR buffer fits special header");

    let (pid, cursor) = if special_hdr.send_pid() {
        // SAFETY: the `const` block above rejects any `N` below
        // `MIN_PREFIX_BUF_SIZE`, which counts a `Header` and a `SpecialHeader`
        // and a PID slot. One of each of the first two has been read, so the
        // PID slot remains and this read fits.
        let (pid_ref, cursor) = cursor
            .read::<ProcessId>()
            .expect("internal: TLR buffer fits PID");
        (Some(*pid_ref), cursor)
    } else {
        (None, cursor)
    };

    let prefix = Prefix {
        header: *header,
        extras: Some(Extras {
            num_copy_handles: special_hdr.num_copy_handles(),
            num_move_handles: special_hdr.num_move_handles(),
            pid,
        }),
    };

    (prefix, cursor.remaining())
}

/// Parsed wire-level prefix shared by requests and responses.
///
/// The three valid prefix shapes on the wire (header only, header + special
/// header, header + special header + PID) collapse into a single value with
/// nested `Option`s, so a consumer cannot read the discriminant bits
/// independently from the bytes they describe.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Prefix {
    /// Wire-level message header (always present).
    pub header: Header,
    /// Optional special header + PID payload. `None` ⇔ the header's
    /// `has_special_header` bit is clear.
    pub extras: Option<Extras>,
}

/// Decoded contents of the special header section.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Extras {
    /// Number of copy handles (≤ 15, bound by the `B4` bitfield width).
    pub num_copy_handles: u8,
    /// Number of move handles (≤ 15, bound by the `B4` bitfield width).
    pub num_move_handles: u8,
    /// Sender's process ID, if the `send_pid` bit was set on the wire.
    pub pid: Option<ProcessId>,
}

/// Sender's process ID as decoded from the HIPC special header payload.
///
/// The HIPC wire layout stores the PID as a native-endian `u64` immediately
/// after the special header, so the type derives the zerocopy byte-conversion
/// traits and is decoded in place rather than copied out.
///
/// The kernel fills this slot itself on transmission, so a server that reads it
/// off an inbound request learns which process sent it without trusting the
/// sender to say.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(transparent)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Returns the raw process ID.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// Writes `value`'s bytes into the prefix of `buf` and returns the tail.
///
/// Both writers total their whole layout and compare it against the
/// destination before emitting a single section, so by the time a section is
/// written the space for it is already accounted for. That is what makes this
/// function infallible, and it is a precondition of calling it rather than
/// something it checks.
#[inline]
pub(crate) fn write_section<'a, T>(buf: &'a mut [u8], value: &T) -> &'a mut [u8]
where
    T: zerocopy::IntoBytes + zerocopy::Immutable + ?Sized,
{
    // SAFETY: `HipcRequest::write_to` and `HipcReply::write_to` both compute
    // the total encoded length and return `WriteError` unless the destination
    // holds it, before emitting any section. Every call here is therefore
    // preceded by a check covering this section, so `buf` holds at least
    // `size_of_val(value)` bytes and the split cannot panic.
    let (buf, tail) = buf.split_at_mut(size_of_val(value));
    // SAFETY: `split_at_mut` above returned a `buf` of exactly
    // `size_of_val(value)` bytes, which is the length `write_to` requires, so
    // the only error it can report is unreachable here.
    value
        .write_to(buf)
        .expect("internal: edge check guarantees buffer fits");
    tail
}
