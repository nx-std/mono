//! CMIF protocol operations for HID service.
//!
//! This module implements HID commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::{mem::size_of, ptr};

use nx_service_applet::aruid::{Aruid, NO_ARUID};
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
    aruid: Option<Aruid>,
) -> Result<SessionHandle, CreateAppletResourceError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::INITIALIZE_APPLET_RESOURCE)
            .context(0x20)
            .data_size(size_of::<u64>())
            .send_pid()
            .send(&mut buf)
            .map_err(CreateAppletResourceError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes (the ARUID).
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), aruid) };
    }

    ipc::send_sync_request(session).map_err(CreateAppletResourceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response::<()>(&buf).map_err(CreateAppletResourceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(CreateAppletResourceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the shared memory handle from IAppletResource.
///
/// This is IAppletResource command 0.
pub fn get_shared_memory_handle(
    session: SessionHandle,
) -> Result<ShmemHandle, GetSharedMemoryHandleError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(applet_resource_cmds::GET_SHARED_MEMORY_HANDLE)
            .send(&mut buf)
            .map_err(GetSharedMemoryHandleError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetSharedMemoryHandleError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response::<()>(&buf).map_err(GetSharedMemoryHandleError::ParseResponse)?;

    let Some(&handle) = resp.copy_handles.first() else {
        return Err(GetSharedMemoryHandleError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid shared memory handle in the response.
    Ok(unsafe { ShmemHandle::from_raw(handle) })
}

/// Activates Npad (controller) input with revision support.
///
/// This is IHidServer command 109 (ActivateNpadWithRevision).
/// Uses revision 0x5 (for firmware 18.0.0+).
///
/// For older firmware (<5.0.0), use command 103 without revision.
pub fn activate_npad(
    session: SessionHandle,
    aruid: Option<Aruid>,
) -> Result<(), ActivateNpadError> {
    // Use modern revision (0x5 for firmware 18.0.0+)
    let revision: u32 = 0x5;
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_NPAD_WITH_REVISION)
            .context(0x20)
            .data_size(size_of::<Input>())
            .send_pid()
            .send(&mut buf)
            .map_err(ActivateNpadError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes; `Input` is
        // `repr(C)` and `input` is a valid value on the stack.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(ActivateNpadError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(ActivateNpadError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad style set.
///
/// This is IHidServer command 100.
pub fn set_supported_npad_style_set(
    session: SessionHandle,
    aruid: Option<Aruid>,
    style_set: u32,
) -> Result<(), SetSupportedNpadStyleSetError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SET_SUPPORTED_NPAD_STYLE_SET)
            .context(0x20)
            .data_size(size_of::<Input>())
            .send_pid()
            .send(&mut buf)
            .map_err(SetSupportedNpadStyleSetError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes; `Input` is
        // `repr(C)` and `input` is a valid value on the stack.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(SetSupportedNpadStyleSetError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(SetSupportedNpadStyleSetError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad ID types.
///
/// This is IHidServer command 102.
pub fn set_supported_npad_id_type(
    session: SessionHandle,
    aruid: Option<Aruid>,
    ids: &[u32],
) -> Result<(), SetSupportedNpadIdTypeError> {
    let buffer_size = core::mem::size_of_val(ids);
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SET_SUPPORTED_NPAD_ID_TYPE)
            .context(0x20)
            .data_size(size_of::<u64>())
            .add_in_pointer(ids.as_ptr().cast::<u8>(), buffer_size)
            .send_pid()
            .send(&mut buf)
            .map_err(SetSupportedNpadIdTypeError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes (the ARUID).
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), aruid) };
    }

    ipc::send_sync_request(session).map_err(SetSupportedNpadIdTypeError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(SetSupportedNpadIdTypeError::ParseResponse)?;

    Ok(())
}

/// Activates touch screen input.
///
/// This is IHidServer command 11.
pub fn activate_touch_screen(
    session: SessionHandle,
    aruid: Option<Aruid>,
) -> Result<(), ActivateTouchScreenError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_TOUCH_SCREEN)
            .context(0x20)
            .data_size(size_of::<u64>())
            .send_pid()
            .send(&mut buf)
            .map_err(ActivateTouchScreenError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes (the ARUID).
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), aruid) };
    }

    ipc::send_sync_request(session).map_err(ActivateTouchScreenError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(ActivateTouchScreenError::ParseResponse)?;

    Ok(())
}

/// Activates keyboard input.
///
/// This is IHidServer command 31.
pub fn activate_keyboard(
    session: SessionHandle,
    aruid: Option<Aruid>,
) -> Result<(), ActivateKeyboardError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_KEYBOARD)
            .context(0x20)
            .data_size(size_of::<u64>())
            .send_pid()
            .send(&mut buf)
            .map_err(ActivateKeyboardError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes (the ARUID).
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), aruid) };
    }

    ipc::send_sync_request(session).map_err(ActivateKeyboardError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(ActivateKeyboardError::ParseResponse)?;

    Ok(())
}

/// Activates mouse input.
///
/// This is IHidServer command 21.
pub fn activate_mouse(
    session: SessionHandle,
    aruid: Option<Aruid>,
) -> Result<(), ActivateMouseError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_MOUSE)
            .context(0x20)
            .data_size(size_of::<u64>())
            .send_pid()
            .send(&mut buf)
            .map_err(ActivateMouseError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes (the ARUID).
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), aruid) };
    }

    ipc::send_sync_request(session).map_err(ActivateMouseError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(ActivateMouseError::ParseResponse)?;

    Ok(())
}

/// Activates gesture recognition.
///
/// This is IHidServer command 91.
pub fn activate_gesture(
    session: SessionHandle,
    aruid: Option<Aruid>,
) -> Result<(), ActivateGestureError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_GESTURE)
            .context(0x20)
            .data_size(size_of::<Input>())
            .send_pid()
            .send(&mut buf)
            .map_err(ActivateGestureError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes; `Input` is
        // `repr(C)` and `input` is a valid value on the stack.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(ActivateGestureError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response::<()>(&buf).map_err(ActivateGestureError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`create_applet_resource`].
#[derive(Debug, thiserror::Error)]
pub enum CreateAppletResourceError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`get_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryHandleError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
    /// Missing shared memory handle in response.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// Error returned by [`activate_npad`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateNpadError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`set_supported_npad_style_set`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadStyleSetError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`set_supported_npad_id_type`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadIdTypeError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`activate_touch_screen`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateTouchScreenError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`activate_keyboard`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateKeyboardError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`activate_mouse`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateMouseError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Error returned by [`activate_gesture`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateGestureError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}
