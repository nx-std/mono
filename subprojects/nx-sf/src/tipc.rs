//! TIPC (Trivial IPC) protocol implementation.
//!
//! TIPC is a simplified IPC protocol introduced in Horizon OS 12.0.0. Unlike
//! CMIF, it has no domain support and stores the command ID directly in the
//! HIPC message type field.
//!
//! # Protocol Stack
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  TIPC  ← this module                │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC                               │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! # Key Differences from CMIF
//!
//! | Aspect              | CMIF                     | TIPC                      |
//! |---------------------|--------------------------|---------------------------|
//! | Command ID          | In CMIF header           | HIPC message type (ID+16) |
//! | Domain support      | Yes                      | No                        |
//! | Magic headers       | SFCI/SFCO                | None                      |
//! | Close command       | Type=2                   | Type=15                   |
//! | Pointer descriptors | Type X/C (statics)       | None                      |
//! | Result code         | In OutHeader.result      | First u32 of data words   |
//! | Object passing      | Domain object IDs        | Move handles              |
//!
//! # Message Format
//!
//! **Request:**
//! ```text
//! [HIPC Header (type = command_id + 16)]
//! [HIPC Descriptors (buffers, handles)]
//! [Data Words (raw payload)]
//! ```
//!
//! **Response:**
//! ```text
//! [HIPC Header]
//! [HIPC Descriptors (handles)]
//! [Result Code (u32)]
//! [Response Payload]
//! ```
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/tipc.h` (fincs, SciresM)

use core::{ptr::NonNull, slice};

use nx_svc::raw::Handle as RawHandle;

use crate::hipc::{self, BufferMode};

/// TIPC command types.
///
/// Unlike CMIF, TIPC encodes the command ID directly in the message type field
/// as `id + 16`. The `Close` variant is a special case with type = 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CommandType {
    /// Close session (type = 15).
    Close = 15,
}

impl CommandType {
    /// Creates a request message type from a command ID.
    ///
    /// TIPC stores command ID in HIPC message type as ID + 16.
    #[inline]
    pub const fn request(id: u32) -> hipc::MessageType {
        hipc::MessageType::from_raw((id + 16) as u16)
    }
}

impl From<CommandType> for hipc::MessageType {
    fn from(cmd: CommandType) -> Self {
        hipc::MessageType::from_raw(cmd as u16)
    }
}

/// Builds a TIPC request message in the given buffer.
///
/// # Safety
///
/// `base` must point to a valid buffer (typically TLS IPC buffer) with at least
/// 0x200 bytes available.
pub unsafe fn make_request(base: NonNull<u8>, fmt: RequestFormat) -> Request<'static> {
    let num_data_words = fmt.data_size.div_ceil(4);

    let hipc_meta = hipc::Metadata {
        message_type: CommandType::request(fmt.request_id),
        num_send_statics: 0, // TIPC doesn't use pointer descriptors
        num_send_buffers: fmt.num_in_buffers as usize,
        num_recv_buffers: fmt.num_out_buffers as usize,
        num_exch_buffers: fmt.num_inout_buffers as usize,
        num_data_words,
        recv_static_mode: None, // TIPC doesn't use pointer descriptors
        send_pid: fmt.send_pid,
        num_copy_handles: fmt.num_handles as usize,
        num_move_handles: 0,
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    let hipc_req = unsafe { hipc::make_request(base, hipc_meta) };

    // Data pointer is directly at the start of data words (no CMIF header)
    let data_ptr = hipc_req.data_words.as_mut_ptr() as *mut u8;
    // SAFETY: data_ptr points within the valid HIPC data words region,
    // and data_size was used to allocate num_data_words.
    let data = unsafe { slice::from_raw_parts_mut(data_ptr, fmt.data_size) };

    Request {
        hipc: hipc_req,
        data,
        send_buffer_idx: 0,
        recv_buffer_idx: 0,
        exch_buffer_idx: 0,
        copy_handle_idx: 0,
    }
}

/// Builds a TIPC close request message.
///
/// # Safety
///
/// `base` must point to a valid buffer with sufficient space.
pub unsafe fn make_close_request(base: NonNull<u8>) {
    let hipc_meta = hipc::Metadata {
        message_type: CommandType::Close.into(),
        ..Default::default()
    };

    // SAFETY: Caller guarantees `base` points to valid buffer with sufficient space.
    unsafe { hipc::make_request(base, hipc_meta) };
}

/// Parses a TIPC response message.
///
/// # Safety
///
/// `base` must point to a valid TIPC response message buffer.
pub unsafe fn parse_response(
    base: NonNull<u8>,
    size: usize,
) -> Result<Response<'static>, ParseResponseError> {
    // SAFETY: Caller guarantees `base` points to valid TIPC response buffer.
    let hipc_resp = unsafe { hipc::parse_response(base) };

    // Result code is the first word of data
    if hipc_resp.data_words.is_empty() {
        return Err(ParseResponseError::EmptyResponse);
    }

    let result = hipc_resp.data_words[0];
    if result != 0 {
        return Err(ParseResponseError::ServiceError(result));
    }

    // SAFETY: We verified data_words is non-empty, so index 1 is within bounds
    // when data_words.len() > 1 (which is implied by having payload data).
    let data_ptr = unsafe { hipc_resp.data_words.as_ptr().add(1) } as *const u8;
    let data_len = size;

    // SAFETY: data_ptr points to valid memory within the response buffer,
    // and caller guarantees size matches the expected response payload.
    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };

    Ok(Response {
        data,
        copy_handles: hipc_resp.copy_handles,
        move_handles: hipc_resp.move_handles,
    })
}

/// Error returned by [`parse_response`].
#[derive(Debug, thiserror::Error)]
pub enum ParseResponseError {
    /// Response data words are empty.
    #[error("empty response data")]
    EmptyResponse,
    /// Service returned a non-zero result code.
    #[error("service error: {0:#x}")]
    ServiceError(u32),
}

/// Request format descriptor for TIPC.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestFormat {
    /// Command/method ID (will be stored as ID + 16 in HIPC message type).
    pub request_id: u32,
    /// Size of payload data in bytes.
    pub data_size: usize,
    /// Number of mapped input buffers (Type A / Send Buffer).
    pub num_in_buffers: u32,
    /// Number of mapped output buffers (Type B / Recv Buffer).
    pub num_out_buffers: u32,
    /// Number of exchange (bidirectional) buffers (Type W).
    pub num_inout_buffers: u32,
    /// Number of handles to copy.
    pub num_handles: u32,
    /// Whether to include process ID.
    pub send_pid: bool,
}

/// Active TIPC request being built.
#[derive(Debug)]
pub struct Request<'a> {
    /// Underlying HIPC request.
    pub hipc: hipc::Request<'a>,
    /// Payload data area.
    pub data: &'a mut [u8],
    // Internal indices for tracking position
    send_buffer_idx: usize,
    recv_buffer_idx: usize,
    exch_buffer_idx: usize,
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

    /// Adds a copy handle to the request.
    pub fn add_handle(&mut self, handle: impl Into<RawHandle>) {
        let idx = self.copy_handle_idx;
        self.hipc.copy_handles[idx] = handle.into();
        self.copy_handle_idx += 1;
    }
}

/// Parsed TIPC response.
#[derive(Debug)]
pub struct Response<'a> {
    /// Response payload data (excludes the result code word).
    pub data: &'a [u8],
    /// Returned copy handles.
    pub copy_handles: &'a [RawHandle],
    /// Returned move handles (used for receiving service objects).
    pub move_handles: &'a [RawHandle],
}
