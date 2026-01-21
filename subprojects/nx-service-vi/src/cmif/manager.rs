//! CMIF operations for IManagerDisplayService.
//!
//! Available only to Manager service type.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    cmif::application::{CreateStrayLayerOutput, NATIVE_WINDOW_SIZE},
    proto::manager_cmds,
    types::{DisplayId, LayerId, ViLayerStack, ViPowerState},
};

/// Creates a managed layer.
pub fn create_managed_layer(
    session: SessionHandle,
    layer_flags: u32,
    display_id: DisplayId,
    aruid: u64,
) -> Result<LayerId, CreateManagedLayerError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::CREATE_MANAGED_LAYER)
        .data_size(24) // layer_flags(4) + pad(4) + display_id(8) + aruid(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_flags: u32,
        pad: u32,
        display_id: u64,
        aruid: u64,
    }

    let input = Input {
        layer_flags,
        pad: 0,
        display_id: display_id.to_raw(),
        aruid,
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(CreateManagedLayerError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CreateManagedLayerError::ParseResponse)?;

    let layer_id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(LayerId::new(layer_id))
}

/// Destroys a managed layer.
pub fn destroy_managed_layer(
    session: SessionHandle,
    layer_id: LayerId,
) -> Result<(), DestroyManagedLayerError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::DESTROY_MANAGED_LAYER)
        .data_size(8) // layer_id
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<u64>().cast_mut(),
            layer_id.to_raw(),
        );
    }

    ipc::send_sync_request(session).map_err(DestroyManagedLayerError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DestroyManagedLayerError::ParseResponse)?;

    Ok(())
}

/// Creates a stray layer (Manager, 7.0.0+).
#[expect(dead_code)]
pub fn create_stray_layer(
    session: SessionHandle,
    layer_flags: u32,
    display_id: DisplayId,
) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::CREATE_STRAY_LAYER)
        .data_size(16) // layer_flags(4) + pad(4) + display_id(8)
        .out_buffers(1) // native_window
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_flags: u32,
        pad: u32,
        display_id: u64,
    }

    let input = Input {
        layer_flags,
        pad: 0,
        display_id: display_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add output buffer for native window
    let mut native_window = [0u8; NATIVE_WINDOW_SIZE];
    req.add_out_buffer(
        native_window.as_mut_ptr(),
        NATIVE_WINDOW_SIZE,
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(CreateStrayLayerError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CreateStrayLayerError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        layer_id: u64,
        native_window_size: u64,
    }

    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(CreateStrayLayerOutput {
        layer_id: LayerId::new(output.layer_id),
        native_window_size: output.native_window_size,
        native_window,
    })
}

/// Sets display alpha.
pub fn set_display_alpha(
    session: SessionHandle,
    display_id: DisplayId,
    alpha: f32,
) -> Result<(), SetDisplayAlphaError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::SET_DISPLAY_ALPHA)
        .data_size(16) // alpha(4) + pad(4) + display_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        alpha: f32,
        pad: u32,
        display_id: u64,
    }

    let input = Input {
        alpha,
        pad: 0,
        display_id: display_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetDisplayAlphaError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetDisplayAlphaError::ParseResponse)?;

    Ok(())
}

/// Sets display layer stack.
pub fn set_display_layer_stack(
    session: SessionHandle,
    display_id: DisplayId,
    layer_stack: ViLayerStack,
) -> Result<(), SetDisplayLayerStackError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::SET_DISPLAY_LAYER_STACK)
        .data_size(16) // layer_stack(4) + pad(4) + display_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_stack: u32,
        pad: u32,
        display_id: u64,
    }

    let input = Input {
        layer_stack: layer_stack as u32,
        pad: 0,
        display_id: display_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetDisplayLayerStackError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetDisplayLayerStackError::ParseResponse)?;

    Ok(())
}

/// Sets display power state.
pub fn set_display_power_state(
    session: SessionHandle,
    display_id: DisplayId,
    power_state: ViPowerState,
) -> Result<(), SetDisplayPowerStateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::SET_DISPLAY_POWER_STATE)
        .data_size(16) // power_state(4) + pad(4) + display_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        power_state: u32,
        pad: u32,
        display_id: u64,
    }

    let input = Input {
        power_state: power_state as u32,
        pad: 0,
        display_id: display_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetDisplayPowerStateError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetDisplayPowerStateError::ParseResponse)?;

    Ok(())
}

/// Adds a layer to a stack.
#[expect(dead_code)]
pub fn add_to_layer_stack(
    session: SessionHandle,
    layer_stack: ViLayerStack,
    layer_id: LayerId,
) -> Result<(), AddToLayerStackError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::ADD_TO_LAYER_STACK)
        .data_size(16) // layer_stack(4) + pad(4) + layer_id(8)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    #[repr(C)]
    struct Input {
        layer_stack: u32,
        pad: u32,
        layer_id: u64,
    }

    let input = Input {
        layer_stack: layer_stack as u32,
        pad: 0,
        layer_id: layer_id.to_raw(),
    };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(AddToLayerStackError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(AddToLayerStackError::ParseResponse)?;

    Ok(())
}

/// Sets content visibility.
pub fn set_content_visibility(
    session: SessionHandle,
    visible: bool,
) -> Result<(), SetContentVisibilityError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(manager_cmds::SET_CONTENT_VISIBILITY)
        .data_size(1) // visible
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), visible as u8);
    }

    ipc::send_sync_request(session).map_err(SetContentVisibilityError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _ = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetContentVisibilityError::ParseResponse)?;

    Ok(())
}

// Error types

/// Error from [`create_managed_layer`].
#[derive(Debug, thiserror::Error)]
pub enum CreateManagedLayerError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`destroy_managed_layer`].
#[derive(Debug, thiserror::Error)]
pub enum DestroyManagedLayerError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`create_stray_layer`] (Manager).
#[derive(Debug, thiserror::Error)]
pub enum CreateStrayLayerError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_display_alpha`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayAlphaError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_display_layer_stack`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayLayerStackError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_display_power_state`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayPowerStateError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`add_to_layer_stack`].
#[derive(Debug, thiserror::Error)]
pub enum AddToLayerStackError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error from [`set_content_visibility`].
#[derive(Debug, thiserror::Error)]
pub enum SetContentVisibilityError {
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
