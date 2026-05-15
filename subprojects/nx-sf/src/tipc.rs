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
//! [`TipcBuilder`] is the high-level entry point for full TIPC requests. It
//! wraps a [`hipc::HipcRequestBuilder`] and exposes only the descriptor kinds
//! TIPC supports (mapped buffers + copy handles). Finalize via
//! [`send`](TipcBuilder::send). [`TipcPayload`] implements
//! [`hipc::HipcPayload`] for direct use with [`hipc::HipcRequestBuilder`].
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/tipc.h` (fincs, SciresM)

use core::{convert::Infallible, mem::size_of};

use nx_svc::raw::Handle as RawHandle;
use nx_sys_thread_tls::IPC_BUFFER_SIZE;
use zerocopy::IntoBytes;

use crate::{
    cmif::RequestLayoutError,
    hipc::{self, BufferDescriptor, BufferMode, HipcPayload, HipcRequestBuilder},
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

/// [`HipcPayload`] writer for a TIPC request body.
///
/// TIPC has no in-band header — the payload bytes sit directly at the start
/// of the data-words region. The output is a [`TipcRequest`] bundling the
/// HIPC frame and the carved data slice.
#[derive(Debug, Clone, Copy)]
pub struct TipcPayload {
    data_size: usize,
}

impl TipcPayload {
    /// Creates a TIPC payload writer of the given byte size.
    #[inline]
    pub const fn new(data_size: usize) -> Self {
        Self { data_size }
    }
}

impl HipcPayload for TipcPayload {
    type Output<'a> = TipcRequest<'a>;
    type Error = Infallible;

    fn encoded_len(&self) -> usize {
        self.data_size
    }

    fn encode<'a>(
        self,
        hipc: hipc::Request<'a>,
        dst: &'a mut [u8],
    ) -> Result<TipcRequest<'a>, Infallible> {
        let (data, _) = dst.split_at_mut(self.data_size);
        Ok(TipcRequest { hipc, data })
    }
}

/// Fluent builder for a TIPC request.
///
/// Wraps a [`HipcRequestBuilder`] with the message type pre-set to
/// `CommandType::request(request_id)` (ID + 16). Exposes only the descriptor
/// kinds TIPC supports — mapped buffers and copy handles.
pub struct TipcBuilder<'a, const N: usize> {
    hipc: HipcRequestBuilder<'a, N>,
    data_size: usize,
}

impl<'a, const N: usize> TipcBuilder<'a, N> {
    /// Starts a new builder for the given command ID and buffer.
    #[inline]
    pub fn new(buf: &'a mut [u8; N], request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(buf, CommandType::request(request_id)),
            data_size: 0,
        }
    }

    /// Sets the size of the payload data area in bytes. The caller fills it
    /// via [`TipcRequest::data`] after [`send`](Self::send).
    #[inline]
    pub fn data_size(mut self, n: usize) -> Self {
        self.data_size = n;
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

    /// Finalizes the request, writing the HIPC frame into the buffer.
    pub fn send(self) -> Result<TipcRequest<'a>, RequestLayoutError> {
        self.hipc.payload(TipcPayload::new(self.data_size))
    }
}

/// Builds a TIPC close request message.
///
/// # Errors
///
/// Returns [`RequestLayoutError`] if the computed request size exceeds the
/// IPC buffer (cannot happen for a close request in practice).
pub fn close_request<const N: usize>(buf: &mut [u8; N]) -> Result<(), RequestLayoutError> {
    HipcRequestBuilder::new(buf, CommandType::Close).payload(TipcPayload::new(0))?;
    Ok(())
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

/// Finalized TIPC request, returned by [`TipcBuilder::send`].
///
/// All HIPC descriptors are populated; the caller fills [`data`](Self::data)
/// before sending the request via `SendSyncRequest`.
#[derive(Debug)]
pub struct TipcRequest<'a> {
    /// Underlying HIPC frame with descriptor slots already populated.
    pub hipc: hipc::Request<'a>,
    /// Payload data area (size matches `TipcBuilder::data_size`).
    pub data: &'a mut [u8],
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
