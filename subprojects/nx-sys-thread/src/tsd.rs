//! Musl libc-style thread-specific data (TSD) slots.
//!
//! This module implements the runtime thread-specific data API that backs
//! `threadTlsAlloc`/`threadTlsGet`/`threadTlsSet`/`threadTlsFree` and the
//! newlib `__syscall_tls_*` family: [`alloc`], [`get`], [`set`], [`free`], and
//! the thread-exit [`run_destructors`] pass.
//!
//! # Runtime TSD, not ELF TLS
//!
//! These slots implement POSIX `pthread_key_create`-style *runtime* storage:
//! keys are allocated dynamically at run time and every thread keeps an
//! independent value per key. This is distinct from ELF TLS (`.tdata`/`.tbss`,
//! the TCB, and the DTV), which is statically laid out by the linker and
//! initialized per thread from the executable image.
//!
//! Following musl libc, destructor metadata is process-global — one entry per
//! key, where a null entry means the key slot is free — while the values are
//! per-thread: each thread owns a flat array of [`NUM_TSD_KEYS`] pointer slots.
//! The per-thread [`ThreadRuntime`] record sits immediately after that array so
//! the core can recover runtime state from a single pointer; `ffi::libnx`
//! mirrors that same pointer into `LibnxThread.tls_array`, so no libnx ABI
//! layout change is required for runtime TLS slots.
//!
//! # Locking
//!
//! [`TSD_KEY_LOCK`] guards the global key table; key deletion ([`free`]) also
//! walks the live-thread registry, which takes its own `THREAD_MUTEX`. When
//! both are needed the order is always [`TSD_KEY_LOCK`] first, then the
//! registry lock.

use core::{
    ffi::c_void,
    ptr::{NonNull, null_mut},
};

#[cfg(feature = "ffi")]
use nx_svc::error::{KernelError, ResultCode, ToRawResultCode};
use nx_sys_sync::RwLock;

use crate::{thread::ThreadControl, thread_list};

/// Number of runtime TSD key slots — musl libc's `PTHREAD_KEYS_MAX` (128).
///
/// These slots are a per-thread heap array owned by `nx-sys-thread` (see the
/// module docs), *not* kernel TLS slots, so nothing ties the cap to the kernel
/// TLS user region. The cap is deliberately musl's `PTHREAD_KEYS_MAX` because
/// the musl key-allocation model is used here; the earlier binding to
/// `nx_sys_thread_tls::NUM_TLS_SLOTS` (27) was an incidental inheritance of an
/// unrelated kernel-slot count. 128 also exceeds libnx's 12-slot `threadTlsAlloc`
/// cap, but the override is strictly more permissive — harmless, and already
/// true at 27.
pub const NUM_TSD_KEYS: usize = 128;

/// Number of times thread exit retries the TSD destructor pass.
///
/// Matches the POSIX/musl `PTHREAD_DESTRUCTOR_ITERATIONS` semantics: a
/// destructor may store a fresh value into its own (or another) key, so the
/// exit path re-scans the slot array up to this many times.
pub const PTHREAD_DESTRUCTOR_ITERATIONS: usize = 4;

/// Destructor invoked on thread exit with a TSD slot's stored value.
///
/// Registered per key when the key is allocated, and run for every non-null
/// slot when the owning thread exits.
pub type Destructor = unsafe extern "C" fn(*mut c_void);

/// A validated runtime TSD key.
///
/// Wraps the slot index of a key, guaranteed to be below [`NUM_TSD_KEYS`]. The
/// C ABI adapters carry a raw `s32`/`u32` slot id;
/// [`from_raw`](TsdKey::from_raw) is the single validation point that turns one
/// into a `TsdKey` at the FFI edge, so the core ([`get`], [`set`], [`free`])
/// never re-checks the range — the type itself carries that invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsdKey(u32);

impl TsdKey {
    /// Validates a raw slot id from a C caller, returning the key when it
    /// names an in-range slot.
    ///
    /// The libnx ABI passes the id as a signed `s32`; a negative value
    /// reinterpreted as `u32` lands far above [`NUM_TSD_KEYS`] and is rejected
    /// here, so this single `u32` entry point validates both the libnx
    /// (`s32`) and newlib (`u32`) raw forms.
    pub fn from_raw(raw: u32) -> Option<Self> {
        ((raw as usize) < NUM_TSD_KEYS).then_some(Self(raw))
    }

    /// Returns the raw slot id for the C ABI.
    pub const fn to_raw(self) -> u32 {
        self.0
    }

    /// Wraps a slot index produced by [`alloc`]'s in-bounds table scan.
    const fn from_index(idx: usize) -> Self {
        Self(idx as u32)
    }

    /// Returns the slot's array index.
    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Per-thread runtime TSD state.
///
/// Stored immediately after a thread's flat TSD slot array in
/// `nx-sys-thread`-owned memory. `tsd` points at that array; `tsd_used` is a
/// fast-path flag set the first time the thread stores a non-null value, so
/// thread exit can skip the destructor scan entirely for threads that never
/// touched TSD.
pub struct ThreadRuntime {
    /// Pointer to this thread's flat array of [`NUM_TSD_KEYS`] value slots.
    pub tsd: *mut *mut c_void,
    /// Whether this thread has ever stored a non-null TSD value.
    pub tsd_used: bool,
}

/// Destructor registered for an allocated runtime TSD key.
///
/// Wrapping the [`Destructor`] in a dedicated type lets the global key table
/// store `Option<DestructorEntry>`, where `None` cleanly means "slot free" and
/// `Some` means "slot allocated". Keys allocated without a user destructor
/// store the [`noop_destructor`] sentinel, so an allocated-but-destructor-less
/// key is still `Some` and is never confused with a free slot.
#[derive(Clone, Copy)]
struct DestructorEntry(Destructor);

/// Process-global lock guarding the runtime TSD key table.
///
/// A thin newtype over [`RwLock`] whose sole purpose is to hand out the RAII
/// [`KeyReadGuard`]/[`KeyWriteGuard`] guards: `nx-sys-sync`'s [`RwLock`] exposes
/// only bare `*_lock`/`*_unlock` calls, so the guards are what keep a key-table
/// critical section from leaking the lock across an early return. `KeyLock` is
/// `Sync` by auto-derivation — `RwLock` itself declares `Send + Sync`.
struct KeyLock(RwLock);

impl KeyLock {
    /// Creates an unlocked key lock.
    const fn new() -> Self {
        Self(RwLock::new())
    }

    /// Acquires the lock for a destructor-table read (key-table snapshot),
    /// returning a guard that releases it on drop.
    fn read(&self) -> KeyReadGuard<'_> {
        self.0.read_lock();
        KeyReadGuard { lock: self }
    }

    /// Acquires the lock for key allocation or deletion, returning a guard that
    /// releases it on drop.
    fn write(&self) -> KeyWriteGuard<'_> {
        self.0.write_lock();
        KeyWriteGuard { lock: self }
    }
}

/// RAII guard for a read-side acquisition of [`KeyLock`].
///
/// Holding the guard is the evidence the read lock is held; dropping it
/// releases the lock, so a key-table read section cannot leak the lock across
/// an early return.
#[must_use = "the key lock is released as soon as the guard is dropped"]
struct KeyReadGuard<'a> {
    lock: &'a KeyLock,
}

impl Drop for KeyReadGuard<'_> {
    fn drop(&mut self) {
        self.lock.0.read_unlock();
    }
}

/// RAII guard for a write-side acquisition of [`KeyLock`].
///
/// Holding the guard is the evidence the write lock is held; dropping it
/// releases the lock, so key allocation and deletion cannot leak the lock
/// across an early return.
#[must_use = "the key lock is released as soon as the guard is dropped"]
struct KeyWriteGuard<'a> {
    lock: &'a KeyLock,
}

impl Drop for KeyWriteGuard<'_> {
    fn drop(&mut self) {
        self.lock.0.write_unlock();
    }
}

/// Process-global runtime TSD key table.
///
/// One entry per key: `None` marks a free slot, `Some` an allocated one. Only
/// ever accessed while [`TSD_KEY_LOCK`] is held.
static mut TSD_KEYS: [Option<DestructorEntry>; NUM_TSD_KEYS] = [None; NUM_TSD_KEYS];

/// Rotating search hint for the next key-allocation scan.
///
/// Mirrors musl libc's `__pthread_key_next`: [`alloc`] starts its free-slot
/// scan here so a freshly freed key is not immediately handed back out. Only
/// ever accessed while [`TSD_KEY_LOCK`] is held.
static mut NEXT_TSD_KEY: usize = 0;

/// Guards [`TSD_KEYS`] and [`NEXT_TSD_KEY`] against concurrent key allocation,
/// deletion, and destructor scans.
static TSD_KEY_LOCK: KeyLock = KeyLock::new();

/// Allocates a runtime TSD key, returning its [`TsdKey`].
///
/// Scans the global key table from the rotating [`NEXT_TSD_KEY`] hint and
/// claims the first free slot. When `destructor` is `None` the slot stores the
/// [`noop_destructor`] sentinel, so the slot still reads as allocated.
///
/// Returns [`TsdAllocError::NoSlotsAvailable`] when every slot is in use.
pub fn alloc(destructor: Option<Destructor>) -> Result<TsdKey, TsdAllocError> {
    // Keys allocated without a user destructor still occupy the slot; the
    // no-op sentinel keeps `Some`/`None` meaning allocated/free.
    let entry = DestructorEntry(destructor.unwrap_or(noop_destructor));

    let claimed = {
        let _guard = TSD_KEY_LOCK.write();

        // SAFETY: the write lock is held for `_guard`'s scope, so `TSD_KEYS`
        // and `NEXT_TSD_KEY` are ours exclusively. The scan only reads and
        // writes in-bounds slot indices.
        unsafe {
            let mut claimed = None;
            for offset in 0..NUM_TSD_KEYS {
                let idx = (NEXT_TSD_KEY + offset) % NUM_TSD_KEYS;
                // Read the slot by value (it is `Copy`) to avoid taking a
                // reference into the mutable static.
                let current = TSD_KEYS[idx];
                if current.is_none() {
                    TSD_KEYS[idx] = Some(entry);
                    NEXT_TSD_KEY = (idx + 1) % NUM_TSD_KEYS;
                    claimed = Some(idx);
                    break;
                }
            }
            claimed
        }
    };

    match claimed {
        Some(idx) => Ok(TsdKey::from_index(idx)),
        None => Err(TsdAllocError::NoSlotsAvailable),
    }
}

/// Errors returned when allocating a runtime thread-specific-data (TSD) key.
#[derive(Debug, thiserror::Error)]
pub enum TsdAllocError {
    /// Every TSD key slot is already in use.
    #[error("no free TSD key slots available")]
    NoSlotsAvailable,
}

#[cfg(feature = "ffi")]
impl ToRawResultCode for TsdAllocError {
    fn to_rc(self) -> ResultCode {
        match self {
            TsdAllocError::NoSlotsAvailable => KernelError::OutOfResource.to_rc(),
        }
    }
}

/// Reads the calling thread's value for a TSD slot.
///
/// Returns null for any slot the current thread has never stored into (its
/// flat array starts zeroed and [`free`] clears deleted slots), so an
/// unallocated key naturally reads back as null. Returns null as well when the
/// caller is not a thread managed by `nx-sys-thread` and therefore owns no TSD
/// array.
pub fn get(key: TsdKey) -> *mut c_void {
    let Some(runtime) = current_runtime() else {
        return null_mut();
    };

    let idx = key.index();
    // SAFETY: `idx` is in bounds of the flat slot array — a `TsdKey` only ever
    // names an in-range slot — and `runtime` points to the calling thread's
    // live runtime record.
    unsafe { *(*runtime.as_ptr()).tsd.add(idx) }
}

/// Stores `value` into the calling thread's TSD slot.
///
/// Storing a non-null value also arms [`ThreadRuntime::tsd_used`], so the
/// thread-exit [`run_destructors`] pass knows the thread touched TSD.
///
/// When the caller is not a thread managed by `nx-sys-thread` it owns no TSD
/// array, so the store is a silent no-op.
pub fn set(key: TsdKey, value: *mut c_void) {
    let Some(runtime) = current_runtime() else {
        return;
    };

    let idx = key.index();
    // SAFETY: `idx` is in bounds of the flat slot array — a `TsdKey` only ever
    // names an in-range slot — and `runtime` points to the calling thread's
    // live runtime record, touched only by this thread.
    unsafe {
        let runtime = runtime.as_ptr();
        *(*runtime).tsd.add(idx) = value;
        if !value.is_null() {
            (*runtime).tsd_used = true;
        }
    }
}

/// Frees a runtime TSD key, clearing its slot in every live thread.
///
/// Mirrors musl libc's `pthread_key_delete`: under the key write lock it walks
/// the live-thread registry, zeroes this slot in every registered thread's TSD
/// array, then drops the global key entry. Walking the registry takes
/// `THREAD_MUTEX`, keeping the [`TSD_KEY_LOCK`]-then-`THREAD_MUTEX` lock order.
///
/// Returns [`TsdFreeError::UnallocatedSlot`] for a slot that is not currently
/// allocated.
///
/// # Caller contract: the key must not be in use by other threads
///
/// `free` must not be called while any other thread may still [`get`] or
/// [`set`] the key, or run thread-exit destructors for it. The registry walk
/// above zeroes the key's slot in every live thread's array, but [`get`],
/// [`set`], and [`run_destructors`] touch a thread's slot array *without*
/// [`TSD_KEY_LOCK`] held — only [`TSD_KEYS`]/[`NEXT_TSD_KEY`] are lock-guarded.
/// So `free` on one thread can write `tsd[idx]` of a second thread while that
/// second thread concurrently reads or writes the same slot: a data race, and
/// undefined behavior under the Rust memory model even though aligned
/// pointer-word stores happen to be atomic on AArch64.
///
/// This is intentional and matches the API it replaces: POSIX leaves
/// `pthread_key_delete` of a key still used by other threads
/// application-undefined, and musl libc races here too. The contract is
/// therefore pushed to the caller — `threadTlsFree` / `__syscall_tls_delete`
/// must only delete a key once no other thread can reference it: deleting a key
/// still referenced by another thread is application-undefined.
pub fn free(key: TsdKey) -> Result<(), TsdFreeError> {
    let idx = key.index();

    let _guard = TSD_KEY_LOCK.write();

    // SAFETY: the write lock is held for `_guard`'s scope, so reading the key
    // entry is exclusive.
    let allocated = unsafe { TSD_KEYS[idx] }.is_some();
    if !allocated {
        return Err(TsdFreeError::UnallocatedSlot);
    }

    // Clear this slot in every live thread, then drop the global key entry.
    // SAFETY: the write lock is held. `for_each` walks the registry under
    // `THREAD_MUTEX`; the closure only zeroes one in-bounds TSD slot per thread
    // and never re-enters the registry, satisfying `for_each`'s contract.
    unsafe {
        thread_list::for_each(|thread| {
            let runtime = (*thread).runtime().as_ptr();
            *(*runtime).tsd.add(idx) = null_mut();
        });
        TSD_KEYS[idx] = None;
    }

    Ok(())
}

/// Errors returned when freeing a runtime TSD key.
#[derive(Debug, thiserror::Error)]
pub enum TsdFreeError {
    /// The key names an in-range slot that was never allocated.
    #[error("TSD slot is not allocated")]
    UnallocatedSlot,
}

#[cfg(feature = "ffi")]
impl ToRawResultCode for TsdFreeError {
    fn to_rc(self) -> ResultCode {
        match self {
            TsdFreeError::UnallocatedSlot => KernelError::InvalidState.to_rc(),
        }
    }
}

/// Runs the TSD destructor pass for an exiting thread.
///
/// Scans the thread's flat slot array, clearing each value *before* invoking
/// its key's destructor (POSIX/musl ordering), and repeats up to
/// [`PTHREAD_DESTRUCTOR_ITERATIONS`] times while destructors keep storing fresh
/// values. Threads that never stored a TSD value are skipped via the
/// [`ThreadRuntime::tsd_used`] fast path.
///
/// # Safety
///
/// `runtime` must point to the exiting thread's own, exclusively-owned
/// [`ThreadRuntime`]; this must run on that thread, before its slot array is
/// reclaimed.
pub unsafe fn run_destructors(runtime: *mut ThreadRuntime) {
    // The runtime record is reached only through raw pointers: a destructor
    // invoked below may call `threadTlsSet` -> `set`, which re-derives a pointer
    // to this *same* `ThreadRuntime` and writes `tsd_used`. A `&mut`/`&` spanning
    // those calls would alias that foreign write — undefined behavior — and,
    // because `&mut` promises exclusivity, would license the compiler to cache
    // `tsd_used` and never observe the re-arm, silently defeating the retry
    // loop: no `&`/`&mut` reference to per-thread or process-global state may
    // be held across a re-entrant callback.
    //
    // SAFETY: the caller guarantees `runtime` points to the exiting thread's
    // own `ThreadRuntime`, exclusively owned for the duration of this call;
    // `&raw mut` projects a field pointer without forming a reference.
    let tsd_used = unsafe { &raw mut (*runtime).tsd_used };

    // Fast path: a thread that never stored a TSD value has nothing to run.
    // SAFETY: `tsd_used` points into the live runtime record (see above).
    if !unsafe { *tsd_used } {
        return;
    }

    // SAFETY: as above; `tsd` is set once at thread creation and never changes.
    let tsd = unsafe { (*runtime).tsd };
    for _ in 0..PTHREAD_DESTRUCTOR_ITERATIONS {
        // Re-load `tsd_used` from memory each pass: a destructor in the prior
        // pass may have re-armed it through `set`, and the raw-pointer read is
        // not cached across the opaque destructor calls.
        // SAFETY: `tsd_used` points into the live runtime record (see above).
        if !unsafe { *tsd_used } {
            break;
        }
        // Re-arm: `set` flips this back on if a destructor stores a new value,
        // which keeps the retry loop going (POSIX destructor semantics).
        // SAFETY: `tsd_used` points into the live runtime record (see above).
        unsafe { *tsd_used = false };

        // Snapshot the global key table under a brief read lock so destructors
        // are free to call `alloc`/`free` without deadlocking on the lock.
        let keys = {
            let _guard = TSD_KEY_LOCK.read();
            // SAFETY: the read lock is held for `_guard`'s scope; the table is
            // copied out by value.
            unsafe { TSD_KEYS }
        };

        for (idx, key) in keys.iter().enumerate() {
            // SAFETY: `idx < NUM_TSD_KEYS`, so `tsd.add(idx)` stays within the
            // exiting thread's flat slot array.
            let slot = unsafe { tsd.add(idx) };
            // SAFETY: `slot` is in-bounds (see above); the exiting thread owns
            // its slot array exclusively, so reading the stored pointer value
            // through the raw pointer is sound.
            let value = unsafe { *slot };
            // Clear the slot before invoking the destructor, matching musl /
            // POSIX ordering so a destructor sees its own key as empty.
            // SAFETY: `slot` is in-bounds and exclusively owned by the exiting
            // thread (see above), so the raw-pointer write is sound.
            unsafe { *slot = null_mut() };

            if value.is_null() {
                continue;
            }
            if let Some(DestructorEntry(destructor)) = *key {
                // SAFETY: `destructor` was registered for this key by `alloc`
                // (a user destructor, or the no-op sentinel); `value` is the
                // non-null pointer the owning thread stored for the key.
                unsafe { destructor(value) };
            }
        }
    }
}

/// No-op destructor sentinel stored for keys allocated without one.
///
/// Running it on thread exit is harmless, so [`run_destructors`] can invoke
/// every allocated key's destructor uniformly without special-casing the
/// destructor-less case.
///
/// # Safety
///
/// This function has no preconditions: the body is empty, so it is always
/// sound to invoke with any `_value`. It is declared `unsafe` only to match
/// the `Destructor = unsafe extern "C" fn(*mut c_void)` callback type.
unsafe extern "C" fn noop_destructor(_value: *mut c_void) {}

/// Resolves the calling thread's [`ThreadRuntime`] via `ThreadVars`.
///
/// Returns `None` when the thread has no core thread state installed (i.e. it
/// is not a thread created or adopted by `nx-sys-thread`).
fn current_runtime() -> Option<NonNull<ThreadRuntime>> {
    let thread = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    let thread = NonNull::new(thread)?;
    // SAFETY: a non-null `thread_info_ptr` is installed by this crate's thread
    // bring-up paths and points to a `ThreadControl` valid for the lifetime of
    // the running thread. `runtime` is fixed at creation, so reading it through
    // the raw pointer forms no `&ThreadControl` and is sound even though the
    // thread concurrently self-mutates `state`/`prev`/`next`.
    Some(unsafe { (*thread.as_ptr()).runtime })
}
