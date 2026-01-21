//! VI service protocol constants and command IDs.

use nx_sf::ServiceName;

/// Service name for vi:u (Application).
pub const SERVICE_NAME_APPLICATION: ServiceName = ServiceName::new_truncate("vi:u");

/// Service name for vi:s (System).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("vi:s");

/// Service name for vi:m (Manager).
pub const SERVICE_NAME_MANAGER: ServiceName = ServiceName::new_truncate("vi:m");

/// Root service command IDs.
///
/// The command ID for GetDisplayService equals the service type value:
/// - 0 = Application (vi:u)
/// - 1 = System (vi:s)
/// - 2 = Manager (vi:m)
pub mod root_cmds {
    /// Get IApplicationDisplayService session (vi:u).
    pub const GET_DISPLAY_SERVICE_APPLICATION: u32 = 0;
    /// Get IApplicationDisplayService session (vi:s).
    pub const GET_DISPLAY_SERVICE_SYSTEM: u32 = 1;
    /// Get IApplicationDisplayService session (vi:m).
    pub const GET_DISPLAY_SERVICE_MANAGER: u32 = 2;

    /// Prepare fatal display (16.0.0+).
    pub const PREPARE_FATAL: u32 = 100;
    /// Show fatal display (16.0.0+).
    pub const SHOW_FATAL: u32 = 101;
    /// Draw fatal rectangle (16.0.0+).
    pub const DRAW_FATAL_RECTANGLE: u32 = 102;
    /// Draw fatal text (UTF-32) (16.0.0+).
    pub const DRAW_FATAL_TEXT32: u32 = 103;
}

/// IApplicationDisplayService command IDs.
pub mod application_cmds {
    /// Get IHOSBinderDriverRelay session.
    pub const GET_RELAY_SERVICE: u32 = 100;
    /// Get ISystemDisplayService session (System/Manager).
    pub const GET_SYSTEM_DISPLAY_SERVICE: u32 = 101;
    /// Get IManagerDisplayService session (Manager).
    pub const GET_MANAGER_DISPLAY_SERVICE: u32 = 102;
    /// Get IHOSBinderDriverIndirect session (System/Manager, 2.0.0+).
    pub const GET_INDIRECT_DISPLAY_TRANSACTION_SERVICE: u32 = 103;

    /// Open a display by name.
    pub const OPEN_DISPLAY: u32 = 1010;
    /// Close a display.
    pub const CLOSE_DISPLAY: u32 = 1020;

    /// Get display resolution.
    pub const GET_DISPLAY_RESOLUTION: u32 = 1102;

    /// Open a layer.
    pub const OPEN_LAYER: u32 = 2020;
    /// Close a layer.
    pub const CLOSE_LAYER: u32 = 2021;
    /// Create a stray layer.
    pub const CREATE_STRAY_LAYER: u32 = 2030;
    /// Destroy a stray layer.
    pub const DESTROY_STRAY_LAYER: u32 = 2031;

    /// Set layer scaling mode.
    pub const SET_LAYER_SCALING_MODE: u32 = 2101;

    /// Get indirect layer image map.
    pub const GET_INDIRECT_LAYER_IMAGE_MAP: u32 = 2450;
    /// Get indirect layer image required memory info.
    pub const GET_INDIRECT_LAYER_IMAGE_REQUIRED_MEMORY_INFO: u32 = 2460;

    /// Get display vsync event.
    pub const GET_DISPLAY_VSYNC_EVENT: u32 = 5202;
}

/// ISystemDisplayService command IDs.
pub mod system_cmds {
    /// Get Z-order count minimum.
    pub const GET_Z_ORDER_COUNT_MIN: u32 = 1200;
    /// Get Z-order count maximum.
    pub const GET_Z_ORDER_COUNT_MAX: u32 = 1202;
    /// Get display logical resolution.
    pub const GET_DISPLAY_LOGICAL_RESOLUTION: u32 = 1203;
    /// Set display magnification (3.0.0+).
    pub const SET_DISPLAY_MAGNIFICATION: u32 = 1204;

    /// Set layer position.
    pub const SET_LAYER_POSITION: u32 = 2201;
    /// Set layer size.
    pub const SET_LAYER_SIZE: u32 = 2203;
    /// Set layer Z-order.
    pub const SET_LAYER_Z: u32 = 2205;
    /// Set layer visibility.
    pub const SET_LAYER_VISIBILITY: u32 = 2207;

    /// Create stray layer (System, before 7.0.0).
    pub const CREATE_STRAY_LAYER: u32 = 2312;
}

/// IManagerDisplayService command IDs.
pub mod manager_cmds {
    /// Create a managed layer.
    pub const CREATE_MANAGED_LAYER: u32 = 2010;
    /// Destroy a managed layer.
    pub const DESTROY_MANAGED_LAYER: u32 = 2011;
    /// Create stray layer (Manager, 7.0.0+).
    pub const CREATE_STRAY_LAYER: u32 = 2012;

    /// Set display alpha.
    pub const SET_DISPLAY_ALPHA: u32 = 4201;
    /// Set display layer stack.
    pub const SET_DISPLAY_LAYER_STACK: u32 = 4203;
    /// Set display power state.
    pub const SET_DISPLAY_POWER_STATE: u32 = 4205;

    /// Add layer to stack.
    pub const ADD_TO_LAYER_STACK: u32 = 6000;

    /// Set content visibility.
    pub const SET_CONTENT_VISIBILITY: u32 = 7000;
}

/// IHOSBinderDriverRelay command IDs.
pub mod binder_cmds {
    /// Transact parcel (before 3.0.0).
    pub const TRANSACT_PARCEL: u32 = 0;
    /// Adjust reference count.
    pub const ADJUST_REFCOUNT: u32 = 1;
    /// Get native handle.
    pub const GET_NATIVE_HANDLE: u32 = 2;
    /// Transact parcel with auto buffer mode (3.0.0+).
    pub const TRANSACT_PARCEL_AUTO: u32 = 3;
}
