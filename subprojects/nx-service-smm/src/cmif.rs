//! CMIF protocol operations for the SM management interface.
//!
//! This module implements `sm:m` commands using the CMIF (Common Message
//! Interface Format) protocol, which is the standard IPC protocol on
//! HOS < 12.0.0 (non-Atmosphere).

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    hipc::{BufferMode, InputBuffer},
    ipc::Handle as SessionHandle,
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
        .with_data(&payload)
        .add_input_buffer(InputBuffer::new(acid_sac, BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(aci0_sac, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterProcessError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RegisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_process`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterProcessError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
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
        .with_data(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(UnregisterProcessError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(UnregisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_process`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterProcessError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
