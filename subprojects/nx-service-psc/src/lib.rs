//! Power state controller (`psc:m`) service implementation.
//!
//! Provides access to the power state controller for registering PM modules
//! that participate in system power-state transitions (sleep, wake, shutdown).
//!
//! ## Architecture
//!
//! The service operates in domain mode. [`connect_cmif`] obtains the root
//! `IPmControl` session and converts it to a domain, then
//! [`PscService::get_pm_module`] returns a [`PscPmModule`] domain sub-object.
//! The sub-object must be initialized via [`PscPmModule::initialize`] before
//! other operations are called.
//!
//! ## Divergence from libnx
//!
//! libnx's `psc.c` keeps a guarded global singleton (`g_pscmSrv`) managed
//! by `NX_GENERATE_SERVICE_GUARD`, and combines `GetPmModule` + `Initialize`
//! into a single `pscmGetPmModule` call. This crate separates them: first
//! obtain the sub-object via [`PscService::get_pm_module`], then initialize
//! it explicitly.
//!
//! Per IC-4, this crate is hosversion-unaware. The `Acknowledge` command that
//! changed wire format in 5.1.0 is exposed as paired
//! [`acknowledge_legacy`](PscPmModule::acknowledge_legacy) (cmd 2, pre-5.1.0)
//! and [`acknowledge`](PscPmModule::acknowledge) (cmd 4, 5.1.0+) methods.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, DomainObject, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{GetPmModuleError, ModuleInitializeError},
    proto::SERVICE_NAME,
    types::{PmModuleId, PmState},
};

/// Connected PSC service wrapper (`psc:m`).
///
/// The service operates in domain mode; sub-objects ([`PscPmModule`]) share
/// the same kernel session.
pub struct PscService {
    domain: Domain,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PscService {}
unsafe impl Sync for PscService {}

impl PscService {
    /// Gets a PM module sub-object.
    ///
    /// The returned [`PscPmModule`] must be initialized via
    /// [`PscPmModule::initialize`] before use.
    pub fn get_pm_module(&self) -> Result<PscPmModule<'_>, GetPmModuleError> {
        let object = cmif::get_pm_module(&self.domain)?;
        Ok(PscPmModule { object })
    }
}

/// PM module sub-object obtained via [`PscService::get_pm_module`].
///
/// The lifetime parameter ties the module to its parent service so the
/// underlying domain session outlives the sub-object. Dropping the module
/// sends a per-object close request on the domain.
pub struct PscPmModule<'svc> {
    object: DomainObject<'svc>,
}

impl PscPmModule<'_> {
    /// Initializes the PM module with its module ID and dependency list.
    ///
    /// Returns the raw event handle for PM state-change notifications.
    /// The caller is responsible for managing the handle lifetime.
    pub fn initialize(
        &self,
        module_id: PmModuleId,
        dependencies: &[u32],
    ) -> Result<u32, ModuleInitializeError> {
        cmif::module_initialize(&self.object, module_id as u32, dependencies)
    }

    /// Gets the current PM state-change request.
    ///
    /// Returns the requested state and associated flags.
    pub fn get_request(&self) -> Result<(PmState, u32), GetRequestError> {
        let out = cmif::module_get_request(&self.object).map_err(GetRequestError::Dispatch)?;
        let state = PmState::from_raw(out.state).ok_or(GetRequestError::UnknownState(out.state))?;
        Ok((state, out.flags))
    }

    /// Acknowledges a PM state transition (pre-5.1.0 wire format).
    ///
    /// On 5.1.0+ use [`acknowledge`](Self::acknowledge).
    #[inline]
    pub fn acknowledge_legacy(&self) -> Result<(), DispatchError> {
        cmif::module_acknowledge_legacy(&self.object)
    }

    /// Acknowledges a PM state transition (5.1.0+ wire format).
    ///
    /// On pre-5.1.0 use [`acknowledge_legacy`](Self::acknowledge_legacy).
    #[inline]
    pub fn acknowledge(&self, state: PmState) -> Result<(), DispatchError> {
        cmif::module_acknowledge(&self.object, state as u32)
    }

    /// Finalizes the PM module, unregistering it from power state notifications.
    #[inline]
    pub fn finalize(&self) -> Result<(), DispatchError> {
        cmif::module_finalize(&self.object)
    }
}

/// Connects to the `psc:m` service using CMIF.
///
/// Performs the SM lookup, queries the pointer-buffer size, and converts
/// the session to a domain, matching libnx's `_pscmInitialize`.
pub fn connect_cmif(sm: &SmService) -> Result<PscService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::open(handle);

    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(PscService { domain })
}

/// Error returned by [`PscPmModule::get_request`].
#[derive(Debug, thiserror::Error)]
pub enum GetRequestError {
    /// IPC dispatch failed.
    #[error("failed to dispatch IPmModule::GetRequest")]
    Dispatch(#[source] DispatchError),
    /// The returned state value is not a recognized `PmState` variant.
    #[error("IPmModule::GetRequest returned unknown state: {0}")]
    UnknownState(u32),
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `psc:m` failed.
    #[error("failed to look up psc:m service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on psc:m session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}
