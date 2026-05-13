//! Capture MTP (`capmtp`) service implementation.
//!
//! Provides access to the capture MTP service for managing MTP (Media Transfer
//! Protocol) sessions on the Nintendo Switch.
//!
//! ## Divergence from libnx
//!
//! libnx's `capmtp.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD`, and bundles TransferMemory creation, session
//! setup, and event acquisition into a single `_capmtpInitialize` call with
//! a hosversion check (`hosversionBefore(11,0,0)`). This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], open a session, then call methods directly.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose when to use
//! this service based on the target firmware version (11.0.0+).

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{ConvertToDomainError, DispatchError, Domain, DomainObject, Session};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use nx_sf::service::DispatchError as CapmtpDispatchError;

pub use self::{
    cmif::{OpenSessionError, SessionOpenError},
    proto::SERVICE_NAME,
};

/// Connected capture MTP root service wrapper.
///
/// The service operates in domain mode; the session sub-object
/// ([`CapmtpSession`]) shares the same kernel session.
pub struct CapmtpService {
    domain: Domain,
}

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for CapmtpService {}
unsafe impl Sync for CapmtpService {}

impl CapmtpService {
    /// Opens a session sub-object.
    pub fn open_session(&self) -> Result<CapmtpSession<'_>, OpenSessionError> {
        let raw_object_id = cmif::open_session(&self.domain)?;
        let object = self
            .domain
            .open_object_raw(raw_object_id)
            .ok_or(OpenSessionError::MissingObject)?;
        Ok(CapmtpSession { object })
    }
}

/// MTP session sub-object obtained via [`CapmtpService::open_session`].
///
/// The lifetime parameter ties the session to its parent service so the
/// underlying domain session outlives the sub-object. Dropping the session
/// sends a per-object close request on the domain.
pub struct CapmtpSession<'svc> {
    object: DomainObject<'svc>,
}

impl CapmtpSession<'_> {
    /// Opens the MTP session with transfer memory, folder/image/video limits,
    /// and a UTF-16 device name.
    ///
    /// `tmem_handle` is the raw handle of a `TransferMemory` object. The caller
    /// is responsible for creating the transfer memory and keeping it alive for
    /// the duration of the session.
    #[inline]
    pub fn open(
        &self,
        tmem_handle: u32,
        tmem_size: u32,
        folder_count: u32,
        max_images: u32,
        max_videos: u32,
        name_utf16: &[u16],
    ) -> Result<(), SessionOpenError> {
        cmif::session_open(
            &self.object,
            tmem_handle,
            tmem_size,
            folder_count,
            max_images,
            max_videos,
            name_utf16,
        )
    }

    /// Closes the MTP session (command-level close, not sub-object close).
    #[inline]
    pub fn close_session(&self) -> Result<(), DispatchError> {
        cmif::session_close(&self.object)
    }

    /// Starts the MTP command handler.
    #[inline]
    pub fn start_command_handler(&self) -> Result<(), DispatchError> {
        cmif::session_start_command_handler(&self.object)
    }

    /// Stops the MTP command handler.
    #[inline]
    pub fn stop_command_handler(&self) -> Result<(), DispatchError> {
        cmif::session_stop_command_handler(&self.object)
    }

    /// Checks whether the command handler is running.
    #[inline]
    pub fn is_running(&self) -> Result<bool, DispatchError> {
        cmif::session_is_running(&self.object)
    }

    /// Gets the connection event handle (copy handle).
    ///
    /// The returned handle can be used with event-waiting primitives.
    #[inline]
    pub fn get_connection_event(&self) -> Result<u32, DispatchError> {
        cmif::session_get_connection_event(&self.object)
    }

    /// Checks whether a USB device is connected.
    #[inline]
    pub fn is_connected(&self) -> Result<bool, DispatchError> {
        cmif::session_is_connected(&self.object)
    }

    /// Gets the scan-error event handle (copy handle).
    ///
    /// The returned handle can be used with event-waiting primitives.
    #[inline]
    pub fn get_scan_error_event(&self) -> Result<u32, DispatchError> {
        cmif::session_get_scan_error_event(&self.object)
    }

    /// Gets the scan-error result code.
    ///
    /// Returns `Ok(())` if no scan error, or a dispatch error on failure.
    #[inline]
    pub fn get_scan_error(&self) -> Result<(), DispatchError> {
        cmif::session_get_scan_error(&self.object)
    }
}

/// Connects to the capture MTP service using CMIF.
///
/// Performs the SM lookup, queries the pointer-buffer size, and converts
/// the session to a domain, matching libnx's `_capmtpInitialize` setup.
pub fn connect_cmif(sm: &SmService) -> Result<CapmtpService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError::GetService)?;

    let session = Session::new(handle);

    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectCmifError::ConvertToDomain(err))?;

    Ok(CapmtpService { domain })
}

/// Errors returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectCmifError {
    /// SM lookup for `capmtp` failed.
    #[error("failed to look up capmtp service via sm")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Converting the session to a domain failed.
    #[error("failed to ConvertToDomain on capmtp session")]
    ConvertToDomain(#[source] ConvertToDomainError),
}
