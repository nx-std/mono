//! Role-specific handles exposed by the applet singleton.
//!
//! Each handle wraps a [`RwLockReadGuard`] over the singleton and `Deref`s to
//! the typed [`Proxy<R>`](nx_service_applet::proxy::Proxy) from
//! `nx-service-applet`. Constructed only via the matching `as_<role>()`
//! accessor in the parent module, which verifies the variant before yielding
//! the handle — that's what makes the inner projection infallible.
//!
//! All method surface lives on `Proxy<R>` in `nx-service-applet`; this module
//! is just the lock-holding glue.

use core::ops::Deref;

use nx_service_applet::{
    proxy::Proxy,
    role::{
        Application,
        LibraryApplet,
        OverlayApplet,
        SystemApplet,
        SystemApplication,
    },
};
use nx_std_sync::rwlock::RwLockReadGuard;

use super::state::AppletSingleton;

/// Defines a role-specific handle: storage, the (unsafe) projection to
/// `Proxy<R>`, and the `Deref` impl that exposes every method
/// `nx-service-applet` defines on `Proxy<R>`.
///
/// The projection is guarded by the public constructor in
/// `services::applet::as_<role>`, which checks the singleton variant before
/// building the handle. The read lock held by `guard` prevents the variant
/// from changing for the handle's lifetime, so the alternate arm is
/// unreachable.
macro_rules! define_handle {
    ($(#[$meta:meta])* $Handle:ident, $variant:ident, $role:ty) => {
        $(#[$meta])*
        pub struct $Handle {
            guard: RwLockReadGuard<'static, Option<AppletSingleton>>,
        }

        impl $Handle {
            /// Wraps an already-verified guard into the typed handle.
            ///
            /// # Safety
            ///
            /// The caller must have inspected the singleton and confirmed it
            /// is `Some(AppletSingleton::$variant(_))`. The read lock held by
            /// `guard` then prevents the variant from changing for the
            /// handle's lifetime.
            pub(super) unsafe fn from_guard(
                guard: RwLockReadGuard<'static, Option<AppletSingleton>>,
            ) -> Self {
                Self { guard }
            }
        }

        impl Deref for $Handle {
            type Target = Proxy<$role>;

            fn deref(&self) -> &Self::Target {
                match self.guard.as_ref() {
                    Some(AppletSingleton::$variant(slot)) => &slot.proxy,
                    // SAFETY: `from_guard`'s caller verified the variant, and
                    // the read lock held by `self.guard` prevents the variant
                    // from changing while this handle exists.
                    _ => unsafe { core::hint::unreachable_unchecked() },
                }
            }
        }
    };
}

define_handle! {
    /// Handle for an `Application`-role applet (appletOE, proxy cmd 0).
    ApplicationHandle, Application, Application
}

define_handle! {
    /// Handle for a `LibraryApplet`-role applet (appletAE, proxy cmd 200/201).
    LibraryAppletHandle, LibraryApplet, LibraryApplet
}

define_handle! {
    /// Handle for a `SystemApplet`-role applet (appletAE, proxy cmd 100).
    SystemAppletHandle, SystemApplet, SystemApplet
}

define_handle! {
    /// Handle for an `OverlayApplet`-role applet (appletAE, proxy cmd 300).
    OverlayAppletHandle, OverlayApplet, OverlayApplet
}

define_handle! {
    /// Handle for a `SystemApplication`-role applet (appletAE, proxy cmd 350).
    SystemApplicationHandle, SystemApplication, SystemApplication
}
