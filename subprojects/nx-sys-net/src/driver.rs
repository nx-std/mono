//! Bringing the socket driver up and down.
//!
//! Two things have to be true before a socket call can work, and they come up and down together:
//! the process is connected to the BSD service, and the descriptor table knows how to dispatch
//! into sockets. [`initialize`] establishes both, [`exit`] releases both, and nothing else in the
//! crate sequences them.
//!
//! Keeping the pair here is what lets [`crate::session`] and [`crate::device`] stay unaware of each
//! other: the device reaches the session for every operation, and nothing reaches back.

use nx_service_bsd::ConnectOptions;
use nx_service_sm::SmService;

use crate::{
    device,
    session,
};

/// Brings the socket driver up, over the service manager session `sm`.
///
/// Connects to the BSD service with `opts` and registers the socket device, in that order: a
/// registered device whose session does not exist would accept descriptors it could not serve.
///
/// `sm` is borrowed rather than opened here: a process gets one service manager session, and by
/// the time a socket is opened the runtime holds it.
///
/// # Errors
///
/// Returns [`InitializeError::AlreadyInitialized`] when the driver is already up, leaving it
/// untouched. Returns [`InitializeError::Connect`] when the service handshake failed, and
/// [`InitializeError::Register`] when the device registry was full — in which case the session
/// established moments earlier is released again, so a failed initialization leaves nothing
/// behind.
pub fn initialize(sm: &SmService, opts: &ConnectOptions) -> Result<(), InitializeError> {
    if device::is_registered() {
        return Err(InitializeError::AlreadyInitialized);
    }

    session::connect(sm, opts).map_err(InitializeError::Connect)?;

    if let Err(err) = device::register() {
        session::disconnect();
        return Err(InitializeError::Register(err));
    }

    Ok(())
}

/// Errors returned by [`initialize`].
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// The driver is already up
    ///
    /// The existing session and device are untouched and remain usable.
    #[error("The socket driver is already initialized")]
    AlreadyInitialized,

    /// The service handshake failed
    ///
    /// Nothing was registered and no session was kept.
    #[error("Failed to connect to the socket service")]
    Connect(#[source] session::ConnectFailed),

    /// The socket device could not be registered
    ///
    /// The session opened for it has been released, so the driver is fully down.
    #[error("Failed to register the socket device")]
    Register(#[source] device::RegisterFailed),
}

/// Takes the socket driver down.
///
/// Unregisters the device first, so that nothing opens a descriptor against a session that is
/// about to go, then releases the session — which blocks until every in-flight command has
/// finished. Does nothing when the driver is not up, so it is safe to call twice and safe to call
/// after a failed [`initialize`].
///
/// Descriptors still open are not closed here. The service releases every socket belonging to the
/// client when the session goes, and the descriptors left behind report failure from then on.
pub fn exit() {
    device::unregister();
    session::disconnect();
}

/// Whether the socket driver is up.
pub fn is_initialized() -> bool {
    device::is_registered() && session::is_connected()
}
