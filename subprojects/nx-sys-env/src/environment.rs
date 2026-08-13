//! The process's environment, and the C-shaped view of it.
//!
//! The two live in one module because they are one piece of state. Every
//! mutation rebuilds the view before releasing the lock, so a C caller reading
//! [`__nx_sys_env__newlib_environ`] directly never sees an array that disagrees with the bindings it
//! was built from. Splitting them would put the rebuild on the far side of a
//! module boundary from the write it has to follow.

#[cfg(feature = "ffi")]
use alloc::ffi::CString;
use alloc::vec::Vec;
#[cfg(feature = "ffi")]
use core::{
    ffi::c_char,
    ptr,
};

use nx_sys_sync::data::RwLock;

/// The process's bindings, in the order they were first bound.
///
/// The only route to the environment, which is what lets every function here
/// stay safe: nothing can be reading it except through this lock.
static ENVIRONMENT: RwLock<Environment> = RwLock::new(Environment::new());

/// The C-facing `environ`, a NULL-terminated array of `KEY=VALUE` strings.
///
/// Exported from this module rather than from `ffi`, because it is a view of
/// the bindings above and is rebuilt by the same write that changes them; a
/// module that held the symbol without holding the store would have to reach
/// back into this one, which the module graph does not allow.
///
/// Repointed whenever the environment changes, so a caller that cached the
/// pointer holds a stale array. That is the POSIX contract: an assignment
/// invalidates what a previous read returned.
#[cfg(feature = "ffi")]
#[unsafe(no_mangle)]
pub(crate) static mut __nx_sys_env__newlib_environ: *mut *mut c_char = EMPTY_ENVIRON.as_ptr();

/// The array `environ` names before anything is bound.
#[cfg(feature = "ffi")]
static EMPTY_ENVIRON: EmptyEnviron = EmptyEnviron([ptr::null_mut()]);

/// Returns the value bound to `key`, or `None` when it is unbound.
///
/// The value is copied out, because the binding it came from may be replaced or
/// removed as soon as the lock is released.
pub fn get(key: &[u8]) -> Option<Vec<u8>> {
    let environment = ENVIRONMENT.read();

    let (_, value) = environment.bindings.iter().find(|(name, _)| name == key)?;

    Some(value.clone())
}

/// Returns every binding, as `(key, value)` pairs.
///
/// The bindings are copied out while the lock is held, so the iterator reflects
/// the environment as it was at that moment rather than tracking later changes.
pub fn vars() -> Vars {
    let environment = ENVIRONMENT.read();

    Vars {
        bindings: environment.bindings.clone().into_iter(),
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

    match environment
        .bindings
        .iter_mut()
        .find(|(name, _)| name == key)
    {
        Some((_, bound)) => *bound = value.to_vec(),
        None => environment.bindings.push((key.to_vec(), value.to_vec())),
    }
    environment.republish();

    Ok(())
}

/// Removes whatever `key` was bound to, reporting whether anything was.
///
/// Removing a name that was never bound is not an error, as it is not for
/// `std::env::remove_var`.
pub fn unset(key: &[u8]) -> bool {
    let mut environment = ENVIRONMENT.write();

    let Some(bound) = environment
        .bindings
        .iter()
        .position(|(name, _)| name == key)
    else {
        return false;
    };
    environment.bindings.remove(bound);
    environment.republish();

    true
}

/// Returns a pointer to the value `key` is bound to, inside the C view, or
/// NULL when it is unbound.
///
/// The pointer addresses the `KEY=VALUE` entry the view owns, past its `=`,
/// which is the same thing the C library hands out and carries the same
/// contract: the next assignment may invalidate it.
#[cfg(feature = "ffi")]
pub(crate) fn c_value_ptr(key: &[u8]) -> *mut c_char {
    let environment = ENVIRONMENT.read();

    let Some(bound) = environment
        .bindings
        .iter()
        .position(|(name, _)| name == key)
    else {
        return ptr::null_mut();
    };

    // The view is rebuilt from the bindings by every mutation, so entry `bound`
    // is the same binding, and its value starts one byte past the name.
    let entry = environment.entries[bound].as_ptr().cast_mut();

    // SAFETY: the entry is `KEY=VALUE`, so `key.len() + 1` bytes in is the
    // start of the value and still inside the same nul-terminated string.
    unsafe { entry.add(key.len() + 1) }
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

/// The bindings, and the C-shaped view rebuilt from them.
struct Environment {
    /// The bindings themselves. This is the storage; the view is derived.
    bindings: Vec<(Vec<u8>, Vec<u8>)>,
    /// `KEY=VALUE` copies, owning what the pointer array addresses.
    #[cfg(feature = "ffi")]
    entries: Vec<CString>,
    /// Pointers into `entries` plus a NULL terminator: the array `environ`
    /// names.
    #[cfg(feature = "ffi")]
    pointers: CPointers,
}

impl Environment {
    /// The environment of a process that has bound nothing.
    const fn new() -> Self {
        Self {
            bindings: Vec::new(),
            #[cfg(feature = "ffi")]
            entries: Vec::new(),
            #[cfg(feature = "ffi")]
            pointers: CPointers(Vec::new()),
        }
    }

    /// Rebuilds the C view from the bindings and repoints `environ` at it.
    ///
    /// Called by every mutation before it releases the lock, which is what
    /// keeps the array and the bindings from disagreeing.
    #[cfg(not(feature = "ffi"))]
    fn republish(&mut self) {}

    /// Rebuilds the C view from the bindings and repoints `environ` at it.
    ///
    /// Called by every mutation before it releases the lock, which is what
    /// keeps the array and the bindings from disagreeing.
    #[cfg(feature = "ffi")]
    fn republish(&mut self) {
        self.entries = self
            .bindings
            .iter()
            .map(|(key, value)| {
                let mut entry = key.clone();
                entry.push(b'=');
                entry.extend_from_slice(value);

                // SAFETY-adjacent: `set` rejects a nul in either half, so the
                // joined entry holds none and the one failure cannot arise.
                CString::new(entry).expect("set rejects a nul in a name or a value")
            })
            .collect();

        self.pointers = CPointers(
            self.entries
                .iter()
                .map(|entry| entry.as_ptr().cast_mut())
                .collect(),
        );
        self.pointers.0.push(ptr::null_mut());

        let published = self.pointers.0.as_mut_ptr();

        // SAFETY: the write lock is held, so no reader of this crate is running,
        // and a C caller is required to re-read `environ` after any assignment
        // rather than hold one across it.
        unsafe { __nx_sys_env__newlib_environ = published };
    }
}

/// The pointer array `environ` names, as a type the lock can hold.
///
/// A `Vec` of raw pointers is neither `Send` nor `Sync`, which would keep the
/// whole environment out of a `static`. The claim is sound here for a reason
/// the bare `Vec` cannot express: these pointers address the `entries` beside
/// them in the same struct, so the lock that guards the struct guards what they
/// point at too, and nothing hands one out except through that lock.
#[cfg(feature = "ffi")]
struct CPointers(Vec<*mut c_char>);

// SAFETY: the pointers address `CString`s owned by the same `Environment`, and
// every route to them is behind the lock that guards it. Moving or sharing the
// struct moves or shares what they point at with them.
#[cfg(feature = "ffi")]
unsafe impl Send for CPointers {}

// SAFETY: as above.
#[cfg(feature = "ffi")]
unsafe impl Sync for CPointers {}

/// `Sync` wrapper for the array `environ` names before anything is bound.
#[cfg(feature = "ffi")]
struct EmptyEnviron([*mut c_char; 1]);

// SAFETY: the array is immutable and holds only a null pointer; sharing it
// across threads cannot observe a data race.
#[cfg(feature = "ffi")]
unsafe impl Sync for EmptyEnviron {}

#[cfg(feature = "ffi")]
impl EmptyEnviron {
    /// Pointer to the null-terminated empty array.
    const fn as_ptr(&self) -> *mut *mut c_char {
        self.0.as_ptr().cast_mut()
    }
}
