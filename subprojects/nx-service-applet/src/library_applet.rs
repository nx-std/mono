//! Library applet creation and control.
//!
//! A library applet is a separate process the system runs on our behalf: the
//! error dialog, the software keyboard, the player-select UI. Launching one
//! spans three interfaces, and this module covers all three.
//!
//! # Ownership
//!
//! The sub-interfaces drained when the proxy opens live as long as the proxy
//! itself, so they are borrowed [`DomainObjectRef`]s. Everything created here is
//! different: the server mints these objects on request and expects them back,
//! so each wrapper owns a [`DomainObject`] and closes it on drop.
//!
//! [`DomainObjectRef`]: nx_sf::service::DomainObjectRef
//! [`DomainObject`]: nx_sf::service::DomainObject
//!
//! # Launch sequence
//!
//! The order below is the protocol, not a convenience. Two steps are easy to
//! miss and neither fails loudly. [`launch`] performs all of it, and an applet
//! that fits its shape — one payload storage in, at most one reply out — needs
//! nothing else from this module:
//!
//! 1. Wait on the launchable event
//!    ([`SelfController::get_library_applet_launchable_event`]). The system
//!    signals it when it is willing to host a library applet; creating one
//!    before then races it.
//! 2. [`LibraryAppletCreator::create_library_applet`] for the accessor.
//! 3. Push the common arguments *first*, then the applet-specific payload. Every
//!    library applet reads its arguments as storage 0, so pushing the payload
//!    first silently misaddresses every field.
//! 4. [`LibraryAppletAccessor::start`], then
//!    [`LibraryAppletAccessor::join`] to wait for the user to dismiss it.
//! 5. [`LibraryAppletAccessor::pop_out_data`] for the reply, if the applet
//!    returns one.
//!
//! [`SelfController::get_library_applet_launchable_event`]: crate::SelfController::get_library_applet_launchable_event
//! [`LibraryAppletCreator::create_library_applet`]: crate::LibraryAppletCreator::create_library_applet

mod accessor;
mod creator;
mod launch;
mod storage;
mod support;

pub use self::{
    accessor::{
        GetAppletStateChangedEventError,
        GetResultError,
        JoinError,
        LibraryAppletAccessor,
        PopOutDataError,
        PushInDataError,
        StartError,
    },
    creator::{
        CreateLibraryAppletError,
        CreateStorageError,
        create_library_applet,
        create_storage,
    },
    launch::{
        LaunchError,
        LibraryApplet,
        PushStorageError,
        launch,
    },
    storage::{
        GetSizeError,
        OpenStorageError,
        ReadStorageError,
        Storage,
        StorageAccessor,
        WriteStorageError,
    },
};
