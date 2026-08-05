//! Where the `fsp-srv` session lives, and how a command borrows it.
//!
//! Every object this crate names (a mounted filesystem, an open file, an open directory walk)
//! is an id inside one session's domain, so an operation needs that session in hand before it can
//! address anything. The session is created by the runtime, which owns the Service Manager
//! bootstrap it depends on, and handed here through [`set`]. This module holds it for the life of
//! the process and lends it out one command at a time.
//!
//! The `with_*` helpers are what every operation goes through. Each rebuilds the wrapper for its
//! object, runs one command, and gives the close obligation back before returning, so the object
//! outlives the borrow and only an explicit close ends it.

use core::ops::Deref;

use nx_service_fs::FsService;
#[cfg(feature = "ffi")]
use nx_service_fs::{
    FsDir,
    FsFile,
    FsFileSystem,
};
#[cfg(feature = "ffi")]
use nx_sf::service::DispatchError;
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::{
        RwLock,
        RwLockReadGuard,
    },
};
#[cfg(feature = "ffi")]
use nx_sys_fd::device::DeviceError;

#[cfg(feature = "ffi")]
use crate::error::report;

/// The process-wide `fsp-srv` session, once the runtime has connected.
static SERVICE: OnceLock<RwLock<Option<FsService>>> = OnceLock::new();

/// Installs the session every command dispatches on, replacing whatever was there.
///
/// Called by the runtime once `fsp-srv` has been reached through the Service Manager. Nothing here
/// connects: the Service Manager is bootstrapped by the runtime, above this crate.
pub fn set(service: FsService) {
    *slot().write() = Some(service);
}

/// Drops the session, closing it and every object issued inside it.
///
/// Every mount must be gone first, or the ids they hold name objects that no longer exist.
pub fn clear() {
    // `FsService` is RAII; dropping the taken value closes the pooled sessions.
    let _ = slot().write().take();
}

/// Borrows the session, or reports that none has been installed.
pub fn get() -> Option<impl Deref<Target = FsService> + 'static> {
    let guard = slot().read();
    guard.is_some().then_some(ServiceRef(guard))
}

/// Returns the handle every object in the session is addressed through.
///
/// The C surface describes an object as a session plus an id, so it needs the handle without
/// needing the service the handle belongs to.
#[cfg(feature = "ffi")]
pub(crate) fn session_handle() -> Option<nx_svc::ipc::Handle> {
    get().map(|service| service.session_handle().to_handle())
}

/// Runs one command against the filesystem `object_id` names.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when no session is installed, and whatever [`report`] makes of a
/// command that failed.
#[cfg(feature = "ffi")]
#[cfg(feature = "ffi")]
pub(crate) fn with_filesystem<R>(
    object_id: u32,
    f: impl FnOnce(&FsFileSystem<'_>) -> Result<R, DispatchError>,
) -> Result<R, DeviceError> {
    let Some(service) = get() else {
        return Err(DeviceError::Io);
    };

    // SAFETY: `object_id` was issued by the server inside this session's domain, and only an
    // explicit close ends it. The obligation is handed straight back below.
    let object = FsFileSystem::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&object);
    let _ = object.into_raw_object_id();

    result.map_err(report)
}

/// Runs one command against the file `object_id` names. See [`with_filesystem`].
///
/// # Errors
///
/// The same as [`with_filesystem`].
#[cfg(feature = "ffi")]
pub(crate) fn with_file<R>(
    object_id: u32,
    f: impl FnOnce(&FsFile<'_>) -> Result<R, DispatchError>,
) -> Result<R, DeviceError> {
    let Some(service) = get() else {
        return Err(DeviceError::Io);
    };

    // SAFETY: as in `with_filesystem`.
    let object = FsFile::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&object);
    let _ = object.into_raw_object_id();

    result.map_err(report)
}

/// Runs one command against the directory `object_id` names. See [`with_filesystem`].
///
/// # Errors
///
/// The same as [`with_filesystem`].
#[cfg(feature = "ffi")]
pub(crate) fn with_dir<R>(
    object_id: u32,
    f: impl FnOnce(&FsDir<'_>) -> Result<R, DispatchError>,
) -> Result<R, DeviceError> {
    let Some(service) = get() else {
        return Err(DeviceError::Io);
    };

    // SAFETY: as in `with_filesystem`.
    let object = FsDir::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&object);
    let _ = object.into_raw_object_id();

    result.map_err(report)
}

/// Closes the filesystem `object_id` names.
///
/// Dropping the rebuilt wrapper is what sends the close, so this is the one place an id is not
/// handed back. A session that is already gone took the object with it, so there is nothing to do.
#[cfg(feature = "ffi")]
#[cfg(feature = "ffi")]
pub(crate) fn close_filesystem(object_id: u32) {
    let Some(service) = get() else {
        return;
    };

    // SAFETY: `object_id` was issued inside this session's domain and is closed exactly once, here.
    drop(FsFileSystem::from_raw_object_id_unchecked(
        &service, object_id,
    ));
}

/// Closes the file `object_id` names. See [`close_filesystem`].
#[cfg(feature = "ffi")]
pub(crate) fn close_file(object_id: u32) {
    let Some(service) = get() else {
        return;
    };

    // SAFETY: as in `close_filesystem`.
    drop(FsFile::from_raw_object_id_unchecked(&service, object_id));
}

/// Closes the directory `object_id` names. See [`close_filesystem`].
#[cfg(feature = "ffi")]
pub(crate) fn close_dir(object_id: u32) {
    let Some(service) = get() else {
        return;
    };

    // SAFETY: as in `close_filesystem`.
    drop(FsDir::from_raw_object_id_unchecked(&service, object_id));
}

/// Returns the slot the session lives in, creating it empty on first use.
fn slot() -> &'static RwLock<Option<FsService>> {
    SERVICE.get_or_init(|| RwLock::new(None))
}

/// Borrows the installed session for as long as the guard is held.
///
/// `Deref` rather than an accessor because this is a lock guard: a caller holds it for the length
/// of one command and reads the service through it, which is the shape every guard in the
/// workspace already has.
struct ServiceRef(RwLockReadGuard<'static, Option<FsService>>);

impl Deref for ServiceRef {
    type Target = FsService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is only wrapped once the slot has been observed occupied, and a write
        // that would empty it cannot run while this read guard is held.
        match self.0.as_ref() {
            Some(service) => service,
            None => unreachable!("the slot was occupied when this guard was taken"),
        }
    }
}
