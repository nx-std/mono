//! Protocol constants and types for the set:sys service.

use core::fmt;

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Service name for the system settings service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("set:sys");

/// Command ID for GetFirmwareVersion (pre-3.0.0).
///
/// This command zeroes the revision field in the output.
pub const CMD_GET_FIRMWARE_VERSION: u32 = 3;

/// Command ID for GetFirmwareVersion2 (3.0.0+).
///
/// This command preserves the revision field in the output.
pub const CMD_GET_FIRMWARE_VERSION_2: u32 = 4;

/// Firmware version information returned by `setsysGetFirmwareVersion`.
///
/// This structure contains detailed information about the system firmware,
/// including version numbers, platform identifier, and display strings.
///
/// # Size
///
/// This structure is exactly 0x100 bytes (256 bytes) to match the IPC buffer
/// requirements of the `GetFirmwareVersion` command.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FirmwareVersion {
    /// Major version number (e.g., 18 for firmware 18.1.0)
    pub major: u8,
    /// Minor version number (e.g., 1 for firmware 18.1.0)
    pub minor: u8,
    /// Patch version number (e.g., 0 for firmware 18.1.0)
    pub patch: u8,
    /// Padding (alignment).
    _pad1: u8,
    /// Revision major number
    pub revision_major: u8,
    /// Revision minor number
    pub revision_minor: u8,
    /// Padding (alignment).
    _pad2: [u8; 2],
    /// Platform identifier string (e.g., "NX")
    pub platform: [u8; 0x20],
    /// Version hash string (build identifier)
    pub version_hash: [u8; 0x40],
    /// Display version string (e.g., "18.1.0")
    pub display_version: [u8; 0x18],
    /// Display title string (full firmware title)
    pub display_title: [u8; 0x80],
}

const_assert_eq!(size_of::<FirmwareVersion>(), 0x100);

impl FirmwareVersion {
    /// Creates a new zeroed `FirmwareVersion`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            _pad1: 0,
            revision_major: 0,
            revision_minor: 0,
            _pad2: [0; 2],
            platform: [0; 0x20],
            version_hash: [0; 0x40],
            display_version: [0; 0x18],
            display_title: [0; 0x80],
        }
    }

    /// Returns the platform string as a `&str`, trimmed of null bytes.
    #[inline]
    pub fn platform_str(&self) -> &str {
        Self::bytes_to_str(&self.platform)
    }

    /// Returns the version hash string as a `&str`, trimmed of null bytes.
    #[inline]
    pub fn version_hash_str(&self) -> &str {
        Self::bytes_to_str(&self.version_hash)
    }

    /// Returns the display version string as a `&str`, trimmed of null bytes.
    #[inline]
    pub fn display_version_str(&self) -> &str {
        Self::bytes_to_str(&self.display_version)
    }

    /// Returns the display title string as a `&str`, trimmed of null bytes.
    #[inline]
    pub fn display_title_str(&self) -> &str {
        Self::bytes_to_str(&self.display_title)
    }

    /// Converts a fixed-size byte array to a string, stopping at the first null byte.
    fn bytes_to_str(bytes: &[u8]) -> &str {
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        // SAFETY: The firmware version strings are ASCII, which is valid UTF-8.
        // If somehow invalid UTF-8 is present, we fall back to empty string.
        core::str::from_utf8(&bytes[..len]).unwrap_or("")
    }
}

impl Default for FirmwareVersion {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FirmwareVersion")
            .field("major", &self.major)
            .field("minor", &self.minor)
            .field("patch", &self.patch)
            .field("revision_major", &self.revision_major)
            .field("revision_minor", &self.revision_minor)
            .field("platform", &self.platform_str())
            .field("display_version", &self.display_version_str())
            .finish_non_exhaustive()
    }
}
