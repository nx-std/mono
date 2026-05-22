//! Audio control (`audctl`) wire-layout types.

use static_assertions::const_assert_eq;

/// Audio output target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioTarget {
    Invalid = 0,
    Speaker = 1,
    Headphone = 2,
    Tv = 3,
    UsbOutputDevice = 4,
    Bluetooth = 5,
}

impl AudioTarget {
    /// Converts a raw `u32` wire value to an [`AudioTarget`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Invalid),
            1 => Some(Self::Speaker),
            2 => Some(Self::Headphone),
            3 => Some(Self::Tv),
            4 => Some(Self::UsbOutputDevice),
            5 => Some(Self::Bluetooth),
            _ => None,
        }
    }
}

/// Audio output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioOutputMode {
    Invalid = 0,
    Pcm1ch = 1,
    Pcm2ch = 2,
    Pcm6ch = 3,
    PcmAuto = 4,
}

impl AudioOutputMode {
    /// Converts a raw `u32` wire value to an [`AudioOutputMode`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Invalid),
            1 => Some(Self::Pcm1ch),
            2 => Some(Self::Pcm2ch),
            3 => Some(Self::Pcm6ch),
            4 => Some(Self::PcmAuto),
            _ => None,
        }
    }
}

/// Force mute policy (pre-14.0.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioForceMutePolicy {
    Disable = 0,
    SpeakerMuteOnHeadphoneUnplugged = 1,
}

impl AudioForceMutePolicy {
    /// Converts a raw `u32` wire value to an [`AudioForceMutePolicy`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Disable),
            1 => Some(Self::SpeakerMuteOnHeadphoneUnplugged),
            _ => None,
        }
    }
}

/// Headphone output level mode (3.0.0+).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioHeadphoneOutputLevelMode {
    Normal = 0,
    HighPower = 1,
}

impl AudioHeadphoneOutputLevelMode {
    /// Converts a raw `u32` wire value to an [`AudioHeadphoneOutputLevelMode`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Normal),
            1 => Some(Self::HighPower),
            _ => None,
        }
    }
}

/// Wire-layout input for [`SetDefaultTarget`](crate::AudctlService::set_default_target).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetDefaultTargetIn {
    pub target: u32,
    pub _pad: u32,
    pub fade_in_ns: u64,
    pub fade_out_ns: u64,
}

const_assert_eq!(size_of::<SetDefaultTargetIn>(), 0x18);

/// Wire-layout input for [`SetTargetVolume`](crate::AudctlService::set_target_volume).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetTargetVolumeIn {
    pub target: u32,
    pub volume: i32,
}

const_assert_eq!(size_of::<SetTargetVolumeIn>(), 0x8);

/// Wire-layout input for [`SetTargetMute`](crate::AudctlService::set_target_mute).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetTargetMuteIn {
    pub mute: u32,
    pub target: u32,
}

const_assert_eq!(size_of::<SetTargetMuteIn>(), 0x8);

/// Wire-layout input for paired u32 commands (set audio output mode, set
/// output mode setting).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct TargetModeIn {
    pub target: u32,
    pub mode: u32,
}

const_assert_eq!(size_of::<TargetModeIn>(), 0x8);
