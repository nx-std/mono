//! CMIF protocol operations for the error context service.

use core::{mem::size_of, ptr};

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Output from a [`pull_context`] call.
#[derive(Debug, Clone, Copy)]
pub struct PullContextOutput {
    /// Unknown output field.
    pub field0: i32,
    /// Total error context size.
    pub total_size: u32,
    /// Actual error context size written to the buffer.
    pub size: u32,
}

/// Pulls error context associated with a descriptor and result code.
pub fn pull_context(
    session: SessionHandle,
    dst: &mut [u8],
    descriptor: u32,
    result: u32,
) -> Result<PullContextOutput, PullContextError> {
    #[repr(C)]
    struct Input {
        descriptor: u32,
        result: u32,
    }

    #[repr(C)]
    struct Output {
        field0: i32,
        total_size: u32,
        size: u32,
    }

    let input = Input { descriptor, result };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(proto::PULL_CONTEXT)
            .data_size(size_of::<Input>())
            .add_out_buffer(dst.as_mut_ptr(), dst.len(), BufferMode::Normal)
            .send(&mut buf)
            .map_err(PullContextError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(PullContextError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<Output>())
        .map_err(PullContextError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<Output>()` bytes.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(PullContextOutput {
        field0: out.field0,
        total_size: out.total_size,
        size: out.size,
    })
}

/// Error returned by [`pull_context`].
#[derive(Debug, thiserror::Error)]
pub enum PullContextError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
