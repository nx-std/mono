//! HTCS service protocol constants.

use nx_sf::ServiceName;

/// Service name for `htcs`.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("htcs");

// ---------------------------------------------------------------------------
// IHtcsManager commands (dispatched on the root domain object)
// ---------------------------------------------------------------------------

/// Gets the "any" peer name (cmd 10).
pub const GET_PEER_NAME_ANY: u32 = 10;

/// Gets the default host name (cmd 11).
pub const GET_DEFAULT_HOST_NAME: u32 = 11;

/// Creates a new socket sub-object (cmd 13).
pub const CREATE_SOCKET: u32 = 13;

/// PID initialization for the manager session (cmd 100).
pub const MANAGER_PID_INIT: u32 = 100;

/// PID initialization for the monitor session (cmd 101).
pub const MONITOR_PID_INIT: u32 = 101;

/// Starts a select operation (cmd 130).
pub const START_SELECT: u32 = 130;

/// Ends a select operation (cmd 131).
pub const END_SELECT: u32 = 131;

// ---------------------------------------------------------------------------
// ISocket commands (dispatched on socket domain sub-objects)
// ---------------------------------------------------------------------------

/// Closes the socket (cmd 0).
pub const SOCKET_CLOSE: u32 = 0;

/// Connects to an address (cmd 1).
pub const SOCKET_CONNECT: u32 = 1;

/// Binds to an address (cmd 2).
pub const SOCKET_BIND: u32 = 2;

/// Listens for connections (cmd 3).
pub const SOCKET_LISTEN: u32 = 3;

/// Shuts down part of a connection (cmd 7).
pub const SOCKET_SHUTDOWN: u32 = 7;

/// File control (cmd 8).
pub const SOCKET_FCNTL: u32 = 8;

/// Starts an async accept operation (cmd 9).
pub const SOCKET_ACCEPT_START: u32 = 9;

/// Gets the result of an async accept (cmd 10).
pub const SOCKET_ACCEPT_RESULTS: u32 = 10;

/// Starts an async recv operation (cmd 11).
pub const SOCKET_RECV_START: u32 = 11;

/// Gets the result of an async recv (cmd 12).
pub const SOCKET_RECV_RESULTS: u32 = 12;

/// Gets the result of an async send (cmd 16).
pub const SOCKET_SEND_RESULTS: u32 = 16;

/// Starts a large-buffer send operation (cmd 17).
pub const SOCKET_START_SEND: u32 = 17;

/// Ends a large-buffer send operation (cmd 19).
pub const SOCKET_END_SEND: u32 = 19;

/// Starts a large-buffer recv operation (cmd 20).
pub const SOCKET_START_RECV: u32 = 20;

/// Ends a large-buffer recv operation (cmd 21).
pub const SOCKET_END_RECV: u32 = 21;

/// Starts an async send operation with buffer (cmd 22).
pub const SOCKET_SEND_START: u32 = 22;

/// Continues a large-buffer send operation (cmd 23).
pub const SOCKET_CONTINUE_SEND: u32 = 23;

/// Gets the underlying file descriptor (cmd 130).
pub const SOCKET_GET_PRIMITIVE: u32 = 130;
