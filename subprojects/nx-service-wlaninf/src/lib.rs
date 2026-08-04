//! WLAN InfraManager Service (`wlan:inf`) implementation.
//!
//! Exposes the WLAN connection-state and RSSI IPC commands as typed Rust
//! functions. CMIF only — non-domain.
//!
//! ## Compatibility
//!
//! `wlan:inf` is available on HOS 1.0.0 – 14.1.2 and is **removed in
//! HOS 15.0.0+**. Following the convention of `nx-service-vi`, this crate
//! is intentionally unaware of `hosversion`; the caller is responsible for
//! gating [`connect_cmif`] on HOS < 15.0.0 and reporting an
//! `IncompatSysVer`-equivalent error otherwise.
//!
//! ## Divergence from libnx
//!
//! libnx's `wlaninf.c` keeps a guarded global singleton (`g_wlaninfSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`WlaninfService`] across calls, and close
//! the session explicitly with `Drop`.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;

mod cmif;
mod proto;

pub use self::{
    cmif::{
        DispatchError,
        GetRssiError,
        GetStateError,
    },
    proto::{
        CMD_GET_RSSI,
        CMD_GET_STATE,
        Rssi,
        SERVICE_NAME,
        WlanInfState,
    },
};

/// Connects to the `wlan:inf` (WLAN InfraManager) service using CMIF.
///
/// The caller must ensure `hosversion < 15.0.0` before calling; see the
/// crate-level docs for the compatibility window.
pub fn connect_cmif(sm: &SmService) -> Result<WlaninfService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(WlaninfService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get wlan:inf service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);

/// WLAN InfraManager (`wlan:inf`) session wrapper.
///
/// Provides type safety to distinguish `wlan:inf` sessions from other services.
#[repr(transparent)]
pub struct WlaninfService(Session);
// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for WlaninfService {}

unsafe impl Sync for WlaninfService {}

/// CMIF protocol methods.
impl WlaninfService {
    /// Reads the current WLAN connection state (`GetState`, cmd 10).
    #[inline]
    pub fn get_state(&self) -> Result<WlanInfState, GetStateError> {
        cmif::get_state(self.0.handle())
    }

    /// Reads the current received signal strength (`GetRSSI`, cmd 12).
    #[inline]
    pub fn get_rssi(&self) -> Result<Rssi, GetRssiError> {
        cmif::get_rssi(self.0.handle())
    }
}

#[cfg(feature = "ffi")]
impl WlaninfService {
    /// Returns the underlying session for libnx `Service*` shadow buffers.
    #[inline]
    pub fn session(&self) -> &Session {
        &self.0
    }
}
