//! Opening the MyPage applet on a friends screen.

use nx_service_acc::{
    AccountNetworkServiceAccountId,
    AccountUid,
};
use nx_service_applet::{
    AppletId,
    LibraryApplet,
    LibraryAppletCreator,
    LibraryAppletExitReason,
    LibraryAppletMode,
    SelfController,
    library_applet::{
        self,
        LaunchError,
    },
};
use nx_service_friends::{
    FriendInvitationGameModeDescription,
    FriendInvitationGroupId,
    FriendInvitationId,
    InAppScreenName,
};
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    Arg,
    ArgCommonData,
    ArgType,
    ArgV1,
    InvitationUserData,
    InviteeCount,
    InviteeList,
    ShowReceivedInvitationDetailData,
    StartFriendInvitationData,
    StartSendingFriendInvitationData,
};

/// Library applet API version addressing the pre-\[9.0.0\] argument layout.
const LA_VERSION_V1: u32 = 0x1;

/// Library applet API version addressing the \[9.0.0+\] argument layout.
const LA_VERSION_V2: u32 = 0x10000;

/// Which friends screen the applet opens on, and the data that screen accepts.
///
/// Every screen here predates \[9.0.0\], so both argument layouts can express it:
/// [`show_v1`](Self::show_v1) writes the older one and
/// [`show_v2`](Self::show_v2) the newer. Deciding which the running system takes
/// is the caller's job: this crate sits below the runtime that knows the system
/// version, so it never asks.
///
/// The account the applet acts as travels in the argument header rather than in
/// the payload, so it is a parameter of the two `show` methods rather than a
/// field of every variant.
#[derive(Debug, Clone, Copy)]
pub enum MyPageScreen<'a> {
    /// Launches with the "Friend List" menu initially selected.
    FriendList,
    /// Shows another account's detail page.
    UserDetailInfo {
        /// The account to show.
        id: AccountNetworkServiceAccountId,
        /// First in-app screen name.
        first_in_app_screen_name: &'a InAppScreenName,
        /// Second in-app screen name.
        second_in_app_screen_name: &'a InAppScreenName,
    },
    /// Sends a friend request to another account.
    ///
    /// The applet reports whether the request went through, so this screen is
    /// one of the three whose reply is read back.
    SendFriendRequest {
        /// The account to send the request to.
        id: AccountNetworkServiceAccountId,
        /// First in-app screen name.
        first_in_app_screen_name: &'a InAppScreenName,
        /// Second in-app screen name.
        second_in_app_screen_name: &'a InAppScreenName,
    },
    /// Launches with the "Add Friend" menu initially selected.
    MethodsOfSendingFriendRequest,
    /// Launches on "Search for Local Users"; leaving that menu exits the applet.
    FacedFriendRequest,
    /// Launches on "Received Friend Requests"; leaving that menu exits the
    /// applet.
    ReceivedFriendRequestList,
    /// Launches on the "Blocked-User List"; leaving that menu exits the applet.
    BlockedUserList,
    /// Launches with the "Profile" menu initially selected.
    MyProfile,
    /// The profile as the HOME menu opens it, startup sound included.
    ///
    /// The same screen as [`MyProfile`](Self::MyProfile); libnx pairs it with
    /// the startup sound at its own entry point, and the pairing belongs to the
    /// variant here.
    MyProfileForHomeMenu,
}

impl MyPageScreen<'_> {
    /// Opens the applet on this screen with the pre-\[9.0.0\] argument layout,
    /// blocking until the user leaves it.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v1(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        uid: AccountUid,
    ) -> Result<(), ShowError> {
        let arg = ArgV1::new(self.ty(), uid, self.common_data());

        show(
            self_controller,
            creator,
            &self.applet(LA_VERSION_V1),
            arg.as_bytes(),
            self.expects_reply(),
        )
    }

    /// Opens the applet on this screen with the \[9.0.0+\] argument layout,
    /// blocking until the user leaves it.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v2(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        uid: AccountUid,
    ) -> Result<(), ShowError> {
        let arg = Arg::new_common(self.ty(), uid, &self.common_data());

        show(
            self_controller,
            creator,
            &self.applet(LA_VERSION_V2),
            arg.as_bytes(),
            self.expects_reply(),
        )
    }

    /// Returns how the applet is launched for this screen.
    const fn applet(self, la_version: u32) -> LibraryApplet {
        LibraryApplet {
            id: AppletId::LibraryAppletMyPage,
            mode: LibraryAppletMode::AllForeground,
            la_version,
            play_startup_sound: matches!(self, Self::MyProfileForHomeMenu),
        }
    }

    /// Returns the screen type this request opens.
    const fn ty(self) -> ArgType {
        match self {
            Self::FriendList => ArgType::ShowFriendList,
            Self::UserDetailInfo { .. } => ArgType::ShowUserDetailInfo,
            Self::SendFriendRequest { .. } => ArgType::StartSendingFriendRequest,
            Self::MethodsOfSendingFriendRequest => ArgType::ShowMethodsOfSendingFriendRequest,
            Self::FacedFriendRequest => ArgType::StartFacedFriendRequest,
            Self::ReceivedFriendRequestList => ArgType::ShowReceivedFriendRequestList,
            Self::BlockedUserList => ArgType::ShowBlockedUserList,
            Self::MyProfile | Self::MyProfileForHomeMenu => ArgType::ShowMyProfile,
        }
    }

    /// Returns the payload this screen carries, cleared when it carries none.
    fn common_data(self) -> ArgCommonData {
        match self {
            Self::UserDetailInfo {
                id,
                first_in_app_screen_name,
                second_in_app_screen_name,
            }
            | Self::SendFriendRequest {
                id,
                first_in_app_screen_name,
                second_in_app_screen_name,
            } => ArgCommonData {
                id,
                first_in_app_screen_name: *first_in_app_screen_name,
                second_in_app_screen_name: *second_in_app_screen_name,
            },
            _ => ArgCommonData::new_zeroed(),
        }
    }

    /// Returns whether the applet pushes a reply for this screen.
    const fn expects_reply(self) -> bool {
        matches!(self, Self::SendFriendRequest { .. })
    }
}

/// Which online-play invitation flow the applet opens on, and the data it
/// carries.
///
/// These three screens arrived with \[9.0.0\], so only the newer argument layout
/// can express them and there is no `show_v1` counterpart. Refusing the call on
/// an older system is the caller's job: this crate sits below the runtime that
/// knows the system version, so it never asks.
///
/// The account the applet acts as travels in the argument header rather than in
/// the payload, so it is a parameter of [`show`](Self::show) rather than a field
/// of every variant.
///
/// No `Debug`: the game-mode description each invitation carries is 0xC00 opaque
/// bytes, which `nx-service-friends` deliberately leaves unprintable.
#[derive(Clone, Copy)]
pub enum MyPageInvitation<'a> {
    /// Picks friends to invite to online play through the applet's UI.
    ///
    /// The applet reports whether the invitations went out, so this flow is one
    /// of the three whose reply is read back.
    StartFriendInvitation {
        /// How many friends the user is asked to pick.
        invitee_count: InviteeCount,
        /// What the invitation says about the game mode.
        desc: &'a FriendInvitationGameModeDescription,
        /// Arbitrary data carried with the invitation.
        userdata: InvitationUserData<'a>,
    },
    /// Sends an online-play invitation to a named list of friends.
    ///
    /// The applet reports whether the invitations went out, so this flow is one
    /// of the three whose reply is read back.
    StartSendingFriendInvitation {
        /// The accounts to invite.
        invitees: InviteeList<'a>,
        /// What the invitation says about the game mode.
        desc: &'a FriendInvitationGameModeDescription,
        /// Arbitrary data carried with the invitation.
        userdata: InvitationUserData<'a>,
    },
    /// Shows the detail page of a received invitation.
    ShowReceivedInvitationDetail {
        /// The invitation to show.
        invitation_id: FriendInvitationId,
        /// The group the invitation belongs to.
        invitation_group_id: FriendInvitationGroupId,
    },
}

impl MyPageInvitation<'_> {
    /// Opens the applet on this flow, blocking until the user leaves it.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        uid: AccountUid,
    ) -> Result<(), ShowError> {
        // The applet is always launched the same way here: none of these three
        // flows plays the startup sound, and all of them take the \[9.0.0+\]
        // layout.
        let applet = LibraryApplet {
            id: AppletId::LibraryAppletMyPage,
            mode: LibraryAppletMode::AllForeground,
            la_version: LA_VERSION_V2,
            play_startup_sound: false,
        };

        let arg = self.build_arg(uid);

        show(
            self_controller,
            creator,
            &applet,
            arg.as_bytes(),
            self.expects_reply(),
        )
    }

    /// Builds the argument storage for this flow.
    fn build_arg(self, uid: AccountUid) -> Arg {
        match self {
            Self::StartFriendInvitation {
                invitee_count,
                desc,
                userdata,
            } => Arg::new_start_friend_invitation(
                uid,
                &StartFriendInvitationData::new(invitee_count, desc, userdata),
            ),
            Self::StartSendingFriendInvitation {
                invitees,
                desc,
                userdata,
            } => Arg::new_start_sending_friend_invitation(
                uid,
                &StartSendingFriendInvitationData::new(invitees, desc, userdata),
            ),
            Self::ShowReceivedInvitationDetail {
                invitation_id,
                invitation_group_id,
            } => Arg::new_show_received_invitation_detail(
                uid,
                &ShowReceivedInvitationDetailData {
                    invitation_id,
                    invitation_group_id,
                },
            ),
        }
    }

    /// Returns whether the applet pushes a reply for this flow.
    const fn expects_reply(self) -> bool {
        matches!(
            self,
            Self::StartFriendInvitation { .. } | Self::StartSendingFriendInvitation { .. }
        )
    }
}

/// Launches the applet with `payload` and judges how it came back.
///
/// Shared by the two request enums, which differ only in the payload they build
/// and in how they are addressed.
fn show(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    applet: &LibraryApplet,
    payload: &[u8],
    expects_reply: bool,
) -> Result<(), ShowError> {
    // The reply is a single result code. libnx also rejects a reply shorter than
    // one; here a storage that cannot fill the buffer fails the read instead,
    // and arrives as `Launch`.
    let mut reported: u32 = 0;
    let reply = if expects_reply {
        Some(reported.as_mut_bytes())
    } else {
        None
    };

    let exit_reason = library_applet::launch(self_controller, creator, applet, payload, reply)
        .map_err(ShowError::Launch)?;

    // libnx treats anything but a normal exit as a failed launch, including the
    // user backing out.
    if exit_reason != LibraryAppletExitReason::Normal {
        return Err(ShowError::AbnormalExit(exit_reason));
    }

    if reported != 0 {
        return Err(ShowError::Reported(reported));
    }

    Ok(())
}

/// Error returned by [`MyPageScreen::show_v1`], [`MyPageScreen::show_v2`] and
/// [`MyPageInvitation::show`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the MyPage applet.
    #[error("failed to launch the friends applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet ran to completion and reported a failure of its own.
    #[error("the applet reported a failure")]
    Reported(u32),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for it.
            Self::AbnormalExit(_) => nx_sf::error::GENERIC_ERROR,
            // The applet did name a code; libnx returns it verbatim.
            Self::Reported(rc) => rc,
        }
    }
}
