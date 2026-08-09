//! Friends service wire-layout types.

use static_assertions::const_assert_eq;

/// Account user ID (matches libnx's `AccountUid`).
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
pub struct AccountUid {
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

/// In-app screen name for friend invitation.
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
pub struct InAppScreenName {
    pub name: [u8; 0x40],
    pub language_code: u64,
}

const_assert_eq!(size_of::<InAppScreenName>(), 0x48);

/// Friend invitation game mode description.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct FriendInvitationGameModeDescription {
    pub data: [u8; 0xC00],
}

const_assert_eq!(size_of::<FriendInvitationGameModeDescription>(), 0xC00);

/// Friend invitation ID.
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
pub struct FriendInvitationId {
    pub id: u64,
}

const_assert_eq!(size_of::<FriendInvitationId>(), 0x8);

/// Friend invitation group ID.
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
pub struct FriendInvitationGroupId {
    pub id: u64,
}

const_assert_eq!(size_of::<FriendInvitationGroupId>(), 0x8);

/// User setting returned by the friends service.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct FriendsUserSetting {
    pub uid: AccountUid,
    pub presence_permission: u32,
    pub play_log_permission: u32,
    pub friend_request_reception: u64,
    pub friend_code: [u8; 0x20],
    pub friend_code_next_issuable_time: u64,
    pub reserved: [u8; 0x7C8],
}

const_assert_eq!(size_of::<FriendsUserSetting>(), 0x810);
