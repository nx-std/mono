//! `pm:shell` (shell) service wrapper.

use core::mem::size_of;

use nx_service_sm::SmService;
use nx_sf::{
    error::{ResultCode, ToResultCode},
    service::{DispatchError, Session},
};
use static_assertions::const_assert_eq;

use super::{
    cmif,
    types::{ProcessId, ProgramId},
};

/// Connected `pm:shell` (shell) service wrapper.
pub struct PmShellService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmShellService {}
unsafe impl Sync for PmShellService {}

impl PmShellService {
    /// Launches a program.
    ///
    /// Returns the launched process ID.
    #[inline]
    pub fn launch_program(
        &self,
        launch_flags: u32,
        location: &NcmProgramLocation,
    ) -> Result<ProcessId, DispatchError> {
        cmif::launch_program(&self.0, launch_flags, location)
    }

    /// Terminates a process by PID.
    #[inline]
    pub fn terminate_process(&self, pid: ProcessId) -> Result<(), DispatchError> {
        cmif::terminate_process(&self.0, pid)
    }

    /// Terminates a program by program ID.
    #[inline]
    pub fn terminate_program(&self, program_id: ProgramId) -> Result<(), DispatchError> {
        cmif::terminate_program(&self.0, program_id)
    }

    /// Gets the process event handle.
    ///
    /// Returns a copy-handle for the event (always autoclear).
    #[inline]
    pub fn get_process_event_handle(&self) -> Result<u32, DispatchError> {
        cmif::get_process_event_handle(&self.0)
    }

    /// Gets the process event info.
    #[inline]
    pub fn get_process_event_info(&self) -> Result<ProcessEventInfo, DispatchError> {
        cmif::get_process_event_info(&self.0)
    }

    /// Cleans up a process (pre-5.0.0 only, cmd 5).
    #[inline]
    pub fn cleanup_process(&self, pid: ProcessId) -> Result<(), DispatchError> {
        cmif::cleanup_process(&self.0, pid)
    }

    /// Clears the JIT debug occurred flag (pre-5.0.0 only, cmd 6).
    #[inline]
    pub fn clear_jit_debug_occurred(&self, pid: ProcessId) -> Result<(), DispatchError> {
        cmif::clear_jit_debug_occurred(&self.0, pid)
    }

    /// Notifies the system that boot has finished.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`notify_boot_finished_legacy`](Self::notify_boot_finished_legacy).
    #[inline]
    pub fn notify_boot_finished(&self) -> Result<(), DispatchError> {
        cmif::notify_boot_finished(&self.0, proto::NOTIFY_BOOT_FINISHED)
    }

    /// Notifies the system that boot has finished (legacy, pre-5.0.0).
    #[inline]
    pub fn notify_boot_finished_legacy(&self) -> Result<(), DispatchError> {
        cmif::notify_boot_finished(&self.0, proto::NOTIFY_BOOT_FINISHED_LEGACY)
    }

    /// Gets the application process ID for shell.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`get_application_process_id_for_shell_legacy`](Self::get_application_process_id_for_shell_legacy).
    #[inline]
    pub fn get_application_process_id_for_shell(&self) -> Result<ProcessId, DispatchError> {
        cmif::get_application_process_id_for_shell(
            &self.0,
            proto::GET_APPLICATION_PROCESS_ID_FOR_SHELL,
        )
    }

    /// Gets the application process ID for shell (legacy, pre-5.0.0).
    #[inline]
    pub fn get_application_process_id_for_shell_legacy(&self) -> Result<ProcessId, DispatchError> {
        cmif::get_application_process_id_for_shell(
            &self.0,
            proto::GET_APPLICATION_PROCESS_ID_FOR_SHELL_LEGACY,
        )
    }

    /// Boosts the system memory resource limit.
    ///
    /// `[5.0.0+]`
    ///
    /// On `[4.0.0–4.1.0]`, use [`boost_system_memory_resource_limit_legacy`](Self::boost_system_memory_resource_limit_legacy).
    #[inline]
    pub fn boost_system_memory_resource_limit(&self, boost_size: u64) -> Result<(), DispatchError> {
        cmif::boost_system_memory_resource_limit(
            &self.0,
            proto::BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT,
            boost_size,
        )
    }

    /// Boosts the system memory resource limit (legacy).
    ///
    /// `[4.0.0–4.1.0]`
    #[inline]
    pub fn boost_system_memory_resource_limit_legacy(
        &self,
        boost_size: u64,
    ) -> Result<(), DispatchError> {
        cmif::boost_system_memory_resource_limit(
            &self.0,
            proto::BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT_LEGACY,
            boost_size,
        )
    }

    /// Boosts the application thread resource limit.
    ///
    /// `[7.0.0+/Atmosphere]`
    #[inline]
    pub fn boost_application_thread_resource_limit(&self) -> Result<(), DispatchError> {
        cmif::boost_application_thread_resource_limit(&self.0)
    }

    /// Boosts the system thread resource limit.
    ///
    /// `[14.0.0+/Atmosphere]`
    #[inline]
    pub fn boost_system_thread_resource_limit(&self) -> Result<(), DispatchError> {
        cmif::boost_system_thread_resource_limit(&self.0)
    }

    /// Gets a process ID from a program ID.
    ///
    /// `[19.0.0+/Atmosphere]`
    #[inline]
    pub fn get_process_id(&self, program_id: ProgramId) -> Result<ProcessId, DispatchError> {
        cmif::shell_get_process_id(&self.0, program_id)
    }
}

#[cfg(feature = "ffi")]
impl PmShellService {
    /// Returns the underlying session for libnx `Service*` shadow buffers.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.0
    }
}

/// Connects to the `pm:shell` (shell) service using CMIF.
pub fn connect_shell_cmif(sm: &SmService) -> Result<PmShellService, ConnectShellCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME)
        .map_err(ConnectShellCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PmShellService(service))
}

/// Error returned by [`connect_shell_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pm:shell service")]
pub struct ConnectShellCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

impl ToResultCode for ConnectShellCmifError {
    fn to_rc(self) -> ResultCode {
        self.0.to_rc()
    }
}

bitflags::bitflags! {
    /// Launch flags for `pm:shell` `LaunchProgram`.
    ///
    /// `[5.0.0+]`
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct LaunchFlag: u32 {
        const NONE = 0;
        const SIGNAL_ON_EXIT = 1 << 0;
        const SIGNAL_ON_START = 1 << 1;
        const SIGNAL_ON_CRASH = 1 << 2;
        const SIGNAL_ON_DEBUG = 1 << 3;
        const START_SUSPENDED = 1 << 4;
        const DISABLE_ASLR = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Launch flags for `pm:shell` `LaunchProgram` (pre-5.0.0).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct LaunchFlagOld: u32 {
        const NONE = 0;
        const SIGNAL_ON_EXIT = 1 << 0;
        const START_SUSPENDED = 1 << 1;
        const SIGNAL_ON_CRASH = 1 << 2;
        const DISABLE_ASLR = 1 << 3;
        const SIGNAL_ON_DEBUG = 1 << 4;
        /// Only available on `[2.0.0+]`.
        const SIGNAL_ON_START = 1 << 5;
    }
}

/// Process event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProcessEvent {
    None = 0,
    Exit = 1,
    Start = 2,
    Crash = 3,
    DebugStart = 4,
    DebugBreak = 5,
}

/// Process event info returned by `pm:shell` `GetProcessEventInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessEventInfo {
    pub event: ProcessEvent,
    pub process_id: ProcessId,
}

const_assert_eq!(size_of::<ProcessEventInfo>(), 0x10);

/// Program location identifying a program by ID and storage.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NcmProgramLocation {
    pub program_id: ProgramId,
    pub storage_id: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<NcmProgramLocation>(), 0x10);

pub(crate) mod proto {
    use nx_sf::ServiceName;

    /// Service name registered with `sm:`.
    pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:shell");

    /// `LaunchProgram` — launches a program, returning its process ID.
    pub const LAUNCH_PROGRAM: u32 = 0;
    /// `TerminateProcess` — terminates a process by PID.
    pub const TERMINATE_PROCESS: u32 = 1;
    /// `TerminateProgram` — terminates a program by program ID.
    pub const TERMINATE_PROGRAM: u32 = 2;
    /// `GetProcessEventHandle` — returns a copy-handle for the process event.
    pub const GET_PROCESS_EVENT_HANDLE: u32 = 3;
    /// `GetProcessEventInfo` — returns the current process event info.
    pub const GET_PROCESS_EVENT_INFO: u32 = 4;

    /// `CleanupProcess`.
    ///
    /// pre-5.0.0 only.
    pub const CLEANUP_PROCESS_LEGACY: u32 = 5;
    /// `ClearJitDebugOccurred`.
    ///
    /// pre-5.0.0 only.
    pub const CLEAR_JIT_DEBUG_OCCURRED_LEGACY: u32 = 6;

    /// `NotifyBootFinished`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const NOTIFY_BOOT_FINISHED_LEGACY: u32 = 7;
    /// `GetApplicationProcessIdForShell`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const GET_APPLICATION_PROCESS_ID_FOR_SHELL_LEGACY: u32 = 8;
    /// `BoostSystemMemoryResourceLimit`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT_LEGACY: u32 = 9;

    /// `NotifyBootFinished`.
    ///
    /// `[5.0.0+]`
    pub const NOTIFY_BOOT_FINISHED: u32 = 5;
    /// `GetApplicationProcessIdForShell`.
    ///
    /// `[5.0.0+]`
    pub const GET_APPLICATION_PROCESS_ID_FOR_SHELL: u32 = 6;
    /// `BoostSystemMemoryResourceLimit`.
    ///
    /// `[5.0.0+]`
    pub const BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT: u32 = 7;
    /// `BoostApplicationThreadResourceLimit`.
    ///
    /// `[7.0.0+/Atmosphere]`
    pub const BOOST_APPLICATION_THREAD_RESOURCE_LIMIT: u32 = 8;
    /// `BoostSystemThreadResourceLimit`.
    ///
    /// `[14.0.0+/Atmosphere]`
    pub const BOOST_SYSTEM_THREAD_RESOURCE_LIMIT: u32 = 10;
    /// `GetProcessId`.
    ///
    /// `[19.0.0+/Atmosphere]`
    pub const GET_PROCESS_ID: u32 = 12;
}
