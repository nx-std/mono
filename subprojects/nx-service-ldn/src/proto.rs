//! Protocol constants and wire-format enums for the `ldn` services.
//!
//! Mirrors the on-wire surface of libnx's `ldn` headers without performing any
//! hosversion gating — the gating is the caller's responsibility (see the
//! crate-level docs).

use nx_sf::ServiceName;

//
// Service names
//

/// `ldn:u` — LocalCommunicationService creator (user variant).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("ldn:u");

/// `ldn:s` — LocalCommunicationService creator (system variant).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("ldn:s");

/// `ldn:m` — Monitor service creator.
pub const SERVICE_NAME_MONITOR: ServiceName = ServiceName::new_truncate("ldn:m");

//
// Priority constants used with `lcs_initialize_with_priority` on system variant.
//

/// Default `__nx_ldn_priority` for `LdnServiceType_System`.
pub const LDN_PRIORITY_SYSTEM: i32 = 0x38;
/// Alternate priority for `LdnServiceType_System`.
pub const LDN_PRIORITY_USER: i32 = 0x5A;

//
// Creator-object command IDs (sent on the converted-to-domain `ldn:u`/`ldn:s`/`ldn:m`
// session, which is the parent domain returning sub-objects).
//

/// Creator cmd: `CreateUserLocalCommService` / `CreateSystemLocalCommService` / `CreateMonitorService`.
pub const CMD_CREATE_SERVICE: u32 = 0;
/// Creator cmd: `CreateClientProcessMonitor` (`[18.0.0+]`).
pub const CMD_CREATE_CLIENT_PROCESS_MONITOR: u32 = 1;

//
// `IUser/ISystemLocalCommunicationService` command IDs.
//

pub const CMD_LCS_GET_STATE: u32 = 0;
pub const CMD_LCS_GET_NETWORK_INFO: u32 = 1;
pub const CMD_LCS_GET_IPV4_ADDRESS: u32 = 2;
pub const CMD_LCS_GET_DISCONNECT_REASON: u32 = 3;
pub const CMD_LCS_GET_SECURITY_PARAMETER: u32 = 4;
pub const CMD_LCS_GET_NETWORK_CONFIG: u32 = 5;
pub const CMD_LCS_GET_STATE_CHANGE_EVENT: u32 = 100;
pub const CMD_LCS_GET_NETWORK_INFO_AND_HISTORY: u32 = 101;
pub const CMD_LCS_SCAN: u32 = 102;
pub const CMD_LCS_SCAN_PRIVATE: u32 = 103;
pub const CMD_LCS_SET_WIRELESS_CONTROLLER_RESTRICTION: u32 = 104;
pub const CMD_LCS_SET_PROTOCOL: u32 = 106;
pub const CMD_LCS_OPEN_ACCESS_POINT: u32 = 200;
pub const CMD_LCS_CLOSE_ACCESS_POINT: u32 = 201;
pub const CMD_LCS_CREATE_NETWORK: u32 = 202;
pub const CMD_LCS_CREATE_NETWORK_PRIVATE: u32 = 203;
pub const CMD_LCS_DESTROY_NETWORK: u32 = 204;
pub const CMD_LCS_REJECT: u32 = 205;
pub const CMD_LCS_SET_ADVERTISE_DATA: u32 = 206;
pub const CMD_LCS_SET_STATION_ACCEPT_POLICY: u32 = 207;
pub const CMD_LCS_ADD_ACCEPT_FILTER_ENTRY: u32 = 208;
pub const CMD_LCS_CLEAR_ACCEPT_FILTER: u32 = 209;
pub const CMD_LCS_OPEN_STATION: u32 = 300;
pub const CMD_LCS_CLOSE_STATION: u32 = 301;
pub const CMD_LCS_CONNECT: u32 = 302;
pub const CMD_LCS_CONNECT_PRIVATE: u32 = 303;
pub const CMD_LCS_DISCONNECT: u32 = 304;
pub const CMD_LCS_INITIALIZE_LEGACY: u32 = 400;
pub const CMD_LCS_FINALIZE: u32 = 401;
/// `InitializeWithVersion` on `ldn:u` (cmd 402) / `SetOperationMode` on `ldn:s` (cmd 402).
pub const CMD_LCS_402: u32 = 402;
/// `InitializeWithVersion` on `ldn:s` (cmd 403) / `SetOperationMode` on `ldn:u` (cmd 403).
pub const CMD_LCS_403: u32 = 403;
/// `InitializeWithPriority` (`ldn:s` only, `[19.0.0+]`).
pub const CMD_LCS_INITIALIZE_WITH_PRIORITY: u32 = 404;
pub const CMD_LCS_ENABLE_ACTION_FRAME: u32 = 500;
pub const CMD_LCS_DISABLE_ACTION_FRAME: u32 = 501;
pub const CMD_LCS_SEND_ACTION_FRAME: u32 = 502;
pub const CMD_LCS_RECV_ACTION_FRAME: u32 = 503;
pub const CMD_LCS_SET_HOME_CHANNEL: u32 = 505;
pub const CMD_LCS_SET_TX_POWER: u32 = 600;
pub const CMD_LCS_RESET_TX_POWER: u32 = 601;

//
// `IMonitorService` command IDs (subset of LCS).
//

pub const CMD_MON_GET_STATE: u32 = 0;
pub const CMD_MON_GET_NETWORK_INFO: u32 = 1;
pub const CMD_MON_GET_IPV4_ADDRESS: u32 = 2;
pub const CMD_MON_GET_SECURITY_PARAMETER: u32 = 4;
pub const CMD_MON_GET_NETWORK_CONFIG: u32 = 5;
pub const CMD_MON_INITIALIZE: u32 = 100;
pub const CMD_MON_FINALIZE: u32 = 101;

//
// `IClientProcessMonitor` command IDs.
//

pub const CMD_ICPM_REGISTER_CLIENT: u32 = 0;

//
// Enums
//

/// Service flavour: `ldn:u` (user) or `ldn:s` (system).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LdnServiceType {
    User = 0,
    System = 1,
}

impl LdnServiceType {
    /// Returns the service-manager name to query.
    #[inline]
    pub const fn service_name(self) -> ServiceName {
        match self {
            Self::User => SERVICE_NAME_USER,
            Self::System => SERVICE_NAME_SYSTEM,
        }
    }
}

/// `State` reported by `GetState` (LCS cmd 0 / Monitor cmd 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LdnState {
    None = 0,
    Initialized = 1,
    AccessPoint = 2,
    AccessPointCreated = 3,
    Station = 4,
    StationConnected = 5,
    Error = 6,
}

impl LdnState {
    /// Parses the raw `u32` returned by the service. Returns `None` if the value
    /// is outside the documented range.
    #[inline]
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::Initialized),
            2 => Some(Self::AccessPoint),
            3 => Some(Self::AccessPointCreated),
            4 => Some(Self::Station),
            5 => Some(Self::StationConnected),
            6 => Some(Self::Error),
            _ => None,
        }
    }
}

/// `DisconnectReason` reported by `GetDisconnectReason` (LCS cmd 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum LdnDisconnectReason {
    None = 0,
    DisconnectedByUser = 1,
    DisconnectedBySystem = 2,
    DestroyedByUser = 3,
    DestroyedBySystem = 4,
    Rejected = 5,
    SignalLost = 6,
}

impl LdnDisconnectReason {
    /// Parses the raw `i16` returned by the service.
    #[inline]
    pub const fn from_raw(raw: i16) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::DisconnectedByUser),
            2 => Some(Self::DisconnectedBySystem),
            3 => Some(Self::DestroyedByUser),
            4 => Some(Self::DestroyedBySystem),
            5 => Some(Self::Rejected),
            6 => Some(Self::SignalLost),
            _ => None,
        }
    }
}

/// `AcceptPolicy` for `SetStationAcceptPolicy` (LCS cmd 207).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LdnAcceptPolicy {
    AlwaysAccept = 0,
    AlwaysReject = 1,
    BlackList = 2,
    WhiteList = 3,
}

/// `SecurityMode` field in `LdnSecurityConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum LdnSecurityMode {
    Any = 0,
    Product = 1,
    Debug = 2,
    SystemDebug = 3,
}

/// `OperationMode` for `SetOperationMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LdnOperationMode {
    Stable = 0,
    HighSpeed = 1,
}

/// `WirelessControllerRestriction` for `SetWirelessControllerRestriction` (LCS cmd 104).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LdnWirelessControllerRestriction {
    Disabled = 0,
    Enabled = 1,
}

/// `Protocol` for `SetProtocol` (LCS cmd 106, `[18.0.0+]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LdnProtocol {
    Nx = 1,
    Unknown3 = 3,
}

/// `ScanFilterFlag` bitmask used in [`crate::types::LdnScanFilter::flags`].
///
/// Hand-rolled newtype — no `bitflags` crate dependency, matching the rest of
/// the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct LdnScanFilterFlag(pub u32);

impl LdnScanFilterFlag {
    pub const EMPTY: Self = Self(0);
    pub const LOCAL_COMMUNICATION_ID: Self = Self(1 << 0);
    pub const SESSION_ID: Self = Self(1 << 1);
    pub const NETWORK_TYPE: Self = Self(1 << 2);
    pub const BSSID: Self = Self(1 << 3);
    pub const SSID: Self = Self(1 << 4);
    pub const SCENE_ID: Self = Self(1 << 5);
    pub const INTENT_ID: Self = Self(Self::LOCAL_COMMUNICATION_ID.0 | Self::SCENE_ID.0);
    pub const NETWORK_ID: Self = Self(Self::INTENT_ID.0 | Self::SESSION_ID.0);

    /// Returns the raw `u32` representation.
    #[inline]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Combines two flag sets.
    #[inline]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if every bit in `flag` is set.
    #[inline]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}
