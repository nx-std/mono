//! Idle System Service (`idle:sys`) implementation.
//!
//! Exposes the idle-system sleep-counter reset command as a typed Rust
//! function. CMIF only — non-domain.
//!
//! ## Divergence from libnx
//!
//! libnx's `idlesys.c` keeps a guarded global singleton (`g_idlesysSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`IdlesysService`] across calls, and close
//! the session explicitly with `Drop`.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;

pub use self::{cmif::ReportUserIsActiveError, proto::SERVICE_NAME};

/// Idle System (`idle:sys`) session wrapper.
///
/// Provides type safety to distinguish `idle:sys` sessions from other services.
#[repr(transparent)]
pub struct IdlesysService(Session);

impl IdlesysService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl IdlesysService {
    /// Reports that the user is active, resetting the sleep counter.
    #[inline]
    pub fn report_user_is_active(&self) -> Result<(), ReportUserIsActiveError> {
        cmif::report_user_is_active(self.0.handle())
    }
}

/// Connects to the `idle:sys` (Idle System) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<IdlesysService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(IdlesysService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get idle:sys service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
