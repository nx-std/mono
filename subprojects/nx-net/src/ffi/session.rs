//! The cached `sfdnsres` resolver session backing the `__nx_net__*` FFI.
//!
//! A C caller of `getaddrinfo` / `gethostbyname` / … never connects to a
//! service explicitly, so the FFI owns the resolver session itself. It is
//! established lazily on the first resolver call and then kept for the life of
//! the process — the same long-lived-singleton pattern the `nx-pm` /
//! `nx-wlaninf` FFIs use for their service handles. A read/write lock guards
//! the slot so concurrent `__nx_net__*` calls share one connection.

use nx_service_sfdnsres::SfdnsresService;
use nx_std_sync::rwlock::RwLock;

use crate::resolve::resolver::{
    self,
    ConnectError,
};

/// The process-wide `sfdnsres` resolver session.
///
/// `None` until the first resolver FFI call connects it; never torn down
/// afterwards, mirroring the C resolver's implicit global session.
static RESOLVER: RwLock<Option<SfdnsresService>> = RwLock::new(None);

/// Runs `op` with the shared resolver session, connecting on first use.
///
/// The first caller performs the `sm:` + `sfdnsres` handshake; later callers
/// reuse the cached session. A connection failure is surfaced as
/// [`ConnectError`] and the session stays unestablished, so the next call
/// retries the handshake.
pub fn with_resolver<T>(op: impl FnOnce(&SfdnsresService) -> T) -> Result<T, ConnectError> {
    // Fast path: a session is already established.
    {
        let guard = RESOLVER.read();
        if let Some(svc) = guard.as_ref() {
            return Ok(op(svc));
        }
    }

    // Slow path: establish the session under the write lock. A caller that
    // lost the race to connect finds the slot already populated.
    let mut guard = RESOLVER.write();
    let svc: &SfdnsresService = match guard.as_mut() {
        Some(svc) => svc,
        None => guard.insert(resolver::connect()?),
    };
    Ok(op(svc))
}
