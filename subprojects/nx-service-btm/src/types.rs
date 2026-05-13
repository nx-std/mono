//! Bluetooth Manager service wire-layout types.

use static_assertions::const_assert_eq;

// ---------------------------------------------------------------------------
// Shared btdrv types (duplicated per crate, per SPEC decision §1)
// ---------------------------------------------------------------------------

/// Bluetooth device address (6-byte MAC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtdrvAddress {
    pub address: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvAddress>(), 0x6);

/// GATT attribute UUID.
///
/// Size field indicates UUID length: 0x2, 0x4, or 0x10 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtdrvGattAttributeUuid {
    pub size: u32,
    pub uuid: [u8; 0x10],
}

const_assert_eq!(size_of::<BtdrvGattAttributeUuid>(), 0x14);

/// BLE advertise packet parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtdrvBleAdvertisePacketParameter {
    pub company_id: u16,
    pub pattern_data: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvBleAdvertisePacketParameter>(), 0x8);

/// BLE scan result.
#[derive(Clone, Copy)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtdrvBleConnectionInfo {
    pub connection_handle: u32,
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
}

const_assert_eq!(size_of::<BtdrvBleConnectionInfo>(), 0xC);

// ---------------------------------------------------------------------------
// BTM enums
// ---------------------------------------------------------------------------

/// Bluetooth Manager state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmState {
    NotInitialized = 0,
    RadioOff = 1,
    MinorSlept = 2,
    RadioOffMinorSlept = 3,
    Slept = 4,
    RadioOffSlept = 5,
    Initialized = 6,
    Working = 7,
}

/// Bluetooth radio mode (pre-9.0.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmBluetoothMode {
    Dynamic2Slot = 0,
    StaticJoy = 1,
}

/// WLAN coexistence mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmWlanMode {
    Local4 = 0,
    Local8 = 1,
    None = 2,
}

/// TSI (Time Slot Interchange) mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmTsiMode {
    Fd3Td3Si10 = 0,
    Fd1Td1Si5 = 1,
    Fd1Td3Si10 = 2,
    Fd1Td5Si15 = 3,
    Fd3Td1Si10 = 4,
    Fd3Td3Si15 = 5,
    Fd5Td1Si15 = 6,
    Fd1Td3Si15 = 7,
    Fd3Td1Si15 = 8,
    Fd1Td1Si10 = 9,
    Fd1Td1Si15 = 10,
    Active = 255,
}

/// Slot mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmSlotMode {
    Slot2 = 0,
    Slot4 = 1,
    Slot6 = 2,
    Active = 3,
}

/// Device profile type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtmProfile {
    None = 0,
    Hid = 1,
    Audio = 2,
}

// ---------------------------------------------------------------------------
// BTM compound types
// ---------------------------------------------------------------------------

/// Bluetooth device name (32 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmBdName {
    pub name: [u8; 0x20],
}

const_assert_eq!(size_of::<BtmBdName>(), 0x20);

/// Bluetooth Class of Device (3 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtmClassOfDevice {
    pub class_of_device: [u8; 3],
}

const_assert_eq!(size_of::<BtmClassOfDevice>(), 0x3);

/// Bluetooth link key (16 bytes).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmLinkKey {
    pub link_key: [u8; 0x10],
}

const_assert_eq!(size_of::<BtmLinkKey>(), 0x10);

/// HID device info (VID/PID pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtmHidDeviceInfo {
    pub vid: u16,
    pub pid: u16,
}

const_assert_eq!(size_of::<BtmHidDeviceInfo>(), 0x4);

/// Host device property \[1.0.0-12.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmHostDevicePropertyV1 {
    pub addr: BtdrvAddress,
    pub class_of_device: BtmClassOfDevice,
    pub name: BtmBdName,
    pub feature_set: u8,
}

const_assert_eq!(size_of::<BtmHostDevicePropertyV1>(), 0x2A);

/// Host device property \[13.0.0+\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmHostDevicePropertyV13 {
    pub addr: BtdrvAddress,
    pub class_of_device: BtmClassOfDevice,
    pub name: [u8; 0xF9],
    pub feature_set: u8,
}

const_assert_eq!(size_of::<BtmHostDevicePropertyV13>(), 0x103);

/// Connected device entry \[1.0.0-12.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmConnectedDeviceV1 {
    pub address: BtdrvAddress,
    pub pad: [u8; 2],
    pub unk_x8: u32,
    pub name: [u8; 0x20],
    pub unk_x2c: [u8; 0x1C],
    pub vid: u16,
    pub pid: u16,
    pub unk_x4c: [u8; 0x20],
}

const_assert_eq!(size_of::<BtmConnectedDeviceV1>(), 0x6C);

/// Connected device entry \[13.0.0+\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmConnectedDeviceV13 {
    pub address: BtdrvAddress,
    pub pad: [u8; 2],
    pub profile: u32,
    pub unk_xc: [u8; 0x40],
    pub name: [u8; 0x20],
    pub unk_x6c: [u8; 0xD9],
    pub pad2: [u8; 3],
}

const_assert_eq!(size_of::<BtmConnectedDeviceV13>(), 0x148);

/// Device condition \[1.0.0-5.0.2\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceConditionV100 {
    pub unk_x0: u32,
    pub unk_x4: u32,
    pub unk_x8: u8,
    pub unk_x9: u8,
    pub max_count: u8,
    pub connected_count: u8,
    pub devices: [BtmConnectedDeviceV1; 8],
}

const_assert_eq!(size_of::<BtmDeviceConditionV100>(), 0x36C);

/// Device condition \[5.1.0-7.0.1\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceConditionV510 {
    pub unk_x0: u32,
    pub unk_x4: u32,
    pub unk_x8: u8,
    pub unk_x9: [u8; 2],
    pub max_count: u8,
    pub connected_count: u8,
    pub pad: [u8; 3],
    pub devices: [BtmConnectedDeviceV1; 8],
}

const_assert_eq!(size_of::<BtmDeviceConditionV510>(), 0x370);

/// Device condition \[8.0.0-8.1.1\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceConditionV800 {
    pub unk_x0: u32,
    pub unk_x4: u32,
    pub unk_x8: u8,
    pub unk_x9: u8,
    pub max_count: u8,
    pub connected_count: u8,
    pub devices: [BtmConnectedDeviceV1; 8],
}

const_assert_eq!(size_of::<BtmDeviceConditionV800>(), 0x36C);

/// Device condition \[9.0.0-12.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceConditionV900 {
    pub unk_x0: u32,
    pub unk_x4: u8,
    pub unk_x5: u8,
    pub max_count: u8,
    pub connected_count: u8,
    pub devices: [BtmConnectedDeviceV1; 8],
}

const_assert_eq!(size_of::<BtmDeviceConditionV900>(), 0x368);

/// Device slot mode entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceSlotMode {
    pub addr: BtdrvAddress,
    pub reserved: [u8; 2],
    pub slot_mode: u32,
}

const_assert_eq!(size_of::<BtmDeviceSlotMode>(), 0xC);

/// Device slot mode list.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceSlotModeList {
    pub device_count: u8,
    pub reserved: [u8; 3],
    pub devices: [BtmDeviceSlotMode; 8],
}

const_assert_eq!(size_of::<BtmDeviceSlotModeList>(), 0x64);

/// Device info \[1.0.0-12.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceInfoV1 {
    pub addr: BtdrvAddress,
    pub class_of_device: BtmClassOfDevice,
    pub name: BtmBdName,
    pub link_key: BtmLinkKey,
    pub reserved: [u8; 3],
    pub profile: u32,
    pub profile_info: [u8; 4],
    pub reserved2: [u8; 0x1C],
}

const_assert_eq!(size_of::<BtmDeviceInfoV1>(), 0x60);

/// Device info \[13.0.0+\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceInfoV13 {
    pub addr: BtdrvAddress,
    pub class_of_device: BtmClassOfDevice,
    pub link_key: BtmLinkKey,
    pub reserved: [u8; 3],
    pub profile: u32,
    pub profile_info: [u8; 4],
    pub reserved2: [u8; 0x1C],
    pub name: [u8; 0xF9],
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<BtmDeviceInfoV13>(), 0x13C);

/// Device info list \[1.0.0-12.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceInfoList {
    pub device_count: u8,
    pub reserved: [u8; 3],
    pub devices: [BtmDeviceInfoV1; 10],
}

const_assert_eq!(size_of::<BtmDeviceInfoList>(), 0x3C4);

/// Device property.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDeviceProperty {
    pub addr: BtdrvAddress,
    pub class_of_device: BtmClassOfDevice,
    pub name: BtmBdName,
}

const_assert_eq!(size_of::<BtmDeviceProperty>(), 0x29);

/// Device property list.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmDevicePropertyList {
    pub device_count: u8,
    pub devices: [BtmDeviceProperty; 15],
}

const_assert_eq!(size_of::<BtmDevicePropertyList>(), 0x268);

/// Zero retransmission list.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmZeroRetransmissionList {
    pub enabled_report_id_count: u8,
    pub enabled_report_id: [u8; 0x10],
}

const_assert_eq!(size_of::<BtmZeroRetransmissionList>(), 0x11);

/// GATT client condition list (opaque).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmGattClientConditionList {
    pub data: [u8; 0x74],
}

const_assert_eq!(size_of::<BtmGattClientConditionList>(), 0x74);

/// GATT service.
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmGattDescriptor {
    pub unk_x0: [u8; 4],
    pub uuid: BtdrvGattAttributeUuid,
    pub handle: u16,
    pub unk_x1a: [u8; 6],
}

const_assert_eq!(size_of::<BtmGattDescriptor>(), 0x20);

/// BLE data path configuration.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BtmBleDataPath {
    pub unk_x0: u8,
    pub pad: [u8; 3],
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<BtmBleDataPath>(), 0x18);

/// Audio device information returned by discovery/connection queries.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmAudioDevice {
    pub addr: BtdrvAddress,
    pub name: [u8; 0xF9],
}

const_assert_eq!(size_of::<BtmAudioDevice>(), 0xFF);

// ---------------------------------------------------------------------------
// Wire input structs for IPC commands
// ---------------------------------------------------------------------------

/// Input for SetBurstMode (cmd 4): address + bool flag.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct AddrBoolIn {
    pub addr: BtdrvAddress,
    pub flag: u8,
}

const_assert_eq!(size_of::<AddrBoolIn>(), 0x7);

/// Input for LlrNotify 9.0.0+ (cmd 13): address + pad + i32.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LlrNotifyIn {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub unk: i32,
}

const_assert_eq!(size_of::<LlrNotifyIn>(), 0xC);

/// Input for BlePairDevice / BleUnpairDeviceOnBoth (cmds 41/42): param + connection_handle.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct BlePairDeviceIn {
    pub param: BtdrvBleAdvertisePacketParameter,
    pub connection_handle: u32,
}

const_assert_eq!(size_of::<BlePairDeviceIn>(), 0xC);

/// Input for BleUnPairDevice (cmd 43): address + param.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct BleUnpairDeviceIn {
    pub addr: BtdrvAddress,
    pub param: BtdrvBleAdvertisePacketParameter,
}

const_assert_eq!(size_of::<BleUnpairDeviceIn>(), 0xE);

/// Input for GetGattService / GetBelongingService: handle + pad + connection_handle.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct HandleConnectionIn {
    pub handle: u16,
    pub pad: u16,
    pub connection_handle: u32,
}

const_assert_eq!(size_of::<HandleConnectionIn>(), 0x8);

/// Input for GetGattService (cmd 47): connection_handle + uuid.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetGattServiceIn {
    pub connection_handle: u32,
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattServiceIn>(), 0x18);

/// Input for ConfigureBleMtu (cmd 53): mtu + pad + connection_handle.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ConfigureBleMtuIn {
    pub mtu: u16,
    pub pad: u16,
    pub connection_handle: u32,
}

const_assert_eq!(size_of::<ConfigureBleMtuIn>(), 0x8);

/// Input for RegisterAppletResourceUserId (cmd 57): unk + aruid.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterAruidIn {
    pub unk: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<RegisterAruidIn>(), 0x10);
