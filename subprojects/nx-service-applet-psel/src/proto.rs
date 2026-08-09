//! Wire structures the player-select applet reads and writes.
//!
//! Every storage here is a fixed-layout payload exchanged verbatim through an
//! `IStorage`, so they are modelled as `repr(C)` structs and converted with
//! zerocopy rather than serialised field by field.
//!
//! # The two settings structs
//!
//! [`UiSettingsV1`] is what the applet read on [1.0.0]; [`UiSettings`] wraps it
//! with the two members [2.0.0+] added. Which of the two is sent is decided by
//! the library applet API version the applet is addressed with, so the outer
//! struct is always built and only its prefix is sent on [1.0.0].
//!
//! # The unknown members
//!
//! `unk_x92`, `unk_x96`, `unk_x97` and `unk_x98` keep their libnx names: the
//! applet reads them, but the only established meaning is which caller sets
//! them to what, which is recorded on each field.

use core::mem::size_of;

use nx_service_acc::{
    AccountUid,
    USER_LIST_SIZE,
};
use static_assertions::const_assert_eq;
use zerocopy::FromZeros as _;

/// Which screen the applet opens on.
///
/// libnx calls this `PselUiMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiMode {
    /// Select one of the users on the console.
    UserSelector = 0,
    /// Create a user.
    UserCreator = 1,
    /// Ensure the selected user has a network service account.
    EnsureNetworkServiceAccountAvailable = 2,
    /// Edit a user's icon.
    UserIconEditor = 3,
    /// Edit a user's nickname.
    UserNicknameEditor = 4,
    /// Create a user, as the starter applet does during console setup.
    UserCreatorForStarter = 5,
    /// Nintendo Account authorization request context.
    NintendoAccountAuthorizationRequestContext = 6,
    /// Introduce an external network service account.
    IntroduceExternalNetworkServiceAccount = 7,
    /// [6.0.0+] Introduce an external network service account, for registration.
    IntroduceExternalNetworkServiceAccountForRegistration = 8,
    /// [6.0.0+] Link a Nintendo Account NNID.
    NintendoAccountNnidLinker = 9,
    /// [6.0.0+] License requirements for a network service.
    LicenseRequirementsForNetworkService = 10,
    /// [7.0.0+] License requirements for a network service, with user context.
    LicenseRequirementsForNetworkServiceWithUserContextImpl = 11,
    /// [7.0.0+] Create a user, for the immediate Nintendo Account login test.
    UserCreatorForImmediateNaLoginTest = 12,
    /// [13.0.0+] Promote a user's qualification.
    UserQualificationPromoter = 13,
}

impl UiMode {
    /// Creates a `UiMode` from the raw value the mode is written as.
    ///
    /// Returns [`None`] when the value names no mode this applet defines.
    #[inline]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::UserSelector),
            1 => Some(Self::UserCreator),
            2 => Some(Self::EnsureNetworkServiceAccountAvailable),
            3 => Some(Self::UserIconEditor),
            4 => Some(Self::UserNicknameEditor),
            5 => Some(Self::UserCreatorForStarter),
            6 => Some(Self::NintendoAccountAuthorizationRequestContext),
            7 => Some(Self::IntroduceExternalNetworkServiceAccount),
            8 => Some(Self::IntroduceExternalNetworkServiceAccountForRegistration),
            9 => Some(Self::NintendoAccountNnidLinker),
            10 => Some(Self::LicenseRequirementsForNetworkService),
            11 => Some(Self::LicenseRequirementsForNetworkServiceWithUserContextImpl),
            12 => Some(Self::UserCreatorForImmediateNaLoginTest),
            13 => Some(Self::UserQualificationPromoter),
            _ => None,
        }
    }

    /// Returns the raw value this mode is written as.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// The message the user-selector screen asks its question with.
///
/// libnx calls this `PselUserSelectionPurpose`. A value the running system does
/// not know is displayed as [`General`](Self::General).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UserSelectionPurpose {
    /// "Select a user."
    General = 0,
    /// [2.0.0+] "Who will receive the points?"
    GameCardRegistration = 1,
    /// [2.0.0+] "Who is using Nintendo eShop?"
    EShopLaunch = 2,
    /// [2.0.0+] "Who is making this purchase?"
    EShopItemShow = 3,
    /// [2.0.0+] "Who is posting?"
    PicturePost = 4,
    /// [2.0.0+] "Select a user to link to a Nintendo Account."
    NintendoAccountLinkage = 5,
    /// [2.0.0+] "Change settings for which user?"
    SettingsUpdate = 6,
    /// [2.0.0+] "Format data for which user?"
    SaveDataDeletion = 7,
    /// [4.0.0+] "Which user will be transferred to another console?"
    UserMigration = 8,
    /// [8.0.0+] "Send save data for which user?"
    SaveDataTransfer = 9,
}

impl UserSelectionPurpose {
    /// Creates a `UserSelectionPurpose` from the raw value it is written as.
    ///
    /// Returns [`None`] when the value names no purpose this applet defines.
    #[inline]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::General),
            1 => Some(Self::GameCardRegistration),
            2 => Some(Self::EShopLaunch),
            3 => Some(Self::EShopItemShow),
            4 => Some(Self::PicturePost),
            5 => Some(Self::NintendoAccountLinkage),
            6 => Some(Self::SettingsUpdate),
            7 => Some(Self::SaveDataDeletion),
            8 => Some(Self::UserMigration),
            9 => Some(Self::SaveDataTransfer),
            _ => None,
        }
    }

    /// Returns the raw value this purpose is written as.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Argument storage the applet read on [1.0.0].
///
/// libnx calls this `PselUiSettingsV1`.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as the
/// libnx struct; the default representation may reorder fields, which would make
/// that borrow read the wrong bytes.
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
pub struct UiSettingsV1 {
    /// Which screen to open, see [`UiMode`].
    pub mode: u32,
    _pad: u32,
    /// The users the screen may not select.
    ///
    /// Only the user-selector screen reads the whole list; every other screen
    /// reads the first entry as its input user, followed by whatever
    /// screen-specific data it defines.
    pub invalid_uid_list: [AccountUid; USER_LIST_SIZE],
    /// The application the selection is made on behalf of.
    pub application_id: u64,
    /// Whether the selected user must be linked to a Nintendo Account.
    pub is_network_service_account_required: u8,
    /// Whether the applet may be skipped when a user can be selected without it.
    pub is_skip_enabled: u8,
    /// Set to 1 by the launcher and system flavours of the user selector.
    pub unk_x92: u8,
    /// Whether the screen offers to create a new user.
    ///
    /// libnx fills this from `accountIsUserRegistrationRequestPermitted`. With
    /// it clear the applet shows a dialog when the user tries to create one.
    pub is_permitted: u8,
    /// Whether the screen offers a button that skips the selection.
    pub show_skip_button: u8,
    /// Whether the screen asks for an additional selection.
    pub additional_select: u8,
    /// [2.0.0+] Whether the user-creation button is enabled.
    ///
    /// The plain and launcher flavours of the user selector set this to 1; the
    /// system flavour passes it through from its caller.
    pub unk_x96: u8,
    /// [6.0.0+] The complement of "an unqualified user may be selected".
    pub unk_x97: u8,
}

const_assert_eq!(size_of::<UiSettingsV1>(), 0x98);

/// Argument storage for the player-select applet.
///
/// libnx calls this `PselUiSettings`. It is [`UiSettingsV1`] plus the two
/// members [2.0.0+] added, and is the struct every screen is built into; a
/// [1.0.0] launch sends only the [`settings`](Self::settings) prefix.
///
/// `repr(C)` for the same reason as [`UiSettingsV1`].
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
pub struct UiSettings {
    /// The members every version of the applet reads.
    pub settings: UiSettingsV1,
    /// [2.0.0+] The message the user-selector screen asks its question with,
    /// see [`UserSelectionPurpose`].
    pub unk_x98: u32,
    _unk_x9c: [u8; 0x4],
}

const_assert_eq!(size_of::<UiSettings>(), 0xA0);

impl UiSettings {
    /// Builds cleared argument storage opening on `mode`.
    ///
    /// libnx calls this `pselUiCreate`. The caller fills the members its screen
    /// reads; see [`PlayerSelect`](crate::PlayerSelect), which does that from
    /// the shape of the request.
    pub fn new(mode: UiMode) -> Self {
        let mut ui = Self::new_zeroed();
        ui.settings.mode = mode.as_raw();
        ui
    }

    /// Records `user` in the first free slot of the user list.
    ///
    /// libnx calls this `pselUiAddUser`. The list is the screen's input user on
    /// every screen but the user selector, where it is the set of users that
    /// may not be picked.
    ///
    /// Does nothing when the list is full, which is what libnx does.
    pub fn add_user(&mut self, user: AccountUid) {
        for slot in &mut self.settings.invalid_uid_list {
            if !slot.is_valid() {
                *slot = user;
                break;
            }
        }
    }
}

/// Reply storage the player-select applet pops back.
///
/// libnx calls this `PselUiReturnArg`.
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
pub struct UiReturnArg {
    /// The applet's own result code; zero means the screen completed.
    pub res: u32,
    _pad: [u8; 0x4],
    /// The user the screen ended on, valid only when `res` is zero.
    pub user_id: AccountUid,
}

const_assert_eq!(size_of::<UiReturnArg>(), 0x18);

/// Caller-supplied settings for the three user-selector screens.
///
/// libnx calls this `PselUserSelectionSettings`. The flag members are the bytes
/// libnx copies rather than Rust `bool`s: a C caller may write any value into
/// them, and only zero and non-zero are distinguished.
///
/// `repr(C)` because the C boundary hands this in by pointer and reads it as the
/// libnx struct; the default representation may reorder fields, which would make
/// that borrow read the wrong bytes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UserSelectionSettings {
    /// The users the screen may not select.
    pub invalid_uid_list: [AccountUid; USER_LIST_SIZE],
    /// Whether the applet may be skipped when a user can be selected without it.
    ///
    /// With this set the first entry of `invalid_uid_list` must be clear and
    /// `additional_select` must be zero; libnx rejects the request otherwise.
    pub is_skip_enabled: u8,
    /// Whether the selected user must be linked to a Nintendo Account.
    pub is_network_service_account_required: u8,
    /// Whether the screen offers a button that skips the selection.
    pub show_skip_button: u8,
    /// Whether the screen asks for an additional selection.
    pub additional_select: u8,
    /// [6.0.0+] Whether a user who does not qualify may still be selected.
    pub is_unqualified_user_selectable: u8,
}

const_assert_eq!(size_of::<UserSelectionSettings>(), 0x88);

/// [2.0.0+] Caller-supplied settings only a system service passes.
///
/// libnx calls this `PselUserSelectionSettingsForSystemService`.
///
/// `repr(C)` for the same reason as [`UserSelectionSettings`].
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UserSelectionSettingsForSystemService {
    /// The message the screen asks its question with, see
    /// [`UserSelectionPurpose`].
    pub purpose: u32,
    /// Whether the user-creation button is enabled.
    ///
    /// Whether pressing it is actually allowed is decided by
    /// [`UiSettingsV1::is_permitted`].
    pub enable_user_creation_button: u8,
    _pad: [u8; 0x3],
}

const_assert_eq!(size_of::<UserSelectionSettingsForSystemService>(), 0x8);

impl UserSelectionSettingsForSystemService {
    /// Builds the settings a system service passes alongside
    /// [`UserSelectionSettings`].
    pub fn new(purpose: UserSelectionPurpose, enable_user_creation_button: bool) -> Self {
        Self {
            purpose: purpose.as_raw(),
            enable_user_creation_button: u8::from(enable_user_creation_button),
            _pad: [0; 0x3],
        }
    }
}

/// Library applet API version the player-select applet is addressed with.
///
/// libnx picks one per running system in `_pselGetLaVersion`. The version also
/// decides how much of [`UiSettings`] the applet reads, which is why it travels
/// with the settings rather than being a constant of the applet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LaVersion {
    /// [1.0.0]: the applet reads [`UiSettingsV1`] and nothing past it.
    V1,
    /// [2.0.0+]: the applet reads the whole of [`UiSettings`].
    V2,
    /// [6.0.0+]: as [`V2`](Self::V2), plus [`UiSettingsV1::unk_x97`].
    V6,
}

impl LaVersion {
    /// Returns the raw version number this version is addressed with.
    #[inline]
    pub(crate) const fn as_raw(self) -> u32 {
        match self {
            Self::V1 => 0x1,
            Self::V2 => 0x10000,
            Self::V6 => 0x20000,
        }
    }
}
