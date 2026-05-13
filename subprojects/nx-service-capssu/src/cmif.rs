//! CMIF protocol operations for the screenshot upload service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_SHIM_LIBRARY_VERSION)
        .data_size(size_of::<SetShimVersionIn>())
        .send_pid()
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = SetShimVersionIn {
        version,
        applet_resource_user_id,
    };

    // SAFETY: req.data points to valid payload area with space for SetShimVersionIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<SetShimVersionIn>().cast_mut(),
            input,
        );
    }

    ipc::send_sync_request(session).map_err(SetShimVersionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetShimVersionError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SAVE_SCREEN_SHOT_EX0)
        .data_size(size_of::<SaveScreenShotIn>())
        .send_pid()
        .in_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: req.data points to valid payload area with space for SaveScreenShotIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<SaveScreenShotIn>().cast_mut(),
            input,
        );
    }

    req.add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(SaveScreenShotEx0Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<ApplicationAlbumEntry>()) }
        .map_err(SaveScreenShotEx0Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for ApplicationAlbumEntry.
    let entry = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ApplicationAlbumEntry>()) };

    Ok(entry)
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SAVE_SCREEN_SHOT_EX1)
        .data_size(size_of::<SaveScreenShotIn>())
        .send_pid()
        .in_buffers(2)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: req.data points to valid payload area with space for SaveScreenShotIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<SaveScreenShotIn>().cast_mut(),
            input,
        );
    }

    req.add_in_buffer(
        (appdata as *const ApplicationData).cast::<u8>(),
        size_of::<ApplicationData>(),
        BufferMode::Normal,
    );
    req.add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(SaveScreenShotEx1Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<ApplicationAlbumEntry>()) }
        .map_err(SaveScreenShotEx1Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for ApplicationAlbumEntry.
    let entry = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ApplicationAlbumEntry>()) };

    Ok(entry)
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SAVE_SCREEN_SHOT_EX2)
        .data_size(size_of::<SaveScreenShotIn>())
        .in_buffers(2)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: req.data points to valid payload area with space for SaveScreenShotIn.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<SaveScreenShotIn>().cast_mut(),
            input,
        );
    }

    req.add_in_buffer(
        (list as *const UserIdList).cast::<u8>(),
        size_of::<UserIdList>(),
        BufferMode::Normal,
    );
    req.add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure);

    ipc::send_sync_request(session).map_err(SaveScreenShotEx2Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<ApplicationAlbumEntry>()) }
        .map_err(SaveScreenShotEx2Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for ApplicationAlbumEntry.
    let entry = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ApplicationAlbumEntry>()) };

    Ok(entry)
}

/// Error returned by [`set_shim_library_version`].
#[derive(Debug, thiserror::Error)]
pub enum SetShimVersionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`save_screen_shot_ex0`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx0Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`save_screen_shot_ex1`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx1Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`save_screen_shot_ex2`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx2Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
