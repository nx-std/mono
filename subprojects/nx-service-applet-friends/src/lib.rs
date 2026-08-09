//! # nx-service-applet-friends
//!
//! The `myPage` library applet: the system Friends menu, which shows friend
//! lists, profiles and friend requests, and from \[9.0.0\] on also sends
//! online-play invitations.
//!
//! # Shape
//!
//! The applet takes one argument storage whose header names a screen and whose
//! payload carries that screen's data. libnx exposes one function per screen,
//! all funnelling into a single private one; which data a screen accepts is
//! fixed, so it belongs to the variant here and a combination libnx would reject
//! cannot be constructed.
//!
//! The screens split in two, because the argument layout did:
//!
//! - [`MyPageScreen`] is every screen the applet has had since launch. Both
//!   layouts express it, so it has a [`show_v1`](MyPageScreen::show_v1) and a
//!   [`show_v2`](MyPageScreen::show_v2).
//! - [`MyPageInvitation`] is the three invitation flows \[9.0.0\] added. Only the
//!   newer layout expresses them, so [`show`](MyPageInvitation::show) stands
//!   alone.
//!
//! # On the system version
//!
//! libnx picks the layout, and refuses the invitation flows outright, by asking
//! `hosversionAtLeast`. This crate cannot: a service crate sits below the
//! runtime that resolves the system version and must not depend on it. So the
//! choice is named in the API instead: the caller picks the method, and the
//! `nx-rt-*` shim that has the version picks for the C callers.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it, so
//! it must not be called from a context that cannot wait indefinitely, nor from
//! one where IPC may already be broken. Applet mode cannot generally launch a
//! nested foreground library applet; an application can.
//!
//! # References
//!
//! - [Switchbrew Wiki: Friend services](https://switchbrew.org/wiki/Friend_services)
//! - [libnx friends_la.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/friends_la.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod my_page;
pub mod proto;

// The account and friends types this crate's own API hands in. Re-exported so a
// consumer naming them does not have to depend on `nx-service-acc` or
// `nx-service-friends` for types it only passes through.
pub use nx_service_acc::{
    AccountNetworkServiceAccountId,
    AccountUid,
};
pub use nx_service_friends::{
    FriendInvitationGameModeDescription,
    FriendInvitationGroupId,
    FriendInvitationId,
    InAppScreenName,
};

pub use self::{
    my_page::{
        MyPageInvitation,
        MyPageScreen,
        ShowError,
    },
    proto::{
        InvitationUserData,
        InvitationUserDataError,
        InviteeCount,
        InviteeCountError,
        InviteeList,
    },
};
