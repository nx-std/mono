//! Error context (`ectx:r`) service implementation.
//!
//! Provides access to error context data associated with error
//! descriptors and result codes via the `ectx:r` service interface.
//!
//! ## Divergence from libnx
//!
//! libnx's `ectx.c` keeps a guarded global singleton (`g_ectxrSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`EctxService`] across calls, and close
//! the session explicitly with `Drop`.
//!
//! libnx gates initialization behind a hosversion check
//! (`hosversionBefore(11,0,0)`). Per IC-4 this crate is
//! hosversion-unaware: the caller selects based on system version.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;

pub use self::{
    cmif::{PullContextError, PullContextOutput},
    proto::SERVICE_NAME,
};

/// Error context reader (`ectx:r`) session wrapper.
#[repr(transparent)]
pub struct EctxService(Session);

impl EctxService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl EctxService {
    /// Pulls error context associated with a descriptor and result code.
    ///
    /// Writes the error context into `dst` and returns metadata about
    /// the context size. Available on \[11.0.0+\]. The caller must
    /// check the system version before calling this method.
    #[inline]
    pub fn pull_context(
        &self,
        dst: &mut [u8],
        descriptor: u32,
        result: u32,
    ) -> Result<PullContextOutput, PullContextError> {
        cmif::pull_context(self.0.handle(), dst, descriptor, result)
    }
}

/// Connects to the `ectx:r` (Error Context Reader) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<EctxService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(EctxService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get ectx:r service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
