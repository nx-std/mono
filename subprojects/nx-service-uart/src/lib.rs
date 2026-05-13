//! UART service (`uart`) implementation.
//!
//! Provides access to the Switch's UART ports for serial communication
//! with Bluetooth, Joy-Con controllers, and MCU hardware.
//!
//! ## Usage
//!
//! 1. Connect to the UART manager via [`connect_cmif`].
//! 2. Query port capabilities via the `has_port` / `is_supported_*` methods.
//! 3. Create a port session via [`UartService::create_port_session`].
//! 4. Open the port, then send/receive data through the [`UartPortSession`].
//! 5. Sessions and the service are closed automatically on `Drop`.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        BindPortEventError, CreatePortSessionError, DispatchInTwoU32sOutBoolError,
        DispatchInU32OutBoolError, DispatchOutU64Error, OpenPortError, PortReceiveError,
        PortSendError,
    },
    proto::SERVICE_NAME,
    types::{
        BindPortEventIn, OpenPortLegacyIn, OpenPortV6In, OpenPortV7In, UartFlowControlMode,
        UartPort, UartPortEventType, UartPortForDev,
    },
};

/// UART manager service wrapper.
#[repr(transparent)]
pub struct UartService(Session);

impl UartService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the UART manager.
impl UartService {
    /// Checks if a production port exists (pre-17.0.0).
    #[inline]
    pub fn has_port(&self, port: UartPort) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::has_port(self.0.handle(), port as u32)
    }

    /// Checks if a dev port exists (pre-17.0.0).
    #[inline]
    pub fn has_port_for_dev(
        &self,
        port: UartPortForDev,
    ) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::has_port_for_dev(self.0.handle(), port as u32)
    }

    /// Checks if a baud rate is supported for a production port (pre-17.0.0).
    #[inline]
    pub fn is_supported_baud_rate(
        &self,
        port: UartPort,
        baud_rate: u32,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_baud_rate(self.0.handle(), port as u32, baud_rate)
    }

    /// Checks if a baud rate is supported for a dev port (pre-17.0.0).
    #[inline]
    pub fn is_supported_baud_rate_for_dev(
        &self,
        port: UartPortForDev,
        baud_rate: u32,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_baud_rate_for_dev(self.0.handle(), port as u32, baud_rate)
    }

    /// Checks if a flow control mode is supported for a production port (pre-17.0.0).
    #[inline]
    pub fn is_supported_flow_control_mode(
        &self,
        port: UartPort,
        mode: UartFlowControlMode,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_flow_control_mode(self.0.handle(), port as u32, mode as u32)
    }

    /// Checks if a flow control mode is supported for a dev port (pre-17.0.0).
    #[inline]
    pub fn is_supported_flow_control_mode_for_dev(
        &self,
        port: UartPortForDev,
        mode: UartFlowControlMode,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_flow_control_mode_for_dev(self.0.handle(), port as u32, mode as u32)
    }

    /// Creates a new port session.
    #[inline]
    pub fn create_port_session(&self) -> Result<UartPortSession, CreatePortSessionError> {
        let service = cmif::create_port_session(self.0.handle())?;
        Ok(UartPortSession(service))
    }

    /// Checks if a port event type is supported for a production port (pre-17.0.0).
    #[inline]
    pub fn is_supported_port_event(
        &self,
        port: UartPort,
        event_type: UartPortEventType,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_port_event(self.0.handle(), port as u32, event_type as u32)
    }

    /// Checks if a port event type is supported for a dev port (pre-17.0.0).
    #[inline]
    pub fn is_supported_port_event_for_dev(
        &self,
        port: UartPortForDev,
        event_type: UartPortEventType,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_port_event_for_dev(self.0.handle(), port as u32, event_type as u32)
    }

    /// Checks if a device variation is supported for a production port ([7.0.0-16.1.0]).
    #[inline]
    pub fn is_supported_device_variation(
        &self,
        port: UartPort,
        device_variation: u32,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_device_variation(self.0.handle(), port as u32, device_variation)
    }

    /// Checks if a device variation is supported for a dev port ([7.0.0-16.1.0]).
    #[inline]
    pub fn is_supported_device_variation_for_dev(
        &self,
        port: UartPortForDev,
        device_variation: u32,
    ) -> Result<bool, DispatchInTwoU32sOutBoolError> {
        cmif::is_supported_device_variation_for_dev(self.0.handle(), port as u32, device_variation)
    }
}

/// UART port session wrapper.
///
/// Represents an open session to a specific UART port. Provides methods
/// for opening the port, sending/receiving data, and managing events.
pub struct UartPortSession(Session);

impl UartPortSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> nx_svc::ipc::Handle {
        self.0.handle()
    }
}

/// CMIF protocol methods for UART port sessions.
impl UartPortSession {
    /// Opens a production port using the pre-6.0.0 wire format (legacy).
    ///
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_legacy(
        &self,
        port: UartPort,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_legacy(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Opens a production port using the 6.x wire format.
    ///
    /// Adds signal inversion flags compared to the legacy variant.
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_v6(
        &self,
        port: UartPort,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        is_invert_tx: bool,
        is_invert_rx: bool,
        is_invert_rts: bool,
        is_invert_cts: bool,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_v6(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            is_invert_tx,
            is_invert_rx,
            is_invert_rts,
            is_invert_cts,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Opens a production port using the 7.0.0+ wire format.
    ///
    /// Adds device variation parameter compared to the 6.x variant.
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_v7(
        &self,
        port: UartPort,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        device_variation: u32,
        is_invert_tx: bool,
        is_invert_rx: bool,
        is_invert_rts: bool,
        is_invert_cts: bool,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_v7(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            device_variation,
            is_invert_tx,
            is_invert_rx,
            is_invert_rts,
            is_invert_cts,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Opens a dev port using the pre-6.0.0 wire format (legacy).
    ///
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_for_dev_legacy(
        &self,
        port: UartPortForDev,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_for_dev_legacy(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Opens a dev port using the 6.x wire format.
    ///
    /// Adds signal inversion flags compared to the legacy variant.
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_for_dev_v6(
        &self,
        port: UartPortForDev,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        is_invert_tx: bool,
        is_invert_rx: bool,
        is_invert_rts: bool,
        is_invert_cts: bool,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_for_dev_v6(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            is_invert_tx,
            is_invert_rx,
            is_invert_rts,
            is_invert_cts,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Opens a dev port using the 7.0.0+ wire format.
    ///
    /// Adds device variation parameter compared to the 6.x variant.
    /// The caller must provide raw transfer-memory handles for the send
    /// and receive buffers. Buffers must be 0x1000-byte aligned.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn open_port_for_dev_v7(
        &self,
        port: UartPortForDev,
        baud_rate: u32,
        flow_control_mode: UartFlowControlMode,
        device_variation: u32,
        is_invert_tx: bool,
        is_invert_rx: bool,
        is_invert_rts: bool,
        is_invert_cts: bool,
        send_tmem_handle: u32,
        receive_tmem_handle: u32,
        send_buffer_length: u64,
        receive_buffer_length: u64,
    ) -> Result<bool, OpenPortError> {
        cmif::port_open_for_dev_v7(
            &self.0,
            port as u32,
            baud_rate,
            flow_control_mode as u32,
            device_variation,
            is_invert_tx,
            is_invert_rx,
            is_invert_rts,
            is_invert_cts,
            send_tmem_handle,
            receive_tmem_handle,
            send_buffer_length,
            receive_buffer_length,
        )
    }

    /// Gets the number of bytes available for writing.
    #[inline]
    pub fn get_writable_length(&self) -> Result<u64, DispatchOutU64Error> {
        cmif::port_get_writable_length(self.0.handle())
    }

    /// Sends data through the port.
    ///
    /// Returns the number of bytes actually written.
    #[inline]
    pub fn send(&self, data: &[u8]) -> Result<u64, PortSendError> {
        cmif::port_send(&self.0, data)
    }

    /// Gets the number of bytes available for reading.
    #[inline]
    pub fn get_readable_length(&self) -> Result<u64, DispatchOutU64Error> {
        cmif::port_get_readable_length(self.0.handle())
    }

    /// Receives data from the port.
    ///
    /// Returns the number of bytes actually read.
    #[inline]
    pub fn receive(&self, buf: &mut [u8]) -> Result<u64, PortReceiveError> {
        cmif::port_receive(&self.0, buf)
    }

    /// Binds a port event and returns (success, event_handle).
    ///
    /// The returned event handle has autoclear=false. The caller must close
    /// it after calling [`unbind_port_event`](Self::unbind_port_event).
    #[inline]
    pub fn bind_port_event(
        &self,
        event_type: UartPortEventType,
        threshold: i64,
    ) -> Result<(bool, u32), BindPortEventError> {
        cmif::port_bind_port_event(&self.0, event_type as u32, threshold)
    }

    /// Unbinds a previously bound port event.
    #[inline]
    pub fn unbind_port_event(
        &self,
        event_type: UartPortEventType,
    ) -> Result<bool, DispatchInU32OutBoolError> {
        cmif::port_unbind_port_event(self.0.handle(), event_type as u32)
    }
}

/// Connects to the UART service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<UartService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(UartService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get uart service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
