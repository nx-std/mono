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
//! identified by a 32-bit object ID.
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/cmif.h` (fincs, SciresM)

use core::{mem::size_of, ptr, slice};

use static_assertions::const_assert_eq;

use crate::hipc::{self, BufferMode};

/// Magic number for CMIF input headers ("SFCI" - Service Framework Command Input).
pub const IN_HEADER_MAGIC: u32 = 0x49434653;

/// Magic number for CMIF output headers ("SFCO" - Service Framework Command Output).
pub const OUT_HEADER_MAGIC: u32 = 0x4F434653;

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
pub unsafe fn make_request(base: *mut u8, fmt: RequestFormat) -> Request<'static> {
    // Calculate total size needed
    let mut actual_size: u32 = 16; // alignment padding
    if fmt.object_id != 0 {
        actual_size += size_of::<DomainInHeader>() as u32 + fmt.num_objects * 4;
    }
    actual_size += size_of::<InHeader>() as u32 + fmt.data_size;
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

    let hipc_meta = hipc::Metadata {
        message_type: command_type as u32,
        num_send_statics: fmt.num_in_auto_buffers + fmt.num_in_pointers,
        num_send_buffers: fmt.num_in_auto_buffers + fmt.num_in_buffers,
        num_recv_buffers: fmt.num_out_auto_buffers + fmt.num_out_buffers,
        num_exch_buffers: fmt.num_inout_buffers,
        num_data_words,
        num_recv_statics: out_pointer_size_table_size + fmt.num_out_fixed_pointers,
        send_pid: if fmt.send_pid { 1 } else { 0 },
        num_copy_handles: fmt.num_handles,
        num_move_handles: 0,
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };

    // Get aligned start for CMIF data
    let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base);

    let (cmif_header_ptr, objects_ptr) = if fmt.object_id != 0 {
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
                    object_id: fmt.object_id,
                    padding: 0,
                    token: fmt.context,
                },
            );
        }

        // SAFETY: domain_hdr is valid and we're advancing by one DomainInHeader.
        let cmif_hdr = unsafe { domain_hdr.add(1) } as *mut InHeader;
        // SAFETY: cmif_hdr is valid and payload_size was calculated from layout.
        let objects = unsafe { (cmif_hdr as *mut u8).add(payload_size as usize) } as *mut u32;
        (cmif_hdr, objects)
    } else {
        (start as *mut InHeader, ptr::null_mut())
    };

    // SAFETY: cmif_header_ptr points to valid aligned location within buffer.
    unsafe {
        ptr::write(
            cmif_header_ptr,
            InHeader {
                magic: IN_HEADER_MAGIC,
                version: if fmt.context != 0 { 1 } else { 0 },
                command_id: fmt.request_id,
                token: if fmt.object_id != 0 { 0 } else { fmt.context },
            },
        );
    }

    // SAFETY: cmif_header_ptr is valid, advancing by one InHeader.
    let data_ptr = unsafe { cmif_header_ptr.add(1) } as *mut u8;
    // SAFETY: data_words is valid and offset was calculated from layout.
    let out_pointer_sizes_ptr = unsafe {
        hipc_req
            .data_words
            .as_mut_ptr()
            .cast::<u8>()
            .add(out_pointer_size_table_offset as usize)
    } as *mut u16;

    Request {
        hipc: hipc_req,
        // SAFETY: data_ptr points to valid region, size matches allocated space.
        data: unsafe { slice::from_raw_parts_mut(data_ptr, fmt.data_size as usize) },
        // SAFETY: out_pointer_sizes_ptr is valid, size matches allocation.
        out_pointer_sizes: unsafe {
            slice::from_raw_parts_mut(out_pointer_sizes_ptr, out_pointer_size_table_size as usize)
        },
        objects: if objects_ptr.is_null() {
            &mut []
        } else {
            // SAFETY: objects_ptr is valid when non-null, size matches num_objects.
            unsafe { slice::from_raw_parts_mut(objects_ptr, fmt.num_objects as usize) }
        },
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
pub unsafe fn make_control_request(base: *mut u8, request_id: u32, size: u32) -> *mut u8 {
    let actual_size = 16 + size_of::<InHeader>() as u32 + size;
    let num_data_words = actual_size.div_ceil(4);

    let hipc_meta = hipc::Metadata {
        message_type: CommandType::Control as u32,
        num_data_words,
        ..Default::default()
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };
    let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base);
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
/// If `object_id` is non-zero, closes a domain object. Otherwise, closes
/// the entire session.
///
/// # Safety
///
/// `base` must point to a valid buffer with sufficient space.
pub unsafe fn make_close_request(base: *mut u8, object_id: u32) {
    if object_id != 0 {
        // Domain object close
        let num_data_words = (16 + size_of::<DomainInHeader>() as u32) / 4;
        let hipc_meta = hipc::Metadata {
            message_type: CommandType::Request as u32,
            num_data_words,
            ..Default::default()
        };

        // SAFETY: Caller guarantees `base` points to valid buffer.
        let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };
        let start = get_aligned_data_start(hipc_req.data_words.as_mut_ptr(), base);
        let domain_hdr = start as *mut DomainInHeader;

        // SAFETY: domain_hdr points to aligned location within valid buffer.
        unsafe {
            ptr::write(
                domain_hdr,
                DomainInHeader {
                    request_type: DomainRequestType::Close as u8,
                    num_in_objects: 0,
                    data_size: 0,
                    object_id,
                    padding: 0,
                    token: 0,
                },
            );
        }
    } else {
        // Session close
        let hipc_meta = hipc::Metadata {
            message_type: CommandType::Close as u32,
            ..Default::default()
        };
        // SAFETY: Caller guarantees `base` points to valid buffer.
        unsafe { hipc::make_request(base, hipc_meta) };
    }
}

/// Parses a CMIF response message.
///
/// Validates the magic number and extracts the result code. On success,
/// returns a [`Response`] with pointers to the response data. On failure,
/// returns the error result code from the service.
///
/// # Safety
///
/// `base` must point to a valid CMIF response message buffer.
pub unsafe fn parse_response(
    base: *const u8,
    is_domain: bool,
    size: u32,
) -> Result<Response<'static>, u32> {
    // SAFETY: Caller guarantees `base` points to valid CMIF response buffer.
    let hipc_resp = unsafe { hipc::parse_response(base) };
    let start = get_aligned_data_start(hipc_resp.data_words.as_ptr() as *mut u32, base);

    let (out_header_ptr, objects_ptr) = if is_domain {
        let domain_hdr = start as *const DomainOutHeader;
        // SAFETY: domain_hdr is valid, advancing by one DomainOutHeader.
        let cmif_hdr = unsafe { domain_hdr.add(1) } as *const OutHeader;
        // SAFETY: cmif_hdr is valid, offset calculated from layout.
        let objects = unsafe { (cmif_hdr as *const u8).add(size_of::<OutHeader>() + size as usize) }
            as *const u32;
        (cmif_hdr, objects)
    } else {
        (start as *const OutHeader, ptr::null())
    };

    // SAFETY: out_header_ptr points to valid aligned OutHeader.
    let out_header = unsafe { ptr::read(out_header_ptr) };

    // Validate magic
    if out_header.magic != OUT_HEADER_MAGIC {
        // Return a generic error for invalid magic
        return Err(0xFFFF);
    }

    // Check result
    if out_header.result != 0 {
        return Err(out_header.result);
    }

    // SAFETY: out_header_ptr is valid, advancing by one OutHeader.
    let data_ptr = unsafe { out_header_ptr.add(1) } as *const u8;

    Ok(Response {
        // SAFETY: data_ptr points to valid region, size matches expected payload.
        data: unsafe { slice::from_raw_parts(data_ptr, size as usize) },
        objects: if objects_ptr.is_null() {
            &[]
        } else if is_domain {
            // Get object count from domain header
            let domain_hdr = start as *const DomainOutHeader;
            // SAFETY: domain_hdr points to valid DomainOutHeader.
            let count = unsafe { ptr::read(domain_hdr) }.num_out_objects as usize;
            // SAFETY: objects_ptr is valid, count from domain header.
            unsafe { slice::from_raw_parts(objects_ptr, count) }
        } else {
            &[]
        },
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Calculates the 16-byte aligned start of the data section.
///
/// CMIF headers must be 16-byte aligned within the HIPC data words.
#[inline]
fn get_aligned_data_start(data_words: *mut u32, base: *const u8) -> *mut u8 {
    // SAFETY: Both pointers are within the same IPC buffer allocation.
    let offset = unsafe { (data_words as *const u8).offset_from(base) } as usize;
    let aligned_offset = (offset + 15) & !15;
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
    pub padding: u32,
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
    pub padding: [u32; 3],
}
const_assert_eq!(size_of::<DomainOutHeader>(), 16);

/// Request format descriptor.
///
/// Describes the layout of a CMIF request to be built.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestFormat {
    /// Domain object ID (0 for non-domain).
    pub object_id: u32,
    /// Command/method ID.
    pub request_id: u32,
    /// Context token for versioning.
    pub context: u32,
    /// Size of payload data in bytes.
    pub data_size: u32,
    /// Server's pointer buffer capacity.
    pub server_pointer_size: u32,
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
    pub server_pointer_size: u32,
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
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size as u32);
    }

    /// Adds a fixed-size output pointer (Type C / Recv List).
    pub fn add_out_fixed_pointer(&mut self, buffer: *mut u8, size: usize) {
        let idx = self.recv_list_idx;
        self.hipc.recv_list[idx] = hipc::RecvListEntry::new_recv(buffer, size);
        self.recv_list_idx += 1;
        self.server_pointer_size = self.server_pointer_size.saturating_sub(size as u32);
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
        if self.server_pointer_size > 0 && size <= self.server_pointer_size as usize {
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
        if self.server_pointer_size > 0 && size <= self.server_pointer_size as usize {
            self.add_out_pointer(buffer, size);
            self.add_out_buffer(ptr::null_mut(), 0, mode);
        } else {
            self.add_out_pointer(ptr::null_mut(), 0);
            self.add_out_buffer(buffer, size, mode);
        }
    }

    /// Adds a domain object ID to the request.
    pub fn add_object(&mut self, object_id: u32) {
        let idx = self.object_idx;
        self.objects[idx] = object_id;
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
    pub copy_handles: &'a [u32],
    /// Returned move handles.
    pub move_handles: &'a [u32],
}
