//! Wire structures the controller applet reads and writes.
//!
//! Every storage here is a fixed-layout payload exchanged verbatim through an
//! `IStorage`, so they are modelled as `repr(C)` structs and converted with
//! zerocopy rather than serialised field by field.
//!
//! # Two argument layouts
//!
//! The controller-support screens take one of two argument structs:
//! [`ControllerSupportArgV3`] on pre-[8.0.0], [`ControllerSupportArg`] from
//! [8.0.0] on. They differ only in how many players they describe, four against
//! eight; the newer one is what a caller builds, and the older is
//! derived from it by [`ControllerSupportArgV3::from_arg`] when the running
//! system needs it. Which one is sent is chosen by
//! [`ControllerSupportVersion`](crate::ControllerSupportVersion), not here.

use core::mem::size_of;

use static_assertions::const_assert_eq;

/// Which screen the controller applet opens on.
///
/// libnx calls this `HidLaControllerSupportMode`. The gap at 3 is libnx's: no
/// mode has that value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControllerSupportMode {
    /// The controller-pairing screen.
    ShowControllerSupport = 0,
    /// The wrist-strap guide. Requires [3.0.0].
    ShowControllerStrapGuide = 1,
    /// The controller firmware-update screen. Requires [3.0.0].
    ShowControllerFirmwareUpdate = 2,
    /// The system's key-remapping screen. Requires [11.0.0].
    ShowControllerKeyRemappingForSystem = 4,
}

impl ControllerSupportMode {
    /// Returns the raw byte this mode is written as.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// On whose behalf the applet was launched.
///
/// libnx calls this `HidLaControllerSupportCaller`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ControllerSupportCaller {
    /// An application. The firmware-update confirmation dialog is shown.
    #[default]
    Application = 0,
    /// The system. The firmware-update confirmation dialog is skipped, as it is
    /// when the update is started from qlaunch's System Settings.
    System = 1,
}

impl ControllerSupportCaller {
    /// Returns the caller `raw` names, or [`None`] when it names none.
    #[inline]
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Application),
            1 => Some(Self::System),
            _ => None,
        }
    }

    /// Returns the raw byte this caller is written as.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// The first argument storage, describing the request itself.
///
/// libnx calls this `HidLaControllerSupportArgPrivate`. It precedes the
/// screen's own argument storage and says how large that one is.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerSupportArgPrivate {
    /// Size of this struct.
    pub private_size: u32,
    /// Size the following storage is declared to have.
    ///
    /// libnx fills this with the controller-support argument size for *every*
    /// mode, including the firmware-update and key-remapping ones whose own
    /// storage is far smaller. That is reproduced here: the field is a declared
    /// size the applet reads, not the number of bytes actually pushed.
    pub arg_size: u32,
    /// Whether the applet presents itself as qlaunch does.
    ///
    /// Only the system controller-support entry point sets it.
    pub flag0: u8,
    /// Whether the request comes from a system entry point.
    pub flag1: u8,
    /// Which screen to open, see [`ControllerSupportMode`].
    pub mode: u8,
    /// On whose behalf the applet runs, see [`ControllerSupportCaller`].
    ///
    /// Zero for every entry point except the system firmware-update and
    /// key-remapping ones.
    pub controller_support_caller: u8,
    /// The Npad style set the system supports.
    pub npad_style_set: u32,
    /// How the system expects a pair of Joy-Cons to be held.
    pub npad_joy_hold_type: u32,
}

const_assert_eq!(size_of::<ControllerSupportArgPrivate>(), 0x14);

/// Size of this struct, as written into
/// [`ControllerSupportArgPrivate::private_size`].
// Narrowing cast: the `const_assert_eq!` above pins the size at 0x14, so it
// fits in a `u32`.
const PRIVATE_SIZE: u32 = size_of::<ControllerSupportArgPrivate>() as u32;

impl ControllerSupportArgPrivate {
    /// Builds the request storage for `mode`, with every flag cleared.
    ///
    /// The caller sets the flags and the caller byte its entry point uses; see
    /// [`ControllerSupport`](crate::ControllerSupport), which does so from the
    /// shape of the request.
    pub const fn new(mode: ControllerSupportMode, arg_size: u32) -> Self {
        Self {
            private_size: PRIVATE_SIZE,
            arg_size,
            flag0: 0,
            flag1: 0,
            mode: mode.as_raw(),
            controller_support_caller: ControllerSupportCaller::Application.as_raw(),
            npad_style_set: 0,
            npad_joy_hold_type: 0,
        }
    }
}

/// The options every controller-support argument layout begins with.
///
/// libnx calls this `HidLaControllerSupportArgHeader`.
#[derive(Debug, Clone, Copy, Default, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerSupportArgHeader {
    /// Fewest players the caller accepts. Must be at least 0 and at most the
    /// layout's player count.
    pub player_count_min: i8,
    /// Most players the caller accepts. Must be at least 1 and at most the
    /// layout's player count.
    pub player_count_max: i8,
    /// Non-zero to let the user take over controllers already connected
    /// elsewhere. Leaving it zero disconnects them instead.
    pub enable_take_over_connection: u8,
    /// Non-zero to left-justify the player boxes.
    pub enable_left_justify: u8,
    /// Non-zero to permit a dual-Joy-Con player.
    pub enable_permit_joy_dual: u8,
    /// Non-zero to allow a single player in handheld, dual or single mode, with
    /// the player counts above ignored. Handheld mode is refused when this is
    /// zero.
    pub enable_single_mode: u8,
    /// Non-zero to use the identification colours.
    pub enable_identification_color: u8,
}

const_assert_eq!(size_of::<ControllerSupportArgHeader>(), 0x7);

/// The outline colour of one player's box.
///
/// libnx calls this `HidLaControllerSupportArgColor`. It is used only when
/// [`ControllerSupportArgHeader::enable_identification_color`] is set.
#[derive(Debug, Clone, Copy, Default, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerSupportArgColor {
    /// Red component.
    pub r: u8,
    /// Green component.
    pub g: u8,
    /// Blue component.
    pub b: u8,
    /// Alpha component.
    pub a: u8,
}

const_assert_eq!(size_of::<ControllerSupportArgColor>(), 0x4);

/// How many players [`ControllerSupportArg`] describes.
pub const MAX_PLAYERS: usize = 8;

/// How many players [`ControllerSupportArgV3`] describes.
pub const MAX_PLAYERS_V3: usize = 4;

/// Longest explain text a player box holds, excluding the NUL terminator.
pub const EXPLAIN_TEXT_LEN: usize = 0x80;

/// Bytes one player's explain text occupies, including the NUL terminator.
const EXPLAIN_TEXT_SIZE: usize = EXPLAIN_TEXT_LEN + 1;

/// Argument storage for the controller-support screens on [8.0.0+].
///
/// libnx calls this `HidLaControllerSupportArg`. It is the layout a caller
/// builds regardless of the running system; [`ControllerSupportArgV3`] is
/// derived from it when an older system needs the shorter one.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as
/// the libnx struct; the default representation may reorder fields, which would
/// make that borrow read the wrong bytes.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerSupportArg {
    /// The options common to both layouts.
    pub hdr: ControllerSupportArgHeader,
    /// Outline colour per player, used when
    /// [`ControllerSupportArgHeader::enable_identification_color`] is set.
    pub identification_color: [ControllerSupportArgColor; MAX_PLAYERS],
    /// Non-zero to display [`Self::explain_text`].
    pub enable_explain_text: u8,
    /// NUL-terminated UTF-8 text shown in each player's box.
    pub explain_text: [[u8; EXPLAIN_TEXT_SIZE]; MAX_PLAYERS],
}

const_assert_eq!(size_of::<ControllerSupportArg>(), 0x430);

impl ControllerSupportArg {
    /// Builds an argument with libnx's defaults: up to four players, take-over
    /// and left-justify and dual Joy-Con all enabled, no explain text.
    ///
    /// Corresponds to libnx's `hidLaCreateControllerSupportArg`.
    pub const fn new() -> Self {
        Self {
            hdr: ControllerSupportArgHeader {
                player_count_min: 0,
                player_count_max: 4,
                enable_take_over_connection: 1,
                enable_left_justify: 1,
                enable_permit_joy_dual: 1,
                enable_single_mode: 0,
                enable_identification_color: 0,
            },
            identification_color: [ControllerSupportArgColor {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            }; MAX_PLAYERS],
            enable_explain_text: 0,
            explain_text: [[0; EXPLAIN_TEXT_SIZE]; MAX_PLAYERS],
        }
    }

    /// Sets the text shown in `player`'s box, replacing whatever was there.
    ///
    /// [`Self::enable_explain_text`] must also be set, or the applet ignores the
    /// text.
    ///
    /// Text longer than [`EXPLAIN_TEXT_LEN`] bytes is truncated. libnx
    /// truncates by bytes, which can leave half a multi-byte character in the
    /// buffer; this cuts at the last character boundary that fits, so the applet
    /// never receives a malformed sequence.
    ///
    /// # Errors
    ///
    /// Returns a [`SetExplainTextError`] when `player` is not a player slot this
    /// layout describes.
    pub fn set_explain_text(
        &mut self,
        player: usize,
        text: &str,
    ) -> Result<(), SetExplainTextError> {
        let slot = self
            .explain_text
            .get_mut(player)
            .ok_or(SetExplainTextError(player))?;

        let mut len = text.len().min(EXPLAIN_TEXT_LEN);
        while !text.is_char_boundary(len) {
            len -= 1;
        }

        slot.fill(0);
        // Neither slicing can panic: `len` starts at `EXPLAIN_TEXT_LEN`, one
        // below `slot.len()`, or at `text.len()`, and only shrinks from there.
        slot[..len].copy_from_slice(&text.as_bytes()[..len]);

        Ok(())
    }
}

/// Error returned by [`ControllerSupportArg::set_explain_text`], carrying the
/// index that named no player box.
#[derive(Debug, thiserror::Error)]
#[error("no such player: {0}")]
pub struct SetExplainTextError(pub usize);

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for SetExplainTextError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        // Rejected at the boundary before any request went out, which is the
        // code libnx returns for it too.
        nx_sf::error::libnx_error(nx_sf::error::LibnxError::BadInput)
    }
}

impl Default for ControllerSupportArg {
    fn default() -> Self {
        Self::new()
    }
}

/// Argument storage for the controller-support screens on pre-[8.0.0].
///
/// libnx calls this `HidLaControllerSupportArgV3`. It is the same layout as
/// [`ControllerSupportArg`] with four player slots instead of eight, and is
/// only ever built by converting one.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerSupportArgV3 {
    /// The options common to both layouts.
    pub hdr: ControllerSupportArgHeader,
    /// Outline colour per player.
    pub identification_color: [ControllerSupportArgColor; MAX_PLAYERS_V3],
    /// Non-zero to display [`Self::explain_text`].
    pub enable_explain_text: u8,
    /// NUL-terminated UTF-8 text shown in each player's box.
    pub explain_text: [[u8; EXPLAIN_TEXT_SIZE]; MAX_PLAYERS_V3],
}

const_assert_eq!(size_of::<ControllerSupportArgV3>(), 0x21C);

/// Narrows an eight-player argument to the four players this layout describes,
/// dropping the rest.
///
/// The player counts are clamped to four as well: a system that reads this
/// layout has no fifth player to give, and libnx clamps them for the same
/// reason.
impl From<&ControllerSupportArg> for ControllerSupportArgV3 {
    fn from(arg: &ControllerSupportArg) -> Self {
        let mut hdr = arg.hdr;
        // Narrowing cast: `MAX_PLAYERS_V3` is the literal 4, so it fits in an
        // `i8`.
        let max = MAX_PLAYERS_V3 as i8;
        hdr.player_count_min = hdr.player_count_min.min(max);
        hdr.player_count_max = hdr.player_count_max.min(max);

        let mut identification_color = [ControllerSupportArgColor::default(); MAX_PLAYERS_V3];
        identification_color.copy_from_slice(&arg.identification_color[..MAX_PLAYERS_V3]);

        let mut explain_text = [[0; EXPLAIN_TEXT_SIZE]; MAX_PLAYERS_V3];
        explain_text.copy_from_slice(&arg.explain_text[..MAX_PLAYERS_V3]);

        Self {
            hdr,
            identification_color,
            enable_explain_text: arg.enable_explain_text,
            explain_text,
        }
    }
}

/// Argument storage for the firmware-update screen.
///
/// libnx calls this `HidLaControllerFirmwareUpdateArg`.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as
/// the libnx struct.
#[derive(Debug, Clone, Copy, Default, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerFirmwareUpdateArg {
    /// Non-zero to force the update through, leaving the user no way to skip
    /// it.
    pub enable_force_update: u8,
    _pad: [u8; 3],
}

const_assert_eq!(size_of::<ControllerFirmwareUpdateArg>(), 0x4);

/// Argument storage for the key-remapping screen.
///
/// libnx calls this `HidLaControllerKeyRemappingArg`. Both members are opaque:
/// the applet reads them but no meaning has been established for either, so they
/// keep their libnx names.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as
/// the libnx struct.
#[derive(Debug, Clone, Copy, Default, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ControllerKeyRemappingArg {
    /// Unknown.
    pub unk_x0: u64,
    /// Unknown.
    pub unk_x8: u32,
    _pad: [u8; 4],
}

const_assert_eq!(size_of::<ControllerKeyRemappingArg>(), 0x10);

/// What the applet reported about the controllers the user chose.
///
/// libnx calls this `HidLaControllerSupportResultInfo`. The applet returns it
/// for every screen, though only the controller-support ones fill it with
/// anything.
///
/// `repr(C)` because the C boundary hands a pointer to one out and reads it as
/// the libnx struct.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct ControllerSupportResultInfo {
    /// How many players the user set up.
    pub player_count: i8,
    _pad: [u8; 3],
    /// Which controller the user selected, as a libnx `HidNpadIdType`.
    pub selected_id: u32,
}

const_assert_eq!(size_of::<ControllerSupportResultInfo>(), 0x8);

/// The reply storage the applet pops back.
///
/// libnx calls this `HidLaControllerSupportResultInfoInternal`.
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
pub struct ControllerSupportResultInfoInternal {
    /// What the user chose.
    pub info: ControllerSupportResultInfo,
    /// Zero when the applet completed. libnx distinguishes 2 from the other
    /// non-zero values; official software does too, and neither says what they
    /// mean.
    pub res: u32,
}

const_assert_eq!(size_of::<ControllerSupportResultInfoInternal>(), 0xC);
