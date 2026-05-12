//! Protocol constants and wire-format types for the wlan:inf service.

use nx_sf::ServiceName;

/// Service name for the WLAN InfraManager service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("wlan:inf");

/// Command ID for `GetState`.
pub const CMD_GET_STATE: u32 = 10;

/// Command ID for `GetRSSI`.
pub const CMD_GET_RSSI: u32 = 12;

/// WLAN connection state reported by `GetState` (cmd 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WlanInfState {
    /// WLAN is disabled, or enabled and not connected.
    NotConnected = 1,
    /// WLAN is connecting.
    Connecting = 2,
    /// WLAN is connected.
    Connected = 3,
}

impl WlanInfState {
    /// Parse a raw `u32` as returned by the service.
    #[inline]
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            1 => Some(Self::NotConnected),
            2 => Some(Self::Connecting),
            3 => Some(Self::Connected),
            _ => None,
        }
    }
}

/// Received signal strength indicator returned by `GetRSSI` (cmd 12).
///
/// Range is roughly –30 dBm (strong) to –90 dBm (barely connected) on a
/// logarithmic scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Rssi(i32);

impl Rssi {
    /// Wraps a raw signed RSSI value as returned by the service.
    #[inline]
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Returns the RSSI in dBm.
    #[inline]
    pub const fn dbm(self) -> i32 {
        self.0
    }
}
