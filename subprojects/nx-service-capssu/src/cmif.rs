//! CMIF protocol operations for the screenshot upload service.

use core::{mem::size_of, ptr};

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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SET_SHIM_LIBRARY_VERSION)
            .data_size(size_of::<SetShimVersionIn>())
            .send_pid()
            .send()
            .map_err(SetShimVersionError::BuildRequest)?;

        let input = SetShimVersionIn {
            version,
            applet_resource_user_id,
        };

        // SAFETY: `req.data` is exactly `size_of::<SetShimVersionIn>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SetShimVersionIn>(), input) };
    }

    ipc::send_sync_request(session).map_err(SetShimVersionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(buf.as_array(), 0).map_err(SetShimVersionError::ParseResponse)?;

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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SAVE_SCREEN_SHOT_EX0)
            .data_size(size_of::<SaveScreenShotIn>())
            .send_pid()
            .add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure)
            .send()
            .map_err(SaveScreenShotEx0Error::BuildRequest)?;

        let input = SaveScreenShotIn {
            attr: *attr,
            report_option,
            _pad: 0,
            applet_resource_user_id,
        };

        // SAFETY: `req.data` is exactly `size_of::<SaveScreenShotIn>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SaveScreenShotIn>(), input) };
    }

    ipc::send_sync_request(session).map_err(SaveScreenShotEx0Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<ApplicationAlbumEntry>())
        .map_err(SaveScreenShotEx0Error::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<ApplicationAlbumEntry>()` bytes.
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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SAVE_SCREEN_SHOT_EX1)
            .data_size(size_of::<SaveScreenShotIn>())
            .send_pid()
            .add_in_buffer(
                (appdata as *const ApplicationData).cast::<u8>(),
                size_of::<ApplicationData>(),
                BufferMode::Normal,
            )
            .add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure)
            .send()
            .map_err(SaveScreenShotEx1Error::BuildRequest)?;

        let input = SaveScreenShotIn {
            attr: *attr,
            report_option,
            _pad: 0,
            applet_resource_user_id,
        };

        // SAFETY: `req.data` is exactly `size_of::<SaveScreenShotIn>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SaveScreenShotIn>(), input) };
    }

    ipc::send_sync_request(session).map_err(SaveScreenShotEx1Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<ApplicationAlbumEntry>())
        .map_err(SaveScreenShotEx1Error::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<ApplicationAlbumEntry>()` bytes.
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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SAVE_SCREEN_SHOT_EX2)
            .data_size(size_of::<SaveScreenShotIn>())
            .add_in_buffer(
                (list as *const UserIdList).cast::<u8>(),
                size_of::<UserIdList>(),
                BufferMode::Normal,
            )
            .add_in_buffer(image.as_ptr(), image.len(), BufferMode::NonSecure)
            .send()
            .map_err(SaveScreenShotEx2Error::BuildRequest)?;

        let input = SaveScreenShotIn {
            attr: *attr,
            report_option,
            _pad: 0,
            applet_resource_user_id,
        };

        // SAFETY: `req.data` is exactly `size_of::<SaveScreenShotIn>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SaveScreenShotIn>(), input) };
    }

    ipc::send_sync_request(session).map_err(SaveScreenShotEx2Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(buf.as_array(), size_of::<ApplicationAlbumEntry>())
        .map_err(SaveScreenShotEx2Error::ParseResponse)?;

    // SAFETY: `resp.data` is at least `size_of::<ApplicationAlbumEntry>()` bytes.
    let entry = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ApplicationAlbumEntry>()) };

    Ok(entry)
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
    ParseResponse(#[source] cmif::ParseRespBytesError),
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
    ParseResponse(#[source] cmif::ParseRespBytesError),
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
    ParseResponse(#[source] cmif::ParseRespBytesError),
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
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
