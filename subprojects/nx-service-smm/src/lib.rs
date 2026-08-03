//! Service Manager management (`sm:m`) protocol implementation.
//!
//! `sm:m` is the privileged side of the Service Manager. Where `sm:` lets any
//! process *look up* services, `sm:m` is how the system tells SM which
//! processes exist and what each one is allowed to do.
//!
//! It exists to keep SM's view of the system in step with the set of running
//! processes. As a process is launched it is registered with SM — together
//! with the Service Access Control (SAC) data describing which services it may
//! use as a client and host as a server — and when it exits it is
//! unregistered. SM leans on this for every later lookup on `sm:`, so a
//! process can only reach the services its SAC grants.
//!
//! Reach for this crate only when writing a process-management component;
//! ordinary programs need just `sm:` (the `nx-service-sm` crate).
//!
//! ## Usage
//!
//! `sm:m` is itself a registered service, so [`connect`] obtains it through an
//! existing [`SmService`] session. The resulting [`SmmService`] then offers
//! two operations — register a process and unregister a process — each in a
//! `_cmif` and a `_tipc` variant.
//!
//! ## Protocol Support
//!
//! `sm:m` speaks the same two IPC protocols as the rest of the system, CMIF
//! and the newer TIPC, but accepts CMIF on a narrower range than the `sm:`
//! port does: only stock Horizon OS older than `[12.0.0]`. Newer firmware and
//! Atmosphère require TIPC. Choosing the `_cmif` or `_tipc` method variant
//! that matches the running system is the caller's responsibility.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod tipc;

pub use self::{
    cmif::{
        RegisterProcessError as RegisterProcessCmifError,
        UnregisterProcessError as UnregisterProcessCmifError,
    },
    proto::SERVICE_NAME,
    tipc::{
        RegisterProcessError as RegisterProcessTipcError,
        UnregisterProcessError as UnregisterProcessTipcError,
    },
};

/// SM management (`sm:m`) service session wrapper.
///
/// Provides type safety to distinguish `sm:m` sessions from regular services.
#[repr(transparent)]
pub struct SmmService(Session);

impl SmmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl SmmService {
    /// Registers a process with the Service Manager using CMIF protocol.
    #[inline]
    pub fn register_process_cmif(
        &self,
        pid: u64,
        acid_sac: &[u8],
        aci0_sac: &[u8],
    ) -> Result<(), RegisterProcessCmifError> {
        cmif::register_process(self.0.handle(), pid, acid_sac, aci0_sac)
    }

    /// Unregisters a process from the Service Manager using CMIF protocol.
    #[inline]
    pub fn unregister_process_cmif(&self, pid: u64) -> Result<(), UnregisterProcessCmifError> {
        cmif::unregister_process(self.0.handle(), pid)
    }
}

/// TIPC protocol methods.
///
/// Requires HOS 12.0.0+ or Atmosphere.
impl SmmService {
    /// Registers a process with the Service Manager using TIPC protocol.
    ///
    /// Requires HOS 12.0.0+ or Atmosphere.
    #[inline]
    pub fn register_process_tipc(
        &self,
        pid: u64,
        acid_sac: &[u8],
        aci0_sac: &[u8],
    ) -> Result<(), RegisterProcessTipcError> {
        tipc::register_process(self.0.handle(), pid, acid_sac, aci0_sac)
    }

    /// Unregisters a process from the Service Manager using TIPC protocol.
    ///
    /// Requires HOS 12.0.0+ or Atmosphere.
    #[inline]
    pub fn unregister_process_tipc(&self, pid: u64) -> Result<(), UnregisterProcessTipcError> {
        tipc::unregister_process(self.0.handle(), pid)
    }
}

/// Connects to the SM management (`sm:m`) service.
///
/// The returned [`SmmService`] can be used with either CMIF or TIPC dispatch;
/// `sm:m` is a single named port, and the protocol choice is per-request.
pub fn connect(sm: &SmService) -> Result<SmmService, ConnectError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectError::GetService)?;

    let service = Session::new(handle, 0);

    Ok(SmmService(service))
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get the `sm:m` service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
