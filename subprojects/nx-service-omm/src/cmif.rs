//! CMIF protocol operations for the operation mode manager service.

use core::{mem::size_of, ptr};

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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::GET_OPERATION_MODE)
            .send()
            .map_err(GetOperationModeError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetOperationModeError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<u8>())
        .map_err(GetOperationModeError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u8>()` bytes.
    let mode = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(mode)
}

/// Error returned by [`get_operation_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetOperationModeError {
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

// ---------------------------------------------------------------------------
// set_operation_mode_policy (3.0.0+)
// ---------------------------------------------------------------------------

/// Sets the operation mode policy (3.0.0+).
pub fn set_operation_mode_policy(
    session: SessionHandle,
    policy: u8,
) -> Result<(), SetOperationModePolicyError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SET_OPERATION_MODE_POLICY)
            .data_size(size_of::<u8>())
            .send()
            .map_err(SetOperationModePolicyError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u8>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u8>(), policy) };
    }

    ipc::send_sync_request(session).map_err(SetOperationModePolicyError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(buf.as_array(), 0)
        .map_err(SetOperationModePolicyError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_operation_mode_policy`].
#[derive(Debug, thiserror::Error)]
pub enum SetOperationModePolicyError {
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

// ---------------------------------------------------------------------------
// get_default_display_resolution (3.0.0+)
// ---------------------------------------------------------------------------

/// Gets the default display resolution (3.0.0+).
///
/// Returns `(width, height)`.
pub fn get_default_display_resolution(
    session: SessionHandle,
) -> Result<(i32, i32), GetDefaultDisplayResolutionError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::GET_DEFAULT_DISPLAY_RESOLUTION)
            .send()
            .map_err(GetDefaultDisplayResolutionError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetDefaultDisplayResolutionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<[i32; 2]>())
        .map_err(GetDefaultDisplayResolutionError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<[i32; 2]>()` bytes.
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
