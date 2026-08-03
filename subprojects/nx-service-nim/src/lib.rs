//! Network Install Manager (`nim`) service implementation.
//!
//! Provides system update task management via a single non-domain
//! CMIF session.
//!
//! ## Divergence from libnx
//!
//! libnx's `nim.c` keeps a guarded global singleton (`g_nimSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`NimService`] across calls, and close
//! the session explicitly with `Drop`.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
pub mod types;

pub use self::{
    cmif::{DestroySystemUpdateTaskError, ListSystemUpdateTaskError},
    proto::SERVICE_NAME,
    types::SystemUpdateTaskId,
};

/// Network Install Manager (`nim`) session wrapper.
#[repr(transparent)]
pub struct NimService(Session);

impl NimService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl NimService {
    /// Destroys a system update task identified by `task_id`.
    #[inline]
    pub fn destroy_system_update_task(
        &self,
        task_id: &SystemUpdateTaskId,
    ) -> Result<(), DestroySystemUpdateTaskError> {
        cmif::destroy_system_update_task(self.0.handle(), task_id)
    }

    /// Lists system update tasks into the provided buffer.
    ///
    /// Returns the total number of tasks reported by the service.
    /// Up to `out.len()` task IDs are written into `out`.
    #[inline]
    pub fn list_system_update_task(
        &self,
        out: &mut [SystemUpdateTaskId],
    ) -> Result<i32, ListSystemUpdateTaskError> {
        cmif::list_system_update_task(self.0.handle(), out)
    }
}

/// Connects to the `nim` (Network Install Manager) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<NimService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(NimService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get nim service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
