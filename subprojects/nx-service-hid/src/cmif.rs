//! CMIF protocol operations for HID service.
//!
//! This module implements HID commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_service_applet::aruid::{
    Aruid,
    NO_ARUID,
};
use nx_sf::{
    cmif,
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    hipc::InPointer,
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
};
use nx_svc::mem::shmem::Handle as ShmemHandle;

use crate::proto::{
    applet_resource_cmds,
    cmds,
};

/// Creates an IAppletResource sub-interface.
///
/// This is IHidServer command 0.
pub fn create_applet_resource(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<OwnedSessionHandle, CreateAppletResourceError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::INITIALIZE_APPLET_RESOURCE)
        .with_context(0x20)
        .with_data_value(&aruid)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(CreateAppletResourceError::SendRequest)?;

    let resp =
        cmif::parse_response::<()>(&buf).map_err(CreateAppletResourceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(CreateAppletResourceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Gets the shared memory handle from IAppletResource.
///
/// This is IAppletResource command 0.
pub fn get_shared_memory_handle(
    session: BorrowedSessionHandle<'_>,
) -> Result<ShmemHandle, GetSharedMemoryHandleError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(applet_resource_cmds::GET_SHARED_MEMORY_HANDLE).build();
    req.send(&mut buf, session)
        .map_err(GetSharedMemoryHandleError::SendRequest)?;

    let resp =
        cmif::parse_response::<()>(&buf).map_err(GetSharedMemoryHandleError::ParseResponse)?;

    let Some(&handle) = resp.copy_handles.first() else {
        return Err(GetSharedMemoryHandleError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid shared memory handle in the response.
    Ok(ShmemHandle::from_raw_unchecked(handle))
}

/// Activates Npad (controller) input with revision support.
///
/// This is IHidServer command 109 (ActivateNpadWithRevision).
/// Uses revision 0x5 (for firmware 18.0.0+).
///
/// For older firmware (<5.0.0), use command 103 without revision.
pub fn activate_npad(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<(), ActivateNpadError> {
    // Use modern revision (0x5 for firmware 18.0.0+)
    let revision: u32 = 0x5;
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_NPAD_WITH_REVISION)
        .with_context(0x20)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ActivateNpadError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ActivateNpadError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad style set.
///
/// This is IHidServer command 100.
pub fn set_supported_npad_style_set(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
    style_set: u32,
) -> Result<(), SetSupportedNpadStyleSetError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::SET_SUPPORTED_NPAD_STYLE_SET)
        .with_context(0x20)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(SetSupportedNpadStyleSetError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetSupportedNpadStyleSetError::ParseResponse)?;

    Ok(())
}

/// Sets the supported Npad ID types.
///
/// This is IHidServer command 102.
pub fn set_supported_npad_id_type(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
    ids: &[u32],
) -> Result<(), SetSupportedNpadIdTypeError> {
    let buffer_size = core::mem::size_of_val(ids);
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `ids` is a live `&[u32]`; reinterpreting its bytes as `&[u8]`
    // of the matching length is sound for the lifetime of the borrow, and
    // `u32` has no padding or invalid bit patterns when read as bytes.
    let ids_bytes = unsafe { core::slice::from_raw_parts(ids.as_ptr().cast::<u8>(), buffer_size) };
    let req = cmif::CmifRequestBuilder::new(cmds::SET_SUPPORTED_NPAD_ID_TYPE)
        .with_context(0x20)
        .with_data_value(&aruid)
        .add_in_pointer(InPointer::new(ids_bytes))
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(SetSupportedNpadIdTypeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetSupportedNpadIdTypeError::ParseResponse)?;

    Ok(())
}

/// Activates touch screen input.
///
/// This is IHidServer command 11.
pub fn activate_touch_screen(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<(), ActivateTouchScreenError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_TOUCH_SCREEN)
        .with_context(0x20)
        .with_data_value(&aruid)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ActivateTouchScreenError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ActivateTouchScreenError::ParseResponse)?;

    Ok(())
}

/// Activates keyboard input.
///
/// This is IHidServer command 31.
pub fn activate_keyboard(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<(), ActivateKeyboardError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_KEYBOARD)
        .with_context(0x20)
        .with_data_value(&aruid)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ActivateKeyboardError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ActivateKeyboardError::ParseResponse)?;

    Ok(())
}

/// Activates mouse input.
///
/// This is IHidServer command 21.
pub fn activate_mouse(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<(), ActivateMouseError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_MOUSE)
        .with_context(0x20)
        .with_data_value(&aruid)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ActivateMouseError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ActivateMouseError::ParseResponse)?;

    Ok(())
}

/// Activates gesture recognition.
///
/// This is IHidServer command 91.
pub fn activate_gesture(
    session: BorrowedSessionHandle<'_>,
    aruid: Option<Aruid>,
) -> Result<(), ActivateGestureError> {
    let aruid = aruid.map(|a| a.to_raw()).unwrap_or(NO_ARUID);

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmds::ACTIVATE_GESTURE)
        .with_context(0x20)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(ActivateGestureError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ActivateGestureError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`create_applet_resource`].
#[derive(Debug, thiserror::Error)]
pub enum CreateAppletResourceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

impl ToResultCode for CreateAppletResourceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`get_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryHandleError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing shared memory handle in response.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

impl ToResultCode for GetSharedMemoryHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`activate_npad`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateNpadError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ActivateNpadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`set_supported_npad_style_set`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadStyleSetError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetSupportedNpadStyleSetError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`set_supported_npad_id_type`].
#[derive(Debug, thiserror::Error)]
pub enum SetSupportedNpadIdTypeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetSupportedNpadIdTypeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`activate_touch_screen`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateTouchScreenError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ActivateTouchScreenError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`activate_keyboard`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateKeyboardError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ActivateKeyboardError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`activate_mouse`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateMouseError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ActivateMouseError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`activate_gesture`].
#[derive(Debug, thiserror::Error)]
pub enum ActivateGestureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ActivateGestureError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
