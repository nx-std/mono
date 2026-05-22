//! CMIF protocol operations for the screenshot upload service.

use core::mem::size_of;

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    proto,
    types::{
        ApplicationAlbumEntry, ApplicationData, SaveScreenShotIn, ScreenShotAttribute,
        SetShimVersionIn, UserIdList,
    },
};

/// Sets the shim library version. Called during initialization on 7.0.0+.
pub fn set_shim_library_version(
    session: SessionHandle,
    version: u64,
    applet_resource_user_id: u64,
) -> Result<(), SetShimVersionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let input = SetShimVersionIn {
        version,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SET_SHIM_LIBRARY_VERSION)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.write_to(&mut buf)
        .map_err(SetShimVersionError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetShimVersionError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetShimVersionError::ParseResponse)?;

    Ok(())
}

/// Saves a screenshot with the given attributes.
pub fn save_screen_shot_ex0(
    session: SessionHandle,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx0Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX0)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer_raw(image.as_ptr(), image.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(SaveScreenShotEx0Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SaveScreenShotEx0Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx0Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Saves a screenshot with attributes and application data. [7.0.0+]
pub fn save_screen_shot_ex1(
    session: SessionHandle,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    appdata: &ApplicationData,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx1Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX1)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer_raw(
            (appdata as *const ApplicationData).cast::<u8>(),
            size_of::<ApplicationData>(),
            BufferMode::Normal,
        )
        .add_input_buffer_raw(image.as_ptr(), image.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(SaveScreenShotEx1Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SaveScreenShotEx1Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx1Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Saves a screenshot with attributes and user IDs. [6.0.0+]
pub fn save_screen_shot_ex2(
    session: SessionHandle,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    list: &UserIdList,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx2Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX2)
        .with_data_value(&input)
        .add_input_buffer_raw(
            (list as *const UserIdList).cast::<u8>(),
            size_of::<UserIdList>(),
            BufferMode::Normal,
        )
        .add_input_buffer_raw(image.as_ptr(), image.len(), BufferMode::NonSecure)
        .build();
    req.write_to(&mut buf)
        .map_err(SaveScreenShotEx2Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SaveScreenShotEx2Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx2Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`set_shim_library_version`].
#[derive(Debug, thiserror::Error)]
pub enum SetShimVersionError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`save_screen_shot_ex0`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx0Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`save_screen_shot_ex1`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx1Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`save_screen_shot_ex2`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx2Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
