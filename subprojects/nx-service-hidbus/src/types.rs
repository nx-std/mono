//! HID Bus service wire-layout types.

use static_assertions::const_assert_eq;

/// Bus type for hidbus devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum BusType {
    LeftJoyRail = 0,
    RightJoyRail = 1,
    /// \[6.0.0+\] RightLarkRail (for microphone).
    RightLarkRail = 2,
}

/// Joy polling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum JoyPollingMode {
    SixAxisSensorDisable = 0,
    SixAxisSensorEnable = 1,
    /// \[6.0.0+\] ButtonOnly.
    ButtonOnly = 2,
}

/// Bus handle identifying a specific hidbus device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BusHandle {
    pub abstracted_pad_id: u32,
    pub internal_index: u8,
    pub player_number: u8,
    pub bus_type_id: u8,
    pub is_valid: u8,
}

const_assert_eq!(size_of::<BusHandle>(), 0x8);

/// Polling received data entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyPollingReceivedData {
    pub data: [u8; 0x30],
    pub out_size: u64,
    pub sampling_number: u64,
}

const_assert_eq!(size_of::<JoyPollingReceivedData>(), 0x40);

/// Data accessor header in shared memory.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DataAccessorHeader {
    pub result: u32,
    pub pad: u32,
    pub unused: [u8; 0x18],
    pub latest_entry: u64,
    pub total_entries: u64,
}

const_assert_eq!(size_of::<DataAccessorHeader>(), 0x30);

/// Entry data for SixAxisSensorDisable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyDisableSixAxisPollingEntryData {
    pub data: [u8; 0x26],
    pub out_size: u8,
    pub pad: u8,
    pub sampling_number: u64,
}

const_assert_eq!(size_of::<JoyDisableSixAxisPollingEntryData>(), 0x30);

/// Ring buffer entry for SixAxisSensorDisable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyDisableSixAxisPollingEntry {
    pub sampling_number: u64,
    pub data: JoyDisableSixAxisPollingEntryData,
}

const_assert_eq!(size_of::<JoyDisableSixAxisPollingEntry>(), 0x38);

/// Entry data for SixAxisSensorEnable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyEnableSixAxisPollingEntryData {
    pub data: [u8; 0x8],
    pub out_size: u8,
    pub pad: [u8; 7],
    pub sampling_number: u64,
}

const_assert_eq!(size_of::<JoyEnableSixAxisPollingEntryData>(), 0x18);

/// Ring buffer entry for SixAxisSensorEnable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyEnableSixAxisPollingEntry {
    pub sampling_number: u64,
    pub data: JoyEnableSixAxisPollingEntryData,
}

const_assert_eq!(size_of::<JoyEnableSixAxisPollingEntry>(), 0x20);

/// Entry data for ButtonOnly polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyButtonOnlyPollingEntryData {
    pub data: [u8; 0x2c],
    pub out_size: u8,
    pub pad: [u8; 3],
    pub sampling_number: u64,
}

const_assert_eq!(size_of::<JoyButtonOnlyPollingEntryData>(), 0x38);

/// Ring buffer entry for ButtonOnly polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyButtonOnlyPollingEntry {
    pub sampling_number: u64,
    pub data: JoyButtonOnlyPollingEntryData,
}

const_assert_eq!(size_of::<JoyButtonOnlyPollingEntry>(), 0x40);

/// Shared-memory data accessor for SixAxisSensorDisable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyDisableSixAxisPollingDataAccessor {
    pub hdr: DataAccessorHeader,
    pub entries: [JoyDisableSixAxisPollingEntry; 0xb],
}

const_assert_eq!(
    size_of::<JoyDisableSixAxisPollingDataAccessor>(),
    0x30 + 0x38 * 0xb
);

/// Shared-memory data accessor for SixAxisSensorEnable polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyEnableSixAxisPollingDataAccessor {
    pub hdr: DataAccessorHeader,
    pub entries: [JoyEnableSixAxisPollingEntry; 0xb],
}

const_assert_eq!(
    size_of::<JoyEnableSixAxisPollingDataAccessor>(),
    0x30 + 0x20 * 0xb
);

/// Shared-memory data accessor for ButtonOnly polling mode.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct JoyButtonOnlyPollingDataAccessor {
    pub hdr: DataAccessorHeader,
    pub entries: [JoyButtonOnlyPollingEntry; 0xb],
}

const_assert_eq!(
    size_of::<JoyButtonOnlyPollingDataAccessor>(),
    0x30 + 0x40 * 0xb
);

/// Common fields for status manager entries.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatusManagerEntryCommon {
    pub is_connected: u8,
    pub pad: [u8; 3],
    pub is_connected_result: u32,
    pub is_enabled: u8,
    pub is_in_focus: u8,
    pub is_polling_mode: u8,
    pub reserved: u8,
    pub polling_mode: u32,
}

const_assert_eq!(size_of::<StatusManagerEntryCommon>(), 0x10);

/// Status manager entry on 5.x.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatusManagerEntryV5 {
    pub common: StatusManagerEntryCommon,
    pub unk: [u8; 0xf0],
}

const_assert_eq!(size_of::<StatusManagerEntryV5>(), 0x100);

/// Status manager entry on 6.0.0+.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatusManagerEntry {
    pub common: StatusManagerEntryCommon,
    pub unk: [u8; 0x70],
}

const_assert_eq!(size_of::<StatusManagerEntry>(), 0x80);

/// Status manager shared memory layout on 5.x.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatusManagerV5 {
    pub entries: [StatusManagerEntryV5; 0x10],
}

const_assert_eq!(size_of::<StatusManagerV5>(), 0x1000);

/// Status manager shared memory layout on 6.0.0+.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StatusManager {
    pub entries: [StatusManagerEntry; 0x13],
    pub unused: [u8; 0x680],
}

const_assert_eq!(size_of::<StatusManager>(), 0x1000);

// --- Wire input structs for IPC commands ---

/// Input for GetBusHandle (cmd 1).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetBusHandleIn {
    pub npad_id: u32,
    pub pad: u32,
    pub bus_type: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetBusHandleIn>(), 0x18);

/// Output for GetBusHandle (cmd 1).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetBusHandleOut {
    pub flag: u8,
    pub pad: [u8; 7],
    pub handle: BusHandle,
}

const_assert_eq!(size_of::<GetBusHandleOut>(), 0x10);

/// Input for Initialize/Finalize (cmds 3, 4).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct BusHandleResIdIn {
    pub handle: BusHandle,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<BusHandleResIdIn>(), 0x10);

/// Input for EnableExternalDevice (cmd 5).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct EnableExternalDeviceIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub handle: BusHandle,
    pub inval: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<EnableExternalDeviceIn>(), 0x20);

/// Input for EnableJoyPollingReceiveMode (cmd 11).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct EnableJoyPollingIn {
    pub tmem_size: u32,
    pub polling_mode: u32,
    pub handle: BusHandle,
}

const_assert_eq!(size_of::<EnableJoyPollingIn>(), 0x10);
