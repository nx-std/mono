//! UART service protocol constants.

use nx_sf::ServiceName;

/// Service name for the UART service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("uart");

// IManager commands

/// Checks if a production port exists (pre-17.0.0).
pub const HAS_PORT: u32 = 0;

/// Checks if a dev port exists (pre-17.0.0).
pub const HAS_PORT_FOR_DEV: u32 = 1;

/// Checks if a baud rate is supported for a production port (pre-17.0.0).
pub const IS_SUPPORTED_BAUD_RATE: u32 = 2;

/// Checks if a baud rate is supported for a dev port (pre-17.0.0).
pub const IS_SUPPORTED_BAUD_RATE_FOR_DEV: u32 = 3;

/// Checks if a flow control mode is supported for a production port (pre-17.0.0).
pub const IS_SUPPORTED_FLOW_CONTROL_MODE: u32 = 4;

/// Checks if a flow control mode is supported for a dev port (pre-17.0.0).
pub const IS_SUPPORTED_FLOW_CONTROL_MODE_FOR_DEV: u32 = 5;

/// Creates a new port session (returns IPortSession move handle).
pub const CREATE_PORT_SESSION: u32 = 6;

/// Checks if a port event type is supported for a production port (pre-17.0.0).
pub const IS_SUPPORTED_PORT_EVENT: u32 = 7;

/// Checks if a port event type is supported for a dev port (pre-17.0.0).
pub const IS_SUPPORTED_PORT_EVENT_FOR_DEV: u32 = 8;

/// Checks if a device variation is supported for a production port ([7.0.0-16.1.0]).
pub const IS_SUPPORTED_DEVICE_VARIATION: u32 = 9;

/// Checks if a device variation is supported for a dev port ([7.0.0-16.1.0]).
pub const IS_SUPPORTED_DEVICE_VARIATION_FOR_DEV: u32 = 10;

// IPortSession commands

/// Opens a port with transfer memory buffers.
pub const PORT_OPEN: u32 = 0;

/// Opens a dev port with transfer memory buffers.
pub const PORT_OPEN_FOR_DEV: u32 = 1;

/// Gets the number of bytes available for writing.
pub const PORT_GET_WRITABLE_LENGTH: u32 = 2;

/// Sends data through the port.
pub const PORT_SEND: u32 = 3;

/// Gets the number of bytes available for reading.
pub const PORT_GET_READABLE_LENGTH: u32 = 4;

/// Receives data from the port.
pub const PORT_RECEIVE: u32 = 5;

/// Binds a port event and returns a copy handle.
pub const PORT_BIND_PORT_EVENT: u32 = 6;

/// Unbinds a port event.
pub const PORT_UNBIND_PORT_EVENT: u32 = 7;
