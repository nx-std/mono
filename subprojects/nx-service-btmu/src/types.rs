//! Bluetooth Manager User service wire-layout types.

use static_assertions::const_assert_eq;

/// Bluetooth device address (6-byte MAC).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvAddress {
    pub address: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvAddress>(), 0x6);

/// GATT attribute UUID.
///
/// Size field indicates UUID length: 0x2, 0x4, or 0x10 bytes.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvGattAttributeUuid {
    pub size: u32,
    pub uuid: [u8; 0x10],
}

const_assert_eq!(size_of::<BtdrvGattAttributeUuid>(), 0x14);

/// BLE advertise packet parameter.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvBleAdvertisePacketParameter {
    pub company_id: u16,
    pub pattern_data: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvBleAdvertisePacketParameter>(), 0x8);

/// BLE scan result.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvBleScanResult {
    pub unk_x0: u8,
    pub addr: BtdrvAddress,
    pub unk_x7: [u8; 0x139],
    pub count: i32,
    pub unk_x144: i32,
}

const_assert_eq!(size_of::<BtdrvBleScanResult>(), 0x148);

/// BLE connection info.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvBleConnectionInfo {
    pub connection_handle: u32,
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
}

const_assert_eq!(size_of::<BtdrvBleConnectionInfo>(), 0xC);

/// GATT service.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtmGattService {
    pub unk_x0: [u8; 4],
    pub uuid: BtdrvGattAttributeUuid,
    pub handle: u16,
    pub unk_x1a: [u8; 2],
    pub instance_id: u16,
    pub end_group_handle: u16,
    pub primary_service: u8,
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<BtmGattService>(), 0x24);

/// GATT characteristic.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtmGattCharacteristic {
    pub unk_x0: [u8; 4],
    pub uuid: BtdrvGattAttributeUuid,
    pub handle: u16,
    pub unk_x1a: [u8; 2],
    pub instance_id: u16,
    pub properties: u8,
    pub unk_x1f: [u8; 5],
}

const_assert_eq!(size_of::<BtmGattCharacteristic>(), 0x24);

/// GATT descriptor.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtmGattDescriptor {
    pub unk_x0: [u8; 4],
    pub uuid: BtdrvGattAttributeUuid,
    pub handle: u16,
    pub unk_x1a: [u8; 6],
}

const_assert_eq!(size_of::<BtmGattDescriptor>(), 0x20);

/// BLE data path configuration.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BtmBleDataPath {
    pub unk_x0: u8,
    pub pad: [u8; 3],
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<BtmBleDataPath>(), 0x18);

/// Input for StartBleScanForGeneral / StartBleScanForPaired (cmds 3, 6).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ScanParamAruidIn {
    pub param: BtdrvBleAdvertisePacketParameter,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ScanParamAruidIn>(), 0x10);

/// Input for StartBleScanForSmartDevice (cmd 8).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ScanUuidAruidIn {
    pub uuid: BtdrvGattAttributeUuid,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ScanUuidAruidIn>(), 0x20);

/// Input for BleConnect (cmd 18).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct BleConnectIn {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<BleConnectIn>(), 0x10);

/// Input for BlePairDevice / BleUnPairDevice (cmds 22, 23).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct PairDeviceIn {
    pub param: BtdrvBleAdvertisePacketParameter,
    pub connection_handle: u32,
}

const_assert_eq!(size_of::<PairDeviceIn>(), 0xC);

/// Input for BleUnPairDevice2 (cmd 24).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct UnpairDevice2In {
    pub addr: BtdrvAddress,
    pub param: BtdrvBleAdvertisePacketParameter,
}

const_assert_eq!(size_of::<UnpairDevice2In>(), 0xE);

/// Input for GetGattServices (cmd 27).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattServicesIn {
    pub connection_handle: u32,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetGattServicesIn>(), 0x10);

/// Input for GetGattService (cmd 28).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattServiceIn {
    pub connection_handle: u32,
    pub uuid: BtdrvGattAttributeUuid,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetGattServiceIn>(), 0x20);

/// Input for GetGattIncludedServices / GetGattCharacteristics / GetGattDescriptors
/// (cmds 29, 31, 32) and GetBelongingGattService (cmd 30).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GattServiceDataIn {
    pub handle: u16,
    pub pad: u16,
    pub connection_handle: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GattServiceDataIn>(), 0x10);

/// Input for ConfigureBleMtu (cmd 34).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ConfigureBleMtuIn {
    pub mtu: u16,
    pub pad: u16,
    pub connection_handle: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ConfigureBleMtuIn>(), 0x10);

/// Input for GetBleMtu (cmd 35).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetBleMtuIn {
    pub connection_handle: u32,
    pub pad: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetBleMtuIn>(), 0x10);

/// Input for RegisterBleGattDataPath / UnregisterBleGattDataPath (cmds 36, 37).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GattDataPathAruidIn {
    pub path: BtmBleDataPath,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GattDataPathAruidIn>(), 0x20);
