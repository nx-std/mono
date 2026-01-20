//! CMIF protocol operations for HID service.
//!
//! This module implements HID commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::ptr;

use nx_sf::cmif;
use nx_svc::{
    ipc::{self, Handle as SessionHandle},
    mem::shmem::Handle as ShmemHandle,
};

use crate::proto::{applet_resource_cmds, cmds};

/// Creates an IAppletResource sub-interface.
///
/// This is IHidServer command 0.
pub fn create_applet_resource(
    session: SessionHandle,
    aruid: u64,
) -> Result<SessionHandle, CreateAppletResourceError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::INITIALIZE_APPLET_RESOURCE)
        .context(0x20)
        .data_size(8) // u64 aruid
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    ipc::send_sync_request(session).map_err(CreateAppletResourceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CreateAppletResourceError::ParseResponse)?;

    // Extract the move handle from response
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(CreateAppletResourceError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the shared memory handle from IAppletResource.
///
/// This is IAppletResource command 0.
pub fn get_shared_memory_handle(
    session: SessionHandle,
) -> Result<ShmemHandle, GetSharedMemoryHandleError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt =
        cmif::RequestFormatBuilder::new(applet_resource_cmds::GET_SHARED_MEMORY_HANDLE).build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSharedMemoryHandleError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSharedMemoryHandleError::ParseResponse)?;

    // Extract the copy handle from response
    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryHandleError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { ShmemHandle::from_raw(handle) })
}

/// Activates Npad (controller) input with revision support.
///
/// This is IHidServer command 109 (ActivateNpadWithRevision).
/// Uses revision 0x5 (for firmware 18.0.0+).
///
/// For older firmware (<5.0.0), use command 103 without revision.
pub fn activate_npad(session: SessionHandle, aruid: u64) -> Result<(), ActivateNpadError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // Use modern revision (0x5 for firmware 18.0.0+)
    let revision: u32 = 0x5;

    let fmt = cmif::RequestFormatBuilder::new(cmds::ACTIVATE_NPAD_WITH_REVISION)
        .context(0x20)
        .data_size(16) // u32 revision + u32 pad + u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write input data: u32 revision, u32 pad, u64 ARUID
    // SAFETY: req.data points to valid payload area with space for the struct.
    #[repr(C)]
    struct Input {
        revision: u32,
        pad: u32,
        aruid: u64,
    }
    let input = Input {
        revision,
        pad: 0,
        aruid,
    };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(ActivateNpadError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ActivateNpadError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad style set.
///
/// This is IHidServer command 100.
pub fn set_supported_npad_style_set(
    session: SessionHandle,
    aruid: u64,
    style_set: u32,
) -> Result<(), SetSupportedNpadStyleSetError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::SET_SUPPORTED_NPAD_STYLE_SET)
        .context(0x20)
        .data_size(16) // u32 style_set + u32 pad + u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write input data: u32 style_set, u32 pad, u64 ARUID
    // SAFETY: req.data points to valid payload area with space for the struct.
    #[repr(C)]
    struct Input {
        style_set: u32,
        pad: u32,
        aruid: u64,
    }
    let input = Input {
        style_set,
        pad: 0,
        aruid,
    };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetSupportedNpadStyleSetError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetSupportedNpadStyleSetError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad ID types.
///
/// This is IHidServer command 102.
pub fn set_supported_npad_id_type(
    session: SessionHandle,
    aruid: u64,
    ids: &[u32],
) -> Result<(), SetSupportedNpadIdTypeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let buffer_size = ids.len() * 4;

    let fmt = cmif::RequestFormatBuilder::new(cmds::SET_SUPPORTED_NPAD_ID_TYPE)
        .context(0x20)
        .data_size(8) // u64 ARUID
        .in_pointers(1) // HipcPointer for IDs array
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID to data section
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    // Add IDs array as input pointer
    req.add_in_pointer(ids.as_ptr().cast::<u8>(), buffer_size);

    ipc::send_sync_request(session).map_err(SetSupportedNpadIdTypeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetSupportedNpadIdTypeError::ParseResponse)?;

    Ok(())
}

/// Activates touch screen input.
///
/// This is IHidServer command 11.
pub fn activate_touch_screen(
    session: SessionHandle,
    aruid: u64,
) -> Result<(), ActivateTouchScreenError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::ACTIVATE_TOUCH_SCREEN)
        .context(0x20)
        .data_size(8) // u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    ipc::send_sync_request(session).map_err(ActivateTouchScreenError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ActivateTouchScreenError::ParseResponse)?;

    Ok(())
}

/// Activates keyboard input.
///
/// This is IHidServer command 31.
pub fn activate_keyboard(session: SessionHandle, aruid: u64) -> Result<(), ActivateKeyboardError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::ACTIVATE_KEYBOARD)
        .context(0x20)
        .data_size(8) // u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    ipc::send_sync_request(session).map_err(ActivateKeyboardError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ActivateKeyboardError::ParseResponse)?;

    Ok(())
}

/// Activates mouse input.
///
/// This is IHidServer command 21.
pub fn activate_mouse(session: SessionHandle, aruid: u64) -> Result<(), ActivateMouseError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::ACTIVATE_MOUSE)
        .context(0x20)
        .data_size(8) // u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    ipc::send_sync_request(session).map_err(ActivateMouseError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ActivateMouseError::ParseResponse)?;

    Ok(())
}

/// Activates gesture recognition.
///
/// This is IHidServer command 91.
pub fn activate_gesture(session: SessionHandle, aruid: u64) -> Result<(), ActivateGestureError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::ACTIVATE_GESTURE)
        .context(0x20)
        .data_size(16) // u32 val + u32 pad + u64 ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write input data: u32 val = 1, u32 pad, u64 ARUID
    // SAFETY: req.data points to valid payload area with space for the struct.
    #[repr(C)]
    struct Input {
        val: u32,
        pad: u32,
        aruid: u64,
    }
    let input = Input {
        val: 1,
        pad: 0,
        aruid,
    };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(ActivateGestureError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ActivateGestureError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`create_applet_resource`].
#[derive(Debug, thiserror::Error)]
pub enum CreateAppletResourceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`get_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryHandleError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing shared memory handle in response.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// Error returned by [`activate_npad`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateNpadError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`set_supported_npad_style_set`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadStyleSetError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`set_supported_npad_id_type`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadIdTypeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`activate_touch_screen`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateTouchScreenError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`activate_keyboard`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateKeyboardError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`activate_mouse`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateMouseError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`activate_gesture`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateGestureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
