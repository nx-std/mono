//! System Power State Manager (`spsm`) service implementation.
//!
//! Provides system shutdown/reboot and error-state commands via a
//! single non-domain CMIF session.
//!
//! ## Divergence from libnx
//!
//! libnx's `spsm.c` keeps a guarded global singleton (`g_spsmSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`SpsmService`] across calls, and close
//! the session explicitly with `Drop`.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;

pub use self::{
    cmif::{PutErrorStateError, ShutdownError},
    proto::SERVICE_NAME,
};

/// System Power State Manager (`spsm`) session wrapper.
#[repr(transparent)]
pub struct SpsmService(Session);

impl SpsmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SpsmService {
    /// Initiates system shutdown or reboot.
    ///
    /// When `reboot` is `true` the system reboots; otherwise it powers off.
    #[inline]
    pub fn shutdown(&self, reboot: bool) -> Result<(), ShutdownError> {
        cmif::shutdown(self.0.handle(), reboot)
    }

    /// Puts the system into an error state.
    #[inline]
    pub fn put_error_state(&self) -> Result<(), PutErrorStateError> {
        cmif::put_error_state(self.0.handle())
    }
}

/// Connects to the `spsm` (System Power State Manager) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<SpsmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(SpsmService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get spsm service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
