//! # nx-sys-env
//!
//! The process's environment variables.
//!
//! This crate is the workspace's stand-in for `std::sys::env`.
//!
//! ## The environment is this crate's, and it starts empty
//!
//! A Switch process is handed a command line and nothing else. There is no
//! block to inherit, no parent to inherit it from, and no loader key that
//! carries one, so a process begins with an empty environment and every
//! binding in it is one the process put there.
//!
//! That makes the environment something this workspace owns outright rather
//! than something it reads out of a C library, and it is why the store here is
//! a Rust one. The C library ships an environment of its own, and building on
//! it would mean adopting a second copy of state this crate is meant to
//! provide; the direction is the reverse, as it is everywhere else here.
//!
//! ## Why nothing in this crate is `unsafe`
//!
//! `std::env::set_var` is an `unsafe fn`, and the reason is not the assignment:
//! it is that the C library keeps its environment in a bare global with no lock
//! on it, so an assignment can free the block another thread is walking, and no
//! caller of the C API can be stopped from doing so.
//!
//! Here the environment is behind an [`RwLock`] and reachable no other way.
//! Readers share it, a writer excludes them, and there is no second route in.
//! The hazard that makes the `std` call `unsafe` does not exist, so [`set`] and
//! [`unset`] are ordinary safe functions.
//!
//! ## Bytes, not text
//!
//! A name and a value are byte strings. Nothing here validates UTF-8, for the
//! reason nothing does at this layer: the vocabulary belongs to the crate above.
//!
//! Two byte sequences are still refused, and both because of the boundary this
//! store is meant to serve. A name may not be empty or hold `=`, which is what
//! separates a name from a value; and neither may hold a nul, which is what
//! terminates one for a C caller.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

extern crate alloc;

// Proves a global allocator exists for the owned types below; the umbrella still owns the
// singular `#[global_allocator]` registration at link time.
extern crate nx_alloc as _;

use alloc::vec::Vec;

use nx_sys_sync::data::RwLock;

/// The process's bindings, in the order they were first bound.
///
/// The only route to the environment, which is what lets every function here
/// stay safe: nothing can be reading it except through this lock.
static ENVIRONMENT: RwLock<Vec<(Vec<u8>, Vec<u8>)>> = RwLock::new(Vec::new());

/// Returns the value bound to `key`, or `None` when it is unbound.
///
/// The value is copied out, because the binding it came from may be replaced or
/// removed as soon as the lock is released.
pub fn get(key: &[u8]) -> Option<Vec<u8>> {
    let environment = ENVIRONMENT.read();

    let (_, value) = environment.iter().find(|(name, _)| name == key)?;

    Some(value.clone())
}

/// Returns every binding, as `(key, value)` pairs.
///
/// The bindings are copied out while the lock is held, so the iterator reflects
/// the environment as it was at that moment rather than tracking later changes.
pub fn vars() -> Vars {
    let environment = ENVIRONMENT.read();

    Vars {
        bindings: environment.clone().into_iter(),
    }
}

/// Binds `key` to `value`, replacing any binding it already had.
///
/// # Errors
///
/// Reports the name or the value being unrepresentable: a name that is empty or
/// holds `=` or a nul byte, or a value that holds a nul byte.
pub fn set(key: &[u8], value: &[u8]) -> Result<(), SetError> {
    if key.is_empty() || key.contains(&b'=') || key.contains(&0) {
        return Err(SetError::InvalidName);
    }
    if value.contains(&0) {
        return Err(SetError::ValueHoldsNul);
    }

    let mut environment = ENVIRONMENT.write();

    match environment.iter_mut().find(|(name, _)| name == key) {
        Some((_, bound)) => *bound = value.to_vec(),
        None => environment.push((key.to_vec(), value.to_vec())),
    }

    Ok(())
}

/// Removes whatever `key` was bound to, reporting whether anything was.
///
/// Removing a name that was never bound is not an error, as it is not for
/// `std::env::remove_var`.
pub fn unset(key: &[u8]) -> bool {
    let mut environment = ENVIRONMENT.write();

    let Some(bound) = environment.iter().position(|(name, _)| name == key) else {
        return false;
    };
    environment.remove(bound);

    true
}

/// Why an assignment could not be made.
#[derive(Debug, thiserror::Error)]
pub enum SetError {
    /// The name is empty, or holds `=` or a nul byte.
    #[error("environment variable name is empty or holds `=` or a nul byte")]
    InvalidName,
    /// The value holds a nul byte, which would truncate it for a C caller.
    #[error("environment variable value holds a nul byte")]
    ValueHoldsNul,
}

/// Iterator over the environment's `(key, value)` pairs.
///
/// Created by [`vars`], over a copy taken while the lock was held.
pub struct Vars {
    bindings: alloc::vec::IntoIter<(Vec<u8>, Vec<u8>)>,
}

impl Iterator for Vars {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<(Vec<u8>, Vec<u8>)> {
        self.bindings.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.bindings.size_hint()
    }
}

impl ExactSizeIterator for Vars {}
