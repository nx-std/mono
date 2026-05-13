//! CMIF protocol operations for the fatal service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto,
    types::{FatalCpuContext, FatalPolicy, ThrowFatalIn},
};

/// Throws a fatal error with the given policy (no CPU context).
pub fn throw_fatal_with_policy(
    session: SessionHandle,
    result_code: u32,
    policy: FatalPolicy,
) -> Result<(), ThrowFatalError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::THROW_FATAL_WITH_POLICY)
        .data_size(size_of::<ThrowFatalIn>())
        .send_pid()
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = ThrowFatalIn {
        result_code,
        policy: policy as u32,
        pid_placeholder: 0,
    };

    // SAFETY: req.data points to valid payload area with space for ThrowFatalIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<ThrowFatalIn>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(ThrowFatalError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(ThrowFatalError::ParseResponse)?;

    Ok(())
}

/// Throws a fatal error with the given policy and CPU context.
pub fn throw_fatal_with_context(
    session: SessionHandle,
    result_code: u32,
    policy: FatalPolicy,
    ctx: &FatalCpuContext,
) -> Result<(), ThrowFatalError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::THROW_FATAL_WITH_CONTEXT)
        .data_size(size_of::<ThrowFatalIn>())
        .send_pid()
        .in_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = ThrowFatalIn {
        result_code,
        policy: policy as u32,
        pid_placeholder: 0,
    };

    // SAFETY: req.data points to valid payload area with space for ThrowFatalIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<ThrowFatalIn>().cast_mut(), input);
    }

    req.add_in_buffer(
        (ctx as *const FatalCpuContext).cast::<u8>(),
        size_of::<FatalCpuContext>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(ThrowFatalError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(ThrowFatalError::ParseResponse)?;

    Ok(())
}

/// Error returned by fatal throw operations.
#[derive(Debug, thiserror::Error)]
pub enum ThrowFatalError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
