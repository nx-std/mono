//! Filesystem-proxy-for-loader (`fsp-ldr`) service implementation.
//!
//! Provides an interface for opening code filesystems, checking archived
//! program status, and setting the current process on the `fsp-ldr` session.
//!
//! ## Divergence from libnx
//!
//! libnx's `fsldr.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`, performs hosversion checks to select the
//! correct `OpenCodeFileSystem` wire format, and automatically calls
//! `SetCurrentProcess` during initialization on firmware 4.0.0+. This crate
//! follows the convention of the other `nx-service-*` crates: connect once
//! via [`connect_cmif`], then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose the
//! appropriate `open_code_filesystem_*` variant and decide whether to call
//! [`FsldrService::set_current_process`] based on the target firmware version.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, Domain, DomainObject, Session};

mod cmif;
mod dispatch;
mod proto;
pub mod types;

pub use nx_sf::service::DispatchError;

pub use self::{
    cmif::OpenCodeFileSystemError,
    proto::SERVICE_NAME,
    types::{FS_MAX_PATH, FsCodeInfo},
};

/// Connected filesystem-proxy-for-loader (`fsp-ldr`) service wrapper.
///
/// The service operates in domain mode; sub-objects returned by
/// `open_code_filesystem_*` methods share the same kernel session.
pub struct FsldrService {
    domain: Domain,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for FsldrService {}
unsafe impl Sync for FsldrService {}

impl FsldrService {
    /// Sets the current process on the service session.
    ///
    /// libnx calls this automatically during initialization on firmware
    /// 4.0.0+. This crate exposes it as an explicit method per IC-4.
    #[inline]
    pub fn set_current_process(&self) -> Result<(), DispatchError> {
        cmif::set_current_process(&self.domain)
    }

    /// Opens a code filesystem (pre-10.0.0).
    ///
    /// Returns a borrowed [`DomainObject`] for the opened filesystem;
    /// dropping it sends a per-object close on the parent domain.
    #[inline]
    pub fn open_code_filesystem_legacy(
        &self,
        tid: u64,
        path: &[u8; FS_MAX_PATH],
    ) -> Result<DomainObject<'_>, OpenCodeFileSystemError> {
        cmif::open_code_filesystem_legacy(&self.domain, tid, path)
    }

    /// Opens a code filesystem (10.0.0–15.x).
    ///
    /// Returns code info and a borrowed [`DomainObject`] for the opened
    /// filesystem.
    #[inline]
    pub fn open_code_filesystem_v10(
        &self,
        tid: u64,
        path: &[u8; FS_MAX_PATH],
        out_code_info: &mut FsCodeInfo,
    ) -> Result<DomainObject<'_>, OpenCodeFileSystemError> {
        cmif::open_code_filesystem_v10(&self.domain, tid, path, out_code_info)
    }

    /// Opens a code filesystem (16.0.0–16.x).
    ///
    /// Adds content attributes. Returns code info via HIPC pointer and a
    /// borrowed [`DomainObject`].
    #[inline]
    pub fn open_code_filesystem_v16(
        &self,
        content_attributes: u8,
        tid: u64,
        path: &[u8; FS_MAX_PATH],
        out_code_info: &mut FsCodeInfo,
    ) -> Result<DomainObject<'_>, OpenCodeFileSystemError> {
        cmif::open_code_filesystem_v16(&self.domain, content_attributes, tid, path, out_code_info)
    }

    /// Opens a code filesystem (17.0.0–19.x).
    ///
    /// Same parameters as v16, but returns code info via HIPC map-alias
    /// instead of pointer.
    #[inline]
    pub fn open_code_filesystem_v17(
        &self,
        content_attributes: u8,
        tid: u64,
        path: &[u8; FS_MAX_PATH],
        out_code_info: &mut FsCodeInfo,
    ) -> Result<DomainObject<'_>, OpenCodeFileSystemError> {
        cmif::open_code_filesystem_v17(&self.domain, content_attributes, tid, path, out_code_info)
    }

    /// Opens a code filesystem (20.0.0+).
    ///
    /// Takes a storage ID instead of a path. Returns code info via HIPC
    /// map-alias and a borrowed [`DomainObject`].
    #[inline]
    pub fn open_code_filesystem_v20(
        &self,
        content_attributes: u8,
        storage_id: u8,
        tid: u64,
        out_code_info: &mut FsCodeInfo,
    ) -> Result<DomainObject<'_>, OpenCodeFileSystemError> {
        cmif::open_code_filesystem_v20(
            &self.domain,
            content_attributes,
            storage_id,
            tid,
            out_code_info,
        )
    }

    /// Checks whether a program (by PID) is archived.
    #[inline]
    pub fn is_archived_program(&self, pid: u64) -> Result<bool, DispatchError> {
        cmif::is_archived_program(&self.domain, pid)
    }
}

/// Connects to the `fsp-ldr` service using CMIF.
///
/// Performs the SM lookup, queries the pointer-buffer size, and converts
/// the session to a domain, matching libnx's `_fsldrInitialize`.
///
/// libnx also calls `SetCurrentProcess` on firmware 4.0.0+; this crate
/// leaves that to the caller via [`FsldrService::set_current_process`]
/// per IC-4.
pub fn connect_cmif(sm: &SmService) -> Result<FsldrService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::open(handle);

    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(FsldrService { domain })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `fsp-ldr` failed.
    #[error("failed to look up fsp-ldr service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on fsp-ldr session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}
