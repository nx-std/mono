//! Runtime-side storage for the applet singleton.
//!
//! The role taxonomy and the [`Proxy<R>`](nx_service_applet::proxy::Proxy)
//! wrapper live in `nx-service-applet`. This module adds the runtime layer:
//! a per-role [`Slot`] that pairs `Proxy<R>` with an [`AppletCache`] of
//! values the FFI surface reads through atomic loads, and a role-erased
//! [`AppletSingleton`] enum so a single `OnceLock<RwLock<...>>` can hold
//! whichever role the current process is.

use core::sync::atomic::{
    AtomicU8,
    AtomicU32,
};

use nx_service_applet::{
    CommonStateGetter,
    LibraryAppletCreator,
    SelfController,
    WindowController,
    aruid::Aruid,
    proxy::Proxy,
    role::{
        Application,
        LibraryApplet,
        OverlayApplet,
        Role,
        SystemApplet,
        SystemApplication,
    },
};
use nx_svc::sync::EventHandle;

/// Lock-free cache of AM state values that change asynchronously via the
/// message queue. Populated at init time and refreshed by `process_message`.
pub(crate) struct AppletCache {
    pub aruid: Option<Aruid>,
    pub focus_state: AtomicU8,
    pub operation_mode: AtomicU8,
    pub performance_mode: AtomicU32,
    /// The event the system signals when a message is waiting.
    pub message_event: OwnedEventHandle,
}

/// The applet message event, owned for as long as the session is.
///
/// Asking the Application Manager for this event mints a new kernel handle
/// every time: the reply carries a copy handle, and the receiver owns it. It is
/// therefore asked for once, when the session is opened, and handed out by name
/// afterwards. A caller that asked per use would leak a handle per call, and
/// the process would run out of them long before it ran out of anything else.
pub(crate) struct OwnedEventHandle(EventHandle);

impl OwnedEventHandle {
    /// Takes ownership of an event handle a service just issued.
    pub fn new(handle: EventHandle) -> Self {
        Self(handle)
    }

    /// Names the event without transferring ownership.
    ///
    /// The borrow keeps the name from outliving the handle it refers to, which
    /// is what stops a caller from waiting on a closed event.
    pub fn as_handle(&self) -> &EventHandle {
        &self.0
    }
}

impl Drop for OwnedEventHandle {
    fn drop(&mut self) {
        // The session is going away with it, so there is nobody left to tell
        // that the close failed, and nothing they could do about it.
        let _ = nx_svc::sync::close_handle(self.0);
    }
}

/// Per-role storage slot: the typed proxy plus the runtime cache.
pub(crate) struct Slot<R: Role> {
    pub proxy: Proxy<R>,
    pub cache: AppletCache,
}

/// The applet session, and how many callers are still holding it open.
///
/// The count is stored beside the session rather than in a counter of its own
/// so the two cannot disagree: both are reached only through the one lock, and
/// a reader that sees a session sees the count that goes with it.
pub(crate) struct AppletState {
    singleton: AppletSingleton,
    /// How many callers of `applet::init` have not yet called `applet::exit`
    ref_count: u32,
}

impl AppletState {
    /// Records a freshly opened session, held by the caller that opened it.
    pub fn new(singleton: AppletSingleton) -> Self {
        Self {
            singleton,
            ref_count: 1,
        }
    }

    /// The session every accessor projects through.
    pub fn singleton(&self) -> &AppletSingleton {
        &self.singleton
    }

    /// Takes the session out, leaving the count behind with the husk.
    ///
    /// Called once the last owner has released it, to close what it holds.
    pub fn into_singleton(self) -> AppletSingleton {
        self.singleton
    }

    /// Adds a caller to the session that is already open.
    pub fn retain(&mut self) {
        self.ref_count += 1;
    }

    /// Removes a caller, and reports whether that was the last one.
    ///
    /// A `true` answer means the session is nobody's now and the caller should
    /// close it; the count is not decremented past zero, so an unmatched
    /// release cannot wrap round and strand the session open.
    #[must_use = "the session is only closed if this says it was the last owner"]
    pub fn release(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
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

    /// Universal core accessor: ILibraryAppletCreator (proxy cmd 11).
    pub fn library_applet_creator(&self) -> LibraryAppletCreator<'_> {
        match self {
            Self::Application(s) => s.proxy.library_applet_creator(),
            Self::LibraryApplet(s) => s.proxy.library_applet_creator(),
            Self::SystemApplet(s) => s.proxy.library_applet_creator(),
            Self::OverlayApplet(s) => s.proxy.library_applet_creator(),
            Self::SystemApplication(s) => s.proxy.library_applet_creator(),
        }
    }

    // NOTE: AudioController / DisplayController / DebugFunctions projections
    // are not yet exposed at the runtime layer — no top-level accessor or FFI
    // shim consumes them. Add them here when a consumer materialises rather
    // than carrying dead infrastructure.
}
