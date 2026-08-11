//! NV service state and singleton API.
//!
//! This module manages the NV service session and provides a singleton interface
//! for accessing NVIDIA driver functionality throughout the application lifecycle.

use nx_service_nv::{
    NvConfig,
    NvService,
    NvServiceType,
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use super::{
    applet,
    sm,
};
use crate::env;

/// Global NV state, lazily initialized.
static NV_STATE: OnceLock<RwLock<Option<NvState>>> = OnceLock::new();

/// Returns a reference to the NV state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<NvState>> {
    NV_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the NV service with the given configuration.
///
/// Counts its callers, so a second caller joins the open session.
/// Multiple calls increment the reference count; actual initialization only
/// happens on the first call.
///
/// # Errors
///
/// Returns an error when the Service Manager is not open, or when the
/// connection was refused. Nothing was opened.
pub fn init(config: NvConfig) -> Result<(), ConnectError> {
    let mut guard = state().write();

    // If already initialized, just increment ref count
    if let Some(ref mut nv_state) = *guard {
        nv_state.ref_count += 1;
        return Ok(());
    }

    let sm = sm::session().map_err(ConnectError::SmNotInitialized)?;

    // Get applet info for service connection via Rust APIs
    let applet_type: nx_service_applet::AppletType = env::applet_type().into();
    let aruid = applet::get_applet_resource_user_id();

    // Connect to NV service
    let service =
        nx_service_nv::connect(&sm, applet_type, aruid, config).map_err(ConnectError::Connect)?;

    *guard = Some(NvState {
        service,
        ref_count: 1,
    });

    Ok(())
}

/// Initializes the NV service with default configuration.
///
/// Uses auto service type detection and 8MB transfer memory.
pub fn init_default() -> Result<(), ConnectError> {
    init(NvConfig::default())
}

/// Gets the NV service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = NvService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(NvServiceRef(guard))
    } else {
        None
    }
}

/// Exits the NV service.
///
/// Decrements the reference count. Actual cleanup only happens when the
/// reference count reaches 0.
pub fn exit() {
    let mut guard = state().write();
    if let Some(ref mut nv_state) = *guard {
        nv_state.ref_count = nv_state.ref_count.saturating_sub(1);
        if nv_state.ref_count == 0 {
            // Take and drop the service: `NvService` is RAII, so dropping
            // releases both session handles and frees the tmem backing.
            let _ = guard.take();
        }
    }
}

/// Returns true if the NV service is currently initialized.
pub fn is_initialized() -> bool {
    state().read().is_some()
}

/// Internal storage for NV service.
struct NvState {
    /// NV service session
    service: NvService,
    /// How many callers of [`init`] have not yet called [`exit`]
    ref_count: u32,
}

/// Wrapper for accessing NvService through RwLockReadGuard.
struct NvServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<NvState>>);

impl core::ops::Deref for NvServiceRef {
    type Target = NvService;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(state) => &state.service,
            // SAFETY: the module accessor builds a `NvServiceRef` only
            // after finding the state present, and the read lock this
            // holds keeps it present for the borrow's lifetime.
            None => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the connection is attempted before the Service Manager is
    /// open, or after it is closed. Nothing was opened.
    #[error("the Service Manager is not initialized")]
    SmNotInitialized(#[source] nx_rt_core::services::sm::NotInitializedError),
    /// The NV service refused the connection.
    ///
    /// Occurs when the server was unreachable or rejected the request. Nothing
    /// was opened.
    #[error("failed to connect to the NV service")]
    Connect(#[source] nx_service_nv::ConnectError),
}

#[cfg(feature = "ffi")]
impl nx_rt_core::error::ToResultCode for ConnectError {
    fn to_rc(self) -> nx_rt_core::error::ResultCode {
        use nx_sf::error::ToResultCode as _;

        match self {
            Self::SmNotInitialized(err) => err.to_rc(),
            Self::Connect(err) => err.to_rc(),
        }
    }
}

/// Global configuration storage.
///
/// Overridable at run time, for a caller that needs a different driver:
/// - `__nx_nv_service_type`
/// - `__nx_nv_transfermem_size`
static NV_CONFIG: OnceLock<RwLock<NvConfigState>> = OnceLock::new();

struct NvConfigState {
    service_type: NvServiceType,
    transfer_mem_size: usize,
}

impl Default for NvConfigState {
    fn default() -> Self {
        Self {
            service_type: NvServiceType::Auto,
            transfer_mem_size: 0x80_0000, // 8 MB
        }
    }
}

/// Gets the current NV service type configuration.
pub fn service_type() -> NvServiceType {
    NV_CONFIG
        .get_or_init(|| RwLock::new(NvConfigState::default()))
        .read()
        .service_type
}

/// Sets the NV service type configuration.
///
/// Must be called before `init()` to have effect.
pub fn set_service_type(service_type: NvServiceType) {
    NV_CONFIG
        .get_or_init(|| RwLock::new(NvConfigState::default()))
        .write()
        .service_type = service_type;
}

/// Gets the current transfer memory size configuration.
pub fn transfer_mem_size() -> usize {
    NV_CONFIG
        .get_or_init(|| RwLock::new(NvConfigState::default()))
        .read()
        .transfer_mem_size
}

/// Sets the transfer memory size configuration.
///
/// Must be called before `init()` to have effect.
pub fn set_transfer_mem_size(size: usize) {
    NV_CONFIG
        .get_or_init(|| RwLock::new(NvConfigState::default()))
        .write()
        .transfer_mem_size = size;
}

/// Creates a configuration from the global settings.
pub fn config() -> NvConfig {
    NvConfig {
        service_type: service_type(),
        transfer_mem_size: transfer_mem_size(),
    }
}
