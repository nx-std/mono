//! Visual Interface (VI) service FFI

use core::{ffi::c_void, mem::MaybeUninit};

use nx_rt_core::error::ToResultCode as _;
use nx_sf::{error::ToResultCode, ffi::Service};

use crate::{
    ffi::common::{GENERIC_ERROR, LibnxError, SyncUnsafeCell, libnx_error},
    services::{applet, vi},
};

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
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_initialize(service_type: i32) -> u32 {
    let vi_service_type = match nx_service_vi::types::ViServiceType::from_raw(service_type) {
        Some(st) => st,
        None => return GENERIC_ERROR,
    };

    // Check if this is the first initialization
    let was_initialized = vi::is_initialized();

    match vi::init(vi_service_type) {
        Ok(()) => {
            // Only update FFI session buffers on first actual initialization
            if !was_initialized {
                set_vi_ffi_sessions();
            }
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Exits the VI service.
///
/// Corresponds to `viExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_exit() {
    let was_initialized = vi::is_initialized();
    vi::exit();
    let still_initialized = vi::is_initialized();

    // Only clear FFI session buffers if the service was actually closed
    if was_initialized && !still_initialized {
        clear_vi_ffi_sessions();
    }
}

/// Gets the IApplicationDisplayService session pointer.
///
/// Corresponds to `viGetSession_IApplicationDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_session_application_display() -> *mut Service {
    VI_FFI_APPLICATION_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverRelay session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverRelay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_session_binder_relay() -> *mut Service {
    VI_FFI_BINDER_RELAY.get().cast::<Service>()
}

/// Gets the ISystemDisplayService session pointer.
///
/// Corresponds to `viGetSession_ISystemDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_session_system_display() -> *mut Service {
    VI_FFI_SYSTEM_DISPLAY.get().cast::<Service>()
}

/// Gets the IManagerDisplayService session pointer.
///
/// Corresponds to `viGetSession_IManagerDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_session_manager_display() -> *mut Service {
    VI_FFI_MANAGER_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverIndirect session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverIndirect()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_session_binder_indirect() -> *mut Service {
    VI_FFI_BINDER_INDIRECT.get().cast::<Service>()
}

/// Opens a display by name.
///
/// Corresponds to `viOpenDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_open_display(
    name: *const core::ffi::c_char,
    display: *mut ViDisplay,
) -> u32 {
    if name.is_null() || display.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Closes a display.
///
/// Corresponds to `viCloseDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_close_display(display: *mut ViDisplay) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &mut *display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.close_display(display_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(display, 0, 1) };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Gets display resolution.
///
/// Corresponds to `viGetDisplayResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_display_resolution(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Gets display logical resolution.
///
/// Corresponds to `viGetDisplayLogicalResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_display_logical_resolution(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Sets display magnification (3.0.0+).
///
/// Corresponds to `viSetDisplayMagnification()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_display_magnification(
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

    // libnx `viSetDisplayMagnification` requires HOS ≥ 3.0.0.
    if crate::env::hos_version::get() < crate::env::hos_version::HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_magnification(display_id, x, y, width, height) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Gets display vsync event handle.
///
/// Corresponds to `viGetDisplayVsyncEvent()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_display_vsync_event(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_vsync_event(display_id) {
        Ok(handle) => {
            unsafe { *event_handle_out = handle };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Sets display power state.
///
/// Corresponds to `viSetDisplayPowerState()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_display_power_state(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Sets display alpha.
///
/// Corresponds to `viSetDisplayAlpha()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_display_alpha(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_alpha(display_id, alpha) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Gets Z-order count minimum.
///
/// Corresponds to `viGetZOrderCountMin()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_z_order_count_min(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Gets Z-order count maximum.
///
/// Corresponds to `viGetZOrderCountMax()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_z_order_count_max(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Creates a layer (managed via applet ARUID if possible, otherwise stray).
///
/// Mirrors libnx `viCreateLayer()` (`services/vi.c:viCreateLayer`):
/// 1. Read the `__nx_vi_layer_id` weak override.
/// 2. If unset and the applet ARUID is non-zero, ask the applet to allocate
///    a managed layer (`appletCreateManagedDisplayLayer`).
/// 3. If a layer id is known, dispatch `_viOpenLayer` (cmd 2020).
/// 4. Otherwise, dispatch `_viCreateStrayLayer` using `__nx_vi_stray_layer_flags`
///    to the right sub-service for the current service-type + HOS version:
///    Application -> IApplicationDisplayService (cmd 2030),
///    System/Manager < 7.0.0 -> ISystemDisplayService (cmd 2312),
///    Manager >= 7.0.0 -> IManagerDisplayService (cmd 2012).
/// 5. Parse the returned native_window parcel for the IGBP binder id; on a
///    malformed parcel, close the layer and return `LibnxError_BadInput`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_create_layer(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    // Zero-initialize the layer struct.
    unsafe { core::ptr::write_bytes(layer, 0, 1) };
    let layer_ref = unsafe { &mut *layer };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);
    let display_name = nx_service_vi::DisplayName::from_array(display_ref.display_name);
    let aruid = applet::get_applet_resource_user_id()
        .map(|a| a.to_raw())
        .unwrap_or(0);

    // libnx: `layer->layer_id = __nx_vi_layer_id;` (weak override default 0).
    let mut layer_id = vi::get_layer_id_override();

    // libnx: when no override and we have an applet ARUID, attempt to allocate
    // a managed display layer; propagate any error verbatim.
    if layer_id == 0 && aruid != 0 {
        let Some(self_controller) = applet::get_self_controller() else {
            return GENERIC_ERROR;
        };
        match self_controller.create_managed_display_layer() {
            Ok(id) => layer_id = id,
            Err(_) => return GENERIC_ERROR,
        }
    }

    let native_window: [u8; nx_service_vi::NATIVE_WINDOW_SIZE];
    let is_stray = layer_id == 0;

    if !is_stray {
        // Managed-layer path: open the pre-allocated layer (cmd 2020).
        match service.open_layer(&display_name, nx_service_vi::LayerId::new(layer_id), aruid) {
            Ok(output) => {
                native_window = output.native_window;
            }
            Err(err) => return err.to_rc(),
        }
    } else {
        // Stray-layer path: dispatch to the right sub-service.
        let flags = vi::get_stray_layer_flags_override();
        let rc = create_stray_layer_for_service(&service, flags, display_id);
        match rc {
            Ok((id, nw)) => {
                layer_id = id;
                native_window = nw;
            }
            Err(code) => return code,
        }
    }

    // Parse the returned parcel to locate the IGBP binder object id.
    let binder_id = match parse_native_window_binder_id(&native_window) {
        Some(id) => id,
        None => {
            // libnx: close the layer and return BadInput.
            let close_layer = ViLayer {
                layer_id,
                igbp_binder_obj_id: 0,
                flags: if is_stray { 0x03 } else { 0x01 },
            };
            let _ = close_layer_internal(&service, &close_layer);
            return libnx_error(LibnxError::BadInput);
        }
    };

    layer_ref.layer_id = layer_id;
    layer_ref.igbp_binder_obj_id = binder_id;
    // initialized (0x01) | stray_layer (0x02) iff this is a stray layer.
    layer_ref.flags = if is_stray { 0x03 } else { 0x01 };
    0
}

/// Dispatches `_viCreateStrayLayer` to the correct sub-service for the
/// current VI service-type and HOS version.
fn create_stray_layer_for_service(
    service: &nx_service_vi::ViService,
    flags: nx_service_vi::ViLayerFlags,
    display_id: nx_service_vi::DisplayId,
) -> Result<(u64, [u8; nx_service_vi::NATIVE_WINDOW_SIZE]), u32> {
    use nx_service_vi::types::ViServiceType;

    match service.service_type() {
        ViServiceType::Default | ViServiceType::Application => service
            .create_stray_layer(flags, display_id)
            .map(|out| (out.layer_id.to_raw(), out.native_window))
            .map_err(ToResultCode::to_rc),
        ViServiceType::System | ViServiceType::Manager => {
            // libnx: pre-7.0.0 goes to ISystemDisplayService (cmd 2312),
            // 7.0.0+ goes to IManagerDisplayService (cmd 2012).
            let use_system =
                crate::env::hos_version::get() < crate::env::hos_version::HosVersion::new(7, 0, 0);
            let result = if use_system {
                service.create_stray_layer_system(flags, display_id)
            } else {
                service.create_stray_layer_manager(flags, display_id)
            };
            result
                .map(|out| (out.layer_id.to_raw(), out.native_window))
                .map_err(ToResultCode::to_rc)
        }
    }
}

/// Internal `viCloseLayer` for use in error-recovery paths (e.g. bad parcel).
fn close_layer_internal(service: &nx_service_vi::ViService, layer: &ViLayer) -> Result<(), u32> {
    let layer_id = nx_service_vi::LayerId::new(layer.layer_id);
    if layer.is_stray_layer() {
        service
            .destroy_stray_layer(layer_id)
            .map_err(ToResultCode::to_rc)
    } else {
        service.close_layer(layer_id).map_err(ToResultCode::to_rc)
    }
}

/// Creates a managed layer.
///
/// Corresponds to `viCreateManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_create_managed_layer(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Destroys a managed layer.
///
/// Corresponds to `viDestroyManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_destroy_managed_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.destroy_managed_layer(layer_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(layer, 0, 1) };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Closes a layer.
///
/// Corresponds to `viCloseLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_close_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    let rc = if layer_ref.is_stray_layer() {
        match service.destroy_stray_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => err.to_rc(),
        }
    } else {
        match service.close_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => err.to_rc(),
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_layer_size(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_size(layer_id, width, height) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Sets layer Z-order.
///
/// Corresponds to `viSetLayerZ()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_layer_z(layer: *const ViLayer, z: i32) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_z(layer_id, z) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Sets layer position.
///
/// Corresponds to `viSetLayerPosition()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_layer_position(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_position(layer_id, x, y) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Sets layer scaling mode.
///
/// Corresponds to `viSetLayerScalingMode()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_layer_scaling_mode(
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

    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Gets indirect layer image map.
///
/// Corresponds to `viGetIndirectLayerImageMap()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_indirect_layer_image_map(
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

    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    // Get ARUID from applet manager
    let aruid = applet::get_applet_resource_user_id()
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
        Err(err) => err.to_rc(),
    }
}

/// Gets indirect layer image required memory info.
///
/// Corresponds to `viGetIndirectLayerImageRequiredMemoryInfo()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_get_indirect_layer_image_required_memory_info(
    width: i32,
    height: i32,
    out_size: *mut u64,
    out_alignment: *mut u64,
) -> u32 {
    let Some(service) = vi::get_service() else {
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
        Err(err) => err.to_rc(),
    }
}

/// Sets content visibility.
///
/// Corresponds to `viSetContentVisibility()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_set_content_visibility(visible: bool) -> u32 {
    let Some(service) = vi::get_service() else {
        return GENERIC_ERROR;
    };

    match service.set_content_visibility(visible) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Returns `Some(LibnxError::IncompatSysVer)` if the active HOS version is
/// older than 16.0.0, which is when libnx made the fatal-display commands
/// available. The caller propagates the resulting result code.
#[inline]
fn require_fatal_display_supported() -> Option<u32> {
    if crate::env::hos_version::get() < crate::env::hos_version::HosVersion::new(16, 0, 0) {
        Some(libnx_error(LibnxError::IncompatSysVer))
    } else {
        None
    }
}

/// Prepares the fatal display (16.0.0+).
///
/// Corresponds to `viManagerPrepareFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_manager_prepare_fatal() -> u32 {
    if let Some(rc) = require_fatal_display_supported() {
        return rc;
    }
    let Some(service) = vi::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.prepare_fatal() {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Shows the fatal display (16.0.0+).
///
/// Corresponds to `viManagerShowFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_manager_show_fatal() -> u32 {
    if let Some(rc) = require_fatal_display_supported() {
        return rc;
    }
    let Some(service) = vi::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.show_fatal() {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Draws a fatal rectangle (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalRectangle()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_manager_draw_fatal_rectangle(
    x: i32,
    y: i32,
    end_x: i32,
    end_y: i32,
    color: u16,
) -> u32 {
    if let Some(rc) = require_fatal_display_supported() {
        return rc;
    }
    let Some(service) = vi::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.draw_fatal_rectangle(x, y, end_x, end_y, color) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Draws fatal text using UTF-32 codepoints (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalText32()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_vi_manager_draw_fatal_text32(
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

    if let Some(rc) = require_fatal_display_supported() {
        return rc;
    }
    let Some(service) = vi::get_service() else {
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
///
/// The FFI snapshots are **non-owning views** of the inner [`ViService`]'s
/// sub-service handles: `own_handle = 0`. The inner [`ViService`] retains
/// exclusive ownership and is responsible for closing the underlying kernel
/// handles on `vi::exit`. C consumers must treat these `Service*` as borrowed
/// for the lifetime of the active VI session.
fn set_vi_ffi_sessions() {
    let Some(service_ref) = vi::get_service() else {
        return;
    };

    // IApplicationDisplayService
    let app_display = Service {
        session: service_ref.application_display_session(),
        own_handle: 0,
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
    let binder_relay_session = service_ref.binder_relay();
    let binder_relay = Service {
        session: binder_relay_session.handle(),
        own_handle: 0,
        object_id: 0,
        pointer_buffer_size: binder_relay_session.pointer_buffer_size(),
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
            own_handle: 0,
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
            own_handle: 0,
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
            own_handle: 0,
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
///
/// Mirrors libnx `viCreateLayer` parcel parsing: read the parcel header at the
/// start of the buffer, validate payload bounds, then read the third `u32` of
/// the payload (the IGBP binder object ID).
fn parse_native_window_binder_id(
    native_window: &[u8; nx_service_vi::NATIVE_WINDOW_SIZE],
) -> Option<u32> {
    use nx_service_vi::ParcelHeader;

    if native_window.len() < ParcelHeader::SIZE {
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

fn vi_draw_fatal_text32_error_to_rc(err: nx_service_vi::DrawFatalText32WrapperError) -> u32 {
    match err {
        nx_service_vi::DrawFatalText32WrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::DrawFatalText32WrapperError::Cmif(e) => match e {
            nx_service_vi::DrawFatalText32Error::SendRequest(e) => e.to_rc(),
            nx_service_vi::DrawFatalText32Error::ParseResponse(e) => e.to_rc(),
        },
    }
}
