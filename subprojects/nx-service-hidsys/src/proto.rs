//! HID System service protocol constants.

use nx_sf::ServiceName;

/// Service name for hid:sys.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hid:sys");

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

pub const SEND_KEYBOARD_LOCK_KEY_EVENT: u32 = 31;

// ---------------------------------------------------------------------------
// Button event handles / activation (PID + ARUID)
// ---------------------------------------------------------------------------

pub const ACQUIRE_HOME_BUTTON_EVENT_HANDLE: u32 = 101;
pub const ACTIVATE_HOME_BUTTON: u32 = 111;
pub const ACQUIRE_SLEEP_BUTTON_EVENT_HANDLE: u32 = 121;
pub const ACTIVATE_SLEEP_BUTTON: u32 = 131;
pub const ACQUIRE_CAPTURE_BUTTON_EVENT_HANDLE: u32 = 141;
pub const ACTIVATE_CAPTURE_BUTTON: u32 = 151;

// ---------------------------------------------------------------------------
// Npad system policy
// ---------------------------------------------------------------------------

pub const APPLY_NPAD_SYSTEM_COMMON_POLICY: u32 = 303;
pub const GET_LAST_ACTIVE_NPAD: u32 = 306;
pub const GET_MASKED_SUPPORTED_NPAD_STYLE_SET: u32 = 310;
pub const GET_NPAD_INTERFACE_TYPE: u32 = 316;
pub const GET_NPAD_LEFT_RIGHT_INTERFACE_TYPE: u32 = 317;
pub const HAS_BATTERY: u32 = 318;
pub const HAS_LEFT_RIGHT_BATTERY: u32 = 319;
pub const GET_UNIQUE_PADS_FROM_NPAD: u32 = 321;

// ---------------------------------------------------------------------------
// Applet resource / handheld control
// ---------------------------------------------------------------------------

pub const SET_APPLET_RESOURCE_USER_ID: u32 = 500;
pub const ENABLE_APPLET_TO_GET_INPUT: u32 = 503;
pub const ENABLE_HANDHELD_HIDS: u32 = 520;
pub const DISABLE_HANDHELD_HIDS: u32 = 521;
pub const SET_JOY_CON_RAIL_ENABLED: u32 = 522;
pub const IS_JOY_CON_RAIL_ENABLED: u32 = 523;
pub const IS_HANDHELD_HIDS_ENABLED: u32 = 524;
pub const IS_JOY_CON_ATTACHED_ON_ALL_RAIL: u32 = 525;
pub const IS_INVERTED_CONTROLLER_CONNECTED_ON_RAIL: u32 = 526;

// ---------------------------------------------------------------------------
// UniquePad events / enumeration
// ---------------------------------------------------------------------------

pub const ACQUIRE_UNIQUE_PAD_CONNECTION_EVENT_HANDLE: u32 = 702;
pub const GET_UNIQUE_PAD_IDS: u32 = 703;
pub const ACQUIRE_JOY_DETACH_ON_BLUETOOTH_OFF_EVENT_HANDLE: u32 = 751;

// ---------------------------------------------------------------------------
// UniquePad device queries / LED / USB
// ---------------------------------------------------------------------------

pub const GET_UNIQUE_PAD_BLUETOOTH_ADDRESS: u32 = 805;
pub const DISCONNECT_UNIQUE_PAD: u32 = 806;
pub const GET_UNIQUE_PAD_TYPE: u32 = 807;
pub const GET_UNIQUE_PAD_INTERFACE: u32 = 808;
pub const GET_UNIQUE_PAD_SERIAL_NUMBER: u32 = 809;
pub const GET_UNIQUE_PAD_CONTROLLER_NUMBER: u32 = 810;
pub const SET_NOTIFICATION_LED_PATTERN: u32 = 830;
pub const SET_NOTIFICATION_LED_PATTERN_WITH_TIMEOUT: u32 = 831;
pub const IS_USB_FULL_KEY_CONTROLLER_ENABLED: u32 = 850;
pub const ENABLE_USB_FULL_KEY_CONTROLLER: u32 = 851;
pub const IS_USB_CONNECTED: u32 = 852;

// ---------------------------------------------------------------------------
// Touch screen
// ---------------------------------------------------------------------------

pub const GET_TOUCH_SCREEN_DEFAULT_CONFIGURATION: u32 = 1153;
pub const IS_FIRMWARE_UPDATE_NEEDED_FOR_NOTIFICATION: u32 = 1154;

// ---------------------------------------------------------------------------
// Button config — legacy [10.0.0-10.2.0]
// ---------------------------------------------------------------------------

pub const LEGACY_IS_BUTTON_CONFIG_SUPPORTED: u32 = 1200;
pub const LEGACY_DELETE_BUTTON_CONFIG: u32 = 1201;
pub const LEGACY_SET_BUTTON_CONFIG_ENABLED: u32 = 1202;
pub const LEGACY_IS_BUTTON_CONFIG_ENABLED: u32 = 1203;
pub const LEGACY_SET_BUTTON_CONFIG_EMBEDDED: u32 = 1204;
pub const LEGACY_SET_BUTTON_CONFIG_FULL: u32 = 1205;
pub const LEGACY_SET_BUTTON_CONFIG_LEFT: u32 = 1206;
pub const LEGACY_SET_BUTTON_CONFIG_RIGHT: u32 = 1207;
pub const LEGACY_GET_BUTTON_CONFIG_EMBEDDED: u32 = 1208;
pub const LEGACY_GET_BUTTON_CONFIG_FULL: u32 = 1209;
pub const LEGACY_GET_BUTTON_CONFIG_LEFT: u32 = 1210;
pub const LEGACY_GET_BUTTON_CONFIG_RIGHT: u32 = 1211;

// ---------------------------------------------------------------------------
// Button config — [11.0.0-17.0.1]
// ---------------------------------------------------------------------------

pub const IS_BUTTON_CONFIG_SUPPORTED: u32 = 1200;
pub const IS_BUTTON_CONFIG_EMBEDDED_SUPPORTED: u32 = 1201;
pub const DELETE_BUTTON_CONFIG: u32 = 1202;
pub const DELETE_BUTTON_CONFIG_EMBEDDED: u32 = 1203;
pub const SET_BUTTON_CONFIG_ENABLED: u32 = 1204;
pub const SET_BUTTON_CONFIG_EMBEDDED_ENABLED: u32 = 1205;
pub const IS_BUTTON_CONFIG_ENABLED: u32 = 1206;
pub const IS_BUTTON_CONFIG_EMBEDDED_ENABLED: u32 = 1207;
pub const SET_BUTTON_CONFIG_EMBEDDED: u32 = 1208;
pub const SET_BUTTON_CONFIG_FULL: u32 = 1209;
pub const SET_BUTTON_CONFIG_LEFT: u32 = 1210;
pub const SET_BUTTON_CONFIG_RIGHT: u32 = 1211;
pub const GET_BUTTON_CONFIG_EMBEDDED: u32 = 1212;
pub const GET_BUTTON_CONFIG_FULL: u32 = 1213;
pub const GET_BUTTON_CONFIG_LEFT: u32 = 1214;
pub const GET_BUTTON_CONFIG_RIGHT: u32 = 1215;

// ---------------------------------------------------------------------------
// Custom button config [10.0.0+]
// ---------------------------------------------------------------------------

pub const IS_CUSTOM_BUTTON_CONFIG_SUPPORTED: u32 = 1250;
pub const IS_DEFAULT_BUTTON_CONFIG_EMBEDDED: u32 = 1251;
pub const IS_DEFAULT_BUTTON_CONFIG_FULL: u32 = 1252;
pub const IS_DEFAULT_BUTTON_CONFIG_LEFT: u32 = 1253;
pub const IS_DEFAULT_BUTTON_CONFIG_RIGHT: u32 = 1254;
pub const IS_BUTTON_CONFIG_STORAGE_EMBEDDED_EMPTY: u32 = 1255;
pub const IS_BUTTON_CONFIG_STORAGE_FULL_EMPTY: u32 = 1256;
pub const IS_BUTTON_CONFIG_STORAGE_LEFT_EMPTY: u32 = 1257;
pub const IS_BUTTON_CONFIG_STORAGE_RIGHT_EMPTY: u32 = 1258;
pub const GET_BUTTON_CONFIG_STORAGE_EMBEDDED_DEPRECATED: u32 = 1259;
pub const GET_BUTTON_CONFIG_STORAGE_FULL_DEPRECATED: u32 = 1260;
pub const GET_BUTTON_CONFIG_STORAGE_LEFT_DEPRECATED: u32 = 1261;
pub const GET_BUTTON_CONFIG_STORAGE_RIGHT_DEPRECATED: u32 = 1262;
pub const SET_BUTTON_CONFIG_STORAGE_EMBEDDED_DEPRECATED: u32 = 1263;
pub const SET_BUTTON_CONFIG_STORAGE_FULL_DEPRECATED: u32 = 1264;
pub const SET_BUTTON_CONFIG_STORAGE_LEFT_DEPRECATED: u32 = 1265;
pub const SET_BUTTON_CONFIG_STORAGE_RIGHT_DEPRECATED: u32 = 1266;
pub const DELETE_BUTTON_CONFIG_STORAGE_EMBEDDED: u32 = 1267;
pub const DELETE_BUTTON_CONFIG_STORAGE_FULL: u32 = 1268;
pub const DELETE_BUTTON_CONFIG_STORAGE_LEFT: u32 = 1269;
pub const DELETE_BUTTON_CONFIG_STORAGE_RIGHT: u32 = 1270;
pub const IS_USING_CUSTOM_BUTTON_CONFIG: u32 = 1271;
pub const IS_ANY_CUSTOM_BUTTON_CONFIG_ENABLED: u32 = 1272;
pub const SET_ALL_CUSTOM_BUTTON_CONFIG_ENABLED: u32 = 1273;
pub const SET_DEFAULT_BUTTON_CONFIG: u32 = 1274;
pub const SET_ALL_DEFAULT_BUTTON_CONFIG: u32 = 1275;
pub const SET_HID_BUTTON_CONFIG_EMBEDDED: u32 = 1276;
pub const SET_HID_BUTTON_CONFIG_FULL: u32 = 1277;
pub const SET_HID_BUTTON_CONFIG_LEFT: u32 = 1278;
pub const SET_HID_BUTTON_CONFIG_RIGHT: u32 = 1279;
pub const GET_HID_BUTTON_CONFIG_EMBEDDED: u32 = 1280;
pub const GET_HID_BUTTON_CONFIG_FULL: u32 = 1281;
pub const GET_HID_BUTTON_CONFIG_LEFT: u32 = 1282;
pub const GET_HID_BUTTON_CONFIG_RIGHT: u32 = 1283;
pub const GET_BUTTON_CONFIG_STORAGE_EMBEDDED: u32 = 1284;
pub const GET_BUTTON_CONFIG_STORAGE_FULL: u32 = 1285;
pub const GET_BUTTON_CONFIG_STORAGE_LEFT: u32 = 1286;
pub const GET_BUTTON_CONFIG_STORAGE_RIGHT: u32 = 1287;
pub const SET_BUTTON_CONFIG_STORAGE_EMBEDDED: u32 = 1288;
pub const SET_BUTTON_CONFIG_STORAGE_FULL: u32 = 1289;
pub const SET_BUTTON_CONFIG_STORAGE_LEFT: u32 = 1290;
pub const SET_BUTTON_CONFIG_STORAGE_RIGHT: u32 = 1291;
