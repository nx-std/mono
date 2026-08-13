//! Screenshot control service protocol constants.

use nx_sf::ServiceName;

/// Service name for the screenshot control service (`caps:sc`).
pub const CAPSSC_SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:sc");

// IScreenShotControlService commands

/// Captures a raw RGBA8 screenshot with a timeout. [2.0.0+, stubbed 5.0.0+]
pub const CAPTURE_RAW_IMAGE_WITH_TIMEOUT: u32 = 2;

/// Opens a raw screenshot read stream. [3.0.0+, debug mode]
pub const OPEN_RAW_SCREEN_SHOT_READ_STREAM: u32 = 1201;

/// Closes a raw screenshot read stream. [3.0.0+, debug mode]
pub const CLOSE_RAW_SCREEN_SHOT_READ_STREAM: u32 = 1202;

/// Reads from a raw screenshot read stream. [3.0.0+, debug mode]
pub const READ_RAW_SCREEN_SHOT_READ_STREAM: u32 = 1203;

/// Captures a JPEG screenshot. [9.0.0+, debug mode before 10.0.0]
pub const CAPTURE_JPEG_SCREEN_SHOT: u32 = 1204;
