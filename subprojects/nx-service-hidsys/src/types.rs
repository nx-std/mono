//! Wire-layout types for the HID System service.

use core::mem::size_of;

use static_assertions::const_assert_eq;

// ---------------------------------------------------------------------------
// Core newtypes
// ---------------------------------------------------------------------------

/// Unique pad identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct UniquePadId {
    pub id: u64,
}

const_assert_eq!(size_of::<UniquePadId>(), 0x8);

/// Unique pad serial number (16-byte fixed-length string).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UniquePadSerialNumber {
    pub serial_number: [u8; 0x10],
}

const_assert_eq!(size_of::<UniquePadSerialNumber>(), 0x10);

/// Bluetooth device address (6 bytes, duplicated per-crate).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtdrvAddress {
    pub address: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvAddress>(), 0x6);

// ---------------------------------------------------------------------------
// Notification LED types
// ---------------------------------------------------------------------------

/// Single mini-cycle in a notification LED pattern.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NotificationLedPatternCycle {
    pub led_intensity: u8,
    pub transition_steps: u8,
    pub final_step_duration: u8,
    pub pad: u8,
}

const_assert_eq!(size_of::<NotificationLedPatternCycle>(), 0x4);

/// Full notification LED pattern configuration.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NotificationLedPattern {
    pub base_mini_cycle_duration: u8,
    pub total_mini_cycles: u8,
    pub total_full_cycles: u8,
    pub start_intensity: u8,
    pub mini_cycles: [NotificationLedPatternCycle; 16],
    pub unk_x44: [u8; 0x2],
    pub pad_x46: [u8; 0x2],
}

const_assert_eq!(size_of::<NotificationLedPattern>(), 0x48);

// ---------------------------------------------------------------------------
// Touch screen configuration
// ---------------------------------------------------------------------------

/// Touch screen configuration (from hid.h).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidTouchScreenConfigurationForNx {
    pub mode: u8,
    pub reserved: [u8; 0xF],
}

const_assert_eq!(size_of::<HidTouchScreenConfigurationForNx>(), 0x10);

// ---------------------------------------------------------------------------
// Button config enums
// ---------------------------------------------------------------------------

/// Digital button assignment target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum HidcfgDigitalButtonAssignment {
    A = 0,
    B = 1,
    X = 2,
    Y = 3,
    StickL = 4,
    StickR = 5,
    L = 6,
    R = 7,
    ZL = 8,
    ZR = 9,
    Select = 10,
    Start = 11,
    Left = 12,
    Up = 13,
    Right = 14,
    Down = 15,
    LeftSL = 16,
    LeftSR = 17,
    RightSL = 18,
    RightSR = 19,
    HomeButton = 20,
    CaptureButton = 21,
    Invalid = 22,
}

/// Analog stick rotation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum HidcfgAnalogStickRotation {
    None = 0,
    Clockwise90 = 1,
    Anticlockwise90 = 2,
}

/// Unique pad hardware type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum UniquePadType {
    Embedded = 0,
    FullKeyController = 1,
    RightController = 2,
    LeftController = 3,
    DebugPadController = 4,
}

// ---------------------------------------------------------------------------
// Button config structs (typed — used by custom/hid button config commands)
// ---------------------------------------------------------------------------

/// Analog stick assignment for button remapping.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgAnalogStickAssignment {
    pub rotation: u32,
    pub is_paired_stick_assigned: u8,
    pub reserved: [u8; 3],
}

const_assert_eq!(size_of::<HidcfgAnalogStickAssignment>(), 0x8);

/// Button configuration for embedded (handheld) controller.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgButtonConfigEmbedded {
    pub hardware_button_left: u32,
    pub hardware_button_up: u32,
    pub hardware_button_right: u32,
    pub hardware_button_down: u32,
    pub hardware_button_a: u32,
    pub hardware_button_b: u32,
    pub hardware_button_x: u32,
    pub hardware_button_y: u32,
    pub hardware_button_stick_l: u32,
    pub hardware_button_stick_r: u32,
    pub hardware_button_l: u32,
    pub hardware_button_r: u32,
    pub hardware_button_zl: u32,
    pub hardware_button_zr: u32,
    pub hardware_button_select: u32,
    pub hardware_button_start: u32,
    pub hardware_button_capture: u32,
    pub hardware_stick_l: HidcfgAnalogStickAssignment,
    pub hardware_stick_r: HidcfgAnalogStickAssignment,
}

const_assert_eq!(size_of::<HidcfgButtonConfigEmbedded>(), 0x54);

/// Button configuration for full (pro) controller.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgButtonConfigFull {
    pub hardware_button_left: u32,
    pub hardware_button_up: u32,
    pub hardware_button_right: u32,
    pub hardware_button_down: u32,
    pub hardware_button_a: u32,
    pub hardware_button_b: u32,
    pub hardware_button_x: u32,
    pub hardware_button_y: u32,
    pub hardware_button_stick_l: u32,
    pub hardware_button_stick_r: u32,
    pub hardware_button_l: u32,
    pub hardware_button_r: u32,
    pub hardware_button_zl: u32,
    pub hardware_button_zr: u32,
    pub hardware_button_select: u32,
    pub hardware_button_start: u32,
    pub hardware_button_capture: u32,
    pub hardware_stick_l: HidcfgAnalogStickAssignment,
    pub hardware_stick_r: HidcfgAnalogStickAssignment,
}

const_assert_eq!(size_of::<HidcfgButtonConfigFull>(), 0x54);

/// Button configuration for left Joy-Con.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgButtonConfigLeft {
    pub hardware_button_left: u32,
    pub hardware_button_up: u32,
    pub hardware_button_right: u32,
    pub hardware_button_down: u32,
    pub hardware_button_stick_l: u32,
    pub hardware_button_l: u32,
    pub hardware_button_zl: u32,
    pub hardware_button_select: u32,
    pub hardware_button_left_sl: u32,
    pub hardware_button_left_sr: u32,
    pub hardware_button_capture: u32,
    pub hardware_stick_l: HidcfgAnalogStickAssignment,
}

const_assert_eq!(size_of::<HidcfgButtonConfigLeft>(), 0x34);

/// Button configuration for right Joy-Con.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgButtonConfigRight {
    pub hardware_button_a: u32,
    pub hardware_button_b: u32,
    pub hardware_button_x: u32,
    pub hardware_button_y: u32,
    pub hardware_button_stick_r: u32,
    pub hardware_button_r: u32,
    pub hardware_button_zr: u32,
    pub hardware_button_start: u32,
    pub hardware_button_right_sl: u32,
    pub hardware_button_right_sr: u32,
    pub hardware_stick_r: HidcfgAnalogStickAssignment,
}

const_assert_eq!(size_of::<HidcfgButtonConfigRight>(), 0x30);

/// Storage name for button config presets (UTF-8 NUL-terminated).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidcfgStorageName {
    pub name: [u8; 0x81],
}

const_assert_eq!(size_of::<HidcfgStorageName>(), 0x81);

// ---------------------------------------------------------------------------
// Legacy opaque button config blobs [10.0.0-10.2.0]
// ---------------------------------------------------------------------------

/// Opaque embedded button config blob (legacy wire format).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidsysButtonConfigEmbedded {
    pub data: [u8; 0x2C8],
}

const_assert_eq!(size_of::<HidsysButtonConfigEmbedded>(), 0x2C8);

/// Opaque full button config blob (legacy wire format).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidsysButtonConfigFull {
    pub data: [u8; 0x2C8],
}

const_assert_eq!(size_of::<HidsysButtonConfigFull>(), 0x2C8);

/// Opaque left button config blob (legacy wire format).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidsysButtonConfigLeft {
    pub data: [u8; 0x1C8],
}

const_assert_eq!(size_of::<HidsysButtonConfigLeft>(), 0x1C8);

/// Opaque right button config blob (legacy wire format).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HidsysButtonConfigRight {
    pub data: [u8; 0x1A0],
}

const_assert_eq!(size_of::<HidsysButtonConfigRight>(), 0x1A0);

// ---------------------------------------------------------------------------
// IPC input structs (pub(crate) — not part of public API)
// ---------------------------------------------------------------------------

/// EnableAppletToGetInput (cmd 503) input.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct EnableAppletToGetInputIn {
    pub permit_input: u8,
    pub pad: [u8; 7],
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<EnableAppletToGetInputIn>(), 0x10);

/// u64 + bool combined input (cmds 1202 legacy, 1273, etc.).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct InU64BoolIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub value: u64,
}

const_assert_eq!(size_of::<InU64BoolIn>(), 0x10);

/// BtdrvAddress + bool combined input (cmds 1204, etc.).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub(crate) struct InAddrBoolIn {
    pub flag: u8,
    pub addr: BtdrvAddress,
}

const_assert_eq!(size_of::<InAddrBoolIn>(), 0x7);

/// SetNotificationLedPattern (cmd 830) input.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetNotificationLedPatternIn {
    pub pattern: NotificationLedPattern,
    pub unique_pad_id: UniquePadId,
}

const_assert_eq!(size_of::<SetNotificationLedPatternIn>(), 0x50);

/// SetNotificationLedPatternWithTimeout (cmd 831) input.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetNotificationLedPatternWithTimeoutIn {
    pub pattern: NotificationLedPattern,
    pub unique_pad_id: UniquePadId,
    pub timeout: u64,
}

const_assert_eq!(size_of::<SetNotificationLedPatternWithTimeoutIn>(), 0x58);

/// IsFirmwareUpdateNeededForNotification (cmd 1154) input.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct IsFirmwareUpdateNeededIn {
    pub val: i32,
    pub pad: u32,
    pub unique_pad_id: UniquePadId,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<IsFirmwareUpdateNeededIn>(), 0x18);

/// GetNpadLeftRightInterfaceType / HasLeftRightBattery output.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LeftRightU8Out {
    pub left: u8,
    pub right: u8,
    pub pad: [u8; 2],
}

const_assert_eq!(size_of::<LeftRightU8Out>(), 0x4);
