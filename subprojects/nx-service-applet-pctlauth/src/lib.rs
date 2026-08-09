//! # nx-service-applet-pctlauth
//!
//! The `auth` library applet: the Parental Controls PIN screens, which ask the
//! user for the passcode or let them register and change it.
//!
//! # Shape
//!
//! [`ParentalAuth`] names which screen the applet opens on and carries the
//! argument bytes that screen accepts, and [`ParentalAuth::show_v1`] /
//! [`ParentalAuth::show_v2`] launch it. libnx exposes one function per screen,
//! all funnelling into a single private one that zeroes the argument struct and
//! sets only what its caller passed; which bytes a screen accepts is fixed, so
//! they belong to the variant here and a combination libnx would never build
//! cannot be constructed.
//!
//! The launch itself is [`nx_service_applet::library_applet::launch`]; what this
//! crate owns is the argument layout and the meaning of the reply.
//!
//! # Why two `show` methods
//!
//! libnx addresses the applet with library-applet API version 1, or 2 from
//! `[4.0.0+]`, and picks between them with `hosversionAtLeast`. The running
//! system version is a fact only the runtime crate may read, so the choice is a
//! method here and the caller that knows the version picks one. The three-byte
//! form of the authentication screen is the one libnx documents as `[4.0.0+]`,
//! so only a `show_v2` caller has reason to fill the second and third bytes.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it, so
//! it must not be called from a context that cannot wait indefinitely, nor from
//! one where IPC may already be broken. It also expects a PIN to already be
//! registered: every screen but [`ParentalAuth::RegisterPasscode`] asks for one.
//!
//! # References
//!
//! - [Switchbrew Wiki: applet ids](https://switchbrew.org/wiki/Applet_Manager_services#AppletId)
//! - [libnx pctlauth.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/pctlauth.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod parental_auth;
pub mod proto;

pub use self::parental_auth::{
    ParentalAuth,
    ShowError,
};
