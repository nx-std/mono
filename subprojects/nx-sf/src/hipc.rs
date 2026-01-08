//! HIPC (Horizon Inter-Process Communication) protocol implementation.
//!
//! HIPC is the low-level message serialization protocol for IPC on Nintendo
//! Switch's Horizon OS. It defines the wire format for passing data, handles,
//! and buffer descriptors between processes via kernel supervisor calls.
//!
//! # Protocol Stack
//!
//! HIPC is the transport layer in the Horizon IPC stack:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  CMIF or TIPC                       │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC  ← this module                │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! Two command protocols build on HIPC:
//! - **CMIF** (Command Message Interface Format): Original protocol with domain
//!   support for multiplexing objects. Uses magic `"SFCI"`/`"SFCO"` headers.
//! - **TIPC** (Tiny IPC): Simplified protocol introduced in HOS 12.0.0. No
//!   domains, command ID stored directly in HIPC message type field.
//!
//! # Message Location
//!
//! Messages are written to the Thread Local Region (TLR) at offset 0x0. Each
//! thread has a 0x200-byte IPC buffer in its TLS area. The kernel reads request
//! messages from and writes response messages to this buffer.
//!
//! # Message Layout
//!
//! ```text
//! Offset  Size   Field
//! ──────────────────────────────────────────────────────────────
//! 0x00    0x08   Header (message type, descriptor counts)
//! 0x08    0x04   SpecialHeader (optional: PID flag, handle counts)
//! 0x0C    0x08   ProcessId (optional: if send_pid is set)
//!         var    Copy Handles (4 bytes × num_copy_handles)
//!         var    Move Handles (4 bytes × num_move_handles)
//!         var    Send Statics / Type X (8 bytes each)
//!         var    Send Buffers / Type A (12 bytes each)
//!         var    Recv Buffers / Type B (12 bytes each)
//!         var    Exch Buffers / Type W (12 bytes each)
//!         var    Data Words (raw payload, 4 bytes each)
//!         var    Recv List / Type C (8 bytes each)
//! ──────────────────────────────────────────────────────────────
//! ```
//!
//! # Descriptor Types (Switchbrew Naming)
//!
//! HIPC defines several descriptor types for transferring data:
//!
//! | Type | Name          | Direction      | Mechanism        | Size Limit |
//! |------|---------------|----------------|------------------|------------|
//! | X    | Send Static   | Client→Server  | Pointer (copy)   | 64 KB      |
//! | A    | Send Buffer   | Client→Server  | Memory mapping   | 4 GB       |
//! | B    | Recv Buffer   | Server→Client  | Memory mapping   | 4 GB       |
//! | W    | Exch Buffer   | Bidirectional  | Memory mapping   | 4 GB       |
//! | C    | Recv List     | Server→Client  | Pointer (copy)   | 64 KB      |
//!
//! ## Pointer Descriptors (Type X / Send Static)
//!
//! Used for small data transfers. The kernel copies data between process
//! address spaces. Each descriptor has a 6-bit index for matching send/receive
//! pairs. Maximum transfer size is 64 KB (16-bit size field).
//!
//! ## Buffer Descriptors (Types A/B/W)
//!
//! Used for larger data transfers via memory mapping:
//!
//! - **Send (A)**: Client memory mapped read-only (R--) into server
//! - **Recv (B)**: Client memory mapped read-write (RW-) into server
//! - **Exchange (W)**: Same buffer for both directions (RW-)
//!
//! Memory mappings are automatically released when the kernel processes the
//! reply message. Buffer descriptors support sizes up to 4 GB (36-bit size).
//!
//! ## Receive List (Type C)
//!
//! Pre-allocated client buffers for receiving pointer data. The server writes
//! to these using send statics. The `recv_static_mode` header field controls:
//! - Mode 0: No receive list
//! - Mode 2: Auto-calculate count from send statics
//! - Mode 2+n: Exactly n receive list entries
//!
//! # Handle Passing
//!
//! Kernel handles (sessions, events, shared memory, etc.) can be passed:
//!
//! - **Copy Handle**: The kernel duplicates the handle. Both processes retain
//!   independent references to the same kernel object.
//! - **Move Handle**: Ownership transfers to the receiver. The sender's handle
//!   becomes invalid after the call.
//!
//! # Address Encoding
//!
//! 64-bit addresses are split across bitfields to fit the packed descriptor
//! format. The encoding varies by descriptor type:
//!
//! **Static Descriptor (8 bytes):**
//! ```text
//! Bits 0-5:   index (6 bits)
//! Bits 6-11:  address[36:41] (6 bits)
//! Bits 12-15: address[32:35] (4 bits)
//! Bits 16-31: size (16 bits, max 64KB)
//! Bits 32-63: address[0:31] (32 bits)
//! ```
//!
//! **Buffer Descriptor (12 bytes):**
//! ```text
//! Bits 0-31:  size[0:31] (32 bits)
//! Bits 32-63: address[0:31] (32 bits)
//! Bits 64-65: mode (2 bits)
//! Bits 66-87: address[36:57] (22 bits)
//! Bits 88-91: size[32:35] (4 bits)
//! Bits 92-95: address[32:35] (4 bits)
//! ```
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/hipc.h` (fincs, SciresM)

use core::{mem::size_of, ptr::NonNull};

use modular_bitfield::prelude::*;
use nx_svc::raw::Handle as RawHandle;
use static_assertions::const_assert_eq;

/// Sentinel value indicating automatic receive static count calculation.
const AUTO_RECV_STATIC: u8 = u8::MAX;

/// Sentinel value indicating no PID in the response.
const RESPONSE_NO_PID: u32 = u32::MAX;

/// Calculates the layout of descriptor arrays within a request buffer.
///
/// # Safety
///
/// `base` must point to a valid buffer with enough space for all descriptors.
pub unsafe fn calc_request_layout<'a>(meta: &Metadata, mut base: *mut u8) -> Request<'a> {
    let copy_handles = if meta.num_copy_handles > 0 {
        let ptr = base as *mut RawHandle;
        let len = meta.num_copy_handles;
        // SAFETY: Caller guarantees buffer has space; advancing within bounds.
        base = unsafe { base.add(len * size_of::<RawHandle>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let move_handles = if meta.num_move_handles > 0 {
        let ptr = base as *mut RawHandle;
        let len = meta.num_move_handles;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<RawHandle>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let send_statics = if meta.num_send_statics > 0 {
        let ptr = base as *mut StaticDescriptor;
        let len = meta.num_send_statics;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<StaticDescriptor>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let send_buffers = if meta.num_send_buffers > 0 {
        let ptr = base as *mut BufferDescriptor;
        let len = meta.num_send_buffers;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<BufferDescriptor>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let recv_buffers = if meta.num_recv_buffers > 0 {
        let ptr = base as *mut BufferDescriptor;
        let len = meta.num_recv_buffers;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<BufferDescriptor>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let exch_buffers = if meta.num_exch_buffers > 0 {
        let ptr = base as *mut BufferDescriptor;
        let len = meta.num_exch_buffers;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<BufferDescriptor>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let data_words = if meta.num_data_words > 0 {
        let ptr = base as *mut u32;
        let len = meta.num_data_words;
        // SAFETY: Previous section consumed; advancing within bounds.
        base = unsafe { base.add(len * size_of::<u32>()) };
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    let recv_list = if let Some(mode) = meta.recv_static_mode {
        let ptr = base as *mut RecvListEntry;
        let len = mode.as_count();
        // SAFETY: ptr is valid, properly aligned, and len matches metadata.
        unsafe { core::slice::from_raw_parts_mut(ptr, len) }
    } else {
        &mut []
    };

    Request {
        send_statics,
        send_buffers,
        recv_buffers,
        exch_buffers,
        data_words,
        recv_list,
        copy_handles,
        move_handles,
    }
}

/// Builds an HIPC request in the given buffer.
///
/// # Safety
///
/// `base` must point to a valid buffer (typically TLS) with enough space.
pub unsafe fn make_request<'a>(base: NonNull<u8>, meta: Metadata) -> Request<'a> {
    let has_special_header = meta.has_special_header();
    let recv_static_mode = meta.recv_static_mode.map_or(0, |m| m.as_count() as u8);
    let header = Header::new()
        .with_message_type(meta.message_type.to_raw())
        .with_num_send_statics(meta.num_send_statics as u8)
        .with_num_send_buffers(meta.num_send_buffers as u8)
        .with_num_recv_buffers(meta.num_recv_buffers as u8)
        .with_num_exch_buffers(meta.num_exch_buffers as u8)
        .with_num_data_words(meta.num_data_words as u16)
        .with_recv_static_mode(recv_static_mode)
        .with_recv_list_offset(0)
        .with_has_special_header(has_special_header);

    let header_ptr = base.as_ptr() as *mut Header;
    // SAFETY: Caller guarantees `base` is valid and properly sized.
    unsafe { header_ptr.write(header) };
    // SAFETY: Advancing past header; cursor remains within buffer.
    let mut cursor = unsafe { base.as_ptr().add(size_of::<Header>()) };

    if has_special_header {
        let special = SpecialHeader::new()
            .with_send_pid(meta.send_pid)
            .with_num_copy_handles(meta.num_copy_handles as u8)
            .with_num_move_handles(meta.num_move_handles as u8);

        let special_ptr = cursor as *mut SpecialHeader;
        // SAFETY: cursor is within bounds after header.
        unsafe { special_ptr.write(special) };
        // SAFETY: Advancing past special header; cursor remains valid.
        cursor = unsafe { cursor.add(size_of::<SpecialHeader>()) };

        if meta.send_pid {
            // SAFETY: Reserving space for PID; cursor remains valid.
            cursor = unsafe { cursor.add(size_of::<u64>()) };
        }
    }

    // SAFETY: cursor now points past all headers to descriptor region.
    unsafe { calc_request_layout(&meta, cursor) }
}

/// Parses an incoming HIPC request from a buffer.
///
/// # Safety
///
/// `base` must point to a valid HIPC message buffer.
pub unsafe fn parse_request<'a>(base: NonNull<u8>) -> ParsedRequest<'a> {
    let base = base.as_ptr();
    let header_ptr = base as *const Header;

    // SAFETY: Caller guarantees `base` points to a valid HIPC message.
    let header = unsafe { header_ptr.read() };
    // SAFETY: Advancing past header; cursor remains within message.
    let mut cursor = unsafe { base.add(size_of::<Header>()) };

    let mut pid = 0u64;
    let mut num_copy_handles = 0usize;
    let mut num_move_handles = 0usize;
    let mut send_pid = false;

    let recv_static_mode = RecvStaticMode::from_raw(header.recv_static_mode());

    if header.has_special_header() {
        let special_ptr = cursor as *const SpecialHeader;
        // SAFETY: cursor points to special header within the message.
        let special = unsafe { special_ptr.read() };
        // SAFETY: Advancing past special header; cursor remains valid.
        cursor = unsafe { cursor.add(size_of::<SpecialHeader>()) };

        send_pid = special.send_pid();
        num_copy_handles = special.num_copy_handles() as usize;
        num_move_handles = special.num_move_handles() as usize;

        if special.send_pid() {
            // SAFETY: PID follows special header when send_pid is set.
            pid = unsafe { (cursor as *const u64).read() };
            // SAFETY: Advancing past PID; cursor remains valid.
            cursor = unsafe { cursor.add(size_of::<u64>()) };
        }
    }

    let meta = Metadata {
        message_type: MessageType::from_raw(header.message_type()),
        num_send_statics: header.num_send_statics() as usize,
        num_send_buffers: header.num_send_buffers() as usize,
        num_recv_buffers: header.num_recv_buffers() as usize,
        num_exch_buffers: header.num_exch_buffers() as usize,
        num_data_words: header.num_data_words() as usize,
        recv_static_mode,
        send_pid,
        num_copy_handles,
        num_move_handles,
    };

    // SAFETY: cursor now points past headers to descriptor region.
    let data = unsafe { calc_request_layout(&meta, cursor) };

    ParsedRequest { meta, data, pid }
}

/// Parses an HIPC response from a buffer.
///
/// # Safety
///
/// `base` must point to a valid HIPC response buffer.
pub unsafe fn parse_response<'a>(base: NonNull<u8>) -> Response<'a> {
    let base = base.as_ptr();
    let header_ptr = base as *const Header;

    // SAFETY: Caller guarantees `base` points to a valid HIPC response.
    let header = unsafe { header_ptr.read() };
    // SAFETY: Advancing past header; cursor remains within response.
    let mut cursor = unsafe { base.add(size_of::<Header>()) };

    let mut pid = RESPONSE_NO_PID as u64;
    let mut num_copy_handles = 0usize;
    let mut num_move_handles = 0usize;

    if header.has_special_header() {
        let special_ptr = cursor as *const SpecialHeader;
        // SAFETY: cursor points to special header within the response.
        let special = unsafe { special_ptr.read() };
        // SAFETY: Advancing past special header; cursor remains valid.
        cursor = unsafe { cursor.add(size_of::<SpecialHeader>()) };

        num_copy_handles = special.num_copy_handles() as usize;
        num_move_handles = special.num_move_handles() as usize;

        if special.send_pid() {
            // SAFETY: PID follows special header when send_pid is set.
            pid = unsafe { (cursor as *const u64).read() };
            // SAFETY: Advancing past PID; cursor remains valid.
            cursor = unsafe { cursor.add(size_of::<u64>()) };
        }
    }

    let copy_handles = if num_copy_handles > 0 {
        let ptr = cursor as *const RawHandle;
        // SAFETY: Advancing within response buffer bounds.
        cursor = unsafe { cursor.add(num_copy_handles * size_of::<RawHandle>()) };
        // SAFETY: ptr is valid, aligned, and count matches header.
        unsafe { core::slice::from_raw_parts(ptr, num_copy_handles) }
    } else {
        &[]
    };

    let move_handles = if num_move_handles > 0 {
        let ptr = cursor as *const RawHandle;
        // SAFETY: Advancing within response buffer bounds.
        cursor = unsafe { cursor.add(num_move_handles * size_of::<RawHandle>()) };
        // SAFETY: ptr is valid, aligned, and count matches header.
        unsafe { core::slice::from_raw_parts(ptr, num_move_handles) }
    } else {
        &[]
    };

    let num_statics = header.num_send_statics() as usize;
    let statics = if num_statics > 0 {
        let ptr = cursor as *const StaticDescriptor;
        // SAFETY: Advancing within response buffer bounds.
        cursor = unsafe { cursor.add(num_statics * size_of::<StaticDescriptor>()) };
        // SAFETY: ptr is valid, aligned, and count matches header.
        unsafe { core::slice::from_raw_parts(ptr, num_statics) }
    } else {
        &[]
    };

    let num_data_words = header.num_data_words() as usize;
    let data_words = if num_data_words > 0 {
        let ptr = cursor as *const u32;
        // SAFETY: ptr is valid, aligned, and count matches header.
        unsafe { core::slice::from_raw_parts(ptr, num_data_words) }
    } else {
        &[]
    };

    Response {
        pid,
        statics,
        data_words,
        copy_handles,
        move_handles,
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

/// HIPC message header (8 bytes).
///
/// This is the first structure in every HIPC message and describes
/// the message type and the counts of various descriptors that follow.
#[bitfield]
#[derive(Debug, Clone, Copy, Default)]
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
    /// Offset to receive list (unused).
    pub recv_list_offset: B11,
    /// Whether a special header follows.
    pub has_special_header: bool,
}

const_assert_eq!(size_of::<Header>(), 8);

/// HIPC special header (4 bytes).
///
/// Present when the message includes PID or handles.
#[bitfield]
#[derive(Debug, Clone, Copy, Default)]
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
#[bitfield]
#[derive(Debug, Clone, Copy, Default)]
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
    pub fn new_send(buffer: *const u8, size: usize, index: u8) -> Self {
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
#[bitfield]
#[derive(Debug, Clone, Copy, Default)]
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
    pub fn new_buffer(buffer: *const u8, size: usize, mode: BufferMode) -> Self {
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

/// Receive list entry for static receive buffers (8 bytes).
#[bitfield]
#[derive(Debug, Clone, Copy, Default)]
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
    pub fn new_recv(buffer: *mut u8, size: usize) -> Self {
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

/// High-level metadata for constructing a request.
///
/// This structure describes the layout of an HIPC request
/// without containing the actual data.
#[derive(Debug, Clone, Copy, Default)]
pub struct Metadata {
    /// Message type (protocol-specific command type).
    pub message_type: MessageType,
    /// Number of send static descriptors.
    pub num_send_statics: usize,
    /// Number of send buffer descriptors.
    pub num_send_buffers: usize,
    /// Number of receive buffer descriptors.
    pub num_recv_buffers: usize,
    /// Number of exchange buffer descriptors.
    pub num_exch_buffers: usize,
    /// Number of data words.
    pub num_data_words: usize,
    /// Receive static mode (`None` means no receive list).
    pub recv_static_mode: Option<RecvStaticMode>,
    /// Whether to send the process ID.
    pub send_pid: bool,
    /// Number of copy handles.
    pub num_copy_handles: usize,
    /// Number of move handles.
    pub num_move_handles: usize,
}

impl Metadata {
    /// Returns whether this metadata requires a special header.
    ///
    /// A special header is needed when sending a PID or any handles.
    #[inline]
    pub const fn has_special_header(&self) -> bool {
        self.send_pid || self.num_copy_handles > 0 || self.num_move_handles > 0
    }
}

/// Message type for HIPC requests.
///
/// This is a newtype wrapper around the raw 16-bit message type field.
/// Protocol-specific command types (CMIF, TIPC) implement `From` to convert
/// to this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// Controls how the receive list (Type C descriptors) is handled in the message.
///
/// Use `Option<RecvStaticMode>` where `None` means no receive list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvStaticMode {
    /// Auto-calculate count from send statics (mode 2).
    Auto,
    /// Explicit count of receive list entries (mode 2+n, where n >= 1).
    Explicit(u8),
}

impl RecvStaticMode {
    /// Parses the _receive static_ mode from the raw header value.
    ///
    /// Returns `None` for mode 0/1 (no receive list).
    #[inline]
    pub const fn from_raw(mode: u8) -> Option<Self> {
        match mode {
            0 | 1 => None,
            2 => Some(Self::Auto),
            n => Some(Self::Explicit(n - 2)),
        }
    }

    /// Returns the count as `usize` for buffer allocation.
    ///
    /// - `Auto` → `AUTO_RECV_STATIC` (255)
    /// - `Explicit(n)` → n
    #[inline]
    pub const fn as_count(self) -> usize {
        match self {
            Self::Auto => AUTO_RECV_STATIC as usize,
            Self::Explicit(n) => n as usize,
        }
    }
}

/// Pointers into a request buffer being constructed.
///
/// This structure holds mutable slices to the various descriptor
/// arrays in an HIPC message being built.
#[derive(Debug)]
pub struct Request<'a> {
    /// Send static descriptors.
    pub send_statics: &'a mut [StaticDescriptor],
    /// Send buffer descriptors.
    pub send_buffers: &'a mut [BufferDescriptor],
    /// Receive buffer descriptors.
    pub recv_buffers: &'a mut [BufferDescriptor],
    /// Exchange buffer descriptors.
    pub exch_buffers: &'a mut [BufferDescriptor],
    /// Data words (raw message data).
    pub data_words: &'a mut [u32],
    /// Receive list entries.
    pub recv_list: &'a mut [RecvListEntry],
    /// Copy handle slots.
    pub copy_handles: &'a mut [RawHandle],
    /// Move handle slots.
    pub move_handles: &'a mut [RawHandle],
}

/// Parsed HIPC response from the server.
#[derive(Debug)]
pub struct Response<'a> {
    /// Process ID from the response (or RESPONSE_NO_PID).
    pub pid: u64,
    /// Static descriptors in the response.
    pub statics: &'a [StaticDescriptor],
    /// Data words (raw response data).
    pub data_words: &'a [u32],
    /// Copy handles received.
    pub copy_handles: &'a [RawHandle],
    /// Move handles received.
    pub move_handles: &'a [RawHandle],
}

/// Parsed incoming HIPC request (for server-side use).
#[derive(Debug)]
pub struct ParsedRequest<'a> {
    /// Request metadata.
    pub meta: Metadata,
    /// Request data pointers.
    pub data: Request<'a>,
    /// Process ID of the sender.
    pub pid: u64,
}
