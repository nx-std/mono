//! Domain newtypes used by `pm:*` service wrappers.

/// Kernel process identifier (`u64`).
///
/// # Invariant
///
/// The wrapped value must be a process id assigned by the Horizon kernel —
/// either returned by a `pm:*` service call (e.g. `LaunchProgram`,
/// `GetApplicationProcessId`) or otherwise vouched for by the caller as
/// referring to a live or known kernel process. Arbitrary `u64` values must
/// not be fabricated into a `ProcessId`; doing so leads to spurious kernel
/// errors on dispatch and silently confuses any code that pattern-matches on
/// well-known PIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Wraps a raw kernel-assigned process id without checking the invariant.
    ///
    /// The caller must ensure `raw` is a process id returned by the Horizon kernel (e.g. via
    /// a `pm:*` dispatch) or otherwise known to identify a kernel process; see the
    /// [type-level invariant](ProcessId). Nothing here can check that, since only the kernel
    /// knows which ids are live. A fabricated id is not undefined behaviour: the dispatch it
    /// reaches fails with a kernel error, which is why this is a safe function.
    pub const fn from_raw_unchecked(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the underlying `u64`.
    pub const fn to_u64(&self) -> u64 {
        self.0
    }
}

/// Program identifier (`u64`).
///
/// # Invariant
///
/// The wrapped value must be a program id sourced from the system — typically
/// an `NcmProgramLocation`, an NCM/NS lookup, or a `pm:*` response — and not
/// a fabricated constant. `pm:*` commands dispatched with an invalid program
/// id surface kernel-level errors that are easy to misdiagnose; constructing
/// a `ProgramId` from an arbitrary `u64` defeats the type's purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ProgramId(u64);

impl ProgramId {
    /// Wraps a raw program id without checking the invariant.
    ///
    /// The caller must ensure `raw` is a program id sourced from a Horizon system component
    /// (NCM/NS/`pm:*` IPC) rather than fabricated; see the [type-level invariant](ProgramId).
    /// As with [`ProcessId::from_raw_unchecked`], a fabricated id surfaces as a kernel error
    /// on dispatch rather than as undefined behaviour.
    pub const fn from_raw_unchecked(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the underlying `u64`.
    pub const fn to_u64(&self) -> u64 {
        self.0
    }
}
