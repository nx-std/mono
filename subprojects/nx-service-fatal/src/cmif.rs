//! CMIF protocol operations for the fatal service.

use core::mem::size_of;

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
    },
    service::BorrowedSessionHandle,
};
use static_assertions::const_assert_eq;
use zerocopy::IntoBytes as _;

use crate::{
    proto,
    types::{
        FatalAarch64Context,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

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

    /// Wire form of the CPU context.
    ///
    /// The service reserves the larger of the two context shapes and picks
    /// between them with `is_aarch32`. This crate builds only for `aarch64`,
    /// so the discriminant is always false and the AArch64 context fills the
    /// reservation exactly, leaving no byte of it unwritten.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct FatalCpuContextWire {
        /// Register state at the fault, filling the reservation exactly.
        ctx: FatalAarch64Context,
        /// Selects the AArch32 context shape. Always false here.
        is_aarch32: u8,
        /// Aligns `context_type` to its 4-byte boundary. Zero on the wire.
        _pad: [u8; 3],
        /// Exception type.
        context_type: u32,
    }

    const_assert_eq!(size_of::<FatalCpuContextWire>(), 0x250);

    let wire = FatalCpuContextWire {
        ctx: ctx.ctx,
        is_aarch32: 0,
        _pad: [0; 3],
        context_type: ctx.context_type,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::THROW_FATAL_WITH_CONTEXT)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(wire.as_bytes(), BufferMode::Normal))
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
