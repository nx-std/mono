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
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;

pub mod binder;
mod cmif;
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
    root_service: Option<Service>,
    /// IApplicationDisplayService session.
    application_display: Service,
    /// IHOSBinderDriverRelay session.
    binder_relay: Service,
    /// ISystemDisplayService session (System/Manager only).
    system_display: Option<Service>,
    /// IManagerDisplayService session (Manager only).
    manager_display: Option<Service>,
    /// IHOSBinderDriverIndirect session (System/Manager, 2.0.0+).
    binder_indirect: Option<Service>,
}

// SAFETY: ViService is safe to send across threads because:
// - All Service instances are just session handles (u32)
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
        self.application_display.session
    }

    /// Returns the IHOSBinderDriverRelay session.
    #[inline]
    pub fn binder_relay(&self) -> &Service {
        &self.binder_relay
    }

    /// Returns the ISystemDisplayService session handle, if available.
    #[inline]
    pub fn system_display_session(&self) -> Option<SessionHandle> {
        self.system_display.as_ref().map(|s| s.session)
    }

    /// Returns the IManagerDisplayService session handle, if available.
    #[inline]
    pub fn manager_display_session(&self) -> Option<SessionHandle> {
        self.manager_display.as_ref().map(|s| s.session)
    }

    /// Returns the IHOSBinderDriverIndirect session handle, if available.
    #[inline]
    pub fn binder_indirect_session(&self) -> Option<SessionHandle> {
        self.binder_indirect.as_ref().map(|s| s.session)
    }

    /// Returns the root service session handle (Manager only, 16.0.0+).
    #[inline]
    pub fn root_service_session(&self) -> Option<SessionHandle> {
        self.root_service.as_ref().map(|s| s.session)
    }

    // =========================================================================
    // IApplicationDisplayService operations
    // =========================================================================

    /// Opens a display by name.
    pub fn open_display(&self, name: &DisplayName) -> Result<DisplayId, OpenDisplayError> {
        cmif::application::open_display(self.application_display.session, name)
    }

    /// Opens the default display.
    pub fn open_default_display(&self) -> Result<DisplayId, OpenDisplayError> {
        self.open_display(&DEFAULT_DISPLAY)
    }

    /// Closes a display.
    pub fn close_display(&self, display_id: DisplayId) -> Result<(), CloseDisplayError> {
        cmif::application::close_display(self.application_display.session, display_id)
    }

    /// Gets display resolution.
    pub fn get_display_resolution(
        &self,
        display_id: DisplayId,
    ) -> Result<DisplayResolution, GetDisplayResolutionError> {
        cmif::application::get_display_resolution(self.application_display.session, display_id)
    }

    /// Opens a layer.
    pub fn open_layer(
        &self,
        display_name: &DisplayName,
        layer_id: LayerId,
        aruid: u64,
    ) -> Result<OpenLayerOutput, OpenLayerError> {
        cmif::application::open_layer(
            self.application_display.session,
            display_name,
            layer_id,
            aruid,
        )
    }

    /// Closes a layer.
    pub fn close_layer(&self, layer_id: LayerId) -> Result<(), CloseLayerError> {
        cmif::application::close_layer(self.application_display.session, layer_id)
    }

    /// Creates a stray layer.
    pub fn create_stray_layer(
        &self,
        layer_flags: ViLayerFlags,
        display_id: DisplayId,
    ) -> Result<CreateStrayLayerOutput, CreateStrayLayerError> {
        cmif::application::create_stray_layer(
            self.application_display.session,
            layer_flags as u32,
            display_id,
        )
    }

    /// Destroys a stray layer.
    pub fn destroy_stray_layer(&self, layer_id: LayerId) -> Result<(), DestroyStrayLayerError> {
        cmif::application::destroy_stray_layer(self.application_display.session, layer_id)
    }

    /// Sets layer scaling mode.
    pub fn set_layer_scaling_mode(
        &self,
        layer_id: LayerId,
        scaling_mode: ViScalingMode,
    ) -> Result<(), SetLayerScalingModeError> {
        cmif::application::set_layer_scaling_mode(
            self.application_display.session,
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
            self.application_display.session,
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
            self.application_display.session,
            width as i64,
            height as i64,
        )
    }

    /// Gets display vsync event handle.
    pub fn get_display_vsync_event(
        &self,
        display_id: DisplayId,
    ) -> Result<nx_svc::raw::Handle, GetDisplayVsyncEventError> {
        cmif::application::get_display_vsync_event(self.application_display.session, display_id)
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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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
            .session;

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

    /// Consumes and closes the VI service session.
    pub fn close(self) {
        // Close sub-services in reverse order of acquisition
        if let Some(indirect) = self.binder_indirect {
            indirect.close();
        }
        if let Some(manager) = self.manager_display {
            manager.close();
        }
        if let Some(system) = self.system_display {
            system.close();
        }
        self.binder_relay.close();
        self.application_display.close();
        if let Some(root) = self.root_service {
            root.close();
        }
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
pub fn connect(sm: &SmService, service_type: ViServiceType) -> Result<ViService, ConnectError> {
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

    // Decide whether to keep root service (Manager 16.0.0+ only)
    // For now, we'll keep it for Manager and close it for others
    // TODO: Check HOS version for 16.0.0+ detection
    let keep_root = actual_type == ViServiceType::Manager;
    if keep_root {
        root_service_handle = Some(Service {
            session: root_handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        });
    } else {
        // Close root service handle
        let _ = nx_svc::ipc::close_handle(root_handle);
    }

    // Get IHOSBinderDriverRelay
    let binder_relay =
        cmif::application::get_relay_service(application_display.session).map_err(|e| {
            application_display.close();
            if let Some(root) = &root_service_handle {
                root.close();
            }
            ConnectError::GetSubService(e)
        })?;

    // Get ISystemDisplayService (System/Manager only)
    let system_display = if actual_type >= ViServiceType::System {
        match cmif::application::get_system_display_service(application_display.session) {
            Ok(s) => Some(s),
            Err(e) => {
                binder_relay.close();
                application_display.close();
                if let Some(root) = &root_service_handle {
                    root.close();
                }
                return Err(ConnectError::GetSubService(e));
            }
        }
    } else {
        None
    };

    // Get IManagerDisplayService (Manager only)
    let manager_display = if actual_type >= ViServiceType::Manager {
        match cmif::application::get_manager_display_service(application_display.session) {
            Ok(s) => Some(s),
            Err(e) => {
                if let Some(sys) = &system_display {
                    sys.close();
                }
                binder_relay.close();
                application_display.close();
                if let Some(root) = &root_service_handle {
                    root.close();
                }
                return Err(ConnectError::GetSubService(e));
            }
        }
    } else {
        None
    };

    // Get IHOSBinderDriverIndirect (System/Manager, 2.0.0+)
    // TODO: Check HOS version for 2.0.0+ detection
    let binder_indirect = if actual_type >= ViServiceType::System {
        cmif::application::get_indirect_display_transaction_service(application_display.session)
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
