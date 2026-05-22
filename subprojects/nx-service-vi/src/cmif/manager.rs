//! CMIF operations for IManagerDisplayService.
//!
//! Available only to Manager service type.

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::{
    cmif::application::{CreateStrayLayerError, CreateStrayLayerOutput},
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(manager_cmds::CREATE_MANAGED_LAYER)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(CreateManagedLayerError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CreateManagedLayerError::SendRequest)?;

    let resp =
        cmif::parse_response::<&u64>(&buf).map_err(CreateManagedLayerError::ParseResponse)?;

    let layer_id = *resp.payload;

    Ok(LayerId::new(layer_id))
}

/// Destroys a managed layer.
pub fn destroy_managed_layer(
    session: SessionHandle,
    layer_id: LayerId,
) -> Result<(), DestroyManagedLayerError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let layer_id_raw = layer_id.to_raw();
    let req = cmif::CmifRequestBuilder::new(manager_cmds::DESTROY_MANAGED_LAYER)
        .with_data_value(&layer_id_raw)
        .build();
    req.write_to(&mut buf)
        .map_err(DestroyManagedLayerError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DestroyManagedLayerError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DestroyManagedLayerError::ParseResponse)?;

    Ok(())
}

/// Creates a stray layer on IManagerDisplayService (cmd 2012, 7.0.0+).
pub fn create_stray_layer(
    session: SessionHandle,
    layer_flags: u32,
    display_id: DisplayId,
) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
    crate::cmif::application::create_stray_layer_dispatch(
        session,
        manager_cmds::CREATE_STRAY_LAYER,
        layer_flags,
        display_id,
    )
}

/// Sets display alpha.
pub fn set_display_alpha(
    session: SessionHandle,
    display_id: DisplayId,
    alpha: f32,
) -> Result<(), SetDisplayAlphaError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(manager_cmds::SET_DISPLAY_ALPHA)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(SetDisplayAlphaError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetDisplayAlphaError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetDisplayAlphaError::ParseResponse)?;

    Ok(())
}

/// Sets display layer stack.
pub fn set_display_layer_stack(
    session: SessionHandle,
    display_id: DisplayId,
    layer_stack: ViLayerStack,
) -> Result<(), SetDisplayLayerStackError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(manager_cmds::SET_DISPLAY_LAYER_STACK)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(SetDisplayLayerStackError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetDisplayLayerStackError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetDisplayLayerStackError::ParseResponse)?;

    Ok(())
}

/// Sets display power state.
pub fn set_display_power_state(
    session: SessionHandle,
    display_id: DisplayId,
    power_state: ViPowerState,
) -> Result<(), SetDisplayPowerStateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(manager_cmds::SET_DISPLAY_POWER_STATE)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(SetDisplayPowerStateError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetDisplayPowerStateError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetDisplayPowerStateError::ParseResponse)?;

    Ok(())
}

/// Adds a layer to a stack.
#[expect(dead_code)]
pub fn add_to_layer_stack(
    session: SessionHandle,
    layer_stack: ViLayerStack,
    layer_id: LayerId,
) -> Result<(), AddToLayerStackError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    #[repr(C)]
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let req = cmif::CmifRequestBuilder::new(manager_cmds::ADD_TO_LAYER_STACK)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(AddToLayerStackError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(AddToLayerStackError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(AddToLayerStackError::ParseResponse)?;

    Ok(())
}

/// Sets content visibility.
pub fn set_content_visibility(
    session: SessionHandle,
    visible: bool,
) -> Result<(), SetContentVisibilityError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let visible_u8: u8 = visible as u8;
    let req = cmif::CmifRequestBuilder::new(manager_cmds::SET_CONTENT_VISIBILITY)
        .with_data_value(&visible_u8)
        .build();
    req.write_to(&mut buf)
        .map_err(SetContentVisibilityError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetContentVisibilityError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetContentVisibilityError::ParseResponse)?;

    Ok(())
}

// Error types

/// Error from [`create_managed_layer`].
#[derive(Debug, thiserror::Error)]
pub enum CreateManagedLayerError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`destroy_managed_layer`].
#[derive(Debug, thiserror::Error)]
pub enum DestroyManagedLayerError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`set_display_alpha`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayAlphaError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`set_display_layer_stack`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayLayerStackError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`set_display_power_state`].
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayPowerStateError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`add_to_layer_stack`].
#[derive(Debug, thiserror::Error)]
pub enum AddToLayerStackError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error from [`set_content_visibility`].
#[derive(Debug, thiserror::Error)]
pub enum SetContentVisibilityError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
