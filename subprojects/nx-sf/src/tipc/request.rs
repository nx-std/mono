//! TIPC request messages: building one as a client, parsing one as a server.
//!
//! [`TipcRequestBuilder`] produces a request to send to a service;
//! [`parse_request`] reads one a client sent to a service this process hosts.
//! Reply building and response parsing live in the sibling `response` module.

use core::ptr;

use nx_svc::raw::Handle as RawHandle;
use nx_sys_thread_tls::IpcBuffer;

use super::wire::{
    CommandType,
    REQUEST_TYPE_BASE,
};
use crate::{
    array_vec::ArrayVec,
    hipc::{
        self,
        BufferDescriptor,
        HIPC_MAX_DESCRIPTORS,
        HipcPayload,
        HipcRequest,
        HipcRequestBuilder,
        InOutBuffer,
        InputBuffer,
        OutputBuffer,
    },
    service::handle::BorrowedSessionHandle,
};

/// Layout error for TIPC request serialization.
///
/// TIPC request writers cannot fail while encoding their own headers; layout
/// failures come from the underlying HIPC request size check.
pub type RequestLayoutError = hipc::WriteError;

/// Send error for TIPC requests.
///
/// Alias for the underlying HIPC send error ([`hipc::SendError`]).
pub type SendError = hipc::SendError;

/// Fluent builder for a TIPC request.
///
/// Wraps a [`HipcRequestBuilder`] with the message type pre-set to
/// `CommandType::request(request_id)` (ID + 16). Exposes only the descriptor
/// kinds TIPC supports - mapped buffers and copy handles.
pub struct TipcRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    payload: &'a [u8],
    input_buffers: ArrayVec<InputBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
    output_buffers: ArrayVec<OutputBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
    inout_buffers: ArrayVec<InOutBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
}

impl<'a> TipcRequestBuilder<'a> {
    /// Starts a new builder for the given command ID.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::request(request_id)),
            payload: &[],
            input_buffers: ArrayVec::new(),
            output_buffers: ArrayVec::new(),
            inout_buffers: ArrayVec::new(),
        }
    }

    /// Sets the request payload bytes to copy into the TIPC data area.
    #[inline]
    pub fn with_data(mut self, data: &'a [u8]) -> Self {
        self.payload = data;
        self
    }

    /// Includes the current process ID in the underlying HIPC request.
    #[inline]
    pub fn with_send_pid(mut self) -> Self {
        self.hipc = self.hipc.with_send_pid();
        self
    }

    /// Adds a mapped input buffer using a Type-A HIPC descriptor.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_input_buffer(mut self, buf: InputBuffer<'a>) -> Self {
        let mode = buf.mode();
        let slice = buf.as_slice();
        let len = slice.len();
        let ptr = if slice.is_empty() {
            ptr::null()
        } else {
            slice.as_ptr()
        };

        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(ptr, len, mode));
        self.input_buffers.push(buf);
        self
    }

    /// Adds a mapped output buffer using a Type-B HIPC descriptor.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_output_buffer(mut self, mut buf: OutputBuffer<'a>) -> Self {
        let mode = buf.mode();
        let len = buf.as_slice().len();
        let ptr = if len == 0 {
            ptr::null_mut()
        } else {
            buf.as_mut_slice().as_mut_ptr()
        };

        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(ptr, len, mode));
        self.output_buffers.push(buf);
        self
    }

    /// Adds a mapped bidirectional buffer using a Type-W HIPC descriptor.
    ///
    /// Empty slices encode a null descriptor instead of taking a dangling
    /// pointer from the slice.
    #[inline]
    pub fn add_inout_buffer(mut self, mut buf: InOutBuffer<'a>) -> Self {
        let mode = buf.mode();
        let len = buf.as_slice().len();
        let ptr = if len == 0 {
            ptr::null_mut()
        } else {
            buf.as_mut_slice().as_mut_ptr()
        };

        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(ptr, len, mode));
        self.inout_buffers.push(buf);
        self
    }

    /// Adds a copy handle to pass with the underlying HIPC request.
    #[inline]
    pub fn add_copy_handle(mut self, handle: impl Into<RawHandle>) -> Self {
        self.hipc = self.hipc.with_copy_handle(handle.into());
        self
    }

    /// Finalizes the request value.
    pub fn build(self) -> TipcRequest<'a> {
        let body = TipcBody {
            payload: self.payload,
            _input_buffers: self.input_buffers,
            _output_buffers: self.output_buffers,
            _inout_buffers: self.inout_buffers,
        };
        self.hipc.with_payload(body).build()
    }
}

/// Finalized TIPC request.
///
/// Alias for a [`HipcRequest`] carrying a [`TipcBody`] payload (raw data
/// words plus the borrowed buffer wrappers attached to the request).
pub type TipcRequest<'a> = HipcRequest<TipcBody<'a>>;

impl TipcRequest<'_> {
    /// Serializes the request into `buf` and sends it on `session`.
    ///
    /// Consuming the request keeps every attached buffer loan alive until
    /// the kernel finishes the syscall, and releases them all when this
    /// returns. The payload borrow deliberately rides along even though its
    /// bytes are copied at serialization time. Parse the response from
    /// `buf` afterwards.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Layout`] when the encoded request does not fit
    /// in the IPC buffer (nothing is sent), and [`SendError::SendRequest`]
    /// when the kernel rejects the underlying `SendSyncRequest`.
    #[inline]
    pub fn send(
        self,
        buf: &mut IpcBuffer,
        session: BorrowedSessionHandle<'_>,
    ) -> Result<(), SendError> {
        self.send_inner(buf, session)
    }
}

/// In-band body for a TIPC request.
///
/// Encodes only the raw data-words payload - TIPC has no per-protocol
/// header. The body also owns the borrowed buffer wrappers attached to the
/// request so each per-slice borrow stays live until the request DTO is
/// dropped.
#[derive(Debug)]
pub struct TipcBody<'a> {
    payload: &'a [u8],
    // Held (never read) to keep each per-slice borrow alive until the
    // request DTO is dropped, enforcing per-slice exclusivity through the
    // borrow checker; see `CmifBody` for the full provenance rationale.
    _input_buffers: ArrayVec<InputBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
    _output_buffers: ArrayVec<OutputBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
    _inout_buffers: ArrayVec<InOutBuffer<'a>, HIPC_MAX_DESCRIPTORS>,
}

impl HipcPayload for TipcBody<'_> {
    #[inline]
    fn encoded_len(&self) -> usize {
        self.payload.len()
    }

    #[inline]
    fn write_to(&self, dst: &mut [u8]) {
        dst[..self.payload.len()].copy_from_slice(self.payload);
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
            hipc: HipcRequestBuilder::new(CommandType::Close).build(),
        }
    }

    /// Serializes the close request into `buf` and sends it on `session`.
    ///
    /// # Errors
    ///
    /// Returns [`SendError::Layout`] when the encoded request does not fit
    /// in the IPC buffer (nothing is sent), and [`SendError::SendRequest`]
    /// when the kernel rejects the underlying `SendSyncRequest`.
    #[inline]
    pub fn send(
        self,
        buf: &mut IpcBuffer,
        session: BorrowedSessionHandle<'_>,
    ) -> Result<(), SendError> {
        self.hipc.send_inner(buf, session)
    }
}

/// Interprets an already-parsed HIPC request as a TIPC one.
///
/// Takes the [`hipc::Request`] rather than the raw buffer because a server
/// needs both halves: the HIPC value owns the buffer descriptors and handles
/// the command's arguments live in, and this function only adds the TIPC
/// reading of its message type. TIPC writes no in-band header of its own, so
/// the data words are the argument region as they stand.
///
/// # Errors
///
/// Returns [`RequestParseError`] when the message type names neither a close
/// nor a command, which is every value below [`CommandType::Close`].
pub fn parse_request<'a>(request: &hipc::Request<'a>) -> Result<Request<'a>, RequestParseError> {
    let raw_type = request.message_type.to_raw();
    match raw_type {
        t if t == CommandType::Close as u16 => Ok(Request::Close),
        t if t >= REQUEST_TYPE_BASE => Ok(Request::Command(Command {
            command_id: u32::from(t - REQUEST_TYPE_BASE),
            payload: request.data_words,
        })),
        other => Err(RequestParseError(other)),
    }
}

/// Error returned by [`parse_request`].
///
/// Occurs when the HIPC message type falls below the range TIPC assigns, so it
/// names neither a close nor a command id. The low types belong to CMIF, so
/// this is the shape a CMIF client reaching a TIPC-only interface produces.
/// Detected before any part of the message body is read.
#[derive(Debug, thiserror::Error)]
#[error("unsupported TIPC command type: {0:#x}")]
pub struct RequestParseError(pub u16);

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for RequestParseError {
    fn to_rc(self) -> crate::error::ResultCode {
        // A request rejected before any handler saw it, so no service assigned
        // it a code to forward.
        crate::error::GENERIC_ERROR
    }
}

/// Parsed inbound TIPC request.
///
/// Returned by [`parse_request`]. The message type decides which of these a
/// request is, so a caller cannot read a command id off a session close. TIPC
/// has no control requests: domain conversion and object cloning are CMIF
/// facilities, and the protocol that dropped domains dropped their control
/// path with them.
#[derive(Debug)]
pub enum Request<'a> {
    /// A command invocation on the session's interface.
    Command(Command<'a>),
    /// A session close. Carries no in-band data, and is not replied to.
    Close,
}

/// Command id and raw arguments decoded from a TIPC request.
#[derive(Debug)]
pub struct Command<'a> {
    /// Method id to invoke on the target interface.
    pub command_id: u32,
    /// The whole data-words region.
    ///
    /// TIPC writes no in-band header, so this is the argument region from its
    /// first byte. It is the raw remainder rather than a sized argument tuple:
    /// it still holds the word padding HIPC added, and how much of it is
    /// arguments is a property of the command's own signature, which a
    /// wire-format parser does not know.
    pub payload: &'a [u8],
}
