//! # Heap-free data-protecting synchronization
//!
//! Generic lock and cell wrappers that bundle an allocation-free `nx-sys-sync`
//! primitive with the data it guards, handing out scoped access through RAII
//! guards:
//!
//! * [`Mutex<T>`] — mutual exclusion over `T`.
//! * [`RwLock<T>`] — many-reader / single-writer access to `T`.
//! * [`OnceLock<T>`] — a `T` written exactly once and then read freely.
//!
//! This is the single authoritative home for the data-wrapper knowledge that
//! heap-free crates would otherwise clone. Every type here is `core`-only — it
//! adds no heap and no allocator to a consumer's dependency graph — so crates
//! that must stay heap-free can protect shared state without reaching for
//! `nx-std-sync`.

mod mutex;
mod oncelock;
mod rwlock;

#[doc(inline)]
pub use self::{
    mutex::{Mutex, MutexGuard},
    oncelock::OnceLock,
    rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};
