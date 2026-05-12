//! Service Manager Management (`sm:m`) Protocol Implementation.
//!
//! This crate exposes the SM management interface used by the kernel and
//! Atmosphere to register and unregister processes with the Service Manager.
//!
//! ## Protocol Support
//!
//! Like the SM service itself, `sm:m` supports two protocols:
//! - **CMIF**: Available on HOS < 12.0.0 (non-Atmosphere).
//! - **TIPC**: Available on HOS 12.0.0+ and Atmosphere.
//!
//! Protocol selection is the caller's responsibility. Use the `_cmif` or
//! `_tipc` method variants on [`SmmService`] as appropriate for the system
//! version.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

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
    pub fn session(&self) -> SessionHandle {
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
///
/// # Arguments
///
/// * `sm` - Service manager session
pub fn connect(sm: &SmService) -> Result<SmmService, ConnectError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectError::GetService)?;

    let service = Session::from_handle(handle, 0);

    Ok(SmmService(service))
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get the `sm:m` service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
