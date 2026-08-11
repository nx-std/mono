//! Filesystem paths: a platform string read as a location.
//!
//! [`Path`] is the borrowed form and [`PathBuf`] the owned one, layered over [`OsStr`] and
//! [`OsString`] exactly as `std` layers them. A `Path` adds no invariant to the bytes underneath:
//! it is the same bytes, read with the separator rule below applied.
//!
//! ## The separator rule
//!
//! Horizon separates path components with `/` and calls a path starting with one absolute, which is
//! the Unix rule and the one `fsp-srv` enforces on the paths it is sent. There is no drive prefix
//! and no second separator to normalize, so the whole rule is [`MAIN_SEPARATOR`].

use alloc::{
    borrow::{
        Cow,
        ToOwned,
    },
    string::String,
};
use core::{
    borrow::Borrow,
    ops::Deref,
};

use super::os_str::{
    OsStr,
    OsString,
};

/// The byte that separates one path component from the next.
///
/// A byte rather than the `char` `std` declares, because every consumer here is comparing against
/// or writing into a byte string and would otherwise encode it first.
pub const MAIN_SEPARATOR: u8 = b'/';

/// A borrowed filesystem path.
///
/// Unsized, and laid out as the [`OsStr`] it wraps, so borrowing one costs nothing.
///
/// ## Formatting
///
/// [`Debug`](#impl-Debug-for-Path) renders the quoted lossy form. There is deliberately no
/// [`core::fmt::Display`]: a path is not required to be text, so rendering one for a human goes
/// through [`Path::display`], which is where that concession is stated.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Path {
    inner: OsStr,
}

impl Path {
    /// Borrows `s` as a path.
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Path {
        // SAFETY: `Path` is a `repr(transparent)` wrapper around `OsStr`, so the two have the same
        // layout and the same validity requirements.
        unsafe { &*(s.as_ref() as *const OsStr as *const Path) }
    }

    /// Returns the path as a platform string.
    pub fn as_os_str(&self) -> &OsStr {
        &self.inner
    }

    /// Returns the path as text, or `None` when its bytes are not valid UTF-8.
    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    /// Returns the path as text, standing in `U+FFFD` for each byte sequence that is not UTF-8.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    /// Copies the path into an owned one.
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.inner.to_os_string())
    }

    /// Reports whether the path starts at the root.
    pub fn is_absolute(&self) -> bool {
        self.has_root()
    }

    /// Reports whether the path is resolved against something else.
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    /// Reports whether the path begins with the separator.
    ///
    /// The same question as [`Path::is_absolute`] here, because Horizon has no drive prefix for the
    /// two to disagree about. Both are kept so that code reads the way it does against `std`.
    pub fn has_root(&self) -> bool {
        self.inner.as_bytes().first() == Some(&MAIN_SEPARATOR)
    }

    /// Returns this path with `path` appended.
    ///
    /// An absolute `path` replaces this one rather than being joined onto it, which is what `std`
    /// does and what keeps a caller from silently addressing a location under the wrong root.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut joined = self.to_path_buf();
        joined.push(path);
        joined
    }

    /// Returns a value that renders the path lossily.
    ///
    /// A path is not required to be text, so it has no [`core::fmt::Display`] impl of its own: a
    /// caller reporting one has to say here that a lossy rendering is acceptable.
    pub fn display(&self) -> Display<'_> {
        Display { path: self }
    }
}

impl core::fmt::Debug for Path {
    /// Renders the platform string underneath, so a path debug-prints quoted and lossy.
    ///
    /// Hand-written rather than derived because the derive would name the wrapper and print the
    /// bytes as a list of integers.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::Path;
    /// assert_eq!(
    ///     alloc::format!("{:?}", Path::new("/switch/app.nro")),
    ///     "\"/switch/app.nro\""
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.inner, f)
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for OsStr {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for OsString {
    fn as_ref(&self) -> &Path {
        Path::new(self.as_os_str())
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::new(self.as_str())
    }
}

impl AsRef<OsStr> for Path {
    fn as_ref(&self) -> &OsStr {
        &self.inner
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        self.to_path_buf()
    }
}

/// Renders a [`Path`] lossily, as produced by [`Path::display`].
///
/// ## Formatting
///
/// [`Display`](#impl-Display-for-Display<'a>) is the rendering this type exists for;
/// [`Debug`](#impl-Debug-for-Display<'a>) quotes it, matching the path it borrows.
pub struct Display<'a> {
    path: &'a Path,
}

impl core::fmt::Display for Display<'_> {
    /// Renders the path's bytes as text, standing in `U+FFFD` for each sequence that is not UTF-8.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::{OsStr, Path};
    /// let path = Path::new(OsStr::from_bytes(b"/save\xffdat"));
    /// assert_eq!(alloc::format!("{}", path.display()), "/save\u{fffd}dat");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.path.inner, f)
    }
}

impl core::fmt::Debug for Display<'_> {
    /// Renders the quoted lossy form, so a borrowed renderer debug-prints as its path does.
    ///
    /// Hand-written rather than derived because the derive would print the wrapper around the path
    /// rather than the path.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::Path;
    /// assert_eq!(
    ///     alloc::format!("{:?}", Path::new("/save.dat").display()),
    ///     "\"/save.dat\""
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.path.inner, f)
    }
}

/// An owned filesystem path.
///
/// The owned counterpart of [`Path`], and what a path is built into when it is joined onto a
/// working directory or extended a component at a time. Derefs to [`Path`].
///
/// ## Formatting
///
/// [`Debug`](#impl-Debug-for-PathBuf) defers to the borrowed form, and there is no
/// [`core::fmt::Display`], for the reason [`Path`] gives.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathBuf {
    inner: OsString,
}

impl PathBuf {
    /// Creates an empty path.
    pub const fn new() -> Self {
        Self {
            inner: OsString::new(),
        }
    }

    /// Borrows the path.
    pub fn as_path(&self) -> &Path {
        Path::new(self.inner.as_os_str())
    }

    /// Appends `path`, inserting a separator if one is needed.
    ///
    /// An absolute `path` replaces what is held rather than extending it, matching `std`. No
    /// separator is inserted when the path is empty or already ends in one, so joining twice does
    /// not produce a doubled separator.
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();

        if path.is_absolute() {
            self.inner.clear();
        } else if needs_separator(self.inner.as_bytes()) {
            self.inner.push(OsStr::from_bytes(&[MAIN_SEPARATOR]));
        }

        self.inner.push(path.as_os_str());
    }

    /// Returns the path as a platform string, consuming it.
    pub fn into_os_string(self) -> OsString {
        self.inner
    }
}

/// Reports whether appending a component to `current` has to insert a separator first.
///
/// An empty path takes the component as its whole self, and one already ending in a separator has
/// the separator it needs, so only a non-empty path ending in something else does.
fn needs_separator(current: &[u8]) -> bool {
    match current.last() {
        None => false,
        Some(&last) => last != MAIN_SEPARATOR,
    }
}

impl core::fmt::Debug for PathBuf {
    /// Renders the borrowed form, so an owned path debug-prints as the [`Path`] inside it does.
    ///
    /// Hand-written rather than derived for the same reason [`Path`]'s is.
    ///
    /// ```
    /// # extern crate alloc;
    /// # use nx_std_path::PathBuf;
    /// assert_eq!(
    ///     alloc::format!("{:?}", PathBuf::from("/save.dat")),
    ///     "\"/save.dat\""
    /// );
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_path(), f)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.as_path()
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<OsStr> for PathBuf {
    fn as_ref(&self) -> &OsStr {
        self.inner.as_os_str()
    }
}

impl From<OsString> for PathBuf {
    fn from(value: OsString) -> Self {
        Self { inner: value }
    }
}

impl From<&Path> for PathBuf {
    fn from(value: &Path) -> Self {
        value.to_path_buf()
    }
}

impl From<&OsStr> for PathBuf {
    fn from(value: &OsStr) -> Self {
        Self {
            inner: value.to_os_string(),
        }
    }
}

impl From<&str> for PathBuf {
    fn from(value: &str) -> Self {
        Self {
            inner: OsString::from(value),
        }
    }
}

impl From<String> for PathBuf {
    fn from(value: String) -> Self {
        Self {
            inner: OsString::from(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_absolute_with_a_leading_separator_returns_true() {
        //* Given
        let path = Path::new("/switch/app.nro");

        //* When
        let absolute = path.is_absolute();

        //* Then
        assert!(
            absolute,
            "a path beginning with the separator starts at the root"
        );
    }

    #[test]
    fn is_relative_without_a_leading_separator_returns_true() {
        //* Given
        let path = Path::new("save.dat");

        //* When
        let relative = path.is_relative();

        //* Then
        assert!(relative, "a path with nothing to start from is relative");
    }

    #[test]
    fn is_absolute_on_the_empty_path_returns_false() {
        //* Given
        let path = Path::new("");

        //* When
        let absolute = path.is_absolute();

        //* Then
        assert!(
            !absolute,
            "an empty path has no root, so it cannot be absolute"
        );
    }

    #[test]
    fn join_with_a_relative_path_inserts_one_separator() {
        //* Given
        let base = Path::new("/nx-tests-fs");

        //* When
        let joined = base.join("save.dat");

        //* Then
        assert_eq!(
            joined.as_os_str().as_bytes(),
            b"/nx-tests-fs/save.dat",
            "a join must separate the two parts with exactly one separator"
        );
    }

    #[test]
    fn join_onto_a_trailing_separator_does_not_double_it() {
        //* Given
        // The root is the one path a device keeps its trailing separator on.
        let base = Path::new("/");

        //* When
        let joined = base.join("save.dat");

        //* Then
        assert_eq!(
            joined.as_os_str().as_bytes(),
            b"/save.dat",
            "a base already ending in a separator must not gain a second one"
        );
    }

    #[test]
    fn join_with_an_absolute_path_replaces_the_base() {
        //* Given
        let base = Path::new("/nx-tests-fs");

        //* When
        let joined = base.join("/switch/app.nro");

        //* Then
        assert_eq!(
            joined.as_os_str().as_bytes(),
            b"/switch/app.nro",
            "an absolute path names a location on its own and must not be joined onto anything"
        );
    }

    #[test]
    fn join_onto_the_empty_path_adds_no_separator() {
        //* Given
        let base = PathBuf::new();

        //* When
        let joined = base.join("save.dat");

        //* Then
        assert_eq!(
            joined.as_os_str().as_bytes(),
            b"save.dat",
            "an empty path takes the component as its whole self"
        );
    }

    #[test]
    fn as_os_str_with_bytes_that_are_not_utf8_returns_them_unchanged() {
        //* Given
        let path = Path::new(OsStr::from_bytes(b"/save\xffdat"));

        //* When
        let bytes = path.as_os_str().as_bytes();

        //* Then
        assert_eq!(
            bytes, b"/save\xffdat",
            "a path that is not UTF-8 must reach the other side unchanged rather than be refused"
        );
    }

    #[test]
    fn display_with_bytes_that_are_not_utf8_substitutes_the_replacement_character() {
        //* Given
        let path = Path::new(OsStr::from_bytes(b"/save\xffdat"));

        //* When
        let rendered = alloc::format!("{}", path.display());

        //* Then
        assert_eq!(
            rendered, "/save\u{fffd}dat",
            "a path reported to a human renders with replacements rather than refusing"
        );
    }
}
