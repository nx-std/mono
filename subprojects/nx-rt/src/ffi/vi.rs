//! Visual Interface (VI) service FFI

use core::{ffi::c_void, mem::MaybeUninit};

use nx_sf::{cmif, service::Service};

use super::common::{GENERIC_ERROR, LibnxError, SyncUnsafeCell, libnx_error};
use crate::{applet_manager, vi_manager};

/// Static buffer for VI IApplicationDisplayService FFI session access.
static VI_FFI_APPLICATION_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IHOSBinderDriverRelay FFI session access.
static VI_FFI_BINDER_RELAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI ISystemDisplayService FFI session access.
static VI_FFI_SYSTEM_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IManagerDisplayService FFI session access.
static VI_FFI_MANAGER_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IHOSBinderDriverIndirect FFI session access.
static VI_FFI_BINDER_INDIRECT: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// C-compatible display structure matching libnx ViDisplay.
#[repr(C)]
pub struct ViDisplay {
    /// Display ID.
    pub display_id: u64,
    /// Display name (64 bytes, null-terminated).
    pub display_name: [u8; 0x40],
    /// Whether the display is initialized.
    pub initialized: bool,
}

/// C-compatible layer structure matching libnx ViLayer.
#[repr(C)]
pub struct ViLayer {
    /// Layer ID.
    pub layer_id: u64,
    /// IGraphicBufferProducer binder object ID.
    pub igbp_binder_obj_id: u32,
    /// Flags: bit 0 = initialized, bit 1 = stray_layer
    flags: u8,
}

impl ViLayer {
    /// Returns whether the layer is initialized.
    #[inline]
    fn is_initialized(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Returns whether this is a stray layer.
    #[inline]
    fn is_stray_layer(&self) -> bool {
        self.flags & 0x02 != 0
    }
}

/// Initializes the VI service.
///
/// Corresponds to `viInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_initialize(service_type: i32) -> u32 {
    let vi_service_type = match nx_service_vi::types::ViServiceType::from_raw(service_type) {
        Some(st) => st,
        None => return GENERIC_ERROR,
    };

    // Check if this is the first initialization
    let was_initialized = vi_manager::is_initialized();

    match vi_manager::init(vi_service_type) {
        Ok(()) => {
            // Only update FFI session buffers on first actual initialization
            if !was_initialized {
                set_vi_ffi_sessions();
            }
            0
        }
        Err(err) => vi_connect_error_to_rc(err),
    }
}

/// Exits the VI service.
///
/// Corresponds to `viExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_exit() {
    let was_initialized = vi_manager::is_initialized();
    vi_manager::exit();
    let still_initialized = vi_manager::is_initialized();

    // Only clear FFI session buffers if the service was actually closed
    if was_initialized && !still_initialized {
        clear_vi_ffi_sessions();
    }
}

/// Gets the IApplicationDisplayService session pointer.
///
/// Corresponds to `viGetSession_IApplicationDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_application_display() -> *mut Service {
    VI_FFI_APPLICATION_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverRelay session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverRelay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_binder_relay() -> *mut Service {
    VI_FFI_BINDER_RELAY.get().cast::<Service>()
}

/// Gets the ISystemDisplayService session pointer.
///
/// Corresponds to `viGetSession_ISystemDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_system_display() -> *mut Service {
    VI_FFI_SYSTEM_DISPLAY.get().cast::<Service>()
}

/// Gets the IManagerDisplayService session pointer.
///
/// Corresponds to `viGetSession_IManagerDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_manager_display() -> *mut Service {
    VI_FFI_MANAGER_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverIndirect session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverIndirect()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_binder_indirect() -> *mut Service {
    VI_FFI_BINDER_INDIRECT.get().cast::<Service>()
}

/// Opens a display by name.
///
/// Corresponds to `viOpenDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_open_display(
    name: *const core::ffi::c_char,
    display: *mut ViDisplay,
) -> u32 {
    if name.is_null() || display.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Zero-initialize the display struct
    unsafe { core::ptr::write_bytes(display, 0, 1) };

    // Copy display name from C string
    let display_ref = unsafe { &mut *display };
    let name_cstr = unsafe { core::ffi::CStr::from_ptr(name) };
    let name_bytes = name_cstr.to_bytes();
    let copy_len = name_bytes.len().min(0x3F);
    display_ref.display_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

    // Create DisplayName from the bytes
    let vi_display_name =
        nx_service_vi::DisplayName::from_ascii(name_cstr.to_str().unwrap_or("Default"));

    match service.open_display(&vi_display_name) {
        Ok(display_id) => {
            display_ref.display_id = display_id.to_raw();
            display_ref.initialized = true;
            0
        }
        Err(err) => vi_open_display_error_to_rc(err),
    }
}

/// Closes a display.
///
/// Corresponds to `viCloseDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_close_display(display: *mut ViDisplay) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &mut *display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.close_display(display_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(display, 0, 1) };
            0
        }
        Err(err) => vi_close_display_error_to_rc(err),
    }
}

/// Gets display resolution.
///
/// Corresponds to `viGetDisplayResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_resolution(
    display: *const ViDisplay,
    width: *mut i32,
    height: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_resolution(display_id) {
        Ok(res) => {
            if !width.is_null() {
                unsafe { *width = res.width as i32 };
            }
            if !height.is_null() {
                unsafe { *height = res.height as i32 };
            }
            0
        }
        Err(err) => vi_get_display_resolution_error_to_rc(err),
    }
}

/// Gets display logical resolution.
///
/// Corresponds to `viGetDisplayLogicalResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_logical_resolution(
    display: *const ViDisplay,
    width: *mut i32,
    height: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_logical_resolution(display_id) {
        Ok(res) => {
            if !width.is_null() {
                unsafe { *width = res.width };
            }
            if !height.is_null() {
                unsafe { *height = res.height };
            }
            0
        }
        Err(err) => vi_get_display_logical_resolution_error_to_rc(err),
    }
}

/// Sets display magnification (3.0.0+).
///
/// Corresponds to `viSetDisplayMagnification()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_magnification(
    display: *const ViDisplay,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_magnification(display_id, x, y, width, height) {
        Ok(()) => 0,
        Err(err) => vi_set_display_magnification_error_to_rc(err),
    }
}

/// Gets display vsync event handle.
///
/// Corresponds to `viGetDisplayVsyncEvent()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_vsync_event(
    display: *const ViDisplay,
    event_handle_out: *mut u32,
) -> u32 {
    if display.is_null() || event_handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_vsync_event(display_id) {
        Ok(handle) => {
            unsafe { *event_handle_out = handle };
            0
        }
        Err(err) => vi_get_display_vsync_event_error_to_rc(err),
    }
}

/// Sets display power state.
///
/// Corresponds to `viSetDisplayPowerState()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_power_state(
    display: *const ViDisplay,
    power_state: u32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    let state = match power_state {
        0 => nx_service_vi::ViPowerState::Off,
        1 => nx_service_vi::ViPowerState::NotScanning,
        2 => nx_service_vi::ViPowerState::On,
        _ => return GENERIC_ERROR,
    };

    match service.set_display_power_state(display_id, state) {
        Ok(()) => 0,
        Err(err) => vi_set_display_power_state_error_to_rc(err),
    }
}

/// Sets display alpha.
///
/// Corresponds to `viSetDisplayAlpha()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_alpha(
    display: *const ViDisplay,
    alpha: f32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_alpha(display_id, alpha) {
        Ok(()) => 0,
        Err(err) => vi_set_display_alpha_error_to_rc(err),
    }
}

/// Gets Z-order count minimum.
///
/// Corresponds to `viGetZOrderCountMin()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_z_order_count_min(
    display: *const ViDisplay,
    z: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_z_order_count_min(display_id) {
        Ok(min_z) => {
            if !z.is_null() {
                unsafe { *z = min_z };
            }
            0
        }
        Err(err) => vi_get_z_order_count_min_error_to_rc(err),
    }
}

/// Gets Z-order count maximum.
///
/// Corresponds to `viGetZOrderCountMax()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_z_order_count_max(
    display: *const ViDisplay,
    z: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_z_order_count_max(display_id) {
        Ok(max_z) => {
            if !z.is_null() {
                unsafe { *z = max_z };
            }
            0
        }
        Err(err) => vi_get_z_order_count_max_error_to_rc(err),
    }
}

/// Creates a layer (uses stray layer or managed layer depending on context).
///
/// Corresponds to `viCreateLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_create_layer(
    display: *const ViDisplay,
    layer: *mut ViLayer,
) -> u32 {
    if display.is_null() || layer.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Zero-initialize the layer struct
    unsafe { core::ptr::write_bytes(layer, 0, 1) };
    let layer_ref = unsafe { &mut *layer };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    // Create stray layer (simplified - libnx has more complex logic)
    match service.create_stray_layer(nx_service_vi::ViLayerFlags::Default, display_id) {
        Ok(output) => {
            layer_ref.layer_id = output.layer_id.to_raw();
            // Parse parcel to get binder object ID
            layer_ref.igbp_binder_obj_id =
                parse_native_window_binder_id(&output.native_window).unwrap_or(0);
            layer_ref.flags = 0x03; // initialized (0x01) | stray_layer (0x02)
            0
        }
        Err(err) => vi_create_stray_layer_error_to_rc(err),
    }
}

/// Creates a managed layer.
///
/// Corresponds to `viCreateManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_create_managed_layer(
    display: *const ViDisplay,
    _layer_flags: u32,
    aruid: u64,
    layer_id_out: *mut u64,
) -> u32 {
    if display.is_null() || layer_id_out.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    // Currently only Default flags are supported (layer_flags is ignored)
    let flags = nx_service_vi::ViLayerFlags::Default;

    match service.create_managed_layer(flags, display_id, aruid) {
        Ok(layer_id) => {
            unsafe { *layer_id_out = layer_id.to_raw() };
            0
        }
        Err(err) => vi_create_managed_layer_error_to_rc(err),
    }
}

/// Destroys a managed layer.
///
/// Corresponds to `viDestroyManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_destroy_managed_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.destroy_managed_layer(layer_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(layer, 0, 1) };
            0
        }
        Err(err) => vi_destroy_managed_layer_error_to_rc(err),
    }
}

/// Closes a layer.
///
/// Corresponds to `viCloseLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_close_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    let rc = if layer_ref.is_stray_layer() {
        match service.destroy_stray_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => vi_destroy_stray_layer_error_to_rc(err),
        }
    } else {
        match service.close_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => vi_close_layer_error_to_rc(err),
        }
    };

    if rc == 0 {
        // Zero-initialize the struct on success
        unsafe { core::ptr::write_bytes(layer, 0, 1) };
    }
    rc
}

/// Sets layer size.
///
/// Corresponds to `viSetLayerSize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_size(
    layer: *const ViLayer,
    width: i32,
    height: i32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_size(layer_id, width, height) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_size_error_to_rc(err),
    }
}

/// Sets layer Z-order.
///
/// Corresponds to `viSetLayerZ()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_z(layer: *const ViLayer, z: i32) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_z(layer_id, z) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_z_error_to_rc(err),
    }
}

/// Sets layer position.
///
/// Corresponds to `viSetLayerPosition()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_position(
    layer: *const ViLayer,
    x: f32,
    y: f32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_position(layer_id, x, y) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_position_error_to_rc(err),
    }
}

/// Sets layer scaling mode.
///
/// Corresponds to `viSetLayerScalingMode()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_scaling_mode(
    layer: *const ViLayer,
    scaling_mode: u32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    let mode = match scaling_mode {
        0 => nx_service_vi::ViScalingMode::None,
        2 => nx_service_vi::ViScalingMode::FitToLayer,
        4 => nx_service_vi::ViScalingMode::PreserveAspectRatio,
        _ => return GENERIC_ERROR,
    };

    match service.set_layer_scaling_mode(layer_id, mode) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_scaling_mode_error_to_rc(err),
    }
}

/// Gets indirect layer image map.
///
/// Corresponds to `viGetIndirectLayerImageMap()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_indirect_layer_image_map(
    buffer: *mut c_void,
    size: usize,
    width: i32,
    height: i32,
    indirect_layer_consumer_handle: u64,
    out_size: *mut u64,
    out_stride: *mut u64,
) -> u32 {
    if buffer.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Get ARUID from applet manager
    let aruid = applet_manager::get_applet_resource_user_id()
        .map(|a| a.to_raw())
        .unwrap_or(0);

    let buffer_slice = unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size) };

    match service.get_indirect_layer_image_map(
        width,
        height,
        indirect_layer_consumer_handle,
        aruid,
        buffer_slice,
    ) {
        Ok(info) => {
            if !out_size.is_null() {
                unsafe { *out_size = info.size as u64 };
            }
            if !out_stride.is_null() {
                unsafe { *out_stride = info.stride as u64 };
            }
            0
        }
        Err(err) => vi_get_indirect_layer_image_map_error_to_rc(err),
    }
}

/// Gets indirect layer image required memory info.
///
/// Corresponds to `viGetIndirectLayerImageRequiredMemoryInfo()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_indirect_layer_image_required_memory_info(
    width: i32,
    height: i32,
    out_size: *mut u64,
    out_alignment: *mut u64,
) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.get_indirect_layer_image_required_memory_info(width, height) {
        Ok(info) => {
            if !out_size.is_null() {
                unsafe { *out_size = info.size as u64 };
            }
            if !out_alignment.is_null() {
                unsafe { *out_alignment = info.alignment as u64 };
            }
            0
        }
        Err(err) => vi_get_indirect_layer_image_required_memory_info_error_to_rc(err),
    }
}

/// Sets content visibility.
///
/// Corresponds to `viSetContentVisibility()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_content_visibility(visible: bool) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.set_content_visibility(visible) {
        Ok(()) => 0,
        Err(err) => vi_set_content_visibility_error_to_rc(err),
    }
}

/// Prepares the fatal display (16.0.0+).
///
/// Corresponds to `viManagerPrepareFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_prepare_fatal() -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.prepare_fatal() {
        Ok(()) => 0,
        Err(err) => vi_prepare_fatal_error_to_rc(err),
    }
}

/// Shows the fatal display (16.0.0+).
///
/// Corresponds to `viManagerShowFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_show_fatal() -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.show_fatal() {
        Ok(()) => 0,
        Err(err) => vi_show_fatal_error_to_rc(err),
    }
}

/// Draws a fatal rectangle (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalRectangle()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_draw_fatal_rectangle(
    x: i32,
    y: i32,
    end_x: i32,
    end_y: i32,
    color: u16,
) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.draw_fatal_rectangle(x, y, end_x, end_y, color) {
        Ok(()) => 0,
        Err(err) => vi_draw_fatal_rectangle_error_to_rc(err),
    }
}

/// Draws fatal text using UTF-32 codepoints (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalText32()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_draw_fatal_text32(
    out_advance: *mut i32,
    x: i32,
    y: i32,
    utf32_codepoints: *const u32,
    num_codepoints: usize,
    scale_x: f32,
    scale_y: f32,
    font_type: u32,
    bg_color: u32,
    fg_color: u32,
    initial_advance: i32,
) -> u32 {
    if utf32_codepoints.is_null() || out_advance.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    let codepoints_slice = unsafe { core::slice::from_raw_parts(utf32_codepoints, num_codepoints) };

    match service.draw_fatal_text32(
        x,
        y,
        codepoints_slice,
        scale_x,
        scale_y,
        font_type,
        bg_color,
        fg_color,
        initial_advance,
    ) {
        Ok(advance) => {
            unsafe { *out_advance = advance };
            0
        }
        Err(err) => vi_draw_fatal_text32_error_to_rc(err),
    }
}

/// Sets VI FFI session buffers from the active service.
fn set_vi_ffi_sessions() {
    let Some(service_ref) = vi_manager::get_service() else {
        return;
    };

    // IApplicationDisplayService
    let app_display = Service {
        session: service_ref.application_display_session(),
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };
    // SAFETY: Called only during first initialization.
    unsafe {
        VI_FFI_APPLICATION_DISPLAY
            .get()
            .cast::<Service>()
            .write(app_display)
    };

    // IHOSBinderDriverRelay
    let binder_relay = Service {
        session: service_ref.binder_relay().session,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };
    // SAFETY: Called only during first initialization.
    unsafe {
        VI_FFI_BINDER_RELAY
            .get()
            .cast::<Service>()
            .write(binder_relay)
    };

    // ISystemDisplayService (optional)
    if let Some(session) = service_ref.system_display_session() {
        let sys_display = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_SYSTEM_DISPLAY
                .get()
                .cast::<Service>()
                .write(sys_display)
        };
    }

    // IManagerDisplayService (optional)
    if let Some(session) = service_ref.manager_display_session() {
        let mgr_display = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_MANAGER_DISPLAY
                .get()
                .cast::<Service>()
                .write(mgr_display)
        };
    }

    // IHOSBinderDriverIndirect (optional)
    if let Some(session) = service_ref.binder_indirect_session() {
        let binder_indirect = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_BINDER_INDIRECT
                .get()
                .cast::<Service>()
                .write(binder_indirect)
        };
    }
}

/// Clears VI FFI session buffers.
fn clear_vi_ffi_sessions() {
    // SAFETY: Called only during exit, after service is closed.
    unsafe {
        VI_FFI_APPLICATION_DISPLAY
            .get()
            .write(MaybeUninit::zeroed());
        VI_FFI_BINDER_RELAY.get().write(MaybeUninit::zeroed());
        VI_FFI_SYSTEM_DISPLAY.get().write(MaybeUninit::zeroed());
        VI_FFI_MANAGER_DISPLAY.get().write(MaybeUninit::zeroed());
        VI_FFI_BINDER_INDIRECT.get().write(MaybeUninit::zeroed());
    }
}

/// Parses native window data to extract binder object ID.
fn parse_native_window_binder_id(
    native_window: &[u8; nx_service_vi::NATIVE_WINDOW_SIZE],
) -> Option<u32> {
    // Parcel header structure
    #[repr(C)]
    struct ParcelHeader {
        payload_off: u32,
        payload_size: u32,
        objects_off: u32,
        objects_size: u32,
    }

    if native_window.len() < core::mem::size_of::<ParcelHeader>() {
        return None;
    }

    let header =
        unsafe { core::ptr::read_unaligned(native_window.as_ptr().cast::<ParcelHeader>()) };

    let payload_off = header.payload_off as usize;
    let payload_size = header.payload_size as usize;

    if payload_off > native_window.len() {
        return None;
    }
    if payload_off + payload_size > native_window.len() {
        return None;
    }
    if payload_size < 3 * 4 {
        return None;
    }

    // Binder object ID is at offset 2 (third u32) in the payload
    let binder_id_offset = payload_off + 2 * 4;
    if binder_id_offset + 4 > native_window.len() {
        return None;
    }

    let binder_id = unsafe {
        core::ptr::read_unaligned(native_window.as_ptr().add(binder_id_offset).cast::<u32>())
    };

    Some(binder_id)
}

fn vi_connect_error_to_rc(err: vi_manager::ConnectError) -> u32 {
    match err {
        vi_manager::ConnectError::Connect(e) => vi_service_connect_error_to_rc(e),
    }
}

fn vi_service_connect_error_to_rc(err: nx_service_vi::ConnectError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::ConnectError::GetService(e) => match e {
            nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        nx_service_vi::ConnectError::NoServiceAvailable => GENERIC_ERROR,
        nx_service_vi::ConnectError::GetDisplayService(e) => vi_get_display_service_error_to_rc(e),
        nx_service_vi::ConnectError::GetSubService(e) => vi_get_sub_service_error_to_rc(e),
    }
}

fn vi_get_display_service_error_to_rc(err: nx_service_vi::GetDisplayServiceError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetDisplayServiceError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayServiceError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetDisplayServiceError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_get_sub_service_error_to_rc(err: nx_service_vi::GetSubServiceError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetSubServiceError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetSubServiceError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetSubServiceError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_open_display_error_to_rc(err: nx_service_vi::OpenDisplayError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::OpenDisplayError::SendRequest(e) => e.to_rc(),
        nx_service_vi::OpenDisplayError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_close_display_error_to_rc(err: nx_service_vi::CloseDisplayError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::CloseDisplayError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CloseDisplayError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_display_resolution_error_to_rc(err: nx_service_vi::GetDisplayResolutionError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetDisplayResolutionError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayResolutionError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_display_logical_resolution_error_to_rc(
    err: nx_service_vi::GetDisplayLogicalResolutionWrapperError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetDisplayLogicalResolutionWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetDisplayLogicalResolutionWrapperError::Cmif(e) => match e {
            nx_service_vi::GetDisplayLogicalResolutionError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetDisplayLogicalResolutionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_display_magnification_error_to_rc(
    err: nx_service_vi::SetDisplayMagnificationWrapperError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetDisplayMagnificationWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayMagnificationWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayMagnificationError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayMagnificationError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_display_vsync_event_error_to_rc(err: nx_service_vi::GetDisplayVsyncEventError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetDisplayVsyncEventError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayVsyncEventError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetDisplayVsyncEventError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_set_display_power_state_error_to_rc(
    err: nx_service_vi::SetDisplayPowerStateWrapperError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetDisplayPowerStateWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayPowerStateWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayPowerStateError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayPowerStateError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_display_alpha_error_to_rc(err: nx_service_vi::SetDisplayAlphaWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetDisplayAlphaWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayAlphaWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayAlphaError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayAlphaError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_z_order_count_min_error_to_rc(err: nx_service_vi::GetZOrderCountMinError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetZOrderCountMinError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetZOrderCountMinError::Cmif(e) => match e {
            nx_service_vi::GetZOrderCountError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetZOrderCountError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_z_order_count_max_error_to_rc(err: nx_service_vi::GetZOrderCountMaxError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetZOrderCountMaxError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetZOrderCountMaxError::Cmif(e) => match e {
            nx_service_vi::GetZOrderCountError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetZOrderCountError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_create_stray_layer_error_to_rc(err: nx_service_vi::CreateStrayLayerError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::CreateStrayLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CreateStrayLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_create_managed_layer_error_to_rc(err: nx_service_vi::CreateManagedLayerWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::CreateManagedLayerWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::CreateManagedLayerWrapperError::Cmif(e) => match e {
            nx_service_vi::CreateManagedLayerError::SendRequest(e) => e.to_rc(),
            nx_service_vi::CreateManagedLayerError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_destroy_managed_layer_error_to_rc(
    err: nx_service_vi::DestroyManagedLayerWrapperError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::DestroyManagedLayerWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::DestroyManagedLayerWrapperError::Cmif(e) => match e {
            nx_service_vi::DestroyManagedLayerError::SendRequest(e) => e.to_rc(),
            nx_service_vi::DestroyManagedLayerError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_close_layer_error_to_rc(err: nx_service_vi::CloseLayerError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::CloseLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CloseLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_destroy_stray_layer_error_to_rc(err: nx_service_vi::DestroyStrayLayerError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::DestroyStrayLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::DestroyStrayLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_set_layer_size_error_to_rc(err: nx_service_vi::SetLayerSizeWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetLayerSizeWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerSizeWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerSizeError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerSizeError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_z_error_to_rc(err: nx_service_vi::SetLayerZWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetLayerZWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerZWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerZError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerZError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_position_error_to_rc(err: nx_service_vi::SetLayerPositionWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetLayerPositionWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerPositionWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerPositionError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerPositionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_scaling_mode_error_to_rc(err: nx_service_vi::SetLayerScalingModeError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetLayerScalingModeError::SendRequest(e) => e.to_rc(),
        nx_service_vi::SetLayerScalingModeError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_indirect_layer_image_map_error_to_rc(
    err: nx_service_vi::GetIndirectLayerImageMapError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetIndirectLayerImageMapError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetIndirectLayerImageMapError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_indirect_layer_image_required_memory_info_error_to_rc(
    err: nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_set_content_visibility_error_to_rc(
    err: nx_service_vi::SetContentVisibilityWrapperError,
) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::SetContentVisibilityWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetContentVisibilityWrapperError::Cmif(e) => match e {
            nx_service_vi::SetContentVisibilityError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetContentVisibilityError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_prepare_fatal_error_to_rc(err: nx_service_vi::PrepareFatalWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::PrepareFatalWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::PrepareFatalWrapperError::Cmif(e) => match e {
            nx_service_vi::PrepareFatalError::SendRequest(e) => e.to_rc(),
            nx_service_vi::PrepareFatalError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_show_fatal_error_to_rc(err: nx_service_vi::ShowFatalWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::ShowFatalWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::ShowFatalWrapperError::Cmif(e) => match e {
            nx_service_vi::ShowFatalError::SendRequest(e) => e.to_rc(),
            nx_service_vi::ShowFatalError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_draw_fatal_rectangle_error_to_rc(err: nx_service_vi::DrawFatalRectangleWrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::DrawFatalRectangleWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::DrawFatalRectangleWrapperError::Cmif(e) => match e {
            nx_service_vi::DrawFatalRectangleError::SendRequest(e) => e.to_rc(),
            nx_service_vi::DrawFatalRectangleError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_draw_fatal_text32_error_to_rc(err: nx_service_vi::DrawFatalText32WrapperError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_service_vi::DrawFatalText32WrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::DrawFatalText32WrapperError::Cmif(e) => match e {
            nx_service_vi::DrawFatalText32Error::SendRequest(e) => e.to_rc(),
            nx_service_vi::DrawFatalText32Error::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}
