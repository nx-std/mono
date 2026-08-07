//! NFC/NFP/Mifare wire-layout types.

use bitflags::bitflags;
pub use nx_service_mii::{
    MiiCharInfo,
    MiiNfpStoreDataExtension,
    MiiStoreData,
    MiiVer3StoreData,
};
use static_assertions::const_assert_eq;

/// NFP service type — determines which service name to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfpServiceType {
    User,
    Debug,
    System,
}

/// NFC service type — determines which service name to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfcServiceType {
    User,
    System,
}

/// NFC state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfcState {
    NonInitialized = 0,
    Initialized = 1,
}

/// NFP device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfpDeviceState {
    Initialized = 0,
    SearchingForTag = 1,
    TagFound = 2,
    TagRemoved = 3,
    TagMounted = 4,
    Unavailable = 5,
}

/// NFC device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfcDeviceState {
    Initialized = 0,
    SearchingForTag = 1,
    TagFound = 2,
    TagRemoved = 3,
    TagMounted = 4,
}

/// NFC Mifare device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfcMifareDeviceState {
    Initialized = 0,
    SearchingForTag = 1,
    TagFound = 2,
    TagRemoved = 3,
    TagMounted = 4,
    Unavailable = 5,
}

/// NFP application area version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NfpApplicationAreaVersion {
    Nintendo3ds = 0,
    WiiU = 1,
    Nintendo3dsV2 = 2,
    Switch = 3,
    Invalid = 0xFF,
}

/// NFP device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfpDeviceType {
    Amiibo = 0,
}

/// Mifare command type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NfcMifareCommand {
    Read = 0x30,
    AuthA = 0x60,
    AuthB = 0x61,
    Write = 0xA0,
    Transfer = 0xB0,
    Decrement = 0xC0,
    Increment = 0xC1,
    Store = 0xC2,
}

/// NFP break type (debug only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NfpBreakType {
    Flush = 0,
    Break1 = 1,
    Break2 = 2,
}

bitflags! {
    /// NFP mount target flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NfpMountTarget: u32 {
        const ROM = 1 << 0;
        const RAM = 1 << 1;
        const ALL = Self::ROM.bits() | Self::RAM.bits();
    }
}

bitflags! {
    /// NFC protocol flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NfcProtocol: u32 {
        const TYPE_A = 1 << 0;
        const TYPE_B = 1 << 1;
        const TYPE_F = 1 << 2;
        const ALL = 0xFFFF_FFFF;
    }
}

bitflags! {
    /// NFC tag type flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NfcTagType: u32 {
        const TYPE1 = 1 << 0;
        const TYPE2 = 1 << 1;
        const TYPE3 = 1 << 2;
        const TYPE4A = 1 << 3;
        const TYPE4B = 1 << 4;
        const TYPE5 = 1 << 5;
        const MIFARE = 1 << 6;
        const ALL = 0xFFFF_FFFF;
    }
}

bitflags! {
    /// NFP amiibo flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NfpAmiiboFlag: u8 {
        const VALID = 1 << 0;
        const APPLICATION_AREA_EXISTS = 1 << 1;
    }
}

/// NFP date.
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
pub struct NfpDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

const_assert_eq!(size_of::<NfpDate>(), 0x04);

/// NFC tag UID.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfcTagUid {
    pub uid: [u8; 10],
    pub uid_length: u8,
    pub reserved: [u8; 0x15],
}

const_assert_eq!(size_of::<NfcTagUid>(), 0x20);

/// Tag info shared by NFP and NFC (identical wire layout).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfcTagInfo {
    pub uid: NfcTagUid,
    pub protocol: u32,
    pub tag_type: u32,
    pub reserved: [u8; 0x30],
}

const_assert_eq!(size_of::<NfcTagInfo>(), 0x58);

/// Alias: libnx uses `NfpTagInfo` for NFP and `NfcTagInfo` for NFC,
/// but the wire layout is identical.
pub type NfpTagInfo = NfcTagInfo;

/// NFP common info (requires Ram mount).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpCommonInfo {
    pub last_write_date: NfpDate,
    pub write_counter: u16,
    pub version: u16,
    pub application_area_size: u32,
    pub reserved: [u8; 0x34],
}

const_assert_eq!(size_of::<NfpCommonInfo>(), 0x40);

/// NFP model info (requires Rom mount).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpModelInfo {
    pub character_id: [u8; 3],
    pub series_id: u8,
    pub numbering_id: u16,
    pub nfp_type: u8,
    pub reserved: [u8; 0x39],
}

const_assert_eq!(size_of::<NfpModelInfo>(), 0x40);

/// NFP register info (requires Ram mount).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpRegisterInfo {
    pub mii: MiiCharInfo,
    pub first_write_date: NfpDate,
    pub amiibo_name: [u8; 41],
    pub font_region: u8,
    pub reserved: [u8; 0x7A],
}

const_assert_eq!(size_of::<NfpRegisterInfo>(), 0x100);

/// NFP register info private (system/debug only, requires Ram mount).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpRegisterInfoPrivate {
    pub mii_store_data: MiiStoreData,
    pub first_write_date: NfpDate,
    pub amiibo_name: [u8; 41],
    pub font_region: u8,
    pub reserved: [u8; 0x8E],
}

const_assert_eq!(size_of::<NfpRegisterInfoPrivate>(), 0x100);

/// NFP admin info (system/debug only).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpAdminInfo {
    pub application_id: u64,
    pub access_id: u32,
    pub crc32_change_counter: u16,
    pub flags: u8,
    pub tag_type: u8,
    pub application_area_version: u8,
    pub reserved: [u8; 0x2F],
}

const_assert_eq!(size_of::<NfpAdminInfo>(), 0x40);

/// Full NFP data (debug only, requires Ram mount).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct NfpData {
    pub tag_magic: u8,
    pub reserved1: [u8; 0x1],
    pub tag_write_counter: u16,
    pub crc32_1: u32,
    pub reserved2: [u8; 0x38],
    pub last_write_date: NfpDate,
    pub write_counter: u16,
    pub version: u16,
    pub application_area_size: u32,
    pub reserved3: [u8; 0x34],
    pub mii_v3: MiiVer3StoreData,
    pub pad: [u8; 0x2],
    pub mii_v3_crc16: u16,
    pub mii_store_data_extension: MiiNfpStoreDataExtension,
    pub first_write_date: NfpDate,
    pub amiibo_name: [u16; 11],
    pub font_region: u8,
    pub unknown1: u8,
    pub crc32_2: u32,
    pub unknown2: [u32; 5],
    pub reserved4: [u8; 0x64],
    pub application_id: u64,
    pub access_id: u32,
    pub settings_crc32_change_counter: u16,
    pub flags: u8,
    pub tag_type: u8,
    pub application_area_version: u8,
    pub application_id_byte: u8,
    pub reserved5: [u8; 0x2E],
    pub application_area: [u8; 0xD8],
}

const_assert_eq!(size_of::<NfpData>(), 0x298);

/// NFC device handle.
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
pub struct NfcDeviceHandle {
    pub handle: [u8; 0x8],
}

const_assert_eq!(size_of::<NfcDeviceHandle>(), 0x08);

/// NFC sector key for Mifare operations.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C, packed)]
pub struct NfcSectorKey {
    pub mifare_command: u8,
    pub unknown: u8,
    pub reserved1: [u8; 0x6],
    pub sector_key: [u8; 0x6],
    pub reserved2: [u8; 0x2],
}

const_assert_eq!(size_of::<NfcSectorKey>(), 0x10);

/// Mifare read block parameter.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C, packed)]
pub struct NfcMifareReadBlockParameter {
    pub sector_number: u8,
    pub reserved: [u8; 0x7],
    pub sector_key: NfcSectorKey,
}

const_assert_eq!(size_of::<NfcMifareReadBlockParameter>(), 0x18);

/// Mifare read block data (output).
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C, packed)]
pub struct NfcMifareReadBlockData {
    pub data: [u8; 0x10],
    pub sector_number: u8,
    pub reserved: [u8; 0x7],
}

const_assert_eq!(size_of::<NfcMifareReadBlockData>(), 0x18);

/// Mifare write block parameter.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct NfcMifareWriteBlockParameter {
    pub data: [u8; 0x10],
    pub sector_number: u8,
    pub reserved: [u8; 0x7],
    pub sector_key: NfcSectorKey,
}

const_assert_eq!(size_of::<NfcMifareWriteBlockParameter>(), 0x28);

/// Required MCU version data (sent during initialization).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct NfcRequiredMcuVersionData {
    pub version: u64,
    pub reserved: [u64; 3],
}

const_assert_eq!(size_of::<NfcRequiredMcuVersionData>(), 0x20);

/// Input for NFP Mount command.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct MountIn {
    pub handle: NfcDeviceHandle,
    pub device_type: u32,
    pub mount_target: u32,
}

const_assert_eq!(size_of::<MountIn>(), 0x10);

/// Input for NFP OpenApplicationArea / CreateApplicationArea / RecreateApplicationArea.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct DeviceHandleAppIdIn {
    pub handle: NfcDeviceHandle,
    pub app_id: u32,
}

const_assert_eq!(size_of::<DeviceHandleAppIdIn>(), 0x0C);

/// Input for NFP BreakTag.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct BreakTagIn {
    pub handle: NfcDeviceHandle,
    pub break_type: u32,
}

const_assert_eq!(size_of::<BreakTagIn>(), 0x0C);

/// Input for NFP WriteNtf.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteNtfIn {
    pub handle: NfcDeviceHandle,
    pub write_type: u32,
}

const_assert_eq!(size_of::<WriteNtfIn>(), 0x0C);

/// Input for NFC StartDetection (4.0.0+ — device handle + protocol).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct NfcStartDetectionIn {
    pub handle: NfcDeviceHandle,
    pub protocol: u32,
}

const_assert_eq!(size_of::<NfcStartDetectionIn>(), 0x0C);

/// Input for NFC SendCommandByPassThrough (device handle + timeout).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SendCommandByPassThroughIn {
    pub handle: NfcDeviceHandle,
    pub timeout: u64,
}

const_assert_eq!(size_of::<SendCommandByPassThroughIn>(), 0x10);

/// Input for interface initialization (ARUID + zero).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct InitializeIn {
    pub aruid: u64,
    pub zero: u64,
}

const_assert_eq!(size_of::<InitializeIn>(), 0x10);
