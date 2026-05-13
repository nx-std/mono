//! Bluetooth user service protocol constants.

use nx_sf::ServiceName;

/// Service name for the Bluetooth user service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("bt");

/// LeClientReadCharacteristic (cmd 0).
pub const LE_CLIENT_READ_CHARACTERISTIC: u32 = 0;

/// LeClientReadDescriptor (cmd 1).
pub const LE_CLIENT_READ_DESCRIPTOR: u32 = 1;

/// LeClientWriteCharacteristic (cmd 2).
pub const LE_CLIENT_WRITE_CHARACTERISTIC: u32 = 2;

/// LeClientWriteDescriptor (cmd 3).
pub const LE_CLIENT_WRITE_DESCRIPTOR: u32 = 3;

/// LeClientRegisterNotification (cmd 4).
pub const LE_CLIENT_REGISTER_NOTIFICATION: u32 = 4;

/// LeClientDeregisterNotification (cmd 5).
pub const LE_CLIENT_DEREGISTER_NOTIFICATION: u32 = 5;

/// SetLeResponse (cmd 6).
pub const SET_LE_RESPONSE: u32 = 6;

/// LeSendIndication (cmd 7).
pub const LE_SEND_INDICATION: u32 = 7;

/// GetLeEventInfo (cmd 8).
pub const GET_LE_EVENT_INFO: u32 = 8;

/// RegisterBleEvent (cmd 9).
pub const REGISTER_BLE_EVENT: u32 = 9;
