//! The process-wide connection to the BSD socket service.
//!
//! Every socket command needs a connected [`BsdService`], and no C caller passes one: `send(fd,
//! …)` names a descriptor and nothing else. So the connection is held here, in one slot for the
//! whole process, and every operation borrows it for the length of one command.
//!
//! ## The service manager session is the caller's, not this crate's
//!
//! Horizon gives a process one `sm:` session, and by the time anything opens a socket the runtime
//! has already taken it. So [`connect`] borrows the session rather than opening one: a second
//! handshake does not get a second session, it fails, and it fails before the BSD service has been
//! reached at all.
//!
//! ## Connected on demand, not on first use
//!
//! The resolver's session connects lazily, because `getaddrinfo` has no initialization call and a
//! C caller has nowhere to have asked for one. Sockets do have one, and the contract is that a
//! program calls it before anything else. Connecting lazily would mean a program that forgot got a
//! session built from a configuration nobody chose — buffer sizes and a session count that suit no
//! workload in particular — and never learned it had skipped a step. So [`connect`] is explicit,
//! and an operation before it reports [`NotConnected`] rather than papering over it.
//!
//! ## The lock is read-mostly for a reason
//!
//! Commands run concurrently: that is what the session pool inside [`BsdService`] is for, and a
//! server accepting on one thread while another receives depends on it. Holding a read guard for
//! the length of a command is what lets that happen, and the write guard is taken only by
//! [`connect`] and [`disconnect`], which run once each.
//!
//! A blocking `recv` therefore holds a read guard for as long as it blocks, which is unbounded.
//! That is deliberate: it blocks a concurrent [`disconnect`], and tearing the service down
//! underneath an in-flight command is precisely what must not happen.

use nx_service_bsd::{
    BsdService,
    ConnectError,
    ConnectOptions,
    connect_with_options,
};
use nx_service_sm::SmService;
use nx_std_sync::rwlock::RwLock;

/// The process-wide BSD service connection.
///
/// `None` until [`connect`] establishes it, and again after [`disconnect`] releases it.
static SERVICE: RwLock<Option<BsdService>> = RwLock::new(None);

/// Establishes the process-wide connection, over the service manager session `sm`.
///
/// # Errors
///
/// Returns [`ConnectFailed::AlreadyConnected`] when a connection is already established, leaving
/// it untouched, and [`ConnectFailed::Connect`] when the handshake failed, in which case nothing
/// was stored and the call can be retried.
pub fn connect(sm: &SmService, opts: &ConnectOptions) -> Result<(), ConnectFailed> {
    let mut guard = SERVICE.write();
    if guard.is_some() {
        return Err(ConnectFailed::AlreadyConnected);
    }

    let service = connect_with_options(sm, opts).map_err(ConnectFailed::Connect)?;
    *guard = Some(service);
    Ok(())
}

/// Errors returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectFailed {
    /// A connection is already established
    ///
    /// The existing connection is untouched and remains usable; nothing was reconnected.
    #[error("The socket service is already connected")]
    AlreadyConnected,

    /// The BSD service handshake failed
    #[error("Failed to connect to the BSD socket service")]
    Connect(#[source] ConnectError),
}

/// Releases the process-wide connection, if one is established.
///
/// Blocks until every in-flight command has finished, because tearing the service down under one
/// would close the sessions its response is due on. Does nothing when no connection is
/// established, so it is safe to call twice and safe to call after a failed [`connect`].
pub fn disconnect() {
    let mut guard = SERVICE.write();
    if let Some(service) = guard.take() {
        service.close();
    }
}

/// Runs `op` against the process-wide connection.
///
/// # Errors
///
/// Returns [`NotConnected`] when no connection is established, in which case `op` did not run.
pub fn with_service<T>(op: impl FnOnce(&BsdService) -> T) -> Result<T, NotConnected> {
    let guard = SERVICE.read();
    match guard.as_ref() {
        Some(service) => Ok(op(service)),
        None => Err(NotConnected),
    }
}

/// Error returned by [`with_service`] when the socket driver was never initialized.
///
/// The operation did not run and nothing was sent.
#[derive(Debug, thiserror::Error)]
#[error("The socket service is not connected")]
pub struct NotConnected;

/// Whether a connection is currently established.
///
/// Answers the question the C initialization path asks before doing anything: a second
/// initialization is an error rather than a reconnect.
pub fn is_connected() -> bool {
    SERVICE.read().is_some()
}
