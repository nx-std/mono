//! System-level random number generation for the Nintendo Switch.
//!
//! This module provides a thread-safe random number generator that uses the ChaCha20
//! algorithm seeded with entropy from the system's True Random Number Generator (TRNG).
//! The implementation ensures that:
//!
//! - The RNG is initialized only once, using system entropy
//! - All operations are thread-safe through atomic operations
//! - The underlying ChaCha20 algorithm provides cryptographically secure random numbers
//! - The system's TRNG is used as the entropy source for seeding
//!
//! # Implementation Details
//!
//! The RNG is initialized lazily on first use. During initialization, it:
//! 1. Collects 256 bits (4 × 64 bits) of entropy from the system TRNG
//! 2. Uses this entropy to seed a ChaCha20 RNG
//! 3. Stores the RNG in a static variable for subsequent use
//!
//! The initialization process is protected by a state machine that ensures:
//! - Only one thread can perform initialization
//! - Other threads will wait for initialization to complete
//! - The RNG is never used before it's fully initialized
//!
//! All random number generation operations are performed using this seeded RNG,
//! ensuring consistent and secure random number generation across the application.

// See: https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-references.html#safe-references
// TODO: Review the safety of having a global mutable reference to a static variable
#![expect(
    static_mut_refs,
    reason = "the global RNG is a `static mut` reached through `assume_init_mut`; replacing it is tracked by the TODO above"
)]

use core::{
    mem::MaybeUninit,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};

use rand::{
    RngCore,
    SeedableRng,
};
use rand_chacha::ChaCha20Rng;

/// Global RNG instance, initialized on first use.
///
/// Uses [`MaybeUninit`] to safely handle initialization
static mut RNG: MaybeUninit<ChaCha20Rng> = MaybeUninit::uninit();

/// Atomic state for the RNG initialization
static RNG_STATE: AtomicRngState = AtomicRngState::new();

/// Fills a buffer with random data.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
pub fn fill_bytes(slice: &mut [u8]) {
    get_rng().fill_bytes(slice);
}

/// Returns a random 64-bit value.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
pub fn next_u64() -> u64 {
    get_rng().next_u64()
}

/// Returns a reference to the global RNG instance, initializing it if necessary.
///
/// This function ensures that the RNG is initialized only once, even in the presence
/// of multiple threads. The initialization is performed using entropy from the
/// system's TRNG.
///
/// # Implementation Details
///
/// The function uses a state machine to handle initialization:
/// 1. If the RNG is uninitialized, it attempts to claim initialization
/// 2. If another thread is initializing, it waits using a spin loop
/// 3. Once initialized, it returns a reference to the RNG
fn get_rng() -> &'static mut ChaCha20Rng {
    loop {
        match RNG_STATE.load_acquire() {
            RngState::Uninitialized => {
                if RNG_STATE.try_claim_initialization().is_err() {
                    continue;
                }

                // We've claimed initialization, so initialize the RNG
                init_rng();
                RNG_STATE.mark_as_initialized();

                break;
            }
            RngState::Initializing => {
                // Someone else is initializing, wait
                core::hint::spin_loop();
            }
            RngState::Initialized => {
                // Already initialized
                break;
            }
        }
    }

    unsafe { RNG.assume_init_mut() }
}

/// Initializes the global RNG with entropy from the system TRNG.
///
/// This function:
/// 1. Collects 256 bits of entropy from the system TRNG
/// 2. Uses this entropy to seed a ChaCha20 RNG
/// 3. Stores the RNG in the global static variable
///
/// # Panics
///
/// This function will panic if it fails to obtain entropy from the system TRNG.
fn init_rng() {
    let mut seed = [0u8; 32];
    let (chunks, _) = seed.as_chunks_mut::<{ size_of::<u64>() }>();
    for (index, chunk) in chunks.iter_mut().enumerate() {
        // Get process TRNG seeds from kernel using the new helper
        // The seed splits into four `u64`-sized chunks, so `index` is at most 3.
        match nx_svc::misc::get_random_entropy(index as u64) {
            // Native-endian, matching the in-memory layout the seed previously
            // took on by being reinterpreted from `[u64; 4]`.
            Ok(entropy) => chunk.copy_from_slice(&entropy.to_ne_bytes()),
            Err(err) => panic!("Failed to get random entropy: {}", err),
        }
    }

    unsafe {
        RNG.write(ChaCha20Rng::from_seed(seed));
    }
}

/// The initialization state of the RNG
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RngState {
    /// RNG has not been initialized yet
    Uninitialized = 0,
    /// RNG is currently being initialized
    Initializing = 1,
    /// RNG has been initialized and is ready to use
    Initialized = 2,
}

impl TryFrom<u8> for RngState {
    type Error = UnknownRngStateError;

    /// Decodes the byte held in the atomic cell.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownRngStateError`] if the byte is not a state discriminant.
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::Initializing),
            2 => Ok(Self::Initialized),
            unknown => Err(UnknownRngStateError(unknown)),
        }
    }
}

/// An error indicating that a byte names no RNG initialization state.
#[derive(Debug, thiserror::Error)]
#[error("Unknown RNG state {0}")]
struct UnknownRngStateError(u8);

/// A thread-safe wrapper around [`RngState`]
#[derive(Debug)]
struct AtomicRngState(AtomicU8);

impl AtomicRngState {
    /// Creates a new [`AtomicRngState`] with the initial state of [`RngState::Uninitialized`]
    const fn new() -> Self {
        Self(AtomicU8::new(RngState::Uninitialized as u8))
    }

    /// Loads the current state with acquire ordering
    fn load_acquire(&self) -> RngState {
        let raw = self.0.load(Ordering::Acquire);
        // The cell is private and every write to it stores an `RngState` discriminant,
        // so the decode cannot fail. Falling back to `Uninitialized` keeps this total
        // instead of panicking on an unreachable branch, and is the safe choice if it
        // ever were reached: a thread that read it would fail its claim and retry,
        // rather than reach an RNG that has not been written.
        RngState::try_from(raw).unwrap_or(RngState::Uninitialized)
    }

    /// Tries to claim initialization of the RNG.
    ///
    /// This function atomically attempts to transition the RNG state from
    /// [`RngState::Uninitialized`] to [`RngState::Initializing`].
    ///
    /// # Errors
    ///
    /// Returns [`InitializationClaimedError`] if another thread holds the claim, or
    /// if initialization has already finished.
    fn try_claim_initialization(&self) -> Result<(), InitializationClaimedError> {
        self.0
            .compare_exchange(
                RngState::Uninitialized as u8,
                RngState::Initializing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            // The state observed by the failed exchange is dropped: the caller re-reads
            // it at the top of its loop, and acting on a value already stale by then is
            // what the loop exists to avoid.
            .map_err(|_| InitializationClaimedError)
    }

    /// Marks the RNG as initialized with release ordering
    fn mark_as_initialized(&self) {
        self.0.store(RngState::Initialized as u8, Ordering::Release);
    }
}

/// An error indicating that another thread already claimed RNG initialization.
#[derive(Debug, thiserror::Error)]
#[error("RNG initialization is already claimed")]
struct InitializationClaimedError;
