//! # nx-service-applet-album
//!
//! The `photoViewer` library applet: the system Album, which presents the
//! screenshots and recordings on the console.
//!
//! # Shape
//!
//! [`AlbumView`] names which set of files the applet presents, and
//! [`AlbumView::show`] launches it. libnx exposes the same three launches as
//! three functions; they differ only in the argument byte and in whether the
//! startup sound plays, so they are one enum here and the pairing of the two
//! cannot be got wrong.
//!
//! The launch itself is [`nx_service_applet::library_applet::launch`]; what this
//! crate owns is the argument byte and the choice of applet.
//!
//! # What it costs
//!
//! Showing the Album launches a separate process over IPC and blocks until the
//! user leaves it, so it must not be called from a context that cannot wait
//! indefinitely, nor from one where IPC may already be broken. Applet mode
//! cannot generally launch a nested foreground library applet; an application
//! can.
//!
//! # References
//!
//! - [Switchbrew Wiki: photoViewer applet](https://switchbrew.org/wiki/Capture_services)
//! - [libnx album_la.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/album_la.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod album_view;
pub mod proto;

pub use self::album_view::{
    AlbumView,
    ShowError,
};
