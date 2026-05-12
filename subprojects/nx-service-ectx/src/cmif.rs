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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

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

    let fmt = cmif::RequestFormatBuilder::new(proto::PULL_CONTEXT)
        .data_size(size_of::<Input>())
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for Input.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<Input>().cast_mut(),
            Input { descriptor, result },
        );
    }

    req.add_out_buffer(dst.as_mut_ptr(), dst.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(PullContextError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<Output>()) }
        .map_err(PullContextError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for Output.
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
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
