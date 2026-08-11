//! # Startup capability fragment (KIP)
//!
//! Declarative description of the kernel capabilities a KIP's *runtime startup*
//! needs: the supervisor calls it invokes before handing control to the
//! boot-time sysmodule.
//!
//! A KIP declares its permitted supervisor calls directly in the
//! kernel-capability descriptors embedded in its `KIP` header; unlike an NSO
//! process, it carries no separate NPDM. Those descriptors are the union of
//! what the sysmodule itself needs and what its runtime startup needs. This
//! module owns the *runtime* half as inspectable data: a build tool merges it
//! with the sysmodule-declared capabilities to emit the KIP header, so no
//! header is hand-maintained.
//!
//! ## The fragment is fixed
//!
//! A KIP has exactly one runtime profile: kernel-launched, no command line,
//! and a fixed `None` applet identity (see the crate root). So, unlike the
//! per-applet-type NSO fragments, there is a single [`CapabilityFragment`]:
//! [`CAPABILITIES`].
//!
//! The startup invokes only the supervisor calls its kind-agnostic bring-up
//! needs:
//!
//! - **heap bring-up**: `svcSetHeapSize` allocates the process heap over the
//!   SVC-backed path.
//! - **Service Manager IPC**: `svcConnectToNamedPort` opens the `sm:` session,
//!   `svcSendSyncRequest` issues each IPC request, and `svcCloseHandle`
//!   releases the session handle.
//!
//! A kernel-launched process receives no `argv`, so the KIP startup runs no
//! command-line scan and needs no memory-query call to probe one. It contacts
//! no Application Manager, so it needs no `appletOE` / `appletAE` service
//! access. The `sm:` endpoint is an always-available named port, not an
//! `sm`-brokered service, so the startup declares no service-access capability
//! at all: only the supervisor calls above.

pub use nx_rt_core::caps::Svc;

/// The minimum kernel capabilities one KIP's runtime startup needs.
///
/// This is the runtime contribution to the KIP header's kernel-capability
/// descriptors: a build tool merges it with the sysmodule-declared capabilities
/// to produce the full header.
#[derive(Debug, Clone, Copy)]
pub struct CapabilityFragment {
    /// Supervisor calls the startup invokes.
    pub svcs: &'static [Svc],
}

/// Supervisor calls a KIP startup invokes: heap bring-up and Service Manager
/// IPC.
const STARTUP_SVCS: [Svc; 4] = [
    Svc::SET_HEAP_SIZE,
    Svc::CONNECT_TO_NAMED_PORT,
    Svc::SEND_SYNC_REQUEST,
    Svc::CLOSE_HANDLE,
];

/// The startup capability fragment for a boot-time KIP.
///
/// A KIP has a single, fixed runtime profile, so this is the only fragment:
/// there is no per-applet-type keying.
pub const CAPABILITIES: CapabilityFragment = CapabilityFragment {
    svcs: &STARTUP_SVCS,
};
