//! CMIF protocol operations for the screenshot upload service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
    },
    service::BorrowedSessionHandle,
};
use static_assertions::const_assert_eq;
use zerocopy::IntoBytes as _;

use super::proto;
use crate::{
    album::ApplicationAlbumEntry,
    screenshot::{
        ApplicationData,
        ScreenShotAttribute,
    },
    user::UserIdList,
};

/// Wire-layout input for [`set_shim_library_version`] (cmd 32).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SetShimVersionIn {
    /// Shim library version the caller implements.
    version: u64,
    /// Applet resource user ID the session belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

/// Sets the shim library version (cmd 32). \[7.0.0+\]
pub(crate) fn set_shim_library_version(
    session: BorrowedSessionHandle<'_>,
    version: u64,
    applet_resource_user_id: u64,
) -> Result<(), SetShimVersionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = SetShimVersionIn {
        version,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SET_SHIM_LIBRARY_VERSION)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(SetShimVersionError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetShimVersionError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_shim_library_version`].
#[derive(Debug, thiserror::Error)]
pub enum SetShimVersionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Wire-layout input for the save-screenshot commands (cmds 203, 205, 210).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SaveScreenShotIn {
    /// Attributes the image is saved with.
    attr: ScreenShotAttribute,
    /// Whether the overlay notification is displayed.
    report_option: u32,
    /// Padding the wire form carries after the report option.
    _pad: u32,
    /// Applet resource user ID the screenshot is attributed to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SaveScreenShotIn>(), 0x50);

/// Saves a screenshot with the given attributes (cmd 203). \[4.0.0+\]
pub(crate) fn save_screen_shot_ex0(
    session: BorrowedSessionHandle<'_>,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx0Error> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX0)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(image, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(SaveScreenShotEx0Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx0Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`save_screen_shot_ex0`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx0Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Saves a screenshot with attributes and application data (cmd 205). \[7.0.0+\]
pub(crate) fn save_screen_shot_ex1(
    session: BorrowedSessionHandle<'_>,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    appdata: &ApplicationData,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx1Error> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX1)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(appdata.as_bytes(), BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(image, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(SaveScreenShotEx1Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx1Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`save_screen_shot_ex1`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx1Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Saves a screenshot with attributes and user IDs (cmd 210). \[6.0.0+\]
pub(crate) fn save_screen_shot_ex2(
    session: BorrowedSessionHandle<'_>,
    attr: &ScreenShotAttribute,
    report_option: u32,
    applet_resource_user_id: u64,
    list: &UserIdList,
    image: &[u8],
) -> Result<ApplicationAlbumEntry, SaveScreenShotEx2Error> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let input = SaveScreenShotIn {
        attr: *attr,
        report_option,
        _pad: 0,
        applet_resource_user_id,
    };
    let req = cmif::CmifRequestBuilder::new(proto::SAVE_SCREEN_SHOT_EX2)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(list.as_bytes(), BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(image, BufferMode::NonSecure))
        .build();
    req.send(&mut buf, session)
        .map_err(SaveScreenShotEx2Error::SendRequest)?;

    let resp = cmif::parse_response::<&ApplicationAlbumEntry>(&buf)
        .map_err(SaveScreenShotEx2Error::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`save_screen_shot_ex2`].
#[derive(Debug, thiserror::Error)]
pub enum SaveScreenShotEx2Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
