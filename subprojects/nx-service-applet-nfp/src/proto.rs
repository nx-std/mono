//! Wire structures the cabinet applet reads and writes.
//!
//! Both storages are fixed-layout payloads exchanged verbatim through an
//! `IStorage`, so they are modelled as `repr(C)` structs and converted with
//! zerocopy rather than serialised field by field.
//!
//! # On `NfpTagInfo`
//!
//! libnx declares `NfpTagInfo` and `NfcTagInfo` as two typedefs with identical
//! bodies (`nfc.h`), and uses the former here. [`NfcTagInfo`] is that same
//! 0x58-byte layout, so it stands in for both rather than being duplicated.

use core::mem::size_of;

use nx_service_nfc::{
    NfcDeviceHandle,
    NfcTagInfo,
    NfpRegisterInfo,
};
use static_assertions::const_assert_eq;
use zerocopy::FromZeros as _;

/// Which settings screen the applet opens on.
///
/// libnx calls this `NfpLaStartParamTypeForAmiiboSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AmiiboSettingsType {
    /// Edit the amiibo's nickname and owner.
    NicknameAndOwnerSettings = 0,
    /// Erase the game data written to the amiibo.
    GameDataEraser = 1,
    /// Restore the amiibo from a backup.
    Restorer = 2,
    /// Format the amiibo.
    Formatter = 3,
}

impl AmiiboSettingsType {
    /// Returns the raw byte this type is written as.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// Caller-supplied opening parameters, copied into the argument storage.
///
/// libnx calls this `NfpLaAmiiboSettingsStartParam`. Every field is opaque: the
/// applet reads them but no meaning has been established for any of them, so
/// they keep their libnx names and stay byte arrays.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as the
/// libnx struct; the default representation may reorder fields, which would make
/// that borrow read the wrong bytes.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct AmiiboSettingsStartParam {
    /// Copied to `StartParamForAmiiboSettings::unk_x4`.
    pub unk_x0: [u8; 0x8],
    /// Copied to `StartParamForAmiiboSettings::unk_x164`.
    pub unk_x8: [u8; 0x20],
    /// Copied to `StartParamForAmiiboSettings::unk_x3`.
    pub unk_x28: u8,
}

const_assert_eq!(size_of::<AmiiboSettingsStartParam>(), 0x29);

/// Flag bit meaning the request is populated at all.
///
/// libnx sets it unconditionally; a request without it is not one the applet
/// acts on.
pub const START_FLAG_PRESENT: u8 = 1 << 0;

/// Flag bit meaning [`StartParamForAmiiboSettings::tag_info`] is populated.
pub const START_FLAG_TAG_INFO: u8 = 1 << 1;

/// Flag bit meaning [`StartParamForAmiiboSettings::register_info`] is populated.
pub const START_FLAG_REGISTER_INFO: u8 = 1 << 2;

/// Argument storage for the cabinet applet.
///
/// libnx calls this `NfpLaStartParamForAmiiboSettings`.
#[derive(Debug, Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C)]
pub struct StartParamForAmiiboSettings {
    _unk_x0: u8,
    /// Which settings screen to open, see [`AmiiboSettingsType`].
    pub ty: u8,
    /// Which of the optional members below are populated.
    pub flags: u8,
    /// [`AmiiboSettingsStartParam::unk_x28`].
    pub unk_x3: u8,
    /// [`AmiiboSettingsStartParam::unk_x0`].
    pub unk_x4: [u8; 0x8],
    /// The amiibo the applet must match, when [`START_FLAG_TAG_INFO`] is set.
    pub tag_info: NfcTagInfo,
    /// Registration data to write, when [`START_FLAG_REGISTER_INFO`] is set.
    pub register_info: NfpRegisterInfo,
    /// [`AmiiboSettingsStartParam::unk_x8`].
    pub unk_x164: [u8; 0x20],
    _unk_x184: [u8; 0x24],
}

const_assert_eq!(size_of::<StartParamForAmiiboSettings>(), 0x1A8);

impl StartParamForAmiiboSettings {
    /// Builds an argument storage opening on `ty`, with every optional member
    /// cleared and only [`START_FLAG_PRESENT`] set.
    ///
    /// The caller fills the members its request carries and sets the matching
    /// flag; see [`AmiiboSettings`](crate::AmiiboSettings), which does both from
    /// the shape of the request.
    pub fn new(ty: AmiiboSettingsType, start_param: &AmiiboSettingsStartParam) -> Self {
        Self {
            _unk_x0: 0,
            ty: ty.as_raw(),
            flags: START_FLAG_PRESENT,
            unk_x3: start_param.unk_x28,
            unk_x4: start_param.unk_x0,
            tag_info: NfcTagInfo::new_zeroed(),
            register_info: NfpRegisterInfo::new_zeroed(),
            unk_x164: start_param.unk_x8,
            _unk_x184: [0; 0x24],
        }
    }
}

/// Flag bit meaning [`ReturnValueForAmiiboSettings::register_info`] is
/// populated.
pub const RETURN_FLAG_REGISTER_INFO: u8 = 1 << 2;

/// Reply storage the cabinet applet pops back.
///
/// libnx calls this `NfpLaReturnValueForAmiiboSettings`.
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
pub struct ReturnValueForAmiiboSettings {
    /// Zero means the applet failed; any other value means it succeeded, and
    /// [`RETURN_FLAG_REGISTER_INFO`] reports whether `register_info` is set.
    pub flags: u8,
    _pad: [u8; 3],
    /// The device the applet used.
    pub handle: NfcDeviceHandle,
    /// The amiibo the applet acted on.
    pub tag_info: NfcTagInfo,
    /// Registration data, valid only when [`RETURN_FLAG_REGISTER_INFO`] is set.
    pub register_info: NfpRegisterInfo,
    _unk_x164: [u8; 0x24],
}

const_assert_eq!(size_of::<ReturnValueForAmiiboSettings>(), 0x188);
