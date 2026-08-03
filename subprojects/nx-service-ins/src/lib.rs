//! Installation notification (`ins`) service implementation.
//!
//! Provides two interfaces for monitoring installation events:
//!
//! - **`ins:r`** (request/read): Retrieve event signal ticks and readable
//!   event handles (IDs 0–4).
//! - **`ins:s`** (send/write): Retrieve writable event handles for
//!   signaling installation events (IDs 0–11).
//!
//! ## Divergence from libnx
//!
//! libnx's `ins.c` keeps guarded global singletons (`g_insrSrv`,
//! `g_inssSrv`) managed by `NX_GENERATE_SERVICE_GUARD`. This crate
//! follows the convention of the other `nx-service-*` crates: connect
//! once via [`connect_insr_cmif`] or [`connect_inss_cmif`], reuse the
//! service wrapper across calls, and close the session explicitly with
//! `Drop` or `Drop`.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;

pub use self::{
    cmif::{GetLastTickError, GetReadableEventError, GetWritableEventError},
    proto::{INSR_SERVICE_NAME, INSS_SERVICE_NAME},
};

/// INS request/read (`ins:r`) session wrapper.
///
/// Provides access to installation event ticks and readable event handles.
#[repr(transparent)]
pub struct InsrService(Session);

impl InsrService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `ins:r`.
impl InsrService {
    /// Gets the last system tick at which the event for `id` was signaled.
    ///
    /// Valid IDs are 0–4. The tick is only updated at minimum once per second.
    #[inline]
    pub fn get_last_tick(&self, id: u32) -> Result<u64, GetLastTickError> {
        cmif::get_last_tick(self.0.handle(), id)
    }

    /// Gets a readable event handle for the given request ID.
    ///
    /// Valid IDs are 0–4. The event is only signaled at minimum once per
    /// second.
    #[inline]
    pub fn get_readable_event(
        &self,
        id: u32,
    ) -> Result<nx_svc::sync::EventHandle, GetReadableEventError> {
        cmif::get_readable_event(self.0.handle(), id)
    }
}

/// INS send/write (`ins:s`) session wrapper.
///
/// Provides access to writable event handles for signaling installation events.
#[repr(transparent)]
pub struct InssService(Session);

impl InssService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods for `ins:s`.
impl InssService {
    /// Gets a writable event handle for the given send ID.
    ///
    /// Valid IDs are 0–11. The returned handle can only be signaled, not
    /// waited on. Clearing is managed by the service.
    ///
    /// Returns the raw handle value. Use `nx_svc::raw::signal_event` to
    /// signal the event.
    #[inline]
    pub fn get_writable_event(&self, id: u32) -> Result<u32, GetWritableEventError> {
        cmif::get_writable_event(self.0.handle(), id)
    }
}

/// Connects to the `ins:r` (Installation Request) service using CMIF.
pub fn connect_insr_cmif(sm: &SmService) -> Result<InsrService, ConnectInsrCmifError> {
    let handle = sm
        .get_service_handle_cmif(INSR_SERVICE_NAME)
        .map_err(ConnectInsrCmifError)?;

    let service = Session::new(handle, 0);

    Ok(InsrService(service))
}

/// Error returned by [`connect_insr_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get ins:r service")]
pub struct ConnectInsrCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// Connects to the `ins:s` (Installation Send) service using CMIF.
pub fn connect_inss_cmif(sm: &SmService) -> Result<InssService, ConnectInssCmifError> {
    let handle = sm
        .get_service_handle_cmif(INSS_SERVICE_NAME)
        .map_err(ConnectInssCmifError)?;

    let service = Session::new(handle, 0);

    Ok(InssService(service))
}

/// Error returned by [`connect_inss_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get ins:s service")]
pub struct ConnectInssCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
