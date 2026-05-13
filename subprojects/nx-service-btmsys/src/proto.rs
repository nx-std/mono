//! Bluetooth Manager System service protocol constants.

use nx_sf::ServiceName;

/// Service name for btm:sys.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("btm:sys");

// IBtmSystemCore commands

/// StartGamepadPairing (cmd 0).
pub const START_GAMEPAD_PAIRING: u32 = 0;

/// CancelGamepadPairing (cmd 1).
pub const CANCEL_GAMEPAD_PAIRING: u32 = 1;

/// ClearGamepadPairingDatabase (cmd 2).
pub const CLEAR_GAMEPAD_PAIRING_DATABASE: u32 = 2;

/// GetPairedGamepadCount (cmd 3).
pub const GET_PAIRED_GAMEPAD_COUNT: u32 = 3;

/// EnableRadio (cmd 4).
pub const ENABLE_RADIO: u32 = 4;

/// DisableRadio (cmd 5).
pub const DISABLE_RADIO: u32 = 5;

/// GetRadioOnOff (cmd 6).
pub const GET_RADIO_ON_OFF: u32 = 6;

/// AcquireRadioEvent (cmd 7, 3.0.0+).
pub const ACQUIRE_RADIO_EVENT: u32 = 7;

/// AcquireGamepadPairingEvent (cmd 8, 3.0.0+).
pub const ACQUIRE_GAMEPAD_PAIRING_EVENT: u32 = 8;

/// IsGamepadPairingStarted (cmd 9, 3.0.0+).
pub const IS_GAMEPAD_PAIRING_STARTED: u32 = 9;

/// StartAudioDeviceDiscovery (cmd 10, 13.0.0+).
pub const START_AUDIO_DEVICE_DISCOVERY: u32 = 10;

/// StopAudioDeviceDiscovery (cmd 11, 13.0.0+).
pub const STOP_AUDIO_DEVICE_DISCOVERY: u32 = 11;

/// IsDiscoveryingAudioDevice (cmd 12, 13.0.0+).
pub const IS_DISCOVERYING_AUDIO_DEVICE: u32 = 12;

/// GetDiscoveredAudioDevice (cmd 13, 13.0.0+).
pub const GET_DISCOVERED_AUDIO_DEVICE: u32 = 13;

/// AcquireAudioDeviceConnectionEvent (cmd 14, 13.0.0+).
pub const ACQUIRE_AUDIO_DEVICE_CONNECTION_EVENT: u32 = 14;

/// ConnectAudioDevice (cmd 15, 13.0.0+).
pub const CONNECT_AUDIO_DEVICE: u32 = 15;

/// IsConnectingAudioDevice (cmd 16, 13.0.0+).
pub const IS_CONNECTING_AUDIO_DEVICE: u32 = 16;

/// GetConnectedAudioDevices (cmd 17, 13.0.0+).
pub const GET_CONNECTED_AUDIO_DEVICES: u32 = 17;

/// DisconnectAudioDevice (cmd 18, 13.0.0+).
pub const DISCONNECT_AUDIO_DEVICE: u32 = 18;

/// AcquirePairedAudioDeviceInfoChangedEvent (cmd 19, 13.0.0+).
pub const ACQUIRE_PAIRED_AUDIO_DEVICE_INFO_CHANGED_EVENT: u32 = 19;

/// GetPairedAudioDevices (cmd 20, 13.0.0+).
pub const GET_PAIRED_AUDIO_DEVICES: u32 = 20;

/// RemoveAudioDevicePairing (cmd 21, 13.0.0+).
pub const REMOVE_AUDIO_DEVICE_PAIRING: u32 = 21;

/// RequestAudioDeviceConnectionRejection (cmd 22, 13.0.0+).
pub const REQUEST_AUDIO_DEVICE_CONNECTION_REJECTION: u32 = 22;

/// CancelAudioDeviceConnectionRejection (cmd 23, 13.0.0+).
pub const CANCEL_AUDIO_DEVICE_CONNECTION_REJECTION: u32 = 23;

// Root service commands

/// GetCore (cmd 0) — returns IBtmSystemCore sub-object.
pub const GET_CORE: u32 = 0;
