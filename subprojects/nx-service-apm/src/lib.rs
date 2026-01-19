//! Application Performance Management (APM) Service Implementation.
//!
//! This crate provides access to the Nintendo Switch's APM service, which
//! manages performance modes and CPU/GPU clock configurations.
//!
//! ## Architecture
//!
//! The APM service consists of two interfaces:
//! - **IManager** (`apm`): Main service interface
//! - **ISession**: Per-session interface for configuration

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;

pub use self::{
    cmif::{
        GetPerformanceConfigurationError, GetPerformanceModeError, OpenSessionError,
        SetPerformanceConfigurationError,
    },
    proto::{PerformanceMode, SERVICE_NAME},
};

/// APM Manager service (IManager) session wrapper.
///
/// Provides type safety to distinguish APM sessions from other services.
#[repr(transparent)]
pub struct ApmService(Service);

impl ApmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the APM service session.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Opens an ISession for performance configuration.
    ///
    /// Returns a new [`ApmSession`] that can be used to get/set performance
    /// configurations.
    #[inline]
    pub fn open_session(&self) -> Result<ApmSession, OpenSessionError> {
        let session_handle = cmif::open_session(self.0.session)?;
        let service = Service {
            session: session_handle,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        Ok(ApmSession(service))
    }

    /// Gets the current performance mode.
    ///
    /// Returns either [`PerformanceMode::Normal`] or [`PerformanceMode::Boost`].
    #[inline]
    pub fn get_performance_mode(&self) -> Result<PerformanceMode, GetPerformanceModeError> {
        cmif::get_performance_mode(self.0.session)
    }
}

/// APM Session (ISession) wrapper.
///
/// Provides per-session performance configuration management.
#[repr(transparent)]
pub struct ApmSession(Service);

impl ApmSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the APM session.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Sets the performance configuration for a given mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The performance mode to configure
    /// * `config` - The performance configuration value (platform-specific)
    ///
    /// # Example Configurations
    ///
    /// - `0x00020003`: Low power
    /// - `0x00020004`: Medium power
    /// - `0x92220007`: High performance
    /// - `0x92220008`: Maximum performance (docked)
    #[inline]
    pub fn set_performance_configuration(
        &self,
        mode: PerformanceMode,
        config: u32,
    ) -> Result<(), SetPerformanceConfigurationError> {
        cmif::set_performance_configuration(self.0.session, mode, config)
    }

    /// Gets the performance configuration for a given mode.
    ///
    /// Returns the current configuration value for the specified mode.
    #[inline]
    pub fn get_performance_configuration(
        &self,
        mode: PerformanceMode,
    ) -> Result<u32, GetPerformanceConfigurationError> {
        cmif::get_performance_configuration(self.0.session, mode)
    }
}

/// Connects to the APM service.
///
/// # Arguments
///
/// * `sm` - Service manager session
///
/// # Returns
///
/// A connected [`ApmService`] instance on success.
pub fn connect(sm: &SmService) -> Result<ApmService, ConnectError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectError::GetService)?;

    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    Ok(ApmService(service))
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service handle from SM.
    #[error("failed to get service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
}
