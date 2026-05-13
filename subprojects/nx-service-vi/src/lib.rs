//! VI (Visual Interface) Service Implementation.
//!
//! This crate provides access to the Nintendo Switch's VI service, which handles:
//! - Display management (open/close, resolution, vsync, power state)
//! - Layer management (create/destroy, position, size, z-order, scaling)
//! - Binder protocol for IGraphicBufferProducer communication
//!
//! The VI service manages display and layer composition on Horizon OS.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

pub mod binder;
mod cmif;
pub mod igbp;
pub mod parcel;
mod proto;
pub mod types;

pub use self::{
    binder::{Binder, BinderError, GetNativeHandleError, InitSessionError, TransactError},
    cmif::{
        application::{
            CloseDisplayError, CloseLayerError, CreateStrayLayerError, CreateStrayLayerOutput,
            DestroyStrayLayerError, DisplayResolution, GetDisplayResolutionError,
            GetDisplayVsyncEventError, GetIndirectLayerImageMapError,
            GetIndirectLayerImageRequiredMemoryInfoError, GetSubServiceError,
            IndirectLayerImageInfo, IndirectLayerMemoryInfo, NATIVE_WINDOW_SIZE, OpenDisplayError,
            OpenLayerError, OpenLayerOutput, SetLayerScalingModeError,
        },
        manager::{
            AddToLayerStackError, CreateManagedLayerError, DestroyManagedLayerError,
            SetContentVisibilityError, SetDisplayAlphaError, SetDisplayLayerStackError,
            SetDisplayPowerStateError,
        },
        root::{
            DrawFatalRectangleError, DrawFatalText32Error, GetDisplayServiceError,
            PrepareFatalError, ShowFatalError,
        },
        system::{
            GetDisplayLogicalResolutionError, GetZOrderCountError, LogicalResolution,
            SetDisplayMagnificationError, SetLayerPositionError, SetLayerSizeError,
            SetLayerVisibilityError, SetLayerZError,
        },
    },
    parcel::{PARCEL_MAX_PAYLOAD, Parcel, ParcelHeader},
    proto::{SERVICE_NAME_APPLICATION, SERVICE_NAME_MANAGER, SERVICE_NAME_SYSTEM},
    types::{
        BinderObjectId, DEFAULT_DISPLAY, DisplayId, DisplayName, LayerId, ViColorRgba4444,
        ViColorRgba8888, ViLayerFlags, ViLayerStack, ViPowerState, ViScalingMode, ViServiceType,
    },
};

/// VI service session wrapper.
///
/// Provides access to display and layer operations.
pub struct ViService {
    /// The actual service type we connected to.
    service_type: ViServiceType,
    /// Root service session (Manager only, 16.0.0+).
    root_service: Option<Session>,
    /// IApplicationDisplayService session.
    application_display: Session,
    /// IHOSBinderDriverRelay session.
    binder_relay: Session,
    /// ISystemDisplayService session (System/Manager only).
    system_display: Option<Session>,
    /// IManagerDisplayService session (Manager only).
    manager_display: Option<Session>,
    /// IHOSBinderDriverIndirect session (System/Manager, 2.0.0+).
    binder_indirect: Option<Session>,
}

// SAFETY: ViService is safe to send across threads because:
// - All Session instances are just session handles (u32)
// - No mutable state that requires synchronization
unsafe impl Send for ViService {}

// SAFETY: ViService is safe to share across threads because:
// - All operations go through the kernel which handles synchronization
unsafe impl Sync for ViService {}

impl ViService {
    /// Returns the service type that was connected.
    #[inline]
    pub fn service_type(&self) -> ViServiceType {
        self.service_type
    }

    /// Returns whether this is a System or Manager service.
    #[inline]
    pub fn is_system_or_manager(&self) -> bool {
        matches!(
            self.service_type,
            ViServiceType::System | ViServiceType::Manager
        )
    }

    /// Returns whether this is a Manager service.
    #[inline]
    pub fn is_manager(&self) -> bool {
        self.service_type == ViServiceType::Manager
    }

    /// Returns the IApplicationDisplayService session handle.
    #[inline]
    pub fn application_display_session(&self) -> SessionHandle {
        self.application_display.handle()
    }

    /// Returns the IHOSBinderDriverRelay session.
    #[inline]
    pub fn binder_relay(&self) -> &Session {
        &self.binder_relay
    }

    /// Returns the ISystemDisplayService session handle, if available.
    #[inline]
    pub fn system_display_session(&self) -> Option<SessionHandle> {
        self.system_display.as_ref().map(|s| s.handle())
    }

    /// Returns the IManagerDisplayService session handle, if available.
    #[inline]
    pub fn manager_display_session(&self) -> Option<SessionHandle> {
        self.manager_display.as_ref().map(|s| s.handle())
    }

    /// Returns the IHOSBinderDriverIndirect session handle, if available.
    #[inline]
    pub fn binder_indirect_session(&self) -> Option<SessionHandle> {
        self.binder_indirect.as_ref().map(|s| s.handle())
    }

    /// Returns the root service session handle (Manager only, 16.0.0+).
    #[inline]
    pub fn root_service_session(&self) -> Option<SessionHandle> {
        self.root_service.as_ref().map(|s| s.handle())
    }

    // =========================================================================
    // IApplicationDisplayService operations
    // =========================================================================

    /// Opens a display by name.
    pub fn open_display(&self, name: &DisplayName) -> Result<DisplayId, OpenDisplayError> {
        cmif::application::open_display(self.application_display.handle(), name)
    }

    /// Opens the default display.
    pub fn open_default_display(&self) -> Result<DisplayId, OpenDisplayError> {
        self.open_display(&DEFAULT_DISPLAY)
    }

    /// Closes a display.
    pub fn close_display(&self, display_id: DisplayId) -> Result<(), CloseDisplayError> {
        cmif::application::close_display(self.application_display.handle(), display_id)
    }

    /// Gets display resolution.
    pub fn get_display_resolution(
        &self,
        display_id: DisplayId,
    ) -> Result<DisplayResolution, GetDisplayResolutionError> {
        cmif::application::get_display_resolution(self.application_display.handle(), display_id)
    }

    /// Opens a layer.
    pub fn open_layer(
        &self,
        display_name: &DisplayName,
        layer_id: LayerId,
        aruid: u64,
    ) -> Result<OpenLayerOutput, OpenLayerError> {
        cmif::application::open_layer(
            self.application_display.handle(),
            display_name,
            layer_id,
            aruid,
        )
    }

    /// Closes a layer.
    pub fn close_layer(&self, layer_id: LayerId) -> Result<(), CloseLayerError> {
        cmif::application::close_layer(self.application_display.handle(), layer_id)
    }

    /// Creates a stray layer on IApplicationDisplayService (cmd 2030).
    ///
    /// Mirrors libnx `_viCreateStrayLayer` for the Application service-type
    /// path. The runtime layer is responsible for selecting between this
    /// and [`create_stray_layer_system`](Self::create_stray_layer_system) /
    /// [`create_stray_layer_manager`](Self::create_stray_layer_manager)
    /// based on the active service type and HOS version.
    pub fn create_stray_layer(
        &self,
        layer_flags: ViLayerFlags,
        display_id: DisplayId,
    ) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
        cmif::application::create_stray_layer(
            self.application_display.handle(),
            layer_flags as u32,
            display_id,
        )
    }

    /// Creates a stray layer on ISystemDisplayService (cmd 2312, pre-7.0.0).
    ///
    /// Requires System or Manager service type. Returns
    /// `CreateStrayLayerWrapperError::NotAvailable` if the ISystemDisplayService
    /// session is not active.
    pub fn create_stray_layer_system(
        &self,
        layer_flags: ViLayerFlags,
        display_id: DisplayId,
    ) -> Result<CreateStrayLayerOutput, CreateStrayLayerWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(CreateStrayLayerWrapperError::NotAvailable)?
            .handle();

        cmif::system::create_stray_layer(session, layer_flags as u32, display_id)
            .map_err(CreateStrayLayerWrapperError::Cmif)
    }

    /// Creates a stray layer on IManagerDisplayService (cmd 2012, 7.0.0+).
    ///
    /// Requires Manager service type. Returns
    /// `CreateStrayLayerWrapperError::NotAvailable` if the IManagerDisplayService
    /// session is not active.
    pub fn create_stray_layer_manager(
        &self,
        layer_flags: ViLayerFlags,
        display_id: DisplayId,
    ) -> Result<CreateStrayLayerOutput, CreateStrayLayerWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(CreateStrayLayerWrapperError::NotAvailable)?
            .handle();

        cmif::manager::create_stray_layer(session, layer_flags as u32, display_id)
            .map_err(CreateStrayLayerWrapperError::Cmif)
    }

    /// Destroys a stray layer.
    pub fn destroy_stray_layer(&self, layer_id: LayerId) -> Result<(), DestroyStrayLayerError> {
        cmif::application::destroy_stray_layer(self.application_display.handle(), layer_id)
    }

    /// Sets layer scaling mode.
    pub fn set_layer_scaling_mode(
        &self,
        layer_id: LayerId,
        scaling_mode: ViScalingMode,
    ) -> Result<(), SetLayerScalingModeError> {
        cmif::application::set_layer_scaling_mode(
            self.application_display.handle(),
            scaling_mode,
            layer_id,
        )
    }

    /// Gets indirect layer image map.
    #[allow(clippy::too_many_arguments)]
    pub fn get_indirect_layer_image_map(
        &self,
        width: i32,
        height: i32,
        indirect_layer_consumer_handle: u64,
        aruid: u64,
        buffer: &mut [u8],
    ) -> Result<IndirectLayerImageInfo, GetIndirectLayerImageMapError> {
        cmif::application::get_indirect_layer_image_map(
            self.application_display.handle(),
            width as i64,
            height as i64,
            indirect_layer_consumer_handle,
            aruid,
            buffer,
        )
    }

    /// Gets indirect layer image required memory info.
    pub fn get_indirect_layer_image_required_memory_info(
        &self,
        width: i32,
        height: i32,
    ) -> Result<IndirectLayerMemoryInfo, GetIndirectLayerImageRequiredMemoryInfoError> {
        cmif::application::get_indirect_layer_image_required_memory_info(
            self.application_display.handle(),
            width as i64,
            height as i64,
        )
    }

    /// Gets display vsync event handle.
    pub fn get_display_vsync_event(
        &self,
        display_id: DisplayId,
    ) -> Result<nx_svc::raw::Handle, GetDisplayVsyncEventError> {
        cmif::application::get_display_vsync_event(self.application_display.handle(), display_id)
    }

    // =========================================================================
    // ISystemDisplayService operations (System/Manager only)
    // =========================================================================

    /// Gets Z-order count minimum.
    ///
    /// Requires System or Manager service type.
    pub fn get_z_order_count_min(
        &self,
        display_id: DisplayId,
    ) -> Result<i32, GetZOrderCountMinError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(GetZOrderCountMinError::NotAvailable)?
            .handle();

        cmif::system::get_z_order_count_min(session, display_id)
            .map(|z| z as i32)
            .map_err(GetZOrderCountMinError::Cmif)
    }

    /// Gets Z-order count maximum.
    ///
    /// Requires System or Manager service type.
    pub fn get_z_order_count_max(
        &self,
        display_id: DisplayId,
    ) -> Result<i32, GetZOrderCountMaxError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(GetZOrderCountMaxError::NotAvailable)?
            .handle();

        cmif::system::get_z_order_count_max(session, display_id)
            .map(|z| z as i32)
            .map_err(GetZOrderCountMaxError::Cmif)
    }

    /// Gets display logical resolution.
    ///
    /// Requires System or Manager service type.
    pub fn get_display_logical_resolution(
        &self,
        display_id: DisplayId,
    ) -> Result<LogicalResolution, GetDisplayLogicalResolutionWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(GetDisplayLogicalResolutionWrapperError::NotAvailable)?
            .handle();

        cmif::system::get_display_logical_resolution(session, display_id)
            .map_err(GetDisplayLogicalResolutionWrapperError::Cmif)
    }

    /// Sets display magnification (3.0.0+).
    ///
    /// Requires System or Manager service type.
    pub fn set_display_magnification(
        &self,
        display_id: DisplayId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), SetDisplayMagnificationWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(SetDisplayMagnificationWrapperError::NotAvailable)?
            .handle();

        cmif::system::set_display_magnification(session, display_id, x, y, width, height)
            .map_err(SetDisplayMagnificationWrapperError::Cmif)
    }

    /// Sets layer position.
    ///
    /// Requires System or Manager service type.
    pub fn set_layer_position(
        &self,
        layer_id: LayerId,
        x: f32,
        y: f32,
    ) -> Result<(), SetLayerPositionWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(SetLayerPositionWrapperError::NotAvailable)?
            .handle();

        cmif::system::set_layer_position(session, layer_id, x, y)
            .map_err(SetLayerPositionWrapperError::Cmif)
    }

    /// Sets layer size.
    ///
    /// Requires System or Manager service type.
    pub fn set_layer_size(
        &self,
        layer_id: LayerId,
        width: i32,
        height: i32,
    ) -> Result<(), SetLayerSizeWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(SetLayerSizeWrapperError::NotAvailable)?
            .handle();

        cmif::system::set_layer_size(session, layer_id, width as i64, height as i64)
            .map_err(SetLayerSizeWrapperError::Cmif)
    }

    /// Sets layer Z-order.
    ///
    /// Requires System or Manager service type.
    pub fn set_layer_z(&self, layer_id: LayerId, z: i32) -> Result<(), SetLayerZWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(SetLayerZWrapperError::NotAvailable)?
            .handle();

        cmif::system::set_layer_z(session, layer_id, z as i64).map_err(SetLayerZWrapperError::Cmif)
    }

    /// Sets layer visibility.
    ///
    /// Requires System or Manager service type.
    pub fn set_layer_visibility(
        &self,
        layer_id: LayerId,
        visible: bool,
    ) -> Result<(), SetLayerVisibilityWrapperError> {
        let session = self
            .system_display
            .as_ref()
            .ok_or(SetLayerVisibilityWrapperError::NotAvailable)?
            .handle();

        cmif::system::set_layer_visibility(session, layer_id, visible)
            .map_err(SetLayerVisibilityWrapperError::Cmif)
    }

    // =========================================================================
    // IManagerDisplayService operations (Manager only)
    // =========================================================================

    /// Creates a managed layer.
    ///
    /// Requires Manager service type.
    pub fn create_managed_layer(
        &self,
        layer_flags: ViLayerFlags,
        display_id: DisplayId,
        aruid: u64,
    ) -> Result<LayerId, CreateManagedLayerWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(CreateManagedLayerWrapperError::NotAvailable)?
            .handle();

        cmif::manager::create_managed_layer(session, layer_flags as u32, display_id, aruid)
            .map_err(CreateManagedLayerWrapperError::Cmif)
    }

    /// Destroys a managed layer.
    ///
    /// Requires Manager service type.
    pub fn destroy_managed_layer(
        &self,
        layer_id: LayerId,
    ) -> Result<(), DestroyManagedLayerWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(DestroyManagedLayerWrapperError::NotAvailable)?
            .handle();

        cmif::manager::destroy_managed_layer(session, layer_id)
            .map_err(DestroyManagedLayerWrapperError::Cmif)
    }

    /// Sets display alpha.
    ///
    /// Requires Manager service type.
    pub fn set_display_alpha(
        &self,
        display_id: DisplayId,
        alpha: f32,
    ) -> Result<(), SetDisplayAlphaWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(SetDisplayAlphaWrapperError::NotAvailable)?
            .handle();

        cmif::manager::set_display_alpha(session, display_id, alpha)
            .map_err(SetDisplayAlphaWrapperError::Cmif)
    }

    /// Sets display layer stack.
    ///
    /// Requires Manager service type.
    pub fn set_display_layer_stack(
        &self,
        display_id: DisplayId,
        layer_stack: ViLayerStack,
    ) -> Result<(), SetDisplayLayerStackWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(SetDisplayLayerStackWrapperError::NotAvailable)?
            .handle();

        cmif::manager::set_display_layer_stack(session, display_id, layer_stack)
            .map_err(SetDisplayLayerStackWrapperError::Cmif)
    }

    /// Sets display power state.
    ///
    /// Requires Manager service type.
    pub fn set_display_power_state(
        &self,
        display_id: DisplayId,
        power_state: ViPowerState,
    ) -> Result<(), SetDisplayPowerStateWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(SetDisplayPowerStateWrapperError::NotAvailable)?
            .handle();

        cmif::manager::set_display_power_state(session, display_id, power_state)
            .map_err(SetDisplayPowerStateWrapperError::Cmif)
    }

    /// Sets content visibility.
    ///
    /// Requires Manager service type.
    pub fn set_content_visibility(
        &self,
        visible: bool,
    ) -> Result<(), SetContentVisibilityWrapperError> {
        let session = self
            .manager_display
            .as_ref()
            .ok_or(SetContentVisibilityWrapperError::NotAvailable)?
            .handle();

        cmif::manager::set_content_visibility(session, visible)
            .map_err(SetContentVisibilityWrapperError::Cmif)
    }

    // =========================================================================
    // Fatal display operations (Manager only, 16.0.0+)
    // =========================================================================

    /// Prepares the fatal display (16.0.0+).
    ///
    /// Requires Manager service type and HOS 16.0.0+.
    pub fn prepare_fatal(&self) -> Result<(), PrepareFatalWrapperError> {
        let session = self
            .root_service
            .as_ref()
            .ok_or(PrepareFatalWrapperError::NotAvailable)?
            .handle();

        cmif::root::prepare_fatal(session).map_err(PrepareFatalWrapperError::Cmif)
    }

    /// Shows the fatal display (16.0.0+).
    ///
    /// Requires Manager service type and HOS 16.0.0+.
    pub fn show_fatal(&self) -> Result<(), ShowFatalWrapperError> {
        let session = self
            .root_service
            .as_ref()
            .ok_or(ShowFatalWrapperError::NotAvailable)?
            .handle();

        cmif::root::show_fatal(session).map_err(ShowFatalWrapperError::Cmif)
    }

    /// Draws a fatal rectangle (16.0.0+).
    ///
    /// Requires Manager service type and HOS 16.0.0+.
    pub fn draw_fatal_rectangle(
        &self,
        x: i32,
        y: i32,
        end_x: i32,
        end_y: i32,
        color: ViColorRgba4444,
    ) -> Result<(), DrawFatalRectangleWrapperError> {
        let session = self
            .root_service
            .as_ref()
            .ok_or(DrawFatalRectangleWrapperError::NotAvailable)?
            .handle();

        cmif::root::draw_fatal_rectangle(session, x, y, end_x, end_y, color)
            .map_err(DrawFatalRectangleWrapperError::Cmif)
    }

    /// Draws fatal text using UTF-32 codepoints (16.0.0+).
    ///
    /// Requires Manager service type and HOS 16.0.0+.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_fatal_text32(
        &self,
        x: i32,
        y: i32,
        utf32_codepoints: &[u32],
        scale_x: f32,
        scale_y: f32,
        font_type: u32,
        bg_color: ViColorRgba8888,
        fg_color: ViColorRgba8888,
        initial_advance: i32,
    ) -> Result<i32, DrawFatalText32WrapperError> {
        let session = self
            .root_service
            .as_ref()
            .ok_or(DrawFatalText32WrapperError::NotAvailable)?
            .handle();

        cmif::root::draw_fatal_text32(
            session,
            x,
            y,
            utf32_codepoints,
            scale_x,
            scale_y,
            font_type,
            bg_color,
            fg_color,
            initial_advance,
        )
        .map_err(DrawFatalText32WrapperError::Cmif)
    }
}

/// Connects to the VI service.
///
/// # Arguments
///
/// * `sm` - Service manager session
/// * `service_type` - The requested service type (Default, Application, System, or Manager)
///
/// # Service Type Resolution
///
/// When `ViServiceType::Default` is specified, the function tries services in order:
/// 1. vi:m (Manager)
/// 2. vi:s (System)
/// 3. vi:u (Application)
///
/// # Returns
///
/// A connected [`ViService`] instance on success.
///
/// Uses default [`ConnectOptions`], which assume "modern" HOS (≥16.0.0): keep
/// the root service for Manager, request the indirect binder for System+Manager.
/// Callers that need version-specific behavior (e.g. runtime crates that know
/// the active HOS version) should use [`connect_with_options`].
pub fn connect(sm: &SmService, service_type: ViServiceType) -> Result<ViService, ConnectError> {
    connect_with_options(sm, service_type, ConnectOptions::default())
}

/// Options that gate sub-service acquisition during [`connect_with_options`].
///
/// The service crate is intentionally unaware of `hosversion`; the caller
/// (typically the runtime crate) decides which sub-services to request based
/// on the active HOS version.
#[derive(Debug, Clone, Copy)]
pub struct ConnectOptions {
    /// Retain the `vi:m` root service after sub-service discovery.
    ///
    /// libnx keeps this open only on HOS ≥ 16.0.0 (used by the fatal-display
    /// commands). When `false`, the root handle is closed once sub-services
    /// are obtained.
    pub keep_root_service: bool,

    /// Request `IHOSBinderDriverIndirect` (cmd 103) on System/Manager.
    ///
    /// libnx only requests this on HOS ≥ 2.0.0.
    pub request_indirect_binder: bool,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            keep_root_service: true,
            request_indirect_binder: true,
        }
    }
}

/// Connects to the VI service with explicit options.
///
/// See [`connect`] for the default-options entry point and
/// [`ConnectOptions`] for what each flag controls.
pub fn connect_with_options(
    sm: &SmService,
    service_type: ViServiceType,
    options: ConnectOptions,
) -> Result<ViService, ConnectError> {
    let mut actual_type = service_type;
    let mut root_service_handle = None;

    // Try to connect to root service
    let root_handle =
        if service_type == ViServiceType::Default || service_type == ViServiceType::Manager {
            match sm.get_service_handle_cmif(SERVICE_NAME_MANAGER) {
                Ok(h) => {
                    actual_type = ViServiceType::Manager;
                    Some(h)
                }
                Err(_) if service_type == ViServiceType::Default => None,
                Err(e) => return Err(ConnectError::GetService(e)),
            }
        } else {
            None
        };

    // Try System if Manager failed or not requested
    let root_handle = if root_handle.is_none()
        && (service_type == ViServiceType::Default || service_type == ViServiceType::System)
    {
        match sm.get_service_handle_cmif(SERVICE_NAME_SYSTEM) {
            Ok(h) => {
                actual_type = ViServiceType::System;
                Some(h)
            }
            Err(_) if service_type == ViServiceType::Default => None,
            Err(e) => return Err(ConnectError::GetService(e)),
        }
    } else {
        root_handle
    };

    // Try Application if System failed or not requested
    let root_handle = if root_handle.is_none()
        && (service_type == ViServiceType::Default || service_type == ViServiceType::Application)
    {
        match sm.get_service_handle_cmif(SERVICE_NAME_APPLICATION) {
            Ok(h) => {
                actual_type = ViServiceType::Application;
                Some(h)
            }
            Err(e) => return Err(ConnectError::GetService(e)),
        }
    } else {
        root_handle
    };

    let root_handle = root_handle.ok_or(ConnectError::NoServiceAvailable)?;

    // Get IApplicationDisplayService
    // Command ID equals the service type value (0=Application, 1=System, 2=Manager)
    let application_display = cmif::root::get_display_service(root_handle, actual_type)
        .map_err(ConnectError::GetDisplayService)?;

    // Keep root service only for Manager when the caller signals we are on
    // HOS ≥ 16.0.0 (the version that introduced the fatal-display commands).
    let keep_root = actual_type == ViServiceType::Manager && options.keep_root_service;
    if keep_root {
        // libnx does not query the root service's pointer-buffer-size;
        // skip the kernel round-trip and adopt the handle as-is.
        root_service_handle = Some(Session::from_handle(root_handle, 0));
    } else {
        // Close root service handle
        let _ = nx_svc::ipc::close_handle(root_handle);
    }

    // Get IHOSBinderDriverRelay.
    //
    // On error, `application_display` and (if held) `root_service_handle`
    // drop here, which closes their kernel handles via the `Session` /
    // `Option<Session>` `Drop` impl. No manual cleanup needed.
    let binder_relay = cmif::application::get_relay_service(application_display.handle())
        .map_err(ConnectError::GetSubService)?;

    // Get ISystemDisplayService (System/Manager only). On error every
    // already-acquired sub-service drops and closes its handle.
    let system_display = if actual_type >= ViServiceType::System {
        Some(
            cmif::application::get_system_display_service(application_display.handle())
                .map_err(ConnectError::GetSubService)?,
        )
    } else {
        None
    };

    // Get IManagerDisplayService (Manager only). Same cleanup story as above.
    let manager_display = if actual_type >= ViServiceType::Manager {
        Some(
            cmif::application::get_manager_display_service(application_display.handle())
                .map_err(ConnectError::GetSubService)?,
        )
    } else {
        None
    };

    // IHOSBinderDriverIndirect is only available on HOS ≥ 2.0.0; the caller
    // gates this via `options.request_indirect_binder` (libnx skips this on
    // older firmware via `hosversionAtLeast(2,0,0)`).
    let binder_indirect = if actual_type >= ViServiceType::System && options.request_indirect_binder
    {
        cmif::application::get_indirect_display_transaction_service(application_display.handle())
            .ok()
    } else {
        None
    };

    Ok(ViService {
        service_type: actual_type,
        root_service: root_service_handle,
        application_display,
        binder_relay,
        system_display,
        manager_display,
        binder_indirect,
    })
}

// =========================================================================
// Wrapper error types for methods that check service availability
// =========================================================================

/// Error for operations requiring System service.
#[derive(Debug, thiserror::Error)]
pub enum GetZOrderCountMinError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] GetZOrderCountError),
}

/// Error for operations requiring System service.
#[derive(Debug, thiserror::Error)]
pub enum GetZOrderCountMaxError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] GetZOrderCountError),
}

/// Error for `create_stray_layer_system` / `create_stray_layer_manager`.
///
/// Returned when the wrapper requires a sub-service session that the active
/// service-type doesn't have (e.g. trying the Manager path with an Application
/// service-type), or when the underlying CMIF operation fails.
#[derive(Debug, thiserror::Error)]
pub enum CreateStrayLayerWrapperError {
    /// Required display sub-service (System or Manager) not available.
    #[error("required display sub-service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] CreateStrayLayerError),
}

/// Error for get_display_logical_resolution wrapper.
#[derive(Debug, thiserror::Error)]
pub enum GetDisplayLogicalResolutionWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] GetDisplayLogicalResolutionError),
}

/// Error for set_display_magnification wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayMagnificationWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetDisplayMagnificationError),
}

/// Error for set_layer_position wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetLayerPositionWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetLayerPositionError),
}

/// Error for set_layer_size wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetLayerSizeWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetLayerSizeError),
}

/// Error for set_layer_z wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetLayerZWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetLayerZError),
}

/// Error for set_layer_visibility wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetLayerVisibilityWrapperError {
    /// System display service not available.
    #[error("system display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetLayerVisibilityError),
}

/// Error for create_managed_layer wrapper.
#[derive(Debug, thiserror::Error)]
pub enum CreateManagedLayerWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] CreateManagedLayerError),
}

/// Error for destroy_managed_layer wrapper.
#[derive(Debug, thiserror::Error)]
pub enum DestroyManagedLayerWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] DestroyManagedLayerError),
}

/// Error for set_display_alpha wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayAlphaWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetDisplayAlphaError),
}

/// Error for set_display_layer_stack wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayLayerStackWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetDisplayLayerStackError),
}

/// Error for set_display_power_state wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetDisplayPowerStateWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetDisplayPowerStateError),
}

/// Error for set_content_visibility wrapper.
#[derive(Debug, thiserror::Error)]
pub enum SetContentVisibilityWrapperError {
    /// Manager display service not available.
    #[error("manager display service not available")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] SetContentVisibilityError),
}

/// Error for prepare_fatal wrapper.
#[derive(Debug, thiserror::Error)]
pub enum PrepareFatalWrapperError {
    /// Root service not available (requires Manager 16.0.0+).
    #[error("root service not available (requires Manager 16.0.0+)")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] PrepareFatalError),
}

/// Error for show_fatal wrapper.
#[derive(Debug, thiserror::Error)]
pub enum ShowFatalWrapperError {
    /// Root service not available (requires Manager 16.0.0+).
    #[error("root service not available (requires Manager 16.0.0+)")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] ShowFatalError),
}

/// Error for draw_fatal_rectangle wrapper.
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalRectangleWrapperError {
    /// Root service not available (requires Manager 16.0.0+).
    #[error("root service not available (requires Manager 16.0.0+)")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] DrawFatalRectangleError),
}

/// Error for draw_fatal_text32 wrapper.
#[derive(Debug, thiserror::Error)]
pub enum DrawFatalText32WrapperError {
    /// Root service not available (requires Manager 16.0.0+).
    #[error("root service not available (requires Manager 16.0.0+)")]
    NotAvailable,
    /// CMIF operation failed.
    #[error("CMIF operation failed")]
    Cmif(#[source] DrawFatalText32Error),
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// No VI service available.
    #[error("no VI service available")]
    NoServiceAvailable,
    /// Failed to get IApplicationDisplayService.
    #[error("failed to get IApplicationDisplayService")]
    GetDisplayService(#[source] cmif::root::GetDisplayServiceError),
    /// Failed to get sub-service.
    #[error("failed to get sub-service")]
    GetSubService(#[source] GetSubServiceError),
}
