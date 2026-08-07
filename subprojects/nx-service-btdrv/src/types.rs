//! Bluetooth Driver service wire-layout types.

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

/// Class of Device (3-byte value).
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
pub struct BtdrvClassOfDevice {
    pub class_of_device: [u8; 3],
}

const_assert_eq!(size_of::<BtdrvClassOfDevice>(), 0x3);

/// Adapter properties \[1.0.0-11.0.1\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvAdapterPropertyOld {
    pub addr: BtdrvAddress,
    pub class_of_device: BtdrvClassOfDevice,
    pub name: [u8; 0xF9],
    pub feature_set: u8,
}

const_assert_eq!(size_of::<BtdrvAdapterPropertyOld>(), 0x103);

/// Single adapter property \[12.0.0+\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvAdapterProperty {
    pub property_type: u8,
    pub size: u8,
    pub data: [u8; 0x100],
}

const_assert_eq!(size_of::<BtdrvAdapterProperty>(), 0x102);

/// Adapter property set \[12.0.0+\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvAdapterPropertySet {
    pub addr: BtdrvAddress,
    pub class_of_device: BtdrvClassOfDevice,
    pub name: [u8; 0xF9],
}

const_assert_eq!(size_of::<BtdrvAdapterPropertySet>(), 0x102);

/// Bluetooth PIN code \[1.0.0-11.0.1\].
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BtdrvBluetoothPinCode {
    pub code: [u8; 0x10],
}

const_assert_eq!(size_of::<BtdrvBluetoothPinCode>(), 0x10);

/// PIN code \[12.0.0+\].
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BtdrvPinCode {
    pub code: [u8; 0x10],
    pub length: u8,
}

const_assert_eq!(size_of::<BtdrvPinCode>(), 0x11);

/// HID data \[1.0.0-8.1.1\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvHidData {
    pub size: u16,
    pub data: [u8; 0x280],
}

const_assert_eq!(size_of::<BtdrvHidData>(), 0x282);

/// HID report \[9.0.0+\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvHidReport {
    pub size: u16,
    pub data: [u8; 0x2BC],
}

const_assert_eq!(size_of::<BtdrvHidReport>(), 0x2BE);

/// PLR statistics \[pre-9.0.0\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvPlrStatistics {
    pub data: [u8; 0x84],
}

const_assert_eq!(size_of::<BtdrvPlrStatistics>(), 0x84);

/// PLR list \[9.0.0+\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvPlrList {
    pub data: [u8; 0xA4],
}

const_assert_eq!(size_of::<BtdrvPlrList>(), 0xA4);

/// Channel map list.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvChannelMapList {
    pub data: [u8; 0x88],
}

const_assert_eq!(size_of::<BtdrvChannelMapList>(), 0x88);

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

/// GATT ID (instance + UUID).
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
pub struct BtdrvGattId {
    pub instance_id: u8,
    pub pad: [u8; 3],
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<BtdrvGattId>(), 0x18);

/// LE connection parameters \[5.0.0-8.1.1\].
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
pub struct BtdrvLeConnectionParams {
    pub addr: BtdrvAddress,
    pub min_conn_interval: u16,
    pub max_conn_interval: u16,
    pub min_ce_length: u16,
    pub max_ce_length: u16,
    pub slave_latency: u16,
    pub supervision_tout: u16,
    pub preference: u8,
    pub pad: u8,
}

const_assert_eq!(size_of::<BtdrvLeConnectionParams>(), 0x14);

/// BLE connection parameter \[9.0.0+\].
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
pub struct BtdrvBleConnectionParameter {
    pub min_conn_interval: u16,
    pub max_conn_interval: u16,
    pub min_ce_length: u16,
    pub max_ce_length: u16,
    pub slave_latency: u16,
    pub supervision_tout: u16,
}

const_assert_eq!(size_of::<BtdrvBleConnectionParameter>(), 0xC);

/// BLE advertise packet data.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvBleAdvertisePacketData {
    pub adv_data_mask: u32,
    pub flag: u8,
    pub manu_data_len: u8,
    pub manu_data: [u8; 0x1F],
    pub pad: [u8; 1],
    pub appearance_data: u16,
    pub num_service: u8,
    pub pad2: [u8; 3],
    pub uuid_val: [BtdrvGattAttributeUuid; 6],
    pub service_data_len: u8,
    pub pad3: [u8; 1],
    pub service_data_uuid: u16,
    pub service_data: [u8; 0x1F],
    pub is_scan_rsp: u8,
    pub tx_power: u8,
    pub pad4: [u8; 3],
}

const_assert_eq!(size_of::<BtdrvBleAdvertisePacketData>(), 0xCC);

/// BLE advertisement entry.
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
pub struct BtdrvBleAdvertisement {
    pub size: u8,
    pub ad_type: u8,
    pub data: [u8; 0x1D],
}

const_assert_eq!(size_of::<BtdrvBleAdvertisement>(), 0x1F);

/// BLE advertise filter.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvBleAdvertiseFilter {
    pub index: u8,
    pub adv: BtdrvBleAdvertisement,
    pub mask: [u8; 0x1D],
    pub mask_size: u8,
}

const_assert_eq!(size_of::<BtdrvBleAdvertiseFilter>(), 0x3E);

/// PCM parameter for audio output.
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
pub struct BtdrvPcmParameter {
    pub unk_x0: u32,
    pub sample_rate: i32,
    pub bits_per_sample: u32,
}

const_assert_eq!(size_of::<BtdrvPcmParameter>(), 0xC);

/// Audio control button state.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct BtdrvAudioControlButtonState {
    pub data: [u8; 0x10],
}

const_assert_eq!(size_of::<BtdrvAudioControlButtonState>(), 0x10);

/// Bluetooth devices settings (opaque, from set service).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct SetSysBluetoothDevicesSettings {
    pub data: [u8; 0x200],
}

const_assert_eq!(size_of::<SetSysBluetoothDevicesSettings>(), 0x200);

/// Fatal reason for crash emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BtdrvFatalReason {
    Invalid = 0,
    Unknown1 = 1,
    CommandTimeout = 2,
    HardwareError = 3,
    Enable = 7,
    Audio = 9,
}

/// addr + u8 packed.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddrU8In {
    pub addr: BtdrvAddress,
    pub val: u8,
}

const_assert_eq!(size_of::<AddrU8In>(), 0x7);

/// addr + pad + u32.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddrU32In {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub val: u32,
}

const_assert_eq!(size_of::<AddrU32In>(), 0xC);

/// addr + pad + u32 + u32.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddrU32U32In {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub val0: u32,
    pub val1: u32,
}

const_assert_eq!(size_of::<AddrU32U32In>(), 0x10);

/// Two bools packed.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct TwoBoolsIn {
    pub flag0: u8,
    pub flag1: u8,
}

const_assert_eq!(size_of::<TwoBoolsIn>(), 0x2);

/// LegacyRespondToPinRequest input \[1.0.0-11.0.1\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct LegacyRespondToPinRequestIn {
    pub addr: BtdrvAddress,
    pub flag: u8,
    pub length: u8,
    pub pin_code: BtdrvBluetoothPinCode,
}

const_assert_eq!(size_of::<LegacyRespondToPinRequestIn>(), 0x18);

/// RespondToPinRequest input \[12.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RespondToPinRequestIn {
    pub addr: BtdrvAddress,
    pub pin_code: BtdrvPinCode,
}

const_assert_eq!(size_of::<RespondToPinRequestIn>(), 0x17);

/// RespondToSspRequest input \[1.0.0-11.0.1\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RespondToSspRequestLegacyIn {
    pub addr: BtdrvAddress,
    pub variant: u8,
    pub accept: u8,
    pub passkey: u32,
}

const_assert_eq!(size_of::<RespondToSspRequestLegacyIn>(), 0xC);

/// RespondToSspRequest input \[12.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RespondToSspRequestIn {
    pub addr: BtdrvAddress,
    pub accept: u8,
    pub _pad: u8,
    pub variant: u32,
    pub passkey: u32,
}

const_assert_eq!(size_of::<RespondToSspRequestIn>(), 0x10);

/// InitializeHid input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct InitializeHidIn {
    pub val: u16,
}

const_assert_eq!(size_of::<InitializeHidIn>(), 0x2);

/// SetHidReport input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetHidReportIn {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub report_type: u32,
}

const_assert_eq!(size_of::<SetHidReportIn>(), 0xC);

/// GetHidReport input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetHidReportIn {
    pub addr: BtdrvAddress,
    pub report_id: u8,
    pub pad: u8,
    pub report_type: u32,
}

const_assert_eq!(size_of::<GetHidReportIn>(), 0xC);

/// TriggerConnection input \[9.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct TriggerConnectionIn {
    pub addr: BtdrvAddress,
    pub timeout: u16,
}

const_assert_eq!(size_of::<TriggerConnectionIn>(), 0x8);

/// StartInquiry input \[12.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct StartInquiryIn {
    pub services: u32,
    pub _pad: u32,
    pub duration: i64,
}

const_assert_eq!(size_of::<StartInquiryIn>(), 0x10);

/// SetBleConnectionParameter input \[9.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetBleConnectionParameterIn {
    pub addr: BtdrvAddress,
    pub flag: u8,
    pub pad: u8,
    pub param: BtdrvBleConnectionParameter,
}

const_assert_eq!(size_of::<SetBleConnectionParameterIn>(), 0x14);

/// SetBleAdvertiseParameter input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetBleAdvertiseParameterIn {
    pub addr: BtdrvAddress,
    pub min_interval: u16,
    pub max_interval: u16,
}

const_assert_eq!(size_of::<SetBleAdvertiseParameterIn>(), 0xA);

/// ConnectGattServer input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ConnectGattServerIn {
    pub client_if: u8,
    pub addr: BtdrvAddress,
    pub is_direct: u8,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<ConnectGattServerIn>(), 0x10);

/// CancelConnectGattServer input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CancelConnectGattServerIn {
    pub client_if: u8,
    pub addr: BtdrvAddress,
    pub is_direct: u8,
}

const_assert_eq!(size_of::<CancelConnectGattServerIn>(), 0x8);

/// GetGattAttribute input \[pre-9.0.0\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattAttributeLegacyIn {
    pub addr: BtdrvAddress,
    pub pad: [u8; 2],
    pub conn_id: u32,
}

const_assert_eq!(size_of::<GetGattAttributeLegacyIn>(), 0xC);

/// GetGattService input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattServiceIn {
    pub conn_id: u32,
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattServiceIn>(), 0x18);

/// ConfigureAttMtu input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ConfigureAttMtuIn {
    pub mtu: u16,
    pub pad: u16,
    pub conn_id: u32,
}

const_assert_eq!(size_of::<ConfigureAttMtuIn>(), 0x8);

/// ConnectGattClient input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ConnectGattClientIn {
    pub server_if: u8,
    pub addr: BtdrvAddress,
    pub is_direct: u8,
}

const_assert_eq!(size_of::<ConnectGattClientIn>(), 0x8);

/// DisconnectGattClient input \[pre-9.0.0\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct DisconnectGattClientLegacyIn {
    pub conn_id: u8,
    pub addr: BtdrvAddress,
}

const_assert_eq!(size_of::<DisconnectGattClientLegacyIn>(), 0x7);

/// AddGattService input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddGattServiceIn {
    pub server_if: u8,
    pub num_handle: u8,
    pub is_primary: u8,
    pub pad: u8,
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<AddGattServiceIn>(), 0x18);

/// EnableGattService input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct EnableGattServiceIn {
    pub server_if: u8,
    pub pad: [u8; 3],
    pub uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<EnableGattServiceIn>(), 0x18);

/// AddGattCharacteristic input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddGattCharacteristicIn {
    pub server_if: u8,
    pub property: u8,
    pub permissions: u16,
    pub serv_uuid: BtdrvGattAttributeUuid,
    pub char_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<AddGattCharacteristicIn>(), 0x2C);

/// AddGattDescriptor input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct AddGattDescriptorIn {
    pub server_if: u8,
    pub pad: u8,
    pub permissions: u16,
    pub serv_uuid: BtdrvGattAttributeUuid,
    pub desc_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<AddGattDescriptorIn>(), 0x2C);

/// GetGattFirstCharacteristic input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattFirstCharacteristicIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub conn_id: u32,
    pub serv_id: BtdrvGattId,
    pub filter_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattFirstCharacteristicIn>(), 0x34);

/// GetGattFirstCharacteristic / GetGattNextCharacteristic output.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GetGattCharacteristicOut {
    pub property: u8,
    pub pad: [u8; 3],
    pub id: BtdrvGattId,
}

const_assert_eq!(size_of::<GetGattCharacteristicOut>(), 0x1C);

/// GetGattNextCharacteristic input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattNextCharacteristicIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub conn_id: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub filter_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattNextCharacteristicIn>(), 0x4C);

/// GetGattFirstDescriptor input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattFirstDescriptorIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub conn_id: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub filter_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattFirstDescriptorIn>(), 0x4C);

/// GetGattNextDescriptor input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetGattNextDescriptorIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub conn_id: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub desc_id: BtdrvGattId,
    pub filter_uuid: BtdrvGattAttributeUuid,
}

const_assert_eq!(size_of::<GetGattNextDescriptorIn>(), 0x64);

/// ReadGattCharacteristic input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ReadGattCharacteristicIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
}

const_assert_eq!(size_of::<ReadGattCharacteristicIn>(), 0x38);

/// ReadGattDescriptor input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ReadGattDescriptorIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub desc_id: BtdrvGattId,
}

const_assert_eq!(size_of::<ReadGattDescriptorIn>(), 0x50);

/// WriteGattCharacteristic input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteGattCharacteristicIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub with_response: u8,
    pub pad: u8,
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
}

const_assert_eq!(size_of::<WriteGattCharacteristicIn>(), 0x38);

/// WriteGattDescriptor input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteGattDescriptorIn {
    pub is_primary: u8,
    pub auth_req: u8,
    pub pad: [u8; 2],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
    pub desc_id: BtdrvGattId,
}

const_assert_eq!(size_of::<WriteGattDescriptorIn>(), 0x50);

/// GattNotification input (register/unregister).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GattNotificationIn {
    pub is_primary: u8,
    pub pad: [u8; 3],
    pub connection_handle: u32,
    pub serv_id: BtdrvGattId,
    pub char_id: BtdrvGattId,
}

const_assert_eq!(size_of::<GattNotificationIn>(), 0x38);

/// SetBleScanParameter input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetBleScanParameterIn {
    pub scan_interval: u16,
    pub scan_window: u16,
}

const_assert_eq!(size_of::<SetBleScanParameterIn>(), 0x4);

/// StartAudioOut input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct StartAudioOutIn {
    pub audio_handle: u32,
    pub pcm_param: BtdrvPcmParameter,
    pub latency: i64,
}

const_assert_eq!(size_of::<StartAudioOutIn>(), 0x18);

/// StartAudioOut output.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct StartAudioOutOut {
    pub latency: i64,
    pub out1: u64,
}

const_assert_eq!(size_of::<StartAudioOutOut>(), 0x10);
