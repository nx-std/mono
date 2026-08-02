//! Wire-format types and prefix decoding for HIPC.
//!
//! The bitfield structs in this module describe the on-the-wire byte layouts
//! the kernel reads and writes. Higher-level views (`Response`, request
//! building) live in the sibling `request`/`response` modules and consume
//! these types.

#![expect(unused_parens, clippy::identity_op)]

use core::mem::size_of;

use modular_bitfield::prelude::*;
use static_assertions::const_assert_eq;

/// Minimum buffer size required to decode the worst-case HIPC wire prefix:
/// [`Header`] (8) + [`SpecialHeader`] (4) + sender PID (`u64`, 8) = 20 bytes.
pub const MIN_PREFIX_BUF_SIZE: usize =
    size_of::<Header>() + size_of::<SpecialHeader>() + size_of::<u64>();

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
/// See [`RECV_LIST_WIRE_NONE`]. Wire mode `1` ("to message buffer") sits
/// between these two values; it is a server-side receive pattern with no
/// client-side constant.
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
