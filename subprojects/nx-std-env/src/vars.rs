//! Environment variables, in the forms `std::env` offers them.
//!
//! `nx-sys-env` owns the bindings and the lock that orders access to them. This
//! module is the conversion to the vocabulary a caller expects, and nothing
//! more: it holds no copy of its own, so every lookup reaches the store and a
//! binding made anywhere is visible to the next read.
//!
//! A Switch process starts with an empty environment, so every binding a
//! program reads is one it, or something it linked, put there.

use alloc::string::String;

use nx_std_path::{
    OsStr,
    OsString,
};

/// Returns the value bound to `key` as text.
///
/// # Errors
///
/// Reports the name being unbound, or bound to a value the encoding rules do
/// not describe, which [`var_os`] returns intact.
pub fn var<K: AsRef<OsStr>>(key: K) -> Result<String, VarError> {
    let value = var_os(key).ok_or(VarError::NotPresent)?;

    match value.to_str() {
        Some(value) => Ok(value.into()),
        None => Err(VarError::NotUnicode(value)),
    }
}

/// Returns the value bound to `key`, as the bytes it was bound to.
///
/// Unlike [`var`], this checks nothing: a value the encoding rules do not
/// describe arrives intact.
pub fn var_os<K: AsRef<OsStr>>(key: K) -> Option<OsString> {
    let value = nx_sys_env::get(key.as_ref().as_bytes())?;

    Some(OsString::from(value))
}

/// Returns every binding in the environment, as text pairs.
///
/// # Panics
///
/// Panics when a name or a value is not valid UTF-8, as `std::env::vars` does.
/// A caller that must tolerate one uses [`vars_os`].
pub fn vars() -> Vars {
    Vars { inner: vars_os() }
}

/// Returns every binding in the environment, as the bytes they were bound to.
///
/// The environment is copied while `nx-sys-env` holds its lock, so the iterator
/// reflects the bindings as they were when it was created rather than tracking
/// later changes.
pub fn vars_os() -> VarsOs {
    VarsOs {
        inner: nx_sys_env::vars(),
    }
}

/// Binds `key` to `value`, replacing any binding it already had.
///
/// # Errors
///
/// Reports the name or the value being unrepresentable: a name that is empty or
/// holds `=` or a nul byte, or a value that holds a nul byte.
///
/// `std::env::set_var` is an `unsafe fn` that returns nothing and panics on
/// these. It is `unsafe` there because the C library keeps its environment in an
/// unlocked global that an assignment can free under a concurrent reader; the
/// environment here is behind a lock and reachable no other way, so the hazard
/// is absent and the failure is worth returning rather than aborting on.
pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(
    key: K,
    value: V,
) -> Result<(), nx_sys_env::SetError> {
    nx_sys_env::set(key.as_ref().as_bytes(), value.as_ref().as_bytes())
}

/// Removes whatever `key` was bound to, reporting whether anything was.
///
/// Removing a name that was never bound is not an error, as it is not for
/// `std::env::remove_var`, which differs only in reporting nothing back.
pub fn remove_var<K: AsRef<OsStr>>(key: K) -> bool {
    nx_sys_env::unset(key.as_ref().as_bytes())
}

/// Why a variable could not be read as text.
#[derive(Debug, thiserror::Error)]
pub enum VarError {
    /// Nothing is bound to the name.
    #[error("environment variable is not present")]
    NotPresent,
    /// Something is bound to it, but not valid UTF-8.
    #[error("environment variable is not valid UTF-8")]
    NotUnicode(OsString),
}

/// Iterator over the environment's bindings, yielding text pairs.
///
/// Created by [`vars`].
pub struct Vars {
    inner: VarsOs,
}

impl Iterator for Vars {
    type Item = (String, String);

    /// # Panics
    ///
    /// Panics when the name or the value is not valid UTF-8.
    fn next(&mut self) -> Option<(String, String)> {
        let (key, value) = self.inner.next()?;

        Some((
            key.to_str()
                .expect("environment variable name is not valid UTF-8; use vars_os to read it")
                .into(),
            value
                .to_str()
                .expect("environment variable value is not valid UTF-8; use vars_os to read it")
                .into(),
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Vars {}

/// Iterator over the environment's bindings, yielding byte-string pairs.
///
/// Created by [`vars_os`].
pub struct VarsOs {
    inner: nx_sys_env::Vars,
}

impl Iterator for VarsOs {
    type Item = (OsString, OsString);

    fn next(&mut self) -> Option<(OsString, OsString)> {
        let (key, value) = self.inner.next()?;

        Some((OsString::from(key), OsString::from(value)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for VarsOs {}
