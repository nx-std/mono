//! CMIF operations for VI root service.
//!
//! The root service is used to get IApplicationDisplayService and
//! fatal display commands (16.0.0+ Manager only).

use core::ptr;

use nx_sf::{cmif, service::Session};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::root_cmds;

/// Gets IApplicationDisplayService session from root service.
///
/// The command ID equals the service type value (0=Application, 1=System, 2=Manager).
/// The input parameter is 1 for System/Manager (uses proxy name exchange), 0 for Application.
pub fn get_display_service(
    session: SessionHandle,
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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmd_id)
            .data_size(4) // inval
            .send(&mut buf)
            .map_err(GetDisplayServiceError::BuildRequest)?;

        // Write inval
        // SAFETY: `req.data` is exactly 4 bytes; writing inval as u32 is sound.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<u32>(), inval);
        }
    }

    ipc::send_sync_request(session).map_err(GetDisplayServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, 0).map_err(GetDisplayServiceError::ParseResponse)?;

    // Sub-service is returned via move handle
    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetDisplayServiceError::MissingHandle);
    };

    // SAFETY: handle is a valid session handle from the kernel
    let session_handle = unsafe { SessionHandle::from_raw(handle) };

    // IApplicationDisplayService is returned via move-handle; libnx does
    // not query its pointer-buffer-size, so skip the kernel round-trip.
    Ok(Session::from_handle(session_handle, 0))
}

/// Prepares the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn prepare_fatal(session: SessionHandle) -> Result<(), PrepareFatalError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(root_cmds::PREPARE_FATAL)
            .send(&mut buf)
            .map_err(PrepareFatalError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(PrepareFatalError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(PrepareFatalError::ParseResponse)?;

    Ok(())
}

/// Shows the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn show_fatal(session: SessionHandle) -> Result<(), ShowFatalError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(root_cmds::SHOW_FATAL)
            .send(&mut buf)
            .map_err(ShowFatalError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(ShowFatalError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(ShowFatalError::ParseResponse)?;

    Ok(())
}

/// Draws a fatal rectangle.
///
/// Available on 16.0.0+ with Manager service type.
pub fn draw_fatal_rectangle(
    session: SessionHandle,
    x: i32,
    y: i32,
    end_x: i32,
    end_y: i32,
    color: u16,
) -> Result<(), DrawFatalRectangleError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        // libnx layout: `struct { u16 color; s32 x, y, end_x, end_y; }` — naturally
        // aligned, total 20 bytes (u16 + 2 bytes padding + 4 * s32).
        #[repr(C)]
        struct Input {
            color: u16,
            x: i32,
            y: i32,
            end_x: i32,
            end_y: i32,
        }

        let input = Input {
            color,
            x,
            y,
            end_x,
            end_y,
        };

        let req = cmif::CmifRequestBuilder::new(root_cmds::DRAW_FATAL_RECTANGLE)
            .data_size(20)
            .send(&mut buf)
            .map_err(DrawFatalRectangleError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(DrawFatalRectangleError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(DrawFatalRectangleError::ParseResponse)?;

    Ok(())
}

/// Draws fatal text using UTF-32 codepoints.
///
/// Available on 16.0.0+ with Manager service type.
#[allow(clippy::too_many_arguments)]
pub fn draw_fatal_text32(
    session: SessionHandle,
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
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        #[repr(C)]
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

        // Add buffer (UTF-32 codepoints as bytes)
        // SAFETY: `utf32_codepoints` is a valid &[u32] slice; reinterpreting its bytes
        // as a u8 slice for the IN buffer is sound (u32 has no padding, any bit pattern
        // is valid as bytes).
        let codepoints_bytes = unsafe {
            core::slice::from_raw_parts(
                utf32_codepoints.as_ptr().cast::<u8>(),
                utf32_codepoints.len() * 4,
            )
        };

        let req = cmif::CmifRequestBuilder::new(root_cmds::DRAW_FATAL_TEXT32)
            .data_size(32) // x(4) + y(4) + scale_x(4) + scale_y(4) + font_type(4) + bg_color(4) + fg_color(4) + initial_advance(4)
            .add_in_buffer(
                codepoints_bytes.as_ptr(),
                codepoints_bytes.len(),
                nx_sf::hipc::BufferMode::Normal,
            )
            .send(&mut buf)
            .map_err(DrawFatalText32Error::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<Input>(), input) };
    }

    ipc::send_sync_request(session).map_err(DrawFatalText32Error::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 4).map_err(DrawFatalText32Error::ParseResponse)?;

    // Output: advance (i32)
    // SAFETY: `resp.data` is exactly 4 bytes; reading it as i32 is sound.
    let advance = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(advance)
}

/// Error from [`get_display_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayServiceError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Missing handle in response.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Error from [`prepare_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum PrepareFatalError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error from [`show_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum ShowFatalError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error from [`draw_fatal_rectangle`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalRectangleError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error from [`draw_fatal_text32`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalText32Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
