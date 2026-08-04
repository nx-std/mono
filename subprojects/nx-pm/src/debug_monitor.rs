//! `pm:dmnt` (debug/monitor) service.

use nx_service_pm::{
    PmDmntService,
    ProcessId,
    ProgramId,
};
use nx_sf::service::DispatchError;
use nx_svc::raw::Handle;

use crate::event::{
    HookId,
    ProcessHook,
};

/// Connected `pm:dmnt` (debug/monitor) service.
///
/// Method pairs `*` / `*_legacy` correspond to the firmware `[5.0.0+]` and
/// pre-5.0.0 command renumbering — callers select based on the running
/// firmware. Modern-only commands (e.g. `ClearHook` on `[6.0.0+]`) are
/// exposed unconditionally and surface a kernel error on older firmware.
pub struct DebugMonitorService {
    inner: PmDmntService,
}

impl DebugMonitorService {
    pub(crate) fn new(inner: PmDmntService) -> Self {
        Self { inner }
    }

    /// Fills `out` with JIT-debug process IDs, returning the number of slots
    /// written.
    ///
    /// `[5.0.0+]`
    pub fn jit_debug_process_ids(
        &self,
        out: &mut [ProcessId],
    ) -> Result<usize, DmntGetJitDebugProcessIdListError> {
        self.inner
            .get_jit_debug_process_id_list(out)
            .map(|c| c as usize)
            .map_err(DmntGetJitDebugProcessIdListError)
    }

    /// Fills `out` with JIT-debug process IDs (pre-5.0.0).
    pub fn jit_debug_process_ids_legacy(
        &self,
        out: &mut [ProcessId],
    ) -> Result<usize, DmntGetJitDebugProcessIdListLegacyError> {
        self.inner
            .get_jit_debug_process_id_list_legacy(out)
            .map(|c| c as usize)
            .map_err(DmntGetJitDebugProcessIdListLegacyError)
    }

    /// Starts a previously-created process.
    ///
    /// `[5.0.0+]`
    pub fn start_process(&self, pid: ProcessId) -> Result<(), DmntStartProcessError> {
        self.inner.start_process(pid).map_err(DmntStartProcessError)
    }

    /// Starts a previously-created process (pre-5.0.0).
    pub fn start_process_legacy(&self, pid: ProcessId) -> Result<(), DmntStartProcessLegacyError> {
        self.inner
            .start_process_legacy(pid)
            .map_err(DmntStartProcessLegacyError)
    }

    /// Resolves a [`ProgramId`] to its running [`ProcessId`].
    ///
    /// `[5.0.0+]`
    pub fn process_id(&self, program_id: ProgramId) -> Result<ProcessId, DmntGetProcessIdError> {
        self.inner
            .get_process_id(program_id)
            .map_err(DmntGetProcessIdError)
    }

    /// Resolves a [`ProgramId`] to its running [`ProcessId`] (pre-5.0.0).
    pub fn process_id_legacy(
        &self,
        program_id: ProgramId,
    ) -> Result<ProcessId, DmntGetProcessIdLegacyError> {
        self.inner
            .get_process_id_legacy(program_id)
            .map_err(DmntGetProcessIdLegacyError)
    }

    /// Installs a process-creation hook for `program_id`.
    ///
    /// `[5.0.0+]`
    pub fn hook_create_process(
        &self,
        program_id: ProgramId,
    ) -> Result<ProcessHook, DmntHookToCreateProcessError> {
        let raw = self
            .inner
            .hook_to_create_process(program_id)
            .map_err(DmntHookToCreateProcessError)?;
        // SAFETY: The dispatch above returned a fresh hook copy-handle this process owns.
        Ok(ProcessHook::from_raw_unchecked(raw as Handle))
    }

    /// Installs a process-creation hook for `program_id` (pre-5.0.0).
    pub fn hook_create_process_legacy(
        &self,
        program_id: ProgramId,
    ) -> Result<ProcessHook, DmntHookToCreateProcessLegacyError> {
        let raw = self
            .inner
            .hook_to_create_process_legacy(program_id)
            .map_err(DmntHookToCreateProcessLegacyError)?;

        // SAFETY: The dispatch above returned a fresh hook copy-handle this process owns.
        Ok(ProcessHook::from_raw_unchecked(raw as Handle))
    }

    /// Returns the application's [`ProcessId`].
    ///
    /// `[5.0.0+]`
    pub fn application_process_id(&self) -> Result<ProcessId, DmntGetApplicationProcessIdError> {
        self.inner
            .get_application_process_id()
            .map_err(DmntGetApplicationProcessIdError)
    }

    /// Returns the application's [`ProcessId`] (pre-5.0.0).
    pub fn application_process_id_legacy(
        &self,
    ) -> Result<ProcessId, DmntGetApplicationProcessIdLegacyError> {
        self.inner
            .get_application_process_id_legacy()
            .map_err(DmntGetApplicationProcessIdLegacyError)
    }

    /// Installs an application-process-creation hook.
    ///
    /// `[5.0.0+]`
    pub fn hook_create_application_process(
        &self,
    ) -> Result<ProcessHook, DmntHookToCreateApplicationProcessError> {
        let raw = self
            .inner
            .hook_to_create_application_process()
            .map_err(DmntHookToCreateApplicationProcessError)?;
        // SAFETY: The dispatch above returned a fresh hook copy-handle this process owns.
        Ok(ProcessHook::from_raw_unchecked(raw as Handle))
    }

    /// Installs an application-process-creation hook (pre-5.0.0).
    pub fn hook_create_application_process_legacy(
        &self,
    ) -> Result<ProcessHook, DmntHookToCreateApplicationProcessLegacyError> {
        let raw = self
            .inner
            .hook_to_create_application_process_legacy()
            .map_err(DmntHookToCreateApplicationProcessLegacyError)?;
        // SAFETY: The dispatch above returned a fresh hook copy-handle this process owns.
        Ok(ProcessHook::from_raw_unchecked(raw as Handle))
    }

    /// Removes a previously installed hook.
    ///
    /// `[6.0.0+]`
    pub fn clear_hook(&self, id: HookId) -> Result<(), DmntClearHookError> {
        self.inner.clear_hook(id.0).map_err(DmntClearHookError)
    }

    /// Resolves a [`ProcessId`] to its [`ProgramId`].
    ///
    /// `[14.0.0+/Atmosphere]`
    pub fn program_id(&self, pid: ProcessId) -> Result<ProgramId, DmntGetProgramIdError> {
        self.inner
            .get_program_id(pid)
            .map_err(DmntGetProgramIdError)
    }
}

/// IPC dispatch failure from `pm:dmnt GetJitDebugProcessIdList`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetJitDebugProcessIdList IPC dispatch failed")]
pub struct DmntGetJitDebugProcessIdListError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt GetJitDebugProcessIdList` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetJitDebugProcessIdList (legacy) IPC dispatch failed")]
pub struct DmntGetJitDebugProcessIdListLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt StartProcess`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt StartProcess IPC dispatch failed")]
pub struct DmntStartProcessError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt StartProcess` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt StartProcess (legacy) IPC dispatch failed")]
pub struct DmntStartProcessLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt GetProcessId`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetProcessId IPC dispatch failed")]
pub struct DmntGetProcessIdError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt GetProcessId` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetProcessId (legacy) IPC dispatch failed")]
pub struct DmntGetProcessIdLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt HookToCreateProcess`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt HookToCreateProcess IPC dispatch failed")]
pub struct DmntHookToCreateProcessError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt HookToCreateProcess` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt HookToCreateProcess (legacy) IPC dispatch failed")]
pub struct DmntHookToCreateProcessLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt GetApplicationProcessId`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetApplicationProcessId IPC dispatch failed")]
pub struct DmntGetApplicationProcessIdError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt GetApplicationProcessId` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt GetApplicationProcessId (legacy) IPC dispatch failed")]
pub struct DmntGetApplicationProcessIdLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt HookToCreateApplicationProcess`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt HookToCreateApplicationProcess IPC dispatch failed")]
pub struct DmntHookToCreateApplicationProcessError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt HookToCreateApplicationProcess` (pre-5.0.0).
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt HookToCreateApplicationProcess (legacy) IPC dispatch failed")]
pub struct DmntHookToCreateApplicationProcessLegacyError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt ClearHook`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt ClearHook IPC dispatch failed")]
pub struct DmntClearHookError(#[source] pub DispatchError);

/// IPC dispatch failure from `pm:dmnt AtmosphereGetProgramId`.
#[derive(Debug, thiserror::Error)]
#[error("pm:dmnt AtmosphereGetProgramId IPC dispatch failed")]
pub struct DmntGetProgramIdError(#[source] pub DispatchError);
