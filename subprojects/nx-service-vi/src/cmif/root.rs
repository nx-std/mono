//! CMIF operations for VI root service.
//!
//! The root service is used to get IApplicationDisplayService and
//! fatal display commands (16.0.0+ Manager only).

use nx_sf::{
    cmif,
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    hipc::InputBuffer,
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
        Session,
    },
};
use zerocopy::IntoBytes as _;

use crate::proto::root_cmds;

/// Gets IApplicationDisplayService session from root service.
///
/// The command ID equals the service type value (0=Application, 1=System, 2=Manager).
/// The input parameter is 1 for System/Manager (uses proxy name exchange), 0 for Application.
pub fn get_display_service(
    session: BorrowedSessionHandle<'_>,
    service_type: crate::types::ViServiceType,
) -> Result<Session, GetDisplayServiceError> {
    // Command ID equals service type value
    let cmd_id = service_type.as_raw() as u32;

    // Input parameter: 1 for System/Manager, 0 for Application
    let inval: u32 = match service_type {
        crate::types::ViServiceType::Application => 0,
        crate::types::ViServiceType::System | crate::types::ViServiceType::Manager => 1,
        // Default should not occur, but treat like Application
        crate::types::ViServiceType::Default => 0,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&inval)
        .build();
    req.send(&mut buf, session)
        .map_err(GetDisplayServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetDisplayServiceError::ParseResponse)?;

    // Sub-service is returned via move handle
    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetDisplayServiceError::MissingHandle);
    };

    // SAFETY: handle is a valid session handle from the kernel
    let session_handle =
        OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(handle));

    // IApplicationDisplayService is returned via move-handle; libnx does
    // not query its pointer-buffer-size, so skip the kernel round-trip.
    Ok(Session::new(session_handle, 0))
}

/// Prepares the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn prepare_fatal(session: BorrowedSessionHandle<'_>) -> Result<(), PrepareFatalError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(root_cmds::PREPARE_FATAL).build();
    req.send(&mut buf, session)
        .map_err(PrepareFatalError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(PrepareFatalError::ParseResponse)?;

    Ok(())
}

/// Shows the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn show_fatal(session: BorrowedSessionHandle<'_>) -> Result<(), ShowFatalError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(root_cmds::SHOW_FATAL).build();
    req.send(&mut buf, session)
        .map_err(ShowFatalError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ShowFatalError::ParseResponse)?;

    Ok(())
}

/// Draws a fatal rectangle.
///
/// Available on 16.0.0+ with Manager service type.
pub fn draw_fatal_rectangle(
    session: BorrowedSessionHandle<'_>,
    x: i32,
    y: i32,
    end_x: i32,
    end_y: i32,
    color: u16,
) -> Result<(), DrawFatalRectangleError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // libnx layout: `struct { u16 color; s32 x, y, end_x, end_y; }` - naturally
    // aligned, total 20 bytes (u16 + 2 bytes padding + 4 * s32).
    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    struct Input {
        color: u16,
        _pad: [u8; 2],
        x: i32,
        y: i32,
        end_x: i32,
        end_y: i32,
    }

    let input = Input {
        color,
        _pad: [0; 2],
        x,
        y,
        end_x,
        end_y,
    };

    let req = cmif::CmifRequestBuilder::new(root_cmds::DRAW_FATAL_RECTANGLE)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(DrawFatalRectangleError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DrawFatalRectangleError::ParseResponse)?;

    Ok(())
}

/// Draws fatal text using UTF-32 codepoints.
///
/// Available on 16.0.0+ with Manager service type.
#[allow(clippy::too_many_arguments)]
pub fn draw_fatal_text32(
    session: BorrowedSessionHandle<'_>,
    x: i32,
    y: i32,
    utf32_codepoints: &[u32],
    scale_x: f32,
    scale_y: f32,
    font_type: u32,
    bg_color: u32,
    fg_color: u32,
    initial_advance: i32,
) -> Result<i32, DrawFatalText32Error> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    struct Input {
        x: i32,
        y: i32,
        scale_x: f32,
        scale_y: f32,
        font_type: u32,
        bg_color: u32,
        fg_color: u32,
        initial_advance: i32,
    }

    let input = Input {
        x,
        y,
        scale_x,
        scale_y,
        font_type,
        bg_color,
        fg_color,
        initial_advance,
    };

    let req = cmif::CmifRequestBuilder::new(root_cmds::DRAW_FATAL_TEXT32)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(
            utf32_codepoints.as_bytes(),
            nx_sf::hipc::BufferMode::Normal,
        ))
        .build();
    req.send(&mut buf, session)
        .map_err(DrawFatalText32Error::SendRequest)?;

    let resp = cmif::parse_response::<&i32>(&buf).map_err(DrawFatalText32Error::ParseResponse)?;

    // Output: advance (i32)
    let advance = *resp.payload;

    Ok(advance)
}

/// Error from [`get_display_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayServiceError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing handle in response.
    #[error("missing handle in response")]
    MissingHandle,
}

impl ToResultCode for GetDisplayServiceError {
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

/// Error from [`prepare_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum PrepareFatalError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for PrepareFatalError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`show_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum ShowFatalError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ShowFatalError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`draw_fatal_rectangle`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalRectangleError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for DrawFatalRectangleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`draw_fatal_text32`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalText32Error {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for DrawFatalText32Error {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
