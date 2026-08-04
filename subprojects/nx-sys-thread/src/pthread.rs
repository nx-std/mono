//! Pthread / newlib thread syscall core.
//!
//! The idiomatic Rust core behind the devkitPro/libsysbase `__syscall_thread_*`
//! override symbols. [`PthreadControl`] is the `Arc`-shared pthread object: it
//! owns a core [`ThreadControl`] plus the thread's POSIX return value, and the
//! `pthread_*` functions here build the `create`/`self`/`exit`/`join`/`detach`
//! lifecycle on top of the shared [`thread`] core — without
//! duplicating any thread lifecycle code.
//!
//! # Core vs. ABI adapter
//!
//! `PthreadControl` is shaped for the Rust core alone — it is deliberately
//! *not* the `#[repr(C)]` `struct __pthread_t` mirror. The `ffi::libsysbase`
//! adapter owns the `LibsysbasePthread` ABI layout and projects it onto this
//! core; C callers never observe a `PthreadControl` directly.
//!
//! # Ownership model
//!
//! [`PthreadControl`] lives inside an [`Arc`], shared by two strong counts. The
//! join handle holds one — inside the [`PthreadJoinHandle`] [`pthread_create`]
//! returns. The spawned thread "holds" the other: it only ever reaches the
//! object through `ThreadVars.thread_info_ptr` (container-of), a raw pointer
//! that owns no count, so [`pthread_create`] leaks one [`Arc::into_raw`] clone
//! for the running thread. The [`Arc`] payload never moves, so the embedded
//! [`ThreadControl`] the thread locates by container-of stays pinned at a fixed
//! address for free.
//!
//! [`pthread_join`] and [`pthread_detach`] are the two reclaiming consumers.
//! [`pthread_join`] waits for the join synchronization edge to prove the thread
//! dead, then reclaims the thread-side count — the thread itself can never drop
//! it, since that would free the [`ThreadControl`] its own exit path still runs
//! on — and frees the object, all on the calling thread. [`pthread_detach`]
//! instead hands the thread off to reclaim *itself*: a detached thread runs the
//! Horizon `__unmapself` port (see [`detach`](crate::detach)) once it exits.
//! Dropping the [`PthreadJoinHandle`] without joining or detaching *detaches*
//! it — so an unjoined handle is reclaimed once its thread exits, not leaked.

use alloc::{
    boxed::Box,
    sync::Arc,
};
use core::{
    cell::UnsafeCell,
    ffi::c_void,
    fmt::{
        self,
        Debug,
    },
    mem::offset_of,
    ptr::{
        self,
        NonNull,
    },
    sync::atomic::AtomicU8,
};

use crate::{
    detach::{
        self,
        DetachState,
        Detachable,
    },
    thread::{
        self,
        Builder,
        CloseError,
        CoreId,
        CreateError,
        Priority,
        StackSpec,
        StartError,
        ThreadControl,
        WaitError,
    },
};

/// Horizon priority assigned to a pthread-created thread.
///
/// newlib's `__syscall_thread_create` ABI carries no priority parameter, so the
/// implementation must pick one. libnx's `__syscall_thread_create` hardcodes
/// `0x3B` in its `threadCreate` call (`newlib.c:190`) — the special priority
/// that enables preemptive multithreading on cores 0–2 (`thread.h`). Matching
/// it keeps the override behaviorally faithful; a lower priority such
/// as the main-thread `0x2C` lacks that property, so a CPU-bound pthread could
/// starve same-core threads. libnx exposes no `svcGetThreadPriority`, so there
/// is no caller priority to inherit.
const DEFAULT_PRIORITY: Priority = Priority::DEFAULT;

/// CPU core assigned to a pthread-created thread.
///
/// The process default core matches libnx's `threadCreate` call in its own
/// newlib pthread shim.
const DEFAULT_CPU_ID: CoreId = CoreId::PROCESS_DEFAULT;

/// POSIX thread routine, as passed to [`pthread_create`].
///
/// Unlike a [`ThreadFunc`] it yields a `*mut c_void`
/// return value, which [`pthread_exit`] stores for a later [`pthread_join`].
pub type PthreadFunc = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

/// Authoritative, Rust-owned runtime object for a single pthread-created thread.
///
/// Owns the core [`ThreadControl`] and the thread's POSIX return value. The
/// embedded `ThreadControl` is the anchor the running thread uses to find this
/// object: `ThreadVars.thread_info_ptr` addresses that field, and
/// [`pthread_self`]/[`pthread_exit`] recover the enclosing `PthreadControl` by
/// container-of arithmetic ([`offset_of!`]).
///
/// Shared through an [`Arc`] and never moved while the thread runs — see the
/// [module ownership model](self#ownership-model).
pub struct PthreadControl {
    /// Core thread state; the container-of anchor for the running thread.
    thread: ThreadControl,
    /// POSIX return value, stored by [`pthread_exit`] and read by
    /// [`pthread_join`].
    ///
    /// An [`UnsafeCell`] so the exiting thread can write it through the shared,
    /// `Arc`-backed object; the [`pthread_join`] read is ordered after that
    /// write by the join synchronization edge, so the access is not a
    /// data race. `None` until [`pthread_exit`] records a value.
    return_value: UnsafeCell<Option<*mut c_void>>,
    /// Detach-vs-exit race state (see [`DetachState`]).
    ///
    /// [`Joinable`](DetachState::Joinable) until [`pthread_detach`] (or the
    /// [`PthreadJoinHandle`] `Drop`) detaches the thread, or the thread's own
    /// exit CAS claims it.
    detach_state: AtomicU8,
}

impl Debug for PthreadControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `return_value` is an `UnsafeCell` the owning thread writes lock-free;
        // it is omitted rather than read, since a `Debug` print could race that
        // write ahead of the join edge.
        f.debug_struct("PthreadControl")
            .field("thread", &self.thread)
            .finish_non_exhaustive()
    }
}

// SAFETY: `PthreadControl` is shared across threads by design — the spawned
// thread reaches it by container-of while the join handle's `Arc` is owned and
// reclaimed on another thread. Its raw-pointer fields address process-wide
// heap/mapped memory, never thread-local storage; the embedded `ThreadControl`
// confines its concurrent self-mutation to atomic fields; and `return_value` is
// an `UnsafeCell` whose write (by the owning thread) and read (by the joiner)
// are ordered by the join edge, never a true data race. Sending the
// object across threads is therefore sound.
unsafe impl Send for PthreadControl {}
// SAFETY: see the `Send` impl above — the same contract makes a shared
// `&PthreadControl` sound to access from more than one thread.
unsafe impl Sync for PthreadControl {}

/// Parameters for creating a pthread-style thread.
///
/// Bundles everything [`pthread_create`] needs that the newlib
/// `__syscall_thread_create` ABI supplies: the routine and its argument, plus
/// how the stack is sourced ([`StackSpec`]). Priority and CPU core are not part
/// of the newlib ABI, so they are not fields here — [`pthread_create`] applies
/// [`DEFAULT_PRIORITY`] and [`DEFAULT_CPU_ID`].
pub struct PthreadCreateConfig {
    /// POSIX thread routine to run on the new thread.
    func: PthreadFunc,
    /// Opaque argument forwarded to `func`.
    arg: *mut c_void,
    /// How the thread's stack memory is sourced.
    stack: StackSpec,
}

impl PthreadCreateConfig {
    /// Builds a pthread-creation config; `stack` selects how the thread's stack
    /// is sourced (see [`StackSpec`]).
    pub fn new(func: PthreadFunc, arg: *mut c_void, stack: StackSpec) -> Self {
        Self { func, arg, stack }
    }
}

/// Move-only join handle for a thread created by [`pthread_create`].
///
/// Owns one strong count of the thread's `Arc`-shared [`PthreadControl`] — the
/// only handle from which the thread can be joined. It is neither `Copy` nor
/// `Clone`, so [`pthread_join`] and [`pthread_detach`], which each consume it
/// by value, can run at most once per thread: a double join or a
/// join-after-detach is a compile error.
///
/// Dropping the handle without joining or detaching *detaches* the thread (see
/// [`detach`](detach::detach)): the thread reclaims itself once it exits. An
/// unjoined handle therefore detaches cleanly instead of leaking — the
/// idiomatic terminal state, mirroring a never-joined `std` `JoinHandle`.
pub struct PthreadJoinHandle {
    /// The thread's `Arc`-shared state. `Some` until [`pthread_join`],
    /// [`pthread_detach`], or the `Drop` detach consumes it, after which the
    /// handle is inert.
    inner: Option<Arc<PthreadControl>>,
}

#[cfg(feature = "ffi")]
impl PthreadJoinHandle {
    /// Raw pointer to the thread's embedded core [`ThreadControl`].
    ///
    /// The `ffi::libsysbase` side registry reverse-maps a running thread back
    /// to the C pthread handle it was registered under (`__syscall_thread_self`)
    /// by matching this against the thread's [`thread::current`] pointer.
    ///
    /// The result is a raw [`NonNull`], never a typed `&` over the
    /// concurrently-live thread.
    ///
    /// # Panics
    ///
    /// Panics — which aborts under `panic = "abort"` — if the handle's `Arc`
    /// was already taken by [`pthread_join`] or [`pthread_detach`]. Unreachable
    /// while the handle is still held by the side registry.
    pub fn thread_control_ptr(&self) -> NonNull<ThreadControl> {
        let arc = self
            .inner
            .as_ref()
            .expect("thread_control_ptr: a PthreadJoinHandle holds its state until joined");
        // SAFETY: `Arc::as_ptr` never returns null.
        let obj = unsafe { NonNull::new_unchecked(Arc::as_ptr(arc).cast_mut()) };
        PthreadControl::thread_ptr(obj)
    }
}

impl Drop for PthreadJoinHandle {
    /// Detaches the thread if it was neither joined nor detached. A handle
    /// already consumed by [`pthread_join`] or [`pthread_detach`] drops as a
    /// no-op.
    fn drop(&mut self) {
        if let Some(control) = self.inner.take() {
            // Neither joined nor detached: detach so the thread reclaims itself
            // once it exits, not leaked (see [`detach`](detach::detach)).
            detach::detach(control);
        }
    }
}

/// Creates and starts a pthread-style thread.
///
/// Unlike the two-step [`thread::create`] + [`thread::start`] core flow, POSIX
/// `pthread_create` semantics require the thread to be running on return, so
/// this function does both. It shares a [`PthreadControl`] through an [`Arc`]
/// between the two steps: the running thread locates that object by
/// container-of from `ThreadVars.thread_info_ptr`, and the `Arc` payload never
/// moves, so its address is fixed before [`thread::start`] wires the
/// back-pointers.
///
/// The returned [`PthreadJoinHandle`] owns the join-handle strong count; a
/// second count is leaked for the spawned thread (see the
/// [module ownership model](self#ownership-model)). The caller (in practice the
/// `ffi::libsysbase` adapter) reclaims the object through [`pthread_join`] or
/// [`pthread_detach`]. Dropping the handle without either detaches the thread
/// (see [`PthreadJoinHandle`]), so it is reclaimed once it exits, not leaked.
///
/// # Safety
///
/// - `config.func` must be a valid pthread routine, and `config.arg` valid to
///   pass to it (or null).
/// - When `config.stack` is [`StackSpec::Provided`], its `base` must point to a
///   page-aligned buffer of `size` bytes that stays valid for the thread's
///   lifetime — the same stack contract [`thread::create`] imposes.
pub unsafe fn pthread_create(
    config: PthreadCreateConfig,
) -> Result<PthreadJoinHandle, PthreadCreateError> {
    // The *data half* of the pthread routine: its function pointer and opaque
    // argument, boxed so they outlive this frame. `pthread_trampoline` reclaims
    // the box on the spawned thread once the routine returns.
    let entry = Box::into_raw(Box::new(PthreadEntry {
        func: config.func,
        arg: config.arg,
    }));

    let thread_config = Builder::new()
        .stack(config.stack)
        .priority(DEFAULT_PRIORITY)
        .core_id(DEFAULT_CPU_ID)
        .build_create(pthread_trampoline, entry.cast::<c_void>());

    // SAFETY: `pthread_trampoline` is a valid entry point and `entry` its
    // matched argument; `config.stack` carries the caller's stack contract
    // straight through to `create`'s matching `# Safety` clause.
    let thread = match unsafe { thread::create(thread_config) } {
        Ok(thread) => thread,
        Err(err) => {
            // `create` failed before the thread ran, so the trampoline never
            // executes and never reclaims `entry` — drop it here.
            // SAFETY: `entry` came from `Box::into_raw` above, unconsumed.
            drop(unsafe { Box::from_raw(entry) });
            return Err(PthreadCreateError::Create(err));
        }
    };

    // Share the pthread state through an `Arc`: the running thread locates the
    // embedded `ThreadControl` by container-of from `ThreadVars.thread_info_ptr`,
    // and the `Arc` payload never moves, so that address stays pinned for free.
    let control = Arc::new(PthreadControl {
        thread,
        return_value: UnsafeCell::new(None),
        detach_state: AtomicU8::new(DetachState::Joinable as u8),
    });

    // Hand the spawned thread its own strong count. It only ever reaches the
    // `PthreadControl` through the raw container-of pointer, which owns no
    // count, so leak one `Arc` clone here for it to "hold"; `pthread_join`
    // reclaims this count once the thread is provably dead.
    let thread_side_ptr = Arc::into_raw(Arc::clone(&control));

    // Project the embedded `thread` field to a raw `NonNull<ThreadControl>`:
    // `start` reaches the now-runnable thread through a raw pointer, never a
    // typed `&`.
    let control_ptr = Arc::as_ptr(&control).cast_mut();
    // SAFETY: `control_ptr` addresses the live `PthreadControl` inside the
    // `Arc`, so `&raw mut (*control_ptr).thread` is a non-null pointer to the
    // stable `ThreadControl` address `start` wires the back-pointers to.
    let thread_ptr = unsafe { NonNull::new_unchecked(&raw mut (*control_ptr).thread) };
    // SAFETY: `thread_ptr` points to the pinned `ThreadControl` of a
    // freshly-created, still-suspended thread, valid for this call.
    match unsafe { thread::start(thread_ptr) } {
        Ok(()) => Ok(PthreadJoinHandle {
            inner: Some(control),
        }),
        Err(err) => {
            // `create` already spawned the kernel thread, but `start` failed
            // and rolled `state` back to `Created`, so the thread stays
            // suspended and `entry_wrap` never runs. Reclaim both `Arc` strong
            // counts to regain sole ownership of the `PthreadControl`, then
            // hand the created-but-not-started `ThreadControl` to
            // `thread::close`, which releases its kernel handle, stack mirror
            // mapping, and `Dtv` node. The never-run trampoline never
            // reclaimed the `PthreadEntry` box, so reconstruct and drop it.
            // SAFETY: `thread_side_ptr` is the `Arc::into_raw` count leaked
            // just above, unconsumed; `from_raw` reclaims it exactly once, and
            // the suspended thread never touches the allocation.
            drop(unsafe { Arc::from_raw(thread_side_ptr) });
            // Reclaiming the thread-side count leaves `control` the sole
            // owner, so `into_inner` yields the `PthreadControl`; a `None`
            // would mean a broken invariant, leaving the thread to leak.
            if let Some(PthreadControl { thread, .. }) = Arc::into_inner(control) {
                // A `Created` thread's stack is mapped but unused, so `close`
                // reclaims it. A `close` failure here only leaks and is
                // effectively unreachable on a fresh handle.
                let _ = thread::close(thread);
            }
            // SAFETY: `entry` came from `Box::into_raw` above and the
            // never-run trampoline never reclaimed it; dropped here once.
            drop(unsafe { Box::from_raw(entry) });
            Err(PthreadCreateError::Start(err))
        }
    }
}

/// Errors returned when creating a pthread-style thread via [`pthread_create`].
#[derive(Debug, thiserror::Error)]
pub enum PthreadCreateError {
    /// [`thread::create`] failed while bringing the thread up.
    #[error("failed to create the pthread")]
    Create(#[source] CreateError),
    /// [`thread::start`] failed while transitioning the created thread to
    /// runnable. Effectively unreachable for a freshly created handle.
    #[error("failed to start the pthread")]
    Start(#[source] StartError),
}

/// Recovers the enclosing [`PthreadControl`] from a pointer to its embedded
/// [`ThreadControl`].
///
/// `ThreadVars.thread_info_ptr` addresses the `thread` field of an `Arc`-shared
/// [`PthreadControl`]; this walks back by that field offset (container-of) to
/// the enclosing object. The result is a raw [`NonNull`] — no `&PthreadControl`
/// is formed, so the concurrent self-mutation of the embedded `ThreadControl`
/// is not a data race.
///
/// # Safety
///
/// `info` must address the embedded `thread` field of an `Arc`-shared
/// `PthreadControl` — i.e. it must be the `ThreadVars.thread_info_ptr` of a
/// thread created by [`pthread_create`]. On a thread created by the plain
/// [`thread`] core (no enclosing `PthreadControl`) the
/// container-of arithmetic yields a bogus pointer.
unsafe fn enclosing_pthread(info: NonNull<ThreadControl>) -> NonNull<PthreadControl> {
    // SAFETY: by the contract `info` addresses the `thread` field of an
    // `Arc`-shared `PthreadControl`, so `byte_sub` by that field offset stays
    // within the same allocation and recovers the enclosing object.
    unsafe {
        info.byte_sub(offset_of!(PthreadControl, thread))
            .cast::<PthreadControl>()
    }
}

/// Returns a raw pointer to the calling thread's [`PthreadControl`], if it has
/// one.
///
/// Reads `ThreadVars.thread_info_ptr` — which points at the embedded
/// [`ThreadControl`] field — and recovers the enclosing `PthreadControl` by
/// container-of arithmetic. Returns `None` only when no core state is installed
/// at all (the caller is not an `nx-sys-thread`-managed thread).
///
/// The result is a [`NonNull`], not a `&PthreadControl`: the embedded
/// `ThreadControl` is mutated without locks by its own thread, so a typed
/// shared reference held across that window would be a data race. It is
/// also bound to the thread's lifetime — valid only until `pthread_join` — so
/// no `'static` reference is handed out.
///
/// # Safety
///
/// The calling thread must have been created by [`pthread_create`]. On a thread
/// created by the plain [`thread`] core (no enclosing
/// `PthreadControl`), the container-of arithmetic yields a bogus pointer and
/// dereferencing the result is undefined behavior. The `ffi::libsysbase`
/// adapter is responsible for the main-thread sentinel handling that keeps this
/// contract intact at the C boundary.
pub unsafe fn pthread_self() -> Option<NonNull<PthreadControl>> {
    let info = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    // SAFETY: by the contract the caller is a pthread-created thread, so a
    // non-null `info` addresses the `thread` field of an `Arc`-shared
    // `PthreadControl` that stays valid until `pthread_join`; `enclosing_pthread`
    // recovers the enclosing object as a raw pointer, forming no `&PthreadControl`.
    NonNull::new(info).map(|info| unsafe { enclosing_pthread(info) })
}

/// Stores a POSIX return value for the calling thread and terminates it.
///
/// Records `value` in the calling thread's [`PthreadControl`] so a later
/// [`pthread_join`] can return it, then tears the thread down through the
/// shared [`thread::exit`] core path (runtime TSD destructors, registry
/// unlink, `svcExitThread`). Never returns.
///
/// # Safety
///
/// - Must be called on a thread created by [`pthread_create`].
/// - Must run at most once per thread, as that thread's final operation — the
///   same contract [`thread::exit`] imposes.
pub unsafe fn pthread_exit(value: *mut c_void) -> ! {
    // Locate this thread's `Arc`-shared `PthreadControl` and store the return
    // value. The thread-info pointer addresses the embedded `thread` field;
    // walk back to the enclosing object (container-of).
    let info = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    let Some(info) = NonNull::new(info) else {
        // No core state — the caller is not an `nx-sys-thread`-managed thread,
        // a `# Safety` violation. Fall back to the plain core exit path, which
        // aborts on this broken invariant.
        // SAFETY: runs once, on this thread, as its final operation.
        unsafe { thread::exit() }
    };
    // SAFETY: on a pthread-created thread `info` addresses the `thread` field
    // of an `Arc`-shared `PthreadControl`; `enclosing_pthread` recovers the
    // enclosing object.
    let control = unsafe { enclosing_pthread(info) };
    // SAFETY: `control` points to this thread's live `PthreadControl`;
    // `raw_get` yields the `return_value` slot without forming a reference, and
    // the joiner reads it only after observing the exit, so this write is
    // ordered before any read of it.
    unsafe {
        *UnsafeCell::raw_get(&raw const (*control.as_ptr()).return_value) = Some(value);
    }

    // Tear the thread down through the detach-aware exit path: the exit prefix,
    // then either `svcExitThread` (still joinable) or self-reclaim through the
    // Horizon `__unmapself` port (detached). Never returns.
    // SAFETY: runs on this pthread-created thread as its final operation;
    // `control` is its `Arc`-shared `PthreadControl` with both `Arc` counts
    // outstanding.
    unsafe { detach::exit_self_or_detached(control) }
}

/// Joins a pthread-style thread and returns its POSIX return value.
///
/// Blocks until the thread has run its exit path, reclaims its stack mapping
/// and kernel handle through [`thread::close`], frees the `Arc`-shared
/// [`PthreadControl`], and returns the value the thread stored via
/// [`pthread_exit`]. It consumes the [`PthreadJoinHandle`], so a thread is
/// joined at most once; it is one of the two reclaiming consumers of a
/// [`pthread_create`] handle, alongside [`pthread_detach`].
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — on a broken
/// handle invariant: a `PthreadJoinHandle` whose `Arc` was already taken, or a
/// shared `Arc` cloned behind the move-only handle. Neither is reachable
/// through this crate's API.
pub fn pthread_join(mut handle: PthreadJoinHandle) -> Result<*mut c_void, PthreadJoinError> {
    // A `PthreadJoinHandle` holds its `Arc` until a consuming call takes it,
    // and every consuming call takes the handle by value, so the slot is `Some`.
    let control = handle
        .inner
        .take()
        .expect("pthread_join: a PthreadJoinHandle holds its state until joined");
    // SAFETY: a `PthreadJoinHandle` is move-only and built only by
    // `pthread_create`, so `control` is an un-cloned, not-yet-reclaimed join
    // handle — exactly what `reclaim` requires. `reclaim` performs the
    // termination wait, the thread-side-count reclaim, and the `close`, all on
    // the calling thread.
    unsafe { reclaim(control) }
}

/// Errors returned when joining a pthread-style thread via [`pthread_join`].
#[derive(Debug, thiserror::Error)]
pub enum PthreadJoinError {
    /// [`thread::wait_for_exit`] failed while waiting for the thread to exit.
    ///
    /// The thread may still be running, so its return value cannot be
    /// recovered and the `Arc`-shared [`PthreadControl`] leaks — the
    /// thread-side `Arc` count keeps it live — rather than reclaiming a
    /// still-running thread's state.
    #[error("failed to wait for the pthread to exit")]
    Wait(#[source] WaitError),
    /// [`thread::close`] failed while reclaiming the exited thread's resources.
    ///
    /// The thread had already exited and recorded its `pthread_exit` return
    /// value before [`thread::close`] ran, so `value` carries that value out:
    /// a `close` failure leaks the stack mapping and kernel handle but does not
    /// invalidate the value the thread recorded.
    #[error("failed to reclaim the joined pthread")]
    Close {
        /// The POSIX return value the joined pthread recorded via `pthread_exit`.
        value: *mut c_void,
        /// The underlying [`thread::close`] failure.
        #[source]
        source: CloseError,
    },
}

/// Waits for a joined pthread to exit, then reclaims it.
///
/// The reclamation path behind [`pthread_join`]: it waits on the thread's
/// termination, then hands off to [`reclaim_after_exit`]. It is also reused for
/// the detach-after-exit case — [`Detachable::reclaim_exited`] calls it when a
/// `pthread_detach` loses the race to the thread's own exit.
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if the caller is
/// not the sole `Arc` owner after reclaiming the thread-side count. Reaching
/// that state means the handle was cloned, violating the `# Safety` contract.
///
/// # Safety
///
/// `thread` must be a join handle returned by [`pthread_create`] that has not
/// been cloned and whose thread-side `Arc` count has not yet been reclaimed —
/// i.e. it must not have been joined or detached already.
unsafe fn reclaim(thread: Arc<PthreadControl>) -> Result<*mut c_void, PthreadJoinError> {
    // Wait for the thread to run its exit path *before* touching the shared
    // object. The thread writes `return_value` — and may still call
    // `pthread_self` — right up until `pthread_exit`, so reading or freeing the
    // `PthreadControl` ahead of the termination wait both returns a stale value
    // and races the exiting thread.
    //
    // Project the embedded `thread` field to a raw `NonNull<ThreadControl>`
    // without forming a typed `&` over the concurrently-live thread:
    // the thread foreign-writes its own `state`/`prev`/`next` right up until
    // `pthread_exit`, which a shared reference held across the wait would race.
    // SAFETY: `thread` is a live `Arc<PthreadControl>`, so `Arc::as_ptr` yields
    // a pointer valid for this call; `&raw mut` projects its `thread` field to
    // a non-null pointer without dereferencing through a reference.
    let thread_control =
        unsafe { NonNull::new_unchecked(&raw mut (*Arc::as_ptr(&thread).cast_mut()).thread) };
    // On a wait failure the thread may still be live: return, dropping this
    // `Arc` count. The thread-side count keeps the object pinned (a
    // leak-on-error) rather than reclaiming a still-running thread's state.
    // SAFETY: `thread_control` points to the embedded `ThreadControl` of the
    // `Arc`-shared `PthreadControl`; it stays valid until this call reclaims it.
    unsafe { thread::wait_for_exit(thread_control) }.map_err(PthreadJoinError::Wait)?;

    // SAFETY: `wait_for_exit` returned `Ok`, proving the thread terminated;
    // `thread` is the un-cloned, not-yet-reclaimed join handle this call
    // received — exactly `reclaim_after_exit`'s contract.
    unsafe { reclaim_after_exit(thread) }
}

/// Reclaims a pthread whose termination has already been observed.
///
/// The post-termination half of [`reclaim`]: it reclaims the thread-side
/// [`Arc`] strong count [`pthread_create`] leaked, frees the [`PthreadControl`]
/// through [`thread::close`], and returns the value the thread recorded via
/// [`pthread_exit`]. Splitting it out keeps the reclaim logic in one place,
/// distinct from the termination wait that precedes it.
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if the caller is
/// not the sole `Arc` owner after reclaiming the thread-side count. Reaching
/// that state means the handle was cloned, violating the `# Safety` contract.
///
/// # Safety
///
/// - `thread` must be a join handle returned by [`pthread_create`] that has not
///   been cloned and whose thread-side `Arc` count has not yet been reclaimed.
/// - The thread must have *already exited*, its termination observed through
///   [`thread::wait_for_exit`] or [`thread::wait_for_any_exit`], so its
///   `pthread_exit` write of `return_value` happened-before this call, and it
///   will never touch the object again.
unsafe fn reclaim_after_exit(thread: Arc<PthreadControl>) -> Result<*mut c_void, PthreadJoinError> {
    // Reclaim the spawned thread's `Arc` strong count. `pthread_create` leaked
    // one `Arc::into_raw` clone for the running thread; the thread can never
    // drop it (that would free the `ThreadControl` its exit path runs on), so
    // it is reclaimed here, now that the thread is provably dead.
    // SAFETY: `pthread_create` leaked exactly one count via `Arc::into_raw` at
    // this data address and the `# Safety` contract forbids cloning the handle,
    // so `from_raw` consumes that count exactly once; the allocation is still
    // live — `thread` holds the other count.
    drop(unsafe { Arc::from_raw(Arc::as_ptr(&thread)) });

    // The caller now holds the sole strong count, so it can move the
    // `PthreadControl` out and reclaim the exited thread's resources.
    let control = Arc::into_inner(thread).expect(
        "reclaim_after_exit: caller must be the sole Arc owner after the thread-side count",
    );
    let PthreadControl {
        thread: control_thread,
        return_value,
        detach_state: _,
    } = control;

    // Read the recorded return value *before* `close`, so a `close` failure
    // carries the value out in `PthreadJoinError::Close` instead of dropping
    // the value the thread recorded via `pthread_exit` — the recorded value is
    // independent of whether reclaiming the stack mapping and handle succeeds.
    // A thread that never recorded a value reads as a null pointer, POSIX's
    // default.
    let value = return_value.into_inner().unwrap_or(ptr::null_mut());

    // Reclaim the exited thread's stack mapping and kernel handle.
    if let Err(source) = thread::close(control_thread) {
        return Err(PthreadJoinError::Close { value, source });
    }

    Ok(value)
}

/// Detaches a pthread-style thread.
///
/// POSIX `pthread_detach` releases the caller from its obligation to join: a
/// detached thread reclaims itself once it exits. [`detach`](detach::detach)
/// resolves the detach-vs-exit race — if the thread is still running it will
/// self-reclaim through the Horizon `__unmapself` port (see
/// [`detach`](crate::detach)) once it exits; if it already exited, this call
/// reclaims it on the spot.
///
/// Consumes the [`PthreadJoinHandle`], so a detached thread cannot then be
/// joined — the move makes a later [`pthread_join`] a compile error. Dropping a
/// handle has the same effect (see [`PthreadJoinHandle`]).
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if the
/// `PthreadJoinHandle`'s `Arc` was already taken, which a move-only handle
/// makes unreachable through this crate's API.
pub fn pthread_detach(mut handle: PthreadJoinHandle) {
    // A `PthreadJoinHandle` holds its `Arc` until a consuming call takes it,
    // and every consuming call takes the handle by value, so the slot is `Some`.
    let control = handle
        .inner
        .take()
        .expect("pthread_detach: a PthreadJoinHandle holds its state until detached");
    detach::detach(control);
}

// `Detachable` lets a detached pthread self-reclaim its `Arc`-shared
// `PthreadControl` through `detach`'s `exit_self_or_detached` / `unmap_self`,
// and lets `pthread_detach` / the `PthreadJoinHandle` `Drop` route through
// `detach::detach`.
impl Detachable for PthreadControl {
    fn thread_ptr(obj: NonNull<Self>) -> NonNull<ThreadControl> {
        // Project the embedded `thread` field without forming a typed `&` over
        // the concurrently-live thread.
        // SAFETY: `obj` addresses a live `PthreadControl`; `&raw mut` projects
        // its `thread` field to a non-null pointer without going through a
        // reference.
        unsafe { NonNull::new_unchecked(&raw mut (*obj.as_ptr()).thread) }
    }

    fn detach_state(obj: NonNull<Self>) -> NonNull<AtomicU8> {
        // SAFETY: `obj` addresses a live `PthreadControl`; `&raw mut` projects
        // its `detach_state` field to a non-null pointer.
        unsafe { NonNull::new_unchecked(&raw mut (*obj.as_ptr()).detach_state) }
    }

    fn into_thread_control(self) -> ThreadControl {
        // `return_value` and `detach_state` drop here — a detached thread has
        // no joiner to receive its POSIX return value.
        self.thread
    }

    unsafe fn reclaim_exited(arc: Arc<Self>) {
        // SAFETY: by the contract `arc` is the un-cloned join handle whose
        // thread has reached its exit CAS — exactly `reclaim`'s precondition.
        // `reclaim` performs the termination wait and the reclaim; the
        // recovered POSIX return value is dropped (a detached thread has no
        // joiner).
        let _ = unsafe { reclaim(arc) };
    }
}

/// The *data half* of a pthread routine, handed to the spawned thread.
///
/// [`pthread_create`] boxes this and passes the raw pointer as the C `arg` of
/// the generic thread entry path; [`pthread_trampoline`] reclaims it on the new
/// thread and re-joins the routine with its argument.
struct PthreadEntry {
    /// POSIX thread routine to invoke.
    func: PthreadFunc,
    /// Opaque argument forwarded to `func`.
    arg: *mut c_void,
}

/// Entry trampoline that runs a pthread routine and stores its return value.
///
/// Passed to [`thread::create`] as the raw [`ThreadFunc`]:
/// it reconstructs the [`PthreadEntry`] box, runs the routine exactly once, and
/// hands the result to [`pthread_exit`] — which stores it and exits through the
/// shared thread lifecycle.
///
/// # Safety
///
/// `arg` must be the `Box::into_raw(Box::new(PthreadEntry { .. }))` pointer that
/// [`pthread_create`] paired with this trampoline.
unsafe extern "C" fn pthread_trampoline(arg: *mut c_void) {
    // SAFETY: `arg` is the `Box::into_raw(Box::new(PthreadEntry { .. }))`
    // pointer `pthread_create` paired with this trampoline.
    let entry = unsafe { Box::from_raw(arg.cast::<PthreadEntry>()) };
    let PthreadEntry { func, arg } = *entry;

    // SAFETY: `func`/`arg` are the `config.func`/`config.arg` pair
    // `pthread_create` stored verbatim in the `PthreadEntry` box — a valid
    // pthread routine and argument by `pthread_create`'s `# Safety` clause.
    let return_value = unsafe { func(arg) };

    // Store the return value and tear the thread down; `pthread_exit` never
    // returns, so control does not fall back to the generic entry wrapper.
    // SAFETY: this runs on a pthread-created thread, as its final operation.
    unsafe { pthread_exit(return_value) }
}
