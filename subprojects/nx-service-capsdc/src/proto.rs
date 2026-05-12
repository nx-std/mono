//! JPEG decoder service protocol constants.

use nx_sf::ServiceName;

/// Service name for the JPEG decoder service (`caps:dc`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:dc");

// IDecoderControlService commands

/// Decodes a JPEG buffer into RGBA8. [4.0.0+]
pub const DECODE_JPEG: u32 = 3001;

/// Shrinks a JPEG's dimensions by 2, auto-selecting quality. [17.0.0+]
pub const SHRINK_JPEG: u32 = 4001;

/// Shrinks a JPEG with explicit target dimensions and quality. [19.0.0+]
pub const SHRINK_JPEG_EX: u32 = 4002;
