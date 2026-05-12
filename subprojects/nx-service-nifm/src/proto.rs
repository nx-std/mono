//! Protocol constants and small wire-format enums for the `nifm` service.
//!
//! All command IDs and enum values mirror libnx's
//! `nx/include/switch/services/nifm.h` and `nx/source/services/nifm.c`.

use nx_sf::ServiceName;

//
// Service names.
//

/// `nifm:u` — user service.
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("nifm:u");
/// `nifm:s` — system service.
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("nifm:s");
/// `nifm:a` — admin service.
pub const SERVICE_NAME_ADMIN: ServiceName = ServiceName::new_truncate("nifm:a");

//
// Creator (static service) command IDs.
//

/// `CreateGeneralService` pre-`[3.0.0]`: no `send_pid`, no payload.
pub(crate) const CMD_CREATE_GENERAL_SERVICE_OLD: u32 = 4;
/// `CreateGeneralService` `[3.0.0+]`: `send_pid` + `u64 reserved = 0` payload.
pub(crate) const CMD_CREATE_GENERAL_SERVICE: u32 = 5;

//
// IGeneralService command IDs.
//

pub(crate) const CMD_IGS_GET_CLIENT_ID: u32 = 1;
pub(crate) const CMD_IGS_CREATE_REQUEST: u32 = 4;
pub(crate) const CMD_IGS_GET_CURRENT_NETWORK_PROFILE: u32 = 5;
pub(crate) const CMD_IGS_ENUMERATE_NETWORK_PROFILES: u32 = 7;
pub(crate) const CMD_IGS_GET_NETWORK_PROFILE: u32 = 8;
pub(crate) const CMD_IGS_SET_NETWORK_PROFILE: u32 = 9;
pub(crate) const CMD_IGS_GET_CURRENT_IP_ADDRESS: u32 = 12;
pub(crate) const CMD_IGS_GET_CURRENT_IP_CONFIG_INFO: u32 = 15;
pub(crate) const CMD_IGS_SET_WIRELESS_COMMUNICATION_ENABLED: u32 = 16;
pub(crate) const CMD_IGS_IS_WIRELESS_COMMUNICATION_ENABLED: u32 = 17;
pub(crate) const CMD_IGS_GET_INTERNET_CONNECTION_STATUS: u32 = 18;
pub(crate) const CMD_IGS_IS_ETHERNET_COMMUNICATION_ENABLED: u32 = 20;
pub(crate) const CMD_IGS_IS_ANY_INTERNET_REQUEST_ACCEPTED: u32 = 21;
pub(crate) const CMD_IGS_IS_ANY_FOREGROUND_REQUEST_ACCEPTED: u32 = 22;
pub(crate) const CMD_IGS_PUT_TO_SLEEP: u32 = 23;
pub(crate) const CMD_IGS_WAKE_UP: u32 = 24;
/// `SetWowlDelayedWakeTime` (`[9.0.0+]`).
pub(crate) const CMD_IGS_SET_WOWL_DELAYED_WAKE_TIME: u32 = 43;

//
// IRequest command IDs.
//

pub(crate) const CMD_REQ_GET_REQUEST_STATE: u32 = 0;
pub(crate) const CMD_REQ_GET_RESULT: u32 = 1;
pub(crate) const CMD_REQ_GET_SYSTEM_EVENT_READABLE_HANDLES: u32 = 2;
pub(crate) const CMD_REQ_CANCEL: u32 = 3;
pub(crate) const CMD_REQ_SUBMIT: u32 = 4;
pub(crate) const CMD_REQ_SET_NETWORK_PROFILE_ID: u32 = 9;
pub(crate) const CMD_REQ_GET_APPLET_INFO: u32 = 21;
/// `SetKeptInSleep` (`[3.0.0+]`).
pub(crate) const CMD_REQ_SET_KEPT_IN_SLEEP: u32 = 23;
/// `RegisterSocketDescriptor` (`[3.0.0+]`).
pub(crate) const CMD_REQ_REGISTER_SOCKET_DESCRIPTOR: u32 = 24;
/// `UnregisterSocketDescriptor` (`[3.0.0+]`).
pub(crate) const CMD_REQ_UNREGISTER_SOCKET_DESCRIPTOR: u32 = 25;

//
// Public enums (wire values must match libnx exactly).
//

/// Selects which `nifm:*` service variant to connect to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum NifmServiceType {
    /// `nifm:u` — user.
    User = 0,
    /// `nifm:s` — system.
    System = 1,
    /// `nifm:a` — admin.
    Admin = 2,
}

impl NifmServiceType {
    /// Returns the SM service name for this kind.
    #[inline]
    pub const fn service_name(self) -> ServiceName {
        match self {
            Self::User => SERVICE_NAME_USER,
            Self::System => SERVICE_NAME_SYSTEM,
            Self::Admin => SERVICE_NAME_ADMIN,
        }
    }
}

/// Connection medium reported by `GetInternetConnectionStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NifmInternetConnectionType {
    /// Wi-Fi connection.
    WiFi = 1,
    /// Ethernet connection.
    Ethernet = 2,
}

impl NifmInternetConnectionType {
    /// Parses the raw `u8` returned by the service.
    #[inline]
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::WiFi),
            2 => Some(Self::Ethernet),
            _ => None,
        }
    }
}

/// Progress of the current internet-connection attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NifmInternetConnectionStatus {
    ConnectingUnknown1 = 0,
    ConnectingUnknown2 = 1,
    ConnectingUnknown3 = 2,
    ConnectingUnknown4 = 3,
    Connected = 4,
}

impl NifmInternetConnectionStatus {
    /// Parses the raw `u8` returned by the service.
    #[inline]
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::ConnectingUnknown1),
            1 => Some(Self::ConnectingUnknown2),
            2 => Some(Self::ConnectingUnknown3),
            3 => Some(Self::ConnectingUnknown4),
            4 => Some(Self::Connected),
            _ => None,
        }
    }
}

/// Lifecycle state of an `IRequest`, as returned by `GetRequestState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NifmRequestState {
    /// Error. Also used as the initial sentinel before any state was fetched.
    Invalid = 0,
    /// Not yet submitted or transient error.
    Unknown1 = 1,
    /// Submitted; awaiting state change.
    OnHold = 2,
    /// Request is satisfied; an internet connection is available.
    Available = 3,
    Unknown4 = 4,
    Unknown5 = 5,
}

impl NifmRequestState {
    /// Maps a raw `u32` to a known variant, returning `Invalid` for unknown values.
    /// libnx's request-state cache uses `0` (`Invalid`) on dispatch failure.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Unknown1,
            2 => Self::OnHold,
            3 => Self::Available,
            4 => Self::Unknown4,
            5 => Self::Unknown5,
            _ => Self::Invalid,
        }
    }
}

/// Wi-Fi authentication scheme stored in a network profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NifmAuthentication {
    Invalid = 0,
    Open = 1,
    Shared = 2,
    Wpa = 3,
    WpaPsk = 4,
    Wpa2 = 5,
    Wpa2Psk = 6,
    Unk7 = 7,
}

/// Wi-Fi link-layer encryption scheme stored in a network profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NifmEncryption {
    Invalid = 0,
    None = 1,
    Wep = 2,
    Tkip = 3,
    Aes = 4,
}

/// Network-profile classification reported / filtered by `EnumerateNetworkProfiles`.
///
/// libnx exposes the underlying bit values via `BIT(0..=2)`; we keep a Rust
/// enum because `EnumerateNetworkProfiles` takes a single discriminant value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NifmNetworkProfileType {
    /// Profile saved by the user.
    User = 1,
    /// Hardcoded list of Nintendo hotspots.
    SsidList = 2,
    /// Temporary profile.
    Temporary = 4,
}

impl NifmNetworkProfileType {
    /// Returns the raw `u8` discriminant used on the wire.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}
