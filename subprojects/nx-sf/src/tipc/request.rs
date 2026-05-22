//! TIPC request building.
//!
//! This module contains request values and fluent builders for TIPC requests.
//! Response parsing lives in the sibling `response` module.

use nx_svc::raw::Handle as RawHandle;

use super::wire::CommandType;
use crate::hipc::{self, BufferDescriptor, BufferMode, HipcRequest, HipcRequestBuilder};

/// Layout error for TIPC request serialization.
///
/// TIPC request writers cannot fail while encoding their own headers; layout
/// failures come from the underlying HIPC request size check.
pub type RequestLayoutError = hipc::WriteError;

/// Fluent builder for a TIPC request.
///
/// Wraps a [`HipcRequestBuilder`] with the message type pre-set to
/// `CommandType::request(request_id)` (ID + 16). Exposes only the descriptor
/// kinds TIPC supports — mapped buffers and copy handles.
pub struct TipcRequestBuilder<'a> {
    hipc: HipcRequestBuilder,
    payload: &'a [u8],
}

impl<'a> TipcRequestBuilder<'a> {
    /// Starts a new builder for the given command ID.
    #[inline]
    pub fn new(request_id: u32) -> Self {
        Self {
            hipc: HipcRequestBuilder::new(CommandType::request(request_id)),
            payload: &[],
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
    #[inline]
    pub fn add_input_buffer_raw(
        mut self,
        buffer: *const u8,
        size: usize,
        mode: BufferMode,
    ) -> Self {
        self.hipc = self
            .hipc
            .with_send_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped output buffer using a Type-B HIPC descriptor.
    #[inline]
    pub fn add_output_buffer_raw(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_recv_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
        self
    }

    /// Adds a mapped bidirectional buffer using a Type-W HIPC descriptor.
    #[inline]
    pub fn add_inout_buffer_raw(mut self, buffer: *mut u8, size: usize, mode: BufferMode) -> Self {
        self.hipc = self
            .hipc
            .with_exch_buffer(BufferDescriptor::new_buffer(buffer, size, mode));
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
        self.hipc.with_payload(self.payload).build()
    }
}

/// Finalized TIPC request.
///
/// Alias for a [`HipcRequest`] carrying a raw byte-slice payload.
pub type TipcRequest<'a> = HipcRequest<&'a [u8]>;

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

    /// Writes the close request into `dst`.
    pub fn write_to<const N: usize>(&self, dst: &mut [u8; N]) -> Result<(), RequestLayoutError> {
        self.hipc.write_to(dst)
    }
}
