//! CMIF protocol operations for the operation mode manager service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

// ---------------------------------------------------------------------------
// get_operation_mode
// ---------------------------------------------------------------------------

/// Gets the current operation mode.
///
/// Returns the raw `u8` value; the caller should convert via
/// [`OperationMode::from_raw`](crate::OperationMode::from_raw).
pub fn get_operation_mode(session: SessionHandle) -> Result<u8, GetOperationModeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_OPERATION_MODE)
        .data_size(0)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetOperationModeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(GetOperationModeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let mode = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(mode)
}

/// Error returned by [`get_operation_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetOperationModeError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// set_operation_mode_policy (3.0.0+)
// ---------------------------------------------------------------------------

/// Sets the operation mode policy (3.0.0+).
pub fn set_operation_mode_policy(
    session: SessionHandle,
    policy: u8,
) -> Result<(), SetOperationModePolicyError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_OPERATION_MODE_POLICY)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), policy);
    }

    ipc::send_sync_request(session).map_err(SetOperationModePolicyError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetOperationModePolicyError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_operation_mode_policy`].
#[derive(Debug, thiserror::Error)]
pub enum SetOperationModePolicyError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// get_default_display_resolution (3.0.0+)
// ---------------------------------------------------------------------------

/// Gets the default display resolution (3.0.0+).
///
/// Returns `(width, height)`.
pub fn get_default_display_resolution(
    session: SessionHandle,
) -> Result<(i32, i32), GetDefaultDisplayResolutionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_DEFAULT_DISPLAY_RESOLUTION)
        .data_size(0)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetDefaultDisplayResolutionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<[i32; 2]>()) }
        .map_err(GetDefaultDisplayResolutionError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for [i32; 2].
    let (width, height) = unsafe {
        let data_ptr = resp.data.as_ptr().cast::<i32>();
        (
            ptr::read_unaligned(data_ptr),
            ptr::read_unaligned(data_ptr.add(1)),
        )
    };

    Ok((width, height))
}

/// Error returned by [`get_default_display_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDefaultDisplayResolutionError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
