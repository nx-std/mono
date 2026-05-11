//! VI service state and singleton API.
//!
//! This module manages the VI service session and provides a singleton interface
//! for accessing display and layer functionality throughout the application lifecycle.

use nx_service_vi::{
    ConnectOptions, ViService,
    types::{ViLayerFlags, ViServiceType},
};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};

use crate::{
    env::hos_version::{self, HosVersion},
    services::sm,
};

/// Global VI state, lazily initialized.
static VI_STATE: OnceLock<RwLock<Option<ViState>>> = OnceLock::new();

/// Returns a reference to the VI state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<ViState>> {
    VI_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the VI service with the given service type.
///
/// This matches libnx's `viInitialize()` behavior with reference counting.
/// Multiple calls increment the reference count; actual initialization only
/// happens on the first call.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init(service_type: ViServiceType) -> Result<(), ConnectError> {
    let mut guard = state().write();

    // If already initialized, just increment ref count
    if let Some(ref mut vi_state) = *guard {
        vi_state.ref_count += 1;
        return Ok(());
    }

    let sm_guard = sm::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // libnx gates root-service retention (fatal-display commands) on HOS ≥ 16.0.0
    // and gates the indirect-binder sub-service on HOS ≥ 2.0.0. Compute the
    // capability bits here so `nx-service-vi` stays version-agnostic.
    let hosver = hos_version::get();
    let options = ConnectOptions {
        keep_root_service: hosver >= HosVersion::new(16, 0, 0),
        request_indirect_binder: hosver >= HosVersion::new(2, 0, 0),
    };

    // Connect to VI service
    let service =
        nx_service_vi::connect_with_options(sm, service_type, options).map_err(ConnectError)?;

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
/// [`ViService`] is consumed via [`ViService::close`] so all sub-service
/// session handles are returned to the kernel — `Service` has no `Drop`
/// impl, so simply dropping the state would leak every handle.
pub fn exit() {
    let mut guard = state().write();
    let should_close = {
        let Some(vi_state) = guard.as_mut() else {
            return;
        };
        vi_state.ref_count = vi_state.ref_count.saturating_sub(1);
        vi_state.ref_count == 0
    };
    if should_close && let Some(vi_state) = guard.take() {
        vi_state.service.close();
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
    /// Reference count for service guard pattern (like libnx's NX_GENERATE_SERVICE_GUARD)
    ref_count: u32,
}

/// Wrapper for accessing ViService through RwLockReadGuard.
struct ViServiceRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<ViState>>);

impl core::ops::Deref for ViServiceRef {
    type Target = ViService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create ViServiceRef when the option is Some
        &self.0.as_ref().unwrap().service
    }
}

/// Error returned by [`init`] when connecting to the VI service fails.
#[derive(Debug, thiserror::Error)]
#[error("failed to connect to VI service")]
pub struct ConnectError(#[source] pub nx_service_vi::ConnectError);

/// Global configuration storage for VI service type.
///
/// Mirrors the weak symbol pattern from libnx:
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
pub fn get_service_type() -> ViServiceType {
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
    init(get_service_type())
}

/// Global override storage mirroring libnx's weak symbols:
///
/// - `__nx_vi_layer_id` — non-zero forces `viCreateLayer` to use that exact
///   layer ID (skipping the applet-managed-layer fallback).
/// - `__nx_vi_stray_layer_flags` — flags passed to `_viCreateStrayLayer` when
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
pub fn get_layer_id_override() -> u64 {
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
pub fn get_stray_layer_flags_override() -> ViLayerFlags {
    layer_overrides().read().stray_layer_flags
}

/// Sets the `__nx_vi_stray_layer_flags` override.
///
/// Must be called before [`init`]/[`init_with_config`] to influence subsequent
/// layer creation.
pub fn set_stray_layer_flags_override(flags: ViLayerFlags) {
    layer_overrides().write().stray_layer_flags = flags;
}
