//! # Startup capability fragment (NSO)
//!
//! Declarative description of the kernel and service capabilities an NSO
//! process's *runtime startup* needs: the supervisor calls it invokes and the
//! system services it connects to before handing control to the application.
//!
//! A Switch process declares its permitted supervisor calls and service access
//! in its NPDM (the kernel-capability and service-access descriptors). Those
//! two surfaces are the union of what the application itself needs and what its
//! runtime startup needs. This module owns the *runtime* half as inspectable
//! data: a build tool merges it with the application-declared capabilities to
//! emit the NPDM, so no NPDM is hand-maintained.
//!
//! ## Per-applet-type fragments
//!
//! Every NSO startup brings up the heap, probes the `__argdata__` command-line
//! region, and talks to the Service Manager: that base set is invariant. The
//! Application Manager (applet) handshake adds to it, and the additions depend
//! on the build-time applet identity:
//!
//! | `nso_applet_type`    | applet type         | SVC profile     | service access |
//! |----------------------|---------------------|-----------------|----------------|
//! | `application`        | `Application`       | base + sync     | `appletOE`     |
//! | `system-application` | `SystemApplication` | base + sync     | `appletAE`     |
//! | `system-applet`      | `SystemApplet`      | base            | `appletAE`     |
//! | `library-applet`     | `LibraryApplet`     | base            | `appletAE`     |
//! | `overlay-applet`     | `OverlayApplet`     | base            | `appletAE`     |
//! | `none`               | `None`              | base            | -              |
//!
//! - **base**: heap bring-up, the `__argdata__` probe, and Service Manager
//!   IPC.
//! - **base + sync**: the base set plus the two synchronization SVCs the
//!   `Application` / `SystemApplication` InFocus wait invokes. The other applet
//!   roles open their Application Manager proxy but skip the InFocus wait, so
//!   the base set suffices.
//! - A background sysmodule (`None`) never contacts the Application Manager, so
//!   it declares no `appletOE` / `appletAE` access.
//!
//! [`CAPABILITIES`] is the fragment for this build's applet identity;
//! [`for_applet`] yields the fragment for any applet type.

pub use nx_rt_core::caps::Svc;
use nx_service_applet::AppletType;

use crate::applet::APPLET_TYPE;

/// A system service the runtime startup connects to through the Service
/// Manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Service {
    /// Service name as registered with `sm:`, e.g. `"appletOE"`.
    pub name: &'static str,
}

impl Service {
    /// `appletOE`: the Application Manager service for the `Application` role.
    pub const APPLET_OE: Self = Self::new("appletOE");
    /// `appletAE`: the Application Manager service for every non-`Application`
    /// applet role.
    pub const APPLET_AE: Self = Self::new("appletAE");

    const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

/// The minimum kernel and service capabilities one NSO startup profile needs.
///
/// This is the runtime contribution to the process's NPDM: a build tool merges
/// it with the application-declared capabilities to produce the full
/// descriptor.
#[derive(Debug, Clone, Copy)]
pub struct CapabilityFragment {
    /// Supervisor calls the startup invokes.
    pub svcs: &'static [Svc],
    /// System services the startup connects to, beyond the always-available
    /// `sm:` named port.
    pub services: &'static [Service],
}

/// Supervisor calls every NSO startup invokes, regardless of applet identity.
const BASE_SVCS: [Svc; 5] = [
    Svc::SET_HEAP_SIZE,
    Svc::QUERY_MEMORY,
    Svc::CONNECT_TO_NAMED_PORT,
    Svc::SEND_SYNC_REQUEST,
    Svc::CLOSE_HANDLE,
];

/// [`BASE_SVCS`] extended with the two synchronization SVCs the `Application` /
/// `SystemApplication` InFocus wait invokes. Built from `BASE_SVCS` so the base
/// set is spelled exactly once.
const FOREGROUND_SVCS: [Svc; BASE_SVCS.len() + 2] = {
    let mut svcs = [Svc::SET_HEAP_SIZE; BASE_SVCS.len() + 2];
    let mut i = 0;
    while i < BASE_SVCS.len() {
        svcs[i] = BASE_SVCS[i];
        i += 1;
    }
    svcs[BASE_SVCS.len()] = Svc::WAIT_SYNCHRONIZATION;
    svcs[BASE_SVCS.len() + 1] = Svc::RESET_SIGNAL;
    svcs
};

/// No service access: a background sysmodule contacts no Application Manager.
const NO_SERVICES: [Service; 0] = [];

/// `appletOE` access: the `Application` role.
const APPLET_OE_SERVICES: [Service; 1] = [Service::APPLET_OE];

/// `appletAE` access: every non-`Application` applet role.
const APPLET_AE_SERVICES: [Service; 1] = [Service::APPLET_AE];

/// Returns the startup capability fragment for a given Application Manager
/// identity.
///
/// Keyed by applet type so the fragment for any `nso_applet_type` selection is
/// inspectable, not only the one this build selected (see [`CAPABILITIES`]).
pub const fn for_applet(applet: AppletType) -> CapabilityFragment {
    match applet {
        // `Application` / `SystemApplication` run the InFocus-wait handshake,
        // which adds the two synchronization SVCs. `Application` opens
        // `appletOE`; `SystemApplication` opens `appletAE`. `Default` resolves
        // to `Application`.
        AppletType::Application | AppletType::Default => CapabilityFragment {
            svcs: &FOREGROUND_SVCS,
            services: &APPLET_OE_SERVICES,
        },
        AppletType::SystemApplication => CapabilityFragment {
            svcs: &FOREGROUND_SVCS,
            services: &APPLET_AE_SERVICES,
        },
        // System / Library / Overlay applets open an `appletAE` proxy but skip
        // the InFocus wait, so the base SVC set suffices.
        AppletType::SystemApplet | AppletType::LibraryApplet | AppletType::OverlayApplet => {
            CapabilityFragment {
                svcs: &BASE_SVCS,
                services: &APPLET_AE_SERVICES,
            }
        }
        // A background sysmodule never contacts the Application Manager.
        AppletType::None => CapabilityFragment {
            svcs: &BASE_SVCS,
            services: &NO_SERVICES,
        },
    }
}

/// The startup capability fragment for this build's Application Manager
/// identity ([`crate::applet::APPLET_TYPE`]).
pub const CAPABILITIES: CapabilityFragment = for_applet(APPLET_TYPE);
