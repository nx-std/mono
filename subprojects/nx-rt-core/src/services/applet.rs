//! Applet Manager (AM) state and singleton API.
//!
//! The role taxonomy and the typed [`Proxy<R>`](nx_service_applet::proxy::Proxy)
//! wrapper live in `nx-service-applet`. This module owns the *runtime*
//! singleton: it pairs the typed proxy with a cache of values FFI consumers
//! read through atomic loads, performs the libnx-faithful init handshake, and
//! tears everything down on exit.
//!
//! Use the universal accessors ([`get_common_state_getter`],
//! [`cached_focus_state`], …) for role-independent operations, and the
//! role-specific [`as_application`] / [`as_library_applet`] / [`as_system_applet`] /
//! [`as_overlay_applet`] / [`as_system_application`] accessors to obtain a
//! typed handle whose method set reflects the actual sub-interface menu for
//! that role.

use core::sync::atomic::Ordering;

use nx_service_applet::{
    AppletFocusHandlingMode,
    AppletFocusState,
    AppletMessage,
    AppletOperationMode,
    AppletPerformanceMode,
    AppletType,
    CommonStateGetter,
    LibraryAppletCreator,
    SelfController,
    WindowController,
    aruid::Aruid,
};
use nx_std_sync::{
    once_lock::OnceLock,
    rwlock::{
        RwLock,
        RwLockReadGuard,
    },
};
use nx_svc::process::Handle as ProcessHandle;

mod error;
mod handle;
mod init;
mod state;

use self::state::{
    AppletSingleton,
    AppletState,
};
pub use self::{
    error::ConnectError,
    handle::{
        ApplicationHandle,
        LibraryAppletHandle,
        OverlayAppletHandle,
        SystemAppletHandle,
        SystemApplicationHandle,
    },
};
use crate::services::sm;

/// Sentinel for "focus state not yet known". `AppletFocusState` discriminants
/// start at 1 (`InFocus`) so 0 is safe as a tombstone.
const FOCUS_STATE_UNKNOWN: u8 = 0;

/// Global applet singleton, lazily initialized.
static APPLET_STATE: OnceLock<RwLock<Option<AppletState>>> = OnceLock::new();

fn state() -> &'static RwLock<Option<AppletState>> {
    APPLET_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the applet service.
///
/// Dispatches on `applet_type` to the matching `open_<role>` helper, which
/// performs the IPC plumbing (via `nx-service-applet`) and the
/// libnx-faithful runtime handshake. Each role brings up a distinct
/// Application Manager proxy command:
///
/// - [`AppletType::Application`] / [`AppletType::Default`] → `open_application`
///   → `appletOE` cmd 0
/// - [`AppletType::SystemApplet`] → `open_system_applet` → `appletAE` cmd 100
/// - [`AppletType::LibraryApplet`] → `open_library_applet`
///   → `appletAE` cmd 200·201
/// - [`AppletType::OverlayApplet`] → `open_overlay_applet` → `appletAE` cmd 300
/// - [`AppletType::SystemApplication`] → `open_system_application`
///   → `appletAE` cmd 350
/// - [`AppletType::None`] → no Application Manager session is opened
///
/// Counts its callers: a second caller joins the session the first opened
/// rather than performing the handshake again, and the session closes when the
/// last of them calls [`exit`]. Without the count, the second call would
/// replace the singleton and drop the first one's proxy handles while the C
/// surface still held a snapshot of them, and the first [`exit`] would tear
/// the session down for everyone.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init(applet_type: AppletType, process_handle: ProcessHandle) -> Result<(), ConnectError> {
    if matches!(applet_type, AppletType::None) {
        return Ok(());
    }

    {
        let mut guard = state().write();
        if let Some(applet_state) = guard.as_mut() {
            applet_state.retain();
            return Ok(());
        }
    }

    let sm_guard = sm::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let singleton = match applet_type {
        // appletOE cmd 0
        AppletType::Application | AppletType::Default => {
            AppletSingleton::Application(init::open_application(sm, process_handle)?)
        }
        // appletAE cmd 200·201
        AppletType::LibraryApplet => {
            AppletSingleton::LibraryApplet(init::open_library_applet(sm, process_handle)?)
        }
        // appletAE cmd 100
        AppletType::SystemApplet => {
            AppletSingleton::SystemApplet(init::open_system_applet(sm, process_handle)?)
        }
        // appletAE cmd 300
        AppletType::OverlayApplet => {
            AppletSingleton::OverlayApplet(init::open_overlay_applet(sm, process_handle)?)
        }
        // appletAE cmd 350
        AppletType::SystemApplication => {
            AppletSingleton::SystemApplication(init::open_system_application(sm, process_handle)?)
        }
        AppletType::None => unreachable!("AppletType::None handled at function entry"),
    };

    let mut guard = state().write();
    *guard = Some(AppletState::new(singleton));
    Ok(())
}

/// Releases one caller's hold on the applet session.
///
/// The session closes once the last caller has let go. For an Application-role
/// applet the focus handling mode is reset to
/// [`AppletFocusHandlingMode::NoSuspend`] first, so the system does not
/// force-suspend the process part-way through the teardown.
pub fn exit() {
    let mut guard = state().write();

    let Some(applet_state) = guard.as_mut() else {
        return;
    };
    if !applet_state.release() {
        return;
    }

    let Some(state) = guard.take() else {
        return;
    };
    let singleton = state.into_singleton();

    if let AppletSingleton::Application(slot) = &singleton {
        // The session is closing either way, so a refusal here costs nothing
        // beyond the suspend window it was meant to shorten.
        let _ = slot
            .proxy
            .set_focus_handling_mode(AppletFocusHandlingMode::NoSuspend);
    }
    // `Proxy<R>` is RAII; dropping `singleton` closes every IPC handle in
    // reverse acquisition order via `Drop`.
    drop(singleton);
}

/// Updates cached applet state in response to a received [`AppletMessage`].
///
/// Mirrors libnx `appletProcessMessage` for the cache-refresh paths: when the
/// system signals that focus/operation/performance mode changed, re-query the
/// authoritative value and update the cache.
pub fn process_message(msg: AppletMessage) {
    let guard = state().read();
    let Some(singleton) = guard.as_ref().map(AppletState::singleton) else {
        return;
    };

    let csg = singleton.common_state_getter();
    let cache = singleton.cache();

    match msg {
        AppletMessage::FocusStateChanged => {
            if let Ok(value) = csg.get_current_focus_state() {
                cache.focus_state.store(value as u8, Ordering::Release);
            }
        }
        AppletMessage::OperationModeChanged => {
            if let Ok(value) = csg.get_operation_mode() {
                cache.operation_mode.store(value as u8, Ordering::Release);
            }
        }
        AppletMessage::PerformanceModeChanged => {
            if let Ok(value) = csg.get_performance_mode() {
                cache
                    .performance_mode
                    .store(value as u32, Ordering::Release);
            }
        }
        _ => {}
    }
}

// Each `as_<role>` is public API of the shared applet manager: the
// per-output-kind entry crates (`nx-rt-nro`, `nx-rt-nso`) consume them from
// their own FFI shims to route role-gated commands through the typed proxy.

/// Returns a typed handle if the applet is initialized as
/// [`AppletType::Application`].
pub fn as_application() -> Option<ApplicationHandle> {
    let guard = state().read();
    match guard.as_ref().map(AppletState::singleton) {
        // SAFETY: variant verified by the match arm above; the read lock held
        // by `guard` prevents the variant from changing for the handle's
        // lifetime.
        Some(AppletSingleton::Application(_)) => {
            Some(unsafe { ApplicationHandle::from_guard(guard) })
        }
        _ => None,
    }
}

/// Returns a typed handle if the applet is initialised as
/// [`AppletType::LibraryApplet`].
pub fn as_library_applet() -> Option<LibraryAppletHandle> {
    let guard = state().read();
    match guard.as_ref().map(AppletState::singleton) {
        // SAFETY: see `as_application`.
        Some(AppletSingleton::LibraryApplet(_)) => {
            Some(unsafe { LibraryAppletHandle::from_guard(guard) })
        }
        _ => None,
    }
}

/// Returns a typed handle if the applet is initialised as
/// [`AppletType::SystemApplet`].
pub fn as_system_applet() -> Option<SystemAppletHandle> {
    let guard = state().read();
    match guard.as_ref().map(AppletState::singleton) {
        // SAFETY: see `as_application`.
        Some(AppletSingleton::SystemApplet(_)) => {
            Some(unsafe { SystemAppletHandle::from_guard(guard) })
        }
        _ => None,
    }
}

/// Returns a typed handle if the applet is initialised as
/// [`AppletType::OverlayApplet`].
pub fn as_overlay_applet() -> Option<OverlayAppletHandle> {
    let guard = state().read();
    match guard.as_ref().map(AppletState::singleton) {
        // SAFETY: see `as_application`.
        Some(AppletSingleton::OverlayApplet(_)) => {
            Some(unsafe { OverlayAppletHandle::from_guard(guard) })
        }
        _ => None,
    }
}

/// Returns a typed handle if the applet is initialised as
/// [`AppletType::SystemApplication`].
pub fn as_system_application() -> Option<SystemApplicationHandle> {
    let guard = state().read();
    match guard.as_ref().map(AppletState::singleton) {
        // SAFETY: see `as_application`.
        Some(AppletSingleton::SystemApplication(_)) => {
            Some(unsafe { SystemApplicationHandle::from_guard(guard) })
        }
        _ => None,
    }
}

/// Gets the [`CommonStateGetter`] sub-interface.
pub fn get_common_state_getter() -> Option<CommonStateGetterRef> {
    let guard = state().read();
    if guard.is_some() {
        Some(CommonStateGetterRef(guard))
    } else {
        None
    }
}

/// Gets the [`SelfController`] sub-interface.
pub fn get_self_controller() -> Option<SelfControllerRef> {
    let guard = state().read();
    if guard.is_some() {
        Some(SelfControllerRef(guard))
    } else {
        None
    }
}

/// Gets the [`LibraryAppletCreator`] sub-interface.
pub fn get_library_applet_creator() -> Option<LibraryAppletCreatorRef> {
    let guard = state().read();
    if guard.is_some() {
        Some(LibraryAppletCreatorRef(guard))
    } else {
        None
    }
}

/// Gets the [`WindowController`] sub-interface.
pub fn get_window_controller() -> Option<WindowControllerRef> {
    let guard = state().read();
    if guard.is_some() {
        Some(WindowControllerRef(guard))
    } else {
        None
    }
}

/// Names the event the system signals when an applet message is waiting.
///
/// The event belongs to the session and is closed with it, so what comes back
/// is the number to wait on rather than a handle to close. Returns `None` when
/// no session is open.
pub fn message_event_handle() -> Option<u32> {
    let guard = state().read();
    Some(
        guard
            .as_ref()?
            .singleton()
            .cache()
            .message_event
            .as_handle()
            .to_raw(),
    )
}

/// Gets the cached applet resource user ID.
///
/// Returns `None` if the applet is not initialised or the ARUID was not
/// available during init.
pub fn get_applet_resource_user_id() -> Option<Aruid> {
    let guard = state().read();
    guard.as_ref().and_then(|s| s.singleton().cache().aruid)
}

/// Returns the cached focus state, or `None` if the applet is not initialised
/// or the cached value is unknown.
pub fn cached_focus_state() -> Option<AppletFocusState> {
    let guard = state().read();
    let raw = guard
        .as_ref()?
        .singleton()
        .cache()
        .focus_state
        .load(Ordering::Acquire);
    if raw == FOCUS_STATE_UNKNOWN {
        return None;
    }
    AppletFocusState::from_raw(raw)
}

/// Returns the cached operation mode. Falls back to the default variant when
/// the applet is not initialized, matching libnx's default-initialized
/// global.
pub fn cached_operation_mode() -> AppletOperationMode {
    let guard = state().read();
    guard
        .as_ref()
        .and_then(|s| {
            AppletOperationMode::from_raw(
                s.singleton().cache().operation_mode.load(Ordering::Acquire),
            )
        })
        .unwrap_or_default()
}

/// Returns the cached performance mode. Falls back to the default variant when
/// the applet is not initialised, matching libnx's default-initialised
/// global.
pub fn cached_performance_mode() -> AppletPerformanceMode {
    let guard = state().read();
    guard
        .as_ref()
        .and_then(|s| {
            AppletPerformanceMode::from_raw(
                s.singleton()
                    .cache()
                    .performance_mode
                    .load(Ordering::Acquire),
            )
        })
        .unwrap_or_default()
}

// Each wrapper holds a read lock on the singleton and projects to a single
// core sub-interface via `AppletSingleton`'s role-erased accessors.
//
// The projection is an inherent `get`, not a `Deref`: a sub-interface is a
// borrowed view carrying the domain's lifetime, so it is built on each call
// rather than stored, and `Deref` can only hand back a reference to something
// the wrapper already holds.

macro_rules! define_core_ref {
    ($Ref:ident, $Target:ident, $method:ident) => {
        pub struct $Ref(RwLockReadGuard<'static, Option<AppletState>>);

        impl $Ref {
            /// Borrows the sub-interface for the duration of `&self`, which is
            /// what keeps it from outliving the read lock.
            #[inline]
            pub fn get(&self) -> $Target<'_> {
                match self.0.as_ref().map(AppletState::singleton) {
                    Some(singleton) => singleton.$method(),
                    // SAFETY: construction is guarded by `is_some()` in the
                    // module-level accessor, and the read lock held by `self.0`
                    // prevents the variant from changing for this wrapper's
                    // lifetime.
                    None => unsafe { core::hint::unreachable_unchecked() },
                }
            }
        }
    };
}

define_core_ref!(CommonStateGetterRef, CommonStateGetter, common_state_getter);
define_core_ref!(SelfControllerRef, SelfController, self_controller);
define_core_ref!(
    LibraryAppletCreatorRef,
    LibraryAppletCreator,
    library_applet_creator
);
define_core_ref!(WindowControllerRef, WindowController, window_controller);
