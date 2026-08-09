//! # nx-service-applet-mii
//!
//! The `miiEdit` library applet: the system Mii editor, which browses and edits
//! the console's Mii database and can make a Mii the database never sees.
//!
//! # Shape
//!
//! [`MiiEdit`] names which database screen the applet opens on and carries the
//! data that screen accepts; [`MiiCharInfoEdit`] does the same for the two
//! screens that edit a Mii without saving it. libnx exposes one function per
//! screen, all funnelling into a single private one whose unused argument
//! members each caller leaves zeroed; which members a screen reads is fixed, so
//! they belong to the variant here and a combination libnx would ignore cannot
//! be constructed.
//!
//! The split between the two types is the applet's own: libnx gives the
//! database screens one reply struct and the Mii-editing pair another, so the
//! two answer with different things: a database index against a Mii.
//!
//! # Firmware versions
//!
//! libnx addresses the applet as argument-storage version 3 below `[10.2.0]` and
//! version 4 from it, and refuses the two Mii-editing screens below `[10.2.0]`
//! entirely. A service crate does not read the firmware version, so both facts
//! surface as API instead: [`MiiEdit`] offers `show_v3` and `show_v4`, and
//! [`MiiCharInfoEdit`] offers only `show_v4`. The caller, in this workspace the
//! `nx-rt-*` FFI shim, decides which applies.
//!
//! # Divergence from the other applet crates
//!
//! Every other library applet wrapper here launches through
//! [`nx_service_applet::library_applet::launch`], which pushes the common
//! arguments as the first storage. The Mii editor is the exception: libnx's
//! `mii_la.c` pushes nothing but the applet's own argument struct, and an
//! applet reads its arguments from the first storage pushed. This crate
//! therefore drives the launch sequence itself, one step short of `launch`.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it,
//! so it must not be called from a context that cannot wait indefinitely, nor
//! from one where IPC may already be broken. Applet mode cannot generally
//! launch a nested foreground library applet; an application can.
//!
//! # References
//!
//! - [Switchbrew Wiki: miiEdit applet](https://switchbrew.org/wiki/Mii_Applet)
//! - [libnx mii_la.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/mii_la.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod mii_edit;
pub mod proto;

// The Mii types this crate's own API hands in and back. Re-exported so a
// consumer naming them does not have to depend on `nx-service-mii` for types it
// only passes through.
pub use nx_service_mii::{
    MiiCharInfo,
    MiiSpecialKeyCode,
};

pub use self::{
    mii_edit::{
        MiiCharInfoEdit,
        MiiCharInfoEditReply,
        MiiEdit,
        MiiEditReply,
        RunError,
        ShowError,
    },
    proto::Uuid,
};
