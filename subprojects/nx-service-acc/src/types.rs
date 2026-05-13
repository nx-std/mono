//! Account service wire-layout types.

use static_assertions::const_assert_eq;

/// Account user ID.
///
/// A 128-bit identifier for a user account. All-zero is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

impl AccountUid {
    /// Returns `true` if this user ID is valid (non-zero).
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.uid[0] != 0 || self.uid[1] != 0
    }
}

/// User data associated with a profile.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountUserData {
    pub unk_x0: u32,
    pub icon_id: u32,
    pub icon_background_color_id: u8,
    pub unk_x9: [u8; 0x7],
    pub mii_id: [u8; 0x10],
    pub unk_x20: [u8; 0x60],
}

const_assert_eq!(size_of::<AccountUserData>(), 0x80);

/// Profile base information.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountProfileBase {
    pub uid: AccountUid,
    pub last_edit_timestamp: u64,
    pub nickname: [u8; 0x20],
}

const_assert_eq!(size_of::<AccountProfileBase>(), 0x38);

/// Network service account identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AccountNetworkServiceAccountId {
    pub id: u64,
}

const_assert_eq!(size_of::<AccountNetworkServiceAccountId>(), 0x08);

/// Maximum number of user profiles the system supports.
pub const USER_LIST_SIZE: usize = 8;

/// Wire-layout input for `InitializeApplicationInfo` (sends PID).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct InitializeApplicationInfoIn {
    pub pid_placeholder: u64,
}

const_assert_eq!(size_of::<InitializeApplicationInfoIn>(), 0x08);

/// Wire-layout input for `IsUserRegistrationRequestPermitted` (sends PID).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct IsUserRegistrationPermittedIn {
    pub pid_placeholder: u64,
}

const_assert_eq!(size_of::<IsUserRegistrationPermittedIn>(), 0x08);
