//! The `getrandom` platform backend.
//!
//! `getrandom` resolves entropy on an unknown target through a symbol the platform is expected to
//! define, and this is Horizon's. Defining it is what makes `rand::rngs::SysRng` work, and with it
//! everything `rand` builds on top: `StdRng`, `SeedableRng::from_os_rng`, and any crate that seeds
//! a generator without naming this one.
//!
//! The symbol is what a caller reaches; nothing here is a Rust API.

use core::slice;

use crate::entropy;

/// The [`getrandom::Error`] code standing for a kernel that refused to report its entropy.
///
/// `getrandom` reserves a range for platform-defined codes, which is what tells a caller the
/// failure came from here rather than from one of its own backends.
const SEED_UNAVAILABLE: u16 = 1;

/// Fills the first `len` bytes at `dest` with random data.
///
/// This is the symbol `getrandom` declares and calls; it is not meant to be called directly.
///
/// # Safety
///
/// `dest` must point to a writable region of at least `len` bytes that stays valid for the
/// duration of the call.
///
/// # Errors
///
/// Returns [`SEED_UNAVAILABLE`] as a custom code on the process's first draw if the kernel refuses
/// to report the entropy the generator is seeded from.
#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom::Error> {
    // SAFETY: the caller guarantees `dest` addresses `len` writable bytes for the call.
    let dst = unsafe { slice::from_raw_parts_mut(dest, len) };

    // `getrandom::Error` is a code and nothing else, so the word the kernel refused cannot travel
    // with it; it is dropped here rather than at a boundary that has no room for it either.
    entropy::fill(dst).map_err(|_| getrandom::Error::new_custom(SEED_UNAVAILABLE))
}
