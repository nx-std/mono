//! I2C bus service (`i2c`) implementation.
//!
//! Provides access to the Switch's I2C bus for communicating with hardware
//! peripherals such as temperature sensors, PMICs, and battery controllers.
//!
//! ## Usage
//!
//! 1. Connect to the I2C manager via [`connect_cmif`].
//! 2. Open a device session via [`I2cService::open_session`].
//! 3. Send/receive data through the [`I2cSession`] wrapper.
//! 4. Sessions and the service are closed automatically on `Drop`.
//!
//! ## Divergence from libnx
//!
//! libnx's `i2c.c` keeps a guarded global singleton (`g_i2cSrv`) managed by
//! `NX_GENERATE_SERVICE_GUARD`. This crate follows the convention of the
//! other `nx-service-*` crates: connect once via [`connect_cmif`], reuse
//! the service wrapper across calls, and let `Drop` close the session.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{ExecuteCommandListError, OpenSessionError, ReceiveAutoError, SendAutoError},
    proto::SERVICE_NAME,
    types::{I2cDevice, I2cTransactionOption},
};

/// I2C manager service wrapper.
///
/// Manages access to the I2C bus and opens sessions for specific devices.
#[repr(transparent)]
pub struct I2cService(Session);

impl I2cService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the I2C manager.
impl I2cService {
    /// Opens a session for the specified I2C device.
    ///
    /// The returned [`I2cSession`] can be used to send and receive data
    /// on the device's I2C bus.
    #[inline]
    pub fn open_session(&self, device: I2cDevice) -> Result<I2cSession, OpenSessionError> {
        let service = cmif::open_session(self.0.handle(), device as u32)?;
        Ok(I2cSession(service))
    }
}

/// I2C device session wrapper.
///
/// Represents an open session to a specific I2C device. Provides methods
/// for sending data, receiving data, and executing command lists.
pub struct I2cSession(Session);

impl I2cSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the I2C device session.
impl I2cSession {
    /// Sends data to the I2C device with automatic buffer selection.
    ///
    /// The `option` parameter controls START/STOP condition generation
    /// on the I2C bus.
    #[inline]
    pub fn send_auto(&self, buf: &[u8], option: I2cTransactionOption) -> Result<(), SendAutoError> {
        cmif::send_auto(&self.0, buf, option)
    }

    /// Receives data from the I2C device with automatic buffer selection.
    ///
    /// The `option` parameter controls START/STOP condition generation
    /// on the I2C bus. Data is written into `buf`.
    #[inline]
    pub fn receive_auto(
        &self,
        buf: &mut [u8],
        option: I2cTransactionOption,
    ) -> Result<(), ReceiveAutoError> {
        cmif::receive_auto(&self.0, buf, option)
    }

    /// Executes a command list on the I2C device.
    ///
    /// Sends `cmd_list` as an input pointer buffer and receives the
    /// result into `dst` via automatic buffer selection.
    #[inline]
    pub fn execute_command_list(
        &self,
        dst: &mut [u8],
        cmd_list: &[u8],
    ) -> Result<(), ExecuteCommandListError> {
        cmif::execute_command_list(&self.0, dst, cmd_list)
    }
}

/// Connects to the I2C bus service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<I2cService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(I2cService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get i2c service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
