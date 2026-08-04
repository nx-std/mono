//! Detached-thread self-reclaim — the Horizon port of musl's `__unmapself`.
//!
//! A *joinable* thread is reclaimed by whoever holds its join handle: the
//! handle waits for the thread to exit and then runs [`thread::close`] from its
//! own stack. A *detached* thread has no such joiner, so it must reclaim
//! itself — yet a thread cannot [`thread::close`] from the very stack it is
//! still executing on, because `close` unmaps that stack's mirror mapping.
//!
//! musl solves this with `__unmapself`: the detached thread switches onto a
//! small process-shared stack and tears its own mapping down from there. The
//! per-arch musl stub is stackless (`munmap` + `exit` in registers), but
//! Horizon teardown is richer than one `munmap` — [`thread::close`] also runs
//! heap `dealloc`s — so this port follows musl's *generic* `__unmapself`
//! (`src/thread/__unmapself.c`): a shared `.bss` exit stack the detached thread
//! switches onto. musl's generic variant is unguarded and therefore racy across
//! concurrent detached exits; this port adds a raw spin-guard ([`EXIT_GUARD`])
//! so only one self-reclaiming thread uses the shared stack at a time.
//!
//! # The detach-vs-exit race
//!
//! Detachment and the thread's own exit run concurrently. A [`DetachState`]
//! atomic on the join-handle object resolves the race with a single CAS each:
//!
//! - the detacher does `CAS(Joinable → Detached)`; on failure the thread has
//!   already exited and the detacher reclaims it itself.
//! - the thread does `CAS(Joinable → Exited)` in [`exit_self_or_detached`]; on
//!   failure it was detached and self-reclaims through [`unmap_self`].
//!
//! Exactly one CAS wins, so the thread is reclaimed exactly once, by exactly
//! one party — never double-freed, never leaked.

use alloc::sync::Arc;
use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{
        AtomicBool,
        AtomicU8,
        Ordering,
    },
};

use crate::thread::{
    self,
    ThreadControl,
};

/// `svcExitThread` syscall immediate (`nx_svc::code::EXIT_THREAD`).
///
/// Issued directly by the [`exit_thread_release_guard`] naked tail, which
/// cannot call through `nx-svc` without touching the stack.
const SVC_EXIT_THREAD: u32 = 0xA;

/// Size, in bytes, of the process-shared exit stack ([`EXIT_STACK`]).
///
/// A self-reclaiming thread only runs [`thread::close`] (a handful of SVCs plus
/// a few heap frees) on it, so 8 KiB is ample. A multiple of 16 so the high end
/// is a valid AArch64 stack pointer.
const EXIT_STACK_SIZE: usize = 0x2000;

/// Serializes self-reclaiming threads over the shared [`EXIT_STACK`] and
/// [`EXIT_THREAD_CONTROL`] slot.
///
/// `true` while a thread owns the shared exit stack. Acquired on the dying
/// thread's *own* stack in [`unmap_self_with`]; released by the
/// [`exit_thread_release_guard`] naked tail with a store-release immediately
/// before `svcExitThread`, with no stack-touching instruction between — which
/// is why this is a raw atomic rather than a `Mutex` whose unlock would be a
/// stack-using call.
static EXIT_GUARD: AtomicBool = AtomicBool::new(false);

/// The process-shared stack a detached thread switches onto to reclaim itself.
///
/// Guarded by [`EXIT_GUARD`]: exactly one self-reclaiming thread uses it at a
/// time. Never freed — it lives for the lifetime of the process.
static EXIT_STACK: ExitStack = ExitStack(UnsafeCell::new([0; EXIT_STACK_SIZE]));

/// Hand-off slot carrying the dying thread's [`ThreadControl`] across the stack
/// switch.
///
/// [`unmap_self_with`] writes it on the dying thread's own stack; [`unmap_self_finish`]
/// reads it back after switching onto [`EXIT_STACK`]. It lives in `.bss`, not in
/// the thread's stack mirror, so it survives the switch. Guarded by
/// [`EXIT_GUARD`].
static EXIT_THREAD_CONTROL: ExitThreadSlot = ExitThreadSlot(UnsafeCell::new(MaybeUninit::uninit()));

/// 16-aligned `.bss` storage for [`EXIT_STACK`].
///
/// `align(16)` plus the 16-multiple [`EXIT_STACK_SIZE`] makes the high end a
/// valid AArch64 stack pointer.
#[repr(align(16))]
struct ExitStack(UnsafeCell<[u8; EXIT_STACK_SIZE]>);

// SAFETY: `EXIT_STACK` is only ever read as a raw `sp` value, and never while
// more than one thread uses it — `EXIT_GUARD` enforces exclusive access. No
// `&`/`&mut` to the buffer is ever formed.
unsafe impl Sync for ExitStack {}

impl ExitStack {
    /// Raw pointer to the 16-aligned high end of the stack, for `mov sp, _`.
    fn top(&self) -> *mut u8 {
        // SAFETY: one-past-the-end of the `EXIT_STACK_SIZE` buffer is a valid
        // pointer to form (never dereferenced); the descending stack grows away
        // from it. It is 16-aligned because `ExitStack` is `align(16)` and the
        // size is a 16-multiple.
        unsafe { self.0.get().cast::<u8>().add(EXIT_STACK_SIZE) }
    }
}

/// `.bss` storage for the [`EXIT_THREAD_CONTROL`] hand-off slot.
struct ExitThreadSlot(UnsafeCell<MaybeUninit<ThreadControl>>);

// SAFETY: the slot is written and read only by the single thread that holds
// `EXIT_GUARD`, so the cross-thread hand-off is fully serialized; no `&`/`&mut`
// aliases the cell.
unsafe impl Sync for ExitThreadSlot {}

/// Lifecycle state of a detachable join-handle object, resolving the
/// detach-vs-exit race (see the [module docs](self#the-detach-vs-exit-race)).
///
/// A trimmed musl `DT_*` ladder: musl's intermediate `DT_EXITING` is dropped
/// because this crate's join wait blocks on the *kernel handle*, not on this
/// state — the kernel handle already signals termination. This atomic exists
/// solely to decide *who reclaims*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum DetachState {
    /// The thread has a live join handle and has not been detached. Initial
    /// state of every [`pthread_create`](crate::pthread::pthread_create)d /
    /// [`spawn`](crate::thread::spawn)ed thread.
    Joinable = 0,
    /// The join handle was detached before the thread exited; the thread will
    /// reclaim itself through [`unmap_self`].
    Detached = 1,
    /// The thread ran its exit path while still joinable; a joiner — or a
    /// detach-after-exit — reclaims it.
    Exited = 2,
}

impl DetachState {
    /// The discriminant, for an [`AtomicU8`] CAS.
    const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// A join-handle object that owns a detachable thread.
///
/// Implemented by the `Arc`-shared payloads behind the two Level-1 join
/// handles — `PthreadControl` and `SpawnInner<T>` — so [`exit_self_or_detached`]
/// and [`unmap_self`] drive both through one path. Each object embeds a core
/// [`ThreadControl`] and a [`DetachState`] atomic.
pub(crate) trait Detachable: Sized {
    /// Raw pointer to the embedded core [`ThreadControl`].
    fn thread_ptr(obj: NonNull<Self>) -> NonNull<ThreadControl>;

    /// Raw pointer to the embedded [`DetachState`] atomic.
    fn detach_state(obj: NonNull<Self>) -> NonNull<AtomicU8>;

    /// Consumes the object, dropping the joiner-facing payload (the recorded
    /// return value — a detached thread has no joiner to receive it) and
    /// yielding the core [`ThreadControl`].
    fn into_thread_control(self) -> ThreadControl;

    /// Reclaims a handle whose thread has already exited (or is mid-exit):
    /// waits for the kernel exit edge, reclaims the thread-side `Arc` count,
    /// and frees the object through [`thread::close`]. Any recorded return
    /// value is dropped.
    ///
    /// The detach-after-exit path: invoked by [`detach`] when the detaching
    /// `CAS(Joinable → Detached)` loses to the thread's own exit.
    ///
    /// # Safety
    ///
    /// `arc` must be the un-cloned join-handle `Arc` whose thread-side count is
    /// still outstanding, and the thread must have observably reached its exit
    /// CAS (state [`Exited`](DetachState::Exited)).
    unsafe fn reclaim_exited(arc: Arc<Self>);
}

/// Runs the calling thread's exit path, self-reclaiming if it was detached;
/// never returns.
///
/// Runs the shared stack-safe exit prefix ([`thread::exit_prefix`]: runtime TSD
/// destructors, live-thread-registry unlink, `state` store), then the
/// `CAS(Joinable → Exited)` that resolves the detach-vs-exit race:
///
/// - **CAS succeeds** — the thread was still joinable, so a joiner (or a later
///   detach-after-exit) reclaims it: issue `svcExitThread` and stop.
/// - **CAS fails** — the state is [`Detached`](DetachState::Detached), so the
///   thread reclaims itself through [`unmap_self`].
///
/// # Safety
///
/// - Must run on a thread created by [`pthread_create`](crate::pthread::pthread_create)
///   or [`spawn`](crate::thread::spawn), as that thread's final operation.
/// - `obj` must be the `Arc`-shared `O` whose embedded `thread` field is the
///   calling thread's pinned [`ThreadControl`], with both the thread-side
///   `Arc::into_raw` count and (once detached) the handle-side count still
///   outstanding — the count state [`unmap_self`] relies on.
pub(crate) unsafe fn exit_self_or_detached<O: Detachable>(obj: NonNull<O>) -> ! {
    // Run the stack-safe teardown prefix on this thread's own stack while its
    // TLS and registry links are still valid.
    // SAFETY: by the contract `obj` encloses the calling thread's pinned
    // `ThreadControl`; `exit_prefix` runs once, here, as the final teardown.
    unsafe { thread::exit_prefix(O::thread_ptr(obj).as_ptr()) };

    // Resolve the detach-vs-exit race. `Acquire` on failure pairs with the
    // detacher's `Release` CAS so a `Detached` observation is fully ordered.
    let detach_state = O::detach_state(obj);
    // SAFETY: `detach_state` addresses the live `DetachState` atomic embedded in
    // the `Arc`-shared `O`; the thread-side `Arc` count keeps that allocation
    // alive until this thread reclaims it.
    let cas = unsafe {
        (*detach_state.as_ptr()).compare_exchange(
            DetachState::Joinable.as_u8(),
            DetachState::Exited.as_u8(),
            Ordering::AcqRel,
            Ordering::Acquire,
        )
    };

    match cas {
        // Still joinable: a joiner or a detach-after-exit reclaims this thread.
        // The exit prefix already ran, so hand straight to the kernel.
        Ok(_) => nx_svc::thread::exit(),
        // Detached: no joiner will ever come — reclaim this thread itself.
        // SAFETY: a failed `Joinable → Exited` CAS means the state is
        // `Detached`, so the detacher leaked its handle-side `Arc` count for
        // this thread to reclaim — exactly `unmap_self`'s precondition.
        Err(_) => unsafe { unmap_self(obj) },
    }
}

/// Detaches a thread's join handle so the thread reclaims itself once it exits.
///
/// Resolves the detach side of the race with a `CAS(Joinable → Detached)`:
///
/// - **CAS succeeds** — the thread is still live: leak `arc`'s strong count so
///   the thread finds both `Arc` counts outstanding and self-reclaims through
///   [`unmap_self`] on exit.
/// - **CAS fails** — the thread already ran its exit CAS (state
///   [`Exited`](DetachState::Exited)) and will not self-reclaim, so reclaim it
///   here through [`Detachable::reclaim_exited`].
///
/// Consumes `arc`, the join handle's sole strong count. The move-only join
/// handles (`JoinHandle` / `PthreadJoinHandle`) guarantee it is un-cloned with
/// its thread-side count still outstanding, so this is a safe `fn` — like
/// [`JoinHandle::join`](crate::thread::JoinHandle::join).
pub(crate) fn detach<O: Detachable>(arc: Arc<O>) {
    // SAFETY: `Arc::as_ptr` never returns null.
    let obj = unsafe { NonNull::new_unchecked(Arc::as_ptr(&arc).cast_mut()) };
    let detach_state = O::detach_state(obj);

    // `Release` on success publishes the `Detached` state to the thread's exit
    // CAS; `Acquire` on failure orders the `Exited` observation.
    // SAFETY: `detach_state` addresses the live `DetachState` atomic of the
    // `Arc`-shared `O`, kept alive by `arc`'s own strong count.
    let cas = unsafe {
        (*detach_state.as_ptr()).compare_exchange(
            DetachState::Joinable.as_u8(),
            DetachState::Detached.as_u8(),
            Ordering::AcqRel,
            Ordering::Acquire,
        )
    };

    match cas {
        // Won the race: the thread is still live and will self-reclaim. Leak
        // this count so `unmap_self` finds both `Arc` counts outstanding.
        Ok(_) => core::mem::forget(arc),
        // Lost: the thread already exited and will not self-reclaim.
        // SAFETY: `arc` is the un-cloned join-handle count with its thread-side
        // count still outstanding, and the `Exited` state proves the thread ran
        // its exit CAS — exactly `reclaim_exited`'s contract.
        Err(_) => unsafe { O::reclaim_exited(arc) },
    }
}

/// Reclaims the calling — detached — thread and terminates it; never returns.
///
/// The Horizon `__unmapself`: takes sole ownership of the `Arc`-shared `O`,
/// then [hands off](unmap_self_with) to the shared exit stack to run
/// [`thread::close`] (which unmaps the very stack this thread runs on) and
/// `svcExitThread`.
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if the calling
/// thread is not the sole `Arc` owner after reclaiming both outstanding counts.
/// Unreachable: the two-`Arc`-count detach contract guarantees exactly those
/// two counts exist, and both are consumed here.
///
/// # Safety
///
/// Must run on a detached thread, as its final operation, with both the
/// thread-side and the detacher-leaked `Arc` strong counts of `obj`
/// outstanding and un-reclaimed.
unsafe fn unmap_self<O: Detachable>(obj: NonNull<O>) -> ! {
    let ptr = obj.as_ptr().cast_const();

    // Reclaim both outstanding `Arc` counts to take sole ownership: one was
    // leaked at creation for the running thread, the other leaked by the
    // detacher's winning `CAS(Joinable → Detached)`. With the CAS resolved no
    // other party can touch `O`, so both counts are reclaimed here exactly once.
    // SAFETY: by the contract two `Arc<O>` strong counts are outstanding at
    // `ptr`; each `from_raw` reclaims one. Dropping the first leaves the second
    // as the sole owner.
    drop(unsafe { Arc::from_raw(ptr) });
    // SAFETY: the second outstanding count — see above.
    let arc = unsafe { Arc::from_raw(ptr) };
    let obj = Arc::into_inner(arc)
        .expect("unmap_self: a self-reclaiming thread must be the sole Arc owner");

    // Move the core `ThreadControl` out; the recorded return value is dropped
    // (a detached thread has no joiner to receive it).
    unmap_self_with(obj.into_thread_control())
}

/// Switches onto the shared exit stack and reclaims `thread` from there; never
/// returns.
///
/// `thread::close` unmaps the stack mirror the calling thread is *still
/// executing on*, so the teardown must run on a different stack. This acquires
/// [`EXIT_GUARD`], parks `thread` in the [`EXIT_THREAD_CONTROL`] hand-off slot,
/// and switches `sp` onto [`EXIT_STACK`] — all on the dying thread's own stack,
/// where its TLS is still valid — then [`unmap_self_finish`] takes over.
fn unmap_self_with(thread: ThreadControl) -> ! {
    // Acquire the shared exit stack. Teardown is short, so a bounded spin is
    // cheaper than parking; concurrent detached exits are rare.
    while EXIT_GUARD.swap(true, Ordering::Acquire) {
        core::hint::spin_loop();
    }

    // Park the `ThreadControl` in the `.bss` hand-off slot so it survives the
    // stack switch.
    // SAFETY: `EXIT_GUARD` is held, so this thread is the slot's exclusive
    // writer; no other reference to the cell exists.
    unsafe { (*EXIT_THREAD_CONTROL.0.get()).write(thread) };

    // Switch `sp` onto the shared exit stack and divert to `unmap_self_finish`.
    // SAFETY: `EXIT_GUARD` is held, so `EXIT_STACK` is this thread's exclusive
    // stack; `top()` is its valid 16-aligned high end.
    unsafe { unmap_self_switch(EXIT_STACK.top()) }
}

/// Reclaims the dying thread from the shared exit stack, then terminates it.
///
/// Runs on [`EXIT_STACK`] after [`unmap_self_switch`] — never call it directly.
/// It reclaims the parked [`ThreadControl`] through [`thread::close`] (stack
/// mirror, backing allocation, DTV node, kernel handle) and ends in the
/// [`exit_thread_release_guard`] naked tail.
extern "C" fn unmap_self_finish() -> ! {
    // Take back the `ThreadControl` parked before the stack switch.
    // SAFETY: this thread holds `EXIT_GUARD` and `unmap_self_with` initialized
    // the slot just before switching here, so it holds a valid `ThreadControl`
    // owned solely by this thread.
    let thread = unsafe { (*EXIT_THREAD_CONTROL.0.get()).assume_init_read() };

    // Reclaim the stack mirror, backing allocation, DTV node and kernel handle.
    // A failure has no recovery sink on the self-reclaim path — as with the
    // former reaper — so it only leaks; the thread must still terminate.
    let _ = thread::close(thread);

    // Release the shared exit stack and issue `svcExitThread`, with no
    // stack-touching instruction between the two — see `exit_thread_release_guard`.
    // SAFETY: this thread holds `EXIT_GUARD`; the naked tail clears exactly that
    // guard and never returns.
    unsafe { exit_thread_release_guard(&raw const EXIT_GUARD) }
}

/// Switches `sp` to `stack_top` and tail-calls [`unmap_self_finish`]; never
/// returns.
///
/// A naked function so no compiler-inserted prologue touches the old stack
/// after the switch.
///
/// # Safety
///
/// `stack_top` must be the 16-aligned high end of a stack owned exclusively by
/// the calling thread ([`EXIT_STACK`], under [`EXIT_GUARD`]).
#[unsafe(naked)]
unsafe extern "C" fn unmap_self_switch(stack_top: *mut u8) -> ! {
    core::arch::naked_asm!(
        "mov sp, x0",       // switch onto the caller-supplied exit stack
        "b {finish}",       // run the teardown from there; never returns
        finish = sym unmap_self_finish,
    )
}

/// Clears the exit guard and issues `svcExitThread`; never returns.
///
/// The store-release that clears `guard` and the `svc` are adjacent with no
/// stack access between them: once the guard is clear another self-reclaiming
/// thread may seize [`EXIT_STACK`], so this thread must not touch its (shared)
/// stack again. A naked function guarantees the compiler inserts nothing.
///
/// # Safety
///
/// `guard` must be [`EXIT_GUARD`], held by the calling thread. The thread is
/// terminated; no resource it still owns is reclaimed after this point.
#[unsafe(naked)]
unsafe extern "C" fn exit_thread_release_guard(guard: *const AtomicBool) -> ! {
    core::arch::naked_asm!(
        "stlrb wzr, [x0]",      // store-release 0 → *guard (frees EXIT_STACK)
        "svc {exit}",           // svcExitThread — never returns
        exit = const SVC_EXIT_THREAD,
    )
}
