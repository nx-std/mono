//! MyPage applet (`myPage` library applet) FFI.
//!
//! libnx's `friends_la.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `friendsLa*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all twelve of libnx's
//! entry points are ported.
//!
//! # Where the system version is decided
//!
//! libnx branches on `hosversion` in five places: once to pick the argument
//! layout, and once in each of the three \[9.0.0+\] entry points to refuse the
//! call outright. `nx-service-applet-friends` cannot make those checks: a
//! service crate must not depend on the runtime that resolves the version, so it
//! exposes the layouts as separate methods and this module does the branching,
//! reproducing libnx exactly.
//!
//! # Nullability
//!
//! libnx dereferences the in-app screen names and the game-mode description
//! unconditionally, and documents the invitation user-data as optional. Each
//! entry point turns its raw pointers into references once, rejecting the
//! mandatory ones when they are null rather than faulting on them.

use core::ffi::c_void;

use nx_service_applet_friends::{
    AccountNetworkServiceAccountId,
    AccountUid,
    FriendInvitationGameModeDescription,
    FriendInvitationGroupId,
    FriendInvitationId,
    InAppScreenName,
    InvitationUserData,
    InviteeCount,
    InviteeList,
    MyPageInvitation,
    MyPageScreen,
};
use nx_sf::error::ToResultCode as _;

use crate::{
    ffi::common::{
        GENERIC_ERROR,
        LibnxError,
        libnx_error,
    },
    services::applet,
};

/// First system version taking the newer argument layout, and the first with the
/// invitation flows at all.
const INVITATION_VERSION: nx_rt_core::env::hos_version::HosVersion =
    nx_rt_core::env::hos_version::HosVersion::new(9, 0, 0);

/// Returns whether the running system takes the \[9.0.0+\] argument layout.
fn is_invitation_version() -> bool {
    nx_rt_core::env::hos_version::get() >= INVITATION_VERSION
}

/// Result code libnx returns for an argument it rejected at the boundary.
const BAD_INPUT: u32 = libnx_error(LibnxError::BadInput);

/// Result code libnx returns for a command the running system does not have.
const INCOMPAT_SYS_VER: u32 = libnx_error(LibnxError::IncompatSysVer);

/// Opens the applet on `screen`, in the layout the running system takes.
///
/// Shared by the nine `friendsLa*` entry points that predate \[9.0.0\].
fn show(screen: MyPageScreen<'_>, uid: AccountUid) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    let self_controller = self_controller.get();
    let creator = creator.get();

    let result = if is_invitation_version() {
        screen.show_v2(&self_controller, &creator, uid)
    } else {
        screen.show_v1(&self_controller, &creator, uid)
    };

    match result {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Opens the applet on `invitation`.
///
/// Shared by the three `friendsLa*` entry points \[9.0.0\] added, which the
/// older layout cannot express at all.
fn show_invitation(invitation: MyPageInvitation<'_>, uid: AccountUid) -> u32 {
    if !is_invitation_version() {
        return INCOMPAT_SYS_VER;
    }

    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    match invitation.show(&self_controller.get(), &creator.get(), uid) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Borrows `size` bytes of caller-supplied invitation user-data.
///
/// A null pointer with a size of zero borrows nothing, which is how libnx
/// documents "no user-data". Returns [`None`] for the combination libnx rejects:
/// a null pointer with a non-zero size.
///
/// # Safety
///
/// `ptr` must be null, or point to `size` readable bytes that stay valid and
/// unwritten for `'a`.
unsafe fn borrow_user_data<'a>(ptr: *const c_void, size: u64) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return if size == 0 { Some(&[]) } else { None };
    }

    // Same-width cast: `usize` is 64 bits on this target.
    let size = size as usize;

    // SAFETY: The caller guarantees `ptr` points to `size` readable bytes that
    // outlive `'a` and are not written while it lasts.
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), size) })
}

/// Launches the applet with the "Friend List" menu initially selected.
///
/// Corresponds to `friendsLaShowFriendList()` in `friends_la.h`. Blocks until
/// the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_friend_list(uid: AccountUid) -> u32 {
    show(MyPageScreen::FriendList, uid)
}

/// Shows another account's detail page.
///
/// Corresponds to `friendsLaShowUserDetailInfo()` in `friends_la.h`. Blocks
/// until the user leaves the applet.
///
/// # Safety
///
/// Both screen-name pointers must be non-null and point to a valid
/// `FriendsInAppScreenName`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_user_detail_info(
    uid: AccountUid,
    id: AccountNetworkServiceAccountId,
    first_in_app_screen_name: *const InAppScreenName,
    second_in_app_screen_name: *const InAppScreenName,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; libnx would dereference it, and it is rejected below instead.
    let (first, second) = unsafe {
        (
            first_in_app_screen_name.as_ref(),
            second_in_app_screen_name.as_ref(),
        )
    };

    let (Some(first_in_app_screen_name), Some(second_in_app_screen_name)) = (first, second) else {
        return BAD_INPUT;
    };

    show(
        MyPageScreen::UserDetailInfo {
            id,
            first_in_app_screen_name,
            second_in_app_screen_name,
        },
        uid,
    )
}

/// Sends a friend request to another account.
///
/// Corresponds to `friendsLaStartSendingFriendRequest()` in `friends_la.h`.
/// Blocks until the user leaves the applet, then reports what the applet said
/// about the request.
///
/// # Safety
///
/// Both screen-name pointers must be non-null and point to a valid
/// `FriendsInAppScreenName`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_friends_la_start_sending_friend_request(
    uid: AccountUid,
    id: AccountNetworkServiceAccountId,
    first_in_app_screen_name: *const InAppScreenName,
    second_in_app_screen_name: *const InAppScreenName,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; libnx would dereference it, and it is rejected below instead.
    let (first, second) = unsafe {
        (
            first_in_app_screen_name.as_ref(),
            second_in_app_screen_name.as_ref(),
        )
    };

    let (Some(first_in_app_screen_name), Some(second_in_app_screen_name)) = (first, second) else {
        return BAD_INPUT;
    };

    show(
        MyPageScreen::SendFriendRequest {
            id,
            first_in_app_screen_name,
            second_in_app_screen_name,
        },
        uid,
    )
}

/// Launches the applet with the "Add Friend" menu initially selected.
///
/// Corresponds to `friendsLaShowMethodsOfSendingFriendRequest()` in
/// `friends_la.h`. Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_methods_of_sending_friend_request(
    uid: AccountUid,
) -> u32 {
    show(MyPageScreen::MethodsOfSendingFriendRequest, uid)
}

/// Launches the applet on "Search for Local Users".
///
/// Corresponds to `friendsLaStartFacedFriendRequest()` in `friends_la.h`. Blocks
/// until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_start_faced_friend_request(
    uid: AccountUid,
) -> u32 {
    show(MyPageScreen::FacedFriendRequest, uid)
}

/// Launches the applet on "Received Friend Requests".
///
/// Corresponds to `friendsLaShowReceivedFriendRequestList()` in `friends_la.h`.
/// Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_received_friend_request_list(
    uid: AccountUid,
) -> u32 {
    show(MyPageScreen::ReceivedFriendRequestList, uid)
}

/// Launches the applet on the "Blocked-User List".
///
/// Corresponds to `friendsLaShowBlockedUserList()` in `friends_la.h`. Blocks
/// until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_blocked_user_list(uid: AccountUid) -> u32 {
    show(MyPageScreen::BlockedUserList, uid)
}

/// Launches the applet with the "Profile" menu initially selected.
///
/// Corresponds to `friendsLaShowMyProfile()` in `friends_la.h`. Blocks until the
/// user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_my_profile(uid: AccountUid) -> u32 {
    show(MyPageScreen::MyProfile, uid)
}

/// Launches the applet on the profile as the HOME menu does, startup sound
/// included.
///
/// Corresponds to `friendsLaShowMyProfileForHomeMenu()` in `friends_la.h`.
/// Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_my_profile_for_home_menu(
    uid: AccountUid,
) -> u32 {
    show(MyPageScreen::MyProfileForHomeMenu, uid)
}

/// Picks friends to invite to online play through the applet's UI.
///
/// Corresponds to `friendsLaStartFriendInvitation()` in `friends_la.h`. Blocks
/// until the user leaves the applet, then reports what the applet said about the
/// invitations. Only available on \[9.0.0+\].
///
/// # Safety
///
/// `desc` must be non-null and point to a valid
/// `FriendsFriendInvitationGameModeDescription`, and `userdata` must be null or
/// point to `userdata_size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_friends_la_start_friend_invitation(
    uid: AccountUid,
    id_count: i32,
    desc: *const FriendInvitationGameModeDescription,
    userdata: *const c_void,
    userdata_size: u64,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so `desc`
    // is null or points to a valid value of its type, and `userdata` is null or
    // points to `userdata_size` readable bytes. Both null cases are rejected or
    // turned into an empty borrow below rather than followed.
    let (desc, userdata) = unsafe { (desc.as_ref(), borrow_user_data(userdata, userdata_size)) };

    let (Some(desc), Some(userdata)) = (desc, userdata) else {
        return BAD_INPUT;
    };

    let (Ok(invitee_count), Ok(userdata)) = (
        InviteeCount::try_from(id_count),
        InvitationUserData::try_from(userdata),
    ) else {
        return BAD_INPUT;
    };

    show_invitation(
        MyPageInvitation::StartFriendInvitation {
            invitee_count,
            desc,
            userdata,
        },
        uid,
    )
}

/// Sends an online-play invitation to a named list of friends.
///
/// Corresponds to `friendsLaStartSendingFriendInvitation()` in `friends_la.h`.
/// Blocks until the user leaves the applet, then reports what the applet said
/// about the invitations. Only available on \[9.0.0+\].
///
/// # Safety
///
/// `id_list` must be non-null and point to `id_count` readable
/// `AccountNetworkServiceAccountId` values, `desc` must be non-null and point to
/// a valid `FriendsFriendInvitationGameModeDescription`, and `userdata` must be
/// null or point to `userdata_size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_friends_la_start_sending_friend_invitation(
    uid: AccountUid,
    id_list: *const AccountNetworkServiceAccountId,
    id_count: i32,
    desc: *const FriendInvitationGameModeDescription,
    userdata: *const c_void,
    userdata_size: u64,
) -> u32 {
    // The count bounds the list before it is borrowed, so an out-of-range one is
    // rejected rather than used as a length.
    let Ok(invitee_count) = InviteeCount::try_from(id_count) else {
        return BAD_INPUT;
    };

    if id_list.is_null() {
        return BAD_INPUT;
    }

    // Widening cast: the count is between 1 and 15.
    let ids = invitee_count.as_raw() as usize;

    // SAFETY: The caller upholds this function's `# Safety` contract, so
    // `id_list` points to `id_count` readable ids, `desc` is null or points to a
    // valid value of its type, and `userdata` is null or points to
    // `userdata_size` readable bytes. `id_list` was rejected above if null.
    let (id_list, desc, userdata) = unsafe {
        (
            core::slice::from_raw_parts(id_list, ids),
            desc.as_ref(),
            borrow_user_data(userdata, userdata_size),
        )
    };

    let (Some(desc), Some(userdata)) = (desc, userdata) else {
        return BAD_INPUT;
    };

    let (Ok(invitees), Ok(userdata)) = (
        InviteeList::try_from(id_list),
        InvitationUserData::try_from(userdata),
    ) else {
        return BAD_INPUT;
    };

    show_invitation(
        MyPageInvitation::StartSendingFriendInvitation {
            invitees,
            desc,
            userdata,
        },
        uid,
    )
}

/// Shows the detail page of a received invitation.
///
/// Corresponds to `friendsLaShowReceivedInvitationDetail()` in `friends_la.h`.
/// Blocks until the user leaves the applet. Only available on \[9.0.0+\].
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_friends_la_show_received_invitation_detail(
    uid: AccountUid,
    invitation_id: FriendInvitationId,
    invitation_group_id: FriendInvitationGroupId,
) -> u32 {
    show_invitation(
        MyPageInvitation::ShowReceivedInvitationDetail {
            invitation_id,
            invitation_group_id,
        },
        uid,
    )
}
