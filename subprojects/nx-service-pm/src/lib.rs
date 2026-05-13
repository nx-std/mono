//! Process manager (`pm`) service implementation.
//!
//! Provides access to the process manager services for launching programs,
//! managing processes, querying boot mode, and inspecting process state.
//!
//! ## Service Variants
//!
//! Four service endpoints are available:
//!
//! - **`pm:bm`** — Boot mode service. Connected via [`connect_bm_cmif`].
//! - **`pm:dmnt`** — Debug/monitor service. Connected via [`connect_dmnt_cmif`].
//! - **`pm:info`** — Process info service. Connected via [`connect_info_cmif`].
//! - **`pm:shell`** — Shell service. Connected via [`connect_shell_cmif`].
//!
//! ## Hosversion variants
//!
//! `pm:dmnt` and `pm:shell` commands were renumbered at `[5.0.0]`. Methods
//! that have different command IDs across versions are exposed as paired
//! `_legacy` (pre-5.0.0) and non-suffixed (5.0.0+) variants. The caller
//! selects based on the system version.
//!
//! Commands that only exist on newer firmware (e.g., `ClearHook` on 6.0.0+,
//! `BoostApplicationThreadResourceLimit` on 7.0.0+) are exposed
//! unconditionally.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    proto::{BM_SERVICE_NAME, DMNT_SERVICE_NAME, INFO_SERVICE_NAME, SHELL_SERVICE_NAME},
    types::{
        NcmProgramLocation, PmBootMode, PmLaunchFlag, PmLaunchFlagOld, PmProcessEvent,
        PmProcessEventInfo, PmResourceLimitValues,
    },
};

// ---------------------------------------------------------------------------
// Boot mode service
// ---------------------------------------------------------------------------

/// Connected `pm:bm` (boot mode) service wrapper.
pub struct PmBmService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmBmService {}
unsafe impl Sync for PmBmService {}

impl PmBmService {
    /// Consumes and closes the service session.

    /// Gets the current boot mode.
    #[inline]
    pub fn get_boot_mode(&self) -> Result<PmBootMode, DispatchError> {
        cmif::get_boot_mode(&self.0)
    }

    /// Sets the boot mode to maintenance.
    #[inline]
    pub fn set_maintenance_boot(&self) -> Result<(), DispatchError> {
        cmif::set_maintenance_boot(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Debug/monitor service
// ---------------------------------------------------------------------------

/// Connected `pm:dmnt` (debug/monitor) service wrapper.
pub struct PmDmntService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmDmntService {}
unsafe impl Sync for PmDmntService {}

impl PmDmntService {
    /// Consumes and closes the service session.

    /// Gets the JIT debug process ID list (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`get_jit_debug_process_id_list_legacy`](Self::get_jit_debug_process_id_list_legacy).
    #[inline]
    pub fn get_jit_debug_process_id_list(
        &self,
        out_pids: &mut [u64],
    ) -> Result<u32, DispatchError> {
        cmif::get_jit_debug_process_id_list(
            &self.0,
            proto::DMNT_GET_JIT_DEBUG_PROCESS_ID_LIST,
            out_pids,
        )
    }

    /// Gets the JIT debug process ID list (legacy, pre-5.0.0).
    #[inline]
    pub fn get_jit_debug_process_id_list_legacy(
        &self,
        out_pids: &mut [u64],
    ) -> Result<u32, DispatchError> {
        cmif::get_jit_debug_process_id_list(
            &self.0,
            proto::DMNT_GET_JIT_DEBUG_PROCESS_ID_LIST_LEGACY,
            out_pids,
        )
    }

    /// Starts a process by PID (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`start_process_legacy`](Self::start_process_legacy).
    #[inline]
    pub fn start_process(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::start_process(&self.0, proto::DMNT_START_PROCESS, pid)
    }

    /// Starts a process by PID (legacy, pre-5.0.0).
    #[inline]
    pub fn start_process_legacy(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::start_process(&self.0, proto::DMNT_START_PROCESS_LEGACY, pid)
    }

    /// Gets a process ID from a program ID (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`get_process_id_legacy`](Self::get_process_id_legacy).
    #[inline]
    pub fn get_process_id(&self, program_id: u64) -> Result<u64, DispatchError> {
        cmif::get_process_id(&self.0, proto::DMNT_GET_PROCESS_ID, program_id)
    }

    /// Gets a process ID from a program ID (legacy, pre-5.0.0).
    #[inline]
    pub fn get_process_id_legacy(&self, program_id: u64) -> Result<u64, DispatchError> {
        cmif::get_process_id(&self.0, proto::DMNT_GET_PROCESS_ID_LEGACY, program_id)
    }

    /// Hooks to be notified when a specific program creates a process (`[5.0.0+]`).
    ///
    /// Returns a copy-handle for the event.
    ///
    /// On pre-5.0.0, use [`hook_to_create_process_legacy`](Self::hook_to_create_process_legacy).
    #[inline]
    pub fn hook_to_create_process(&self, program_id: u64) -> Result<u32, DispatchError> {
        cmif::hook_to_create_process(&self.0, proto::DMNT_HOOK_TO_CREATE_PROCESS, program_id)
    }

    /// Hooks to be notified when a specific program creates a process
    /// (legacy, pre-5.0.0).
    ///
    /// Returns a copy-handle for the event.
    #[inline]
    pub fn hook_to_create_process_legacy(&self, program_id: u64) -> Result<u32, DispatchError> {
        cmif::hook_to_create_process(
            &self.0,
            proto::DMNT_HOOK_TO_CREATE_PROCESS_LEGACY,
            program_id,
        )
    }

    /// Gets the application process ID (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`get_application_process_id_legacy`](Self::get_application_process_id_legacy).
    #[inline]
    pub fn get_application_process_id(&self) -> Result<u64, DispatchError> {
        cmif::get_application_process_id(&self.0, proto::DMNT_GET_APPLICATION_PROCESS_ID)
    }

    /// Gets the application process ID (legacy, pre-5.0.0).
    #[inline]
    pub fn get_application_process_id_legacy(&self) -> Result<u64, DispatchError> {
        cmif::get_application_process_id(&self.0, proto::DMNT_GET_APPLICATION_PROCESS_ID_LEGACY)
    }

    /// Hooks to be notified when the application process is created (`[5.0.0+]`).
    ///
    /// Returns a copy-handle for the event.
    ///
    /// On pre-5.0.0, use [`hook_to_create_application_process_legacy`](Self::hook_to_create_application_process_legacy).
    #[inline]
    pub fn hook_to_create_application_process(&self) -> Result<u32, DispatchError> {
        cmif::hook_to_create_application_process(
            &self.0,
            proto::DMNT_HOOK_TO_CREATE_APPLICATION_PROCESS,
        )
    }

    /// Hooks to be notified when the application process is created
    /// (legacy, pre-5.0.0).
    ///
    /// Returns a copy-handle for the event.
    #[inline]
    pub fn hook_to_create_application_process_legacy(&self) -> Result<u32, DispatchError> {
        cmif::hook_to_create_application_process(
            &self.0,
            proto::DMNT_HOOK_TO_CREATE_APPLICATION_PROCESS_LEGACY,
        )
    }

    /// Clears a hook (`[6.0.0+]`).
    #[inline]
    pub fn clear_hook(&self, which: u32) -> Result<(), DispatchError> {
        cmif::clear_hook(&self.0, which)
    }

    /// Gets a program ID from a PID (`[14.0.0+/Atmosphere]`).
    #[inline]
    pub fn get_program_id(&self, pid: u64) -> Result<u64, DispatchError> {
        cmif::dmnt_get_program_id(&self.0, pid)
    }
}

// ---------------------------------------------------------------------------
// Info service
// ---------------------------------------------------------------------------

/// Connected `pm:info` (process info) service wrapper.
pub struct PmInfoService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmInfoService {}
unsafe impl Sync for PmInfoService {}

impl PmInfoService {
    /// Consumes and closes the service session.

    /// Gets a program ID from a process ID.
    #[inline]
    pub fn get_program_id(&self, pid: u64) -> Result<u64, DispatchError> {
        cmif::info_get_program_id(&self.0, pid)
    }

    /// Gets the applet's current resource limit values (`[14.0.0+/Atmosphere]`).
    #[inline]
    pub fn get_applet_current_resource_limit_values(
        &self,
    ) -> Result<PmResourceLimitValues, DispatchError> {
        cmif::get_applet_current_resource_limit_values(&self.0)
    }

    /// Gets the applet's peak resource limit values (`[14.0.0+/Atmosphere]`).
    #[inline]
    pub fn get_applet_peak_resource_limit_values(
        &self,
    ) -> Result<PmResourceLimitValues, DispatchError> {
        cmif::get_applet_peak_resource_limit_values(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Shell service
// ---------------------------------------------------------------------------

/// Connected `pm:shell` (shell) service wrapper.
pub struct PmShellService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PmShellService {}
unsafe impl Sync for PmShellService {}

impl PmShellService {
    /// Consumes and closes the service session.

    /// Launches a program.
    ///
    /// Returns the launched process ID.
    #[inline]
    pub fn launch_program(
        &self,
        launch_flags: u32,
        location: &NcmProgramLocation,
    ) -> Result<u64, DispatchError> {
        cmif::launch_program(&self.0, launch_flags, location)
    }

    /// Terminates a process by PID.
    #[inline]
    pub fn terminate_process(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::terminate_process(&self.0, pid)
    }

    /// Terminates a program by program ID.
    #[inline]
    pub fn terminate_program(&self, program_id: u64) -> Result<(), DispatchError> {
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
    pub fn get_process_event_info(&self) -> Result<PmProcessEventInfo, DispatchError> {
        cmif::get_process_event_info(&self.0)
    }

    /// Cleans up a process (pre-5.0.0 only, cmd 5).
    #[inline]
    pub fn cleanup_process(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::cleanup_process(&self.0, pid)
    }

    /// Clears the JIT debug occurred flag (pre-5.0.0 only, cmd 6).
    #[inline]
    pub fn clear_jit_debug_occurred(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::clear_jit_debug_occurred(&self.0, pid)
    }

    /// Notifies the system that boot has finished (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`notify_boot_finished_legacy`](Self::notify_boot_finished_legacy).
    #[inline]
    pub fn notify_boot_finished(&self) -> Result<(), DispatchError> {
        cmif::notify_boot_finished(&self.0, proto::SHELL_NOTIFY_BOOT_FINISHED)
    }

    /// Notifies the system that boot has finished (legacy, pre-5.0.0).
    #[inline]
    pub fn notify_boot_finished_legacy(&self) -> Result<(), DispatchError> {
        cmif::notify_boot_finished(&self.0, proto::SHELL_NOTIFY_BOOT_FINISHED_LEGACY)
    }

    /// Gets the application process ID for shell (`[5.0.0+]`).
    ///
    /// On pre-5.0.0, use [`get_application_process_id_for_shell_legacy`](Self::get_application_process_id_for_shell_legacy).
    #[inline]
    pub fn get_application_process_id_for_shell(&self) -> Result<u64, DispatchError> {
        cmif::get_application_process_id_for_shell(
            &self.0,
            proto::SHELL_GET_APPLICATION_PROCESS_ID_FOR_SHELL,
        )
    }

    /// Gets the application process ID for shell (legacy, pre-5.0.0).
    #[inline]
    pub fn get_application_process_id_for_shell_legacy(&self) -> Result<u64, DispatchError> {
        cmif::get_application_process_id_for_shell(
            &self.0,
            proto::SHELL_GET_APPLICATION_PROCESS_ID_FOR_SHELL_LEGACY,
        )
    }

    /// Boosts the system memory resource limit (`[5.0.0+]`).
    ///
    /// On `[4.0.0–4.1.0]`, use [`boost_system_memory_resource_limit_legacy`](Self::boost_system_memory_resource_limit_legacy).
    #[inline]
    pub fn boost_system_memory_resource_limit(&self, boost_size: u64) -> Result<(), DispatchError> {
        cmif::boost_system_memory_resource_limit(
            &self.0,
            proto::SHELL_BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT,
            boost_size,
        )
    }

    /// Boosts the system memory resource limit (legacy, `[4.0.0–4.1.0]`).
    #[inline]
    pub fn boost_system_memory_resource_limit_legacy(
        &self,
        boost_size: u64,
    ) -> Result<(), DispatchError> {
        cmif::boost_system_memory_resource_limit(
            &self.0,
            proto::SHELL_BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT_LEGACY,
            boost_size,
        )
    }

    /// Boosts the application thread resource limit (`[7.0.0+/Atmosphere]`).
    #[inline]
    pub fn boost_application_thread_resource_limit(&self) -> Result<(), DispatchError> {
        cmif::boost_application_thread_resource_limit(&self.0)
    }

    /// Boosts the system thread resource limit (`[14.0.0+/Atmosphere]`).
    #[inline]
    pub fn boost_system_thread_resource_limit(&self) -> Result<(), DispatchError> {
        cmif::boost_system_thread_resource_limit(&self.0)
    }

    /// Gets a process ID from a program ID (`[19.0.0+/Atmosphere]`).
    #[inline]
    pub fn get_process_id(&self, program_id: u64) -> Result<u64, DispatchError> {
        cmif::shell_get_process_id(&self.0, program_id)
    }
}

// ---------------------------------------------------------------------------
// Connection functions
// ---------------------------------------------------------------------------

/// Connects to the `pm:bm` (boot mode) service using CMIF.
pub fn connect_bm_cmif(sm: &SmService) -> Result<PmBmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::BM_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PmBmService(service))
}

/// Connects to the `pm:dmnt` (debug/monitor) service using CMIF.
pub fn connect_dmnt_cmif(sm: &SmService) -> Result<PmDmntService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::DMNT_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PmDmntService(service))
}

/// Connects to the `pm:info` (process info) service using CMIF.
pub fn connect_info_cmif(sm: &SmService) -> Result<PmInfoService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::INFO_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PmInfoService(service))
}

/// Connects to the `pm:shell` (shell) service using CMIF.
pub fn connect_shell_cmif(sm: &SmService) -> Result<PmShellService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SHELL_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PmShellService(service))
}

/// Error returned by connection functions.
#[derive(Debug, thiserror::Error)]
#[error("failed to get pm service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
