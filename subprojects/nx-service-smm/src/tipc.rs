//! TIPC protocol operations for the SM management interface.
//!
//! This module implements `sm:m` commands using the TIPC (Tiny IPC)
//! protocol, which is used on HOS 12.0.0+ and Atmosphere.

use core::{mem::size_of, ptr};

use nx_sf::{hipc::BufferMode, tipc};
use nx_svc::ipc::{self, Handle as SessionHandle};

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::REGISTER_PROCESS,
        data_size: size_of::<u64>(),
        num_in_buffers: 2,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { tipc::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), pid);
    }

    req.add_in_buffer(acid_sac.as_ptr(), acid_sac.len(), BufferMode::Normal);
    req.add_in_buffer(aci0_sac.as_ptr(), aci0_sac.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(RegisterProcessError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp =
        unsafe { tipc::parse_response(ipc_buf, 0) }.map_err(RegisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_process`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterProcessError {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::UNREGISTER_PROCESS,
        data_size: size_of::<u64>(),
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { tipc::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), pid);
    }

    ipc::send_sync_request(session).map_err(UnregisterProcessError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { tipc::parse_response(ipc_buf, 0) }
        .map_err(UnregisterProcessError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_process`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterProcessError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
