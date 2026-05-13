//! Bluetooth Manager service protocol constants.

use nx_sf::ServiceName;

/// Service name for btm.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("btm");

// ---------------------------------------------------------------------------
// Command IDs — always the same across versions
// ---------------------------------------------------------------------------

pub const GET_STATE: u32 = 0;
pub const GET_HOST_DEVICE_PROPERTY: u32 = 1;
pub const ACQUIRE_DEVICE_CONDITION_EVENT: u32 = 2;
pub const GET_DEVICE_CONDITION: u32 = 3;
pub const SET_BURST_MODE: u32 = 4;
pub const SET_SLOT_MODE: u32 = 5;
pub const SET_BLUETOOTH_MODE: u32 = 6;
pub const SET_WLAN_MODE: u32 = 7;
pub const ACQUIRE_DEVICE_INFO_EVENT: u32 = 8;
pub const GET_DEVICE_INFO: u32 = 9;
pub const ADD_DEVICE_INFO: u32 = 10;
pub const REMOVE_DEVICE_INFO: u32 = 11;
pub const INCREASE_DEVICE_INFO_ORDER: u32 = 12;
pub const LLR_NOTIFY: u32 = 13;
pub const ENABLE_RADIO: u32 = 14;
pub const DISABLE_RADIO: u32 = 15;
pub const HID_DISCONNECT: u32 = 16;
pub const HID_SET_RETRANSMISSION_MODE: u32 = 17;
pub const ACQUIRE_AWAKE_REQ_EVENT: u32 = 18;
pub const ACQUIRE_LLR_STATE_EVENT: u32 = 19;
pub const IS_LLR_STARTED: u32 = 20;
pub const ENABLE_SLOT_SAVING: u32 = 21;
pub const PROTECT_DEVICE_INFO: u32 = 22;
pub const ACQUIRE_BLE_SCAN_EVENT: u32 = 23;

// ---------------------------------------------------------------------------
// Command IDs — 5.1.0+ layout
// ---------------------------------------------------------------------------

pub const GET_BLE_SCAN_PARAMETER_GENERAL: u32 = 24;
pub const GET_BLE_SCAN_PARAMETER_SMART_DEVICE: u32 = 25;
pub const START_BLE_SCAN_FOR_GENERAL: u32 = 26;
pub const STOP_BLE_SCAN_FOR_GENERAL: u32 = 27;
pub const GET_BLE_SCAN_RESULTS_FOR_GENERAL: u32 = 28;
pub const START_BLE_SCAN_FOR_PAIRED: u32 = 29;
pub const STOP_BLE_SCAN_FOR_PAIRED: u32 = 30;
pub const START_BLE_SCAN_FOR_SMART_DEVICE: u32 = 31;
pub const STOP_BLE_SCAN_FOR_SMART_DEVICE: u32 = 32;
pub const GET_BLE_SCAN_RESULTS_FOR_SMART_DEVICE: u32 = 33;
pub const ACQUIRE_BLE_CONNECTION_EVENT: u32 = 34;
pub const BLE_CONNECT: u32 = 35;
pub const BLE_OVERRIDE_CONNECTION: u32 = 36;
pub const BLE_DISCONNECT: u32 = 37;
pub const BLE_GET_CONNECTION_STATE: u32 = 38;
pub const BLE_GET_GATT_CLIENT_CONDITION_LIST: u32 = 39;
pub const ACQUIRE_BLE_PAIRING_EVENT: u32 = 40;
pub const BLE_PAIR_DEVICE: u32 = 41;
pub const BLE_UNPAIR_DEVICE_ON_BOTH: u32 = 42;
pub const BLE_UNPAIR_DEVICE: u32 = 43;
pub const BLE_GET_PAIRED_ADDRESSES: u32 = 44;
pub const ACQUIRE_BLE_SERVICE_DISCOVERY_EVENT: u32 = 45;
pub const GET_GATT_SERVICES: u32 = 46;
pub const GET_GATT_SERVICE: u32 = 47;
pub const GET_GATT_INCLUDED_SERVICES: u32 = 48;
pub const GET_BELONGING_SERVICE: u32 = 49;
pub const GET_GATT_CHARACTERISTICS: u32 = 50;
pub const GET_GATT_DESCRIPTORS: u32 = 51;
pub const ACQUIRE_BLE_MTU_CONFIG_EVENT: u32 = 52;
pub const CONFIGURE_BLE_MTU: u32 = 53;
pub const GET_BLE_MTU: u32 = 54;
pub const REGISTER_BLE_GATT_DATA_PATH: u32 = 55;
pub const UNREGISTER_BLE_GATT_DATA_PATH: u32 = 56;
pub const REGISTER_APPLET_RESOURCE_USER_ID: u32 = 57;
pub const UNREGISTER_APPLET_RESOURCE_USER_ID: u32 = 58;
pub const SET_APPLET_RESOURCE_USER_ID: u32 = 59;

// ---------------------------------------------------------------------------
// Command IDs — 5.0.0-5.0.2 legacy layout (before BLE scan commands existed)
// ---------------------------------------------------------------------------

pub const BLE_CONNECT_LEGACY: u32 = 24;
pub const BLE_DISCONNECT_LEGACY: u32 = 25;
pub const BLE_GET_CONNECTION_STATE_LEGACY: u32 = 26;
pub const BLE_GET_GATT_CLIENT_CONDITION_LIST_LEGACY: u32 = 27;
pub const ACQUIRE_BLE_PAIRING_EVENT_LEGACY: u32 = 28;
pub const GET_GATT_SERVICES_LEGACY: u32 = 29;
pub const GET_GATT_SERVICE_LEGACY: u32 = 30;
pub const GET_GATT_INCLUDED_SERVICES_LEGACY: u32 = 31;
pub const GET_BELONGING_SERVICE_LEGACY: u32 = 32;
pub const GET_GATT_CHARACTERISTICS_LEGACY: u32 = 33;
pub const GET_GATT_DESCRIPTORS_LEGACY: u32 = 34;
pub const ACQUIRE_BLE_MTU_CONFIG_EVENT_LEGACY: u32 = 35;
pub const CONFIGURE_BLE_MTU_LEGACY: u32 = 36;
pub const GET_BLE_MTU_LEGACY: u32 = 37;
pub const REGISTER_BLE_GATT_DATA_PATH_LEGACY: u32 = 38;
pub const UNREGISTER_BLE_GATT_DATA_PATH_LEGACY: u32 = 39;
pub const REGISTER_APPLET_RESOURCE_USER_ID_LEGACY: u32 = 40;
pub const UNREGISTER_APPLET_RESOURCE_USER_ID_LEGACY: u32 = 41;
pub const SET_APPLET_RESOURCE_USER_ID_LEGACY: u32 = 42;
