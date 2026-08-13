//! The command line, in the two forms `std::env` offers it.
//!
//! `nx-sys-args` holds arguments as the bytes the loader delivered. This module
//! is the conversion to the vocabulary a caller expects, and nothing more: the
//! store, the scan and the lifetime all belong to the layer underneath.

use alloc::string::String;

use nx_std_path::OsString;

/// Returns the arguments this program was started with.
///
/// The first argument is traditionally the path the program was loaded from,
/// but a loader may set it to anything at all, so it is not a fact to rely on.
///
/// # Panics
///
/// Panics when an argument is not valid UTF-8, as `std::env::args` does. A
/// caller that must tolerate such an argument uses [`args_os`], which is the
/// form the loader actually delivered.
pub fn args() -> Args {
    Args {
        inner: nx_sys_args::args(),
    }
}

/// Returns the arguments this program was started with, as the byte strings the
/// loader delivered.
///
/// The first argument is traditionally the path the program was loaded from,
/// but a loader may set it to anything at all, so it is not a fact to rely on.
///
/// Unlike [`args`], this checks nothing: an argument the encoding rules do not
/// describe arrives intact.
pub fn args_os() -> ArgsOs {
    ArgsOs {
        inner: nx_sys_args::args(),
    }
}

/// Iterator over the command-line arguments, yielding a [`String`] each.
///
/// Created by [`args`].
pub struct Args {
    inner: nx_sys_args::Args,
}

impl Iterator for Args {
    type Item = String;

    /// # Panics
    ///
    /// Panics when the argument is not valid UTF-8.
    fn next(&mut self) -> Option<String> {
        let arg = self.inner.next()?;

        Some(
            core::str::from_utf8(arg)
                .expect("command-line argument is not valid UTF-8; use args_os to read it")
                .into(),
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Args {}

/// Iterator over the command-line arguments, yielding an [`OsString`] each.
///
/// Created by [`args_os`].
pub struct ArgsOs {
    inner: nx_sys_args::Args,
}

impl Iterator for ArgsOs {
    type Item = OsString;

    fn next(&mut self) -> Option<OsString> {
        let arg = self.inner.next()?;

        Some(OsString::from(arg.to_vec()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for ArgsOs {}
