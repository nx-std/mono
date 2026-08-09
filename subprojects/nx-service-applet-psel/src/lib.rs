//! # nx-service-applet-psel
//!
//! The `playerSelect` library applet: the system user picker, which also hosts
//! the screens for creating a user and for editing one's icon or nickname.
//!
//! # Shape
//!
//! [`PlayerSelect`] names which screen the applet opens on and carries the data
//! that screen accepts, and [`PlayerSelect::show_v2`] and its siblings launch
//! it. libnx exposes one function per screen, each filling a shared settings
//! struct and handing it to `pselUiShow`; which members a screen fills is
//! fixed, so they belong to the variant here and a combination libnx would
//! reject cannot be constructed.
//!
//! [`show_ui_v2`] and its siblings are that same launch over a
//! [`proto::UiSettings`] the caller assembled itself, which is what libnx's
//! `pselUiShow` is.
//!
//! The launch itself is [`nx_service_applet::library_applet::launch`]; what this
//! crate owns is the settings layout, the members each screen fills, and the
//! meaning of the reply.
//!
//! # Versions
//!
//! The applet has been addressed at three library applet API versions, and the
//! version decides both how much of the settings struct is sent and which
//! members the applet reads. libnx picks one from the running system version;
//! a service crate may not read that, so the choice is spelled into the entry
//! points instead: `_v1` for [1.0.0], `_v2` for [2.0.0+], `_v6` for [6.0.0+].
//! The caller picks the one its console takes. Two screens arrived the same way
//! and are the caller's to withhold: `NintendoAccountNnidLinker` on [6.0.0+]
//! and `UserQualificationPromoter` on [13.0.0+].
//!
//! # What the account service still owns
//!
//! libnx's user-selector entry points ask the account service two questions
//! before launching anything: whether creating a user is permitted, and whether
//! a user can be selected without showing the applet at all. Neither is this
//! crate's to ask, since it holds no account session, so the first arrives as
//! [`PlayerSelect`] data and the second stays with the caller, who skips the
//! launch when it answers.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it,
//! so it must not be called from a context that cannot wait indefinitely, nor
//! from one where IPC may already be broken.
//!
//! # References
//!
//! - [Switchbrew Wiki: Applet Manager services](https://switchbrew.org/wiki/Applet_Manager_services)
//! - [libnx psel.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/psel.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod player_select;
pub mod proto;

// The account types this crate's own API hands in and back. Re-exported so a
// consumer naming them does not have to depend on `nx-service-acc` for types it
// only passes through.
pub use nx_service_acc::{
    AccountUid,
    USER_LIST_SIZE,
};

pub use self::player_select::{
    PlayerSelect,
    ShowError,
    show_ui_v1,
    show_ui_v2,
    show_ui_v6,
};
