//! CMIF operations for VI root service.
//!
//! The root service is used to get IApplicationDisplayService and
//! fatal display commands (16.0.0+ Manager only).

use core::ptr;

use nx_sf::{cmif, service::Service};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::root_cmds;

/// Gets IApplicationDisplayService session from root service.
///
/// The command ID equals the service type value (0=Application, 1=System, 2=Manager).
/// The input parameter is 1 for System/Manager (uses proxy name exchange), 0 for Application.
pub fn get_display_service(
    session: SessionHandle,
    service_type: crate::types::ViServiceType,
) -> Result<Service, GetDisplayServiceError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // Command ID equals service type value
    let cmd_id = service_type.as_raw() as u32;

    // Input parameter: 1 for System/Manager, 0 for Application
    let inval: u32 = match service_type {
        crate::types::ViServiceType::Application => 0,
        crate::types::ViServiceType::System | crate::types::ViServiceType::Manager => 1,
        // Default should not occur, but treat like Application
        crate::types::ViServiceType::Default => 0,
    };

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(4) // inval
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write inval
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), inval);
    }

    ipc::send_sync_request(session).map_err(GetDisplayServiceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetDisplayServiceError::ParseResponse)?;

    // Sub-service is returned via move handle
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetDisplayServiceError::MissingHandle)?;

    // SAFETY: handle is a valid session handle from the kernel
    let session_handle = unsafe { SessionHandle::from_raw(handle) };

    Ok(Service::new(session_handle))
}

/// Prepares the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn prepare_fatal(session: SessionHandle) -> Result<(), PrepareFatalError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(root_cmds::PREPARE_FATAL).build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _ = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(PrepareFatalError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(PrepareFatalError::ParseResponse)?;

    Ok(())
}

/// Shows the fatal display.
///
/// Available on 16.0.0+ with Manager service type.
pub fn show_fatal(session: SessionHandle) -> Result<(), ShowFatalError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(root_cmds::SHOW_FATAL).build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _ = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(ShowFatalError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ShowFatalError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(root_cmds::DRAW_FATAL_RECTANGLE)
        .data_size(18) // color(2) + x(4) + y(4) + end_x(4) + end_y(4)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Input struct layout: color(u16), x(i32), y(i32), end_x(i32), end_y(i32)
    // Note: libnx packs this as { u16 color, s32 x, y, end_x, end_y } with alignment
    #[repr(C, packed)]
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

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(DrawFatalRectangleError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DrawFatalRectangleError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(root_cmds::DRAW_FATAL_TEXT32)
        .data_size(32) // x(4) + y(4) + scale_x(4) + scale_y(4) + font_type(4) + bg_color(4) + fg_color(4) + initial_advance(4)
        .in_buffers(1) // UTF-32 codepoints
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

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

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add buffer (UTF-32 codepoints as bytes)
    req.add_in_buffer(
        utf32_codepoints.as_ptr().cast(),
        utf32_codepoints.len() * 4,
        nx_sf::hipc::BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(DrawFatalText32Error::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DrawFatalText32Error::ParseResponse)?;

    // Output: advance (i32)
    let advance = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(advance)
}

/// Error from [`get_display_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayServiceError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing handle in response.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Error from [`prepare_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum PrepareFatalError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`show_fatal`].
#[derive(Debug, thiserror::Error)]
pub enum ShowFatalError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`draw_fatal_rectangle`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalRectangleError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`draw_fatal_text32`].
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalText32Error {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
