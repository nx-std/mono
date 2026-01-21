//! CMIF (Command Message Interface Format) protocol implementation.
//!
//! CMIF is the command serialization layer built on top of HIPC. It provides
//! structured message formatting with magic headers for validation, command
//! IDs for method dispatch, and domain support for object multiplexing.
//!
//! # Protocol Stack
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  CMIF  ← this module                │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC                               │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! # Message Format
//!
//! CMIF messages are embedded within the HIPC data words section:
//!
//! **Non-Domain Request:**
//! ```text
//! [HIPC Header + Descriptors]
//! [Padding to 16-byte alignment]
//! [CmifInHeader (16 bytes): magic="SFCI", version, command_id, token]
//! [Payload data]
//! [Output pointer size table]
//! ```
//!
//! **Domain Request:**
//! ```text
//! [HIPC Header + Descriptors]
//! [Padding to 16-byte alignment]
//! [DomainInHeader (16 bytes): type, object_id, token]
//! [CmifInHeader (16 bytes)]
//! [Payload data]
//! [Object IDs array]
//! [Output pointer size table]
//! ```
//!
//! # Magic Numbers
//!
//! - `"SFCI"` (0x49434653): Service Framework Command Input
//! - `"SFCO"` (0x4F434653): Service Framework Command Output
//!
//! # Domains
//!
//! Domains allow multiplexing multiple service objects over a single session
//! handle, reducing kernel resource usage. Each object within a domain is
//! identified by a 32-bit [`ObjectId`].
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/cmif.h` (fincs, SciresM)

use core::{mem::size_of, ptr, ptr::NonNull, slice};

use nx_svc::raw::Handle as RawHandle;
use static_assertions::const_assert_eq;

use crate::hipc::{self, BufferMode};

/// Magic number for CMIF input headers ("SFCI" - Service Framework Command Input).
const IN_HEADER_MAGIC: u32 = 0x49434653;

/// Magic number for CMIF output headers ("SFCO" - Service Framework Command Output).
const OUT_HEADER_MAGIC: u32 = 0x4F434653;

/// Builds a CMIF request message in the given buffer.
///
/// Constructs a complete CMIF request including HIPC framing, optional domain
/// header, CMIF header, and reserves space for payload data. Returns a
/// [`Request`] with pointers to all sections ready for populating.
///
/// # Safety
///
/// `base` must point to a valid buffer (typically TLS IPC buffer) with at least
/// 0x200 bytes available.
pub unsafe fn make_request(base: NonNull<u8>, fmt: RequestFormat) -> Request<'static> {
    // Calculate total size needed
    let mut actual_size: u32 = 16; // alignment padding
    if fmt.object_id.is_some() {
        actual_size += size_of::<DomainInHeader>() as u32 + fmt.num_objects * 4;
    }
    actual_size += size_of::<InHeader>() as u32 + fmt.data_size as u32;
    actual_size = (actual_size + 1) & !1; // half-word align

    let out_pointer_size_table_offset = actual_size;
    let out_pointer_size_table_size = fmt.num_out_auto_buffers + fmt.num_out_pointers;
    actual_size += 2 * out_pointer_size_table_size;

    let num_data_words = actual_size.div_ceil(4);

    // Build HIPC request
    let command_type = if fmt.context != 0 {
        CommandType::RequestWithContext
    } else {
        CommandType::Request
    };

    let num_recv_statics = out_pointer_size_table_size + fmt.num_out_fixed_pointers;
    let recv_static_mode = if num_recv_statics > 0 {
        Some(hipc::RecvStaticMode::Explicit(num_recv_statics as u8))
    } else {
        None
    };

    let hipc_meta = hipc::Metadata {
        message_type: command_type.into(),
        num_send_statics: (fmt.num_in_auto_buffers + fmt.num_in_pointers) as usize,
        num_send_buffers: (fmt.num_in_auto_buffers + fmt.num_in_buffers) as usize,
        num_recv_buffers: (fmt.num_out_auto_buffers + fmt.num_out_buffers) as usize,
        num_exch_buffers: fmt.num_inout_buffers as usize,
        num_data_words: num_data_words as usize,
        recv_static_mode,
        send_pid: fmt.send_pid,
        num_copy_handles: fmt.num_handles as usize,
        num_move_handles: 0,
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };

    // Get aligned start for CMIF data
    let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base.as_ptr());

    let (cmif_header_ptr, objects) = if let Some(object_id) = fmt.object_id {
        // Domain request: write domain header first
        let domain_hdr = start as *mut DomainInHeader;
        let payload_size = size_of::<InHeader>() as u16 + fmt.data_size as u16;

        // SAFETY: start points to aligned location within valid HIPC data words.
        unsafe {
            ptr::write(
                domain_hdr,
                DomainInHeader {
                    request_type: DomainRequestType::SendMessage as u8,
                    num_in_objects: fmt.num_objects as u8,
                    data_size: payload_size,
                    object_id: object_id.to_raw(),
                    _padding: 0,
                    token: fmt.context,
                },
            );
        }

        // SAFETY: domain_hdr is valid and we're advancing by one DomainInHeader.
        let cmif_hdr = unsafe { domain_hdr.add(1) } as *mut InHeader;
        // SAFETY: cmif_hdr is valid and payload_size was calculated from layout.
        let objects_ptr = unsafe { (cmif_hdr as *mut u8).add(payload_size as usize) } as *mut u32;
        // SAFETY: objects_ptr is valid, size matches num_objects.
        let objects = unsafe { slice::from_raw_parts_mut(objects_ptr, fmt.num_objects as usize) };
        (cmif_hdr, objects)
    } else {
        (start as *mut InHeader, &mut [][..])
    };

    // SAFETY: cmif_header_ptr points to valid aligned location within buffer.
    unsafe {
        ptr::write(
            cmif_header_ptr,
            InHeader {
                magic: IN_HEADER_MAGIC,
                version: if fmt.context != 0 { 1 } else { 0 },
                command_id: fmt.request_id,
                token: if fmt.object_id.is_some() {
                    0
                } else {
                    fmt.context
                },
            },
        );
    }

    // SAFETY: cmif_header_ptr is valid, advancing by one InHeader.
    let data_ptr = unsafe { cmif_header_ptr.add(1) } as *mut u8;
    // SAFETY: data_ptr points to valid region, size matches allocated space.
    let data = unsafe { slice::from_raw_parts_mut(data_ptr, fmt.data_size) };

    // SAFETY: data_words is valid and offset was calculated from layout.
    let out_pointer_sizes_ptr = unsafe {
        hipc_req
            .data_words
            .as_mut_ptr()
            .cast::<u8>()
            .add(out_pointer_size_table_offset as usize)
    } as *mut u16;
    // SAFETY: out_pointer_sizes_ptr is valid, size matches allocation.
    let out_pointer_sizes = unsafe {
        slice::from_raw_parts_mut(out_pointer_sizes_ptr, out_pointer_size_table_size as usize)
    };

    Request {
        hipc: hipc_req,
        data,
        out_pointer_sizes,
        objects,
        server_pointer_size: fmt.server_pointer_size,
        cur_in_ptr_id: 0,
        send_buffer_idx: 0,
        recv_buffer_idx: 0,
        exch_buffer_idx: 0,
        send_static_idx: 0,
        recv_list_idx: 0,
        out_pointer_size_idx: 0,
        object_idx: 0,
        copy_handle_idx: 0,
    }
}

/// Builds a CMIF control request message.
///
/// Control requests are used for session management operations like
/// `ConvertToDomain`, `CloneObject`, and `QueryPointerBufferSize`. Returns a
/// pointer to the payload area for writing request data.
///
/// # Safety
///
/// `base` must point to a valid buffer with sufficient space.
pub unsafe fn make_control_request(base: NonNull<u8>, request_id: u32, size: u32) -> *mut u8 {
    let actual_size = 16 + size_of::<InHeader>() as u32 + size;
    let num_data_words = actual_size.div_ceil(4);

    let hipc_meta = hipc::Metadata {
        message_type: CommandType::Control.into(),
        num_data_words: num_data_words as usize,
        ..Default::default()
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };
    let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base.as_ptr());
    let hdr = start as *mut InHeader;

    // SAFETY: hdr points to aligned location within valid buffer.
    unsafe {
        ptr::write(
            hdr,
            InHeader {
                magic: IN_HEADER_MAGIC,
                version: 0,
                command_id: request_id,
                token: 0,
            },
        );
        hdr.add(1) as *mut u8
    }
}

/// Builds a CMIF close request message.
///
/// If `object_id` is `Some`, closes a domain object. Otherwise, closes
/// the entire session.
///
/// # Safety
///
/// `base` must point to a valid buffer with sufficient space.
pub unsafe fn make_close_request(base: NonNull<u8>, object_id: Option<ObjectId>) {
    if let Some(object_id) = object_id {
        // Domain object close
        let num_data_words = (16 + size_of::<DomainInHeader>() as u32) / 4;
        let hipc_meta = hipc::Metadata {
            message_type: CommandType::Request.into(),
            num_data_words: num_data_words as usize,
            ..Default::default()
        };

        // SAFETY: Caller guarantees `base` points to valid buffer.
        let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };
        let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base.as_ptr());
        let domain_hdr = start as *mut DomainInHeader;

        // SAFETY: domain_hdr points to aligned location within valid buffer.
        unsafe {
            ptr::write(
                domain_hdr,
                DomainInHeader {
                    request_type: DomainRequestType::Close as u8,
                    num_in_objects: 0,
                    data_size: 0,
                    object_id: object_id.to_raw(),
                    _padding: 0,
                    token: 0,
                },
            );
        }
    } else {
        // Session close
        let hipc_meta = hipc::Metadata {
            message_type: CommandType::Close.into(),
            ..Default::default()
        };
        // SAFETY: Caller guarantees `base` points to valid buffer.
        unsafe { hipc::make_request(base, hipc_meta) };
    }
}

/// Parses a CMIF response message.
///
/// Validates the magic number and extracts the result code. On success,
/// returns a [`Response`] with pointers to the response data.
///
/// # Safety
///
/// `base` must point to a valid CMIF response message buffer.
pub unsafe fn parse_response(
    base: NonNull<u8>,
    is_domain: bool,
    size: usize,
) -> Result<Response<'static>, ParseResponseError> {
    // SAFETY: Caller guarantees `base` points to valid CMIF response buffer.
    let hipc_resp = unsafe { hipc::parse_response(base) };
    let start = get_aligned_data_start(hipc_resp.data_words.as_ptr() as *mut u32, base.as_ptr());

    let (out_header_ptr, objects) = if is_domain {
        let domain_hdr = start as *const DomainOutHeader;
        // SAFETY: domain_hdr is valid, advancing by one DomainOutHeader.
        let cmif_hdr = unsafe { domain_hdr.add(1) } as *const OutHeader;
        // SAFETY: cmif_hdr is valid, offset calculated from layout.
        let objects_ptr =
            unsafe { (cmif_hdr as *const u8).add(size_of::<OutHeader>() + size) } as *const u32;
        // SAFETY: domain_hdr points to valid DomainOutHeader.
        let count = unsafe { ptr::read(domain_hdr) }.num_out_objects as usize;
        // SAFETY: objects_ptr is valid, count from domain header.
        let objects = unsafe { slice::from_raw_parts(objects_ptr, count) };
        (cmif_hdr, objects)
    } else {
        (start as *const OutHeader, &[][..])
    };

    // SAFETY: out_header_ptr points to valid aligned OutHeader.
    let out_header = unsafe { ptr::read(out_header_ptr) };

    // Validate magic
    if out_header.magic != OUT_HEADER_MAGIC {
        return Err(ParseResponseError::InvalidMagic);
    }

    // Check result
    if out_header.result != 0 {
        return Err(ParseResponseError::ServiceError(out_header.result));
    }

    // SAFETY: out_header_ptr is valid, advancing by one OutHeader.
    let data_ptr = unsafe { out_header_ptr.add(1) } as *const u8;
    // SAFETY: data_ptr points to valid region, size matches expected payload.
    let data = unsafe { slice::from_raw_parts(data_ptr, size) };

    Ok(Response {
        data,
        objects,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseResponseError {
    /// Response contains invalid CMIF magic header.
    #[error("invalid CMIF magic header")]
    InvalidMagic,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
}

/// Calculates the 16-byte aligned start of the data section.
///
/// CMIF headers must be 16-byte aligned within the HIPC data words.
#[inline]
fn get_aligned_data_start(data_words: *mut u32, base: *const u8) -> *mut u8 {
    // SAFETY: Both pointers are within the same IPC buffer allocation.
    let offset = unsafe { (data_words as *const u8).offset_from(base) } as usize;
    let aligned_offset = (offset + 0xF) & !0xF;
    // SAFETY: aligned_offset is within buffer bounds (base + aligned_offset <= buffer end).
    unsafe { (base as *mut u8).add(aligned_offset) }
}

/// CMIF command type (stored in HIPC message type field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandType {
    /// Invalid command.
    Invalid = 0,
    /// Legacy request (pre-5.0.0).
    LegacyRequest = 1,
    /// Close session.
    Close = 2,
    /// Legacy control request.
    LegacyControl = 3,
    /// Standard request.
    Request = 4,
    /// Control request (domain conversion, cloning, etc.).
    Control = 5,
    /// Request with context token (5.0.0+).
    RequestWithContext = 6,
    /// Control request with context token.
    ControlWithContext = 7,
}

impl From<CommandType> for hipc::MessageType {
    fn from(cmd: CommandType) -> Self {
        hipc::MessageType::from_raw(cmd as u16)
    }
}

/// Domain request type (stored in domain header).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DomainRequestType {
    /// Invalid request.
    Invalid = 0,
    /// Send message to domain object.
    SendMessage = 1,
    /// Close domain object.
    Close = 2,
}

/// CMIF input header (16 bytes).
///
/// Present at the start of every CMIF request payload.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct InHeader {
    /// Magic number (`"SFCI"` = 0x49434653).
    pub magic: u32,
    /// Protocol version (0 = standard, 1 = with context).
    pub version: u32,
    /// Command/method ID to invoke.
    pub command_id: u32,
    /// Context token for versioning (non-domain only).
    pub token: u32,
}
const_assert_eq!(size_of::<InHeader>(), 16);

/// CMIF output header (16 bytes).
///
/// Present at the start of every CMIF response payload.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct OutHeader {
    /// Magic number (`"SFCO"` = 0x4F434653).
    pub magic: u32,
    /// Protocol version.
    pub version: u32,
    /// Result code (0 = success).
    pub result: u32,
    /// Echo of request token.
    pub token: u32,
}

const_assert_eq!(size_of::<OutHeader>(), 16);

/// Domain input header (16 bytes).
///
/// Prepended to CMIF header for domain requests.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct DomainInHeader {
    /// Request type (SendMessage or Close).
    pub request_type: u8,
    /// Number of object IDs in request.
    pub num_in_objects: u8,
    /// Size of CMIF header + payload.
    pub data_size: u16,
    /// Target object ID within domain.
    pub object_id: u32,
    /// Reserved padding.
    _padding: u32,
    /// Context token.
    pub token: u32,
}

const_assert_eq!(size_of::<DomainInHeader>(), 16);

/// Domain output header (16 bytes).
///
/// Prepended to CMIF header for domain responses.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct DomainOutHeader {
    /// Number of object IDs returned.
    pub num_out_objects: u32,
    /// Reserved padding.
    _padding: [u32; 3],
}

const_assert_eq!(size_of::<DomainOutHeader>(), 16);

/// Request format descriptor.
///
/// Describes the layout of a CMIF request to be built.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestFormat {
    /// Domain object ID (`None` for non-domain sessions).
    pub object_id: Option<ObjectId>,
    /// Command/method ID.
    pub request_id: u32,
    /// Context token for versioning.
    pub context: u32,
    /// Size of payload data in bytes.
    pub data_size: usize,
    /// Server's pointer buffer capacity.
    pub server_pointer_size: usize,
    /// Number of auto-select input buffers.
    pub num_in_auto_buffers: u32,
    /// Number of auto-select output buffers.
    pub num_out_auto_buffers: u32,
    /// Number of mapped input buffers.
    pub num_in_buffers: u32,
    /// Number of mapped output buffers.
    pub num_out_buffers: u32,
    /// Number of exchange (bidirectional) buffers.
    pub num_inout_buffers: u32,
    /// Number of input pointer descriptors.
    pub num_in_pointers: u32,
    /// Number of output pointer descriptors.
    pub num_out_pointers: u32,
    /// Number of fixed-size output pointers.
    pub num_out_fixed_pointers: u32,
    /// Number of object IDs to pass.
    pub num_objects: u32,
    /// Number of handles to copy.
    pub num_handles: u32,
    /// Whether to include process ID.
    pub send_pid: bool,
}

/// Builder for constructing [`RequestFormat`].
#[derive(Debug, Clone, Default)]
pub struct RequestFormatBuilder {
    inner: RequestFormat,
}

impl RequestFormatBuilder {
    /// Creates a new builder with the given command ID.
    pub fn new(request_id: u32) -> Self {
        Self {
            inner: RequestFormat {
                request_id,
                ..Default::default()
            },
        }
    }

    /// Sets the domain object ID.
    pub fn object_id(mut self, id: ObjectId) -> Self {
        self.inner.object_id = Some(id);
        self
    }

    /// Sets the context token.
    pub fn context(mut self, context: u32) -> Self {
        self.inner.context = context;
        self
    }

    /// Sets the payload data size in bytes.
    pub fn data_size(mut self, size: usize) -> Self {
        self.inner.data_size = size;
        self
    }

    /// Sets the server pointer buffer size.
    pub fn server_pointer_size(mut self, size: usize) -> Self {
        self.inner.server_pointer_size = size;
        self
    }

    /// Sets the number of auto-select input buffers.
    pub fn in_auto_buffers(mut self, count: u32) -> Self {
        self.inner.num_in_auto_buffers = count;
        self
    }

    /// Sets the number of auto-select output buffers.
    pub fn out_auto_buffers(mut self, count: u32) -> Self {
        self.inner.num_out_auto_buffers = count;
        self
    }

    /// Sets the number of mapped input buffers.
    pub fn in_buffers(mut self, count: u32) -> Self {
        self.inner.num_in_buffers = count;
        self
    }

    /// Sets the number of mapped output buffers.
    pub fn out_buffers(mut self, count: u32) -> Self {
        self.inner.num_out_buffers = count;
        self
    }

    /// Sets the number of exchange (bidirectional) buffers.
    pub fn inout_buffers(mut self, count: u32) -> Self {
        self.inner.num_inout_buffers = count;
        self
    }

    /// Sets the number of input pointer descriptors.
    pub fn in_pointers(mut self, count: u32) -> Self {
        self.inner.num_in_pointers = count;
        self
    }

    /// Sets the number of output pointer descriptors.
    pub fn out_pointers(mut self, count: u32) -> Self {
        self.inner.num_out_pointers = count;
        self
    }

    /// Sets the number of fixed-size output pointers.
    pub fn out_fixed_pointers(mut self, count: u32) -> Self {
        self.inner.num_out_fixed_pointers = count;
        self
    }

    /// Sets the number of object IDs to pass.
    pub fn objects(mut self, count: u32) -> Self {
        self.inner.num_objects = count;
        self
    }

    /// Sets the number of handles to copy.
    pub fn handles(mut self, count: u32) -> Self {
        self.inner.num_handles = count;
        self
    }

    /// Enables sending the process ID.
    pub fn send_pid(mut self) -> Self {
        self.inner.send_pid = true;
        self
    }

    /// Builds the [`RequestFormat`].
    pub fn build(self) -> RequestFormat {
        self.inner
    }
}

/// Active CMIF request being built.
///
/// Contains mutable slices to all sections of the request for populating.
/// Use the `add_*` methods to populate the request incrementally.
#[derive(Debug)]
pub struct Request<'a> {
    /// Underlying HIPC request.
    pub hipc: hipc::Request<'a>,
    /// Payload data area.
    pub data: &'a mut [u8],
    /// Output pointer size table.
    pub out_pointer_sizes: &'a mut [u16],
    /// Object IDs array (domain only).
    pub objects: &'a mut [u32],
    /// Remaining server pointer buffer space.
    pub server_pointer_size: usize,
    /// Current input pointer index.
    pub cur_in_ptr_id: u32,
    // Internal indices for tracking position
    send_buffer_idx: usize,
    recv_buffer_idx: usize,
    exch_buffer_idx: usize,
    send_static_idx: usize,
    recv_list_idx: usize,
    out_pointer_size_idx: usize,
    object_idx: usize,
    copy_handle_idx: usize,
}

impl Request<'_> {
    /// Adds a mapped input buffer (Type A / Send Buffer).
    pub fn add_in_buffer(&mut self, buffer: *const u8, size: usize, mode: BufferMode) {
        let idx = self.send_buffer_idx;
        self.hipc.send_buffers[idx] = hipc::BufferDescriptor::new_buffer(buffer, size, mode);
        self.send_buffer_idx += 1;
    }

    /// Adds a mapped output buffer (Type B / Recv Buffer).
    pub fn add_out_buffer(&mut self, buffer: *mut u8, size: usize, mode: BufferMode) {
        let idx = self.recv_buffer_idx;
        self.hipc.recv_buffers[idx] = hipc::BufferDescriptor::new_buffer(buffer, size, mode);
        self.recv_buffer_idx += 1;
    }

    /// Adds an exchange buffer (Type W / bidirectional).
    pub fn add_inout_buffer(&mut self, buffer: *mut u8, size: usize, mode: BufferMode) {
        let idx = self.exch_buffer_idx;
        self.hipc.exch_buffers[idx] = hipc::BufferDescriptor::new_buffer(buffer, size, mode);
        self.exch_buffer_idx += 1;
    }

    /// Adds an input pointer descriptor (Type X / Send Static).
    pub fn add_in_pointer(&mut self, buffer: *const u8, size: usize) {
        let idx = self.send_static_idx;
        self.hipc.send_statics[idx] =
            hipc::StaticDescriptor::new_send(buffer, size, self.cur_in_ptr_id as u8);
        self.send_static_idx += 1;
        self.cur_in_ptr_id += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
    }

    /// Adds a fixed-size output pointer (Type C / Recv List).
    pub fn add_out_fixed_pointer(&mut self, buffer: *mut u8, size: usize) {
        let idx = self.recv_list_idx;
        self.hipc.recv_list[idx] = hipc::RecvListEntry::new_recv(buffer, size);
        self.recv_list_idx += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size);
    }

    /// Adds a variable-size output pointer with size tracking.
    pub fn add_out_pointer(&mut self, buffer: *mut u8, size: usize) {
        self.add_out_fixed_pointer(buffer, size);
        let idx = self.out_pointer_size_idx;
        self.out_pointer_sizes[idx] = size as u16;
        self.out_pointer_size_idx += 1;
    }

    /// Adds an auto-select input buffer.
    ///
    /// Uses inline pointer if the buffer fits in the server's pointer buffer,
    /// otherwise falls back to a mapped buffer.
    pub fn add_in_auto_buffer(&mut self, buffer: *const u8, size: usize, mode: BufferMode) {
        if self.server_pointer_size > 0 && size <= self.server_pointer_size {
            self.add_in_pointer(buffer, size);
            self.add_in_buffer(ptr::null(), 0, mode);
        } else {
            self.add_in_pointer(ptr::null(), 0);
            self.add_in_buffer(buffer, size, mode);
        }
    }

    /// Adds an auto-select output buffer.
    ///
    /// Uses inline pointer if the buffer fits in the server's pointer buffer,
    /// otherwise falls back to a mapped buffer.
    pub fn add_out_auto_buffer(&mut self, buffer: *mut u8, size: usize, mode: BufferMode) {
        if self.server_pointer_size > 0 && size <= self.server_pointer_size {
            self.add_out_pointer(buffer, size);
            self.add_out_buffer(ptr::null_mut(), 0, mode);
        } else {
            self.add_out_pointer(ptr::null_mut(), 0);
            self.add_out_buffer(buffer, size, mode);
        }
    }

    /// Adds a domain object ID to the request.
    pub fn add_object(&mut self, id: ObjectId) {
        let idx = self.object_idx;
        self.objects[idx] = id.to_raw();
        self.object_idx += 1;
    }

    /// Adds a copy handle to the request.
    pub fn add_handle(&mut self, handle: u32) {
        let idx = self.copy_handle_idx;
        self.hipc.copy_handles[idx] = handle;
        self.copy_handle_idx += 1;
    }
}

/// Parsed CMIF response.
///
/// Contains slices to the response data and any returned objects/handles.
#[derive(Debug)]
pub struct Response<'a> {
    /// Response payload data.
    pub data: &'a [u8],
    /// Returned domain object IDs.
    pub objects: &'a [u32],
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles.
    pub move_handles: &'a [RawHandle],
}

/// A domain object identifier.
///
/// Identifies a specific service object within a CMIF domain session.
/// Object ID 0 is invalid; valid object IDs start at 1.
///
/// # Object IDs in CMIF Domains
///
/// When a service session is converted to a **domain**, it can multiplex
/// multiple service objects over a single IPC session handle. Each object
/// within the domain is identified by a unique 32-bit **Object ID**.
///
/// ## How Domains Work
///
/// Without domains, each service object requires its own kernel session handle.
/// This consumes kernel resources and limits scalability. Domains solve this by:
///
/// 1. Converting a session to a domain via `ConvertToDomain` control request
/// 2. The original service becomes object ID 1 within the domain
/// 3. Subsequent service objects acquired through this session get unique IDs
/// 4. All objects share the single underlying session handle
///
/// ## Message Format
///
/// In domain mode, CMIF requests include a [`DomainInHeader`] that specifies:
/// - The target object ID for the request
/// - Input object IDs being passed to the service
///
/// Responses include a [`DomainOutHeader`] with output object IDs.
///
/// ## Relationship to HIPC
///
/// Object IDs are a CMIF-layer concept built on top of HIPC:
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │  Service (object_id = N)            │  ← ObjectId identifies target
/// ├─────────────────────────────────────┤
/// │  CMIF DomainInHeader { object_id }  │  ← ObjectId encoded here
/// ├─────────────────────────────────────┤
/// │  HIPC (session handle)              │  ← Single handle for all objects
/// └─────────────────────────────────────┘
/// ```
///
/// HIPC itself knows nothing about object IDs - it only deals with session
/// handles, buffer descriptors, and raw data. The CMIF layer adds the domain
/// abstraction on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Creates an `ObjectId` from a raw value.
    ///
    /// Returns `None` if `raw` is zero, as zero is not a valid object ID.
    /// Valid object IDs start at 1 when a session is converted to a domain.
    #[inline]
    pub(crate) const fn new(raw: u32) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Creates an `ObjectId` from a raw value without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is non-zero and represents a valid
    /// object ID obtained from the kernel (via `ConvertToDomain` or similar).
    #[inline]
    pub(crate) const unsafe fn new_unchecked(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw `u32` value of this object ID.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}
