//! AV module (`avm`) service implementation.
//!
//! Provides access to the AVM service for querying and managing application
//! version lists on the Nintendo Switch.
//!
//! ## Divergence from libnx
//!
//! libnx's `avm.c` keeps a guarded global singleton (`g_AvmSrv`) managed
//! by `NX_GENERATE_SERVICE_GUARD`, and enforces a hosversion check during
//! initialization (`hosversionBefore(6,0,0)`). This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose when to use
//! this service based on the target firmware version (6.0.0+).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    ConvertToDomainError,
    Domain,
    DomainObject,
    Session,
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use nx_sf::service::DispatchError;

pub use self::{
    cmif::GetVersionListImporterError,
    proto::SERVICE_NAME,
    types::{
        AvmRequiredVersionEntry,
        AvmVersionListEntry,
    },
};

/// Connected AVM service wrapper.
///
/// The service operates in domain mode; sub-objects (such as
/// [`AvmVersionListImporter`]) share the same kernel session.
pub struct AvmService {
    domain: Domain,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for AvmService {}
unsafe impl Sync for AvmService {}

impl AvmService {
    /// Gets the highest available version for a title pair.
    #[inline]
    pub fn get_highest_available_version(
        &self,
        id_1: u64,
        id_2: u64,
    ) -> Result<u32, DispatchError> {
        cmif::get_highest_available_version(&self.domain, id_1, id_2)
    }

    /// Gets the highest required version for a title pair.
    #[inline]
    pub fn get_highest_required_version(&self, id_1: u64, id_2: u64) -> Result<u32, DispatchError> {
        cmif::get_highest_required_version(&self.domain, id_1, id_2)
    }

    /// Gets a single version list entry by application ID.
    #[inline]
    pub fn get_version_list_entry(
        &self,
        application_id: u64,
    ) -> Result<AvmVersionListEntry, DispatchError> {
        cmif::get_version_list_entry(&self.domain, application_id)
    }

    /// Gets a version list importer sub-object.
    pub fn get_version_list_importer(
        &self,
    ) -> Result<AvmVersionListImporter<'_>, GetVersionListImporterError> {
        let object = cmif::get_version_list_importer(&self.domain)?;
        Ok(AvmVersionListImporter { object })
    }

    /// Gets the launch-required version for an application.
    #[inline]
    pub fn get_launch_required_version(&self, application_id: u64) -> Result<u32, DispatchError> {
        cmif::get_launch_required_version(&self.domain, application_id)
    }

    /// Upgrades the launch-required version for an application.
    #[inline]
    pub fn upgrade_launch_required_version(
        &self,
        application_id: u64,
        version: u32,
    ) -> Result<(), DispatchError> {
        cmif::upgrade_launch_required_version(&self.domain, application_id, version)
    }

    /// Pushes the launch version for an application.
    #[inline]
    pub fn push_launch_version(
        &self,
        application_id: u64,
        version: u32,
    ) -> Result<(), DispatchError> {
        cmif::push_launch_version(&self.domain, application_id, version)
    }

    /// Lists all version list entries into a buffer.
    ///
    /// Returns the number of entries written.
    #[inline]
    pub fn list_version_list(
        &self,
        buffer: &mut [AvmVersionListEntry],
    ) -> Result<u32, DispatchError> {
        cmif::list_version_list(&self.domain, buffer)
    }

    /// Lists all required-version entries into a buffer.
    ///
    /// Returns the number of entries written.
    #[inline]
    pub fn list_required_version(
        &self,
        buffer: &mut [AvmRequiredVersionEntry],
    ) -> Result<u32, DispatchError> {
        cmif::list_required_version(&self.domain, buffer)
    }
}

/// Version list importer sub-object obtained via
/// [`AvmService::get_version_list_importer`].
///
/// The lifetime parameter ties the importer to its parent service so the
/// underlying domain session outlives the sub-object. Dropping the importer
/// sends a per-object close request on the domain.
pub struct AvmVersionListImporter<'svc> {
    object: DomainObject<'svc>,
}

impl AvmVersionListImporter<'_> {
    /// Sets the timestamp on the importer.
    #[inline]
    pub fn set_timestamp(&self, timestamp: u64) -> Result<(), DispatchError> {
        cmif::importer_set_timestamp(&self.object, timestamp)
    }

    /// Sets the version list data on the importer.
    #[inline]
    pub fn set_data(&self, entries: &[AvmVersionListEntry]) -> Result<(), DispatchError> {
        cmif::importer_set_data(&self.object, entries)
    }

    /// Flushes the importer, committing the data.
    #[inline]
    pub fn flush(&self) -> Result<(), DispatchError> {
        cmif::importer_flush(&self.object)
    }
}

/// Connects to the AVM service using CMIF.
///
/// Performs the SM lookup, queries the pointer-buffer size, and converts
/// the session to a domain, matching libnx's `_avmInitialize`.
pub fn connect_cmif(sm: &SmService) -> Result<AvmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::open(handle);

    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(AvmService { domain })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `avm` failed.
    #[error("failed to look up avm service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on avm session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}
