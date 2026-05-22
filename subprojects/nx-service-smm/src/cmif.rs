//! CMIF protocol operations for the SM management interface.
//!
//! This module implements `sm:m` commands using the CMIF (Common Message
//! Interface Format) protocol, which is the standard IPC protocol on
//! HOS < 12.0.0 (non-Atmosphere).

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Registers a process with the Service Manager using CMIF protocol.
///
/// Sends `pid` as the inline payload and attaches the two service-access
/// control buffers (`acid_sac`, `aci0_sac`) as Type-A (HipcMapAlias) input
/// buffers.
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
    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_PROCESS)
        .data(&payload)
        .add_in_buffer(acid_sac.as_ptr(), acid_sac.len(), BufferMode::Normal)
        .add_in_buffer(aci0_sac.as_ptr(), aci0_sac.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(RegisterProcessError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(RegisterProcessError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(RegisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_process`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterProcessError {
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

/// Unregisters a process from the Service Manager using CMIF protocol.
#[inline]
pub fn unregister_process(session: SessionHandle, pid: u64) -> Result<(), UnregisterProcessError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let mut payload = [0u8; size_of::<u64>()];
    // SAFETY: `payload` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u64>(), pid) };
    let req = cmif::CmifRequestBuilder::new(proto::UNREGISTER_PROCESS)
        .data(&payload)
        .build();
    req.write_to(&mut buf)
        .map_err(UnregisterProcessError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(UnregisterProcessError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(UnregisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_process`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterProcessError {
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
