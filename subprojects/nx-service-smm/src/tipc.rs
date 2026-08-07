//! TIPC protocol operations for the SM management interface.
//!
//! This module implements `sm:m` commands using the TIPC (Tiny IPC)
//! protocol, which is used on HOS 12.0.0+ and Atmosphere.

use nx_sf::{
    hipc::{
        BufferMode,
        InputBuffer,
    },
    service::BorrowedSessionHandle,
    tipc,
};
use zerocopy::IntoBytes as _;

use crate::proto;

/// Registers a process with the Service Manager using TIPC protocol.
///
/// Sends `pid` as the inline payload and attaches the two service-access
/// control buffers (`acid_sac`, `aci0_sac`) as Type-A (HipcMapAlias) input
/// buffers.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn register_process(
    session: BorrowedSessionHandle<'_>,
    pid: u64,
    acid_sac: &[u8],
    aci0_sac: &[u8],
) -> Result<(), RegisterProcessError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = tipc::TipcRequestBuilder::new(proto::REGISTER_PROCESS)
        .with_data(pid.as_bytes())
        .add_input_buffer(InputBuffer::new(acid_sac, BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(aci0_sac, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterProcessError::SendRequest)?;

    tipc::parse_response::<()>(&buf).map_err(RegisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_process`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterProcessError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

/// Unregisters a process from the Service Manager using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn unregister_process(
    session: BorrowedSessionHandle<'_>,
    pid: u64,
) -> Result<(), UnregisterProcessError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = tipc::TipcRequestBuilder::new(proto::UNREGISTER_PROCESS)
        .with_data(pid.as_bytes())
        .build();
    req.send(&mut buf, session)
        .map_err(UnregisterProcessError::SendRequest)?;

    tipc::parse_response::<()>(&buf).map_err(UnregisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_process`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterProcessError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
