//! libnx `thread.h` C ABI adapter.
//!
//! Owns the libnx-compatible [`LibnxThread`] layout and re-exports the
//! ABI-visible [`ThreadContext`] register-dump type. Every `__nx_sys_thread__*`
//! symbol here is a thin wrapper that validates raw input, calls the idiomatic
//! Rust core, and maps the result back to a libnx `Result` code.
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/thread.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/kernel/thread.h)
//! - [switchbrew/libnx: switch/arm/thread_context.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/thread_context.h)
//!
//! Every adapter is wired to the core: the anchoring-free adapters
//! (`threadExit`, `threadGetCurHandle`, `threadTls{Alloc,Get,Set,Free}`,
//! `__libnx_init_thread`) call it directly, and the eight `ThreadControl`-anchored
//! lifecycle adapters (`threadCreate`/`Start`/`WaitForExit`/`Close`/`Pause`/
//! `Resume`/`DumpContext`/`GetSelf`) recover their pinned core object through
//! the [`SideRegistry`] keyed by the caller-owned `*mut LibnxThread`
//! (Resolved Question #5).
//!
//! # Concurrency contract
//!
//! The lifecycle adapters recover the pinned [`ThreadControl`] from
//! `LIBNX_REGISTRY` and then operate on it *after* the registry lock is
//! released. The same `*mut LibnxThread` must therefore not be passed to two
//! lifecycle adapters concurrently from different threads: a `threadClose`
//! racing any other adapter on one handle could free the core object mid-call.
//! This single-owner-at-a-time contract is inherited from libnx, whose
//! `Thread` struct is likewise not internally locked.

use alloc::boxed::Box;
use core::{
    ffi::{c_int, c_void},
    mem::offset_of,
    ptr::{NonNull, null_mut},
};

use nx_svc::error::{KernelError, ToResultCode as _};
/// Re-export of the ABI-visible AArch64 register-dump structure.
///
/// `nx-svc` already defines the `#[repr(C)]` layout that matches libnx's
/// `ThreadContext`; the libnx adapter exposes it under its own module path so
/// callers of `threadDumpContext` see a stable name.
pub use nx_svc::raw::ThreadContext;
use static_assertions::const_assert_eq;

use super::registry::SideRegistry;
use crate::{
    error::ToResultCode as _,
    thread::{self, Builder, CoreId, Priority, StackSpec, ThreadControl},
    tsd,
};

/// Anchors each libnx-created thread's caller-owned `*mut LibnxThread` to its
/// `Box`-pinned core [`ThreadControl`].
///
/// `threadCreate` inserts the entry and `threadClose` evicts it; the other
/// lifecycle adapters recover the pinned core by lookup. See [`SideRegistry`]
/// for why this is distinct from the live-thread registry.
static LIBNX_REGISTRY: SideRegistry<*mut LibnxThread, NonNull<ThreadControl>> = SideRegistry::new();

/// Raw libnx thread handle (`Handle` in `libnx`, a 32-bit kernel handle).
pub type Handle = u32;

/// Raw libnx result code (`Result` in `libnx`).
pub type ResultCode = u32;

/// Thread entry point, as passed to `threadCreate`.
pub type ThreadFunc = unsafe extern "C" fn(*mut c_void);

/// Thread-specific-data destructor, as passed to `threadTlsAlloc`.
pub type Destructor = unsafe extern "C" fn(*mut c_void);

/// ABI-compatible mirror of libnx's `Thread` structure.
///
/// This type exists purely to satisfy the libnx `thread.h` C ABI. The
/// authoritative runtime object is the Rust core's `ThreadControl`; the adapter
/// mirrors the ABI-visible fields here for C callers.
#[repr(C)]
pub struct LibnxThread {
    /// Thread handle.
    pub handle: Handle,
    /// Whether the stack memory is automatically allocated.
    pub owns_stack_mem: bool,
    /// Pointer to stack memory.
    pub stack_mem: *mut c_void,
    /// Pointer to stack memory mirror.
    pub stack_mirror: *mut c_void,
    /// Stack size.
    pub stack_sz: usize,
    /// Pointer to the thread's runtime TLS slot array.
    pub tls_array: *mut *mut c_void,
    /// Next thread in the live-thread list.
    pub next: *mut LibnxThread,
    /// Address of the previous link's `next` field.
    pub prev_next: *mut *mut LibnxThread,
}

// AArch64 C layout: Handle(4) + bool(1) + pad(3) + 6×ptr/usize(48) = 56 bytes.
const_assert_eq!(size_of::<LibnxThread>(), 56);
const_assert_eq!(offset_of!(LibnxThread, handle), 0);
const_assert_eq!(offset_of!(LibnxThread, owns_stack_mem), 4);
const_assert_eq!(offset_of!(LibnxThread, stack_mem), 8);
const_assert_eq!(offset_of!(LibnxThread, stack_mirror), 16);
const_assert_eq!(offset_of!(LibnxThread, stack_sz), 24);
const_assert_eq!(offset_of!(LibnxThread, tls_array), 32);
const_assert_eq!(offset_of!(LibnxThread, next), 40);
const_assert_eq!(offset_of!(LibnxThread, prev_next), 48);

/// Creates a thread (`threadCreate`).
///
/// # Safety
///
/// `t` must point to a writable, properly aligned `LibnxThread`. `stack_mem`,
/// if non-null, must be page-aligned and remain valid for the thread's
/// lifetime. `entry` must be a valid entry point for the thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_create(
    t: *mut LibnxThread,
    entry: Option<ThreadFunc>,
    arg: *mut c_void,
    stack_mem: *mut c_void,
    stack_sz: usize,
    prio: c_int,
    cpuid: c_int,
) -> ResultCode {
    // Edge validation: a null handle struct has nowhere to mirror state into,
    // and a null entry point cannot be run.
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(entry) = entry else {
        return KernelError::InvalidArgument.to_rc();
    };

    // Project the raw libnx scalars into the core creation config. libnx
    // `threadCreate` adopts a non-null `stack_mem` as the whole backing buffer
    // and auto-allocates otherwise; it never rounds the size, so `create` owns
    // every remaining check — page alignment, stack sizing, the SVC.
    let stack = match NonNull::new(stack_mem) {
        Some(base) => StackSpec::Provided {
            base,
            size: stack_sz,
        },
        None => StackSpec::Auto(stack_sz),
    };
    let config = Builder::new()
        .stack(stack)
        .priority(Priority::new(prio))
        .core_id(CoreId::new(cpuid))
        .build_create(entry, arg);
    // SAFETY: `entry` is non-null and `arg` is the caller's opaque argument;
    // `stack_mem`/`stack_sz` carry the C caller's stack contract straight
    // through to `create`'s matching `# Safety` clause.
    let control = match unsafe { thread::create(config) } {
        Ok(control) => control,
        Err(err) => return err.to_rc(),
    };

    // Mirror the ABI-visible fields into the caller's `LibnxThread` before the
    // core object is pinned. The thread is `Created` (suspended) and not yet
    // started, so reading these creation-fixed fields races no concurrent
    // writer. `tls_array`/`next`/`prev_next` stay null — the live-thread list
    // is the core's `thread_list` registry, not this mirror.
    // SAFETY: `t` is non-null per the edge check and, by the `# Safety`
    // contract, points to a writable, aligned `LibnxThread`.
    unsafe {
        (*t).handle = control.handle().to_raw();
        (*t).owns_stack_mem = control.owns_stack_mem();
        (*t).stack_mem = control.stack_mem().map_or(null_mut(), NonNull::as_ptr);
        (*t).stack_mirror = control.stack_mirror().as_ptr();
        (*t).stack_sz = control.stack_size();
        (*t).tls_array = null_mut();
        (*t).next = null_mut();
        (*t).prev_next = null_mut();
    }

    // Heap-pin the core object and anchor the caller's handle to it. The pinned
    // address is what `threadStart` wires the thread's back-pointers to; the
    // entry lives until `threadClose` evicts it.
    let pinned = Box::into_raw(Box::new(control));
    // SAFETY: `Box::into_raw` never yields a null pointer.
    let pinned = unsafe { NonNull::new_unchecked(pinned) };
    LIBNX_REGISTRY.insert(t, pinned);
    0
}

/// Starts a created thread (`threadStart`).
///
/// # Safety
///
/// `t` must point to a valid `LibnxThread` previously filled by
/// [`__nx_sys_thread__thread_create`]. The same handle must not be operated on
/// by another thread-lifecycle adapter concurrently — see the module-level
/// concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_start(t: *mut LibnxThread) -> ResultCode {
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    // Recover the pinned core object. A handle with no entry — never created,
    // or already `threadClose`d — fails the lookup instead of dereferencing a
    // dangling pointer.
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // Reject a re-start of an already-running thread: `start` re-wires the
    // thread's entry-args/TCB back-pointers, which would race a live thread.
    // A transient `&ThreadControl` for one atomic `state` read is sound (see
    // `thread::current`'s docs); a thread left `Created` by a failed
    // earlier `start` is not running and may still be retried.
    // SAFETY: `control` is the registry's pinned, live `ThreadControl` for `t`.
    if unsafe { control.as_ref() }.is_running() {
        return KernelError::Busy.to_rc();
    }
    // SAFETY: `control` is the `Box`-pinned `ThreadControl` `threadCreate`
    // produced and kept at this fixed address until `threadClose` evicts it —
    // exactly `start`'s pinning contract — and the not-running check above
    // rules out re-starting a live thread.
    match unsafe { thread::start(control) } {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Exits the current thread (`threadExit`).
///
/// # Safety
///
/// Must be called from a thread registered with the `nx-sys-thread` core. Does
/// not return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_exit() -> ! {
    // SAFETY: the libnx `threadExit` ABI imposes the same contract — called on
    // an `nx-sys-thread`-managed thread, as its final operation — that
    // `thread::exit` requires.
    unsafe { thread::exit() }
}

/// Waits for a thread to finish executing (`threadWaitForExit`).
///
/// # Safety
///
/// `t` must point to a valid `LibnxThread`. The same handle must not be
/// operated on by another thread-lifecycle adapter concurrently — see the
/// module-level concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_wait_for_exit(t: *mut LibnxThread) -> ResultCode {
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // SAFETY: `control` is the registry's `Box`-pinned `ThreadControl` for `t`,
    // kept valid for the whole wait by the still-present registry entry.
    match unsafe { thread::wait_for_exit(control) } {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Frees the resources of an exited thread (`threadClose`).
///
/// # Safety
///
/// `t` must point to a valid `LibnxThread` whose thread has already exited. The
/// same handle must not be operated on by another thread-lifecycle adapter
/// concurrently — see the module-level concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_close(t: *mut LibnxThread) -> ResultCode {
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // Reject a still-running thread *before* evicting it: `thread::close` frees
    // the `Box`-pinned `ThreadControl`, and a running thread still dereferences
    // that address (its registry links, its lifecycle writes). A transient
    // `&ThreadControl` for one atomic `state` read is sound — `state` is an
    // `AtomicU8` (see `thread::current`'s docs).
    // SAFETY: `control` is the registry's pinned, live `ThreadControl` for `t`.
    if unsafe { control.as_ref() }.is_running() {
        return KernelError::Busy.to_rc();
    }

    // Not running, so its stack is reclaimable: evict the entry and reconstitute
    // the `Box` `threadCreate` pinned.
    let Some(control) = LIBNX_REGISTRY.remove(t) else {
        // Lost a race with a concurrent `threadClose(t)`; that call owns the
        // reclamation.
        return KernelError::InvalidHandle.to_rc();
    };
    // SAFETY: `control` came from `Box::into_raw` in `threadCreate` and was
    // just evicted from the registry, so this reconstitutes and consumes that
    // `Box` exactly once; the thread is not running, so freeing its
    // `ThreadControl` aliases no live pointer.
    let control = *unsafe { Box::from_raw(control.as_ptr()) };
    match thread::close(control) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Pauses a thread (`threadPause`).
///
/// # Safety
///
/// `t` must point to a valid `LibnxThread`. The same handle must not be
/// operated on by another thread-lifecycle adapter concurrently — see the
/// module-level concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_pause(t: *mut LibnxThread) -> ResultCode {
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // SAFETY: `control` is the registry's `Box`-pinned `ThreadControl` for `t`,
    // valid for the call.
    match unsafe { thread::pause(control) } {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Resumes a paused thread (`threadResume`).
///
/// # Safety
///
/// `t` must point to a valid `LibnxThread`. The same handle must not be
/// operated on by another thread-lifecycle adapter concurrently — see the
/// module-level concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_resume(t: *mut LibnxThread) -> ResultCode {
    if t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // SAFETY: `control` is the registry's `Box`-pinned `ThreadControl` for `t`,
    // valid for the call.
    match unsafe { thread::resume(control) } {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Dumps the registers of a paused thread (`threadDumpContext`).
///
/// # Safety
///
/// `ctx` must point to a writable, properly aligned `ThreadContext`, and `t`
/// must point to a valid `LibnxThread`. The same handle must not be operated on
/// by another thread-lifecycle adapter concurrently — see the module-level
/// concurrency contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_dump_context(
    ctx: *mut ThreadContext,
    t: *mut LibnxThread,
) -> ResultCode {
    if ctx.is_null() || t.is_null() {
        return KernelError::InvalidPointer.to_rc();
    }
    let Some(control) = LIBNX_REGISTRY.get(t) else {
        return KernelError::InvalidHandle.to_rc();
    };
    // SAFETY: `control` is the registry's `Box`-pinned `ThreadControl` for `t`,
    // valid for the call.
    let context = match unsafe { thread::dump_context(control) } {
        Ok(context) => context,
        Err(err) => return err.to_rc(),
    };
    // SAFETY: `ctx` is non-null per the edge check and, by the `# Safety`
    // contract, points to a writable, aligned `ThreadContext`.
    unsafe { ctx.write(context) };
    0
}

/// Returns the current thread structure (`threadGetSelf`).
///
/// Reverse-maps the calling thread's core [`ThreadControl`] back to the libnx
/// handle it was registered under. Returns null when the caller is not a thread
/// created through `threadCreate` — the main thread, a pthread-created thread,
/// or a Level-1 `spawn` — since no `LibnxThread` exists for it; libnx instead
/// returns whatever `ThreadVars.thread_ptr` holds. This is a recorded
/// divergence from libnx's observable behavior, which the FFI override symbols
/// otherwise reproduce.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__thread_get_self() -> *mut LibnxThread {
    let Some(current) = thread::current() else {
        return null_mut();
    };
    LIBNX_REGISTRY
        .find_key(|control| *control == current)
        .unwrap_or_default()
}

/// Returns the current thread's raw handle (`threadGetCurHandle`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__thread_get_cur_handle() -> Handle {
    thread::get_current_handle().to_raw()
}

/// Allocates a runtime TLS slot (`threadTlsAlloc`).
///
/// Returns the slot id, or a negative value on failure.
///
/// # Safety
///
/// `destructor`, if provided, must be safe to invoke on thread exit with the
/// slot's stored value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_tls_alloc(destructor: Option<Destructor>) -> i32 {
    match tsd::alloc(destructor) {
        // A slot id is below `NUM_TSD_KEYS` (128), well within `i32` range.
        Ok(key) => key.to_raw() as i32,
        // libnx `threadTlsAlloc` reports exhaustion as a negative slot id.
        Err(tsd::TsdAllocError::NoSlotsAvailable) => -1,
    }
}

/// Reads the current thread's value for a TLS slot (`threadTlsGet`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__thread_tls_get(slot_id: i32) -> *mut c_void {
    // Validate the raw slot id once at the edge; an out-of-range id reads back
    // as a null value, matching libnx `threadTlsGet`.
    match tsd::TsdKey::from_raw(slot_id as u32) {
        Some(key) => tsd::get(key),
        None => null_mut(),
    }
}

/// Stores a value into the current thread's TLS slot (`threadTlsSet`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__thread_tls_set(slot_id: i32, value: *mut c_void) {
    // `threadTlsSet` returns `void`; an out-of-range slot id is silently
    // ignored, matching libnx.
    if let Some(key) = tsd::TsdKey::from_raw(slot_id as u32) {
        tsd::set(key, value);
    }
}

/// Frees a runtime TLS slot (`threadTlsFree`).
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_thread__thread_tls_free(slot_id: i32) {
    // `threadTlsFree` returns `void`; an invalid or unallocated slot id is
    // silently ignored, matching libnx.
    if let Some(key) = tsd::TsdKey::from_raw(slot_id as u32) {
        let _ = tsd::free(key);
    }
}

/// Initializes the main thread (`__libnx_init_thread`).
///
/// # Safety
///
/// Must be called exactly once, on the main thread, during runtime startup
/// before any other thread API is used.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__libnx_init_thread() {
    // SAFETY: `__libnx_init_thread` is invoked exactly once, on the main
    // thread, during runtime startup before any other thread API runs —
    // exactly `thread::init_main_thread`'s contract.
    unsafe { thread::init_main_thread() }
}
