//! The kernel capabilities a runtime startup needs, as inspectable data.
//!
//! A process declares the supervisor calls it may issue before it runs: a KIP
//! in its header's capability descriptors, an NSO in its NPDM. Those
//! permissions are the union of what the program itself needs and what its
//! runtime startup needs, and this module owns the vocabulary for naming the
//! second half so a build tool can merge it with the first.
//!
//! Which calls a given startup makes is a per-output-kind fact and lives with
//! that kind's entry crate. The descriptor below is not: an SVC number and the
//! name it goes by are the same on every kind, and two copies of a spec fact
//! drift.

use nx_svc::code;

/// A supervisor call (SVC) a runtime startup invokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Svc {
    /// Kernel SVC number: the immediate operand of the `svc` instruction.
    pub number: u16,
    /// The name the call goes by, e.g. `"svcSetHeapSize"`.
    pub name: &'static str,
}

impl Svc {
    /// `svcSetHeapSize`: the SVC-backed heap path allocates the process heap.
    pub const SET_HEAP_SIZE: Self = Self::new(code::SET_HEAP_SIZE, "svcSetHeapSize");
    /// `svcQueryMemory`: the argv reader probes the `__argdata__` mapping.
    pub const QUERY_MEMORY: Self = Self::new(code::QUERY_MEMORY, "svcQueryMemory");
    /// `svcConnectToNamedPort`: opens the `sm:` session.
    pub const CONNECT_TO_NAMED_PORT: Self =
        Self::new(code::CONNECT_TO_NAMED_PORT, "svcConnectToNamedPort");
    /// `svcSendSyncRequest`: issues every CMIF / TIPC IPC request.
    pub const SEND_SYNC_REQUEST: Self = Self::new(code::SEND_SYNC_REQUEST, "svcSendSyncRequest");
    /// `svcCloseHandle`: releases session and event handles.
    pub const CLOSE_HANDLE: Self = Self::new(code::CLOSE_HANDLE, "svcCloseHandle");
    /// `svcWaitSynchronization`: blocks on the applet message event during
    /// the InFocus wait.
    pub const WAIT_SYNCHRONIZATION: Self =
        Self::new(code::WAIT_SYNCHRONIZATION, "svcWaitSynchronization");
    /// `svcResetSignal`: clears the applet message event during the InFocus
    /// wait.
    pub const RESET_SIGNAL: Self = Self::new(code::RESET_SIGNAL, "svcResetSignal");

    const fn new(number: u16, name: &'static str) -> Self {
        Self { number, name }
    }
}
