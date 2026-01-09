//! Service registry for lazy-initialized service sessions.
//!
//! This module provides a centralized registry for storing and retrieving
//! service sessions by [`ServiceName`]. Services are stored as type-erased
//! `Arc<dyn Any>` values and can be lazily initialized on first access.

use alloc::{sync::Arc, vec::Vec};
use core::any::Any;

use nx_service_set::SetSysService;
use nx_sf::ServiceName;
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::service_manager;

/// Initial capacity for the service registry.
const INITIAL_CAPACITY: usize = 8;

/// Type alias for registry entries.
type RegistryEntry = (ServiceName, Arc<dyn Any + Send + Sync>);

/// Global service registry, lazily initialized on first access.
static REGISTRY: OnceLock<RwLock<Vec<RegistryEntry>>> = OnceLock::new();

/// Initializes the `set:sys` service session.
///
/// Connects to set:sys via SM and stores the session in the registry.
/// Selects CMIF or TIPC protocol based on HOS version.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn setsys_init() -> Result<(), SetsysConnectError> {
    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let setsys = if service_manager::should_use_tipc() {
        nx_service_set::connect_tipc(sm).map_err(SetsysConnectError::Tipc)?
    } else {
        nx_service_set::connect_cmif(sm).map_err(SetsysConnectError::Cmif)?
    };
    insert(nx_service_set::SERVICE_NAME, setsys);

    Ok(())
}

/// Gets the `set:sys` service session.
pub fn setsys_get() -> Option<Arc<SetSysService>> {
    get::<SetSysService>(nx_service_set::SERVICE_NAME)
}

/// Gets or initializes the `set:sys` service session.
///
/// Selects CMIF or TIPC protocol based on HOS version.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn setsys_get_or_init() -> Result<Arc<SetSysService>, SetsysConnectError> {
    get_or_init(nx_service_set::SERVICE_NAME, || {
        let sm_guard = service_manager::sm_session();
        let sm = sm_guard.as_ref().expect("SM not initialized");

        if service_manager::should_use_tipc() {
            nx_service_set::connect_tipc(sm).map_err(SetsysConnectError::Tipc)
        } else {
            nx_service_set::connect_cmif(sm).map_err(SetsysConnectError::Cmif)
        }
    })
}

/// Exits the set:sys service session.
pub fn setsys_exit() {
    remove(nx_service_set::SERVICE_NAME);
}

/// Error returned by [`setsys_init`] and [`setsys_get_or_init`].
#[derive(Debug, thiserror::Error)]
pub enum SetsysConnectError {
    /// Failed to connect using CMIF protocol.
    #[error("failed to connect to set:sys (CMIF)")]
    Cmif(#[source] nx_service_set::ConnectCmifError),
    /// Failed to connect using TIPC protocol.
    #[error("failed to connect to set:sys (TIPC)")]
    Tipc(#[source] nx_service_set::ConnectTipcError),
}

/// Returns a reference to the registry, initializing it if needed.
fn registry() -> &'static RwLock<Vec<RegistryEntry>> {
    REGISTRY.get_or_init(|| RwLock::new(Vec::with_capacity(INITIAL_CAPACITY)))
}

/// Gets a service session by name.
///
/// Returns `None` if the service is not registered or if the type doesn't match.
fn get<T: Any + Send + Sync>(name: ServiceName) -> Option<Arc<T>> {
    let guard = registry().read();

    for (n, service) in guard.iter() {
        if *n == name {
            return service.clone().downcast::<T>().ok();
        }
    }
    None
}

/// Gets or initializes a service session.
///
/// If a service with the given name is already registered and matches type `T`,
/// returns a clone of it. Otherwise, calls `init` to create a new service,
/// stores it in the registry, and returns it.
///
/// # Errors
///
/// Returns an error if `init` fails. If a service exists but has a different
/// type, `init` will be called and the existing entry will be replaced.
fn get_or_init<T, F, E>(name: ServiceName, init: F) -> Result<Arc<T>, E>
where
    T: Any + Send + Sync,
    F: FnOnce() -> Result<T, E>,
{
    // Try to get existing service with read lock
    {
        let guard = registry().read();
        for (entry_name, entry_service) in guard.iter() {
            if *entry_name == name {
                if let Ok(typed) = entry_service.clone().downcast::<T>() {
                    return Ok(typed);
                }
                // Type mismatch, will replace below
                break;
            }
        }
    }

    // Initialize new service
    let service = Arc::new(init()?);

    // Store in registry with write lock
    let mut guard = registry().write();

    // Remove existing entry with same name (if any)
    guard.retain(|(entry_name, _)| *entry_name != name);

    // Add new entry
    guard.push((name, service.clone()));

    Ok(service)
}

/// Inserts a service session into the registry.
///
/// Returns the previous value if one existed with the same name.
fn insert<T: Any + Send + Sync>(
    name: ServiceName,
    service: T,
) -> Option<Arc<dyn Any + Send + Sync>> {
    let mut guard = registry().write();

    // Find and remove existing entry
    let mut old = None;
    guard.retain(|(entry_name, entry_service)| {
        if *entry_name == name {
            old = Some(entry_service.clone());
            false
        } else {
            true
        }
    });

    // Add new entry
    guard.push((name, Arc::new(service)));

    old
}

/// Removes a service session from the registry.
///
/// Returns the removed value if it existed.
fn remove(name: ServiceName) -> Option<Arc<dyn Any + Send + Sync>> {
    let mut guard = registry().write();

    let mut removed = None;
    guard.retain(|(entry_name, entry_service)| {
        if *entry_name == name {
            removed = Some(entry_service.clone());
            false
        } else {
            true
        }
    });

    removed
}
