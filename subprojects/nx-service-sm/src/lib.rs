//! Service Manager Protocol Implementation.
//!
//! This crate provides stateless SM protocol operations. All operations are
//! methods on [`SmService`], which wraps an SM session handle.
//!
//! ## Protocol Support
//!
//! SM supports two protocols:
//! - **CMIF**: Available on all HOS versions
//! - **TIPC**: Available on HOS 12.0.0+ and Atmosphere
//!
//! Protocol selection is the caller's responsibility. Use the `_cmif` or `_tipc`
//! method variants as appropriate for your system version.

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

pub use nx_sf::ServiceName;
use nx_sf::service::Service;
use nx_svc::ipc::{self, Handle as SessionHandle};

mod cmif;
mod proto;
mod tipc;

pub use self::{
    cmif::{
        DetachClientError as DetachClientCmifError, GetServiceError as GetServiceCmifError,
        RegisterClientError as RegisterClientCmifError,
        RegisterServiceError as RegisterServiceCmifError,
        UnregisterServiceError as UnregisterServiceCmifError,
    },
    proto::SM_PORT_NAME,
    tipc::{
        DetachClientError as DetachClientTipcError, GetServiceError as GetServiceTipcError,
        RegisterClientError as RegisterClientTipcError,
        RegisterServiceError as RegisterServiceTipcError,
        UnregisterServiceError as UnregisterServiceTipcError,
    },
};

/// Sleep duration (in nanoseconds) when SM is not available during connection.
const CONNECT_RETRY_SLEEP_NS: u64 = 50_000_000; // 50ms

/// Service Manager session wrapper.
///
/// Provides type safety to distinguish SM sessions from regular services.
#[repr(transparent)]
pub struct SmService(Service);

impl SmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the SM session.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }
}

/// CMIF protocol methods.
impl SmService {
    /// Gets a service handle by name using CMIF protocol.
    #[inline]
    pub fn get_service_handle_cmif(
        &self,
        name: ServiceName,
    ) -> Result<SessionHandle, GetServiceCmifError> {
        cmif::get_service_handle(self.0.session, name)
    }

    /// Registers a service using CMIF protocol.
    #[inline]
    pub fn register_service_cmif(
        &self,
        name: ServiceName,
        is_light: bool,
        max_sessions: i32,
    ) -> Result<SessionHandle, RegisterServiceCmifError> {
        cmif::register_service(self.0.session, name, is_light, max_sessions)
    }

    /// Unregisters a service using CMIF protocol.
    #[inline]
    pub fn unregister_service_cmif(
        &self,
        name: ServiceName,
    ) -> Result<(), UnregisterServiceCmifError> {
        cmif::unregister_service(self.0.session, name)
    }

    /// Detaches the client using CMIF protocol.
    ///
    /// Only available on HOS 11.0.0-11.0.1.
    #[inline]
    pub fn detach_client_cmif(&self) -> Result<(), DetachClientCmifError> {
        cmif::detach_client(self.0.session)
    }
}

/// TIPC protocol methods.
///
/// Requires HOS 12.0.0+ or Atmosphere.
impl SmService {
    /// Gets a service handle by name using TIPC protocol.
    ///
    /// Requires HOS 12.0.0+ or Atmosphere.
    #[inline]
    pub fn get_service_handle_tipc(
        &self,
        name: ServiceName,
    ) -> Result<SessionHandle, GetServiceTipcError> {
        tipc::get_service_handle(self.0.session, name)
    }

    /// Registers a service using TIPC protocol.
    #[inline]
    pub fn register_service_tipc(
        &self,
        name: ServiceName,
        is_light: bool,
        max_sessions: i32,
    ) -> Result<SessionHandle, RegisterServiceTipcError> {
        tipc::register_service(self.0.session, name, is_light, max_sessions)
    }

    /// Unregisters a service using TIPC protocol.
    #[inline]
    pub fn unregister_service_tipc(
        &self,
        name: ServiceName,
    ) -> Result<(), UnregisterServiceTipcError> {
        tipc::unregister_service(self.0.session, name)
    }

    /// Detaches the client using TIPC protocol.
    ///
    /// Only available on Atmosphere.
    #[inline]
    pub fn detach_client_tipc(&self) -> Result<(), DetachClientTipcError> {
        tipc::detach_client(self.0.session)
    }
}

/// Connects to the _Service Manager_.
///
/// Connects to the "sm:" named port and registers as a client, retrying
/// until the port becomes available (with 50ms sleep between attempts).
///
/// Returns an [`SmService`] that can be used for SM operations.
pub fn connect() -> Result<SmService, ConnectError> {
    // Connect to "sm:" named port, retrying on NotFound
    let handle = loop {
        match ipc::connect_to_named_port(SM_PORT_NAME) {
            Ok(handle) => break handle,
            Err(ipc::ConnectError::NotFound) => {
                // SM not yet available, wait and retry
                nx_svc::thread::sleep(CONNECT_RETRY_SLEEP_NS);
            }
            Err(err) => return Err(ConnectError::Connect(err)),
        }
    };

    // Create a minimal service (no pointer buffer query yet)
    let service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    // Send RegisterClient (command 0) via CMIF with send_pid=true
    cmif::register_client(service.session).map_err(ConnectError::RegisterClient)?;

    Ok(SmService(service))
}

/// Error returned by [`connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to the "sm:" named port.
    #[error("failed to connect to sm:")]
    Connect(#[source] ipc::ConnectError),
    /// Failed to register client with SM.
    #[error("failed to register client")]
    RegisterClient(#[source] cmif::RegisterClientError),
}
