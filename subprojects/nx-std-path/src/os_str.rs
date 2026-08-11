//! Platform strings: the bytes an operating system accepts, before anything decides they are text.
//!
//! [`OsStr`] is the borrowed form and [`OsString`] the owned one, and they relate to each other the
//! way [`str`] and [`String`] do. What separates them from those two is the absence of an encoding
//! rule: an `OsStr` is any byte sequence, because that is what Horizon's filesystem accepts and
//! what a caller coming through the C standard library is able to hand over.

use alloc::{
    borrow::{
        Cow,
        ToOwned,
    },
    string::String,
    vec::Vec,
};
use core::{
    borrow::Borrow,
    ops::Deref,
};

/// A borrowed platform string.
///
/// Unsized, and laid out as the bytes it wraps, so a `&OsStr` costs exactly what a `&[u8]` does and
/// borrowing one from a byte slice moves nothing.
///
/// ## Formatting
///
/// [`Display`](#impl-Display-for-OsStr) renders the bytes lossily; [`Debug`](#impl-Debug-for-OsStr)
/// quotes that same rendering.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct OsStr {
    inner: [u8],
}

impl OsStr {
    /// Borrows `s` as a platform string.
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }

    /// Borrows `bytes` as a platform string.
    ///
    /// The bytes are taken as they stand: this is the entry point for a path that arrived from the
    /// C standard library or from an IPC response, neither of which promises an encoding.
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        // SAFETY: `OsStr` is a `repr(transparent)` wrapper around `[u8]`, so a `&[u8]` and a
        // `&OsStr` have the same layout and the same validity requirements.
        unsafe { &*(bytes as *const [u8] as *const OsStr) }
    }

    /// Returns the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Returns the string as text, or `None` when the bytes are not valid UTF-8.
    pub fn to_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.inner).ok()
    }

    /// Returns the string as text, standing in `U+FFFD` for each byte sequence that is not UTF-8.
    ///
    /// Borrows when the bytes are already UTF-8, so the common case allocates nothing.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        match core::str::from_utf8(&self.inner) {
            Ok(text) => Cow::Borrowed(text),
            Err(_) => Cow::Owned(alloc::format!("{self}")),
        }
    }

    /// Copies the string into an owned one.
    pub fn to_os_string(&self) -> OsString {
        OsString::from_vec(self.inner.to_vec())
    }

    /// Returns the length in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Reports whether the string holds no bytes.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl core::fmt::Display for OsStr {
    /// Renders the bytes as text, standing in `U+FFFD` for each sequence that is not UTF-8, so a
    /// platform string can be reported without first proving it is text.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::OsStr;
    /// assert_eq!(
    ///     alloc::format!("{}", OsStr::from_bytes(b"save\xffdat")),
    ///     "save\u{fffd}dat"
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut rest = &self.inner[..];
        loop {
            let err = match core::str::from_utf8(rest) {
                Ok(text) => return f.write_str(text),
                Err(err) => err,
            };

            let (valid, invalid) = rest.split_at(err.valid_up_to());
            // SAFETY: `valid_up_to` reports how many leading bytes decoded as UTF-8, so the prefix
            // it names is valid UTF-8 by construction.
            f.write_str(unsafe { core::str::from_utf8_unchecked(valid) })?;
            core::fmt::Write::write_char(f, char::REPLACEMENT_CHARACTER)?;

            // A `None` length means the input ended mid-character rather than holding a bad byte,
            // so the replacement above stood in for the remainder and there is nothing left.
            match err.error_len() {
                Some(len) => rest = &invalid[len..],
                None => return Ok(()),
            }
        }
    }
}

impl core::fmt::Debug for OsStr {
    /// Renders the lossy [`core::fmt::Display`] form inside double quotes, so a platform string
    /// reads like the `str` it usually is.
    ///
    /// Hand-written rather than derived because the derive would print the byte slice underneath as
    /// a list of integers, which is unreadable for what is almost always text.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::OsStr;
    /// assert_eq!(
    ///     alloc::format!("{:?}", OsStr::new("save.dat")),
    ///     "\"save.dat\""
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Write::write_char(f, '"')?;
        core::fmt::Display::fmt(self, f)?;
        core::fmt::Write::write_char(f, '"')
    }
}

impl AsRef<OsStr> for OsStr {
    fn as_ref(&self) -> &OsStr {
        self
    }
}

impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes(self.as_bytes())
    }
}

impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes(self.as_bytes())
    }
}

impl ToOwned for OsStr {
    type Owned = OsString;

    fn to_owned(&self) -> Self::Owned {
        self.to_os_string()
    }
}

impl PartialEq<str> for OsStr {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<OsStr> for str {
    fn eq(&self, other: &OsStr) -> bool {
        other == self
    }
}

/// An owned platform string.
///
/// The owned counterpart of [`OsStr`], and the type a path is built into before it is handed back
/// out. Derefs to [`OsStr`], so every borrowed operation is reachable on one of these.
///
/// ## Formatting
///
/// [`Display`](#impl-Display-for-OsString) and [`Debug`](#impl-Debug-for-OsString) both defer to the
/// borrowed form, so an owned string renders exactly as the [`OsStr`] inside it does.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OsString {
    inner: Vec<u8>,
}

impl OsString {
    /// Creates an empty platform string.
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Takes ownership of `bytes` as a platform string.
    pub const fn from_vec(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    /// Returns the bytes, consuming the string.
    pub fn into_vec(self) -> Vec<u8> {
        self.inner
    }

    /// Borrows the string.
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes(&self.inner)
    }

    /// Appends `s`.
    pub fn push<S: AsRef<OsStr>>(&mut self, s: S) {
        self.inner.extend_from_slice(s.as_ref().as_bytes());
    }

    /// Empties the string, keeping the allocation for what is written next.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl core::fmt::Display for OsString {
    /// Renders the borrowed form, so an owned string prints as the [`OsStr`] inside it does.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::OsString;
    /// assert_eq!(alloc::format!("{}", OsString::from("save.dat")), "save.dat");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_os_str(), f)
    }
}

impl core::fmt::Debug for OsString {
    /// Renders the borrowed form, so an owned string debug-prints as the [`OsStr`] inside it does.
    ///
    /// Hand-written rather than derived for the same reason [`OsStr`]'s is: the derive would print
    /// the bytes as a list of integers.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::OsString;
    /// assert_eq!(
    ///     alloc::format!("{:?}", OsString::from("save.dat")),
    ///     "\"save.dat\""
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_os_str(), f)
    }
}

impl Deref for OsString {
    type Target = OsStr;

    fn deref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl Borrow<OsStr> for OsString {
    fn borrow(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl From<&OsStr> for OsString {
    fn from(value: &OsStr) -> Self {
        value.to_os_string()
    }
}

impl From<&str> for OsString {
    fn from(value: &str) -> Self {
        Self::from_vec(value.as_bytes().to_vec())
    }
}

impl From<String> for OsString {
    fn from(value: String) -> Self {
        Self::from_vec(value.into_bytes())
    }
}

impl From<Vec<u8>> for OsString {
    fn from(value: Vec<u8>) -> Self {
        Self::from_vec(value)
    }
}

impl PartialEq<str> for OsString {
    fn eq(&self, other: &str) -> bool {
        self.as_os_str() == other
    }
}

impl PartialEq<OsString> for str {
    fn eq(&self, other: &OsString) -> bool {
        other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_with_bytes_that_are_not_utf8_keeps_them_unchanged() {
        //* Given
        // A byte that no UTF-8 sequence can start with, which is what makes this the interesting
        // case: an encoding-checked type could not carry it.
        let bytes: &[u8] = b"save\xffdat";

        //* When
        let os_str = OsStr::from_bytes(bytes);

        //* Then
        assert_eq!(
            os_str.as_bytes(),
            bytes,
            "the bytes must survive the borrow untouched"
        );
    }

    #[test]
    fn to_str_with_bytes_that_are_not_utf8_returns_none() {
        //* Given
        let os_str = OsStr::from_bytes(b"save\xffdat");

        //* When
        let text = os_str.to_str();

        //* Then
        assert!(
            text.is_none(),
            "bytes that are not UTF-8 must not be reported as text"
        );
    }

    #[test]
    fn display_with_bytes_that_are_not_utf8_substitutes_the_replacement_character() {
        //* Given
        let os_str = OsStr::from_bytes(b"save\xffdat");

        //* When
        let rendered = alloc::format!("{os_str}");

        //* Then
        assert_eq!(
            rendered, "save\u{fffd}dat",
            "each byte sequence that is not UTF-8 must render as one replacement character"
        );
    }

    #[test]
    fn display_with_a_truncated_sequence_ends_after_one_replacement_character() {
        //* Given
        // A leading byte that promises two more and gets none, which is the case that reports no
        // error length because the input ran out rather than holding a bad byte.
        let os_str = OsStr::from_bytes(b"save\xe2\x82");

        //* When
        let rendered = alloc::format!("{os_str}");

        //* Then
        assert_eq!(
            rendered, "save\u{fffd}",
            "a sequence cut off by the end of the string must render as one replacement character"
        );
    }

    #[test]
    fn debug_with_any_bytes_quotes_the_rendering() {
        //* Given
        let os_str = OsStr::new("save.dat");

        //* When
        let rendered = alloc::format!("{os_str:?}");

        //* Then
        assert_eq!(
            rendered, "\"save.dat\"",
            "a platform string must debug-print quoted"
        );
    }

    #[test]
    fn to_string_lossy_with_bytes_that_are_text_borrows_them() {
        //* Given
        let os_str = OsStr::new("save.dat");

        //* When
        let lossy = os_str.to_string_lossy();

        //* Then
        assert!(
            matches!(lossy, Cow::Borrowed(_)),
            "bytes that are already UTF-8 must not be copied to be read as text"
        );
    }

    #[test]
    fn push_with_a_second_string_appends_without_a_separator() {
        //* Given
        let mut owned = OsString::from("sdmc:");

        //* When
        owned.push("/save.dat");

        //* Then
        assert_eq!(
            owned.as_bytes(),
            b"sdmc:/save.dat",
            "push must join the bytes and add nothing of its own"
        );
    }
}
