//! Wire contract for the system settings interface: what it is called, what it answers to, and
//! the values its answers carry.

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Name the system settings interface is registered under.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("set:sys");

/// `GetFirmwareVersion` - the firmware the console runs, with the revision zeroed.
pub const GET_FIRMWARE_VERSION: u32 = 3;

/// `GetFirmwareVersion2` - the firmware the console runs.
///
/// `[3.0.0+]`
pub const GET_FIRMWARE_VERSION_2: u32 = 4;

/// `GetColorSetId` - which of the two system themes is selected.
pub const GET_COLOR_SET_ID: u32 = 23;

/// `GetSettingsItemValueSize` - how many bytes a settings item takes.
pub const GET_SETTINGS_ITEM_VALUE_SIZE: u32 = 37;

/// `GetSettingsItemValue` - the bytes of a settings item.
pub const GET_SETTINGS_ITEM_VALUE: u32 = 38;

/// Which of the two system themes the console is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ColorSetId {
    /// The light theme.
    Light = 0,
    /// The dark theme.
    Dark = 1,
}

impl ColorSetId {
    /// Returns the value the interface names this theme by.
    pub fn to_raw(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for ColorSetId {
    type Error = UnknownColorSetId;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Light),
            1 => Ok(Self::Dark),
            _ => Err(UnknownColorSetId(raw)),
        }
    }
}

/// Error returned when a theme value names no theme this crate knows.
///
/// Occurs when the console answers with a theme added after this crate's list was written. The
/// value is carried so a caller can report it.
#[derive(Debug, thiserror::Error)]
#[error("no system theme is known by the value {0}")]
pub struct UnknownColorSetId(pub u32);

/// Which section of the settings a item is read out of, such as `hbloader`.
///
/// A settings item is addressed by two names, and they are two types here because the interface
/// answers a section and a key the wrong way round with "no such item" rather than an error that
/// says which of them was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SettingsName(NameField);

impl SettingsName {
    /// Returns the name as the fixed field the interface carries it in.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl core::str::FromStr for SettingsName {
    type Err = InvalidSettingsText;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        name.parse::<NameField>().map(Self)
    }
}

/// Which item inside a [`SettingsName`] section is read, such as `applet_heap_size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SettingsItemKey(NameField);

impl SettingsItemKey {
    /// Returns the key as the fixed field the interface carries it in.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl core::str::FromStr for SettingsItemKey {
    type Err = InvalidSettingsText;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        key.parse::<NameField>().map(Self)
    }
}

/// The fixed field both halves of a settings item's address are carried in.
///
/// The interface reads each of them out of a field of its own width, terminator included, so a
/// name is padded to that width whatever its length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct NameField([u8; NameField::LEN]);

const_assert_eq!(size_of::<NameField>(), 0x48);

impl NameField {
    /// How many bytes the wire field holds, terminator included.
    const LEN: usize = 0x48;

    /// Returns the field, padding and all.
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl core::str::FromStr for NameField {
    type Err = InvalidSettingsText;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // One byte of the field is the terminator, so the text itself gets the rest.
        if text.len() > Self::LEN - 1 {
            return Err(InvalidSettingsText::TooLong { len: text.len() });
        }
        if !text.is_ascii() {
            return Err(InvalidSettingsText::NotAscii);
        }
        if text.bytes().any(|byte| byte == 0) {
            return Err(InvalidSettingsText::InteriorNul);
        }

        let mut field = [0u8; Self::LEN];
        field[..text.len()].copy_from_slice(text.as_bytes());

        Ok(Self(field))
    }
}

/// Error returned when a settings section name or item key is parsed from text.
#[derive(Debug, thiserror::Error)]
pub enum InvalidSettingsText {
    /// The text is longer than the field that carries it
    ///
    /// Occurs when the text leaves no room for the terminator. Sending it truncated would address
    /// a different item, so nothing is parsed.
    #[error("a settings name takes at most {} bytes, and this one takes {len}", NameField::LEN - 1)]
    TooLong {
        /// How long the text was.
        len: usize,
    },

    /// The text holds a byte outside ASCII
    ///
    /// Occurs when the text is not one of the interface's own names, which are ASCII.
    #[error("a settings name is ASCII")]
    NotAscii,

    /// The text holds a terminator of its own
    ///
    /// Occurs when the text has an interior NUL, which would address the part before it rather
    /// than the whole.
    #[error("a settings name holds no terminator of its own")]
    InteriorNul,
}

/// The firmware a console runs.
///
/// # Size
///
/// This structure is exactly 0x100 bytes (256 bytes) to match the IPC buffer
/// requirements of the `GetFirmwareVersion` command.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
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

impl core::fmt::Debug for FirmwareVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::{
        ColorSetId,
        InvalidSettingsText,
        NameField,
        SettingsItemKey,
        SettingsName,
    };

    #[test]
    fn settings_name_from_str_pads_the_name_to_the_field() {
        //* Given
        let name = "hbloader";

        //* When
        let parsed = name.parse::<SettingsName>();

        //* Then
        let parsed = parsed.expect("a short ASCII name fits the field");
        let bytes = parsed.as_bytes();
        assert_eq!(
            bytes.len(),
            NameField::LEN,
            "the field is sent at its full width whatever the name's length"
        );
        assert_eq!(&bytes[..name.len()], name.as_bytes(), "the name leads");
        assert!(
            bytes[name.len()..].iter().all(|byte| *byte == 0),
            "everything after the name is padding"
        );
    }

    #[test]
    fn settings_name_from_str_with_no_room_for_the_terminator_fails() {
        //* Given
        // A name that fills the field, leaving nothing for the terminator after it.
        let filled = [b'a'; NameField::LEN];
        let name = core::str::from_utf8(&filled).expect("ASCII is UTF-8");

        //* When
        let parsed = name.parse::<SettingsName>();

        //* Then
        let err = parsed.expect_err("a name that fills the field leaves no terminator");
        assert!(
            matches!(err, InvalidSettingsText::TooLong { .. }),
            "expected TooLong, got {err:?}"
        );
    }

    #[test]
    fn settings_item_key_from_str_with_an_interior_nul_fails() {
        //* Given
        let key = "applet\0heap_size";

        //* When
        let parsed = key.parse::<SettingsItemKey>();

        //* Then
        let err =
            parsed.expect_err("a terminator inside the key addresses only the part before it");
        assert!(
            matches!(err, InvalidSettingsText::InteriorNul),
            "expected InteriorNul, got {err:?}"
        );
    }

    #[test]
    fn color_set_id_try_from_with_an_unknown_value_fails() {
        //* Given
        let raw = 2;

        //* When
        let theme = ColorSetId::try_from(raw);

        //* Then
        assert!(
            theme.is_err(),
            "a theme value this crate does not know cannot become one"
        );
    }
}
