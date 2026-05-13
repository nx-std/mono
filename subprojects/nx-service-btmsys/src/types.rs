//! Bluetooth Manager System service wire-layout types.

use static_assertions::const_assert_eq;

/// Bluetooth device address (6-byte MAC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BtdrvAddress {
    pub address: [u8; 6],
}

const_assert_eq!(size_of::<BtdrvAddress>(), 0x6);

/// Audio device information returned by discovery/connection queries.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct BtmAudioDevice {
    pub addr: BtdrvAddress,
    pub name: [u8; 0xF9],
}

const_assert_eq!(size_of::<BtmAudioDevice>(), 0xFF);
