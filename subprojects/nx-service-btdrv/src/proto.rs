//! Bluetooth Driver service protocol constants.

use nx_sf::ServiceName;

/// Service name for the Bluetooth Driver service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("btdrv");

// ---------------------------------------------------------------------------
// Core Bluetooth (stable, cmds 0-35)
// ---------------------------------------------------------------------------

pub const INITIALIZE_BLUETOOTH_DRIVER: u32 = 0;
pub const INITIALIZE_BLUETOOTH: u32 = 1;
pub const ENABLE_BLUETOOTH: u32 = 2;
pub const DISABLE_BLUETOOTH: u32 = 3;
pub const FINALIZE_BLUETOOTH: u32 = 4;
pub const GET_ADAPTER_PROPERTIES: u32 = 5;
pub const GET_ADAPTER_PROPERTY: u32 = 6;
pub const SET_ADAPTER_PROPERTY: u32 = 7;
pub const START_INQUIRY: u32 = 8;
pub const STOP_INQUIRY: u32 = 9;
pub const CREATE_BOND: u32 = 10;
pub const REMOVE_BOND: u32 = 11;
pub const CANCEL_BOND: u32 = 12;
pub const RESPOND_TO_PIN_REQUEST: u32 = 13;
pub const RESPOND_TO_SSP_REQUEST: u32 = 14;
pub const GET_EVENT_INFO: u32 = 15;
pub const INITIALIZE_HID: u32 = 16;
pub const OPEN_HID_CONNECTION: u32 = 17;
pub const CLOSE_HID_CONNECTION: u32 = 18;
pub const WRITE_HID_DATA: u32 = 19;
pub const WRITE_HID_DATA2: u32 = 20;
pub const SET_HID_REPORT: u32 = 21;
pub const GET_HID_REPORT: u32 = 22;
pub const TRIGGER_CONNECTION: u32 = 23;
pub const ADD_PAIRED_DEVICE_INFO: u32 = 24;
pub const GET_PAIRED_DEVICE_INFO: u32 = 25;
pub const FINALIZE_HID: u32 = 26;
pub const GET_HID_EVENT_INFO: u32 = 27;
pub const SET_TSI: u32 = 28;
pub const ENABLE_BURST_MODE: u32 = 29;
pub const SET_ZERO_RETRANSMISSION: u32 = 30;
pub const ENABLE_MC_MODE: u32 = 31;
pub const ENABLE_LLR_SCAN: u32 = 32;
pub const DISABLE_LLR_SCAN: u32 = 33;
pub const ENABLE_RADIO: u32 = 34;
pub const SET_VISIBILITY: u32 = 35;

// ---------------------------------------------------------------------------
// HID report / radio settings (4.0.0+ shift — EnableTbfcScan inserted at 36)
// ---------------------------------------------------------------------------

pub const ENABLE_TBFC_SCAN: u32 = 36; // 4.0.0+

pub const REGISTER_HID_REPORT_EVENT: u32 = 37; // 4.0.0+
pub const REGISTER_HID_REPORT_EVENT_LEGACY: u32 = 36; // pre-4.0.0

pub const GET_HID_REPORT_EVENT_INFO: u32 = 38; // 4.0.0+
pub const GET_HID_REPORT_EVENT_INFO_LEGACY: u32 = 37; // pre-4.0.0

pub const GET_LATEST_PLR: u32 = 39; // 4.0.0+
pub const GET_LATEST_PLR_LEGACY: u32 = 38; // pre-4.0.0

pub const GET_PENDING_CONNECTIONS: u32 = 40; // 4.0.0+
pub const GET_PENDING_CONNECTIONS_LEGACY: u32 = 39; // pre-4.0.0

pub const GET_CHANNEL_MAP: u32 = 41; // 4.0.0+
pub const GET_CHANNEL_MAP_LEGACY: u32 = 40; // pre-4.0.0

pub const ENABLE_TX_POWER_BOOST_SETTING: u32 = 42; // 4.0.0+
pub const ENABLE_TX_POWER_BOOST_SETTING_LEGACY: u32 = 41; // pre-4.0.0

pub const IS_TX_POWER_BOOST_SETTING_ENABLED: u32 = 43; // 4.0.0+
pub const IS_TX_POWER_BOOST_SETTING_ENABLED_LEGACY: u32 = 42; // pre-4.0.0

pub const ENABLE_AFH_SETTING: u32 = 44; // 4.0.0+
pub const ENABLE_AFH_SETTING_LEGACY: u32 = 43; // pre-4.0.0

pub const IS_AFH_SETTING_ENABLED: u32 = 45; // 4.0.0+
pub const IS_AFH_SETTING_ENABLED_LEGACY: u32 = 44; // pre-4.0.0

// ---------------------------------------------------------------------------
// BLE (5.0.0+, stable IDs)
// ---------------------------------------------------------------------------

pub const INITIALIZE_BLE: u32 = 46;
pub const ENABLE_BLE: u32 = 47;
pub const DISABLE_BLE: u32 = 48;
pub const FINALIZE_BLE: u32 = 49;
pub const SET_BLE_VISIBILITY: u32 = 50;
pub const SET_BLE_CONNECTION_PARAMETER: u32 = 51;
pub const SET_BLE_DEFAULT_CONNECTION_PARAMETER: u32 = 52;
pub const SET_BLE_ADVERTISE_DATA: u32 = 53;
pub const SET_BLE_ADVERTISE_PARAMETER: u32 = 54;
pub const START_BLE_SCAN: u32 = 55;
pub const STOP_BLE_SCAN: u32 = 56;
pub const ADD_BLE_SCAN_FILTER_CONDITION: u32 = 57;
pub const DELETE_BLE_SCAN_FILTER_CONDITION: u32 = 58;
pub const DELETE_BLE_SCAN_FILTER: u32 = 59;
pub const CLEAR_BLE_SCAN_FILTERS: u32 = 60;
pub const ENABLE_BLE_SCAN_FILTER: u32 = 61;

// ---------------------------------------------------------------------------
// GATT (5.0.0+, 5.1.0 shift — CancelConnectGattServer inserted at 66)
// ---------------------------------------------------------------------------

pub const REGISTER_GATT_CLIENT: u32 = 62;
pub const UNREGISTER_GATT_CLIENT: u32 = 63;
pub const UNREGISTER_ALL_GATT_CLIENTS: u32 = 64;
pub const CONNECT_GATT_SERVER: u32 = 65;

pub const CANCEL_CONNECT_GATT_SERVER: u32 = 66; // 5.1.0+

pub const DISCONNECT_GATT_SERVER: u32 = 67; // 5.1.0+
pub const DISCONNECT_GATT_SERVER_LEGACY: u32 = 66; // pre-5.1.0

pub const GET_GATT_ATTRIBUTE: u32 = 68; // 5.1.0+
pub const GET_GATT_ATTRIBUTE_LEGACY: u32 = 67; // pre-5.1.0

pub const GET_GATT_SERVICE: u32 = 69; // 5.1.0+
pub const GET_GATT_SERVICE_LEGACY: u32 = 68; // pre-5.1.0

pub const CONFIGURE_ATT_MTU: u32 = 70; // 5.1.0+
pub const CONFIGURE_ATT_MTU_LEGACY: u32 = 69; // pre-5.1.0

pub const REGISTER_GATT_SERVER: u32 = 71; // 5.1.0+
pub const REGISTER_GATT_SERVER_LEGACY: u32 = 70; // pre-5.1.0

pub const UNREGISTER_GATT_SERVER: u32 = 72; // 5.1.0+
pub const UNREGISTER_GATT_SERVER_LEGACY: u32 = 71; // pre-5.1.0

pub const CONNECT_GATT_CLIENT: u32 = 73; // 5.1.0+
pub const CONNECT_GATT_CLIENT_LEGACY: u32 = 72; // pre-5.1.0

pub const DISCONNECT_GATT_CLIENT: u32 = 74; // 5.1.0+
pub const DISCONNECT_GATT_CLIENT_LEGACY: u32 = 73; // pre-5.1.0

pub const ADD_GATT_SERVICE: u32 = 75;

pub const ENABLE_GATT_SERVICE: u32 = 76; // 5.1.0+
pub const ENABLE_GATT_SERVICE_LEGACY: u32 = 74; // pre-5.1.0

pub const ADD_GATT_CHARACTERISTIC: u32 = 77;

pub const ADD_GATT_DESCRIPTOR: u32 = 78; // 5.1.0+
pub const ADD_GATT_DESCRIPTOR_LEGACY: u32 = 76; // pre-5.1.0

pub const GET_BLE_MANAGED_EVENT_INFO: u32 = 79; // 5.1.0+
pub const GET_BLE_MANAGED_EVENT_INFO_LEGACY: u32 = 78; // pre-5.1.0

pub const GET_GATT_FIRST_CHARACTERISTIC: u32 = 80; // 5.1.0+
pub const GET_GATT_FIRST_CHARACTERISTIC_LEGACY: u32 = 79; // pre-5.1.0

pub const GET_GATT_NEXT_CHARACTERISTIC: u32 = 81; // 5.1.0+
pub const GET_GATT_NEXT_CHARACTERISTIC_LEGACY: u32 = 80; // pre-5.1.0

pub const GET_GATT_FIRST_DESCRIPTOR: u32 = 82; // 5.1.0+
pub const GET_GATT_FIRST_DESCRIPTOR_LEGACY: u32 = 81; // pre-5.1.0

pub const GET_GATT_NEXT_DESCRIPTOR: u32 = 83; // 5.1.0+
pub const GET_GATT_NEXT_DESCRIPTOR_LEGACY: u32 = 82; // pre-5.1.0

pub const REGISTER_GATT_MANAGED_DATA_PATH: u32 = 84;
pub const UNREGISTER_GATT_MANAGED_DATA_PATH: u32 = 85;
pub const REGISTER_GATT_HID_DATA_PATH: u32 = 86;
pub const UNREGISTER_GATT_HID_DATA_PATH: u32 = 87;
pub const REGISTER_GATT_DATA_PATH: u32 = 88;

pub const UNREGISTER_GATT_DATA_PATH: u32 = 89; // 5.1.0+
pub const UNREGISTER_GATT_DATA_PATH_LEGACY: u32 = 83; // pre-5.1.0

pub const READ_GATT_CHARACTERISTIC: u32 = 90; // 5.1.0+
pub const READ_GATT_CHARACTERISTIC_LEGACY: u32 = 89; // pre-5.1.0

pub const READ_GATT_DESCRIPTOR: u32 = 91; // 5.1.0+
pub const READ_GATT_DESCRIPTOR_LEGACY: u32 = 90; // pre-5.1.0

pub const WRITE_GATT_CHARACTERISTIC: u32 = 92; // 5.1.0+
pub const WRITE_GATT_CHARACTERISTIC_LEGACY: u32 = 91; // pre-5.1.0

pub const WRITE_GATT_DESCRIPTOR: u32 = 93; // 5.1.0+
pub const WRITE_GATT_DESCRIPTOR_LEGACY: u32 = 92; // pre-5.1.0

pub const REGISTER_GATT_NOTIFICATION: u32 = 94;

pub const UNREGISTER_GATT_NOTIFICATION: u32 = 95; // 5.1.0+
pub const UNREGISTER_GATT_NOTIFICATION_LEGACY: u32 = 93; // pre-5.1.0

pub const GET_LE_HID_EVENT_INFO: u32 = 96; // 5.1.0+
pub const GET_LE_HID_EVENT_INFO_LEGACY: u32 = 95; // pre-5.1.0

pub const REGISTER_BLE_HID_EVENT: u32 = 97; // 5.1.0+
pub const REGISTER_BLE_HID_EVENT_LEGACY: u32 = 96; // pre-5.1.0

// ---------------------------------------------------------------------------
// BLE scan parameter (5.1.0+)
// ---------------------------------------------------------------------------

pub const SET_BLE_SCAN_PARAMETER: u32 = 98;

// ---------------------------------------------------------------------------
// Piconet (10.0.0+)
// ---------------------------------------------------------------------------

pub const MOVE_TO_SECONDARY_PICONET: u32 = 99;

// ---------------------------------------------------------------------------
// Bluetooth state (12.0.0+)
// ---------------------------------------------------------------------------

pub const IS_BLUETOOTH_ENABLED: u32 = 100;

// ---------------------------------------------------------------------------
// Audio (12.0.0+)
// ---------------------------------------------------------------------------

pub const ACQUIRE_AUDIO_EVENT: u32 = 128;
pub const GET_AUDIO_EVENT_INFO: u32 = 129;
pub const OPEN_AUDIO_CONNECTION: u32 = 130;
pub const CLOSE_AUDIO_CONNECTION: u32 = 131;
pub const OPEN_AUDIO_OUT: u32 = 132;
pub const CLOSE_AUDIO_OUT: u32 = 133;
pub const ACQUIRE_AUDIO_OUT_STATE_CHANGED_EVENT: u32 = 134;
pub const START_AUDIO_OUT: u32 = 135;
pub const STOP_AUDIO_OUT: u32 = 136;
pub const GET_AUDIO_OUT_STATE: u32 = 137;
pub const GET_AUDIO_OUT_FEEDING_CODEC: u32 = 138;
pub const GET_AUDIO_OUT_FEEDING_PARAMETER: u32 = 139;
pub const ACQUIRE_AUDIO_OUT_BUFFER_AVAILABLE_EVENT: u32 = 140;
pub const SEND_AUDIO_DATA: u32 = 141;
pub const ACQUIRE_AUDIO_CONTROL_INPUT_STATE_CHANGED_EVENT: u32 = 142;
pub const GET_AUDIO_CONTROL_INPUT_STATE: u32 = 143;
pub const ACQUIRE_AUDIO_CONNECTION_STATE_CHANGED_EVENT: u32 = 144;
pub const GET_CONNECTED_AUDIO_DEVICE: u32 = 145;
pub const CLOSE_AUDIO_CONTROL_INPUT: u32 = 146;
pub const REGISTER_AUDIO_CONTROL_NOTIFICATION: u32 = 147;
pub const SEND_AUDIO_CONTROL_PASSTHROUGH_COMMAND: u32 = 148;
pub const SEND_AUDIO_CONTROL_SET_ABSOLUTE_VOLUME_COMMAND: u32 = 149;

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

pub const IS_MANUFACTURING_MODE: u32 = 256;
pub const EMULATE_BLUETOOTH_CRASH: u32 = 257;
pub const GET_BLE_CHANNEL_MAP: u32 = 258;
