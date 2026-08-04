//! Runtime-side storage for the applet singleton.
//!
//! The role taxonomy and the [`Proxy<R>`](nx_service_applet::proxy::Proxy)
//! wrapper live in `nx-service-applet`. This module adds the runtime layer:
//! a per-role [`Slot`] that pairs `Proxy<R>` with an [`AppletCache`] of
//! values the FFI surface reads through atomic loads, and a role-erased
//! [`AppletSingleton`] enum so a single `OnceLock<RwLock<...>>` can hold
//! whichever role the current process is.

use core::sync::atomic::{AtomicU8, AtomicU32};

use nx_service_applet::{
    CommonStateGetter, SelfController, WindowController,
    aruid::Aruid,
    proxy::Proxy,
    role::{Application, LibraryApplet, OverlayApplet, Role, SystemApplet, SystemApplication},
};

/// Lock-free cache of AM state values that change asynchronously via the
/// message queue. Populated at init time and refreshed by `process_message`.
pub(crate) struct AppletCache {
    pub aruid: Option<Aruid>,
    pub focus_state: AtomicU8,
    pub operation_mode: AtomicU8,
    pub performance_mode: AtomicU32,
}

/// Per-role storage slot: the typed proxy plus the runtime cache.
pub(crate) struct Slot<R: Role> {
    pub proxy: Proxy<R>,
    pub cache: AppletCache,
}

/// Role-erased singleton. Variants wrap a typed [`Slot<R>`] so universal
/// accessors can project through `match` while role-specific handles
/// downcast to a single variant.
pub(crate) enum AppletSingleton {
    Application(Slot<Application>),
    LibraryApplet(Slot<LibraryApplet>),
    SystemApplet(Slot<SystemApplet>),
    OverlayApplet(Slot<OverlayApplet>),
    SystemApplication(Slot<SystemApplication>),
}

impl AppletSingleton {
    /// Projects to the runtime cache, regardless of role.
    pub fn cache(&self) -> &AppletCache {
        match self {
            Self::Application(s) => &s.cache,
            Self::LibraryApplet(s) => &s.cache,
            Self::SystemApplet(s) => &s.cache,
            Self::OverlayApplet(s) => &s.cache,
            Self::SystemApplication(s) => &s.cache,
        }
    }

    /// Universal core accessor: ICommonStateGetter (proxy cmd 0).
    pub fn common_state_getter(&self) -> CommonStateGetter<'_> {
        match self {
            Self::Application(s) => s.proxy.common_state_getter(),
            Self::LibraryApplet(s) => s.proxy.common_state_getter(),
            Self::SystemApplet(s) => s.proxy.common_state_getter(),
            Self::OverlayApplet(s) => s.proxy.common_state_getter(),
            Self::SystemApplication(s) => s.proxy.common_state_getter(),
        }
    }

    /// Universal core accessor: ISelfController (proxy cmd 1).
    pub fn self_controller(&self) -> SelfController<'_> {
        match self {
            Self::Application(s) => s.proxy.self_controller(),
            Self::LibraryApplet(s) => s.proxy.self_controller(),
            Self::SystemApplet(s) => s.proxy.self_controller(),
            Self::OverlayApplet(s) => s.proxy.self_controller(),
            Self::SystemApplication(s) => s.proxy.self_controller(),
        }
    }

    /// Universal core accessor: IWindowController (proxy cmd 2).
    pub fn window_controller(&self) -> WindowController<'_> {
        match self {
            Self::Application(s) => s.proxy.window_controller(),
            Self::LibraryApplet(s) => s.proxy.window_controller(),
            Self::SystemApplet(s) => s.proxy.window_controller(),
            Self::OverlayApplet(s) => s.proxy.window_controller(),
            Self::SystemApplication(s) => s.proxy.window_controller(),
        }
    }

    // NOTE: AudioController / DisplayController / LibraryAppletCreator /
    // DebugFunctions projections are not yet exposed at the runtime layer —
    // no top-level accessor or FFI shim consumes them. Add them here when a
    // consumer materialises rather than carrying dead infrastructure.
}
