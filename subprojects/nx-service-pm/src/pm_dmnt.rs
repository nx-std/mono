//! `pm:dmnt` (debug/monitor) service wrapper.

use nx_service_sm::SmService;
use nx_sf::{
    error::{ResultCode, ToResultCode},
    service::{DispatchError, Session},
};

use super::{
    cmif,
    types::{ProcessId, ProgramId},
};

/// Connected `pm:dmnt` (debug/monitor) service wrapper.
pub struct PmDmntService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmDmntService {}
unsafe impl Sync for PmDmntService {}

impl PmDmntService {
    /// Gets the JIT debug process ID list.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`get_jit_debug_process_id_list_legacy`](Self::get_jit_debug_process_id_list_legacy).
    #[inline]
    pub fn get_jit_debug_process_id_list(
        &self,
        out_pids: &mut [ProcessId],
    ) -> Result<u32, DispatchError> {
        cmif::get_jit_debug_process_id_list(&self.0, proto::GET_JIT_DEBUG_PROCESS_ID_LIST, out_pids)
    }

    /// Gets the JIT debug process ID list (legacy, pre-5.0.0).
    #[inline]
    pub fn get_jit_debug_process_id_list_legacy(
        &self,
        out_pids: &mut [ProcessId],
    ) -> Result<u32, DispatchError> {
        cmif::get_jit_debug_process_id_list(
            &self.0,
            proto::GET_JIT_DEBUG_PROCESS_ID_LIST_LEGACY,
            out_pids,
        )
    }

    /// Starts a process by PID.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`start_process_legacy`](Self::start_process_legacy).
    #[inline]
    pub fn start_process(&self, pid: ProcessId) -> Result<(), DispatchError> {
        cmif::start_process(&self.0, proto::START_PROCESS, pid)
    }

    /// Starts a process by PID (legacy, pre-5.0.0).
    #[inline]
    pub fn start_process_legacy(&self, pid: ProcessId) -> Result<(), DispatchError> {
        cmif::start_process(&self.0, proto::START_PROCESS_LEGACY, pid)
    }

    /// Gets a process ID from a program ID.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`get_process_id_legacy`](Self::get_process_id_legacy).
    #[inline]
    pub fn get_process_id(&self, program_id: ProgramId) -> Result<ProcessId, DispatchError> {
        cmif::get_process_id(&self.0, proto::GET_PROCESS_ID, program_id)
    }

    /// Gets a process ID from a program ID (legacy, pre-5.0.0).
    #[inline]
    pub fn get_process_id_legacy(&self, program_id: ProgramId) -> Result<ProcessId, DispatchError> {
        cmif::get_process_id(&self.0, proto::GET_PROCESS_ID_LEGACY, program_id)
    }

    /// Hooks to be notified when a specific program creates a process.
    ///
    /// `[5.0.0+]`
    ///
    /// Returns a copy-handle for the event.
    ///
    /// On pre-5.0.0, use [`hook_to_create_process_legacy`](Self::hook_to_create_process_legacy).
    #[inline]
    pub fn hook_to_create_process(&self, program_id: ProgramId) -> Result<u32, DispatchError> {
        cmif::hook_to_create_process(&self.0, proto::HOOK_TO_CREATE_PROCESS, program_id)
    }

    /// Hooks to be notified when a specific program creates a process
    /// (legacy, pre-5.0.0).
    ///
    /// Returns a copy-handle for the event.
    #[inline]
    pub fn hook_to_create_process_legacy(
        &self,
        program_id: ProgramId,
    ) -> Result<u32, DispatchError> {
        cmif::hook_to_create_process(&self.0, proto::HOOK_TO_CREATE_PROCESS_LEGACY, program_id)
    }

    /// Gets the application process ID.
    ///
    /// `[5.0.0+]`
    ///
    /// On pre-5.0.0, use [`get_application_process_id_legacy`](Self::get_application_process_id_legacy).
    #[inline]
    pub fn get_application_process_id(&self) -> Result<ProcessId, DispatchError> {
        cmif::get_application_process_id(&self.0, proto::GET_APPLICATION_PROCESS_ID)
    }

    /// Gets the application process ID (legacy, pre-5.0.0).
    #[inline]
    pub fn get_application_process_id_legacy(&self) -> Result<ProcessId, DispatchError> {
        cmif::get_application_process_id(&self.0, proto::GET_APPLICATION_PROCESS_ID_LEGACY)
    }

    /// Hooks to be notified when the application process is created.
    ///
    /// `[5.0.0+]`
    ///
    /// Returns a copy-handle for the event.
    ///
    /// On pre-5.0.0, use [`hook_to_create_application_process_legacy`](Self::hook_to_create_application_process_legacy).
    #[inline]
    pub fn hook_to_create_application_process(&self) -> Result<u32, DispatchError> {
        cmif::hook_to_create_application_process(&self.0, proto::HOOK_TO_CREATE_APPLICATION_PROCESS)
    }

    /// Hooks to be notified when the application process is created
    /// (legacy, pre-5.0.0).
    ///
    /// Returns a copy-handle for the event.
    #[inline]
    pub fn hook_to_create_application_process_legacy(&self) -> Result<u32, DispatchError> {
        cmif::hook_to_create_application_process(
            &self.0,
            proto::HOOK_TO_CREATE_APPLICATION_PROCESS_LEGACY,
        )
    }

    /// Clears a hook.
    ///
    /// `[6.0.0+]`
    #[inline]
    pub fn clear_hook(&self, which: u32) -> Result<(), DispatchError> {
        cmif::clear_hook(&self.0, which)
    }

    /// Gets a program ID from a PID.
    ///
    /// `[14.0.0+/Atmosphere]`
    #[inline]
    pub fn get_program_id(&self, pid: ProcessId) -> Result<ProgramId, DispatchError> {
        cmif::dmnt_get_program_id(&self.0, pid)
    }
}

#[cfg(feature = "ffi")]
impl PmDmntService {
    /// Returns the underlying session for libnx `Service*` shadow buffers.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.0
    }
}

/// Connects to the `pm:dmnt` (debug/monitor) service using CMIF.
pub fn connect_dmnt_cmif(sm: &SmService) -> Result<PmDmntService, ConnectDmntCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SERVICE_NAME)
        .map_err(ConnectDmntCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PmDmntService(service))
}

/// Error returned by [`connect_dmnt_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pm:dmnt service")]
pub struct ConnectDmntCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

impl ToResultCode for ConnectDmntCmifError {
    fn to_rc(self) -> ResultCode {
        self.0.to_rc()
    }
}

pub(crate) mod proto {
    use nx_sf::ServiceName;

    /// Service name registered with `sm:`.
    pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pm:dmnt");

    /// `GetJitDebugProcessIdList`.
    ///
    /// `[5.0.0+]`
    pub const GET_JIT_DEBUG_PROCESS_ID_LIST: u32 = 0;
    /// `StartProcess`.
    ///
    /// `[5.0.0+]`
    pub const START_PROCESS: u32 = 1;
    /// `GetProcessId`.
    ///
    /// `[5.0.0+]`
    pub const GET_PROCESS_ID: u32 = 2;
    /// `HookToCreateProcess`.
    ///
    /// `[5.0.0+]`
    pub const HOOK_TO_CREATE_PROCESS: u32 = 3;
    /// `GetApplicationProcessId`.
    ///
    /// `[5.0.0+]`
    pub const GET_APPLICATION_PROCESS_ID: u32 = 4;
    /// `HookToCreateApplicationProcess`.
    ///
    /// `[5.0.0+]`
    pub const HOOK_TO_CREATE_APPLICATION_PROCESS: u32 = 5;
    /// `ClearHook`.
    ///
    /// `[6.0.0+]`
    pub const CLEAR_HOOK: u32 = 6;
    /// `GetProgramId`.
    ///
    /// `[14.0.0+/Atmosphere]`
    pub const GET_PROGRAM_ID: u32 = 7;

    /// `GetJitDebugProcessIdList`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const GET_JIT_DEBUG_PROCESS_ID_LIST_LEGACY: u32 = 1;
    /// `StartProcess`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const START_PROCESS_LEGACY: u32 = 2;
    /// `GetProcessId`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const GET_PROCESS_ID_LEGACY: u32 = 3;
    /// `HookToCreateProcess`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const HOOK_TO_CREATE_PROCESS_LEGACY: u32 = 4;
    /// `GetApplicationProcessId`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const GET_APPLICATION_PROCESS_ID_LEGACY: u32 = 5;
    /// `HookToCreateApplicationProcess`.
    ///
    /// pre-5.0.0 legacy numbering.
    pub const HOOK_TO_CREATE_APPLICATION_PROCESS_LEGACY: u32 = 6;
}
