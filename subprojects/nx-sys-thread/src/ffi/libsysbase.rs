//! devkitPro/libsysbase newlib thread syscall C ABI adapter.
//!
//! Owns the newlib `struct __pthread_t`-compatible [`LibsysbasePthread`] layout
//! and the `__syscall_thread_*`, `__syscall_tls_*`, `__syscall_nanosleep`,
//! `sched_yield`, and `sched_getcpu` override symbols. Each wrapper validates
//! raw input, calls the idiomatic Rust core, and returns errno-style statuses.
//!
//! # References
//!
//! - [switchbrew/libnx: source/runtime/newlib.c](https://github.com/switchbrew/libnx/blob/master/nx/source/runtime/newlib.c)
//! - libgloss/libsysbase/syscall_support.c
//!
//! Every adapter is wired to the core: the TSD-key, `nanosleep`, and scheduler
//! adapters call it directly, and the `__syscall_thread_*` pthread-lifecycle
//! adapters anchor each caller-owned `*mut LibsysbasePthread` to its
//! `Arc`-shared pthread core through the [`SideRegistry`] (Resolved Question #5).

use alloc::boxed::Box;
use core::{
    ffi::{
        c_int,
        c_long,
        c_void,
    },
    mem::offset_of,
    ptr::{
        NonNull,
        null_mut,
    },
    time::Duration,
};

use static_assertions::const_assert_eq;

use super::{
    libnx::{
        Destructor,
        LibnxThread,
    },
    reent,
    registry::SideRegistry,
};
use crate::{
    pthread,
    thread::{
        self,
        StackSpec,
    },
    tsd,
};

/// POSIX error: invalid argument.
const EINVAL: c_int = 22;

/// POSIX error: resource temporarily unavailable (no free TSD key slot, or a
/// transient resource shortage creating a thread).
const EAGAIN: c_int = 11;

/// POSIX error: out of memory — a pthread creation failure, matching libnx
/// `__syscall_thread_create`'s `ENOMEM` returns.
const ENOMEM: c_int = 12;

/// Process exit status libnx routes a main-thread `pthread_exit` through
/// (`exit(EXIT_FAILURE)`).
const EXIT_FAILURE: c_int = 1;

/// Nanoseconds in one second — the `Timespec::tv_nsec` upper bound.
const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Sentinel pthread handle for the kernel-created main thread.
///
/// libnx's `newlib.c` uses `(struct __pthread_t*)~0` (`THRD_MAIN_HANDLE`): the
/// main thread has no heap-allocated `__pthread_t`, so `__syscall_thread_self`
/// reports this value and `__syscall_thread_join`/`detach` recognize it. It can
/// never collide with a real `Box`-allocated handle.
const MAIN_THREAD_SENTINEL: *mut LibsysbasePthread = usize::MAX as *mut LibsysbasePthread;

/// Anchors each pthread-created thread's caller-owned `*mut LibsysbasePthread`
/// to its `Arc`-shared [`PthreadJoinHandle`](crate::pthread::PthreadJoinHandle).
///
/// `__syscall_thread_create` inserts the entry; `__syscall_thread_join` and
/// `__syscall_thread_detach` evict and consume it. See [`SideRegistry`] for why
/// this is distinct from the live-thread registry.
static LIBSYSBASE_REGISTRY: SideRegistry<*mut LibsysbasePthread, pthread::PthreadJoinHandle> =
    SideRegistry::new();

// SAFETY: `exit` is the newlib process-exit symbol provided by the linked
// libsysbase/newlib archive; it runs `atexit` handlers and never returns.
unsafe extern "C" {
    fn exit(code: c_int) -> !;
}

/// pthread thread routine, as passed to `__syscall_thread_create`.
pub type PthreadFunc = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

/// ABI-compatible mirror of newlib/libsysbase's `struct __pthread_t`.
///
/// Layout: `{ Thread thr; void *rc; }` — a libnx [`LibnxThread`] followed by the
/// thread's stored return value.
#[repr(C)]
pub struct LibsysbasePthread {
    /// Embedded libnx thread structure.
    pub thr: LibnxThread,
    /// Thread return value, stored by `pthread_exit` and read by `pthread_join`.
    pub return_value: *mut c_void,
}

// AArch64 C layout: LibnxThread(56) + ptr(8) = 64 bytes.
const_assert_eq!(size_of::<LibsysbasePthread>(), 64);
const_assert_eq!(offset_of!(LibsysbasePthread, thr), 0);
const_assert_eq!(offset_of!(LibsysbasePthread, return_value), 56);

/// ABI-compatible mirror of newlib's `struct timespec`.
#[repr(C)]
pub struct Timespec {
    /// Whole seconds.
    pub tv_sec: c_long,
    /// Nanoseconds within the second.
    pub tv_nsec: c_long,
}

// AArch64 C layout: 2×long(8) = 16 bytes.
const_assert_eq!(size_of::<Timespec>(), 16);
const_assert_eq!(offset_of!(Timespec, tv_sec), 0);
const_assert_eq!(offset_of!(Timespec, tv_nsec), 8);

/// Returns the current thread's pthread handle (`__syscall_thread_self`).
///
/// The kernel-created main thread has no heap `LibsysbasePthread`, so it is
/// reported as [`MAIN_THREAD_SENTINEL`] rather than via container-of recovery
/// on a thread with no enclosing pthread core (Resolved Question #5). A thread
/// not created through `__syscall_thread_create` — a libnx `threadCreate`
/// thread, a Level-1 `spawn` — likewise has no entry and is reported as the
/// sentinel rather than a bogus handle.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__syscall_thread_self() -> *mut LibsysbasePthread {
    if thread::is_main_thread() {
        return MAIN_THREAD_SENTINEL;
    }
    let Some(current) = thread::current() else {
        // No core state at all — not an `nx-sys-thread`-managed thread.
        return MAIN_THREAD_SENTINEL;
    };
    // Reverse-map the running `ThreadControl` to the C handle it was registered
    // under in `__syscall_thread_create`.
    LIBSYSBASE_REGISTRY
        .find_key(|handle| handle.thread_control_ptr() == current)
        .unwrap_or(MAIN_THREAD_SENTINEL)
}

/// Stores a return value and exits the current thread (`__syscall_thread_exit`).
///
/// # Safety
///
/// Must be called from a pthread-created thread registered with the
/// `nx-sys-thread` core. Does not return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_thread_exit(value: *mut c_void) -> ! {
    if thread::is_main_thread() {
        // libnx routes a main-thread `pthread_exit` through `exit(EXIT_FAILURE)`;
        // the main thread has no enclosing `PthreadControl`, so `pthread_exit`'s
        // container-of recovery must not run on it.
        // SAFETY: `exit` is the newlib process-exit symbol; it never returns.
        unsafe { exit(EXIT_FAILURE) }
    }
    // Not the main thread, so the caller is a pthread-created thread —
    // `pthread_exit`'s container-of contract holds.
    // SAFETY: runs on a pthread-created thread as its final operation;
    // `pthread_exit` stores `value` and tears the thread down, never returning.
    unsafe { pthread::pthread_exit(value) }
}

/// Creates and starts a pthread-style thread (`__syscall_thread_create`).
///
/// # Safety
///
/// `thread` must point to a writable `*mut LibsysbasePthread`. `stack_addr`, if
/// non-null, must be a valid stack region for the thread's lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_thread_create(
    thread: *mut *mut LibsysbasePthread,
    func: Option<PthreadFunc>,
    arg: *mut c_void,
    stack_addr: *mut c_void,
    stack_size: usize,
) -> c_int {
    /// Page-alignment mask: a page is `0x1000` bytes.
    const PAGE_MASK: usize = 0xFFF;

    // Edge validation: a null out-pointer has nowhere to publish the handle,
    // and a null routine cannot be run.
    if thread.is_null() {
        return EINVAL;
    }
    let Some(func) = func else {
        return EINVAL;
    };
    // libnx `__syscall_thread_create` rejects a misaligned stack pointer.
    if (stack_addr as usize) & PAGE_MASK != 0 {
        return EINVAL;
    }
    // Resolve how the thread's stack is sourced. libnx substitutes 128 KiB for
    // a zero stack size and *rejects* a non-page-aligned non-zero size;
    // `nx-sys-thread` rounds a non-zero size up instead (the rev 53 F5
    // behavior) — but only for the auto-allocate path, where `thread::create`
    // owns the backing buffer (newlib's default pthread stack size is not
    // page-aligned). A caller-provided buffer is never rounded: rounding its
    // size up would make `thread::create` map and write the per-thread control
    // regions past the buffer's end, so a non-page-aligned size is rejected
    // with `EINVAL`, matching libnx's `__syscall_thread_create`.
    let stack = match (NonNull::new(stack_addr), stack_size) {
        (None, 0) => StackSpec::Auto(thread::DEFAULT_STACK_SIZE),
        (None, size) => match size.checked_add(PAGE_MASK) {
            Some(rounded) => StackSpec::Auto(rounded & !PAGE_MASK),
            // A near-`usize::MAX` request cannot be page-rounded.
            None => return EINVAL,
        },
        (Some(_), size) if size & PAGE_MASK != 0 => return EINVAL,
        (Some(base), size) => StackSpec::Provided { base, size },
    };

    let config = pthread::PthreadCreateConfig::new(func, arg, stack);
    // SAFETY: `func` is a non-null pthread routine and `arg` its opaque
    // argument; `stack` carries the caller's stack contract straight through to
    // `pthread_create`'s matching `# Safety` clause.
    let handle = match unsafe { pthread::pthread_create(config) } {
        Ok(handle) => handle,
        // libnx reports a creation failure as `ENOMEM`.
        Err(_) => return ENOMEM,
    };

    // Allocate the caller-owned C handle. Its embedded `thr` is ABI padding the
    // newlib pthread layer never reads — the authoritative state is the
    // `Arc`-shared core the side registry holds — so it is left zeroed.
    let pt = Box::into_raw(Box::new(LibsysbasePthread {
        thr: LibnxThread {
            handle: 0,
            owns_stack_mem: false,
            stack_mem: null_mut(),
            stack_mirror: null_mut(),
            stack_sz: 0,
            tls_array: null_mut(),
            next: null_mut(),
            prev_next: null_mut(),
        },
        return_value: null_mut(),
    }));
    LIBSYSBASE_REGISTRY.insert(pt, handle);

    // SAFETY: `thread` is non-null per the edge check and, by the `# Safety`
    // contract, points to a writable `*mut LibsysbasePthread`.
    unsafe { thread.write(pt) };
    0
}

/// Joins a pthread-style thread and returns its value (`__syscall_thread_join`).
///
/// # Safety
///
/// `thread` must point to a valid `LibsysbasePthread` created by
/// [`__nx_sys_thread__syscall_thread_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_thread_join(
    thread: *mut LibsysbasePthread,
) -> *mut c_void {
    // libnx returns NULL for joining the main-thread sentinel; a null or stale
    // handle is treated the same — never dereferenced.
    if thread.is_null() || thread == MAIN_THREAD_SENTINEL {
        return null_mut();
    }
    let Some(handle) = LIBSYSBASE_REGISTRY.remove(thread) else {
        // A handle with no entry — never created, or already joined/detached.
        return null_mut();
    };

    let value = match pthread::pthread_join(handle) {
        Ok(value) => value,
        // The thread exited and recorded its return value before `close`
        // failed, so the recorded value is still carried out (the stack
        // mapping/handle leak — `close`'s documented failure behavior).
        Err(pthread::PthreadJoinError::Close { value, .. }) => value,
        // The termination wait failed (effectively unreachable for an unbounded
        // wait on a valid handle) — no return value can be recovered.
        Err(pthread::PthreadJoinError::Wait(_)) => null_mut(),
    };

    // Reclaim the C handle struct allocated in `__syscall_thread_create`.
    // SAFETY: `thread` came from `Box::into_raw` there and was just evicted
    // from the registry, so this reconstitutes and frees that `Box` once.
    drop(unsafe { Box::from_raw(thread) });
    value
}

/// Detaches a pthread-style thread (`__syscall_thread_detach`).
///
/// The vendored libnx leaves `__syscall_thread_detach` unimplemented, but the
/// `nx-sys-thread` core has a real `pthread_detach`, so this adapter wires to
/// it: a detached thread self-reclaims via the Horizon `__unmapself` port once
/// it exits. Detachment is infallible, so this always returns success.
///
/// # Safety
///
/// `thread` must point to a valid `LibsysbasePthread`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_thread_detach(
    thread: *mut LibsysbasePthread,
) -> c_int {
    // Detaching the main-thread sentinel, a null, or a stale handle is invalid.
    if thread.is_null() || thread == MAIN_THREAD_SENTINEL {
        return EINVAL;
    }
    let Some(handle) = LIBSYSBASE_REGISTRY.remove(thread) else {
        return EINVAL;
    };

    pthread::pthread_detach(handle);

    // The caller relinquishes the handle on detach, so reclaim the C handle
    // struct now; the `Arc`-shared core is reclaimed by the detached thread
    // itself once it exits.
    // SAFETY: `thread` came from `Box::into_raw` in `__syscall_thread_create`
    // and was just evicted from the registry, so this frees that `Box` once.
    drop(unsafe { Box::from_raw(thread) });
    0
}

/// Creates a pthread TLS key (`__syscall_tls_create`).
///
/// # Safety
///
/// `key` must point to a writable `u32`. `destructor`, if provided, must be
/// safe to invoke on thread exit with the key's stored value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_tls_create(
    key: *mut u32,
    destructor: Option<Destructor>,
) -> c_int {
    // Edge validation: a null out-pointer has nowhere to publish the key.
    if key.is_null() {
        return EINVAL;
    }
    match tsd::alloc(destructor) {
        Ok(tsd_key) => {
            // SAFETY: `key` is non-null per the check above and, by the
            // `# Safety` contract, points to a writable `u32`.
            unsafe { key.write(tsd_key.to_raw()) };
            0
        }
        // newlib maps key-table exhaustion to `EAGAIN`.
        Err(tsd::TsdAllocError) => EAGAIN,
    }
}

/// Stores a value for a pthread TLS key (`__syscall_tls_set`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__syscall_tls_set(key: u32, value: *const c_void) -> c_int {
    // Validate the raw `u32` key once at the edge into a `TsdKey`.
    match tsd::TsdKey::from_raw(key) {
        Some(tsd_key) => {
            tsd::set(tsd_key, value.cast_mut());
            0
        }
        None => EINVAL,
    }
}

/// Reads the current thread's value for a pthread TLS key (`__syscall_tls_get`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__syscall_tls_get(key: u32) -> *mut c_void {
    // An out-of-range key resolves to a null value, matching newlib.
    match tsd::TsdKey::from_raw(key) {
        Some(tsd_key) => tsd::get(tsd_key),
        None => null_mut(),
    }
}

/// Deletes a pthread TLS key (`__syscall_tls_delete`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__syscall_tls_delete(key: u32) -> c_int {
    match tsd::TsdKey::from_raw(key) {
        Some(tsd_key) => match tsd::free(tsd_key) {
            Ok(()) => 0,
            Err(tsd::TsdFreeError) => EINVAL,
        },
        None => EINVAL,
    }
}

/// Suspends the current thread for a duration (`__syscall_nanosleep`).
///
/// On a failure path returns `-1` with `errno = EINVAL`, matching libnx's
/// `__syscall_nanosleep` (`newlib.c`). libnx only rejects a null `req`; this
/// adapter additionally rejects an out-of-range `timespec` (`tv_sec < 0`, or
/// `tv_nsec` outside `0..1e9`) — a deliberate POSIX-correctness divergence,
/// since folding garbage values into a `Duration` would request an absurd
/// sleep. Both rejections report `EINVAL`, the POSIX `nanosleep` error.
///
/// # Safety
///
/// `req` must point to a valid `Timespec`. `rem`, if non-null, must point to a
/// writable `Timespec`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__syscall_nanosleep(
    req: *const Timespec,
    rem: *mut Timespec,
) -> c_int {
    // Edge validation: `req` must be a readable, well-formed `timespec`.
    if req.is_null() {
        reent::set_errno(EINVAL);
        return -1;
    }
    // SAFETY: `req` is non-null per the check above and, by the `# Safety`
    // contract, points to a valid `Timespec`.
    let req = unsafe { &*req };
    // POSIX requires `tv_sec >= 0` and `tv_nsec` in `0..1_000_000_000`.
    if req.tv_sec < 0 || req.tv_nsec < 0 || (req.tv_nsec as u64) >= NANOS_PER_SEC {
        reent::set_errno(EINVAL);
        return -1;
    }

    // Fold the validated `timespec` into a `Duration`. `tv_sec`/`tv_nsec` are
    // non-negative and `tv_nsec < NANOS_PER_SEC` per the checks above, so the
    // casts are lossless and `Duration::new` carries no nanosecond overflow.
    let dur = Duration::new(req.tv_sec as u64, req.tv_nsec as u32);
    thread::sleep(dur);

    // The sleep always runs to completion, so no time remains; zero `rem` when
    // the caller asked for it, matching libnx.
    if !rem.is_null() {
        // SAFETY: `rem` is non-null per the check and, by the `# Safety`
        // contract, points to a writable `Timespec`.
        unsafe {
            rem.write(Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            });
        }
    }
    0
}

/// Yields the current thread's remaining time slice (`sched_yield`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__sched_yield() -> c_int {
    thread::yield_thread();
    0
}

/// Returns the processor number the current thread is running on (`sched_getcpu`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__sched_getcpu() -> c_int {
    // The Switch has four cores, so the id always fits a `c_int`.
    thread::get_current_cpu() as c_int
}
