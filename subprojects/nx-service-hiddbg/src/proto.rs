//! HID Debug service protocol constants.

use nx_sf::ServiceName;

/// Service name for hid:dbg.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hid:dbg");

// ---------------------------------------------------------------------------
// AutoPilot commands
// ---------------------------------------------------------------------------

pub const SET_DEBUG_PAD_AUTO_PILOT_STATE: u32 = 1;
pub const UNSET_DEBUG_PAD_AUTO_PILOT_STATE: u32 = 2;
pub const SET_TOUCH_SCREEN_AUTO_PILOT_STATE: u32 = 11;
pub const UNSET_TOUCH_SCREEN_AUTO_PILOT_STATE: u32 = 12;
pub const SET_MOUSE_AUTO_PILOT_STATE: u32 = 21;
pub const UNSET_MOUSE_AUTO_PILOT_STATE: u32 = 22;
pub const SET_KEYBOARD_AUTO_PILOT_STATE: u32 = 31;
pub const UNSET_KEYBOARD_AUTO_PILOT_STATE: u32 = 32;
pub const DEACTIVATE_HOME_BUTTON: u32 = 110;
pub const SET_SLEEP_BUTTON_AUTO_PILOT_STATE: u32 = 121;
pub const UNSET_SLEEP_BUTTON_AUTO_PILOT_STATE: u32 = 122;

// ---------------------------------------------------------------------------
// Controller color / serial flash commands
// ---------------------------------------------------------------------------

pub const UPDATE_CONTROLLER_COLOR: u32 = 221;
pub const UPDATE_DESIGN_INFO: u32 = 224;
pub const ACQUIRE_OPERATION_EVENT_HANDLE: u32 = 228;
pub const READ_SERIAL_FLASH: u32 = 229;
pub const WRITE_SERIAL_FLASH: u32 = 230;
pub const GET_OPERATION_RESULT: u32 = 231;
pub const GET_UNIQUE_PAD_DEVICE_TYPE_SET_INTERNAL: u32 = 234;

// ---------------------------------------------------------------------------
// AbstractedPad commands (5.0.0-8.1.0)
// ---------------------------------------------------------------------------

pub const GET_ABSTRACTED_PAD_HANDLES: u32 = 301;
pub const GET_ABSTRACTED_PAD_STATE: u32 = 302;
pub const GET_ABSTRACTED_PADS_STATE: u32 = 303;
pub const SET_AUTO_PILOT_VIRTUAL_PAD_STATE: u32 = 321;
pub const UNSET_AUTO_PILOT_VIRTUAL_PAD_STATE: u32 = 322;
pub const UNSET_ALL_AUTO_PILOT_VIRTUAL_PAD_STATE: u32 = 323;

// ---------------------------------------------------------------------------
// HDLS commands (7.0.0+)
// ---------------------------------------------------------------------------

pub const ATTACH_HDLS_WORK_BUFFER: u32 = 324;
pub const RELEASE_HDLS_WORK_BUFFER: u32 = 325;
pub const DUMP_HDLS_NPAD_ASSIGNMENT_STATE: u32 = 326;
pub const DUMP_HDLS_STATES: u32 = 327;
pub const APPLY_HDLS_NPAD_ASSIGNMENT_STATE: u32 = 328;
pub const APPLY_HDLS_STATE_LIST: u32 = 329;
pub const ATTACH_HDLS_VIRTUAL_DEVICE: u32 = 330;
pub const DETACH_HDLS_VIRTUAL_DEVICE: u32 = 331;
pub const SET_HDLS_STATE: u32 = 332;
