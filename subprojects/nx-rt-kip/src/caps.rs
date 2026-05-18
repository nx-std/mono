//! # Startup capability fragment (KIP)
//!
//! Declarative description of the kernel capabilities a KIP's *runtime startup*
//! needs — the supervisor calls it invokes before handing control to the
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
//! A KIP has exactly one runtime profile — kernel-launched, no command line,
//! and a fixed `None` applet identity (see the crate root). So, unlike the
//! per-applet-type NSO fragments, there is a single [`CapabilityFragment`]:
//! [`CAPABILITIES`].
//!
//! The startup invokes only the supervisor calls its kind-agnostic bring-up
//! needs:
//!
//! - **heap bring-up** — `svcSetHeapSize` allocates the process heap over the
//!   SVC-backed path.
//! - **Service Manager IPC** — `svcConnectToNamedPort` opens the `sm:` session,
//!   `svcSendSyncRequest` issues each IPC request, and `svcCloseHandle`
//!   releases the session handle.
//!
//! A kernel-launched process receives no `argv`, so the KIP startup runs no
//! command-line scan and needs no memory-query call to probe one. It contacts
//! no Application Manager, so it needs no `appletOE` / `appletAE` service
//! access. The `sm:` endpoint is an always-available named port, not an
//! `sm`-brokered service, so the startup declares no service-access capability
//! at all — only the supervisor calls above.

use nx_svc::code;

/// A supervisor call (SVC) the KIP runtime startup invokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Svc {
    /// Kernel SVC number — the immediate operand of the `svc` instruction.
    pub number: u16,
    /// libnx-style name, e.g. `"svcSetHeapSize"`.
    pub name: &'static str,
}

impl Svc {
    /// `svcSetHeapSize` — the SVC-backed heap path allocates the process heap.
    pub const SET_HEAP_SIZE: Self = Self::new(code::SET_HEAP_SIZE, "svcSetHeapSize");
    /// `svcConnectToNamedPort` — opens the `sm:` session.
    pub const CONNECT_TO_NAMED_PORT: Self =
        Self::new(code::CONNECT_TO_NAMED_PORT, "svcConnectToNamedPort");
    /// `svcSendSyncRequest` — issues every CMIF / TIPC IPC request.
    pub const SEND_SYNC_REQUEST: Self = Self::new(code::SEND_SYNC_REQUEST, "svcSendSyncRequest");
    /// `svcCloseHandle` — releases the Service Manager session handle.
    pub const CLOSE_HANDLE: Self = Self::new(code::CLOSE_HANDLE, "svcCloseHandle");

    const fn new(number: u16, name: &'static str) -> Self {
        Self { number, name }
    }
}

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
/// A KIP has a single, fixed runtime profile, so this is the only fragment —
/// there is no per-applet-type keying.
pub const CAPABILITIES: CapabilityFragment = CapabilityFragment {
    svcs: &STARTUP_SVCS,
};
