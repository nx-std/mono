//! Relocatable Object (RO) service implementation.
//!
//! Provides access to the runtime object loading services for loading and
//! unloading NRO/NRR modules at runtime.
//!
//! ## Service Variants
//!
//! Three service endpoints are available:
//!
//! - **`ldr:ro`** — The primary loader RO service. Connected via
//!   [`connect_ldr_ro_cmif`].
//! - **`ro:1`** — Alternative RO service available on `[7.0.0+]`. Connected
//!   via [`connect_ro1_cmif`].
//! - **`ro:dmnt`** — Debug/monitor service for querying loaded module
//!   information. Available on `[3.0.0+]`. Connected via
//!   [`connect_ro_dmnt_cmif`].
//!
//! ## Divergence from libnx
//!
//! libnx's `ro.c` keeps three guarded global singletons (`g_roSrv`,
//! `g_ro1Srv`, `g_dmntSrv`) managed by `NX_GENERATE_SERVICE_GUARD`, and
//! auto-calls `_rosrvInitialize` (cmd 4) during initialization for
//! `ldr:ro` and `ro:1`. This crate separates connection from
//! initialization: [`connect_ldr_ro_cmif`] and [`connect_ro1_cmif`] call
//! the `Initialize` command automatically during connection, matching
//! the libnx behavior.
//!
//! Per IC-4, this crate is hosversion-unaware. The `LoadNrrEx` command
//! (cmd 10, `[7.0.0+]`) is exposed alongside `LoadNrr` (cmd 2) and the
//! caller selects the appropriate variant.

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
        LDR_RO_SERVICE_NAME,
        RO_DMNT_SERVICE_NAME,
        RO1_SERVICE_NAME,
    },
    types::LoaderModuleInfo,
};

/// Connected RO service wrapper (`ldr:ro` or `ro:1`).
///
/// Both `ldr:ro` and `ro:1` expose the same command interface for loading
/// and unloading NRO/NRR modules. The service is non-domain.
pub struct RoService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for RoService {}
unsafe impl Sync for RoService {}

impl RoService {
    /// Loads an NRO module into the process address space.
    ///
    /// `nro_address` and `bss_address` are addresses within the process's
    /// memory where the NRO and BSS sections have been mapped.
    ///
    /// Returns the load address on success.
    #[inline]
    pub fn load_nro(
        &self,
        nro_address: u64,
        nro_size: u64,
        bss_address: u64,
        bss_size: u64,
    ) -> Result<u64, DispatchError> {
        cmif::load_nro(&self.0, nro_address, nro_size, bss_address, bss_size)
    }

    /// Unloads a previously loaded NRO module.
    #[inline]
    pub fn unload_nro(&self, nro_address: u64) -> Result<(), DispatchError> {
        cmif::unload_nro(&self.0, nro_address)
    }

    /// Loads an NRR (NRO Registration Record).
    #[inline]
    pub fn load_nrr(&self, nrr_address: u64, nrr_size: u64) -> Result<(), DispatchError> {
        cmif::load_nrr(&self.0, nrr_address, nrr_size)
    }

    /// Unloads a previously loaded NRR.
    #[inline]
    pub fn unload_nrr(&self, nrr_address: u64) -> Result<(), DispatchError> {
        cmif::unload_nrr(&self.0, nrr_address)
    }

    /// Loads an NRR with extended validation (`[7.0.0+]`).
    ///
    /// On pre-7.0.0, use [`load_nrr`](Self::load_nrr) instead.
    #[inline]
    pub fn load_nrr_ex(&self, nrr_address: u64, nrr_size: u64) -> Result<(), DispatchError> {
        cmif::load_nrr_ex(&self.0, nrr_address, nrr_size)
    }
}

/// Connected `ro:dmnt` debug/monitor service wrapper.
///
/// Provides access to module information for arbitrary processes.
pub struct RoDmntService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for RoDmntService {}
unsafe impl Sync for RoDmntService {}

impl RoDmntService {
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

/// Connects to the `ldr:ro` service using CMIF.
///
/// Performs the SM lookup and automatically calls the `Initialize` command
/// (cmd 4) to register the current process, matching libnx's
/// `_ldrRoInitialize`.
pub fn connect_ldr_ro_cmif(sm: &SmService) -> Result<RoService, ConnectCmifError> {
    connect_ro_impl(sm, proto::LDR_RO_SERVICE_NAME)
}

/// Connects to the `ro:1` service using CMIF (`[7.0.0+]`).
///
/// Performs the SM lookup and automatically calls the `Initialize` command
/// (cmd 4) to register the current process, matching libnx's
/// `_ro1Initialize`.
pub fn connect_ro1_cmif(sm: &SmService) -> Result<RoService, ConnectCmifError> {
    connect_ro_impl(sm, proto::RO1_SERVICE_NAME)
}

fn connect_ro_impl(
    sm: &SmService,
    service_name: nx_sf::ServiceName,
) -> Result<RoService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectCmifError::GetService)?;

    let service = Session::new(handle, 0);

    if let Err(err) = cmif::initialize(&service) {
        return Err(ConnectCmifError::Initialize(err));
    }

    Ok(RoService(service))
}

/// Connects to the `ro:dmnt` debug/monitor service using CMIF (`[3.0.0+]`).
///
/// No initialization command is needed for `ro:dmnt`.
pub fn connect_ro_dmnt_cmif(sm: &SmService) -> Result<RoDmntService, ConnectDmntCmifError> {
    let handle = sm
        .get_service_handle_cmif(proto::RO_DMNT_SERVICE_NAME)
        .map_err(ConnectDmntCmifError::GetService)?;

    let service = Session::new(handle, 0);

    Ok(RoDmntService(service))
}

/// Errors returned by [`connect_ldr_ro_cmif`] and [`connect_ro1_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup failed.
    #[error("failed to look up RO service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// The `Initialize` command (cmd 4) failed.
    #[error("failed to initialize RO service session")]
    Initialize(#[source] DispatchError),
}

/// Errors returned by [`connect_ro_dmnt_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectDmntCmifError {
    /// SM lookup for `ro:dmnt` failed.
    #[error("failed to look up ro:dmnt service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
