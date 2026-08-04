//! CMIF protocol operations for the fatal service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
    },
    service::BorrowedSessionHandle,
};

use crate::{
    proto,
    types::{
        FatalCpuContext,
        FatalPolicy,
        ThrowFatalIn,
    },
};

/// Throws a fatal error with the given policy (no CPU context).
pub fn throw_fatal_with_policy(
    session: BorrowedSessionHandle<'_>,
    result_code: u32,
    policy: FatalPolicy,
) -> Result<(), ThrowFatalError> {
    let input = ThrowFatalIn {
        result_code,
        policy,
        pid_placeholder: 0,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::THROW_FATAL_WITH_POLICY)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ThrowFatalError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ThrowFatalError::ParseResponse)?;

    Ok(())
}

/// Throws a fatal error with the given policy and CPU context.
pub fn throw_fatal_with_context(
    session: BorrowedSessionHandle<'_>,
    result_code: u32,
    policy: FatalPolicy,
    ctx: &FatalCpuContext,
) -> Result<(), ThrowFatalError> {
    let input = ThrowFatalIn {
        result_code,
        policy,
        pid_placeholder: 0,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::THROW_FATAL_WITH_CONTEXT)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(
            // SAFETY: `FatalCpuContext` is `#[repr(C)]` plain data; reinterpreting
            // as a byte slice for the IPC wire is sound.
            unsafe {
                core::slice::from_raw_parts(
                    (ctx as *const FatalCpuContext).cast::<u8>(),
                    core::mem::size_of::<FatalCpuContext>(),
                )
            },
            BufferMode::Normal,
        ))
        .build();
    req.send(&mut buf, session)
        .map_err(ThrowFatalError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ThrowFatalError::ParseResponse)?;

    Ok(())
}

/// Error returned by fatal throw operations.
#[derive(Debug, thiserror::Error)]
pub enum ThrowFatalError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
