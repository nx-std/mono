//! HID protocol constants and types.

use nx_sf::ServiceName;

/// Service name for HID.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hid");

/// HID service command IDs
pub mod cmds {
    // IHidServer command: CreateAppletResource
    pub const INITIALIZE_APPLET_RESOURCE: u32 = 0;

    // Touch screen
    pub const ACTIVATE_TOUCH_SCREEN: u32 = 11;

    // Mouse
    pub const ACTIVATE_MOUSE: u32 = 21;

    // Keyboard
    pub const ACTIVATE_KEYBOARD: u32 = 31;

    // Gesture
    pub const ACTIVATE_GESTURE: u32 = 91;

    // Npad
    pub const SET_SUPPORTED_NPAD_STYLE_SET: u32 = 100;
    pub const SET_SUPPORTED_NPAD_ID_TYPE: u32 = 102;
    pub const ACTIVATE_NPAD_WITH_REVISION: u32 = 109;
}

/// IAppletResource command IDs
pub mod applet_resource_cmds {
    pub const GET_SHARED_MEMORY_HANDLE: u32 = 0;
}
