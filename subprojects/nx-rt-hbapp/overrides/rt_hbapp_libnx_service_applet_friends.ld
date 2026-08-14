/*
 * MyPage applet (`myPage` library applet) API
 * Rust: ffi/libnx/friends.rs
 * libnx: applets/friends_la.c
 *
 * Covers every `friendsLa*` entry point. libnx reaches the applet through
 * `g_appletILibraryAppletCreator`, which is `static` in applet.c and cannot be
 * aliased, so once `appletInitialize` is overridden no `friendsLa*` function
 * left to libnx has a session to use. All twelve are ported.
 */

/* Friends screens */
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_friend_list);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_user_detail_info);
EXTERN(__nx_rt_hbapp__libnx_friends_la_start_sending_friend_request);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_methods_of_sending_friend_request);
EXTERN(__nx_rt_hbapp__libnx_friends_la_start_faced_friend_request);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_received_friend_request_list);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_blocked_user_list);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_my_profile);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_my_profile_for_home_menu);

/* Online-play invitations [9.0.0+] */
EXTERN(__nx_rt_hbapp__libnx_friends_la_start_friend_invitation);
EXTERN(__nx_rt_hbapp__libnx_friends_la_start_sending_friend_invitation);
EXTERN(__nx_rt_hbapp__libnx_friends_la_show_received_invitation_detail);

/* Friends screens */
friendsLaShowFriendList = __nx_rt_hbapp__libnx_friends_la_show_friend_list;
friendsLaShowUserDetailInfo = __nx_rt_hbapp__libnx_friends_la_show_user_detail_info;
friendsLaStartSendingFriendRequest = __nx_rt_hbapp__libnx_friends_la_start_sending_friend_request;
friendsLaShowMethodsOfSendingFriendRequest = __nx_rt_hbapp__libnx_friends_la_show_methods_of_sending_friend_request;
friendsLaStartFacedFriendRequest = __nx_rt_hbapp__libnx_friends_la_start_faced_friend_request;
friendsLaShowReceivedFriendRequestList = __nx_rt_hbapp__libnx_friends_la_show_received_friend_request_list;
friendsLaShowBlockedUserList = __nx_rt_hbapp__libnx_friends_la_show_blocked_user_list;
friendsLaShowMyProfile = __nx_rt_hbapp__libnx_friends_la_show_my_profile;
friendsLaShowMyProfileForHomeMenu = __nx_rt_hbapp__libnx_friends_la_show_my_profile_for_home_menu;

/* Online-play invitations [9.0.0+] */
friendsLaStartFriendInvitation = __nx_rt_hbapp__libnx_friends_la_start_friend_invitation;
friendsLaStartSendingFriendInvitation = __nx_rt_hbapp__libnx_friends_la_start_sending_friend_invitation;
friendsLaShowReceivedInvitationDetail = __nx_rt_hbapp__libnx_friends_la_show_received_invitation_detail;
