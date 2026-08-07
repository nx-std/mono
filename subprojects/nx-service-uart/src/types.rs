//! UART wire-layout types and enums.

use static_assertions::const_assert_eq;

/// UART production port identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UartPort {
    Bluetooth = 1,
    JoyConR = 2,
    JoyConL = 3,
    Mcu = 4,
}

/// UART development port identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UartPortForDev {
    JoyConR = 1,
    JoyConL = 2,
    Bluetooth = 3,
}

/// Flow control mode for UART ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UartFlowControlMode {
    None = 0,
    Hardware = 1,
}

/// Port event types for binding/unbinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UartPortEventType {
    SendBufferEmpty = 0,
    SendBufferReady = 1,
    ReceiveBufferReady = 2,
    ReceiveEnd = 3,
}

/// OpenPort input for pre-6.0.0 wire format.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenPortLegacyIn {
    pub port: u32,
    pub baud_rate: u32,
    pub flow_control_mode: u32,
    pub pad: u32,
    pub send_buffer_length: u64,
    pub receive_buffer_length: u64,
}

const_assert_eq!(size_of::<OpenPortLegacyIn>(), 0x20);

/// OpenPort input for 6.x wire format (adds signal inversion flags).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenPortV6In {
    pub is_invert_tx: u8,
    pub is_invert_rx: u8,
    pub is_invert_rts: u8,
    pub is_invert_cts: u8,
    pub port: u32,
    pub baud_rate: u32,
    pub flow_control_mode: u32,
    pub send_buffer_length: u64,
    pub receive_buffer_length: u64,
}

const_assert_eq!(size_of::<OpenPortV6In>(), 0x20);

/// OpenPort input for 7.0.0+ wire format (adds device variation).
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OpenPortV7In {
    pub is_invert_tx: u8,
    pub is_invert_rx: u8,
    pub is_invert_rts: u8,
    pub is_invert_cts: u8,
    pub port: u32,
    pub baud_rate: u32,
    pub flow_control_mode: u32,
    pub device_variation: u32,
    pub pad: u32,
    pub send_buffer_length: u64,
    pub receive_buffer_length: u64,
}

const_assert_eq!(size_of::<OpenPortV7In>(), 0x28);

/// BindPortEvent input payload.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct BindPortEventIn {
    pub port_event_type: u32,
    pub pad: u32,
    pub threshold: i64,
}

const_assert_eq!(size_of::<BindPortEventIn>(), 0x10);
