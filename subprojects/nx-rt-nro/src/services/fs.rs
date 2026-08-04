//! `fsp-srv` service state and singleton API.
//!
//! This module manages the `fsp-srv` service session and provides a singleton
//! interface for accessing the filesystem service throughout the application
//! lifecycle.
//!
//! Unlike the other service managers, the session this one owns is a *domain*
//! backed by a pool of cloned sessions, mirroring libnx's `g_fsSessionMgr`.
//! Every filesystem, file and directory handed to C is a domain object id
//! addressed through that pool, so the pool must outlive them all.

use nx_service_fs::FsService;
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use crate::services::sm;

/// Global `fsp-srv` state, lazily initialized.
static FS_STATE: OnceLock<RwLock<Option<FsState>>> = OnceLock::new();

/// Returns a reference to the `fsp-srv` state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<FsState>> {
    FS_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the `fsp-srv` service.
///
/// This matches libnx's `fsInitialize()`: it looks the service up through SM,
/// converts the session to a domain, announces the current process, and clones
/// the session into the request pool.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init() -> Result<(), ConnectError> {
    let sm_guard = sm::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let service = nx_service_fs::connect_cmif(sm).map_err(ConnectError)?;

    let mut guard = state().write();
    *guard = Some(FsState { service });

    Ok(())
}

/// Gets the `fsp-srv` service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = FsService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(FsServiceRef(guard))
    } else {
        None
    }
}

/// Exits the `fsp-srv` service.
pub fn exit() {
    let mut guard = state().write();
    // `FsService` is RAII; dropping the taken state closes the pooled sessions.
    let _ = guard.take();
}

/// Internal storage for the `fsp-srv` service.
struct FsState {
    /// `fsp-srv` service session, converted to a domain and pooled.
    service: FsService,
}

/// Wrapper for accessing `FsService` through `RwLockReadGuard`.
struct FsServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<FsState>>);

impl core::ops::Deref for FsServiceRef {
    type Target = FsService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create FsServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`] when connecting to the `fsp-srv` service fails.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to fsp-srv service")]
pub struct ConnectError(#[source] pub nx_service_fs::ConnectCmifError);

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        self.0.to_rc()
    }
}
