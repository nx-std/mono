//! Wire-layout types for the HID Debug service.

use static_assertions::const_assert_eq;

/// Analog stick state (from HidAnalogStickState).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HidAnalogStickState {
    pub x: i32,
    pub y: i32,
}

const_assert_eq!(size_of::<HidAnalogStickState>(), 0x8);

/// 3D vector (from HidVector).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HidVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

const_assert_eq!(size_of::<HidVector>(), 0xC);

/// Touch screen state (from HidTouchState).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HidTouchState {
    pub delta_time: u64,
    pub attributes: u32,
    pub finger_id: u32,
    pub x: u32,
    pub y: u32,
    pub diameter_x: u32,
    pub diameter_y: u32,
    pub rotation_angle: u32,
    pub reserved: u32,
}

const_assert_eq!(size_of::<HidTouchState>(), 0x28);

/// Unique pad identifier (from HidsysUniquePadId).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct UniquePadId {
    pub id: u64,
}

const_assert_eq!(size_of::<UniquePadId>(), 0x8);

/// State for overriding DebugPad input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct DebugPadAutoPilotState {
    pub attributes: u32,
    pub buttons: u32,
    pub analog_stick_l: HidAnalogStickState,
    pub analog_stick_r: HidAnalogStickState,
}

const_assert_eq!(size_of::<DebugPadAutoPilotState>(), 0x18);

/// State for overriding Mouse input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct MouseAutoPilotState {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub wheel_delta: i32,
    pub buttons: u32,
    pub attributes: u32,
}

const_assert_eq!(size_of::<MouseAutoPilotState>(), 0x1C);

/// State for overriding Keyboard input.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct KeyboardAutoPilotState {
    pub modifiers: u64,
    pub keys: [u64; 4],
}

const_assert_eq!(size_of::<KeyboardAutoPilotState>(), 0x28);

/// State for overriding the Sleep button.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct SleepButtonAutoPilotState {
    pub buttons: u64,
}

const_assert_eq!(size_of::<SleepButtonAutoPilotState>(), 0x8);

/// HDLS handle identifying a virtual controller.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HdlsHandle {
    pub handle: u64,
}

const_assert_eq!(size_of::<HdlsHandle>(), 0x8);

/// HDLS session identifier (returned by AttachHdlsWorkBuffer on 13.0.0+).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HdlsSessionId {
    pub id: u64,
}

const_assert_eq!(size_of::<HdlsSessionId>(), 0x8);

/// HDLS device info for \[7.0.0-8.1.0\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HdlsDeviceInfoV7 {
    pub device_type_internal: u32,
    pub single_color_body: u32,
    pub single_color_buttons: u32,
    pub npad_interface_type: u8,
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<HdlsDeviceInfoV7>(), 0x10);

/// HDLS device info for \[9.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HdlsDeviceInfo {
    pub device_type: u8,
    pub npad_interface_type: u8,
    pub pad: [u8; 2],
    pub single_color_body: u32,
    pub single_color_buttons: u32,
    pub color_left_grip: u32,
    pub color_right_grip: u32,
}

const_assert_eq!(size_of::<HdlsDeviceInfo>(), 0x14);

/// HDLS state for \[7.0.0-8.1.0\].
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HdlsStateV7 {
    pub is_powered: u8,
    pub flags: u8,
    pub unk_x2: [u8; 6],
    pub battery_level: u32,
    pub buttons: u32,
    pub analog_stick_l: HidAnalogStickState,
    pub analog_stick_r: HidAnalogStickState,
    pub indicator: u8,
    pub padding: [u8; 3],
}

const_assert_eq!(size_of::<HdlsStateV7>(), 0x24);

/// HDLS state for \[9.0.0-11.0.1\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HdlsStateV9 {
    pub battery_level: u32,
    pub flags: u32,
    pub buttons: u64,
    pub analog_stick_l: HidAnalogStickState,
    pub analog_stick_r: HidAnalogStickState,
    pub indicator: u8,
    /// Trailing slack: three bytes the wire format names, plus the four the
    /// 8-byte alignment adds.
    pub padding: [u8; 7],
}

const_assert_eq!(size_of::<HdlsStateV9>(), 0x28);

/// HDLS state for \[12.0.0+\].
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HdlsState {
    pub battery_level: u32,
    pub flags: u32,
    pub buttons: u64,
    pub analog_stick_l: HidAnalogStickState,
    pub analog_stick_r: HidAnalogStickState,
    pub six_axis_sensor_acceleration: HidVector,
    pub six_axis_sensor_angle: HidVector,
    pub attribute: u32,
    pub indicator: u8,
    pub padding: [u8; 3],
}

const_assert_eq!(size_of::<HdlsState>(), 0x40);

/// HDLS Npad assignment entry.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsNpadAssignmentEntry {
    pub handle: HdlsHandle,
    pub unk_x8: u32,
    pub unk_xc: u32,
    pub unk_x10: u64,
    pub unk_x18: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<HdlsNpadAssignmentEntry>(), 0x20);

/// HDLS Npad assignment (read/written via transfer memory).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsNpadAssignment {
    pub total_entries: i32,
    pub pad: u32,
    pub entries: [HdlsNpadAssignmentEntry; 0x10],
}

const_assert_eq!(size_of::<HdlsNpadAssignment>(), 0x208);

/// HDLS state list entry for \[7.0.0-8.1.0\].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsStateListEntryV7 {
    pub handle: HdlsHandle,
    pub device: HdlsDeviceInfoV7,
    pub state: HdlsStateV7,
}

const_assert_eq!(size_of::<HdlsStateListEntryV7>(), 0x40);

/// HDLS state list for \[7.0.0-8.1.0\] (read/written via transfer memory).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsStateListV7 {
    pub total_entries: i32,
    pub pad: u32,
    pub entries: [HdlsStateListEntryV7; 0x10],
}

const_assert_eq!(size_of::<HdlsStateListV7>(), 0x408);

/// HDLS state list entry for \[9.0.0-11.0.1\].
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub struct HdlsStateListEntryV9 {
    pub handle: HdlsHandle,
    pub device: HdlsDeviceInfo,
    pub state: HdlsStateV9,
}

const_assert_eq!(size_of::<HdlsStateListEntryV9>(), 0x48);

/// HDLS state list for \[9.0.0-11.0.1\] (read/written via transfer memory).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsStateListV9 {
    pub total_entries: i32,
    pub pad: u32,
    pub entries: [HdlsStateListEntryV9; 0x10],
}

const_assert_eq!(size_of::<HdlsStateListV9>(), 0x488);

/// HDLS state list entry for \[12.0.0+\].
#[derive(Clone, Copy)]
#[repr(C, align(8))]
pub struct HdlsStateListEntry {
    pub handle: HdlsHandle,
    pub device: HdlsDeviceInfo,
    pub state: HdlsState,
}

const_assert_eq!(size_of::<HdlsStateListEntry>(), 0x60);

/// HDLS state list for \[12.0.0+\] (read/written via transfer memory).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HdlsStateList {
    pub total_entries: i32,
    pub pad: u32,
    pub entries: [HdlsStateListEntry; 0x10],
}

const_assert_eq!(size_of::<HdlsStateList>(), 0x608);

/// Abstracted pad handle.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct AbstractedPadHandle {
    pub handle: u64,
}

const_assert_eq!(size_of::<AbstractedPadHandle>(), 0x8);

/// Abstracted pad state.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct AbstractedPadState {
    pub kind: u32,
    pub flags: u8,
    pub pad: [u8; 3],
    pub single_color_body: u32,
    pub single_color_buttons: u32,
    pub npad_interface_type: u8,
    pub pad2: [u8; 3],
    pub state: HdlsStateV7,
    pub unused: [u8; 0x60],
}

const_assert_eq!(size_of::<AbstractedPadState>(), 0x98);

/// Input for UpdateControllerColor (cmd 221).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct UpdateControllerColorIn {
    pub color_body: u32,
    pub color_buttons: u32,
    pub unique_pad_id: UniquePadId,
}

const_assert_eq!(size_of::<UpdateControllerColorIn>(), 0x10);

/// Input for UpdateDesignInfo (cmd 224).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct UpdateDesignInfoIn {
    pub color_body: u32,
    pub color_buttons: u32,
    pub color_left_grip: u32,
    pub color_right_grip: u32,
    pub inval: u8,
    pub pad: [u8; 7],
    pub unique_pad_id: UniquePadId,
}

const_assert_eq!(size_of::<UpdateDesignInfoIn>(), 0x20);

/// Input for ReadSerialFlash (cmd 229).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ReadSerialFlashIn {
    pub offset: u32,
    pub pad: u32,
    pub size: u64,
    pub unique_pad_id: UniquePadId,
}

const_assert_eq!(size_of::<ReadSerialFlashIn>(), 0x18);

/// Input for WriteSerialFlash (cmd 230).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct WriteSerialFlashIn {
    pub offset: u32,
    pub pad: u32,
    pub tmem_size: u64,
    pub size: u64,
    pub unique_pad_id: UniquePadId,
}

const_assert_eq!(size_of::<WriteSerialFlashIn>(), 0x20);

/// Input for SetAutoPilotVirtualPadState (cmd 321).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetAutoPilotVirtualPadIn {
    pub abstracted_virtual_pad_id: i8,
    pub pad: [u8; 7],
    pub state: AbstractedPadState,
}

const_assert_eq!(size_of::<SetAutoPilotVirtualPadIn>(), 0xA0);

/// Input for ApplyHdlsNpadAssignmentState \[13.0.0+\] (cmd 328).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ApplyHdlsNpadAssignmentIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub session_id: HdlsSessionId,
}

const_assert_eq!(size_of::<ApplyHdlsNpadAssignmentIn>(), 0x10);

/// Input for SetHdlsState \[7.0.0-8.1.0\] (cmd 332).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetHdlsStateV7In {
    pub state: HdlsStateV7,
    /// Alignment slack ahead of the 8-byte-aligned handle.
    pub pad: [u8; 4],
    pub handle: HdlsHandle,
}

const_assert_eq!(size_of::<SetHdlsStateV7In>(), 0x30);

/// Input for SetHdlsState \[9.0.0-11.0.1\] (cmd 332).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetHdlsStateV9In {
    pub handle: HdlsHandle,
    pub state: HdlsStateV9,
}

const_assert_eq!(size_of::<SetHdlsStateV9In>(), 0x30);

/// Input for SetHdlsState \[12.0.0+\] (cmd 332).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetHdlsStateIn {
    pub handle: HdlsHandle,
    pub state: HdlsState,
}

const_assert_eq!(size_of::<SetHdlsStateIn>(), 0x48);
