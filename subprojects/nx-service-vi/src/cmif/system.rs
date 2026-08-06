//! CMIF operations for ISystemDisplayService.
//!
//! Available to System and Manager service types.

use nx_sf::{
    cmif,
    error::{
        ResultCode,
        ToResultCode,
    },
    service::BorrowedSessionHandle,
};

use crate::{
    cmif::application::{
        CreateStrayLayerError,
        CreateStrayLayerOutput,
    },
    proto::system_cmds,
    types::{
        DisplayId,
        LayerId,
    },
};

/// Creates a stray layer on ISystemDisplayService (cmd 2312, pre-7.0.0).
pub fn create_stray_layer(
    session: BorrowedSessionHandle<'_>,
    layer_flags: u32,
    display_id: DisplayId,
) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
    crate::cmif::application::create_stray_layer_dispatch(
        session,
        system_cmds::CREATE_STRAY_LAYER,
        layer_flags,
        display_id,
    )
}

/// Gets Z-order count minimum.
pub fn get_z_order_count_min(
    session: BorrowedSessionHandle<'_>,
    display_id: DisplayId,
) -> Result<i64, GetZOrderCountError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let display_id_raw = display_id.to_raw();
    let req = cmif::CmifRequestBuilder::new(system_cmds::GET_Z_ORDER_COUNT_MIN)
        .with_data_value(&display_id_raw)
        .build();
    req.send(&mut buf, session)
        .map_err(GetZOrderCountError::SendRequest)?;

    let resp = cmif::parse_response::<&i64>(&buf).map_err(GetZOrderCountError::ParseResponse)?;

    let z = *resp.payload;

    Ok(z)
}

/// Gets Z-order count maximum.
pub fn get_z_order_count_max(
    session: BorrowedSessionHandle<'_>,
    display_id: DisplayId,
) -> Result<i64, GetZOrderCountError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let display_id_raw = display_id.to_raw();
    let req = cmif::CmifRequestBuilder::new(system_cmds::GET_Z_ORDER_COUNT_MAX)
        .with_data_value(&display_id_raw)
        .build();
    req.send(&mut buf, session)
        .map_err(GetZOrderCountError::SendRequest)?;

    let resp = cmif::parse_response::<&i64>(&buf).map_err(GetZOrderCountError::ParseResponse)?;

    let z = *resp.payload;

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
    session: BorrowedSessionHandle<'_>,
    display_id: DisplayId,
) -> Result<LogicalResolution, GetDisplayLogicalResolutionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let display_id_raw = display_id.to_raw();
    let req = cmif::CmifRequestBuilder::new(system_cmds::GET_DISPLAY_LOGICAL_RESOLUTION)
        .with_data_value(&display_id_raw)
        .build();
    req.send(&mut buf, session)
        .map_err(GetDisplayLogicalResolutionError::SendRequest)?;

    #[repr(C)]
    #[derive(zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
    struct Output {
        width: i32,
        height: i32,
    }

    let resp = cmif::parse_response::<&Output>(&buf)
        .map_err(GetDisplayLogicalResolutionError::ParseResponse)?;

    Ok(LogicalResolution {
        width: resp.payload.width,
        height: resp.payload.height,
    })
}

/// Sets display magnification (3.0.0+).
pub fn set_display_magnification(
    session: BorrowedSessionHandle<'_>,
    display_id: DisplayId,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), SetDisplayMagnificationError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(system_cmds::SET_DISPLAY_MAGNIFICATION)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(SetDisplayMagnificationError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetDisplayMagnificationError::ParseResponse)?;

    Ok(())
}

/// Sets layer position.
pub fn set_layer_position(
    session: BorrowedSessionHandle<'_>,
    layer_id: LayerId,
    x: f32,
    y: f32,
) -> Result<(), SetLayerPositionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(system_cmds::SET_LAYER_POSITION)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(SetLayerPositionError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetLayerPositionError::ParseResponse)?;

    Ok(())
}

/// Sets layer size.
pub fn set_layer_size(
    session: BorrowedSessionHandle<'_>,
    layer_id: LayerId,
    width: i64,
    height: i64,
) -> Result<(), SetLayerSizeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(system_cmds::SET_LAYER_SIZE)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(SetLayerSizeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetLayerSizeError::ParseResponse)?;

    Ok(())
}

/// Sets layer Z-order.
pub fn set_layer_z(
    session: BorrowedSessionHandle<'_>,
    layer_id: LayerId,
    z: i64,
) -> Result<(), SetLayerZError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    struct Input {
        layer_id: u64,
        z: i64,
    }

    let input = Input {
        layer_id: layer_id.to_raw(),
        z,
    };

    let req = cmif::CmifRequestBuilder::new(system_cmds::SET_LAYER_Z)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(SetLayerZError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetLayerZError::ParseResponse)?;

    Ok(())
}

/// Sets layer visibility.
pub fn set_layer_visibility(
    session: BorrowedSessionHandle<'_>,
    layer_id: LayerId,
    visible: bool,
) -> Result<(), SetLayerVisibilityError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(system_cmds::SET_LAYER_VISIBILITY)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(SetLayerVisibilityError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetLayerVisibilityError::ParseResponse)?;

    Ok(())
}

// Error types

/// Error from Z-order count operations.
#[derive(Debug, thiserror::Error)]
pub enum GetZOrderCountError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for GetZOrderCountError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`get_display_logical_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayLogicalResolutionError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for GetDisplayLogicalResolutionError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`set_display_magnification`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayMagnificationError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetDisplayMagnificationError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`set_layer_position`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerPositionError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetLayerPositionError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`set_layer_size`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerSizeError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetLayerSizeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`set_layer_z`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerZError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetLayerZError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error from [`set_layer_visibility`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerVisibilityError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetLayerVisibilityError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
