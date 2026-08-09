//! Wire structures the MyPage applet reads, and the bounds it imposes on them.
//!
//! The applet takes one fixed-layout argument storage, in one of two layouts:
//! [`ArgV1`] before \[9.0.0\] and [`Arg`] from \[9.0.0\] on. Both are exchanged
//! verbatim through an `IStorage`, so they are modelled as `repr(C)` structs and
//! converted with zerocopy rather than serialised field by field.
//!
//! Two of the payloads only accept a value inside a range the buffer they are
//! copied into fixes: at most 15 invitees, and fewer than 0x400 bytes of
//! user-data. Those ranges belong to the wire form, so [`InviteeCount`],
//! [`InviteeList`] and [`InvitationUserData`] are declared here beside the
//! buffers they bound, and the payload constructors accept nothing else.
//!
//! # On the argument union
//!
//! libnx declares the \[9.0.0+\] payload as a union of a raw `u8[0x1090]` and
//! four typed members. [`Arg`] keeps the raw array and the typed members stand
//! beside it as their own structs, each written into the array by the matching
//! constructor. A Rust `union` would buy nothing here: every read of it would be
//! `unsafe`, and the applet only ever reads the one member the header's type
//! selects.

use core::mem::size_of;

use nx_service_acc::{
    AccountNetworkServiceAccountId,
    AccountUid,
};
use nx_service_friends::{
    FriendInvitationGameModeDescription,
    FriendInvitationGroupId,
    FriendInvitationId,
    InAppScreenName,
};
use static_assertions::{
    const_assert,
    const_assert_eq,
};
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

/// Which screen the applet opens on.
///
/// libnx calls this `FriendsLaArgType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ArgType {
    /// Launches with the "Friend List" menu initially selected.
    ShowFriendList = 0,
    /// Shows another account's detail page.
    ShowUserDetailInfo = 1,
    /// Sends a friend request to another account.
    StartSendingFriendRequest = 2,
    /// Launches with the "Add Friend" menu initially selected.
    ShowMethodsOfSendingFriendRequest = 3,
    /// Launches on "Search for Local Users"; leaving that menu exits the applet.
    StartFacedFriendRequest = 4,
    /// Launches on "Received Friend Requests"; leaving that menu exits the applet.
    ShowReceivedFriendRequestList = 5,
    /// Launches on the "Blocked-User List"; leaving that menu exits the applet.
    ShowBlockedUserList = 6,
    /// Launches with the "Profile" menu initially selected.
    ShowMyProfile = 7,
    /// \[9.0.0+\] Picks friends to invite to online play through the applet's UI.
    StartFriendInvitation = 8,
    /// \[9.0.0+\] Sends an online-play invitation to a named list of friends.
    StartSendingFriendInvitation = 9,
    /// \[9.0.0+\] Shows the detail page of a received invitation.
    ShowReceivedInvitationDetail = 10,
}

impl ArgType {
    /// Returns the raw value this type is written as.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Header shared by both argument layouts.
///
/// libnx calls this `FriendsLaArgHeader`.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ArgHeader {
    /// Which screen to open, see [`ArgType`].
    pub ty: u32,
    _pad: u32,
    /// The account the applet acts as.
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<ArgHeader>(), 0x18);

impl ArgHeader {
    /// Builds a header opening `ty` as `uid`.
    pub const fn new(ty: ArgType, uid: AccountUid) -> Self {
        Self {
            ty: ty.as_raw(),
            _pad: 0,
            uid,
        }
    }
}

/// The payload every pre-\[9.0.0\] screen shares.
///
/// libnx calls this `FriendsLaArgCommonData`. Only
/// [`ArgType::ShowUserDetailInfo`] and [`ArgType::StartSendingFriendRequest`]
/// populate it; every other screen leaves it cleared.
#[derive(Debug, Clone, Copy, zerocopy::FromZeros, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ArgCommonData {
    /// The other account the screen is about.
    pub id: AccountNetworkServiceAccountId,
    /// First in-app screen name.
    pub first_in_app_screen_name: InAppScreenName,
    /// Second in-app screen name.
    pub second_in_app_screen_name: InAppScreenName,
}

const_assert_eq!(size_of::<ArgCommonData>(), 0x98);

/// Argument storage for the applet, before \[9.0.0\].
///
/// libnx calls this `FriendsLaArgV1`. It can only express the screens that fit
/// [`ArgCommonData`], which is why the \[9.0.0+\] screens have no pre-\[9.0.0\]
/// form.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ArgV1 {
    /// Which screen to open, and as whom.
    pub hdr: ArgHeader,
    /// The screen's payload, cleared when the screen carries none.
    pub data: ArgCommonData,
}

const_assert_eq!(size_of::<ArgV1>(), 0xB0);

impl ArgV1 {
    /// Builds the argument storage for `ty`, carrying `data`.
    pub const fn new(ty: ArgType, uid: AccountUid, data: ArgCommonData) -> Self {
        Self {
            hdr: ArgHeader::new(ty, uid),
            data,
        }
    }
}

/// Size of the argument union, and so of every payload written into it.
pub const ARG_DATA_SIZE: usize = 0x1090;

/// Argument storage for the applet, from \[9.0.0\] on.
///
/// libnx calls this `FriendsLaArg`. The payload is the union described in the
/// [module docs](self); use the constructor matching the screen rather than
/// writing [`Arg::data`] by hand.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct Arg {
    /// Which screen to open, and as whom.
    pub hdr: ArgHeader,
    /// The selected union member, followed by zeroes.
    pub data: [u8; ARG_DATA_SIZE],
}

const_assert_eq!(size_of::<Arg>(), 0x10A8);

impl Arg {
    /// Builds the argument storage for a screen carrying [`ArgCommonData`].
    pub fn new_common(ty: ArgType, uid: AccountUid, data: &ArgCommonData) -> Self {
        Self::new_with_data(ty, uid, data.as_bytes())
    }

    /// Builds the argument storage for [`ArgType::StartFriendInvitation`].
    pub fn new_start_friend_invitation(uid: AccountUid, data: &StartFriendInvitationData) -> Self {
        Self::new_with_data(ArgType::StartFriendInvitation, uid, data.as_bytes())
    }

    /// Builds the argument storage for [`ArgType::StartSendingFriendInvitation`].
    pub fn new_start_sending_friend_invitation(
        uid: AccountUid,
        data: &StartSendingFriendInvitationData,
    ) -> Self {
        Self::new_with_data(ArgType::StartSendingFriendInvitation, uid, data.as_bytes())
    }

    /// Builds the argument storage for [`ArgType::ShowReceivedInvitationDetail`].
    pub fn new_show_received_invitation_detail(
        uid: AccountUid,
        data: &ShowReceivedInvitationDetailData,
    ) -> Self {
        Self::new_with_data(ArgType::ShowReceivedInvitationDetail, uid, data.as_bytes())
    }

    /// Builds the argument storage for `ty`, with `data` at the front of the
    /// union and the rest cleared.
    ///
    /// Private because the pairing of `ty` with the union member is what makes
    /// the storage readable by the applet; each constructor above fixes both.
    fn new_with_data(ty: ArgType, uid: AccountUid, data: &[u8]) -> Self {
        let mut arg = Self {
            hdr: ArgHeader::new(ty, uid),
            data: [0; ARG_DATA_SIZE],
        };

        // The four callers above each pass one of the payload structs below, and
        // every one of them is const-asserted to be at most `ARG_DATA_SIZE`
        // bytes, so this slice index cannot be out of range.
        arg.data[..data.len()].copy_from_slice(data);

        arg
    }
}

/// Number of [`AccountNetworkServiceAccountId`] entries the invitee list holds.
///
/// The applet accepts at most 15 of them, see [`InviteeCount`]; the sixteenth
/// slot exists in the layout and is never filled.
pub const ID_LIST_CAPACITY: usize = 16;

/// Size of the user-data buffer carried with an invitation.
pub const USER_DATA_CAPACITY: usize = 0x400;

/// Smallest number of invitees the applet accepts.
const MIN_INVITEES: i32 = 1;

/// Largest number of invitees the applet accepts.
const MAX_INVITEES: i32 = 15;

/// How many friends an invitation reaches, between 1 and 15.
///
/// The applet rejects anything outside that range, so the range is checked once
/// here rather than at every place the count is passed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InviteeCount(i32);

impl InviteeCount {
    /// Returns the raw count the argument storage carries.
    #[inline]
    pub const fn as_raw(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for InviteeCount {
    type Error = InviteeCountError;

    fn try_from(count: i32) -> Result<Self, Self::Error> {
        match count {
            MIN_INVITEES..=MAX_INVITEES => Ok(Self(count)),
            _ => Err(InviteeCountError),
        }
    }
}

/// Error returned when converting a count of invitees the applet would reject.
#[derive(Debug, thiserror::Error)]
#[error("the number of invitees must be between 1 and 15")]
pub struct InviteeCountError;

/// The accounts an invitation is sent to, between 1 and 15 of them.
///
/// The list travels with its own length, so wrapping it keeps the two from
/// disagreeing as well as bounding the length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InviteeList<'a>(&'a [AccountNetworkServiceAccountId]);

impl InviteeList<'_> {
    /// Returns how many accounts the invitation reaches.
    #[inline]
    pub const fn count(&self) -> InviteeCount {
        // Narrowing cast: the length passed the same range check when this list
        // was built, so it is at most `MAX_INVITEES`.
        InviteeCount(self.0.len() as i32)
    }

    /// Returns the accounts the invitation reaches.
    #[inline]
    pub const fn as_slice(&self) -> &[AccountNetworkServiceAccountId] {
        self.0
    }
}

impl<'a> TryFrom<&'a [AccountNetworkServiceAccountId]> for InviteeList<'a> {
    type Error = InviteeCountError;

    fn try_from(list: &'a [AccountNetworkServiceAccountId]) -> Result<Self, Self::Error> {
        // A list too long to count in an `i32` is one the applet would reject
        // anyway, so the conversion failing is the same error as the range check
        // below failing.
        let count = i32::try_from(list.len()).map_err(|_| InviteeCountError)?;
        InviteeCount::try_from(count)?;

        Ok(Self(list))
    }
}

/// Arbitrary data carried with an invitation, shorter than
/// [`USER_DATA_CAPACITY`].
///
/// libnx documents the limit as 0x400 but rejects a size equal to the buffer, so
/// the last byte of the buffer is never reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvitationUserData<'a>(&'a [u8]);

impl InvitationUserData<'_> {
    /// Returns the data the invitation carries.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0
    }
}

impl<'a> TryFrom<&'a [u8]> for InvitationUserData<'a> {
    type Error = InvitationUserDataError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() >= USER_DATA_CAPACITY {
            return Err(InvitationUserDataError);
        }

        Ok(Self(data))
    }
}

/// Error returned when converting invitation user-data the applet would reject.
#[derive(Debug, thiserror::Error)]
#[error("the invitation user-data must be shorter than 0x400 bytes")]
pub struct InvitationUserDataError;

/// Payload for [`ArgType::StartFriendInvitation`].
///
/// The invitees are picked in the applet's own UI, so only their count is sent.
#[derive(Clone, Copy, zerocopy::FromZeros, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct StartFriendInvitationData {
    /// How many friends the user is asked to pick.
    pub id_count: i32,
    _pad: u32,
    /// How much of [`Self::userdata`] is populated.
    pub userdata_size: u64,
    /// Arbitrary user-data, see [`Self::userdata_size`].
    pub userdata: [u8; USER_DATA_CAPACITY],
    /// What the invitation says about the game mode.
    pub desc: FriendInvitationGameModeDescription,
}

const_assert_eq!(size_of::<StartFriendInvitationData>(), 0x1010);
const_assert!(size_of::<StartFriendInvitationData>() <= ARG_DATA_SIZE);

impl StartFriendInvitationData {
    /// Builds the payload, clearing the unused tail of the user-data buffer.
    pub fn new(
        id_count: InviteeCount,
        desc: &FriendInvitationGameModeDescription,
        userdata: InvitationUserData<'_>,
    ) -> Self {
        let userdata = userdata.as_bytes();

        let mut data = Self::new_zeroed();
        data.id_count = id_count.as_raw();
        // Widening cast: `u64` holds every `usize` this target has.
        data.userdata_size = userdata.len() as u64;
        data.userdata[..userdata.len()].copy_from_slice(userdata);
        data.desc = *desc;
        data
    }
}

/// Payload for [`ArgType::StartSendingFriendInvitation`].
///
/// The invitees are named rather than picked, so the list travels with the
/// count.
#[derive(Clone, Copy, zerocopy::FromZeros, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct StartSendingFriendInvitationData {
    /// How many entries of [`Self::id_list`] are populated.
    pub id_count: i32,
    _pad: u32,
    /// The accounts to invite, see [`Self::id_count`].
    pub id_list: [AccountNetworkServiceAccountId; ID_LIST_CAPACITY],
    /// How much of [`Self::userdata`] is populated.
    pub userdata_size: u64,
    /// Arbitrary user-data, see [`Self::userdata_size`].
    pub userdata: [u8; USER_DATA_CAPACITY],
    /// What the invitation says about the game mode.
    pub desc: FriendInvitationGameModeDescription,
}

const_assert_eq!(size_of::<StartSendingFriendInvitationData>(), 0x1090);
const_assert!(size_of::<StartSendingFriendInvitationData>() <= ARG_DATA_SIZE);

impl StartSendingFriendInvitationData {
    /// Builds the payload, clearing the unused tails of both buffers.
    pub fn new(
        invitees: InviteeList<'_>,
        desc: &FriendInvitationGameModeDescription,
        userdata: InvitationUserData<'_>,
    ) -> Self {
        let id_list = invitees.as_slice();
        let userdata = userdata.as_bytes();

        let mut data = Self::new_zeroed();
        data.id_count = invitees.count().as_raw();
        data.id_list[..id_list.len()].copy_from_slice(id_list);
        // Widening cast: `u64` holds every `usize` this target has.
        data.userdata_size = userdata.len() as u64;
        data.userdata[..userdata.len()].copy_from_slice(userdata);
        data.desc = *desc;
        data
    }
}

/// Payload for [`ArgType::ShowReceivedInvitationDetail`].
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ShowReceivedInvitationDetailData {
    /// The invitation to show.
    pub invitation_id: FriendInvitationId,
    /// The group the invitation belongs to.
    pub invitation_group_id: FriendInvitationGroupId,
}

const_assert_eq!(size_of::<ShowReceivedInvitationDetailData>(), 0x10);
const_assert!(size_of::<ShowReceivedInvitationDetailData>() <= ARG_DATA_SIZE);
