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
#![allow(static_mut_refs)]

use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use rand::{RngCore, SeedableRng};
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
///
/// # Arguments
///
/// * `slice` - The buffer to fill with random data
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
    let mut seed = [0u64; 4];
    for (i, item) in seed.iter_mut().enumerate() {
        // Get process TRNG seeds from kernel using the new helper
        match nx_svc::misc::get_random_entropy(i as u64) {
            Ok(val) => *item = val,
            Err(err) => panic!("Failed to get random entropy: {}", err),
        }
    }

    unsafe {
        RNG.write(ChaCha20Rng::from_seed(core::mem::transmute::<
            [u64; 4],
            [u8; 32],
        >(seed)));
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
        match self.0.load(Ordering::Acquire) {
            0 => RngState::Uninitialized,
            1 => RngState::Initializing,
            2 => RngState::Initialized,
            _ => unreachable!(),
        }
    }

    /// Tries to claim initialization of the RNG.
    ///
    /// This function atomically attempts to transition the RNG state from
    /// [`RngState::Uninitialized`] to [`RngState::Initializing`].
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition was successful
    /// * `Err(())` if another thread has already claimed initialization
    fn try_claim_initialization(&self) -> Result<(), ()> {
        self.0
            .compare_exchange(
                RngState::Uninitialized as u8,
                RngState::Initializing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_or_else(|_| Err(()), |_| Ok(()))
    }

    /// Marks the RNG as initialized with release ordering
    fn mark_as_initialized(&self) {
        self.0.store(RngState::Initialized as u8, Ordering::Release);
    }
}
