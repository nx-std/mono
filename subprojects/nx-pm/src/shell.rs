//! `pm:shell` service.

use nx_service_pm::{NcmProgramLocation, PmShellService, ProcessEventInfo as RawProcessEventInfo};
pub use nx_service_pm::{ProcessEvent as ProcessEventKind, ProcessId, ProgramId};
use nx_sf::service::DispatchError;
use nx_svc::raw::Handle;

use crate::event::ProcessEvent;

/// Connected `pm:shell` service.
///
/// Method pairs `*` / `*_legacy` correspond to the firmware `[5.0.0+]` and
/// pre-5.0.0 command renumbering. Modern-only or firmware-gated commands
/// (e.g. `BoostApplicationThreadResourceLimit` on `[7.0.0+/Atmosphere]`) are
/// exposed unconditionally and surface a kernel error on older firmware.
pub struct ShellService {
    inner: PmShellService,
}

impl ShellService {
    pub(crate) fn new(inner: PmShellService) -> Self {
        Self { inner }
    }

    /// Launches a program, returning the new [`ProcessId`] (`[5.0.0+]` flag layout).
    pub fn launch_program(
        &self,
        flags: LaunchFlags,
        location: &ProgramLocation,
    ) -> Result<ProcessId, ShellLaunchProgramError> {
        self.inner
            .launch_program(flags.bits(), &location.to_wire())
            .map_err(ShellLaunchProgramError)
    }

    /// Launches a program using the pre-5.0.0 flag layout.
    pub fn launch_program_legacy(
        &self,
        flags: LaunchFlagsLegacy,
        location: &ProgramLocation,
    ) -> Result<ProcessId, ShellLaunchProgramLegacyError> {
        self.inner
            .launch_program(flags.bits(), &location.to_wire())
            .map_err(ShellLaunchProgramLegacyError)
    }

    /// Terminates a process by [`ProcessId`].
    pub fn terminate_process(&self, pid: ProcessId) -> Result<(), ShellTerminateProcessError> {
        self.inner
            .terminate_process(pid)
            .map_err(ShellTerminateProcessError)
    }

    /// Terminates the process matching the given [`ProgramId`].
    pub fn terminate_program(
        &self,
        program_id: ProgramId,
    ) -> Result<(), ShellTerminateProgramError> {
        self.inner
            .terminate_program(program_id)
            .map_err(ShellTerminateProgramError)
    }

    /// Acquires the process-event handle (autoclear).
    pub fn process_event(&self) -> Result<ProcessEvent, ShellGetProcessEventHandleError> {
        let raw = self
            .inner
            .get_process_event_handle()
            .map_err(ShellGetProcessEventHandleError)?;
        // SAFETY: The dispatch above returned a fresh event copy-handle this process owns.
        Ok(ProcessEvent::from_raw_unchecked(raw as Handle))
    }

    /// Reads the next [`ProcessEventInfo`].
    pub fn process_event_info(&self) -> Result<ProcessEventInfo, ShellGetProcessEventInfoError> {
        self.inner
            .get_process_event_info()
            .map(ProcessEventInfo::from)
            .map_err(ShellGetProcessEventInfoError)
    }

    /// Cleans up the resources of a terminated process (pre-5.0.0).
    pub fn cleanup_process(&self, pid: ProcessId) -> Result<(), ShellCleanupProcessError> {
        self.inner
            .cleanup_process(pid)
            .map_err(ShellCleanupProcessError)
    }

    /// Clears the JIT-debug-occurred flag for a process (pre-5.0.0).
    pub fn clear_jit_debug_occurred(
        &self,
        pid: ProcessId,
    ) -> Result<(), ShellClearJitDebugOccurredError> {
        self.inner
            .clear_jit_debug_occurred(pid)
            .map_err(ShellClearJitDebugOccurredError)
    }

    /// Signals that boot has finished (`[5.0.0+]`).
    pub fn notify_boot_finished(&self) -> Result<(), ShellNotifyBootFinishedError> {
        self.inner
            .notify_boot_finished()
            .map_err(ShellNotifyBootFinishedError)
    }

    /// Signals that boot has finished (pre-5.0.0).
    pub fn notify_boot_finished_legacy(&self) -> Result<(), ShellNotifyBootFinishedLegacyError> {
        self.inner
            .notify_boot_finished_legacy()
            .map_err(ShellNotifyBootFinishedLegacyError)
    }

    /// Returns the application's [`ProcessId`] (`[5.0.0+]`).
    pub fn application_process_id_for_shell(
        &self,
    ) -> Result<ProcessId, ShellGetApplicationProcessIdForShellError> {
        self.inner
            .get_application_process_id_for_shell()
            .map_err(ShellGetApplicationProcessIdForShellError)
    }

    /// Returns the application's [`ProcessId`] (pre-5.0.0).
    pub fn application_process_id_for_shell_legacy(
        &self,
    ) -> Result<ProcessId, ShellGetApplicationProcessIdForShellLegacyError> {
        self.inner
            .get_application_process_id_for_shell_legacy()
            .map_err(ShellGetApplicationProcessIdForShellLegacyError)
    }

    /// Boosts the system memory resource limit by `boost` bytes (`[5.0.0+]`).
    pub fn boost_system_memory_resource_limit(
        &self,
        boost: u64,
    ) -> Result<(), ShellBoostSystemMemoryResourceLimitError> {
        self.inner
            .boost_system_memory_resource_limit(boost)
            .map_err(ShellBoostSystemMemoryResourceLimitError)
    }

    /// Boosts the system memory resource limit by `boost` bytes
    /// (`[4.0.0-4.1.0]`).
    pub fn boost_system_memory_resource_limit_legacy(
        &self,
        boost: u64,
    ) -> Result<(), ShellBoostSystemMemoryResourceLimitLegacyError> {
        self.inner
            .boost_system_memory_resource_limit_legacy(boost)
            .map_err(ShellBoostSystemMemoryResourceLimitLegacyError)
    }

    /// Boosts the application thread resource limit (`[7.0.0+/Atmosphere]`).
    pub fn boost_application_thread_resource_limit(
        &self,
    ) -> Result<(), ShellBoostApplicationThreadResourceLimitError> {
        self.inner
            .boost_application_thread_resource_limit()
            .map_err(ShellBoostApplicationThreadResourceLimitError)
    }

    /// Boosts the system thread resource limit (`[14.0.0+/Atmosphere]`).
    pub fn boost_system_thread_resource_limit(
        &self,
    ) -> Result<(), ShellBoostSystemThreadResourceLimitError> {
        self.inner
            .boost_system_thread_resource_limit()
            .map_err(ShellBoostSystemThreadResourceLimitError)
    }

    /// Resolves a [`ProgramId`] to its running [`ProcessId`]
    /// (`[19.0.0+/Atmosphere]`).
    pub fn process_id(&self, program_id: ProgramId) -> Result<ProcessId, ShellGetProcessIdError> {
        self.inner
            .get_process_id(program_id)
            .map_err(ShellGetProcessIdError)
    }
}

/// IPC dispatch failure from `pm:shell LaunchProgram` (`[5.0.0+]`).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell LaunchProgram IPC dispatch failed")]
pub struct ShellLaunchProgramError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell LaunchProgram` (pre-5.0.0 flag layout).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell LaunchProgram (legacy) IPC dispatch failed")]
pub struct ShellLaunchProgramLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell TerminateProcess`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell TerminateProcess IPC dispatch failed")]
pub struct ShellTerminateProcessError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell TerminateProgram`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell TerminateProgram IPC dispatch failed")]
pub struct ShellTerminateProgramError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell GetProcessEventHandle`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell GetProcessEventHandle IPC dispatch failed")]
pub struct ShellGetProcessEventHandleError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell GetProcessEventInfo`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell GetProcessEventInfo IPC dispatch failed")]
pub struct ShellGetProcessEventInfoError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell CleanupProcess` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell CleanupProcess IPC dispatch failed")]
pub struct ShellCleanupProcessError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell ClearExceptionOccurred` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell ClearJitDebugOccurred IPC dispatch failed")]
pub struct ShellClearJitDebugOccurredError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell NotifyBootFinished` (`[5.0.0+]`).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell NotifyBootFinished IPC dispatch failed")]
pub struct ShellNotifyBootFinishedError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell NotifyBootFinished` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell NotifyBootFinished (legacy) IPC dispatch failed")]
pub struct ShellNotifyBootFinishedLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell GetApplicationProcessIdForShell` (`[5.0.0+]`).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell GetApplicationProcessIdForShell IPC dispatch failed")]
pub struct ShellGetApplicationProcessIdForShellError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell GetApplicationProcessIdForShell` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell GetApplicationProcessIdForShell (legacy) IPC dispatch failed")]
pub struct ShellGetApplicationProcessIdForShellLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell BoostSystemMemoryResourceLimit` (`[5.0.0+]`).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell BoostSystemMemoryResourceLimit IPC dispatch failed")]
pub struct ShellBoostSystemMemoryResourceLimitError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell BoostSystemMemoryResourceLimit` (`[4.0.0-4.1.0]`).
#[derive(Debug, thiserror::Error)]
#[error("pm:shell BoostSystemMemoryResourceLimit (legacy) IPC dispatch failed")]
pub struct ShellBoostSystemMemoryResourceLimitLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell BoostApplicationThreadResourceLimit`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell BoostApplicationThreadResourceLimit IPC dispatch failed")]
pub struct ShellBoostApplicationThreadResourceLimitError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell BoostSystemThreadResourceLimit`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell BoostSystemThreadResourceLimit IPC dispatch failed")]
pub struct ShellBoostSystemThreadResourceLimitError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:shell AtmosphereGetProcessId`.
#[derive(Debug, thiserror::Error)]
#[error("pm:shell AtmosphereGetProcessId IPC dispatch failed")]
pub struct ShellGetProcessIdError(#[source] pub DispatchError);

/// Program location identifying a program by id and storage medium.
///
/// Layout matches `NcmProgramLocation` from `pm:shell LaunchProgram`.
#[derive(Debug, Clone, Copy)]
pub struct ProgramLocation {
    pub program_id: ProgramId,
    pub storage_id: u8,
}

impl ProgramLocation {
    /// Builds a [`ProgramLocation`] from the program id and storage medium.
    pub const fn new(program_id: ProgramId, storage_id: u8) -> Self {
        Self {
            program_id,
            storage_id,
        }
    }

    /// Converts to the wire layout consumed by `nx-service-pm`.
    pub(crate) fn to_wire(self) -> NcmProgramLocation {
        NcmProgramLocation {
            program_id: self.program_id,
            storage_id: self.storage_id,
            pad: [0; 7],
        }
    }
}

bitflags::bitflags! {
    /// Launch flags for `pm:shell LaunchProgram` on firmware `[5.0.0+]`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct LaunchFlags: u32 {
        const SIGNAL_ON_EXIT   = 1 << 0;
        const SIGNAL_ON_START  = 1 << 1;
        const SIGNAL_ON_CRASH  = 1 << 2;
        const SIGNAL_ON_DEBUG  = 1 << 3;
        const START_SUSPENDED  = 1 << 4;
        const DISABLE_ASLR     = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Launch flags for `pm:shell LaunchProgram` on firmware `[1.0.0-4.1.0]`.
    ///
    /// `SIGNAL_ON_START` is only honoured on `[2.0.0+]`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct LaunchFlagsLegacy: u32 {
        const SIGNAL_ON_EXIT   = 1 << 0;
        const START_SUSPENDED  = 1 << 1;
        const SIGNAL_ON_CRASH  = 1 << 2;
        const DISABLE_ASLR     = 1 << 3;
        const SIGNAL_ON_DEBUG  = 1 << 4;
        const SIGNAL_ON_START  = 1 << 5;
    }
}

/// Decoded payload from `pm:shell GetProcessEventInfo`.
#[derive(Debug, Clone, Copy)]
pub struct ProcessEventInfo {
    pub kind: ProcessEventKind,
    pub pid: ProcessId,
}

impl From<RawProcessEventInfo> for ProcessEventInfo {
    fn from(raw: RawProcessEventInfo) -> Self {
        Self {
            kind: raw.event,
            pid: raw.process_id,
        }
    }
}
