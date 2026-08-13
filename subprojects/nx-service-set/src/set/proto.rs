//! Wire contract for the settings interface: what it is called, what it answers to, and the
//! values its answers carry.

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Name the settings interface is registered under.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("set");

/// `GetLanguageCode` - the tag of the language the console is set to.
pub const GET_LANGUAGE_CODE: u32 = 0;

/// `GetAvailableLanguageCodes` - every tag the console offers.
///
/// Answers into a pointer buffer, and closes the session when asked for more entries than it
/// has. Replaced at `[4.0.0]` by [`GET_AVAILABLE_LANGUAGE_CODES`], which does neither.
pub const GET_AVAILABLE_LANGUAGE_CODES_LEGACY: u32 = 1;

/// `MakeLanguageCode` - the tag for a language index, including one the console does not offer.
///
/// `[4.0.0+]`
pub const MAKE_LANGUAGE_CODE: u32 = 2;

/// `GetAvailableLanguageCodeCount` - how many tags the console offers.
///
/// Replaced at `[4.0.0]` by [`GET_AVAILABLE_LANGUAGE_CODE_COUNT`].
pub const GET_AVAILABLE_LANGUAGE_CODE_COUNT_LEGACY: u32 = 3;

/// `GetRegionCode` - which region the console was sold into.
pub const GET_REGION_CODE: u32 = 4;

/// `GetAvailableLanguageCodes` - every tag the console offers.
///
/// `[4.0.0+]`
pub const GET_AVAILABLE_LANGUAGE_CODES: u32 = 5;

/// `GetAvailableLanguageCodeCount` - how many tags the console offers.
///
/// `[4.0.0+]`
pub const GET_AVAILABLE_LANGUAGE_CODE_COUNT: u32 = 6;

/// `GetQuestFlag` - whether the console is a retail demo unit.
///
/// `[5.0.0+]`
pub const GET_QUEST_FLAG: u32 = 8;

/// `GetDeviceNickName` - the name the owner gave the console.
///
/// `[10.1.0+]`
pub const GET_DEVICE_NICKNAME: u32 = 11;

/// A language tag, as the interface carries it: NUL-padded ASCII in eight bytes.
///
/// The tag is a BCP 47 name such as `en-US` or `ja`. It is eight bytes on the wire whatever its
/// length, so a shorter tag is padded and the padding is not part of the value.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct LanguageCode([u8; LanguageCode::LEN]);

const_assert_eq!(size_of::<LanguageCode>(), size_of::<u64>());

impl LanguageCode {
    /// How many bytes the wire field holds.
    pub const LEN: usize = 8;

    /// The tag that is all padding, which is what a field nobody has written holds.
    pub(crate) const EMPTY: Self = Self([0; Self::LEN]);

    /// Returns the tag without its padding.
    pub fn as_str(&self) -> &str {
        let len = self
            .0
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(Self::LEN);
        // The constructors reject anything that is not ASCII up to the terminator, and ASCII is
        // valid UTF-8, so this range cannot fail to decode.
        core::str::from_utf8(&self.0[..len]).unwrap_or("")
    }

    /// Returns the tag as the eight bytes the interface carries it in.
    pub fn to_raw(self) -> u64 {
        u64::from_le_bytes(self.0)
    }
}

impl TryFrom<u64> for LanguageCode {
    type Error = InvalidLanguageCode;

    fn try_from(raw: u64) -> Result<Self, Self::Error> {
        let bytes = raw.to_le_bytes();

        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(Self::LEN);
        if !bytes[..len].is_ascii() {
            return Err(InvalidLanguageCode::NotAscii);
        }
        if bytes[len..].iter().any(|byte| *byte != 0) {
            return Err(InvalidLanguageCode::TrailingBytes);
        }

        Ok(Self(bytes))
    }
}

/// Error returned when a tag is decoded from the wire.
#[derive(Debug, thiserror::Error)]
pub enum InvalidLanguageCode {
    /// The tag holds a byte outside ASCII
    ///
    /// Occurs when the field is not a language tag at all: the value was read from a reply, so
    /// this means the interface answered with something this crate cannot name.
    #[error("a language tag is ASCII")]
    NotAscii,

    /// The padding after the tag is not padding
    ///
    /// Occurs when a byte after the terminator is set, which would make the tag's length depend
    /// on where a reader stops. Nothing is decoded.
    #[error("a language tag has nothing after its terminator")]
    TrailingBytes,
}

impl core::str::FromStr for LanguageCode {
    type Err = LanguageCodeParseError;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        if tag.len() > Self::LEN {
            return Err(LanguageCodeParseError::TooLong { len: tag.len() });
        }

        let mut bytes = [0u8; Self::LEN];
        bytes[..tag.len()].copy_from_slice(tag.as_bytes());

        // The same check the wire form goes through, so a tag written in a program and a tag read
        // from the console are the same value or neither is.
        Self::try_from(u64::from_le_bytes(bytes)).map_err(LanguageCodeParseError::Invalid)
    }
}

/// Error returned when a tag is parsed from text.
#[derive(Debug, thiserror::Error)]
pub enum LanguageCodeParseError {
    /// The text is longer than the field that carries it
    ///
    /// Occurs when a tag of more than [`LanguageCode::LEN`] bytes is parsed. Truncating it would
    /// produce a different language, so nothing is parsed.
    #[error(
        "a language tag takes at most {} bytes, and this one takes {len}",
        LanguageCode::LEN
    )]
    TooLong {
        /// How long the text was.
        len: usize,
    },

    /// The text is not a tag
    ///
    /// Occurs when the text holds a byte outside ASCII, or holds a terminator with more text
    /// after it.
    #[error("the text is not a language tag")]
    Invalid(#[source] InvalidLanguageCode),
}

impl AsRef<str> for LanguageCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::fmt::Debug for LanguageCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LanguageCode({:?})", self.as_str())
    }
}

/// A language the console offers, by the index the interface orders them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Language {
    /// Japanese
    Japanese = 0,
    /// American English
    AmericanEnglish = 1,
    /// French
    French = 2,
    /// German
    German = 3,
    /// Italian
    Italian = 4,
    /// Spanish
    Spanish = 5,
    /// Simplified Chinese
    Chinese = 6,
    /// Korean
    Korean = 7,
    /// Dutch
    Dutch = 8,
    /// Portuguese
    Portuguese = 9,
    /// Russian
    Russian = 10,
    /// Traditional Chinese
    Taiwanese = 11,
    /// British English
    BritishEnglish = 12,
    /// Canadian French
    CanadianFrench = 13,
    /// Latin American Spanish
    LatinAmericanSpanish = 14,
    /// Simplified Chinese, as named from `[4.0.0]`
    ChineseSimplified = 15,
    /// Traditional Chinese, as named from `[4.0.0]`
    ChineseTraditional = 16,
    /// Brazilian Portuguese
    ///
    /// `[10.1.0+]`
    BrazilianPortuguese = 17,
}

impl Language {
    /// Returns the index the interface knows this language by.
    pub fn to_raw(self) -> i32 {
        self as i32
    }
}

impl TryFrom<i32> for Language {
    type Error = UnknownLanguage;

    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Japanese),
            1 => Ok(Self::AmericanEnglish),
            2 => Ok(Self::French),
            3 => Ok(Self::German),
            4 => Ok(Self::Italian),
            5 => Ok(Self::Spanish),
            6 => Ok(Self::Chinese),
            7 => Ok(Self::Korean),
            8 => Ok(Self::Dutch),
            9 => Ok(Self::Portuguese),
            10 => Ok(Self::Russian),
            11 => Ok(Self::Taiwanese),
            12 => Ok(Self::BritishEnglish),
            13 => Ok(Self::CanadianFrench),
            14 => Ok(Self::LatinAmericanSpanish),
            15 => Ok(Self::ChineseSimplified),
            16 => Ok(Self::ChineseTraditional),
            17 => Ok(Self::BrazilianPortuguese),
            _ => Err(UnknownLanguage(raw)),
        }
    }
}

/// Error returned when a language index names no language this crate knows.
///
/// Occurs when a console offers a language added after this list was written. The index is
/// carried so a caller can report it.
#[derive(Debug, thiserror::Error)]
#[error("no language is known by the index {0}")]
pub struct UnknownLanguage(pub i32);

/// The region a console was sold into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RegionCode {
    /// Japan
    Japan = 0,
    /// The Americas
    Americas = 1,
    /// Europe
    Europe = 2,
    /// Australia and New Zealand
    Australia = 3,
    /// Hong Kong, Taiwan and Korea
    HongKongTaiwanKorea = 4,
    /// China
    China = 5,
}

impl RegionCode {
    /// Returns the value the interface names this region by.
    pub fn to_raw(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for RegionCode {
    type Error = UnknownRegionCode;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Japan),
            1 => Ok(Self::Americas),
            2 => Ok(Self::Europe),
            3 => Ok(Self::Australia),
            4 => Ok(Self::HongKongTaiwanKorea),
            5 => Ok(Self::China),
            _ => Err(UnknownRegionCode(raw)),
        }
    }
}

/// Error returned when a region value names no region this crate knows.
///
/// Occurs when the console answers with a region added after this list was written. The value is
/// carried so a caller can report it.
#[derive(Debug, thiserror::Error)]
#[error("no region is known by the value {0}")]
pub struct UnknownRegionCode(pub u32);

/// The name the owner gave the console.
///
/// The interface answers into a fixed field whatever the name's length, so the value here is that
/// field and [`DeviceNickname::as_str`] is what reads the name out of it.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct DeviceNickname {
    /// The name, NUL-terminated.
    nickname: [u8; DeviceNickname::LEN],
}

const_assert_eq!(size_of::<DeviceNickname>(), 0x80);

impl DeviceNickname {
    /// How many bytes the wire field holds.
    pub const LEN: usize = 0x80;

    /// Returns a field with nothing in it, for a command to answer into.
    pub const fn new() -> Self {
        Self {
            nickname: [0; Self::LEN],
        }
    }

    /// Returns the name without its padding.
    ///
    /// An owner may name a console in any script, and the interface does not promise the field
    /// decodes: a field that does not reads as empty rather than as an error, because a console
    /// whose name cannot be shown is still a console.
    pub fn as_str(&self) -> &str {
        let len = self
            .nickname
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(Self::LEN);
        core::str::from_utf8(&self.nickname[..len]).unwrap_or("")
    }
}

impl Default for DeviceNickname {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for DeviceNickname {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DeviceNickname({:?})", self.as_str())
    }
}

impl core::fmt::Display for DeviceNickname {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceNickname,
        InvalidLanguageCode,
        Language,
        LanguageCode,
        LanguageCodeParseError,
        RegionCode,
    };

    #[test]
    fn language_code_from_str_with_a_short_tag_pads_it() {
        //* Given
        let tag = "ja";

        //* When
        let code = tag.parse::<LanguageCode>();

        //* Then
        let code = code.expect("a two-byte ASCII tag fits the field");
        assert_eq!(code.as_str(), "ja", "the padding is not part of the tag");
        assert_eq!(
            code.to_raw(),
            u64::from_le_bytes(*b"ja\0\0\0\0\0\0"),
            "the wire form pads the tag to the width of the field"
        );
    }

    #[test]
    fn language_code_from_str_with_a_tag_longer_than_the_field_fails() {
        //* Given
        let tag = "en-US-x-longer";

        //* When
        let code = tag.parse::<LanguageCode>();

        //* Then
        let err = code.expect_err("a tag past the width of the field cannot be carried");
        assert!(
            matches!(err, LanguageCodeParseError::TooLong { len: 14 }),
            "expected TooLong carrying the length, got {err:?}"
        );
    }

    #[test]
    fn language_code_try_from_with_bytes_after_the_terminator_fails() {
        //* Given
        // A tag that ends after two bytes, with a byte set beyond the terminator.
        let raw = u64::from_le_bytes(*b"ja\0\0\0\0\0X");

        //* When
        let code = LanguageCode::try_from(raw);

        //* Then
        let err = code.expect_err("padding that is not padding leaves the tag's length ambiguous");
        assert!(
            matches!(err, InvalidLanguageCode::TrailingBytes),
            "expected TrailingBytes, got {err:?}"
        );
    }

    #[test]
    fn language_code_try_from_with_a_padded_tag_reads_the_tag_alone() {
        //* Given
        let raw = u64::from_le_bytes(*b"en-US\0\0\0");

        //* When
        let code = LanguageCode::try_from(raw);

        //* Then
        let code = code.expect("the tag is ASCII and everything after it is a terminator");
        assert_eq!(
            code.as_str(),
            "en-US",
            "the padding is not part of the tag, and Display renders what as_str returns"
        );
    }

    #[test]
    fn language_try_from_with_an_index_past_the_list_fails() {
        //* Given
        let index = 18;

        //* When
        let language = Language::try_from(index);

        //* Then
        assert!(
            language.is_err(),
            "an index this crate has no language for cannot become one"
        );
    }

    #[test]
    fn region_code_try_from_with_an_unknown_value_fails() {
        //* Given
        let raw = 6;

        //* When
        let region = RegionCode::try_from(raw);

        //* Then
        assert!(
            region.is_err(),
            "a region value this crate does not know cannot become one"
        );
    }

    #[test]
    fn device_nickname_as_str_stops_at_the_terminator() {
        //* Given
        let mut field = DeviceNickname::new();
        let name = b"Lounge Switch";
        // The wire form is the whole field, so the name is written into the front of it.
        let bytes = <DeviceNickname as zerocopy::IntoBytes>::as_mut_bytes(&mut field);
        bytes[..name.len()].copy_from_slice(name);

        //* When
        let read = field.as_str();

        //* Then
        assert_eq!(read, "Lounge Switch", "the padding is not part of the name");
    }
}
