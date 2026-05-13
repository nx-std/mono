//! Program Launch service (`pgl`) implementation.
//!
//! Provides program launch, termination, and process event observation on the
//! Switch. Available on HOS 10.0.0+.
//!
//! ## Protocol Support
//!
//! The PGL service supports two IPC protocols:
//! - **CMIF**: Available on HOS 10.0.0–11.x (pre-12.0.0).
//! - **TIPC**: Available on HOS 12.0.0+.
//!
//! Protocol selection is the caller's responsibility. Use the `_cmif` or
//! `_tipc` method variants on [`PglService`] as appropriate for the system
//! version.
//!
//! ## CMIF-Only Commands
//!
//! [`PglService::trigger_application_snapshot_dumper`] (cmd 12) is only
//! available via CMIF (pre-12.0.0). It has no TIPC variant.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod tipc;
mod types;

pub use self::{
    cmif::{
        DispatchError as DispatchCmifError, GetEventObserverError as GetEventObserverCmifError,
        GetProcessEventError as GetProcessEventCmifError,
    },
    proto::SERVICE_NAME,
    tipc::{
        DispatchError as DispatchTipcError, GetEventObserverError as GetEventObserverTipcError,
        GetProcessEventError as GetProcessEventTipcError,
    },
    types::{
        ContentMetaInfo, NcmProgramLocation, PglLaunchFlag, ProcessEvent, ProcessEventInfo,
        SnapShotDumpType,
    },
};

/// PGL service session wrapper.
///
/// Provides type safety to distinguish PGL sessions from regular services.
#[repr(transparent)]
pub struct PglService(Session);

impl PglService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods (HOS 10.0.0–11.x).
impl PglService {
    /// Launches a program (cmd 0, CMIF).
    #[inline]
    pub fn launch_program_cmif(
        &self,
        loc: &NcmProgramLocation,
        pm_launch_flags: u32,
        pgl_launch_flags: PglLaunchFlag,
    ) -> Result<u64, DispatchCmifError> {
        cmif::launch_program(self.0.handle(), loc, pm_launch_flags, pgl_launch_flags)
    }

    /// Terminates a process (cmd 1, CMIF).
    #[inline]
    pub fn terminate_process_cmif(&self, pid: u64) -> Result<(), DispatchCmifError> {
        cmif::terminate_process(self.0.handle(), pid)
    }

    /// Launches a program from a host content path (cmd 2, CMIF).
    #[inline]
    pub fn launch_program_from_host_cmif(
        &self,
        content_path: &[u8],
        pm_launch_flags: u32,
    ) -> Result<u64, DispatchCmifError> {
        cmif::launch_program_from_host(self.0.handle(), content_path, pm_launch_flags)
    }

    /// Gets host content meta info (cmd 4, CMIF).
    #[inline]
    pub fn get_host_content_meta_info_cmif(
        &self,
        content_path: &[u8],
    ) -> Result<ContentMetaInfo, DispatchCmifError> {
        cmif::get_host_content_meta_info(self.0.handle(), content_path)
    }

    /// Gets the application process ID (cmd 5, CMIF).
    #[inline]
    pub fn get_application_process_id_cmif(&self) -> Result<u64, DispatchCmifError> {
        cmif::get_application_process_id(self.0.handle())
    }

    /// Boosts system memory resource limit (cmd 6, CMIF).
    #[inline]
    pub fn boost_system_memory_resource_limit_cmif(
        &self,
        size: u64,
    ) -> Result<(), DispatchCmifError> {
        cmif::boost_system_memory_resource_limit(self.0.handle(), size)
    }

    /// Checks whether a process is tracked (cmd 7, CMIF).
    #[inline]
    pub fn is_process_tracked_cmif(&self, pid: u64) -> Result<bool, DispatchCmifError> {
        cmif::is_process_tracked(self.0.handle(), pid)
    }

    /// Enables/disables application crash reports (cmd 8, CMIF).
    #[inline]
    pub fn enable_application_crash_report_cmif(
        &self,
        enable: bool,
    ) -> Result<(), DispatchCmifError> {
        cmif::enable_application_crash_report(self.0.handle(), enable)
    }

    /// Checks whether application crash reports are enabled (cmd 9, CMIF).
    #[inline]
    pub fn is_application_crash_report_enabled_cmif(&self) -> Result<bool, DispatchCmifError> {
        cmif::is_application_crash_report_enabled(self.0.handle())
    }

    /// Enables/disables all-thread dump on crash (cmd 10, CMIF).
    #[inline]
    pub fn enable_application_all_thread_dump_on_crash_cmif(
        &self,
        enable: bool,
    ) -> Result<(), DispatchCmifError> {
        cmif::enable_application_all_thread_dump_on_crash(self.0.handle(), enable)
    }

    /// Triggers the application snapshot dumper (cmd 12, CMIF-only / pre-12.0.0).
    #[inline]
    pub fn trigger_application_snapshot_dumper(
        &self,
        dump_type: SnapShotDumpType,
        arg: &[u8],
    ) -> Result<(), DispatchCmifError> {
        cmif::trigger_application_snapshot_dumper(self.0.handle(), dump_type, arg)
    }

    /// Gets an event observer sub-object (cmd 20, CMIF).
    #[inline]
    pub fn get_event_observer_cmif(&self) -> Result<PglEventObserver, GetEventObserverCmifError> {
        let service = cmif::get_event_observer(self.0.handle())?;
        Ok(PglEventObserver(service))
    }
}

/// TIPC protocol methods (HOS 12.0.0+).
impl PglService {
    /// Launches a program (cmd 0, TIPC).
    #[inline]
    pub fn launch_program_tipc(
        &self,
        loc: &NcmProgramLocation,
        pm_launch_flags: u32,
        pgl_launch_flags: PglLaunchFlag,
    ) -> Result<u64, DispatchTipcError> {
        tipc::launch_program(self.0.handle(), loc, pm_launch_flags, pgl_launch_flags)
    }

    /// Terminates a process (cmd 1, TIPC).
    #[inline]
    pub fn terminate_process_tipc(&self, pid: u64) -> Result<(), DispatchTipcError> {
        tipc::terminate_process(self.0.handle(), pid)
    }

    /// Launches a program from a host content path (cmd 2, TIPC).
    #[inline]
    pub fn launch_program_from_host_tipc(
        &self,
        content_path: &[u8],
        pm_launch_flags: u32,
    ) -> Result<u64, DispatchTipcError> {
        tipc::launch_program_from_host(self.0.handle(), content_path, pm_launch_flags)
    }

    /// Gets host content meta info (cmd 4, TIPC).
    #[inline]
    pub fn get_host_content_meta_info_tipc(
        &self,
        content_path: &[u8],
    ) -> Result<ContentMetaInfo, DispatchTipcError> {
        tipc::get_host_content_meta_info(self.0.handle(), content_path)
    }

    /// Gets the application process ID (cmd 5, TIPC).
    #[inline]
    pub fn get_application_process_id_tipc(&self) -> Result<u64, DispatchTipcError> {
        tipc::get_application_process_id(self.0.handle())
    }

    /// Boosts system memory resource limit (cmd 6, TIPC).
    #[inline]
    pub fn boost_system_memory_resource_limit_tipc(
        &self,
        size: u64,
    ) -> Result<(), DispatchTipcError> {
        tipc::boost_system_memory_resource_limit(self.0.handle(), size)
    }

    /// Checks whether a process is tracked (cmd 7, TIPC).
    #[inline]
    pub fn is_process_tracked_tipc(&self, pid: u64) -> Result<bool, DispatchTipcError> {
        tipc::is_process_tracked(self.0.handle(), pid)
    }

    /// Enables/disables application crash reports (cmd 8, TIPC).
    #[inline]
    pub fn enable_application_crash_report_tipc(
        &self,
        enable: bool,
    ) -> Result<(), DispatchTipcError> {
        tipc::enable_application_crash_report(self.0.handle(), enable)
    }

    /// Checks whether application crash reports are enabled (cmd 9, TIPC).
    #[inline]
    pub fn is_application_crash_report_enabled_tipc(&self) -> Result<bool, DispatchTipcError> {
        tipc::is_application_crash_report_enabled(self.0.handle())
    }

    /// Enables/disables all-thread dump on crash (cmd 10, TIPC).
    #[inline]
    pub fn enable_application_all_thread_dump_on_crash_tipc(
        &self,
        enable: bool,
    ) -> Result<(), DispatchTipcError> {
        tipc::enable_application_all_thread_dump_on_crash(self.0.handle(), enable)
    }

    /// Gets an event observer sub-object (cmd 20, TIPC).
    #[inline]
    pub fn get_event_observer_tipc(&self) -> Result<PglEventObserver, GetEventObserverTipcError> {
        let service = tipc::get_event_observer(self.0.handle())?;
        Ok(PglEventObserver(service))
    }
}

/// PGL event observer sub-object.
///
/// Returned by [`PglService::get_event_observer_cmif`] or
/// [`PglService::get_event_observer_tipc`]. Provides process lifecycle event
/// monitoring.
pub struct PglEventObserver(Session);

impl PglEventObserver {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the event observer.
impl PglEventObserver {
    /// Gets the process event handle (cmd 0, CMIF, copy handle, autoclear=true).
    #[inline]
    pub fn get_process_event_cmif(&self) -> Result<u32, GetProcessEventCmifError> {
        cmif::observer_get_process_event(self.0.handle())
    }

    /// Gets the process event info (cmd 1, CMIF).
    #[inline]
    pub fn get_process_event_info_cmif(&self) -> Result<ProcessEventInfo, DispatchCmifError> {
        cmif::observer_get_process_event_info(self.0.handle())
    }
}

/// TIPC protocol methods for the event observer.
impl PglEventObserver {
    /// Gets the process event handle (cmd 0, TIPC, copy handle, autoclear=true).
    #[inline]
    pub fn get_process_event_tipc(&self) -> Result<u32, GetProcessEventTipcError> {
        tipc::observer_get_process_event(self.0.handle())
    }

    /// Gets the process event info (cmd 1, TIPC).
    #[inline]
    pub fn get_process_event_info_tipc(&self) -> Result<ProcessEventInfo, DispatchTipcError> {
        tipc::observer_get_process_event_info(self.0.handle())
    }
}

/// Connects to the PGL service using CMIF.
///
/// The caller is responsible for using CMIF methods on the returned service.
/// This is the appropriate connect function for HOS 10.0.0–11.x.
pub fn connect_cmif(sm: &SmService) -> Result<PglService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PglService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pgl service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the PGL service using TIPC.
///
/// The caller is responsible for using TIPC methods on the returned service.
/// This is the appropriate connect function for HOS 12.0.0+.
pub fn connect_tipc(sm: &SmService) -> Result<PglService, ConnectTipcError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectTipcError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PglService(service))
}

/// Error returned by [`connect_tipc`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pgl service")]
pub struct ConnectTipcError(#[source] pub nx_service_sm::GetServiceCmifError);
