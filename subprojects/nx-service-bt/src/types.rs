//! Bluetooth user service wire-layout types.

use static_assertions::const_assert_eq;

/// GATT attribute UUID.
///
/// Size field indicates UUID length: 0x2, 0x4, or 0x10 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BtdrvGattAttributeUuid {
    pub size: u32,
    pub uuid: [u8; 0x10],
}

const_assert_eq!(size_of::<BtdrvGattAttributeUuid>(), 0x14);

/// GATT ID combining an instance ID with a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BtdrvGattId {
    pub instance_id: u8,
    pub pad: [u8; 3],
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<BtdrvGattId>(), 0x18);

/// BLE event type returned by [`get_le_event_info`](crate::BtService::get_le_event_info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtdrvBleEventType {
    ClientRegistration = 0,
    ServerRegistration = 1,
    ConnectionUpdate = 2,
    PreferredConnectionParameters = 3,
    ClientConnection = 4,
    ServerConnection = 5,
    ScanResult = 6,
    ScanFilter = 7,
    ClientNotify = 8,
    ClientCacheSave = 9,
    ClientCacheLoad = 10,
    ClientConfigureMtu = 11,
    ServerAddAttribute = 12,
    ServerAttributeOperation = 13,
}

// --- Wire input structs for IPC commands ---

/// Input for LeClientReadCharacteristic (cmd 0).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ReadCharacteristicIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ReadCharacteristicIn>(), 0x40);

/// Input for LeClientReadDescriptor (cmd 1).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ReadDescriptorIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub desc_id: BtdrvGattId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ReadDescriptorIn>(), 0x58);

/// Input for LeClientWriteCharacteristic (cmd 2).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteCharacteristicIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub with_response: u8,
    pub pad: u8,
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<WriteCharacteristicIn>(), 0x40);

/// Input for LeClientWriteDescriptor (cmd 3).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteDescriptorIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub desc_id: BtdrvGattId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<WriteDescriptorIn>(), 0x58);

/// Input for LeClientRegisterNotification / DeregisterNotification (cmds 4, 5).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct NotificationIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<NotificationIn>(), 0x40);

/// Input for SetLeResponse (cmd 6).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetLeResponseIn {
    pub server_if: u8,
    pub pad: [u8; 3],
    pub serv_uuid: BtdrvGattAttributeUuid,
    pub char_uuid: BtdrvGattAttributeUuid,
    pub pad2: [u8; 4],
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetLeResponseIn>(), 0x38);

/// Input for LeSendIndication (cmd 7).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SendIndicationIn {
    pub server_if: u8,
    pub noconfirm: u8,
    pub pad: [u8; 2],
    pub serv_uuid: BtdrvGattAttributeUuid,
    pub char_uuid: BtdrvGattAttributeUuid,
    pub pad2: [u8; 4],
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SendIndicationIn>(), 0x38);
