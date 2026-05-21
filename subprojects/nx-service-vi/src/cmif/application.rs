//! CMIF operations for IApplicationDisplayService.
//!
//! This is the main display service interface available to all apps.

use core::ptr;

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
    service::Session,
};
use nx_svc::raw::Handle as RawHandle;

use crate::{
    proto::application_cmds,
    types::{DisplayId, DisplayName, LayerId, ViScalingMode},
};

/// Gets IHOSBinderDriverRelay session.
pub fn get_relay_service(session: SessionHandle) -> Result<Session, GetSubServiceError> {
    get_sub_service_no_params(session, application_cmds::GET_RELAY_SERVICE)
}

/// Gets ISystemDisplayService session.
pub fn get_system_display_service(session: SessionHandle) -> Result<Session, GetSubServiceError> {
    get_sub_service_no_params(session, application_cmds::GET_SYSTEM_DISPLAY_SERVICE)
}

/// Gets IManagerDisplayService session.
pub fn get_manager_display_service(session: SessionHandle) -> Result<Session, GetSubServiceError> {
    get_sub_service_no_params(session, application_cmds::GET_MANAGER_DISPLAY_SERVICE)
}

/// Gets IHOSBinderDriverIndirect session (2.0.0+).
pub fn get_indirect_display_transaction_service(
    session: SessionHandle,
) -> Result<Session, GetSubServiceError> {
    get_sub_service_no_params(
        session,
        application_cmds::GET_INDIRECT_DISPLAY_TRANSACTION_SERVICE,
    )
}

/// Helper to get a sub-service with no input parameters.
fn get_sub_service_no_params(
    session: SessionHandle,
    cmd_id: u32,
) -> Result<Session, GetSubServiceError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifRequestBuilder::new(cmd_id)
            .send(&mut buf)
            .map_err(GetSubServiceError::BuildRequest)?;
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetSubServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(GetSubServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetSubServiceError::MissingHandle);
    };

    // SAFETY: handle is a valid session handle from the kernel
    let session_handle = unsafe { SessionHandle::from_raw(handle) };

    // Sub-services here are non-domain handles obtained via move-handle.
    // Skip the pointer-buffer-size query (libnx doesn't issue it on these
    // sub-services either); callers that need pointer-buffer dispatch can
    // upgrade the value later.
    Ok(Session::from_handle(session_handle, 0))
}

/// Opens a display by name.
pub fn open_display(
    session: SessionHandle,
    name: &DisplayName,
) -> Result<DisplayId, OpenDisplayError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::OPEN_DISPLAY)
            .data_size(0x40) // DisplayName
            .send(&mut buf)
            .map_err(OpenDisplayError::BuildRequest)?;

        // Write display name
        // SAFETY: `req` is exactly 0x40 bytes, matching the DisplayName payload size.
        unsafe {
            ptr::copy_nonoverlapping(name.as_bytes().as_ptr(), req.as_mut_ptr(), 0x40);
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(OpenDisplayError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 8).map_err(OpenDisplayError::ParseResponse)?;

    // Output: display_id (u64)
    // SAFETY: `resp.data` is exactly 8 bytes; reading it as u64 is sound.
    let display_id = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(DisplayId::new(display_id))
}

/// Closes a display.
pub fn close_display(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<(), CloseDisplayError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::CLOSE_DISPLAY)
            .data_size(8) // display_id
            .send(&mut buf)
            .map_err(CloseDisplayError::BuildRequest)?;

        // SAFETY: `req` is exactly 8 bytes; writing display_id as u64 is sound.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u64>(), display_id.to_raw());
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(CloseDisplayError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(CloseDisplayError::ParseResponse)?;

    Ok(())
}

/// Display resolution output.
#[derive(Debug, Clone, Copy)]
pub struct DisplayResolution {
    /// Width in pixels.
    pub width: i64,
    /// Height in pixels.
    pub height: i64,
}

/// Gets display resolution.
pub fn get_display_resolution(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<DisplayResolution, GetDisplayResolutionError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::GET_DISPLAY_RESOLUTION)
            .data_size(8) // display_id
            .send(&mut buf)
            .map_err(GetDisplayResolutionError::BuildRequest)?;

        // SAFETY: `req` is exactly 8 bytes; writing display_id as u64 is sound.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u64>(), display_id.to_raw());
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetDisplayResolutionError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, 16).map_err(GetDisplayResolutionError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        width: i64,
        height: i64,
    }

    // SAFETY: `resp.data` is exactly 16 bytes; reading it as Output (two i64) is sound.
    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(DisplayResolution {
        width: output.width,
        height: output.height,
    })
}

/// Native window data from layer operations.
pub const NATIVE_WINDOW_SIZE: usize = 0x100;

/// Output from open_layer.
#[derive(Debug)]
pub struct OpenLayerOutput {
    /// Native window size (bytes of valid data in native_window).
    pub native_window_size: u64,
    /// Native window data (parcel containing IGraphicBufferProducer).
    pub native_window: [u8; NATIVE_WINDOW_SIZE],
}

/// Opens a layer.
pub fn open_layer(
    session: SessionHandle,
    display_name: &DisplayName,
    layer_id: LayerId,
    aruid: u64,
) -> Result<OpenLayerOutput, OpenLayerError> {
    let mut native_window = [0u8; NATIVE_WINDOW_SIZE];

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        #[repr(C)]
        struct Input {
            display_name: [u8; 0x40],
            layer_id: u64,
            aruid: u64,
        }

        let mut input = Input {
            display_name: [0; 0x40],
            layer_id: layer_id.to_raw(),
            aruid,
        };
        input.display_name.copy_from_slice(display_name.as_bytes());

        let req = cmif::CmifRequestBuilder::new(application_cmds::OPEN_LAYER)
            .data_size(0x40 + 8 + 8) // display_name + layer_id + aruid
            .send_pid()
            .add_out_buffer(
                native_window.as_mut_ptr(),
                NATIVE_WINDOW_SIZE,
                BufferMode::Normal,
            )
            .send(&mut buf)
            .map_err(OpenLayerError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<Input>(), input) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(OpenLayerError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 8).map_err(OpenLayerError::ParseResponse)?;

    // Output: native_window_size (u64)
    // SAFETY: `resp.data` is exactly 8 bytes; reading it as u64 is sound.
    let native_window_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(OpenLayerOutput {
        native_window_size,
        native_window,
    })
}

/// Closes a layer.
pub fn close_layer(session: SessionHandle, layer_id: LayerId) -> Result<(), CloseLayerError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::CLOSE_LAYER)
            .data_size(8) // layer_id
            .send(&mut buf)
            .map_err(CloseLayerError::BuildRequest)?;

        // SAFETY: `req` is exactly 8 bytes; writing layer_id as u64 is sound.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u64>(), layer_id.to_raw());
        }
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(CloseLayerError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(CloseLayerError::ParseResponse)?;

    Ok(())
}

/// Output from create_stray_layer.
#[derive(Debug)]
pub struct CreateStrayLayerOutput {
    /// Layer ID.
    pub layer_id: LayerId,
    /// Native window size (bytes of valid data in native_window).
    pub native_window_size: u64,
    /// Native window data (parcel containing IGraphicBufferProducer).
    pub native_window: [u8; NATIVE_WINDOW_SIZE],
}

/// Creates a stray layer on IApplicationDisplayService (cmd 2030).
pub fn create_stray_layer(
    session: SessionHandle,
    layer_flags: u32,
    display_id: DisplayId,
) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
    create_stray_layer_dispatch(
        session,
        application_cmds::CREATE_STRAY_LAYER,
        layer_flags,
        display_id,
    )
}

/// Wire-format helper for `_viCreateStrayLayer`.
///
/// Libnx (`vi.c:_viCreateStrayLayer`) dispatches the same payload to three
/// different sub-services depending on `service_type` + `hosversion`:
/// - `IApplicationDisplayService` cmd 2030 (any service type),
/// - `ISystemDisplayService` cmd 2312 (System/Manager, pre-7.0.0),
/// - `IManagerDisplayService` cmd 2012 (Manager, 7.0.0+).
///
/// The wire format is identical across all three, so the only thing that
/// varies is the (session, cmd_id) pair. The runtime layer chooses which
/// pair to pass in based on hosversion.
pub(crate) fn create_stray_layer_dispatch(
    session: SessionHandle,
    cmd_id: u32,
    layer_flags: u32,
    display_id: DisplayId,
) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
    let mut native_window = [0u8; NATIVE_WINDOW_SIZE];

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

        let req = cmif::CmifRequestBuilder::new(cmd_id)
            .data_size(16) // layer_flags(4) + pad(4) + display_id(8)
            .add_out_buffer(
                native_window.as_mut_ptr(),
                NATIVE_WINDOW_SIZE,
                BufferMode::Normal,
            )
            .send(&mut buf)
            .map_err(CreateStrayLayerError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<Input>(), input) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(CreateStrayLayerError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, 16).map_err(CreateStrayLayerError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        layer_id: u64,
        native_window_size: u64,
    }

    // SAFETY: `resp.data` is exactly 16 bytes; reading it as Output (two u64) is sound.
    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(CreateStrayLayerOutput {
        layer_id: LayerId::new(output.layer_id),
        native_window_size: output.native_window_size,
        native_window,
    })
}

/// Destroys a stray layer.
pub fn destroy_stray_layer(
    session: SessionHandle,
    layer_id: LayerId,
) -> Result<(), DestroyStrayLayerError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::DESTROY_STRAY_LAYER)
            .data_size(8) // layer_id
            .send(&mut buf)
            .map_err(DestroyStrayLayerError::BuildRequest)?;

        // SAFETY: `req` is exactly 8 bytes; writing layer_id as u64 is sound.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u64>(), layer_id.to_raw());
        }
        ipc::send_sync_request(&mut buf, session).map_err(DestroyStrayLayerError::SendRequest)?;
    }

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(DestroyStrayLayerError::ParseResponse)?;

    Ok(())
}

/// Sets layer scaling mode.
pub fn set_layer_scaling_mode(
    session: SessionHandle,
    scaling_mode: ViScalingMode,
    layer_id: LayerId,
) -> Result<(), SetLayerScalingModeError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        #[repr(C)]
        struct Input {
            scaling_mode: u32,
            pad: u32,
            layer_id: u64,
        }

        let input = Input {
            scaling_mode: scaling_mode as u32,
            pad: 0,
            layer_id: layer_id.to_raw(),
        };

        let req = cmif::CmifRequestBuilder::new(application_cmds::SET_LAYER_SCALING_MODE)
            .data_size(16) // scaling_mode(4) + pad(4) + layer_id(8)
            .send(&mut buf)
            .map_err(SetLayerScalingModeError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<Input>(), input) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(SetLayerScalingModeError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(SetLayerScalingModeError::ParseResponse)?;

    Ok(())
}

/// Output from get_indirect_layer_image_map.
#[derive(Debug, Clone, Copy)]
pub struct IndirectLayerImageInfo {
    /// Image size in bytes.
    pub size: i64,
    /// Image stride in bytes.
    pub stride: i64,
}

/// Gets indirect layer image map.
#[allow(clippy::too_many_arguments)]
pub fn get_indirect_layer_image_map(
    session: SessionHandle,
    width: i64,
    height: i64,
    indirect_layer_consumer_handle: u64,
    aruid: u64,
    buffer: &mut [u8],
) -> Result<IndirectLayerImageInfo, GetIndirectLayerImageMapError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        #[repr(C)]
        struct Input {
            width: i64,
            height: i64,
            indirect_layer_consumer_handle: u64,
            aruid: u64,
        }

        let input = Input {
            width,
            height,
            indirect_layer_consumer_handle,
            aruid,
        };

        // HipcMapTransferAllowsNonSecure maps to NonSecure buffer mode
        let req = cmif::CmifRequestBuilder::new(application_cmds::GET_INDIRECT_LAYER_IMAGE_MAP)
            .data_size(32) // width(8) + height(8) + handle(8) + aruid(8)
            .send_pid()
            .add_out_buffer(buffer.as_mut_ptr(), buffer.len(), BufferMode::NonSecure)
            .send(&mut buf)
            .map_err(GetIndirectLayerImageMapError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<Input>(), input) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetIndirectLayerImageMapError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 16)
        .map_err(GetIndirectLayerImageMapError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        size: i64,
        stride: i64,
    }

    // SAFETY: `resp.data` is exactly 16 bytes; reading it as Output (two i64) is sound.
    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(IndirectLayerImageInfo {
        size: output.size,
        stride: output.stride,
    })
}

/// Output from get_indirect_layer_image_required_memory_info.
#[derive(Debug, Clone, Copy)]
pub struct IndirectLayerMemoryInfo {
    /// Required memory size in bytes.
    pub size: i64,
    /// Required memory alignment.
    pub alignment: i64,
}

/// Gets indirect layer image required memory info.
pub fn get_indirect_layer_image_required_memory_info(
    session: SessionHandle,
    width: i64,
    height: i64,
) -> Result<IndirectLayerMemoryInfo, GetIndirectLayerImageRequiredMemoryInfoError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

        #[repr(C)]
        struct Input {
            width: i64,
            height: i64,
        }

        let input = Input { width, height };

        let req = cmif::CmifRequestBuilder::new(
            application_cmds::GET_INDIRECT_LAYER_IMAGE_REQUIRED_MEMORY_INFO,
        )
        .data_size(16) // width(8) + height(8)
        .send(&mut buf)
        .map_err(GetIndirectLayerImageRequiredMemoryInfoError::BuildRequest)?;

        // SAFETY: `req` is exactly `size_of::<Input>()` bytes.
        unsafe { ptr::write_unaligned(req.as_mut_ptr().cast::<Input>(), input) };
        ipc::send_sync_request(&mut buf, session)
    }
    .map_err(GetIndirectLayerImageRequiredMemoryInfoError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 16)
        .map_err(GetIndirectLayerImageRequiredMemoryInfoError::ParseResponse)?;

    #[repr(C)]
    struct Output {
        size: i64,
        alignment: i64,
    }

    // SAFETY: `resp.data` is exactly 16 bytes; reading it as Output (two i64) is sound.
    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok(IndirectLayerMemoryInfo {
        size: output.size,
        alignment: output.alignment,
    })
}

/// Gets display vsync event.
pub fn get_display_vsync_event(
    session: SessionHandle,
    display_id: DisplayId,
) -> Result<RawHandle, GetDisplayVsyncEventError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(application_cmds::GET_DISPLAY_VSYNC_EVENT)
            .data_size(8) // display_id
            .send(&mut buf)
            .map_err(GetDisplayVsyncEventError::BuildRequest)?;

        // SAFETY: `req` is exactly 8 bytes; writing display_id as u64 is sound.
        unsafe {
            ptr::write_unaligned(req.as_mut_ptr().cast::<u64>(), display_id.to_raw());
        }
        ipc::send_sync_request(&mut buf, session)
            .map_err(GetDisplayVsyncEventError::SendRequest)?;
    }

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        cmif::parse_response_bytes(&buf, 0).map_err(GetDisplayVsyncEventError::ParseResponse)?;

    let Some(&event_handle) = resp.copy_handles.first() else {
        return Err(GetDisplayVsyncEventError::MissingHandle);
    };

    Ok(event_handle)
}

// Error types

/// Error from sub-service acquisition.
#[derive(Debug, thiserror::Error)]
pub enum GetSubServiceError {
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

/// Error from [`open_display`].
#[derive(Debug, thiserror::Error)]
pub enum OpenDisplayError {
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

/// Error from [`close_display`].
#[derive(Debug, thiserror::Error)]
pub enum CloseDisplayError {
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

/// Error from [`get_display_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayResolutionError {
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

/// Error from [`open_layer`].
#[derive(Debug, thiserror::Error)]
pub enum OpenLayerError {
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

/// Error from [`close_layer`].
#[derive(Debug, thiserror::Error)]
pub enum CloseLayerError {
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

/// Error from [`create_stray_layer`].
#[derive(Debug, thiserror::Error)]
pub enum CreateStrayLayerError {
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

/// Error from [`destroy_stray_layer`].
#[derive(Debug, thiserror::Error)]
pub enum DestroyStrayLayerError {
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

/// Error from [`set_layer_scaling_mode`].
#[derive(Debug, thiserror::Error)]
pub enum SetLayerScalingModeError {
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

/// Error from [`get_indirect_layer_image_map`].
#[derive(Debug, thiserror::Error)]
pub enum GetIndirectLayerImageMapError {
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

/// Error from [`get_indirect_layer_image_required_memory_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetIndirectLayerImageRequiredMemoryInfoError {
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

/// Error from [`get_display_vsync_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayVsyncEventError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Missing event handle in response.
    #[error("missing event handle in response")]
    MissingHandle,
}
