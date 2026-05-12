//! Board Power Control (`bpc`) protocol constants.

use nx_sf::ServiceName;

/// Service name for BPC on HOS 2.0.0+.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("bpc");

/// Legacy service name for BPC on HOS < 2.0.0.
pub const SERVICE_NAME_LEGACY: ServiceName = ServiceName::new_truncate("bpc:c");

/// Initiates a full system shutdown.
pub const SHUTDOWN_SYSTEM: u32 = 0;

/// Initiates a full system reboot.
pub const REBOOT_SYSTEM: u32 = 1;

/// Gets the current sleep button state. Available on [2.0.0–13.2.1].
pub const GET_SLEEP_BUTTON_STATE: u32 = 6;

/// Gets whether the power button is currently pushed. Available on [6.0.0+].
pub const GET_POWER_BUTTON: u32 = 14;
