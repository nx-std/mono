//! LP2P service protocol constants.

use nx_sf::ServiceName;

/// Service name for `lp2p:app`.
pub const SERVICE_NAME_APP: ServiceName = ServiceName::new_truncate("lp2p:app");

/// Service name for `lp2p:sys`.
pub const SERVICE_NAME_SYS: ServiceName = ServiceName::new_truncate("lp2p:sys");

// ---------------------------------------------------------------------------
// Root service commands
// ---------------------------------------------------------------------------

/// Creates an INetworkService sub-object (cmd 0, PID + u32).
pub const CREATE_NETWORK_SERVICE: u32 = 0;

/// Creates an INetworkServiceMonitor sub-object (cmd 8, PID).
pub const CREATE_NETWORK_SERVICE_MONITOR: u32 = 8;

// ---------------------------------------------------------------------------
// INetworkService commands (dispatched on the domain sub-object)
// ---------------------------------------------------------------------------

/// Scans for nearby groups (cmd 512).
pub const SCAN: u32 = 512;

/// Creates a group (cmd 768).
pub const CREATE_GROUP: u32 = 768;

/// Destroys the current group (cmd 776).
pub const DESTROY_GROUP: u32 = 776;

/// Sets advertise data (cmd 784).
pub const SET_ADVERTISE_DATA: u32 = 784;

/// Sends data to another group (cmd 1536).
pub const SEND_TO_OTHER_GROUP: u32 = 1536;

/// Receives data from another group (cmd 1544).
pub const RECV_FROM_OTHER_GROUP: u32 = 1544;

/// Adds an acceptable group ID (cmd 1552).
pub const ADD_ACCEPTABLE_GROUP_ID: u32 = 1552;

/// Removes the acceptable group ID (cmd 1560).
pub const REMOVE_ACCEPTABLE_GROUP_ID: u32 = 1560;

// ---------------------------------------------------------------------------
// INetworkServiceMonitor commands (dispatched on the non-domain session)
// ---------------------------------------------------------------------------

/// Attaches the network interface state change event (cmd 256).
pub const ATTACH_NETWORK_INTERFACE_STATE_CHANGE_EVENT: u32 = 256;

/// Gets the last network interface error (cmd 264).
pub const GET_NETWORK_INTERFACE_LAST_ERROR: u32 = 264;

/// Gets the current role (cmd 272).
pub const GET_ROLE: u32 = 272;

/// Gets advertise data with role validation (cmd 280).
pub const GET_ADVERTISE_DATA: u32 = 280;

/// Gets advertise data without role validation (cmd 281).
pub const GET_ADVERTISE_DATA_2: u32 = 281;

/// Gets the current group info (cmd 288).
pub const GET_GROUP_INFO: u32 = 288;

/// Joins a group (cmd 296).
pub const JOIN: u32 = 296;

/// Gets the group owner info (cmd 304).
pub const GET_GROUP_OWNER: u32 = 304;

/// Gets the IP configuration (cmd 312).
pub const GET_IP_CONFIG: u32 = 312;

/// Leaves the current group (cmd 320).
pub const LEAVE: u32 = 320;

/// Attaches the join event (cmd 328).
pub const ATTACH_JOIN_EVENT: u32 = 328;

/// Gets the current group members (cmd 336).
pub const GET_MEMBERS: u32 = 336;
