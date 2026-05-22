//! TIPC protocol operations for the SM management interface.
//!
//! This module implements `sm:m` commands using the TIPC (Tiny IPC)
//! protocol, which is used on HOS 12.0.0+ and Atmosphere.

use core::{mem::size_of, ptr};

use nx_sf::{cmif, hipc::BufferMode, ipc, tipc};
use nx_svc::ipc::Handle as SessionHandle;

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
    session: SessionHandle,
    pid: u64,
    acid_sac: &[u8],
    aci0_sac: &[u8],
) -> Result<(), RegisterProcessError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u64>()];
    // SAFETY: `payload` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u64>(), pid) };
    let req = tipc::TipcRequestBuilder::new(proto::REGISTER_PROCESS)
        .with_data(&payload)
        .add_input_buffer_raw(acid_sac.as_ptr(), acid_sac.len(), BufferMode::Normal)
        .add_input_buffer_raw(aci0_sac.as_ptr(), aci0_sac.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(RegisterProcessError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(RegisterProcessError::SendRequest)?;

    tipc::parse_response(&buf, 0).map_err(RegisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_process`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterProcessError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

/// Unregisters a process from the Service Manager using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn unregister_process(session: SessionHandle, pid: u64) -> Result<(), UnregisterProcessError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u64>()];
    // SAFETY: `payload` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u64>(), pid) };
    let req = tipc::TipcRequestBuilder::new(proto::UNREGISTER_PROCESS)
        .with_data(&payload)
        .build();
    req.write_to(&mut buf)
        .map_err(UnregisterProcessError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(UnregisterProcessError::SendRequest)?;

    tipc::parse_response(&buf, 0).map_err(UnregisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_process`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterProcessError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
