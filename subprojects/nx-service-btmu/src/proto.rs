//! Bluetooth Manager User service protocol constants.

use nx_sf::ServiceName;

/// Service name for btm:u.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("btm:u");

// Root service commands

/// GetCore (cmd 0) -- returns IBtmUserCore sub-object.
pub const GET_CORE: u32 = 0;

// IBtmUserCore -- BLE scan commands

/// AcquireBleScanEvent (cmd 0).
pub const ACQUIRE_BLE_SCAN_EVENT: u32 = 0;

/// GetBleScanFilterParameter (cmd 1).
pub const GET_BLE_SCAN_FILTER_PARAMETER: u32 = 1;

/// GetBleScanFilterParameter2 (cmd 2).
pub const GET_BLE_SCAN_FILTER_PARAMETER2: u32 = 2;

/// StartBleScanForGeneral (cmd 3).
pub const START_BLE_SCAN_FOR_GENERAL: u32 = 3;

/// StopBleScanForGeneral (cmd 4).
pub const STOP_BLE_SCAN_FOR_GENERAL: u32 = 4;

/// GetBleScanResultsForGeneral (cmd 5).
pub const GET_BLE_SCAN_RESULTS_FOR_GENERAL: u32 = 5;

/// StartBleScanForPaired (cmd 6).
pub const START_BLE_SCAN_FOR_PAIRED: u32 = 6;

/// StopBleScanForPaired (cmd 7).
pub const STOP_BLE_SCAN_FOR_PAIRED: u32 = 7;

/// StartBleScanForSmartDevice (cmd 8).
pub const START_BLE_SCAN_FOR_SMART_DEVICE: u32 = 8;

/// StopBleScanForSmartDevice (cmd 9).
pub const STOP_BLE_SCAN_FOR_SMART_DEVICE: u32 = 9;

/// GetBleScanResultsForSmartDevice (cmd 10).
pub const GET_BLE_SCAN_RESULTS_FOR_SMART_DEVICE: u32 = 10;

// IBtmUserCore -- BLE connection commands

/// AcquireBleConnectionEvent (cmd 17).
pub const ACQUIRE_BLE_CONNECTION_EVENT: u32 = 17;

/// BleConnect (cmd 18).
pub const BLE_CONNECT: u32 = 18;

/// BleDisconnect (cmd 19).
pub const BLE_DISCONNECT: u32 = 19;

/// BleGetConnectionState (cmd 20).
pub const BLE_GET_CONNECTION_STATE: u32 = 20;

// IBtmUserCore -- BLE pairing commands

/// AcquireBlePairingEvent (cmd 21).
pub const ACQUIRE_BLE_PAIRING_EVENT: u32 = 21;

/// BlePairDevice (cmd 22).
pub const BLE_PAIR_DEVICE: u32 = 22;

/// BleUnPairDevice (cmd 23).
pub const BLE_UNPAIR_DEVICE: u32 = 23;

/// BleUnPairDevice2 (cmd 24).
pub const BLE_UNPAIR_DEVICE2: u32 = 24;

/// BleGetPairedDevices (cmd 25).
pub const BLE_GET_PAIRED_DEVICES: u32 = 25;

// IBtmUserCore -- GATT service discovery commands

/// AcquireBleServiceDiscoveryEvent (cmd 26).
pub const ACQUIRE_BLE_SERVICE_DISCOVERY_EVENT: u32 = 26;

/// GetGattServices (cmd 27).
pub const GET_GATT_SERVICES: u32 = 27;

/// GetGattService (cmd 28).
pub const GET_GATT_SERVICE: u32 = 28;

/// GetGattIncludedServices (cmd 29).
pub const GET_GATT_INCLUDED_SERVICES: u32 = 29;

/// GetBelongingGattService (cmd 30).
pub const GET_BELONGING_GATT_SERVICE: u32 = 30;

/// GetGattCharacteristics (cmd 31).
pub const GET_GATT_CHARACTERISTICS: u32 = 31;

/// GetGattDescriptors (cmd 32).
pub const GET_GATT_DESCRIPTORS: u32 = 32;

// IBtmUserCore -- BLE MTU commands

/// AcquireBleMtuConfigEvent (cmd 33).
pub const ACQUIRE_BLE_MTU_CONFIG_EVENT: u32 = 33;

/// ConfigureBleMtu (cmd 34).
pub const CONFIGURE_BLE_MTU: u32 = 34;

/// GetBleMtu (cmd 35).
pub const GET_BLE_MTU: u32 = 35;

// IBtmUserCore -- GATT data path commands

/// RegisterBleGattDataPath (cmd 36).
pub const REGISTER_BLE_GATT_DATA_PATH: u32 = 36;

/// UnregisterBleGattDataPath (cmd 37).
pub const UNREGISTER_BLE_GATT_DATA_PATH: u32 = 37;
