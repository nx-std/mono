//! FilesystemProxy-ProgramRegistry (`fsp-pr`) service implementation.
//!
//! Manages filesystem access controls for programs via a domain CMIF session.
//!
//! ## Divergence from libnx
//!
//! libnx's `fspr.c` keeps a guarded global singleton (`g_fsprSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`, and auto-calls
//! `fsprSetCurrentProcess` during initialization on `[4.0.0+]`.
//! This crate follows the hosversion-unaware convention: connect once
//! via [`connect_cmif`], and call [`FsprService::set_current_process`]
//! explicitly when the caller knows the system is `[4.0.0+]`.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, Session};

mod cmif;
mod proto;

pub use self::proto::SERVICE_NAME;

/// Default filesystem access header granting full permissions.
pub const DEFAULT_FS_ACCESS_HEADER: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, // version = 1
    0xFF, 0xFF, 0xFF, 0xFF, // permissions (all)
    0xFF, 0xFF, 0xFF, 0xFF, // permissions (all)
    0x1C, 0x00, 0x00, 0x00, // content_owner_info_offset
    0x00, 0x00, 0x00, 0x00, // content_owner_info_size
    0x1C, 0x00, 0x00, 0x00, // save_data_owner_info_offset
    0x00, 0x00, 0x00, 0x00, // save_data_owner_info_size
];

/// Default filesystem access control granting full permissions.
pub const DEFAULT_FS_ACCESS_CONTROL: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, // version = 1
    0xFF, 0xFF, 0xFF, 0xFF, // permissions (all)
    0xFF, 0xFF, 0xFF, 0xFF, // permissions (all)
    0x00, 0x00, 0x00, 0x00, // content_owner_id_min_offset
    0x00, 0x00, 0x00, 0x00, // content_owner_id_max_offset
    0xFF, 0xFF, 0xFF, 0xFF, // save_data_owner_id_min
    0xFF, 0xFF, 0xFF, 0xFF, // save_data_owner_id_max
    0x00, 0x00, 0x00, 0x00, // save_data_owner_info_offset
    0x00, 0x00, 0x00, 0x00, // save_data_owner_info_size
    0xFF, 0xFF, 0xFF, 0xFF, // access_flags_min
    0xFF, 0xFF, 0xFF, 0xFF, // access_flags_max
];

/// FilesystemProxy-ProgramRegistry (`fsp-pr`) session wrapper.
pub struct FsprService {
    domain: Domain,
}

impl FsprService {
    /// Registers a program's filesystem access controls.
    ///
    /// If `fs_access_header` is empty, the default full-permission header
    /// ([`DEFAULT_FS_ACCESS_HEADER`]) is used. Likewise for
    /// `fs_access_control` ([`DEFAULT_FS_ACCESS_CONTROL`]).
    #[inline]
    pub fn register_program(
        &self,
        pid: u64,
        tid: u64,
        storage_id: u8,
        fs_access_header: &[u8],
        fs_access_control: &[u8],
        fs_access_control_restriction_mode: u8,
    ) -> Result<(), DispatchError> {
        let fah = if fs_access_header.is_empty() {
            DEFAULT_FS_ACCESS_HEADER
        } else {
            fs_access_header
        };
        let fac = if fs_access_control.is_empty() {
            DEFAULT_FS_ACCESS_CONTROL
        } else {
            fs_access_control
        };
        cmif::register_program(
            &self.domain,
            pid,
            tid,
            storage_id,
            fah,
            fac,
            fs_access_control_restriction_mode,
        )
    }

    /// Unregisters a program.
    #[inline]
    pub fn unregister_program(&self, pid: u64) -> Result<(), DispatchError> {
        cmif::unregister_program(&self.domain, pid)
    }

    /// Sets the current process on the fsp-pr session (`[4.0.0+]`).
    #[inline]
    pub fn set_current_process(&self) -> Result<(), DispatchError> {
        cmif::set_current_process(&self.domain)
    }

    /// Enables or disables program verification (pre-`[10.0.0]`).
    #[inline]
    pub fn set_enabled_program_verification(&self, enabled: bool) -> Result<(), DispatchError> {
        cmif::set_enabled_program_verification(&self.domain, enabled)
    }
}

/// Connects to the `fsp-pr` (FilesystemProxy-ProgramRegistry) service
/// using CMIF, converting the session to a domain.
pub fn connect_cmif(sm: &SmService) -> Result<FsprService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::new(handle);

    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(FsprService { domain })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `fsp-pr` failed.
    #[error("failed to look up fsp-pr service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on fsp-pr session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}
