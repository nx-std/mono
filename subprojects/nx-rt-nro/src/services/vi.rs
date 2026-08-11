//! VI service state and singleton API.
//!
//! This module manages the VI service session and provides a singleton interface
//! for accessing display and layer functionality throughout the application lifecycle.

use nx_service_vi::{
    ConnectOptions,
    ViService,
    types::{
        ViLayerFlags,
        ViServiceType,
    },
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::RwLock,
};

use super::sm;
use crate::env::hos_version::{
    self,
    HosVersion,
};

/// Global VI state, lazily initialized.
static VI_STATE: OnceLock<RwLock<Option<ViState>>> = OnceLock::new();

/// Returns a reference to the VI state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<ViState>> {
    VI_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the VI service with the given service type.
///
/// Counts its callers, so a second caller joins the open session.
/// Multiple calls increment the reference count; actual initialization only
/// happens on the first call.
///
/// # Errors
///
/// Returns an error when the Service Manager is not open, or when the
/// connection was refused. Nothing was opened.
pub fn init(service_type: ViServiceType) -> Result<(), ConnectError> {
    let mut guard = state().write();

    // If already initialized, just increment ref count
    if let Some(ref mut vi_state) = *guard {
        vi_state.ref_count += 1;
        return Ok(());
    }

    let sm = sm::session().map_err(ConnectError::SmNotInitialized)?;

    // libnx gates root-service retention (fatal-display commands) on HOS ≥ 16.0.0
    // and gates the indirect-binder sub-service on HOS ≥ 2.0.0. Compute the
    // capability bits here so `nx-service-vi` stays version-agnostic.
    let hosver = hos_version::get();
    let options = ConnectOptions {
        keep_root_service: hosver >= HosVersion::new(16, 0, 0),
        request_indirect_binder: hosver >= HosVersion::new(2, 0, 0),
    };

    // Connect to VI service
    let service = nx_service_vi::connect_with_options(&sm, service_type, options)
        .map_err(ConnectError::Connect)?;

    *guard = Some(ViState {
        service,
        ref_count: 1,
    });

    Ok(())
}

/// Initializes the VI service with default configuration.
///
/// Uses auto service type detection (tries Manager, then System, then Application).
pub fn init_default() -> Result<(), ConnectError> {
    init(ViServiceType::Default)
}

/// Gets the VI service.
pub fn get_service() -> Option<impl core::ops::Deref<Target = ViService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(ViServiceRef(guard))
    } else {
        None
    }
}

/// Exits the VI service.
///
/// Decrements the reference count. Actual cleanup only happens when the
/// reference count reaches 0. When the last reference is released the inner
/// [`ViService`] is dropped; its `Session` fields are RAII-owned and the
/// kernel handles are closed automatically by `Drop`.
pub fn exit() {
    let mut guard = state().write();
    let should_close = {
        let Some(vi_state) = guard.as_mut() else {
            return;
        };
        vi_state.ref_count = vi_state.ref_count.saturating_sub(1);
        vi_state.ref_count == 0
    };
    if should_close {
        // Dropping `ViState` releases all owned sub-service sessions via RAII.
        let _ = guard.take();
    }
}

/// Returns true if the VI service is currently initialized.
pub fn is_initialized() -> bool {
    state().read().is_some()
}

/// Internal storage for VI service.
struct ViState {
    /// VI service session
    service: ViService,
    /// How many callers of [`init`] have not yet called [`exit`]
    ref_count: u32,
}

/// Wrapper for accessing ViService through RwLockReadGuard.
struct ViServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<ViState>>);

impl core::ops::Deref for ViServiceRef {
    type Target = ViService;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(state) => &state.service,
            // SAFETY: the module accessor builds a `ViServiceRef` only
            // after finding the state present, and the read lock this
            // holds keeps it present for the borrow's lifetime.
            None => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

/// Error returned by [`init`] when connecting to the VI service fails.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// No Service Manager session is open.
    ///
    /// Occurs when the connection is attempted before the Service Manager is
    /// open, or after it is closed. Nothing was opened.
    #[error("the Service Manager is not initialized")]
    SmNotInitialized(#[source] nx_rt_core::services::sm::NotInitializedError),
    /// The display service refused the connection.
    ///
    /// Occurs when the server was unreachable or rejected the request. Nothing
    /// was opened.
    #[error("failed to connect to VI service")]
    Connect(#[source] nx_service_vi::ConnectError),
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

/// Global configuration storage for VI service type.
///
/// Overridable at run time, for a caller that needs a different display:
/// - `__nx_vi_service_type`
static VI_CONFIG: OnceLock<RwLock<ViConfigState>> = OnceLock::new();

struct ViConfigState {
    service_type: ViServiceType,
}

impl Default for ViConfigState {
    fn default() -> Self {
        Self {
            service_type: ViServiceType::Default,
        }
    }
}

/// Gets the current VI service type configuration.
pub fn service_type() -> ViServiceType {
    VI_CONFIG
        .get_or_init(|| RwLock::new(ViConfigState::default()))
        .read()
        .service_type
}

/// Sets the VI service type configuration.
///
/// Must be called before `init()` to have effect.
pub fn set_service_type(service_type: ViServiceType) {
    VI_CONFIG
        .get_or_init(|| RwLock::new(ViConfigState::default()))
        .write()
        .service_type = service_type;
}

/// Creates configuration and initializes using the global service type setting.
pub fn init_with_config() -> Result<(), ConnectError> {
    init(service_type())
}

/// Global override storage, read when a session is opened:
///
/// - `__nx_vi_layer_id`: non-zero forces `viCreateLayer` to use that exact
///   layer ID (skipping the applet-managed-layer fallback).
/// - `__nx_vi_stray_layer_flags`: flags passed to `_viCreateStrayLayer` when
///   no managed layer is available.
static VI_LAYER_OVERRIDES: OnceLock<RwLock<ViLayerOverrides>> = OnceLock::new();

#[derive(Clone, Copy)]
struct ViLayerOverrides {
    layer_id: u64,
    stray_layer_flags: ViLayerFlags,
}

impl Default for ViLayerOverrides {
    fn default() -> Self {
        Self {
            layer_id: 0,
            stray_layer_flags: ViLayerFlags::Default,
        }
    }
}

fn layer_overrides() -> &'static RwLock<ViLayerOverrides> {
    VI_LAYER_OVERRIDES.get_or_init(|| RwLock::new(ViLayerOverrides::default()))
}

/// Returns the current `__nx_vi_layer_id` override (0 if unset).
pub fn layer_id_override() -> u64 {
    layer_overrides().read().layer_id
}

/// Sets the `__nx_vi_layer_id` override. Pass `0` to clear.
///
/// Must be called before [`init`]/[`init_with_config`] to influence subsequent
/// layer creation.
pub fn set_layer_id_override(layer_id: u64) {
    layer_overrides().write().layer_id = layer_id;
}

/// Returns the current `__nx_vi_stray_layer_flags` override
/// (defaults to [`ViLayerFlags::Default`]).
pub fn stray_layer_flags_override() -> ViLayerFlags {
    layer_overrides().read().stray_layer_flags
}

/// Sets the `__nx_vi_stray_layer_flags` override.
///
/// Must be called before [`init`]/[`init_with_config`] to influence subsequent
/// layer creation.
pub fn set_stray_layer_flags_override(flags: ViLayerFlags) {
    layer_overrides().write().stray_layer_flags = flags;
}
