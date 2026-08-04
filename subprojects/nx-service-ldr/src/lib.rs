//! Loader (`ldr`) service implementation.
//!
//! Provides access to the loader services for setting program arguments,
//! creating processes, querying program information, and module inspection.
//!
//! ## Service Variants
//!
//! Three service endpoints are available:
//!
//! - **`ldr:shel`** — Shell loader service. Connected via [`connect_shell_cmif`].
//! - **`ldr:dmnt`** — Debug/monitor loader service. Connected via
//!   [`connect_dmnt_cmif`]. Extends the shell command set with
//!   `GetProcessModuleInfo`.
//! - **`ldr:pm`** — Process manager loader service. Connected via
//!   [`connect_pm_cmif`]. Manages process creation, program info, and
//!   pin/unpin.
//!
//! ## Hosversion variants
//!
//! - `SetProgramArguments` has two wire formats: `set_program_arguments_legacy`
//!   (pre-11.0.0) and `set_program_arguments` (11.0.0+).
//! - `CreateProcess` has two wire formats: `create_process_legacy`
//!   (pre-20.0.0/non-Atmosphere) and `create_process` (20.0.0+/Atmosphere).
//! - `GetProgramInfo` has two wire formats: `get_program_info_v1`
//!   (1.0.0–18.1.0/non-Atmosphere) and `get_program_info`
//!   (19.0.0+/Atmosphere).
//! - `SetEnabledProgramVerification` is only available on `[10.0.0+]`.
//!
//! All variants are exposed unconditionally; the caller selects based on
//! system version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    DispatchError,
    Session,
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    proto::{
        DMNT_SERVICE_NAME,
        PM_SERVICE_NAME,
        SHELL_SERVICE_NAME,
    },
    types::{
        LoaderModuleInfo,
        LoaderProgramAttributes,
        LoaderProgramInfo,
        LoaderProgramInfoV1,
        NcmProgramLocation,
    },
};

// ---------------------------------------------------------------------------
// Shell service
// ---------------------------------------------------------------------------

/// Connected `ldr:shel` (shell) loader service wrapper.
pub struct LdrShellService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for LdrShellService {}
unsafe impl Sync for LdrShellService {}

impl LdrShellService {
    /// Sets program arguments (`[11.0.0+]`).
    ///
    /// On pre-11.0.0, use [`set_program_arguments_legacy`](Self::set_program_arguments_legacy).
    #[inline]
    pub fn set_program_arguments(&self, program_id: u64, args: &[u8]) -> Result<(), DispatchError> {
        cmif::set_program_arguments(&self.0, program_id, args)
    }

    /// Sets program arguments (legacy, pre-11.0.0).
    #[inline]
    pub fn set_program_arguments_legacy(
        &self,
        program_id: u64,
        args: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_program_arguments_legacy(&self.0, program_id, args)
    }

    /// Flushes all program arguments.
    #[inline]
    pub fn flush_arguments(&self) -> Result<(), DispatchError> {
        cmif::flush_arguments(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Dmnt service
// ---------------------------------------------------------------------------

/// Connected `ldr:dmnt` (debug/monitor) loader service wrapper.
///
/// Extends the shell command set with `GetProcessModuleInfo`.
pub struct LdrDmntService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for LdrDmntService {}
unsafe impl Sync for LdrDmntService {}

impl LdrDmntService {
    /// Sets program arguments (`[11.0.0+]`).
    #[inline]
    pub fn set_program_arguments(&self, program_id: u64, args: &[u8]) -> Result<(), DispatchError> {
        cmif::set_program_arguments(&self.0, program_id, args)
    }

    /// Sets program arguments (legacy, pre-11.0.0).
    #[inline]
    pub fn set_program_arguments_legacy(
        &self,
        program_id: u64,
        args: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::set_program_arguments_legacy(&self.0, program_id, args)
    }

    /// Flushes all program arguments.
    #[inline]
    pub fn flush_arguments(&self) -> Result<(), DispatchError> {
        cmif::flush_arguments(&self.0)
    }

    /// Gets module information for a process.
    ///
    /// Fills `out_modules` with information about loaded modules and returns
    /// the number of entries written.
    #[inline]
    pub fn get_process_module_info(
        &self,
        pid: u64,
        out_modules: &mut [LoaderModuleInfo],
    ) -> Result<i32, DispatchError> {
        cmif::get_process_module_info(&self.0, pid, out_modules)
    }
}

// ---------------------------------------------------------------------------
// Pm service
// ---------------------------------------------------------------------------

/// Connected `ldr:pm` (process manager) loader service wrapper.
pub struct LdrPmService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for LdrPmService {}
unsafe impl Sync for LdrPmService {}

impl LdrPmService {
    /// Creates a process (legacy, pre-20.0.0/non-Atmosphere).
    ///
    /// Returns the process handle on success.
    #[inline]
    pub fn create_process_legacy(
        &self,
        pin_id: u64,
        flags: u32,
        reslimit_handle: u32,
    ) -> Result<u32, DispatchError> {
        cmif::create_process_legacy(&self.0, pin_id, flags, reslimit_handle)
    }

    /// Creates a process (`[20.0.0+/Atmosphere]`).
    ///
    /// Returns the process handle on success.
    #[inline]
    pub fn create_process(
        &self,
        pin_id: u64,
        flags: u32,
        reslimit_handle: u32,
        attrs: &LoaderProgramAttributes,
    ) -> Result<u32, DispatchError> {
        cmif::create_process(&self.0, pin_id, flags, reslimit_handle, attrs)
    }

    /// Gets program info (legacy, `[1.0.0–18.1.0]`, non-Atmosphere).
    #[inline]
    pub fn get_program_info_v1(
        &self,
        loc: &NcmProgramLocation,
        out: &mut LoaderProgramInfoV1,
    ) -> Result<(), DispatchError> {
        cmif::get_program_info_v1(&self.0, loc, out)
    }

    /// Gets program info (`[19.0.0+/Atmosphere]`).
    #[inline]
    pub fn get_program_info(
        &self,
        loc: &NcmProgramLocation,
        attrs: &LoaderProgramAttributes,
        out: &mut LoaderProgramInfo,
    ) -> Result<(), DispatchError> {
        cmif::get_program_info(&self.0, loc, attrs, out)
    }

    /// Pins a program, returning a pin ID.
    #[inline]
    pub fn pin_program(&self, loc: &NcmProgramLocation) -> Result<u64, DispatchError> {
        cmif::pin_program(&self.0, loc)
    }

    /// Unpins a previously pinned program.
    #[inline]
    pub fn unpin_program(&self, pin_id: u64) -> Result<(), DispatchError> {
        cmif::unpin_program(&self.0, pin_id)
    }

    /// Enables or disables program verification (`[10.0.0+]`).
    #[inline]
    pub fn set_enabled_program_verification(&self, enabled: bool) -> Result<(), DispatchError> {
        cmif::set_enabled_program_verification(&self.0, enabled)
    }
}

// ---------------------------------------------------------------------------
// Connection functions
// ---------------------------------------------------------------------------

/// Connects to the `ldr:shel` (shell) loader service using CMIF.
pub fn connect_shell_cmif(sm: &SmService) -> Result<LdrShellService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::SHELL_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(LdrShellService(service))
}

/// Connects to the `ldr:dmnt` (debug/monitor) loader service using CMIF.
pub fn connect_dmnt_cmif(sm: &SmService) -> Result<LdrDmntService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::DMNT_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(LdrDmntService(service))
}

/// Connects to the `ldr:pm` (process manager) loader service using CMIF.
pub fn connect_pm_cmif(sm: &SmService) -> Result<LdrPmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::PM_SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(LdrPmService(service))
}

/// Error returned by connection functions.
#[derive(Debug, thiserror::Error)]
#[error("failed to get ldr service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
