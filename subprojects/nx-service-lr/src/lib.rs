//! Location resolver (`lr`) service implementation.
//!
//! Provides access to the location resolver manager for looking up and
//! redirecting content paths on the system (program paths, control paths,
//! HTML document paths, data paths, and legal information paths).
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root
//! `ILocationResolverManager` session, then
//! [`LrService::open_location_resolver`] and
//! [`LrService::open_registered_location_resolver`] return sub-objects with
//! their own independent session handles.
//!
//! ## Divergence from libnx
//!
//! libnx's `lr.c` keeps a guarded global singleton (`g_lrSrv`) managed by
//! `NX_GENERATE_SERVICE_GUARD`, and enforces hosversion checks at each call
//! site. This crate follows the convention of the other `nx-service-*` crates:
//! connect once via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware. The redirect-application
//! commands that changed wire format in 9.0.0 (adding a second title ID) are
//! exposed as paired `_legacy` / non-legacy methods so the caller can select
//! per firmware version. `erase_program_redirection` (cmd 12, 5.0.0+) is
//! exposed unconditionally.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::{
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, DispatchError, OwnedSessionHandle, Session},
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::{OpenLocationResolverError, OpenRegisteredLocationResolverError},
    proto::SERVICE_NAME,
    types::{LR_MAX_PATH, StorageId},
};

/// Location resolver manager (`lr`) root session wrapper.
///
/// Use [`open_location_resolver`](Self::open_location_resolver) and
/// [`open_registered_location_resolver`](Self::open_registered_location_resolver)
/// to create resolver sub-objects.
#[repr(transparent)]
pub struct LrService(Session);

impl LrService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Opens a location resolver for the given storage.
    pub fn open_location_resolver(
        &self,
        storage: StorageId,
    ) -> Result<LrLocationResolver, OpenLocationResolverError> {
        let raw_handle = cmif::open_location_resolver(&self.0, storage as u8)?;

        // SAFETY: the kernel returned a valid move handle for the new resolver
        // session; ownership transfers to the new `Session`.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(LrLocationResolver(Session::new(handle, 0)))
    }

    /// Opens the registered location resolver.
    pub fn open_registered_location_resolver(
        &self,
    ) -> Result<LrRegisteredLocationResolver, OpenRegisteredLocationResolverError> {
        let raw_handle = cmif::open_registered_location_resolver(&self.0)?;

        // SAFETY: the kernel returned a valid move handle for the new resolver
        // session; ownership transfers to the new `Session`.
        let handle = OwnedSessionHandle::from_handle_unchecked(
            RawSessionHandle::from_raw_unchecked(raw_handle),
        );
        Ok(LrRegisteredLocationResolver(Session::new(handle, 0)))
    }
}

/// Location resolver sub-object (`ILocationResolver`).
///
/// Obtained via [`LrService::open_location_resolver`]. Owns its own
/// independent session handle.
#[repr(transparent)]
pub struct LrLocationResolver(Session);

impl LrLocationResolver {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `LrLocationResolver`.
impl LrLocationResolver {
    /// Resolves the program path for the given title ID.
    #[inline]
    pub fn resolve_program_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::resolve_program_path(&self.0, tid, out)
    }

    /// Redirects the program path for the given title ID.
    #[inline]
    pub fn redirect_program_path(
        &self,
        tid: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_program_path(&self.0, tid, path)
    }

    /// Resolves the application control path for the given title ID.
    #[inline]
    pub fn resolve_application_control_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::resolve_application_control_path(&self.0, tid, out)
    }

    /// Resolves the application HTML document path for the given title ID.
    #[inline]
    pub fn resolve_application_html_document_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::resolve_application_html_document_path(&self.0, tid, out)
    }

    /// Resolves the data path for the given title ID.
    #[inline]
    pub fn resolve_data_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::resolve_data_path(&self.0, tid, out)
    }

    /// Redirects the application control path (pre-9.0.0 wire format).
    ///
    /// Uses a single title ID. On 9.0.0+ use
    /// [`redirect_application_control_path`](Self::redirect_application_control_path).
    #[inline]
    pub fn redirect_application_control_path_legacy(
        &self,
        tid: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_control_path_legacy(&self.0, tid, path)
    }

    /// Redirects the application control path (9.0.0+ wire format).
    ///
    /// Takes two title IDs. On pre-9.0.0 use
    /// [`redirect_application_control_path_legacy`](Self::redirect_application_control_path_legacy).
    #[inline]
    pub fn redirect_application_control_path(
        &self,
        tid: u64,
        tid2: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_control_path(&self.0, tid, tid2, path)
    }

    /// Redirects the application HTML document path (pre-9.0.0 wire format).
    ///
    /// Uses a single title ID. On 9.0.0+ use
    /// [`redirect_application_html_document_path`](Self::redirect_application_html_document_path).
    #[inline]
    pub fn redirect_application_html_document_path_legacy(
        &self,
        tid: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_html_document_path_legacy(&self.0, tid, path)
    }

    /// Redirects the application HTML document path (9.0.0+ wire format).
    ///
    /// Takes two title IDs. On pre-9.0.0 use
    /// [`redirect_application_html_document_path_legacy`](Self::redirect_application_html_document_path_legacy).
    #[inline]
    pub fn redirect_application_html_document_path(
        &self,
        tid: u64,
        tid2: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_html_document_path(&self.0, tid, tid2, path)
    }

    /// Resolves the application legal information path for the given title ID.
    #[inline]
    pub fn resolve_application_legal_information_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::resolve_application_legal_information_path(&self.0, tid, out)
    }

    /// Redirects the application legal information path (pre-9.0.0 wire format).
    ///
    /// Uses a single title ID. On 9.0.0+ use
    /// [`redirect_application_legal_information_path`](Self::redirect_application_legal_information_path).
    #[inline]
    pub fn redirect_application_legal_information_path_legacy(
        &self,
        tid: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_legal_information_path_legacy(&self.0, tid, path)
    }

    /// Redirects the application legal information path (9.0.0+ wire format).
    ///
    /// Takes two title IDs. On pre-9.0.0 use
    /// [`redirect_application_legal_information_path_legacy`](Self::redirect_application_legal_information_path_legacy).
    #[inline]
    pub fn redirect_application_legal_information_path(
        &self,
        tid: u64,
        tid2: u64,
        path: &[u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::redirect_application_legal_information_path(&self.0, tid, tid2, path)
    }

    /// Refreshes the location resolver, re-scanning content locations.
    #[inline]
    pub fn refresh(&self) -> Result<(), DispatchError> {
        cmif::refresh(&self.0)
    }

    /// Erases a program path redirection. \[5.0.0+\]
    #[inline]
    pub fn erase_program_redirection(&self, tid: u64) -> Result<(), DispatchError> {
        cmif::erase_program_redirection(&self.0, tid)
    }
}

/// Registered location resolver sub-object (`IRegisteredLocationResolver`).
///
/// Obtained via [`LrService::open_registered_location_resolver`]. Owns its
/// own independent session handle.
#[repr(transparent)]
pub struct LrRegisteredLocationResolver(Session);

impl LrRegisteredLocationResolver {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `LrRegisteredLocationResolver`.
impl LrRegisteredLocationResolver {
    /// Resolves the registered program path for the given title ID.
    #[inline]
    pub fn resolve_program_path(
        &self,
        tid: u64,
        out: &mut [u8; LR_MAX_PATH],
    ) -> Result<(), DispatchError> {
        cmif::reg_resolve_program_path(&self.0, tid, out)
    }
}

/// Connects to the `lr` service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<LrService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(LrService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get lr service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
