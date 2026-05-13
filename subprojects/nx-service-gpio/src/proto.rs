//! GPIO service protocol constants.

use nx_sf::ServiceName;

/// Service name for the GPIO service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("gpio");

// IGpioManager commands

/// Opens a pad session by pad name.
pub const OPEN_SESSION: u32 = 1;

/// Checks if a wake event is active (pre-7.0.0).
pub const IS_WAKE_EVENT_ACTIVE: u32 = 3;

/// Opens a pad session by device code (7.0.0+).
pub const OPEN_SESSION2: u32 = 7;

/// Checks if a wake event is active by device code (7.0.0+).
pub const IS_WAKE_EVENT_ACTIVE2: u32 = 8;

// IGpioPadSession commands

/// Sets the pad direction.
pub const PAD_SET_DIRECTION: u32 = 0;

/// Gets the pad direction.
pub const PAD_GET_DIRECTION: u32 = 1;

/// Sets the interrupt mode.
pub const PAD_SET_INTERRUPT_MODE: u32 = 2;

/// Gets the interrupt mode.
pub const PAD_GET_INTERRUPT_MODE: u32 = 3;

/// Enables or disables the interrupt.
pub const PAD_SET_INTERRUPT_ENABLE: u32 = 4;

/// Gets whether the interrupt is enabled.
pub const PAD_GET_INTERRUPT_ENABLE: u32 = 5;

/// Gets the interrupt status (pre-17.0.0).
pub const PAD_GET_INTERRUPT_STATUS: u32 = 6;

/// Clears the interrupt status (pre-17.0.0).
pub const PAD_CLEAR_INTERRUPT_STATUS: u32 = 7;

/// Sets the pad output value.
pub const PAD_SET_VALUE: u32 = 8;

/// Gets the pad input value.
pub const PAD_GET_VALUE: u32 = 9;

/// Binds the interrupt and returns an event handle.
pub const PAD_BIND_INTERRUPT: u32 = 10;

/// Unbinds the interrupt.
pub const PAD_UNBIND_INTERRUPT: u32 = 11;

/// Enables or disables debounce.
pub const PAD_SET_DEBOUNCE_ENABLED: u32 = 12;

/// Gets whether debounce is enabled.
pub const PAD_GET_DEBOUNCE_ENABLED: u32 = 13;

/// Sets the debounce time in milliseconds.
pub const PAD_SET_DEBOUNCE_TIME: u32 = 14;

/// Gets the debounce time in milliseconds.
pub const PAD_GET_DEBOUNCE_TIME: u32 = 15;
