//! The process entropy source every random byte in the program comes from.
//!
//! The kernel gives a process 256 bits of entropy and no way to ask for more, so this module is
//! what turns that fixed seed into a stream: a ChaCha20 generator, seeded on first use, behind a
//! lock. Both the callers above it draw from here, and drawing anywhere else in the process would
//! mean a second generator seeded from the same words.

use core::cell::UnsafeCell;

// `rand_core` is reached through `chacha20` rather than named as a dependency of its own: the
// traits below are the ones `ChaCha20Rng`'s impls are written against, and taking them from the
// crate that defines it is what keeps the two versions from drifting apart.
use chacha20::{
    ChaCha20Rng,
    rand_core::{
        Rng as _,
        SeedableRng as _,
    },
};
use nx_sys_sync::Mutex;

/// Fills `dst` with random bytes.
///
/// # Errors
///
/// Returns [`SeedError`] on the process's first draw if the kernel refuses to report the entropy
/// the generator is seeded from.
pub fn fill(dst: &mut [u8]) -> Result<(), SeedError> {
    let mut guard = SHARED.lock();
    guard.rng()?.fill_bytes(dst);
    Ok(())
}

/// Returns a random `u64`.
///
/// # Errors
///
/// Returns [`SeedError`] on the process's first draw if the kernel refuses to report the entropy
/// the generator is seeded from.
pub fn next_u64() -> Result<u64, SeedError> {
    let mut guard = SHARED.lock();
    Ok(guard.rng()?.next_u64())
}

/// The kernel refused to report the entropy the generator is seeded from.
#[derive(Debug, thiserror::Error)]
#[error("Kernel entropy word {index} is unavailable")]
pub struct SeedError {
    index: u64,
    #[source]
    source: nx_svc::misc::GetInfoError,
}

/// The one generator the process shares, with the lock that orders access to it.
static SHARED: SharedRng = SharedRng::new();

/// A ChaCha20 generator behind a lock, seeded from kernel entropy on first use.
struct SharedRng {
    lock: Mutex,
    rng: UnsafeCell<Option<ChaCha20Rng>>,
}

// SAFETY: `rng` is private to this module and reached only through a `SharedRngGuard`, which exists
// only while `lock` is held. At most one `&mut ChaCha20Rng` derived from the cell is live at a
// time, and never on two threads at once.
unsafe impl Sync for SharedRng {}

impl SharedRng {
    const fn new() -> Self {
        Self {
            lock: Mutex::new(),
            rng: UnsafeCell::new(None),
        }
    }

    /// Takes the lock, blocking until it is free.
    fn lock(&self) -> SharedRngGuard<'_> {
        self.lock.lock();
        SharedRngGuard(self)
    }
}

/// Exclusive access to [`SharedRng`], releasing the lock when dropped.
struct SharedRngGuard<'a>(&'a SharedRng);

impl SharedRngGuard<'_> {
    /// Returns the generator, seeding it if this is the process's first draw.
    ///
    /// # Errors
    ///
    /// Returns [`SeedError`] if the kernel refuses to report its entropy. The lock is released
    /// either way, so a failed draw leaves the generator reachable for the next one.
    fn rng(&mut self) -> Result<&mut ChaCha20Rng, SeedError> {
        // SAFETY: this guard holds the lock for as long as it lives, and the cell is named nowhere
        // else, so this is the only live borrow of the generator.
        let slot = unsafe { &mut *self.0.rng.get() };

        match slot {
            Some(rng) => Ok(rng),
            // Binding the whole slot rather than matching `None` is what lets the seed be written
            // through it here; matching the variant would end the borrow the `Some` arm returns.
            empty => Ok(empty.insert(seed_from_process_entropy()?)),
        }
    }
}

impl Drop for SharedRngGuard<'_> {
    fn drop(&mut self) {
        self.0.lock.unlock();
    }
}

/// Seeds a ChaCha20 generator from the kernel's per-process entropy.
///
/// # Errors
///
/// Returns [`SeedError`] naming the word the kernel refused to report.
fn seed_from_process_entropy() -> Result<ChaCha20Rng, SeedError> {
    let mut seed = [0u8; 32];
    let (chunks, _) = seed.as_chunks_mut::<{ size_of::<u64>() }>();
    for (index, chunk) in chunks.iter_mut().enumerate() {
        // The seed splits into four `u64`-sized chunks, so `index` is at most 3.
        let index = index as u64;
        let entropy = nx_svc::misc::get_random_entropy(index)
            .map_err(|source| SeedError { index, source })?;
        // Native-endian, matching the in-memory layout the seed takes on when ChaCha20 reads it
        // back as `[u8; 32]`.
        chunk.copy_from_slice(&entropy.to_ne_bytes());
    }

    Ok(ChaCha20Rng::from_seed(seed))
}
