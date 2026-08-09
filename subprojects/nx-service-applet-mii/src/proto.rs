//! Wire structures the Mii editor applet reads and writes.
//!
//! The argument storage and both reply storages are fixed-layout payloads
//! exchanged verbatim through an `IStorage`, so they are modelled as `repr(C)`
//! structs and converted with zerocopy rather than serialised field by field.

use core::mem::{
    offset_of,
    size_of,
};

use nx_service_mii::MiiCharInfo;
use static_assertions::const_assert_eq;
use zerocopy::IntoBytes as _;

/// 128-bit UUID, wire-equivalent to libnx's `typedef struct { u8 uuid[0x10]; } Uuid;`.
///
/// libnx spells this with its workspace-wide `Uuid` typedef. Nothing in this
/// workspace owns that type; each crate that needs one declares its own, so the
/// Mii editor's copy lives here.
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
pub struct Uuid {
    /// The raw 16 bytes, in the order they cross the wire.
    pub bytes: [u8; 0x10],
}

const_assert_eq!(size_of::<Uuid>(), 0x10);

/// Which screen the applet opens on.
///
/// libnx calls this `MiiLaAppletMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AppletMode {
    /// Open the editor on the console's Mii database.
    ShowMiiEdit = 0,
    /// Add a Mii to the database.
    AppendMii = 1,
    /// Add a Mii image.
    AppendMiiImage = 2,
    /// Replace a Mii image.
    UpdateMiiImage = 3,
    /// Create a Mii without saving it. Added in `[10.2.0]`.
    CreateMii = 4,
    /// Edit a Mii without saving it. Added in `[10.2.0]`.
    EditMii = 5,
}

impl AppletMode {
    /// Returns the raw word this mode is written as.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Argument-storage version libnx addresses the applet with below `[10.2.0]`.
///
/// libnx picks between this and [`INPUT_VERSION_V4`] in `_miiLaGetVersion`.
pub const INPUT_VERSION_V3: i32 = 3;

/// Argument-storage version libnx addresses the applet with from `[10.2.0]`.
pub const INPUT_VERSION_V4: i32 = 4;

/// How many entries of the valid-uuid array the applet reads.
pub const VALID_UUID_ARRAY_LEN: usize = 8;

/// Size of the union at offset 0xC of [`AppletInput`].
const PAYLOAD_SIZE: usize = 0x80;

/// The union at offset 0xC of [`AppletInput`].
///
/// libnx declares it as a union of `Uuid valid_uuid_array[8]` and a
/// `{ MiiCharInfo char_info; u8 unused_x64[0x28]; }` struct, both 0x80 bytes.
/// A Rust `union` cannot derive [`zerocopy::IntoBytes`], so the region is held
/// as bytes and each constructor names the case that fills it.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct AppletInputPayload {
    bytes: [u8; PAYLOAD_SIZE],
}

const_assert_eq!(size_of::<AppletInputPayload>(), PAYLOAD_SIZE);
// The char-info case is a Mii followed by 0x28 unused bytes, so the Mii fits
// with room to spare; this is what keeps `char_info` below from panicking.
const_assert_eq!(size_of::<MiiCharInfo>() + 0x28, PAYLOAD_SIZE);

impl AppletInputPayload {
    /// Builds a cleared payload, for the screens that read neither case.
    pub const fn empty() -> Self {
        Self {
            bytes: [0; PAYLOAD_SIZE],
        }
    }

    /// Builds the valid-uuid-array case.
    ///
    /// Entries past [`VALID_UUID_ARRAY_LEN`] are dropped, which is what libnx's
    /// `_miiLaInitializeValidUuidArray` does with a longer array.
    pub fn valid_uuids(uuids: &[Uuid]) -> Self {
        let mut payload = Self::empty();

        // `chunks_exact_mut` yields exactly `VALID_UUID_ARRAY_LEN` slots, so
        // zipping against the input truncates a longer array rather than
        // rejecting it.
        for (slot, uuid) in payload.bytes.chunks_exact_mut(size_of::<Uuid>()).zip(uuids) {
            slot.copy_from_slice(uuid.as_bytes());
        }

        payload
    }

    /// Builds the char-info case, holding the Mii the applet opens on.
    pub fn char_info(char_info: &MiiCharInfo) -> Self {
        let mut payload = Self::empty();

        payload.bytes[..size_of::<MiiCharInfo>()].copy_from_slice(char_info.as_bytes());

        payload
    }
}

/// Argument storage for the Mii editor applet.
///
/// libnx calls this `MiiLaAppletInput`.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct AppletInput {
    /// Argument-storage version, see [`INPUT_VERSION_V3`] and
    /// [`INPUT_VERSION_V4`].
    pub version: i32,
    /// Which screen to open, see [`AppletMode`].
    pub mode: u32,
    /// Which Mii database the applet works against, see
    /// [`MiiSpecialKeyCode`](nx_service_mii::MiiSpecialKeyCode).
    ///
    /// libnx types this `s32` while the only non-zero value it assigns has bit
    /// 31 set; `u32` carries the same bits without a sign-reinterpreting cast.
    pub special_key_code: u32,
    /// The union the screen reads its data from, see [`AppletInputPayload`].
    pub payload: AppletInputPayload,
    /// The Mii image being replaced, read only by
    /// [`AppletMode::UpdateMiiImage`].
    pub used_uuid: Uuid,
    _unk_x9c: [u8; 0x64],
}

const_assert_eq!(size_of::<AppletInput>(), 0x100);
const_assert_eq!(offset_of!(AppletInput, payload), 0x0C);
const_assert_eq!(offset_of!(AppletInput, used_uuid), 0x8C);

impl AppletInput {
    /// Builds an argument storage opening on `mode`.
    ///
    /// `used_uuid` is cleared; only [`AppletMode::UpdateMiiImage`] sets it, and
    /// [`MiiEdit`](crate::MiiEdit) fills it from the shape of the request.
    pub const fn new(
        version: i32,
        mode: AppletMode,
        special_key_code: u32,
        payload: AppletInputPayload,
    ) -> Self {
        Self {
            version,
            mode: mode.as_raw(),
            special_key_code,
            payload,
            used_uuid: Uuid { bytes: [0; 0x10] },
            _unk_x9c: [0; 0x64],
        }
    }
}

/// `res` value meaning the user completed the screen.
pub const RESULT_SUCCESS: u32 = 0;

/// `res` value meaning the user left the screen without completing it.
pub const RESULT_CANCEL: u32 = 1;

/// Reply storage the applet pops back for every screen but the two that edit a
/// Mii without saving it.
///
/// libnx calls this `MiiLaAppletOutput`.
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
pub struct AppletOutput {
    /// [`RESULT_SUCCESS`] or [`RESULT_CANCEL`].
    pub res: u32,
    /// The database index the screen produced.
    ///
    /// Set only on [`RESULT_SUCCESS`], and only for a mode other than
    /// [`AppletMode::ShowMiiEdit`].
    pub index: i32,
    _unk_x8: [u8; 0x18],
}

const_assert_eq!(size_of::<AppletOutput>(), 0x20);

/// Reply storage the applet pops back for the two screens that edit a Mii
/// without saving it.
///
/// libnx calls this `MiiLaAppletOutputForCharInfoEditing`.
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
pub struct AppletOutputForCharInfoEditing {
    /// [`RESULT_SUCCESS`] or [`RESULT_CANCEL`].
    pub res: u32,
    /// The Mii the user made, set only on [`RESULT_SUCCESS`].
    pub char_info: MiiCharInfo,
    _unused: [u8; 0x24],
}

const_assert_eq!(size_of::<AppletOutputForCharInfoEditing>(), 0x80);
const_assert_eq!(offset_of!(AppletOutputForCharInfoEditing, char_info), 0x04);
