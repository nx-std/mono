//! # nx-service-applet-nfp
//!
//! The `cabinet` library applet: the system amiibo editor, which reads and
//! writes an NFC figure the user scans while it runs.
//!
//! # Shape
//!
//! [`AmiiboSettings`] names which settings screen the applet opens on and
//! carries the data that screen accepts, and [`AmiiboSettings::start`] launches
//! it. libnx exposes one function per screen, all funnelling into a single
//! private one whose unused arguments each caller passes as null; which
//! arguments a screen accepts is fixed, so they belong to the variant here and a
//! combination libnx would reject cannot be constructed.
//!
//! The launch itself is [`nx_service_applet::library_applet::launch`]; what this
//! crate owns is the argument layout, the flag bits derived from the request,
//! and the meaning of the reply.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it, so
//! it must not be called from a context that cannot wait indefinitely, nor from
//! one where IPC may already be broken. It also does nothing without a physical
//! amiibo: every screen waits on a scan.
//!
//! # References
//!
//! - [Switchbrew Wiki: cabinet applet](https://switchbrew.org/wiki/NFC_services)
//! - [libnx nfp_la.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/nfp_la.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod amiibo_settings;
pub mod proto;

// The NFC types this crate's own API hands in and back. Re-exported so a
// consumer naming them does not have to depend on `nx-service-nfc` for types it
// only passes through.
pub use nx_service_nfc::{
    NfcDeviceHandle,
    NfcTagInfo,
    NfpRegisterInfo,
};

pub use self::amiibo_settings::{
    AmiiboSettings,
    AmiiboSettingsReply,
    StartError,
};
