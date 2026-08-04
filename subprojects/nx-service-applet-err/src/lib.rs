//! # nx-service-applet-err
//!
//! The `error` library applet: the system dialog that presents an error to the
//! user, with text this process supplies.
//!
//! # Why this exists
//!
//! When a homebrew process crashes, the console shows a generic "the program
//! has closed due to an error" dialog that says nothing about the cause. That
//! dialog belongs to `qlaunch`, and nothing here can add text to it. Atmosphère
//! does not escalate to its own fatal screen either: `creport` skips
//! `fatalThrowWithContext` for user breaks, which is what a Rust panic raises,
//! so a panic never reaches the screen that would show registers.
//!
//! The SD-card crash report is no better for this purpose. `creport` reads
//! exactly four bytes at the break address and parses them as a `Result`, so a
//! panic that points it at a message string yields a nonsense code and the text
//! is never written.
//!
//! This applet is the one mechanism that puts caller-supplied text on screen:
//! 2 KB in the dialog, plus another 2 KB behind its "Details" button.
//!
//! # What it costs
//!
//! Showing the dialog is not a crash primitive. It launches a separate process
//! over IPC and blocks until the user dismisses it, which rules out three
//! contexts:
//!
//! * Anywhere IPC may already be broken. A panic raised inside a half-torn-down
//!   session that then makes more IPC calls turns a clean abort into a hang.
//! * Anywhere that cannot block indefinitely.
//! * Applet mode, where launching a nested foreground library applet is not
//!   generally permitted. An application (a title override or a forwarder) can.
//!
//! So this belongs above the panic handler, not inside it: a caller that knows
//! its services are healthy, with `svcBreak` still the floor underneath.
//!
//! # Shape
//!
//! [`ApplicationError`] carries the payload and nothing else; building one
//! performs no IPC. [`ApplicationError::show`] does all of it, driving the
//! sequence documented in [`nx_service_applet::library_applet`]: wait for the
//! launchable event, create the applet, push the common arguments and then the
//! error argument, start, wait, and read the reply.
//!
//! Only the *application* error variant is implemented. The others in libnx's
//! `error.h` share [`proto::ErrorCommonHeader`] but each needs its own argument
//! struct, and several need extra storages.
//!
//! # References
//!
//! - [Switchbrew Wiki: error applet](https://switchbrew.org/wiki/Error_Applet)
//! - [libnx error.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/error.h)

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

mod application_error;
pub mod proto;

pub use self::application_error::{
    ApplicationError,
    PushStorageError,
    ShowError,
};
