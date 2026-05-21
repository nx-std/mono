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
//! # Builder model
//!
//! [`TipcRequestBuilder`] is the high-level entry point for full TIPC
//! requests. It wraps a [`hipc::HipcRequestBuilder`] and exposes only the
//! descriptor kinds TIPC supports (mapped buffers + copy handles). Finalize
//! via [`send`](TipcRequestBuilder::send), which takes the destination
//! buffer. The builder holds no buffer reference. [`TipcPayload`] implements
//! [`hipc::HipcPayload`] for direct use with [`hipc::HipcRequestBuilder`].
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/tipc.h` (fincs, SciresM)

use core::mem::size_of;

use nx_svc::raw::Handle as RawHandle;
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::IntoBytes;

use crate::{
    cmif::RequestLayoutError,
    hipc::{self, BufferDescriptor, BufferMode, HipcRequest, HipcRequestBuilder},
};

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

/// Fluent builder for a TIPC request.
///
/// Wraps a [`HipcRequestBuilder`] with the message type pre-set to
/// `CommandType::request(request_id)` (ID + 16). Exposes only the descriptor
/// kinds TIPC supports — mapped buffers and copy handles.
pub struct TipcRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    data: &'a [u8],
}

impl<'a> TipcRequestBuilder<'a> {
    /// Starts a new builder for the given command ID.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::request(request_id)),
            data: &[],
        }
    }

    /// Sets the request payload data.
    #[inline]
    pub fn data(mut self, data: &'a [u8]) -> Self {
        self.data = data;
        self
    }

    /// Enables sending the process ID alongside the request.
    #[inline]
    pub fn send_pid(mut self) -> Self {
        self.hipc = self.hipc.with_send_pid();
        self
    }

    /// Adds a mapped input buffer (Type A / Send Buffer).
    #[inline]
    pub fn add_in_buffer(mut self, buffer: *const u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped output buffer (Type B / Recv Buffer).
    #[inline]
    pub fn add_out_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds an exchange (in/out) buffer (Type W).
    #[inline]
    pub fn add_inout_buffer(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a copy handle to the request.
    #[inline]
    pub fn add_copy_handle(mut self, handle: impl Into<RawHandle>) -> Self {
        self.hipc = self.hipc.with_copy_handle(handle.into());
        self
    }

    /// Finalizes the request DTO.
    pub fn build(self) -> TipcRequest<'a> {
        let data_len = self.data.len();
        let hipc = self.hipc.build(data_len);
        TipcRequest {
            hipc,
            data: self.data,
        }
    }
}

/// TIPC close request DTO.
#[derive(Debug, Clone)]
pub struct TipcCloseRequest {
    hipc: HipcRequest,
}

impl TipcCloseRequest {
    /// Creates a session-close request.
    pub fn session() -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::Close).build(0),
        }
    }

    /// Writes the close request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        self.hipc.write_to(dst)
    }
}

/// Builds a TIPC close request message.
pub fn close_request<const N: usize>(buf: &mut [u8; N]) -> Result<(), RequestLayoutError> {
    TipcCloseRequest::session().write_to(buf)
}

/// Parses a TIPC response message.
pub fn parse_response<'a>(
    buf: &'a [u8; IPC_BUFFER_SIZE],
    size: usize,
) -> Result<Response<'a>, ParseResponseError> {
    let hipc_resp = hipc::parse_response(buf)?;

    // Result code is the first word of data.
    if hipc_resp.data_words.is_empty() {
        return Err(ParseResponseError::EmptyResponse);
    }

    let result = hipc_resp.data_words[0];
    if result != 0 {
        return Err(ParseResponseError::ServiceError(result));
    }

    // Skip the 4-byte result code prefix.
    let (_result_word, payload) = hipc_resp
        .data_words
        .as_bytes()
        .split_at_checked(size_of::<u32>())
        .ok_or(ParseResponseError::TruncatedResult)?;
    let (data, _) = payload
        .split_at_checked(size)
        .ok_or(ParseResponseError::TruncatedPayload)?;

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
    /// Underlying HIPC layer rejected the response.
    #[error("HIPC parse: {0}")]
    Hipc(#[from] hipc::ResponseParseError),
    /// Response data words too small to contain the result code word.
    #[error("TIPC response too small for result code")]
    TruncatedResult,
    /// Response too small to contain the caller-requested payload size.
    #[error("TIPC response too small for payload")]
    TruncatedPayload,
}

/// Finalized TIPC request.
#[derive(Debug, Clone)]
pub struct TipcRequest<'a> {
    hipc: HipcRequest,
    data: &'a [u8],
}

impl<'a> TipcRequest<'a> {
    /// Writes the TIPC request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        self.hipc.write_to(dst)?;
        let start = self.hipc.data_words_offset();
        let data = &mut dst[start..start + self.data.len()];
        data.copy_from_slice(self.data);
        Ok(())
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
