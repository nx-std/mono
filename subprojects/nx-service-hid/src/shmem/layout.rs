//! HID shared memory layout (0x40000 bytes).
//!
//! This module defines the exact memory layout of the HID shared memory region.
//! All structures must match the official layout exactly for correct operation.

/// Size of the HID shared memory region.
pub const HID_SHARED_MEMORY_SIZE: usize = 0x40000;

/// Placeholder for individual input device sections.
///
/// Each input device (touch, mouse, keyboard, etc.) has its own section in
/// shared memory with LIFO buffers. This is a simplified version.
#[repr(C)]
pub struct HidDebugPadSharedMemoryFormat {
    _data: [u8; 0x400],
}

#[repr(C)]
pub struct HidTouchScreenSharedMemoryFormat {
    _data: [u8; 0x3000],
}

#[repr(C)]
pub struct HidMouseSharedMemoryFormat {
    _data: [u8; 0x400],
}

#[repr(C)]
pub struct HidKeyboardSharedMemoryFormat {
    _data: [u8; 0x400],
}

#[repr(C)]
pub struct HidDigitizerSharedMemoryFormat {
    _data: [u8; 0x400],
}

#[repr(C)]
pub struct HidHomeButtonSharedMemoryFormat {
    _data: [u8; 0x200],
}

#[repr(C)]
pub struct HidSleepButtonSharedMemoryFormat {
    _data: [u8; 0x200],
}

#[repr(C)]
pub struct HidCaptureButtonSharedMemoryFormat {
    _data: [u8; 0x200],
}

#[repr(C)]
pub struct HidInputDetectorSharedMemoryFormat {
    _data: [u8; 0x800],
}

#[repr(C)]
pub struct HidUniquePadSharedMemoryFormat {
    _data: [u8; 0x400],
}

#[repr(C)]
pub struct HidNpadSharedMemoryFormat {
    _data: [u8; 0x32000],
}

#[repr(C)]
pub struct HidGestureSharedMemoryFormat {
    _data: [u8; 0x800],
}

#[repr(C)]
pub struct HidConsoleSixAxisSensor {
    _data: [u8; 0x400],
}

/// HID shared memory structure (0x40000 bytes).
///
/// This contains all input device states in a fixed layout.
#[repr(C)]
pub struct HidSharedMemory {
    pub debug_pad: HidDebugPadSharedMemoryFormat,
    pub touchscreen: HidTouchScreenSharedMemoryFormat,
    pub mouse: HidMouseSharedMemoryFormat,
    pub keyboard: HidKeyboardSharedMemoryFormat,
    pub digitizer: HidDigitizerSharedMemoryFormat,
    pub home_button: HidHomeButtonSharedMemoryFormat,
    pub sleep_button: HidSleepButtonSharedMemoryFormat,
    pub capture_button: HidCaptureButtonSharedMemoryFormat,
    pub input_detector: HidInputDetectorSharedMemoryFormat,
    pub unique_pad: HidUniquePadSharedMemoryFormat,
    pub npad: HidNpadSharedMemoryFormat,
    pub gesture: HidGestureSharedMemoryFormat,
    pub console_six_axis_sensor: HidConsoleSixAxisSensor,
    _padding: [u8; 0x3DE0],
}

impl HidSharedMemory {
    /// Size of the shared memory region.
    pub const SIZE: usize = HID_SHARED_MEMORY_SIZE;
}
