//! CMIF operations for ISystemDisplayService.
//!
//! Available to System and Manager service types.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto::system_cmds,
    types::{DisplayId, LayerId},
};

/// Gets Z-order count minimum.
pub fn get_z_order_count_min(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<i64, GetZOrderCountError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::GET_Z_ORDER_COUNT_MIN)
        .data_size(8) // display_id
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<u64>().cast_mut(),
            display_id.to_raw(),
        );
    }

    ipc::send_sync_request(session).map_err(GetZOrderCountError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetZOrderCountError::ParseResponse)?;

    let z = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i64>()) };

    Ok(z)
}

/// Gets Z-order count maximum.
pub fn get_z_order_count_max(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<i64, GetZOrderCountError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::GET_Z_ORDER_COUNT_MAX)
        .data_size(8) // display_id
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<u64>().cast_mut(),
            display_id.to_raw(),
        );
    }

    ipc::send_sync_request(session).map_err(GetZOrderCountError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetZOrderCountError::ParseResponse)?;

    let z = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i64>()) };

    Ok(z)
}

/// Display logical resolution output.
#[derive(Debug, Clone, Copy)]
pub struct LogicalResolution {
    /// Width in logical units.
    pub width: i32,
    /// Height in logical units.
    pub height: i32,
}

/// Gets display logical resolution.
pub fn get_display_logical_resolution(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<LogicalResolution, GetDisplayLogicalResolutionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::GET_DISPLAY_LOGICAL_RESOLUTION)
        .data_size(8) // display_id
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<u64>().cast_mut(),
            display_id.to_raw(),
        );
    }

    ipc::send_sync_request(session).map_err(GetDisplayLogicalResolutionError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetDisplayLogicalResolutionError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        width: i32,
        height: i32,
    }

    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(LogicalResolution {
        width: output.width,
        height: output.height,
    })
}

/// Sets display magnification (3.0.0+).
pub fn set_display_magnification(
    session: SessionHandle,
    display_id: DisplayId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), SetDisplayMagnificationError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::SET_DISPLAY_MAGNIFICATION)
        .data_size(24) // x(4) + y(4) + width(4) + height(4) + display_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        display_id: u64,
    }

    let input = Input {
        x,
        y,
        width,
        height,
        display_id: display_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetDisplayMagnificationError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetDisplayMagnificationError::ParseResponse)?;

    Ok(())
}

/// Sets layer position.
pub fn set_layer_position(
    session: SessionHandle,
    layer_id: LayerId,
    x: f32,
    y: f32,
) -> Result<(), SetLayerPositionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::SET_LAYER_POSITION)
        .data_size(16) // x(4) + y(4) + layer_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        x: f32,
        y: f32,
        layer_id: u64,
    }

    let input = Input {
        x,
        y,
        layer_id: layer_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetLayerPositionError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetLayerPositionError::ParseResponse)?;

    Ok(())
}

/// Sets layer size.
pub fn set_layer_size(
    session: SessionHandle,
    layer_id: LayerId,
    width: i64,
    height: i64,
) -> Result<(), SetLayerSizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::SET_LAYER_SIZE)
        .data_size(24) // layer_id(8) + width(8) + height(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_id: u64,
        width: i64,
        height: i64,
    }

    let input = Input {
        layer_id: layer_id.to_raw(),
        width,
        height,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetLayerSizeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetLayerSizeError::ParseResponse)?;

    Ok(())
}

/// Sets layer Z-order.
pub fn set_layer_z(
    session: SessionHandle,
    layer_id: LayerId,
    z: i64,
) -> Result<(), SetLayerZError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::SET_LAYER_Z)
        .data_size(16) // layer_id(8) + z(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_id: u64,
        z: i64,
    }

    let input = Input {
        layer_id: layer_id.to_raw(),
        z,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetLayerZError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetLayerZError::ParseResponse)?;

    Ok(())
}

/// Sets layer visibility.
pub fn set_layer_visibility(
    session: SessionHandle,
    layer_id: LayerId,
    visible: bool,
) -> Result<(), SetLayerVisibilityError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(system_cmds::SET_LAYER_VISIBILITY)
        .data_size(16) // visible(1) + pad(7) + layer_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        visible: u8,
        _pad: [u8; 7],
        layer_id: u64,
    }

    let input = Input {
        visible: visible as u8,
        _pad: [0; 7],
        layer_id: layer_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetLayerVisibilityError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetLayerVisibilityError::ParseResponse)?;

    Ok(())
}

// Error types

/// Error from Z-order count operations.
#[derive(Debug, thiserror::Error)]
pub enum GetZOrderCountError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`get_display_logical_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayLogicalResolutionError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_display_magnification`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayMagnificationError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_layer_position`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerPositionError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_layer_size`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerSizeError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_layer_z`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerZError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_layer_visibility`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerVisibilityError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
