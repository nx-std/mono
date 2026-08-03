//! Operation Mode Manager (`omm`) service implementation.
//!
//! Provides operation mode queries and policy configuration via the
//! `omm` IPC service.
//!
//! ## Hosversion variants
//!
//! Commands 10 (`SetOperationModePolicy`) and 11
//! (`GetDefaultDisplayResolution`) are only available on HOS 3.0.0+.
//! This crate exposes all commands unconditionally and leaves version
//! selection to the caller.
//!
//! ## Divergence from libnx
//!
//! libnx's `omm.c` keeps a guarded global singleton (`g_ommSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD` and checks `hosversionBefore`
//! at runtime. This crate follows the convention of the other
//! `nx-service-*` crates: connect once via [`connect_cmif`], reuse the
//! [`OmmService`] across calls, and close the session explicitly with
//! `Drop`. Hosversion gating is the caller's
//! responsibility.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{GetDefaultDisplayResolutionError, GetOperationModeError, SetOperationModePolicyError},
    proto::SERVICE_NAME,
    types::{OperationMode, OperationModePolicy},
};

/// Operation Mode Manager service (`omm`) session wrapper.
#[repr(transparent)]
pub struct OmmService(Session);

impl OmmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl OmmService {
    /// Gets the current operation mode.
    #[inline]
    pub fn get_operation_mode(&self) -> Result<OperationMode, GetOperationModeError> {
        let raw = cmif::get_operation_mode(self.0.handle())?;
        Ok(OperationMode::from_raw(raw).unwrap_or(OperationMode::Handheld))
    }

    /// Sets the operation mode policy (3.0.0+).
    #[inline]
    pub fn set_operation_mode_policy(
        &self,
        policy: OperationModePolicy,
    ) -> Result<(), SetOperationModePolicyError> {
        cmif::set_operation_mode_policy(self.0.handle(), policy.as_raw())
    }

    /// Gets the default display resolution (3.0.0+).
    ///
    /// Returns `(width, height)`.
    #[inline]
    pub fn get_default_display_resolution(
        &self,
    ) -> Result<(i32, i32), GetDefaultDisplayResolutionError> {
        cmif::get_default_display_resolution(self.0.handle())
    }
}

/// Connects to the `omm` (Operation Mode Manager) service using CMIF.
///
/// The caller must close the returned [`OmmService`] when done.
pub fn connect_cmif(sm: &SmService) -> Result<OmmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(OmmService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get omm service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
