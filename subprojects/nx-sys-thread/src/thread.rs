//! Idiomatic core thread state.
//!
//! [`ThreadControl`] is the Rust-owned, authoritative runtime object for a
//! Horizon thread created or adopted by `nx-sys-thread`. It owns the kernel
//! handle, the stack-mapping metadata, the per-thread runtime TLS state
//! ([`ThreadRuntime`]), and the thread's lifecycle [`ThreadState`].
//!
//! # Core vs. ABI adapter
//!
//! `ThreadControl` is deliberately *not* shaped around any C ABI: its field
//! set, ordering, and types are chosen for the Rust core alone. libnx
//! `thread.h` layout compatibility is the separate concern of the
//! `ffi::libnx` adapter's `#[repr(C)]` `LibnxThread`, which mirrors the
//! ABI-visible subset of this state for C callers. C callers never observe a
//! `ThreadControl` directly.

use alloc::{
    alloc::{
        Layout,
        alloc_zeroed,
        dealloc,
    },
    boxed::Box,
    sync::Arc,
    vec::Vec,
};
use core::{
    cell::UnsafeCell,
    ffi::c_void,
    mem::{
        MaybeUninit,
        offset_of,
    },
    num::NonZeroU64,
    ptr::{
        self,
        NonNull,
    },
    sync::atomic::{
        AtomicPtr,
        AtomicU8,
        AtomicU64,
        Ordering,
    },
    time::Duration,
};

#[cfg(feature = "ffi")]
use nx_svc::error::{
    KernelError,
    ResultCode,
    ToResultCode as _,
};
use nx_svc::{
    mem::{
        UnmapMemoryError,
        query_memory,
        unmap_memory,
    },
    raw::ThreadContext,
    result::Error,
    sync::{
        MAX_WAIT_HANDLES,
        WaitSyncError,
        wait_synchronization_multiple,
        wait_synchronization_single,
    },
    thread::{
        CloseHandleError,
        CreateThreadError,
        GetContext3Error,
        Handle,
        PauseThreadError,
        ResumeThreadError,
        StartThreadError,
        close_handle,
    },
};
use nx_sys_mem::{
    buf::BufferRef,
    stack::{
        self,
        MapError,
    },
};
use nx_sys_thread_tls::{
    ReentPtr,
    ThreadInfoPtr,
    ThreadPointer,
};
use static_assertions::const_assert_eq;

#[cfg(feature = "ffi")]
use crate::error::{
    _sealed,
    ToResultCode,
};
use crate::{
    detach::{
        self,
        DetachState,
        Detachable,
    },
    tcb::Tcb,
    thread_list,
    tls_block,
    tsd::{
        self,
        NUM_TSD_KEYS,
        ThreadRuntime,
    },
};

/// Horizon OS memory page size, in bytes.
///
/// Stack allocations and the mapped stack mirror are page-aligned, matching the
/// kernel's `svcMapMemory` granularity.
const PAGE_SIZE: usize = 0x1000;

/// Bytes reserved for a spawned thread's newlib `_reent` block.
///
/// The non-FFI build links no newlib and runs spawned threads with a null
/// `reent` pointer, so it reserves nothing. The `ffi` build provisions a real
/// per-thread `_reent` ([`ffi::reent`](crate::ffi::reent)); its size is the
/// devkitA64 newlib ABI value `(sizeof(struct _reent) + 0xF) & ~0xF`, sourced
/// from the C shim so the layout can never transcribe — and drift from — the
/// ABI.
#[cfg(not(feature = "ffi"))]
fn reent_size() -> usize {
    0
}

#[cfg(feature = "ffi")]
fn reent_size() -> usize {
    crate::ffi::reent::block_size()
}

/// Entry-point function for a Horizon thread.
///
/// Receives the single `*mut c_void` argument supplied at thread creation. The
/// crate's entry wrapper invokes it and tears the thread down once it returns.
pub type ThreadFunc = unsafe extern "C" fn(*mut c_void);

/// A process-unique, never-recycled identifier for a thread.
///
/// A Horizon kernel [`Handle`] is recycled after `svcCloseHandle`, so a stashed
/// handle can later compare equal to an unrelated thread. A `ThreadId` is
/// assigned once at thread creation from a monotonic counter and is never
/// reused for the lifetime of the process, so it is the sound basis for thread
/// identity comparisons: two `ThreadId`s are equal iff they name the same
/// thread. Mirrors `std::thread::ThreadId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(NonZeroU64);

impl ThreadId {
    /// Allocates the next never-recycled `ThreadId` from the process-wide
    /// monotonic counter.
    ///
    /// # Panics
    ///
    /// Panics if the counter is exhausted — only reachable after `u64::MAX`
    /// thread creations, i.e. never in practice.
    fn next() -> Self {
        /// Process-wide monotonic thread counter; the first thread gets id 1.
        static COUNTER: AtomicU64 = AtomicU64::new(1);

        // `fetch_add` returns the pre-increment value, so ids are strictly
        // increasing from 1. `Relaxed` suffices: the counter establishes no
        // happens-before, only per-thread uniqueness.
        let raw = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroU64::new(raw).expect("nx-sys-thread: ThreadId counter exhausted"))
    }

    /// Returns the raw `u64` identity value.
    ///
    /// Lets a C ABI adapter stash and compare a thread's identity where it
    /// cannot carry a typed `ThreadId`.
    pub const fn as_u64(self) -> u64 {
        self.0.get()
    }
}

/// Lifecycle state of a [`ThreadControl`].
///
/// A thread is [`Created`](ThreadState::Created) from [`create`] until [`start`]
/// makes it runnable, [`Running`](ThreadState::Running) while it executes, and
/// [`Exited`](ThreadState::Exited) once the thread-exit path has run.
/// `nx-sys-thread` never reuses a `ThreadControl` across lifecycles, so these
/// three states fully describe the object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThreadState {
    /// The thread has been created and suspended, but [`start`] has not yet
    /// made it runnable. Its stack is mapped but not in use.
    Created = 0,
    /// The thread has been started and has not yet run its exit path.
    Running = 1,
    /// The thread has run its exit path and released its runtime state.
    Exited = 2,
}

/// Authoritative, Rust-owned runtime object for a single Horizon thread.
///
/// Owns the kernel [`Handle`], the stack-mapping metadata, the per-thread
/// [`ThreadRuntime`] (runtime TLS slots), and the [`ThreadState`] lifecycle.
/// The thread-creation path constructs it; the `ffi::libnx` adapter mirrors the
/// ABI-visible subset of these fields into its `#[repr(C)]` `LibnxThread`.
#[derive(Debug)]
pub struct ThreadControl {
    /// Process-unique, never-recycled identity for this thread.
    ///
    /// Assigned once at creation. Identity comparisons use this instead of the
    /// recyclable kernel [`handle`](Self::handle), which Horizon reuses after
    /// `svcCloseHandle`.
    id: ThreadId,
    /// Horizon kernel handle for this thread.
    handle: Handle,
    /// Whether `nx-sys-thread` allocated the stack memory and must free it.
    owns_stack_mem: bool,
    /// Pointer to the stack allocation backing this thread, or `None` when no
    /// backing pointer is tracked (the main thread, whose stack is owned by the
    /// kernel).
    stack_mem: Option<NonNull<c_void>>,
    /// Virtual mirror of the stack mapped for the kernel.
    stack_mirror: NonNull<c_void>,
    /// Usable stack size in bytes.
    stack_size: usize,
    /// Per-thread runtime TLS state, stored immediately after the thread's flat
    /// TSD slot array in `nx-sys-thread`-owned memory.
    ///
    /// Fixed at creation and never mutated afterward. Exposed `pub(crate)` so
    /// the teardown path ([`exit`]) can read it through a raw `ThreadControl`
    /// pointer; the [`runtime`](Self::runtime) accessor covers callers that hold
    /// the thread by reference.
    pub(crate) runtime: NonNull<ThreadRuntime>,
    /// Current lifecycle state, encoded as a [`ThreadState`] discriminant.
    ///
    /// The owning thread transitions this to [`Exited`](ThreadState::Exited) on
    /// its exit path with no lock held, concurrently with observers running on
    /// other threads ([`is_running`](Self::is_running)). It is therefore an
    /// [`AtomicU8`]: the interior mutability keeps a shared `&ThreadControl`
    /// sound across that lock-free self-mutation, which a plain field
    /// would not.
    state: AtomicU8,
    /// Previous thread in the process-wide live-thread registry.
    ///
    /// This link is *not* part of `ThreadControl`'s own state — the struct
    /// merely hosts the storage for the intrusive list. The
    /// [`thread_list`] registry owns it: it is only read or
    /// written while that module's `THREAD_MUTEX` is held, and no other code
    /// touches it. Hence, `pub(crate)` with no accessor.
    ///
    /// An [`AtomicPtr`] rather than a plain pointer: the registry mutates these
    /// links during the thread's own bring-up and teardown, concurrently with
    /// observers holding a shared `&ThreadControl`, so the interior mutability
    /// keeps that reference sound. `THREAD_MUTEX`, not the atomic
    /// ordering, supplies all happens-before — registry accesses use
    /// [`Relaxed`](Ordering::Relaxed).
    pub(crate) prev: AtomicPtr<ThreadControl>,
    /// Next thread in the process-wide live-thread registry.
    ///
    /// See [`prev`](Self::prev) — same registry ownership, locking, and
    /// [`AtomicPtr`] rationale.
    pub(crate) next: AtomicPtr<ThreadControl>,
}

impl ThreadControl {
    /// Returns this thread's process-unique [`ThreadId`].
    ///
    /// Unlike [`handle`](Self::handle), a `ThreadId` is never recycled, so it is
    /// the sound basis for identity comparisons.
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Returns the Horizon kernel handle for this thread.
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// Returns `true` while the thread is running: [`start`] has made it
    /// runnable, and it has not yet run its exit path.
    ///
    /// Returns `false` both before [`start`] (state
    /// [`Created`](ThreadState::Created)) and after the exit path (state
    /// [`Exited`](ThreadState::Exited)).
    pub fn is_running(&self) -> bool {
        // `state` is `Release`-stored by `start` (the `Created`/`Running`
        // transitions) and by `exit` (the final `Exited` store); this `Acquire`
        // load pairs with whichever store happened-before. The owning thread
        // mutates `state` lock-free, so this load may run concurrently with
        // that write.
        self.state.load(Ordering::Acquire) == ThreadState::Running as u8
    }

    /// Returns whether `nx-sys-thread` owns (and must free) the stack memory.
    pub fn owns_stack_mem(&self) -> bool {
        self.owns_stack_mem
    }

    /// Returns the pointer to the backing stack allocation, if one is tracked.
    pub fn stack_mem(&self) -> Option<NonNull<c_void>> {
        self.stack_mem
    }

    /// Returns the virtual mirror of the thread's stack.
    pub fn stack_mirror(&self) -> NonNull<c_void> {
        self.stack_mirror
    }

    /// Returns the thread's stack size in bytes.
    ///
    /// For a [`create`]d thread this is the *usable* stack extent — the mapped
    /// region minus the TLS/TCB/control-block reservation at the top. For the
    /// main thread, whose kernel-owned stack is discovered via `svcQueryMemory`,
    /// this is the *whole* mapping the kernel reports, matching libnx
    /// `__libnx_init_thread`.
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    /// Returns the per-thread [`ThreadRuntime`] (runtime TLS slot state).
    pub fn runtime(&self) -> NonNull<ThreadRuntime> {
        self.runtime
    }
}

/// A Horizon thread scheduling priority.
///
/// Horizon priorities span `0..=0x3F`, where a *lower* numeric value is a
/// *higher* scheduling priority. The wrapped value is forwarded verbatim to
/// `svcCreateThread`, which is the authority that rejects an out-of-range or
/// process-disallowed priority. This newtype exists to keep the priority
/// distinct from the equally-`i32`-typed [`CoreId`] in [`ThreadCreateConfig`]
/// and [`SpawnConfig`] — so the two cannot be transposed at a call site — not
/// to re-validate what the kernel already checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority(i32);

impl Priority {
    /// Default Horizon priority for a thread built without an explicit one.
    ///
    /// `0x3B` is the priority libnx's thread shims pass to `svcCreateThread`
    /// (`threadCreate` in `__syscall_thread_create`, `newlib.c:190`) — the
    /// value that enables preemptive multithreading on cores 0–2. [`Builder`]
    /// applies it so a Level-1 caller need not pick a priority itself.
    pub const DEFAULT: Self = Self(0x3B);

    /// Wraps a raw Horizon priority value.
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    /// Returns the raw priority value for the `svcCreateThread` ABI.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

/// The CPU core a new thread is assigned to.
///
/// Horizon's quad-core processor exposes cores `0..=3`; the sentinel `-2`
/// selects the process default core, which libnx's thread shims pass for "no
/// explicit affinity". Like [`Priority`], the wrapped value reaches
/// `svcCreateThread` verbatim — the kernel rejects a core outside the process
/// affinity mask — so this newtype is a type-distinctness wrapper, not a
/// validating one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreId(i32);

impl CoreId {
    /// The process default core (`-2`): the kernel picks the core per the
    /// process's default core configuration.
    pub const PROCESS_DEFAULT: Self = Self(-2);

    /// Wraps a raw Horizon core id.
    pub const fn new(core: i32) -> Self {
        Self(core)
    }

    /// Returns the raw core id for the `svcCreateThread` ABI.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

/// How a new thread's stack memory is sourced.
///
/// Replaces the former "optional stack buffer + separate size" pair: the
/// [`Provided`](StackSpec::Provided) variant carries the caller's buffer *and
/// that buffer's own size* together, so a provided buffer can never be paired
/// with a size that describes a different region. [`create`] requires every
/// size here to be page-aligned.
#[derive(Debug, Clone, Copy)]
pub enum StackSpec {
    /// [`create`] allocates the page-aligned backing buffer itself; the field
    /// is the requested usable stack size, before the per-thread control
    /// regions reserved on top of it.
    Auto(usize),
    /// The caller supplies the whole backing buffer; [`create`] maps and lays
    /// out exactly `size` bytes from `base`, never rounding past the buffer.
    Provided {
        /// Page-aligned start of the caller-owned buffer.
        base: NonNull<c_void>,
        /// Exact buffer length in bytes; must be page-aligned.
        size: usize,
    },
}

/// Parameters for creating a new Horizon thread.
///
/// Bundles everything [`create`] needs: the entry point and its argument, how
/// the stack is sourced ([`StackSpec`]), and Horizon scheduling parameters.
pub struct ThreadCreateConfig {
    /// User entry-point function.
    entry: ThreadFunc,
    /// Opaque argument forwarded to `entry`.
    arg: *mut c_void,
    /// How the thread's stack memory is sourced.
    stack: StackSpec,
    /// Horizon thread priority.
    prio: Priority,
    /// Target CPU core.
    cpuid: CoreId,
}

/// Creates a new Horizon thread in the *created* (suspended) state.
///
/// Performs the full creation flow: it allocates (or adopts) a page-aligned
/// buffer, maps a stack mirror through [`stack`], lays out the
/// per-thread control regions — entry-args block, TCB span, ELF TLS block,
/// runtime `_reent` slot, flat TSD array, and [`ThreadRuntime`] — initializes
/// the TLS image and TCB/DTV header, and finally issues `svcCreateThread`.
///
/// [`StackSpec`] selects how the stack is sized: [`StackSpec::Auto`] carries
/// the usable size and lets `create` allocate the backing buffer;
/// [`StackSpec::Provided`] carries the caller's buffer and its exact size,
/// which `create` maps as-is. Every size must be page-aligned.
///
/// The returned [`ThreadControl`] is suspended and must be transitioned to
/// runnable before it executes. The authoritative `*mut ThreadControl`
/// back-pointers (in the TCB and the entry-args block) stay null until the
/// caller pins the `ThreadControl` and starts it.
///
/// # Safety
///
/// - `config.entry` must be a valid entry point for the new thread.
/// - `config.arg` must be valid to pass to `entry`, or null.
/// - When `config.stack` is [`StackSpec::Provided`], its `base` must point to a
///   page-aligned buffer of exactly `size` bytes that stays valid for the
///   thread's lifetime.
pub unsafe fn create(config: ThreadCreateConfig) -> Result<ThreadControl, CreateError> {
    // Size of the entry-args block and the control block reserved after the
    // usable stack; `close` reclaims exactly this span (see
    // [`control_block_size`]).
    let entry_args_sz = size_of::<ThreadEntryArgs>();
    let control_sz = control_block_size();

    // Resolve the backing buffer: its base pointer, the mapped size, the layout
    // describing it, and whether `create` owns (and must free) the allocation.
    let (backing, map_size, layout, owns_stack_mem) = match config.stack {
        StackSpec::Auto(usable) => {
            // Horizon requires a page-aligned stack size (libnx `threadCreate`
            // rejects otherwise).
            if usable & (PAGE_SIZE - 1) != 0 {
                return Err(CreateError::InvalidStackAlignment);
            }
            // Guard the control-region overhead and page rounding against
            // `usize` overflow: a near-`usize::MAX` `usable` would wrap
            // `map_size` small, underflow `usable_stack`, and yield an
            // out-of-bounds `stack_top`. `entry_args_sz + control_sz` is a
            // small fixed sum and cannot overflow on its own.
            let map_size = usable
                .checked_add(entry_args_sz + control_sz)
                .and_then(|sum| checked_align_up(sum, PAGE_SIZE))
                .ok_or(CreateError::StackTooLarge)?;
            let layout = Layout::from_size_align(map_size, PAGE_SIZE)
                .map_err(|_| CreateError::OutOfMemory)?;
            // SAFETY: `layout` has a non-zero, page-aligned size.
            let ptr = unsafe { alloc_zeroed(layout) };
            let Some(backing) = NonNull::new(ptr) else {
                return Err(CreateError::OutOfMemory);
            };
            (backing.cast::<c_void>(), map_size, layout, true)
        }
        StackSpec::Provided { base, size } => {
            // Horizon requires a page-aligned base and size; a non-page-aligned
            // `size` would additionally make the mapping run past the caller's
            // buffer, so it must be rejected — never rounded.
            if base.as_ptr() as usize & (PAGE_SIZE - 1) != 0 || size & (PAGE_SIZE - 1) != 0 {
                return Err(CreateError::InvalidStackAlignment);
            }
            // The caller-provided buffer must host the control regions, the
            // entry-args block, and a non-empty usable stack.
            if size <= entry_args_sz + control_sz {
                return Err(CreateError::StackTooSmall);
            }
            let layout =
                Layout::from_size_align(size, PAGE_SIZE).map_err(|_| CreateError::OutOfMemory)?;
            (base, size, layout, false)
        }
    };

    let usable_stack = map_size - entry_args_sz - control_sz;

    // Map the stack mirror; release an owned allocation if the mapping fails.
    // SAFETY: `backing` points to `map_size` page-aligned bytes that stay valid
    // until the `unmap`/`dealloc` paths below.
    let buffer = unsafe { BufferRef::from_raw_parts(backing, layout) };
    // SAFETY: mapping a freshly resolved stack buffer into the stack region.
    let mapped = match unsafe { stack::map(buffer) } {
        Ok(mapped) => mapped,
        Err(err) => {
            if owns_stack_mem {
                // SAFETY: `backing`/`layout` are the pair returned by
                // `alloc_zeroed`; the buffer was never mapped.
                unsafe { dealloc(backing.cast::<u8>().as_ptr(), layout) };
            }
            return Err(CreateError::MapFailed(err));
        }
    };

    // Project the per-thread control regions inside the mapped mirror; see
    // [`MirrorLayout`] for the single authoritative definition of the layout.
    // SAFETY: the mirror spans `map_size` page-aligned bytes, sized above to
    // hold the usable stack followed by every control region.
    let regions = unsafe { MirrorLayout::new(mapped.mapped_mem_ptr(), usable_stack) };
    let args_ptr = regions.entry_args();
    let tcb_ptr = regions.tcb();
    let tls_start = regions.tls_start();
    let tsd_ptr = regions.tsd();
    let runtime_ptr = regions.runtime();

    // Per-thread newlib `_reent` pointer. The non-FFI build links no newlib and
    // hands spawned threads a null `reent`; the `ffi` build points at the
    // mirror slot `MirrorLayout` reserves, initialized below once the thread is
    // spawned (and still suspended).
    #[cfg(not(feature = "ffi"))]
    let reent: *mut c_void = ptr::null_mut();
    #[cfg(feature = "ffi")]
    let reent: *mut c_void = regions.reent().as_ptr();

    // Hand-off block read once by `entry_wrap` on the new thread. The `thread`
    // back-pointer stays null until the `ThreadControl` is pinned and started.
    // SAFETY: `args_ptr` is a writable, aligned `ThreadEntryArgs` slot.
    unsafe {
        args_ptr.write(ThreadEntryArgs {
            thread: ptr::null_mut(),
            entry: config.entry,
            arg: config.arg,
            reent,
            tls: tls_start.as_ptr(),
            runtime: runtime_ptr.as_ptr(),
        });
    }

    // Spawn the kernel thread; it stays suspended until `start`. The entry-args
    // block sits exactly at the stack top, so its address is both the `arg`
    // handed to `entry_wrap` and the `stack_top` the kernel decrements from.
    let entry = entry_wrap as unsafe extern "C" fn(*mut ThreadEntryArgs) as *mut c_void;
    let stack_top = args_ptr.cast::<c_void>().as_ptr();
    // SAFETY: `entry`, `args_ptr`, and `stack_top` stay valid for the thread's
    // entire lifetime, and `stack_top` is 16-byte aligned.
    let handle = match unsafe {
        nx_svc::thread::create(
            entry,
            stack_top,
            stack_top,
            config.prio.to_raw(),
            config.cpuid.to_raw(),
        )
    } {
        Ok(handle) => handle,
        Err(err) => {
            // SAFETY: `mapped` was produced by the `stack::map` call above.
            let _ = unsafe { stack::unmap(mapped) };
            if owns_stack_mem {
                // SAFETY: `backing`/`layout` are the `alloc_zeroed` pair.
                unsafe { dealloc(backing.cast::<u8>().as_ptr(), layout) };
            }
            return Err(CreateError::SvcFailed(err));
        }
    };

    // Initialize the TLS image, TCB/DTV header, and runtime TSD state. The new
    // thread is suspended, so none of these races with it.
    // SAFETY: `tls_start` owns `tls_sz` bytes, `tcb_ptr` is an aligned `Tcb`
    // slot, `tsd_ptr` covers `tsd_sz` bytes, and `runtime_ptr` is an aligned
    // `ThreadRuntime` slot — all inside the mapping.
    unsafe {
        tls_block::init_tls_block(tls_start.as_ptr());
        tls_block::init_tcb_and_dtv(tcb_ptr.as_ptr(), tls_start.as_ptr());
        ptr::write_bytes(tsd_ptr.as_ptr(), 0, NUM_TSD_KEYS);
        runtime_ptr.as_ptr().write(ThreadRuntime {
            tsd: tsd_ptr.as_ptr(),
            tsd_used: false,
        });
    }

    // Provision the thread's newlib `_reent` block, mirroring libnx
    // `threadCreate`: `_REENT_INIT_PTR` plus inheritance of the creating
    // thread's stdio. The new thread is suspended, so this does not race.
    #[cfg(feature = "ffi")]
    // SAFETY: `regions.reent()` is the `reent_size()`-byte `_reent` slot inside
    // the mapping, exclusively owned while the new thread stays suspended.
    unsafe {
        crate::ffi::reent::init_block(regions.reent());
    }

    Ok(ThreadControl {
        id: ThreadId::next(),
        handle,
        owns_stack_mem,
        stack_mem: Some(backing),
        stack_mirror: mapped.mapped_mem_ptr(),
        stack_size: usable_stack,
        runtime: runtime_ptr,
        state: AtomicU8::new(ThreadState::Created as u8),
        prev: AtomicPtr::new(ptr::null_mut()),
        next: AtomicPtr::new(ptr::null_mut()),
    })
}

/// Errors returned when creating a new thread.
///
/// Validation variants ([`InvalidStackAlignment`](CreateError::InvalidStackAlignment),
/// [`StackTooSmall`](CreateError::StackTooSmall),
/// [`StackTooLarge`](CreateError::StackTooLarge)) are produced before any
/// kernel object is touched; the remaining variants wrap a failure from a
/// downstream allocation, stack mapping, or `svcCreateThread`.
#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    /// The requested stack size is not a multiple of the page size.
    #[error("stack size not page-aligned")]
    InvalidStackAlignment,
    /// A caller-provided stack buffer is too small to also host the per-thread
    /// control regions (TCB span, ELF TLS block, reent, TSD array, runtime).
    #[error("provided stack too small")]
    StackTooSmall,
    /// The requested stack size is so large that adding the per-thread control
    /// regions and rounding up to the page size would overflow `usize`.
    ///
    /// Only [`StackSpec::Auto`] stacks reach this check: the control-region
    /// overhead and `PAGE_SIZE` rounding are folded into the requested usable
    /// size, so a near-`usize::MAX` request would wrap the mapped size small.
    /// Rejecting it up front keeps the usable stack from underflowing and the
    /// stack top from pointing outside the mapping.
    #[error("requested stack too large")]
    StackTooLarge,
    /// Heap allocation of the thread's stack/TLS block failed.
    #[error("out of memory")]
    OutOfMemory,
    /// Mapping the stack mirror into the stack region failed.
    #[error("failed to map stack memory")]
    MapFailed(#[from] MapError),
    /// The `svcCreateThread` system call failed.
    #[error("SVC create thread failed")]
    SvcFailed(#[from] CreateThreadError),
}

#[cfg(feature = "ffi")]
impl ToResultCode for CreateError {
    fn to_rc(self) -> ResultCode {
        match self {
            CreateError::InvalidStackAlignment
            | CreateError::StackTooSmall
            | CreateError::StackTooLarge => KernelError::InvalidSize.to_rc(),
            CreateError::OutOfMemory => KernelError::OutOfMemory.to_rc(),
            CreateError::MapFailed(err) => match err {
                MapError::VirtAddrAllocFailed => KernelError::OutOfAddressSpace.to_rc(),
                MapError::Svc(err) => err.to_rc(),
            },
            CreateError::SvcFailed(err) => err.to_rc(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for CreateError {}

/// Parameters for spawning a Horizon thread that runs a Rust closure.
///
/// [`SpawnConfig`] is [`ThreadCreateConfig`] without the C `entry`/`arg` pair:
/// [`spawn`] synthesizes those itself from the closure, so the caller cannot
/// supply an entry point that `spawn` would have to ignore or overwrite (a
/// type-driven-design choice — no field exists that `spawn` does not honor).
/// Only the stack and Horizon scheduling parameters stay caller-controlled.
pub struct SpawnConfig {
    /// How the thread's stack memory is sourced.
    stack: StackSpec,
    /// Horizon thread priority.
    prio: Priority,
    /// Target CPU core.
    cpuid: CoreId,
}

/// Default stack size, in bytes, for a thread built without an explicit size.
///
/// `128 KiB`, the size libnx's `__syscall_thread_create` substitutes when a
/// caller passes a zero stack size (`newlib.c:169`). It is page-aligned, as
/// [`create`] requires.
pub const DEFAULT_STACK_SIZE: usize = 128 * 1024;

/// Builder for a [`ThreadCreateConfig`] or a [`SpawnConfig`].
///
/// Modeled on [`std::thread::Builder`]: every scheduling parameter has a
/// default, so a caller overrides only what it cares about and never has to
/// remember a positional argument order. The shared parameters — the stack
/// ([`StackSpec`]), priority, and CPU core — are set through the chained
/// methods; the terminal [`build_create`](Builder::build_create) /
/// [`build_spawn`](Builder::build_spawn) methods finish the builder into the
/// config the matching entry point ([`create`] / [`spawn`]) consumes.
///
/// Defaults: an auto-allocated [`DEFAULT_STACK_SIZE`] stack,
/// [`Priority::DEFAULT`], and [`CoreId::PROCESS_DEFAULT`].
#[derive(Debug, Clone, Copy)]
pub struct Builder {
    /// How the thread's stack memory is sourced.
    stack: StackSpec,
    /// Horizon thread priority.
    prio: Priority,
    /// Target CPU core.
    cpuid: CoreId,
}

impl Builder {
    /// Creates a builder with every parameter at its default (see the
    /// [type docs](Builder)).
    pub const fn new() -> Self {
        Self {
            stack: StackSpec::Auto(DEFAULT_STACK_SIZE),
            prio: Priority::DEFAULT,
            cpuid: CoreId::PROCESS_DEFAULT,
        }
    }

    /// Sets how the thread's stack is sourced.
    ///
    /// The default is an auto-allocated [`DEFAULT_STACK_SIZE`] stack; pass
    /// [`StackSpec::Provided`] to adopt a caller-owned buffer (it must stay
    /// valid for the thread's lifetime, see [`create`]'s `# Safety` clause).
    pub const fn stack(mut self, stack: StackSpec) -> Self {
        self.stack = stack;
        self
    }

    /// Sets the Horizon scheduling priority.
    pub const fn priority(mut self, prio: Priority) -> Self {
        self.prio = prio;
        self
    }

    /// Sets the target CPU core.
    pub const fn core_id(mut self, cpuid: CoreId) -> Self {
        self.cpuid = cpuid;
        self
    }

    /// Finishes the builder into a [`ThreadCreateConfig`] for [`create`], with
    /// the given C entry point and its opaque argument.
    pub const fn build_create(self, entry: ThreadFunc, arg: *mut c_void) -> ThreadCreateConfig {
        ThreadCreateConfig {
            entry,
            arg,
            stack: self.stack,
            prio: self.prio,
            cpuid: self.cpuid,
        }
    }

    /// Finishes the builder into a [`SpawnConfig`] for [`spawn`].
    pub const fn build_spawn(self) -> SpawnConfig {
        SpawnConfig {
            stack: self.stack,
            prio: self.prio,
            cpuid: self.cpuid,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawns a new Horizon thread that runs the closure `f` and returns a
/// [`JoinHandle`].
///
/// `spawn` is the idiomatic, closure-accepting Level 1 entry point layered on
/// top of [`create`] + [`start`]: a Rust caller passes a normal `FnOnce`
/// closure instead of hand-writing an `extern "C"` shim and erasing its
/// captures through `*mut c_void`. Unlike the two-step [`create`] + [`start`]
/// core flow, `spawn` brings the thread all the way to *running* — matching
/// `std::thread::spawn` — and hands back a move-only [`JoinHandle<T>`] whose
/// [`join`](JoinHandle::join) yields the value the closure produced.
///
/// # Ownership
///
/// The spawned thread's state lives inside an [`Arc`], shared by two strong
/// counts. The returned [`JoinHandle`] holds one; the spawned thread "holds"
/// the other — it reaches its state only through `ThreadVars.thread_info_ptr`
/// (container-of), a raw pointer that owns no count, so `spawn` leaks one
/// [`Arc::into_raw`] clone for it. The [`Arc`] payload never moves, so the
/// embedded [`ThreadControl`] the thread locates by container-of stays pinned
/// for free. [`JoinHandle::join`] reclaims the thread-side count once the
/// thread is provably dead. Dropping the handle without joining *detaches* the
/// thread (see [`JoinHandle`]): the thread reclaims itself once it exits, so an
/// unjoined handle is not leaked.
///
/// The closure is `Box`-allocated because the thread body runs *after* `spawn`
/// returns — a stack-local closure would dangle. The raw `Box` pointer travels
/// as the C `arg`; the trampoline reclaims it on the spawned thread once the
/// closure returns. If [`create`] or [`start`] fails before the thread runs,
/// the trampoline never executes, so `spawn` reconstructs and drops the `Box`
/// itself — the captures are never leaked.
///
/// `F` is bounded `FnOnce() -> T + Send + 'static` and `T` is `Send + 'static`:
/// `FnOnce` because a thread body runs exactly once, `Send` because the closure
/// and its return value cross to another thread, and `'static` because the
/// closure must not borrow the caller's frame.
///
/// # Safety
///
/// When `config.stack` is [`StackSpec::Provided`], its `base` must point to a
/// page-aligned buffer of `size` bytes that stays valid for the thread's
/// lifetime — the same stack contract [`create`] imposes. The
/// closure-to-[`ThreadFunc`] conversion itself adds no unsafety: the
/// monomorphized trampoline is always a valid function pointer.
pub unsafe fn spawn<F, T>(config: SpawnConfig, f: F) -> Result<JoinHandle<T>, SpawnError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Heap-allocate the closure: the thread body runs after `spawn` returns, so
    // the closure must outlive this stack frame. The raw pointer is the *data
    // half* paired with the `closure_trampoline::<F, T>` *code half*.
    let raw = Box::into_raw(Box::new(f));

    // `closure_trampoline::<F, T>` monomorphizes to a plain `extern "C"`
    // function — a valid `ThreadFunc` — and `raw` is its matched `arg`.
    let create_config = ThreadCreateConfig {
        entry: closure_trampoline::<F, T>,
        arg: raw.cast::<c_void>(),
        stack: config.stack,
        prio: config.prio,
        cpuid: config.cpuid,
    };

    // SAFETY: `closure_trampoline::<F, T>` is a valid entry point and `raw` is a
    // valid argument for it; `config.stack` carries the caller's stack
    // contract straight through to `create`'s matching `# Safety` clause.
    let thread = match unsafe { create(create_config) } {
        Ok(thread) => thread,
        Err(err) => {
            // `create` failed before the thread ran, so the trampoline never
            // executes and never reclaims the closure — drop it here to avoid
            // leaking the captures.
            // SAFETY: `raw` came from `Box::into_raw` above and has not been
            // consumed; this reclaims that ownership exactly once.
            drop(unsafe { Box::from_raw(raw) });
            return Err(SpawnError::Create(err));
        }
    };

    // Share the thread state through an `Arc`: the running thread locates the
    // embedded `ThreadControl` by container-of from `ThreadVars.thread_info_ptr`,
    // and the `Arc` payload never moves, so that address stays pinned for free.
    let inner = Arc::new(SpawnInner {
        thread,
        result: UnsafeCell::new(None),
        detach_state: AtomicU8::new(DetachState::Joinable as u8),
    });

    // Hand the spawned thread its own strong count. It only ever reaches the
    // `SpawnInner` through the raw container-of pointer, which owns no count,
    // so leak one `Arc` clone here for it to "hold"; `JoinHandle::join`
    // reclaims this count once the thread is provably dead.
    let thread_side_ptr = Arc::into_raw(Arc::clone(&inner));

    // Project the embedded `thread` field to a raw `NonNull<ThreadControl>`:
    // `start` reaches the now-runnable thread through a raw pointer, never a
    // typed `&`.
    let inner_ptr = Arc::as_ptr(&inner).cast_mut();
    // SAFETY: `inner_ptr` addresses the live `SpawnInner` inside the `Arc`, so
    // `&raw mut (*inner_ptr).thread` is a non-null pointer to the stable
    // `ThreadControl` address `start` wires the back-pointers to.
    let thread_ptr = unsafe { NonNull::new_unchecked(&raw mut (*inner_ptr).thread) };
    // SAFETY: `thread_ptr` points to the pinned `ThreadControl` of a
    // freshly-created, still-suspended thread; the `Arc` keeps it pinned at
    // that address for the spawned thread's whole lifetime.
    match unsafe { start(thread_ptr) } {
        Ok(()) => Ok(JoinHandle { inner: Some(inner) }),
        Err(err) => {
            // `create` already spawned the kernel thread, but `start` failed
            // and rolled `state` back to `Created`, so the thread stays
            // suspended and `entry_wrap` never runs. Reclaim both `Arc` strong
            // counts to regain sole ownership of the `SpawnInner`, then hand
            // the created-but-not-started `ThreadControl` to `close`, which
            // releases its kernel handle, stack mirror mapping, and `Dtv`
            // node. The never-run trampoline never reclaimed the boxed
            // closure, so reconstruct and drop it here.
            // SAFETY: `thread_side_ptr` is the `Arc::into_raw` count leaked
            // just above, unconsumed; `from_raw` reclaims it exactly once, and
            // the suspended thread never touches the allocation.
            drop(unsafe { Arc::from_raw(thread_side_ptr) });
            // Reclaiming the thread-side count leaves `inner` the sole owner,
            // so `into_inner` yields the `SpawnInner`; a `None` would mean a
            // broken invariant, leaving the thread to leak rather than reclaim.
            if let Some(SpawnInner { thread, .. }) = Arc::into_inner(inner) {
                // A `Created` thread's stack is mapped but unused, so `close`
                // reclaims it. A `close` failure here only leaks and is
                // effectively unreachable on a fresh handle.
                let _ = close(thread);
            }
            // SAFETY: `raw` came from `Box::into_raw` above and the never-run
            // trampoline never reclaimed it; this drops the captures once.
            drop(unsafe { Box::from_raw(raw) });
            Err(SpawnError::Start(err))
        }
    }
}

/// A move-only handle to a thread spawned by [`spawn`].
///
/// `JoinHandle` owns one strong count of the spawned thread's `Arc`-shared
/// state and is the only way to retrieve the value its closure produced. It is
/// deliberately neither `Copy` nor `Clone`: [`join`](Self::join) consumes the
/// handle by value, so a thread can be joined at most once and a double join is
/// a compile error rather than a runtime use-after-free.
///
/// Dropping a `JoinHandle` without joining *detaches* the thread (see
/// [`detach`](detach::detach)): the thread reclaims itself once it exits. An
/// unjoined handle therefore detaches cleanly instead of leaking — the terminal
/// state of a never-joined `std::thread::JoinHandle`.
pub struct JoinHandle<T: Send + 'static> {
    /// The spawned thread's `Arc`-shared state — its pinned core
    /// [`ThreadControl`] and the closure's return-value slot.
    ///
    /// `Some` until [`join`](Self::join) or the `Drop` detach consumes it,
    /// after which the handle is inert.
    inner: Option<Arc<SpawnInner<T>>>,
}

impl<T: Send + 'static> JoinHandle<T> {
    /// Waits for the spawned thread to finish and returns the value its closure
    /// produced.
    ///
    /// Blocks until the thread has run its exit path, reclaims its stack
    /// mapping and kernel handle through [`close`], frees the `Arc`-shared
    /// thread state, and returns the closure's return value. Consuming `self`
    /// makes a second join on the same thread a compile error.
    ///
    /// # Errors
    ///
    /// Returns [`JoinError::Wait`] if waiting for the thread to terminate
    /// fails: the thread may still be running, so its return value cannot be
    /// recovered and the `Arc`-shared thread state leaks — the thread-side
    /// `Arc` count keeps it live — rather than reclaiming a still-running
    /// thread's state. Returns [`JoinError::Close`] if [`close`] fails *after*
    /// the thread exited; the closure's return value is still recovered and
    /// rides out in that variant, since the recorded value is independent of
    /// whether the stack mapping and kernel handle could be reclaimed.
    ///
    /// # Panics
    ///
    /// Panics — which aborts the process under `panic = "abort"` — on a broken
    /// invariant rather than a recoverable condition: a handle whose state was
    /// already taken, a joiner that is not the sole `Arc` owner after
    /// reclaiming the thread-side count, or a thread that terminated without
    /// recording a return value. None is reachable — a `JoinHandle` is
    /// move-only so it cannot be cloned or joined twice, and a thread that
    /// reached `svcExitThread` has always run the trampoline that records the
    /// value.
    pub fn join(mut self) -> Result<T, JoinError<T>> {
        // Take the shared state out of the handle. A `JoinHandle` holds it
        // until `join` (here) or `Drop` consumes it, and both take the handle
        // by value, so the slot is always `Some` at this point.
        let inner = self
            .inner
            .take()
            .expect("JoinHandle::join: a JoinHandle holds its state until joined");

        // Project the embedded `thread` field to a raw `NonNull<ThreadControl>`
        // without forming a typed `&` over the concurrently-live thread:
        // the joined thread foreign-writes its own `state`/`prev`/
        // `next` right up until it exits.
        // SAFETY: `inner` is a live `Arc<SpawnInner<T>>`, so `Arc::as_ptr`
        // yields a pointer valid for this call; `&raw mut` projects its
        // `thread` field to a non-null pointer without going through a
        // reference.
        let thread_ptr =
            unsafe { NonNull::new_unchecked(&raw mut (*Arc::as_ptr(&inner).cast_mut()).thread) };

        // Wait for the thread to run its exit path *before* reading or freeing
        // the shared state: the thread writes `result` right up until
        // it exits, so an earlier read would race it and return a stale value.
        // SAFETY: `thread_ptr` points to the embedded `ThreadControl` of the
        // `Arc`-shared `SpawnInner`; `inner` keeps it alive across the wait.
        unsafe { wait_for_exit(thread_ptr) }.map_err(JoinError::Wait)?;

        // `wait_for_exit` returned `Ok`: the kernel signaled termination only
        // after `svcExitThread`, so the trampoline's write of `result`
        // happened-before this point and the thread will never touch the state
        // again.
        // SAFETY: `inner` is the un-cloned join-handle `Arc` whose thread-side
        // count `spawn` leaked is still live; the termination wait just above
        // discharges `reclaim_after_exit_spawn`'s exit precondition.
        unsafe { reclaim_after_exit_spawn(inner) }
    }
}

impl<T: Send + 'static> Drop for JoinHandle<T> {
    /// Detaches the spawned thread if it was never joined.
    ///
    /// A `JoinHandle` dropped without [`join`](Self::join) *detaches* its
    /// thread: [`detach`](detach::detach) resolves the detach-vs-exit race so
    /// the thread reclaims itself once it exits — or reclaims it here, if it
    /// already exited. An unjoined handle therefore detaches cleanly instead of
    /// leaking, the terminal state of a never-joined `std::thread::JoinHandle`.
    /// Dropping a handle `join` already consumed is a no-op.
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            detach::detach(inner);
        }
    }
}

// `Detachable` lets a detached `spawn` thread self-reclaim its `Arc`-shared
// `SpawnInner` through `detach`'s `exit_self_or_detached` / `unmap_self`, and
// lets `JoinHandle`'s `Drop` route through `detach::detach`.
impl<T: Send + 'static> Detachable for SpawnInner<T> {
    fn thread_ptr(obj: NonNull<Self>) -> NonNull<ThreadControl> {
        // Project the embedded `thread` field without forming a typed `&` over
        // the concurrently-live thread.
        // SAFETY: `obj` addresses a live `SpawnInner<T>`; `&raw mut` projects
        // its `thread` field to a non-null pointer without going through a
        // reference.
        unsafe { NonNull::new_unchecked(&raw mut (*obj.as_ptr()).thread) }
    }

    fn detach_state(obj: NonNull<Self>) -> NonNull<AtomicU8> {
        // SAFETY: `obj` addresses a live `SpawnInner<T>`; `&raw mut` projects
        // its `detach_state` field to a non-null pointer.
        unsafe { NonNull::new_unchecked(&raw mut (*obj.as_ptr()).detach_state) }
    }

    fn into_thread_control(self) -> ThreadControl {
        // `result` (the recorded return value) and `detach_state` drop here — a
        // detached thread has no joiner to receive its return value.
        self.thread
    }

    unsafe fn reclaim_exited(arc: Arc<Self>) {
        // SAFETY: `Arc::as_ptr` never returns null.
        let obj = unsafe { NonNull::new_unchecked(Arc::as_ptr(&arc).cast_mut()) };
        let thread_ptr = Self::thread_ptr(obj);
        // SAFETY: `thread_ptr` addresses the embedded `ThreadControl`; `arc`
        // keeps it alive across the wait.
        if unsafe { wait_for_exit(thread_ptr) }.is_err() {
            // The thread may still be live — leak rather than reclaim a
            // running thread's state; the detach path has no error channel.
            return;
        }
        // SAFETY: `wait_for_exit` returned `Ok`, proving the thread exited;
        // `arc` is the un-cloned join handle whose thread-side count is still
        // outstanding — exactly `reclaim_after_exit_spawn`'s contract. The
        // recovered return value is dropped (a detached thread has no joiner).
        let _ = unsafe { reclaim_after_exit_spawn(arc) };
    }
}

/// Reclaims a [`spawn`]ed thread whose termination has already been observed.
///
/// The post-termination half of [`JoinHandle::join`], shared with the
/// detach-after-exit path through [`SpawnInner`]'s [`Detachable`] impl: it
/// reclaims the thread-side [`Arc`] strong count [`spawn`] leaked, frees the
/// thread state through [`close`], and returns the value the closure recorded.
/// Splitting it out lets the join path and the detach path funnel into one
/// reclaim path (DRY).
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if the caller is
/// not the sole `Arc` owner after reclaiming the thread-side count, or if the
/// thread exited without recording a return value. Both indicate a broken
/// invariant: the join handle cannot be cloned, and a thread that reached
/// `svcExitThread` has always run the trampoline that records the value.
///
/// # Safety
///
/// - `inner` must be the un-cloned [`spawn`] join-handle `Arc` whose thread-side
///   count has not yet been reclaimed.
/// - The thread must have *already exited*, its termination observed through
///   [`wait_for_exit`]/[`wait_for_any_exit`], so its trampoline write of
///   `result` happened-before this call, and it never touches the state again.
unsafe fn reclaim_after_exit_spawn<T: Send + 'static>(
    inner: Arc<SpawnInner<T>>,
) -> Result<T, JoinError<T>> {
    // Reclaim the thread-side `Arc` count `spawn` leaked; the thread can never
    // drop it itself (that would free the `ThreadControl` its exit path runs
    // on), so it is reclaimed here, now that the thread is provably dead.
    // SAFETY: `spawn` leaked exactly one count via `Arc::into_raw` at this data
    // address and the join handle cannot be cloned, so `from_raw` consumes that
    // count exactly once; the allocation is still live — `inner` holds the
    // other count.
    drop(unsafe { Arc::from_raw(Arc::as_ptr(&inner)) });

    // The caller now holds the sole strong count, so it can move the state out
    // and reclaim the exited thread's resources.
    let inner = Arc::into_inner(inner).expect(
        "reclaim_after_exit_spawn: caller must be the sole Arc owner after the thread-side count",
    );
    let SpawnInner {
        thread,
        result,
        detach_state: _,
    } = inner;

    // Read the recorded return value *before* `close`. The trampoline runs on
    // every thread that reaches `svcExitThread`, and the termination wait above
    // proves it did, so `result` is `Some`. Reading it first means a `close`
    // failure carries the value out in `JoinError::Close` instead of dropping
    // the value the closure successfully computed — the recorded value is
    // independent of whether reclaiming the stack mapping and handle succeeds.
    let value = result
        .into_inner()
        .expect("reclaim_after_exit_spawn: an exited thread must have recorded its return value");

    // Reclaim the exited thread's stack mapping and kernel handle.
    if let Err(source) = close(thread) {
        return Err(JoinError::Close { value, source });
    }

    Ok(value)
}

/// Errors returned when joining a spawned thread via [`JoinHandle::join`].
#[derive(Debug, thiserror::Error)]
pub enum JoinError<T> {
    /// [`wait_for_exit`] failed while waiting for the thread to exit.
    ///
    /// The thread may still be running, so its return value cannot be
    /// recovered and the `Arc`-shared thread state leaks — the thread-side
    /// `Arc` count keeps it live — rather than reclaiming a still-running
    /// thread's state.
    #[error("failed to wait for the spawned thread to exit")]
    Wait(#[source] WaitError),
    /// [`close`] failed while reclaiming the exited thread's resources.
    ///
    /// The thread had already exited and recorded its return value before
    /// [`close`] ran, so `value` carries that value out: a `close` failure
    /// leaks the stack mapping and kernel handle but does not invalidate the
    /// value the closure computed.
    #[error("failed to reclaim the joined thread")]
    Close {
        /// The value the joined thread's closure produced and recorded.
        value: T,
        /// The underlying [`close`] failure.
        #[source]
        source: CloseError,
    },
}

/// Error returned when [`spawn`] fails to bring the spawned thread up.
///
/// `spawn` adds no failure mode of its own beyond [`create`]'s and [`start`]'s,
/// so each variant wraps an underlying error. It uses `#[source]` rather than
/// `#[from]` so the failure path stays explicit: `spawn` must reclaim the boxed
/// closure — and, on a `start` failure, the leaked `Arc` counts — before
/// mapping the error, which a `?`-driven `#[from]` conversion would bypass.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    /// [`create`] failed while bringing the spawned thread up.
    #[error("failed to create the spawned thread")]
    Create(#[source] CreateError),
    /// [`start`] failed while transitioning the created thread to runnable.
    /// Effectively unreachable for a freshly created handle.
    #[error("failed to start the spawned thread")]
    Start(#[source] StartError),
}

// `SpawnError` deliberately carries no `ToResultCode` impl: `spawn` is a
// Level-1 idiomatic-Rust API with no C ABI override symbol, so the error never
// crosses the FFI boundary. This matches `JoinError`, the other Level-1-only
// error. libnx's separate `threadCreate`/`threadStart` entries map to `create`/
// `start`, whose `CreateError`/`StartError` already carry `ToResultCode`.

/// Monomorphized trampoline that re-joins a closure with its captures on the
/// spawned thread and records its return value.
///
/// [`spawn`] splits a closure into a *code half* — this function — and a *data
/// half* — a `Box<F>` that reaches the new thread through the C `arg` slot.
/// Each `closure_trampoline::<F, T>` instantiation is monomorphized into a
/// concrete `extern "C"` function, a valid [`ThreadFunc`]; [`entry_wrap`]'s
/// existing `(args.entry)(args.arg)` call invokes it. It reconstructs the
/// `Box<F>`, runs the closure exactly once, drops the box — freeing the
/// captures on the spawned thread — and stores the closure's return value into
/// the `Arc`-shared [`SpawnInner`] the running thread locates by container-of,
/// so a later [`JoinHandle::join`] can retrieve it.
///
/// It then tears the thread down through [`detach::exit_self_or_detached`],
/// which never returns — so, unlike a plain libnx entry point, control never
/// falls back to [`entry_wrap`]'s generic [`exit`]. Diverting here is what lets
/// a detached spawned thread self-reclaim.
///
/// # Safety
///
/// `arg` must be the `Box::into_raw(Box::new(f))` pointer that `spawn::<F, T>`
/// paired with this exact `closure_trampoline::<F, T>` instantiation
/// (type-welding). No API exposes this pointer separately, so the matched pair
/// can only ever be formed inside one `spawn::<F, T>` monomorphization.
unsafe extern "C" fn closure_trampoline<F, T>(arg: *mut c_void)
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // SAFETY: `arg` is the `Box::into_raw(Box::new(f))` pointer that
    // `spawn::<F, T>` paired with this exact `closure_trampoline::<F, T>`
    // instantiation (type-welding).
    let f = unsafe { Box::from_raw(arg.cast::<F>()) };
    let value = f();

    // Record the return value into the `Arc`-shared `SpawnInner`. The running
    // thread reaches that object only through `ThreadVars.thread_info_ptr`,
    // which addresses its embedded `ThreadControl` — walk back to the
    // enclosing `SpawnInner` by container-of.
    let info = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    let Some(info) = NonNull::new(info) else {
        // No core state — unreachable for a `spawn`-created thread, whose
        // `thread_info_ptr` is always wired. Fall back to the plain exit path,
        // which aborts on this broken invariant.
        // SAFETY: runs once, on this thread, as its final operation.
        unsafe { exit() }
    };
    // SAFETY: a thread spawned by `spawn` has its `thread_info_ptr` wired to
    // the `thread` field of an `Arc`-shared `SpawnInner<T>`, so
    // `enclosing_spawn_inner` recovers the enclosing object.
    let inner = unsafe { enclosing_spawn_inner::<T>(info) };
    // SAFETY: `inner` points to this thread's live `SpawnInner`; `raw_get`
    // yields the `result` slot without forming a reference, and
    // `JoinHandle::join` reads it only after `wait_for_exit` observes the exit,
    // so this write is ordered before any read of it.
    unsafe {
        *UnsafeCell::raw_get(&raw const (*inner.as_ptr()).result) = Some(value);
    }

    // Tear the thread down: run the exit prefix, then either `svcExitThread`
    // (still joinable) or self-reclaim (detached). Never returns.
    // SAFETY: runs on this `spawn`-created thread as its final operation;
    // `inner` is its `Arc`-shared `SpawnInner` with both `Arc` counts live.
    unsafe { detach::exit_self_or_detached(inner) }
}

/// The `Arc`-shared state of a thread spawned by [`spawn`].
///
/// Bundles the spawned thread's pinned core [`ThreadControl`] with the slot for
/// its closure's return value. Shared through an [`Arc`] between the
/// [`JoinHandle`] and the running thread; the latter reaches it only by
/// container-of from `ThreadVars.thread_info_ptr` (see [`enclosing_spawn_inner`]),
/// so the `Arc` payload must never move — which holding it in an `Arc`
/// guarantees.
struct SpawnInner<T> {
    /// Core thread state; the container-of anchor for the running thread.
    thread: ThreadControl,
    /// The closure's return value, stored by the spawned thread through
    /// [`closure_trampoline`] and read by [`JoinHandle::join`].
    ///
    /// An [`UnsafeCell`] so the exiting thread can write it through the shared,
    /// `Arc`-backed object; the `join` read is ordered after that write by the
    /// join synchronization edge, so the access is not a data race.
    /// `None` until the trampoline records the value.
    result: UnsafeCell<Option<T>>,
    /// Detach-vs-exit race state (see [`DetachState`]).
    ///
    /// [`Joinable`](DetachState::Joinable) until [`JoinHandle`]'s `Drop` detaches
    /// the thread or the thread's own exit CAS claims it.
    detach_state: AtomicU8,
}

// SAFETY: `SpawnInner` is shared across threads by design — the spawned thread
// reaches it by container-of while the `JoinHandle`'s `Arc` is owned and
// reclaimed on another thread. The embedded `ThreadControl` confines its
// concurrent self-mutation to atomic fields, and `result` is an `UnsafeCell`
// whose write (by the spawned thread) and read (by the joiner) are ordered by
// the join edge, never a true data race. The `T: Send` bound covers the
// return value itself crossing from the spawned thread to the joiner.
unsafe impl<T: Send> Send for SpawnInner<T> {}
// SAFETY: see the `Send` impl above — the same contract makes a shared
// `&SpawnInner` sound to access from more than one thread.
unsafe impl<T: Send> Sync for SpawnInner<T> {}

/// Recovers the enclosing [`SpawnInner`] from a pointer to its embedded
/// [`ThreadControl`].
///
/// `ThreadVars.thread_info_ptr` addresses the `thread` field of an `Arc`-shared
/// [`SpawnInner`]; this walks back by that field offset (container-of) to the
/// enclosing object. The result is a raw [`NonNull`] — no `&SpawnInner` is
/// formed, so the concurrent self-mutation of the embedded `ThreadControl` is
/// not a data race.
///
/// # Safety
///
/// `info` must address the embedded `thread` field of an `Arc`-shared
/// `SpawnInner<T>` — i.e. it must be the `ThreadVars.thread_info_ptr` of a
/// thread created by [`spawn`]. On a thread created by any other path the
/// container-of arithmetic yields a bogus pointer.
unsafe fn enclosing_spawn_inner<T>(info: NonNull<ThreadControl>) -> NonNull<SpawnInner<T>> {
    // SAFETY: by the contract `info` addresses the `thread` field of an
    // `Arc`-shared `SpawnInner<T>`, so `byte_sub` by that field offset stays
    // within the same allocation and recovers the enclosing object.
    unsafe {
        info.byte_sub(offset_of!(SpawnInner<T>, thread))
            .cast::<SpawnInner<T>>()
    }
}

/// Arguments handed to a new thread's entry wrapper.
///
/// Written at the top of the new thread's stack by the creation path and read
/// once, on the thread's first instruction, by the entry wrapper. It carries
/// every pointer the wrapper needs to bring the thread up — core state, the
/// user entry point and its argument, the newlib reentrancy block, and the ELF
/// TLS block — before any user code runs. The sixth field,
/// [`runtime`](Self::runtime), is a reserved layout slot the wrapper does not
/// consume; see its field doc.
///
/// `#[repr(C)]` with a size that is a multiple of 16 bytes: Horizon's thread
/// entry ABI keeps the stack pointer 16-byte aligned, so this block can sit
/// directly at the stack top without disturbing that invariant. libnx's own
/// `ThreadEntryArgs` is a sequencing reference for this layout, not an ABI
/// contract — C callers never observe this struct.
//
// Exposed as `pub` rather than kept private: the creation path (`thread.rs`)
// and the entry wrapper both construct/read it, but neither lands until Phase 2.
// A private struct with no constructor would trip `dead_code`, and the project
// forbids `expect(dead_code)`; this mirrors the Task 1.3 "pub vs defer"
// tradeoff already taken for `ThreadState`.
#[repr(C)]
pub struct ThreadEntryArgs {
    /// Authoritative core state for the new thread.
    pub thread: *mut ThreadControl,
    /// User-supplied entry-point function.
    pub entry: ThreadFunc,
    /// Opaque argument forwarded to [`entry`](Self::entry).
    pub arg: *mut c_void,
    /// Newlib `_reent` reentrancy block for the thread, or null when unused.
    pub reent: *mut c_void,
    /// Base of the thread's ELF TLS block (`__tls_start` location).
    pub tls: *mut u8,
    /// Reserved slot — populated by [`create`] but never read by
    /// [`entry_wrap`].
    ///
    /// It points at the new thread's [`ThreadRuntime`], but the entry wrapper
    /// has no need for it: the thread reaches its runtime through
    /// `ThreadControl.runtime` (wired by [`create`]), so consulting this field
    /// would only duplicate that pointer — Task 2.2 deliberately dropped the
    /// redundant "attach `args.runtime`" bring-up step for exactly this reason.
    ///
    /// The field is retained so `ThreadEntryArgs` stays six pointer-sized
    /// fields wide — 48 bytes, a 16-byte multiple — letting the block sit at
    /// the stack top without breaking Horizon's 16-byte stack-alignment
    /// invariant. (See the struct-level note and the `const_assert_eq!`s
    /// below.) Dropping it would shrink the block to 40 bytes and violate that
    /// invariant.
    pub runtime: *mut ThreadRuntime,
}

// Six pointer-sized fields: 48 bytes on AArch64, a 16-byte multiple, so the
// block keeps the stack 16-byte aligned when placed at the stack top.
// Alignment stays plain pointer alignment.
const_assert_eq!(size_of::<ThreadEntryArgs>(), 6 * size_of::<usize>());
const_assert_eq!(size_of::<ThreadEntryArgs>() % 16, 0);
const_assert_eq!(align_of::<ThreadEntryArgs>(), align_of::<*mut c_void>());

/// Tears the calling thread down and terminates it; never returns.
///
/// Runs the thread's runtime TSD destructors, unregisters it from the
/// process-wide live-thread registry, marks its core state
/// [`Exited`](ThreadState::Exited), and finally issues `svcExitThread`.
///
/// Destructors run *before* the thread leaves the registry, matching musl /
/// POSIX ordering: a concurrent runtime-TLS key deletion ([`tsd::free`]) must
/// still observe this thread while its destructors execute.
///
/// This is the core teardown path. [`entry_wrap`] calls it once the user entry
/// point returns, and the `ffi::libnx` adapter routes `threadExit` through it.
///
/// # Panics
///
/// Panics — which aborts the process, as the crate builds with
/// `panic = "abort"` — when the calling thread has no core state installed
/// (`ThreadVars.thread_info_ptr` is null), i.e. it was not created or adopted
/// by `nx-sys-thread`. Reaching that state violates the `# Safety` contract
/// below, so it is a caller bug rather than a recoverable condition.
///
/// # Safety
///
/// - Must be called on a thread created or adopted by `nx-sys-thread`, whose
///   `ThreadVars` footer this crate installed.
/// - Must run at most once per thread, as that thread's final operation: no
///   borrow of its stack, ELF TLS block, or [`ThreadControl`] may outlive the
///   call.
pub unsafe fn exit() -> ! {
    // Resolve the calling thread's authoritative core state from the kernel
    // TLS footer installed during thread bring-up.
    let thread = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    let Some(thread) = NonNull::new(thread) else {
        // No core state: the thread was never registered by `nx-sys-thread`,
        // so there is nothing to tear down and the `# Safety` contract was
        // broken. Abort rather than silently masking the caller's bug.
        panic!("nx-sys-thread: exit() called on a thread not managed by nx-sys-thread");
    };

    // Run the stack-safe teardown prefix, then hand the thread back to the
    // kernel; `svcExitThread` never returns.
    // SAFETY: `thread` is the calling thread's own `ThreadControl`, resolved
    // from its TLS footer; `exit` runs once, as the thread's final operation.
    unsafe { exit_prefix(thread.as_ptr()) };
    nx_svc::thread::exit()
}

/// Runs the stack-safe prefix of a thread's exit path.
///
/// Runs the calling thread's runtime TSD destructors, unregisters it from the
/// process-wide live-thread registry, and marks its core state
/// [`Exited`](ThreadState::Exited) — every teardown step that must run on the
/// thread's *own* stack, while its TLS and registry links are still valid. What
/// follows differs by caller: plain [`exit`] issues `svcExitThread`; the
/// detach-aware path ([`detach::exit_self_or_detached`]) then runs its
/// detach-state CAS.
///
/// Destructors run *before* the thread leaves the registry, matching musl /
/// POSIX ordering: a concurrent runtime-TLS key deletion ([`tsd::free`]) must
/// still observe this thread while its destructors execute.
///
/// # Safety
///
/// - `thread` must be the calling thread's own [`ThreadControl`], created or
///   adopted by `nx-sys-thread` and still registered.
/// - Must run at most once per thread, as part of its final teardown.
pub(crate) unsafe fn exit_prefix(thread: *mut ThreadControl) {
    // Run the runtime TSD destructors while the thread is still registered, so
    // a concurrent key deletion sees a consistent registry.
    // SAFETY: `exit_prefix` runs on the owning thread before its runtime state
    // is reclaimed. `runtime` is fixed at creation, so reading it through the
    // raw `thread` pointer forms no `&ThreadControl` and is sound even though
    // the thread is concurrently registered; it yields this thread's own
    // `ThreadRuntime`.
    unsafe {
        let runtime = (*thread).runtime.as_ptr();
        tsd::run_destructors(runtime);
    }

    // Unregister from the live-thread list, then mark the core state exited.
    // `remove` takes `THREAD_MUTEX` and clears this node's registry links; the
    // `state` store is a lock-free `Release` that observers pair with the
    // `Acquire` load in `is_running`.
    // SAFETY: `thread` is the calling thread's own `ThreadControl`, registered
    // by the bring-up path and still live. `state` is an `AtomicU8`, so this
    // store through the raw pointer races no concurrent observer.
    unsafe {
        thread_list::remove(thread);
        (*thread)
            .state
            .store(ThreadState::Exited as u8, Ordering::Release);
    }
}

/// Reclaims the resources of a thread that has run its exit path.
///
/// Unmaps the stack mirror, frees the backing allocation when `nx-sys-thread`
/// owns it, reclaims the heap-allocated DTV node, and drops the user-space
/// reference to the kernel thread object. Consumes the [`ThreadControl`], since
/// the thread object must not be observed afterward.
///
/// `close` is the counterpart of [`create`]: it expects a [`ThreadControl`]
/// produced by `create`, whose backing allocation and control-block layout it
/// reverses. A [`ThreadControl`] with no tracked backing pointer — the
/// kernel-owned main thread — has no `nx-sys-thread`-mapped mirror, so only its
/// kernel handle is closed.
///
/// Returns [`CloseError::StillRunning`] only when the thread is *running*
/// ([`ThreadState::Running`]): reclaiming a live thread's stack would fault it
/// mid-execution, so callers must wait for it to exit first. A
/// created-but-not-started thread ([`Created`](ThreadState::Created)) is
/// suspended with an unused stack, and an exited thread has released its stack,
/// so `close` reclaims both — including a thread left over from a failed
/// [`start`], which rolls the state back to `Created`.
pub fn close(thread: ThreadControl) -> Result<(), CloseError> {
    // Reclaiming a running thread's stack would fault it mid-execution. A
    // created-but-not-started thread is suspended with an unused stack and an
    // exited thread has released its stack, so only a running thread is
    // rejected.
    if thread.is_running() {
        return Err(CloseError::StillRunning);
    }

    // Reclaim the mapped stack block and its DTV node. Threads created by
    // `create` always track a backing pointer here; one without it (the
    // kernel-owned main thread) has no `nx-sys-thread`-mapped mirror to unmap.
    if let Some(stack_mem) = thread.stack_mem() {
        // The DTV is a heap node separate from the mapped block; its pointer
        // lives in the TCB inside the mirror, so it must be read before the
        // mirror is unmapped.
        // SAFETY: `create` mapped this mirror with room for every control
        // region and placed the `Tcb` there; the exited thread no longer
        // accesses its stack region.
        let dtv = unsafe {
            let regions = MirrorLayout::new(thread.stack_mirror(), thread.stack_size());
            (*regions.tcb().as_ptr()).dtv
        };

        // `create` mapped exactly this span: usable stack + entry-args block +
        // control block. It is already page-aligned, since `create`
        // page-aligned the original mapping.
        let map_size = thread.stack_size() + size_of::<ThreadEntryArgs>() + control_block_size();

        // Unmap the stack mirror from the stack region.
        unmap_memory(thread.stack_mirror(), stack_mem, map_size)
            .map_err(CloseError::UnmapFailed)?;

        // Free the backing allocation when `nx-sys-thread` owns it.
        if thread.owns_stack_mem() {
            // SAFETY: `map_size`/`PAGE_SIZE` reproduce the exact layout
            // `create` passed to `alloc_zeroed` — `PAGE_SIZE` is a non-zero
            // power of two and `map_size` was already validated by `create`'s
            // successful allocation and mapping.
            let layout = unsafe { Layout::from_size_align_unchecked(map_size, PAGE_SIZE) };
            // SAFETY: the mirror is now unmapped, so `stack_mem` is the sole
            // alias of the backing allocation; it is freed exactly once.
            unsafe { dealloc(stack_mem.cast::<u8>().as_ptr(), layout) };
        }

        // Reclaim the heap-allocated DTV node `init_tcb_and_dtv` produced.
        if !dtv.is_null() {
            // SAFETY: `dtv` came from `Box::into_raw` in `init_tcb_and_dtv` and
            // has not been reclaimed; this consumes that ownership exactly once.
            drop(unsafe { Box::from_raw(dtv) });
        }
    }

    // Drop the user-space reference to the kernel thread object.
    close_handle(thread.handle()).map_err(CloseError::CloseHandleFailed)?;

    Ok(())
}

/// Errors returned when reclaiming an exited thread's resources via [`close`].
///
/// [`StillRunning`](CloseError::StillRunning) is produced before any kernel
/// object is touched; the remaining variants wrap a failure from the stack
/// unmap or the handle close.
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// The thread is running, so its stack is in use and cannot be reclaimed.
    /// A created-but-not-started thread is *not* running and remains
    /// reclaimable.
    #[error("thread is still running")]
    StillRunning,
    /// Unmapping the thread's stack mirror failed.
    #[error("failed to unmap stack memory")]
    UnmapFailed(#[from] UnmapMemoryError),
    /// Closing the thread's kernel handle failed.
    #[error("failed to close thread handle")]
    CloseHandleFailed(#[from] CloseHandleError),
}

#[cfg(feature = "ffi")]
impl ToResultCode for CloseError {
    fn to_rc(self) -> ResultCode {
        match self {
            CloseError::StillRunning => KernelError::Busy.to_rc(),
            CloseError::UnmapFailed(err) => err.to_rc(),
            CloseError::CloseHandleFailed(err) => err.to_rc(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for CloseError {}

/// Transitions a created thread to *runnable*.
///
/// [`create`] returns a suspended thread whose authoritative `*mut ThreadControl`
/// back-pointers are still null — its address was not stable while `create`
/// ran. `start` receives the now-pinned [`ThreadControl`] and, before issuing
/// `svcStartThread`, wires both access paths to it: the [`ThreadEntryArgs`]
/// block the [entry wrapper](entry_wrap) reads, and the [`Tcb`] `thread` slot
/// the TP-relative path resolves. Both sit at fixed offsets `create` reserved
/// in the mapped stack mirror, and the thread is still suspended, so the writes
/// never race it.
///
/// `start` then transitions the [`ThreadState`] from
/// [`Created`](ThreadState::Created) to [`Running`](ThreadState::Running)
/// before issuing `svcStartThread`. A `svcStartThread` failure rolls the state
/// back to `Created`: the thread never ran, so [`close`] can still reclaim it.
///
/// `thread` is a raw [`NonNull`], not a `&ThreadControl`: the spawned thread
/// keeps this exact address as its pinned core-state pointer for its whole
/// lifetime (see the `# Safety` pinning contract), which outlives any borrow a
/// `&ThreadControl` parameter could carry.
///
/// # Safety
///
/// - `thread` must point to a [`ThreadControl`] produced by [`create`] and not
///   yet started, valid for the duration of the call.
/// - It must stay pinned at that address for the spawned thread's whole
///   lifetime: the wired back-pointers — and the thread's own registry links
///   and lifecycle writes — dereference it. Moving or dropping the
///   [`ThreadControl`] while the thread runs leaves those pointers dangling.
pub unsafe fn start(thread: NonNull<ThreadControl>) -> Result<(), StartError> {
    // The pinned core-state address both back-pointers must carry.
    let thread_ptr = thread.as_ptr();

    // Read the creation-fixed mirror geometry and kernel handle through the raw
    // pointer; `start` forms no `&ThreadControl`, so the post-`svcStartThread`
    // self-mutation window cannot race it. `stack_mirror`/`stack_size`/
    // `handle` are all fixed at creation and never written by the running thread.
    // SAFETY: by the `# Safety` contract `thread` points to a valid, suspended
    // `ThreadControl` whose fields `create` initialized.
    let (stack_mirror, stack_size, handle) = unsafe {
        (
            (*thread_ptr).stack_mirror,
            (*thread_ptr).stack_size,
            (*thread_ptr).handle,
        )
    };

    // Project the entry-args block and TCB that `create` wrote inside this
    // thread's mapped stack mirror.
    // SAFETY: `create` mapped this mirror with room for every control region.
    let regions = unsafe { MirrorLayout::new(stack_mirror, stack_size) };
    // SAFETY: the thread stays suspended until the `svcStartThread` below, so
    // wiring its entry-args and TCB back-pointers does not race it.
    unsafe {
        (*regions.entry_args().as_ptr()).thread = thread_ptr;
        (*regions.tcb().as_ptr()).thread = thread_ptr;
    }

    // Mark the thread runnable before the kernel starts it. While it is still
    // suspended this store cannot race it; once `svcStartThread` succeeds the
    // thread owns every later `state` transition, and `start` writes `state`
    // only here and on the failure path below.
    // SAFETY: `state` is an `AtomicU8`; the still-suspended thread performs no
    // concurrent access.
    unsafe {
        (*thread_ptr)
            .state
            .store(ThreadState::Running as u8, Ordering::Release);
    }

    match nx_svc::thread::start(handle) {
        Ok(()) => Ok(()),
        Err(err) => {
            // The kernel refused to start the thread, so it never ran. Roll
            // `state` back to `Created` so `close` can reclaim the thread
            // (its stack, mapping, and handle) instead of leaking it.
            // SAFETY: as above — the thread never started, so nothing races
            // this store.
            unsafe {
                (*thread_ptr)
                    .state
                    .store(ThreadState::Created as u8, Ordering::Release);
            }
            Err(match err {
                StartThreadError::InvalidHandle => StartError::InvalidHandle,
                StartThreadError::Unknown(err) => StartError::Unknown(err),
            })
        }
    }
}

/// Errors returned when transitioning a created thread to runnable via
/// [`start`].
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// The thread handle is invalid — typically the thread was already started.
    #[error("invalid thread handle")]
    InvalidHandle,
    /// Any other, unrecognized kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

#[cfg(feature = "ffi")]
impl ToResultCode for StartError {
    fn to_rc(self) -> ResultCode {
        match self {
            StartError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            StartError::Unknown(err) => err.to_raw(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for StartError {}

/// Blocks the calling thread until `thread` has exited.
///
/// Waits on the thread's kernel handle, which the kernel signals when the
/// thread terminates; the wait is unbounded. Once it returns `Ok`, the thread
/// has run its exit path and [`close`] can safely reclaim its resources — the
/// two together form a join.
///
/// `thread` is a raw [`NonNull`] supplied by the caller — the live-thread
/// registry and [`current`] hand out raw pointers, and [`start`] pins the
/// joined thread to one. `wait_for_exit` reads only the creation-fixed
/// `handle`, so it forms no `&ThreadControl`.
///
/// # Safety
///
/// `thread` must point to a [`ThreadControl`] that stays valid for the whole
/// wait — in practice the joined thread's pinned core state, which its joiner
/// keeps alive until this call returns.
pub unsafe fn wait_for_exit(thread: NonNull<ThreadControl>) -> Result<(), WaitError> {
    // Read the creation-fixed kernel handle through the raw pointer; forming a
    // `&ThreadControl` would race the target thread's self-mutation.
    // SAFETY: by the `# Safety` contract `thread` points to a valid
    // `ThreadControl`; `handle` is fixed at creation and never written again.
    let handle = unsafe { (*thread.as_ptr()).handle };
    // SAFETY: `handle` is the thread's kernel handle from `create` — a valid,
    // process-owned thread handle, never a pseudo-handle — and the slice
    // borrowing it stays live for the whole syscall.
    unsafe { wait_synchronization_single(&handle, u64::MAX) }.map_err(WaitError::from)
}

/// Blocks until any one of `threads` has exited, returning its index.
///
/// The multi-handle counterpart of [`wait_for_exit`]: it waits on every
/// thread's kernel handle at once and returns the index — into `threads` — of
/// the first to terminate. At most [`MAX_WAIT_HANDLES`] threads are waited on;
/// if `threads` is longer, only its leading prefix is waited and the returned
/// index still falls within that prefix, so the caller revisits the remainder.
///
/// Like [`wait_for_exit`], it reads only each thread's creation-fixed `handle`
/// through a raw pointer, forming no `&ThreadControl`.
///
/// # Panics
///
/// Panics if `threads` is empty — a zero-handle wait is a caller bug, and the
/// kernel rejects it regardless.
///
/// # Safety
///
/// Every pointer in `threads` must address a [`ThreadControl`] that stays valid
/// for the whole wait — in practice each thread's pinned core state, kept alive
/// by its reclaiming owner until this call returns.
pub unsafe fn wait_for_any_exit(threads: &[NonNull<ThreadControl>]) -> Result<usize, WaitError> {
    assert!(
        !threads.is_empty(),
        "wait_for_any_exit: the wait set must not be empty"
    );

    // Read each thread's creation-fixed kernel handle through its raw pointer;
    // forming a `&ThreadControl` would race the target thread's self-mutation.
    // The kernel caps a wait at `MAX_WAIT_HANDLES` handles, so collect
    // at most that many — the returned index then stays within the prefix.
    let handles: Vec<Handle> = threads
        .iter()
        .take(MAX_WAIT_HANDLES)
        .map(|thread| {
            // SAFETY: by the `# Safety` contract each pointer addresses a valid
            // `ThreadControl`; `handle` is fixed at creation, never rewritten.
            unsafe { (*thread.as_ptr()).handle }
        })
        .collect();

    // SAFETY: every handle is a thread's kernel handle from `create` — a valid,
    // process-owned thread handle, never a pseudo-handle — and the `Vec`
    // borrowing them stays live for the whole syscall.
    unsafe { wait_synchronization_multiple(handles.iter(), u64::MAX) }.map_err(WaitError::from)
}

/// Errors returned when waiting for a thread to exit via [`wait_for_exit`] or
/// [`wait_for_any_exit`].
#[derive(Debug, thiserror::Error)]
pub enum WaitError {
    /// Thread termination was requested while waiting.
    #[error("thread termination requested while waiting")]
    TerminationRequested,
    /// The supplied thread handle is invalid.
    #[error("invalid thread handle")]
    InvalidHandle,
    /// The kernel rejected the handle-array pointer (internal error).
    #[error("invalid handle-array pointer")]
    InvalidPointer,
    /// The wait timed out before the thread exited.
    #[error("wait timed out")]
    Timeout,
    /// The wait was canceled (`svcCancelSynchronization`).
    #[error("wait cancelled")]
    Cancelled,
    /// The kernel rejected the handle count as out of range.
    #[error("handle count out of range")]
    OutOfRange,
    /// Any other, unrecognized kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

impl From<WaitSyncError> for WaitError {
    fn from(err: WaitSyncError) -> Self {
        match err {
            WaitSyncError::TerminationRequested => WaitError::TerminationRequested,
            WaitSyncError::InvalidHandle => WaitError::InvalidHandle,
            WaitSyncError::InvalidPointer => WaitError::InvalidPointer,
            WaitSyncError::TimedOut => WaitError::Timeout,
            WaitSyncError::Cancelled => WaitError::Cancelled,
            WaitSyncError::OutOfRange => WaitError::OutOfRange,
            WaitSyncError::Unknown(err) => WaitError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl ToResultCode for WaitError {
    fn to_rc(self) -> ResultCode {
        match self {
            WaitError::TerminationRequested => KernelError::TerminationRequested.to_rc(),
            WaitError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            WaitError::InvalidPointer => KernelError::InvalidPointer.to_rc(),
            WaitError::Timeout => KernelError::TimedOut.to_rc(),
            WaitError::Cancelled => KernelError::Cancelled.to_rc(),
            WaitError::OutOfRange => KernelError::OutOfRange.to_rc(),
            WaitError::Unknown(err) => err.to_raw(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for WaitError {}

/// Pauses a running thread.
///
/// Delegates to `svcSetThreadActivity`. The operation is asynchronous: a
/// successful return only means the kernel accepted the pause request.
///
/// `thread` is a raw [`NonNull`] supplied by the caller; `pause` reads only the
/// creation-fixed `handle`, so it forms no `&ThreadControl`.
///
/// # Safety
///
/// `thread` must point to a [`ThreadControl`] valid for the duration of the
/// call.
pub unsafe fn pause(thread: NonNull<ThreadControl>) -> Result<(), PauseError> {
    // SAFETY: by the `# Safety` contract `thread` points to a valid
    // `ThreadControl`; `handle` is creation-fixed and never written by the
    // running thread.
    let handle = unsafe { (*thread.as_ptr()).handle };
    nx_svc::thread::pause(handle).map_err(|err| match err {
        PauseThreadError::InvalidHandle => PauseError::InvalidHandle,
        PauseThreadError::Unknown(err) => PauseError::Unknown(err),
    })
}

/// Errors returned when pausing a thread via [`pause`].
#[derive(Debug, thiserror::Error)]
pub enum PauseError {
    /// The supplied thread handle is invalid.
    #[error("invalid thread handle")]
    InvalidHandle,
    /// Any other, unrecognized kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

#[cfg(feature = "ffi")]
impl ToResultCode for PauseError {
    fn to_rc(self) -> ResultCode {
        match self {
            PauseError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            PauseError::Unknown(err) => err.to_raw(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for PauseError {}

/// Resumes a paused thread.
///
/// Delegates to `svcSetThreadActivity`. The operation is asynchronous: a
/// successful return only means the kernel accepted the resume request.
///
/// `thread` is a raw [`NonNull`] supplied by the caller; `resume` reads only
/// the creation-fixed `handle`, so it forms no `&ThreadControl`.
///
/// # Safety
///
/// `thread` must point to a [`ThreadControl`] valid for the duration of the
/// call.
pub unsafe fn resume(thread: NonNull<ThreadControl>) -> Result<(), ResumeError> {
    // SAFETY: by the `# Safety` contract `thread` points to a valid
    // `ThreadControl`; `handle` is creation-fixed and never written by the
    // running thread.
    let handle = unsafe { (*thread.as_ptr()).handle };
    nx_svc::thread::resume(handle).map_err(|err| match err {
        ResumeThreadError::InvalidHandle => ResumeError::InvalidHandle,
        ResumeThreadError::Unknown(err) => ResumeError::Unknown(err),
    })
}

/// Errors returned when resuming a thread via [`resume`].
#[derive(Debug, thiserror::Error)]
pub enum ResumeError {
    /// The supplied thread handle is invalid.
    #[error("invalid thread handle")]
    InvalidHandle,
    /// Any other, unrecognized kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

#[cfg(feature = "ffi")]
impl ToResultCode for ResumeError {
    fn to_rc(self) -> ResultCode {
        match self {
            ResumeError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            ResumeError::Unknown(err) => err.to_raw(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for ResumeError {}

/// Captures the CPU context of a paused thread.
///
/// Delegates to `svcGetThreadContext3`. The target thread must already be
/// paused (see [`pause`]) for the snapshot to be consistent. The returned
/// [`ThreadContext`] is the raw `nx-svc` register layout; the `ffi::libnx`
/// adapter owns projecting it onto the ABI-visible context type for C callers.
///
/// `thread` is a raw [`NonNull`] supplied by the caller; `dump_context` reads
/// only the creation-fixed `handle`, so it forms no `&ThreadControl`.
///
/// # Safety
///
/// `thread` must point to a [`ThreadControl`] valid for the duration of the
/// call.
pub unsafe fn dump_context(
    thread: NonNull<ThreadControl>,
) -> Result<ThreadContext, DumpContextError> {
    // SAFETY: by the `# Safety` contract `thread` points to a valid
    // `ThreadControl`; `handle` is creation-fixed and never written by the
    // running thread.
    let handle = unsafe { (*thread.as_ptr()).handle };
    nx_svc::thread::get_context3(handle).map_err(|err| match err {
        GetContext3Error::InvalidHandle => DumpContextError::InvalidHandle,
        GetContext3Error::Unknown(err) => DumpContextError::Unknown(err),
    })
}

/// Errors returned when capturing a thread's CPU context via [`dump_context`].
#[derive(Debug, thiserror::Error)]
pub enum DumpContextError {
    /// The supplied thread handle is invalid.
    #[error("invalid thread handle")]
    InvalidHandle,
    /// Any other, unrecognized kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

#[cfg(feature = "ffi")]
impl ToResultCode for DumpContextError {
    fn to_rc(self) -> ResultCode {
        match self {
            DumpContextError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            DumpContextError::Unknown(err) => err.to_raw(),
        }
    }
}

#[cfg(feature = "ffi")]
impl _sealed::Sealed for DumpContextError {}

/// Returns a raw pointer to the calling thread's authoritative
/// [`ThreadControl`], if it has one.
///
/// Reads `ThreadVars.thread_info_ptr`, which the thread bring-up path
/// points at the thread's core state. Returns `None` when the caller is not a
/// thread created or adopted by `nx-sys-thread`, so no core state is installed.
///
/// The result is a [`NonNull`], not a `&ThreadControl`: a borrow would be bound
/// to the thread's lifetime, not the process's, and there is no sound lifetime
/// to hand one out with. Forming a transient `&ThreadControl` from this pointer
/// is itself sound — `state`/`prev`/`next` are atomic, so the thread's
/// lock-free self-mutation does not race a shared reference — but the
/// unbounded lifetime, not a data race, is why this returns a raw pointer.
pub fn current() -> Option<NonNull<ThreadControl>> {
    let ptr = nx_sys_thread_tls::get_thread_info_ptr::<ThreadControl>();
    NonNull::new(ptr)
}

/// Returns whether the calling thread is the kernel-created main thread.
///
/// Compares the calling thread's [`current`] core-state pointer against the
/// process-static [`struct@MAIN_THREAD`] slot that [`init_main_thread`] adopts
/// the main thread into. The `ffi::libsysbase` adapter uses this to give the
/// main thread its sentinel pthread handle instead of running
/// [`crate::pthread::pthread_self`]'s container-of recovery on a thread with no
/// enclosing `PthreadControl` (Resolved Question #5).
///
/// Returns `false` before [`init_main_thread`] has run and on every thread
/// other than the main one.
#[cfg(feature = "ffi")]
pub fn is_main_thread() -> bool {
    // `&raw const` takes the static's address without forming a reference and
    // without reading its (possibly uninitialized) contents — `is_main_thread`
    // only compares pointers, never dereferences `MAIN_THREAD`.
    let main_ptr = (&raw const MAIN_THREAD).cast::<ThreadControl>();
    current().is_some_and(|cur| ptr::eq(cur.as_ptr(), main_ptr))
}

/// Returns the calling thread's Horizon kernel handle.
///
/// Reads `ThreadVars.handle` from the kernel TLS footer. Unlike [`current`] it
/// works on any thread, since the kernel maintains this field unconditionally.
pub fn get_current_handle() -> Handle {
    nx_sys_thread_tls::get_current_thread_handle()
}

/// Returns the id of the CPU core the calling thread is running on.
///
/// Delegates to `svcGetCurrentProcessorNumber`; the value is in `0..=3` on the
/// Switch's quad-core processor.
pub fn get_current_cpu() -> u32 {
    nx_svc::thread::get_current_processor_number()
}

/// Suspends the calling thread for *at least* `dur`.
///
/// Delegates to `svcSleepThread`. A [`Duration`] is non-negative by
/// construction, so this core helper needs no input validation: a C `timespec`
/// caller validates `tv_sec`/`tv_nsec` ranges at the FFI edge before folding
/// them into the `Duration`. The duration is reduced to whole nanoseconds and
/// capped at `i64::MAX` (~292 years) by [`nx_svc::thread::sleep`] — far beyond
/// any realistic sleep.
pub fn sleep(dur: Duration) {
    // `Duration::as_nanos` is a `u128`; saturate it into the `u64` `nx-svc`
    // accepts, which then caps the value at `i64::MAX` for the SVC.
    let nanos = u64::try_from(dur.as_nanos()).unwrap_or(u64::MAX);
    nx_svc::thread::sleep(nanos);
}

/// Yields the calling thread's remaining time slice to the scheduler.
///
/// Delegates to `svcSleepThread` with Horizon's core-migration yield value
/// (`-1`): the kernel reschedules another ready thread and may migrate the
/// caller to a different CPU core. This matches libnx `sched_yield`, which
/// issues `svcSleepThread(-1)`.
pub fn yield_thread() {
    nx_svc::thread::yield_with_migration();
}

/// Storage for the main thread's authoritative [`ThreadControl`].
///
/// The kernel creates the main thread before any Rust code runs, so — unlike a
/// spawned thread, whose `ThreadControl` lives inside its mapped stack block —
/// its core state has no natural per-thread home. [`init_main_thread`] performs
/// the single initializing write into this process-static slot; the address
/// then stays pinned for the whole process, satisfying the registry's
/// stable-address contract for every node it links (see [`thread_list`]).
static mut MAIN_THREAD: MaybeUninit<ThreadControl> = MaybeUninit::uninit();

/// The main thread's flat runtime TSD slot array.
///
/// Statically zeroed, mirroring the zero-initialized array [`create`] lays out
/// for a spawned thread; [`MAIN_RUNTIME`] points its `tsd` field here.
static mut MAIN_TSD: [*mut c_void; NUM_TSD_KEYS] = [ptr::null_mut(); NUM_TSD_KEYS];

/// The main thread's per-thread runtime TSD state.
///
/// Process-static, like [`MAIN_THREAD`]: a spawned thread keeps its
/// [`ThreadRuntime`] in the mapped stack block, but the kernel-created main
/// thread has none, so the record lives here. `tsd` is wired to [`MAIN_TSD`] at
/// const-init time; `tsd_used` starts clear.
static mut MAIN_RUNTIME: ThreadRuntime = ThreadRuntime {
    tsd: (&raw mut MAIN_TSD).cast::<*mut c_void>(),
    tsd_used: false,
};

/// Adopts the kernel-created main thread into `nx-sys-thread`'s core state.
///
/// The main thread exists before any Rust code runs: the kernel created it and
/// `nx-rt` installed its `ThreadVars` footer. This function completes the
/// adoption so the main thread becomes indistinguishable from a [`create`]d one
/// to the rest of the crate. It builds the process-static [`ThreadControl`],
/// discovers the kernel-owned stack via `svcQueryMemory`, registers it in the
/// live-thread registry, and points `ThreadVars.thread_info_ptr` at the core
/// state.
///
/// After it returns, [`current`] yields the main thread's `ThreadControl` and
/// the runtime TSD APIs ([`tsd::get`]/[`tsd::set`]) work on the main thread.
///
/// The main thread needs no [`Tcb`]/[`Dtv`](crate::tcb::Dtv) header: its ELF
/// TLS block is the loader-provided image and TP-relative access never walks a
/// crate-built TCB, while `current`/`exit`/`current_runtime` resolve it via
/// `ThreadVars.thread_info_ptr`. Building one would write into the process
/// `.bss` (`switch.ld` reserves no TCB span ahead of `.main.tls`), so this
/// function deliberately omits it — matching libnx, which writes no main-thread
/// TCB either.
///
/// The main thread's stack is kernel-owned, so [`ThreadControl::owns_stack_mem`]
/// is `false` and no backing pointer is tracked — [`close`] on it would only
/// drop the kernel handle. In practice the main thread is never closed; it
/// lives for the whole process.
///
/// # Panics
///
/// Panics — which aborts the process under `panic = "abort"` — if
/// `svcQueryMemory` cannot resolve the main thread's own stack mapping. That
/// would indicate a broken runtime rather than a recoverable condition, so it
/// is surfaced as a fatal bug, mirroring [`exit`]'s handling of a missing
/// `ThreadVars` footer.
///
/// # Safety
///
/// - Must be called exactly once, on the main thread, during process bring-up.
/// - `nx-rt` must have already installed the main thread's `ThreadVars` footer.
/// - No other `nx-sys-thread` thread API may have run on the main thread yet.
pub unsafe fn init_main_thread() {
    // The kernel maintains `ThreadVars.handle` for every thread, including the
    // main thread `nx-rt` brought up.
    let handle = nx_sys_thread_tls::get_current_thread_handle();

    // Discover the kernel-owned main thread stack: query the memory region that
    // contains a stack-local probe variable. `svcQueryMemory` reports the base
    // address and extent of the whole mapping.
    let probe = 0u8;
    let probe_addr = ptr::from_ref(&probe) as usize;
    let (mem_info, _) = match query_memory(probe_addr) {
        Ok(info) => info,
        Err(_) => {
            panic!("nx-sys-thread: init_main_thread() could not query the main thread stack")
        }
    };
    let Some(stack_mirror) = NonNull::new(mem_info.addr as *mut c_void) else {
        panic!("nx-sys-thread: init_main_thread() resolved a null main thread stack base");
    };

    // Build the process-static core state. The stack is kernel-owned, so no
    // backing allocation is tracked and `close` would only drop the handle.
    // `stack_size` records the whole `svcQueryMemory` mapping (`mem_info.size`),
    // not a usable extent — `stack_size()` documents this main-thread case.
    // SAFETY: `MAIN_RUNTIME` is a process-static `ThreadRuntime`; its address is
    // non-null and stays valid for the whole process lifetime.
    let runtime = unsafe { NonNull::new_unchecked(&raw mut MAIN_RUNTIME) };
    let main_ptr = (&raw mut MAIN_THREAD).cast::<ThreadControl>();
    // SAFETY: `MAIN_THREAD` is uninitialized process-static storage; this is its
    // single initializing write, guarded by the call-once `# Safety` clause.
    unsafe {
        main_ptr.write(ThreadControl {
            id: ThreadId::next(),
            handle,
            owns_stack_mem: false,
            stack_mem: None,
            stack_mirror,
            stack_size: mem_info.size,
            runtime,
            state: AtomicU8::new(ThreadState::Running as u8),
            prev: AtomicPtr::new(ptr::null_mut()),
            next: AtomicPtr::new(ptr::null_mut()),
        });
    }

    // The main thread needs no crate-built TCB/DTV header: its ELF TLS block is
    // the loader-provided image, TP-relative access never walks a crate TCB,
    // and `current`/`exit` resolve it via `ThreadVars.thread_info_ptr`.
    // `switch.ld` reserves no TCB span ahead of `.main.tls`, so writing one
    // would corrupt the `.bss` global the linker placed there.

    // Register in the live-thread list so runtime TSD key deletion and
    // lifecycle code reach the main thread like any other.
    // SAFETY: `main_ptr` is the freshly-initialized, not-yet-registered
    // `ThreadControl`, pinned for the whole process lifetime.
    unsafe { thread_list::insert(main_ptr) };

    // Point `ThreadVars.thread_info_ptr` at the core state; `nx-rt`
    // already installed the rest of the footer.
    // SAFETY: runs once, on the main thread; `main_ptr` stays valid forever.
    unsafe { nx_sys_thread_tls::set_thread_info_ptr(main_ptr) };
}

/// Trampoline that runs as a spawned thread's first instruction.
///
/// [`create`] hands this function's address to `svcCreateThread` and passes the
/// [`ThreadEntryArgs`] block as its argument. Before any user code runs, the
/// trampoline brings the thread up: it initializes the kernel TLS footer
/// (`ThreadVars`) so the thread can locate itself and so sync primitives can
/// read its handle, then registers the thread in the process-wide live-thread
/// list. It invokes the user entry point and terminates the thread once that
/// returns.
///
/// # Safety
///
/// - `args` must be the [`ThreadEntryArgs`] block written by [`create`] at the
///   new thread's stack top.
/// - `args.thread` must have been wired to the pinned [`ThreadControl`] by the
///   thread-start path before this thread was made runnable.
unsafe extern "C" fn entry_wrap(args: *mut ThreadEntryArgs) {
    // SAFETY: `create` writes a fully-initialized `ThreadEntryArgs` at the
    // stack top and hands this exact address to `svcCreateThread`; the
    // thread-start path fills `args.thread` before resuming this thread.
    let args = unsafe { &*args };
    let thread = args.thread;

    // Bring up the kernel TLS footer first: it carries the thread handle that
    // sync primitives read, so it must be valid before any user code or mutex
    // use. `ThreadVars.thread_info_ptr` points at the core `ThreadControl`;
    // the thread pointer is `__tls_start` walked back over the TCB span.
    // SAFETY: `args.thread` is the pinned `ThreadControl` for this thread,
    // valid for its entire lifetime.
    let handle = unsafe { (*thread).handle() };
    // SAFETY: `args.tls` points `tls_start_offset()` bytes past the TCB span,
    // so walking back by that span yields the in-bounds thread-pointer address.
    let tls_tp = unsafe { args.tls.sub(tls_block::tls_start_offset()) };
    // SAFETY: runs exactly once, on this thread, before any `ThreadVars` read.
    unsafe {
        nx_sys_thread_tls::init_thread_vars(
            handle,
            // SAFETY: `thread` is this thread's pinned `ThreadControl`.
            ThreadInfoPtr::from_ptr_unchecked(thread.cast::<c_void>()),
            // SAFETY: `args.reent` is the `_reent` slot carved for this thread.
            ReentPtr::from_ptr_unchecked(args.reent),
            // SAFETY: `tls_tp` is `args.tls` walked back over the TCB span, which is
            // this thread's thread-pointer value.
            ThreadPointer::from_ptr_unchecked(tls_tp.cast::<c_void>()),
        );
    }

    // Register in the process-wide live-thread list so runtime TLS key deletion
    // and lifecycle code can reach this thread. `insert` takes `THREAD_MUTEX`.
    // SAFETY: `thread` is a valid `ThreadControl`, not yet registered, that
    // stays live until its exit path removes it.
    unsafe { thread_list::insert(thread) };

    // SAFETY: by `entry_wrap`'s `# Safety` contract `args` is the block
    // `create` wrote, so `args.entry`/`args.arg` are the verbatim copy of
    // `create`'s `config.entry`/`config.arg` — a valid entry/arg pair by
    // `create`'s `# Safety` clause. The trampoline invokes it exactly once.
    unsafe { (args.entry)(args.arg) };

    // Tear the thread down through the core exit path: it runs the runtime TSD
    // destructors and unregisters the thread before `svcExitThread`.
    // SAFETY: this thread was brought up by `create` + the bring-up above, so
    // its `ThreadVars` footer is installed; `exit` runs once, as the final op.
    unsafe { exit() }
}

/// Returns the size, in bytes, of the per-thread control block reserved after
/// the usable stack and the [`ThreadEntryArgs`] block.
///
/// The block covers the TCB span, the ELF TLS image, the `_reent` slot, the
/// flat TSD array, and the [`ThreadRuntime`] record. It is rounded up to the
/// TCB span so the TCB — and the ELF TLS block immediately after it — land on a
/// [`tls_start_offset`](tls_block::tls_start_offset)-aligned (hence
/// `__tls_align`-aligned) address. [`create`] and [`close`] share this single
/// formula so the mapped extent they compute always agrees.
fn control_block_size() -> usize {
    let tcb_span = tls_block::tls_start_offset();
    let tls_sz = tls_block::tls_size();
    let tsd_sz = NUM_TSD_KEYS * size_of::<*mut c_void>();
    let runtime_sz = size_of::<ThreadRuntime>();
    align_up(
        tcb_span + tls_sz + reent_size() + tsd_sz + runtime_sz,
        tcb_span,
    )
}

/// Projects the per-thread control regions inside a thread's mapped stack
/// mirror.
///
/// [`create`] lays the mirror out as
/// `[ usable stack | entry args | TCB span | TLS block | reent | TSD | runtime ]`
/// and [`start`]/[`close`] later revisit those regions. `MirrorLayout` is the
/// single authoritative definition of that layout: every region offset is
/// derived here from the mirror base and the usable-stack size, so the call
/// sites cannot transcribe — and drift out of step with — the layout by hand.
struct MirrorLayout {
    /// Base of the mapped mirror — the bottom of the usable stack.
    base: NonNull<u8>,
    /// Size, in bytes, of the usable stack preceding the control regions.
    usable_stack: usize,
}

impl MirrorLayout {
    /// Builds a layout view over a thread's mapped stack mirror.
    ///
    /// # Safety
    /// `mirror` must be the base of a mapped stack mirror that [`create`] sized
    /// to hold the usable stack followed by every per-thread control region,
    /// and `usable_stack` must be that mirror's usable-stack size. The accessors
    /// derive in-bounds region pointers only under this contract.
    unsafe fn new(mirror: NonNull<c_void>, usable_stack: usize) -> Self {
        Self {
            base: mirror.cast::<u8>(),
            usable_stack,
        }
    }

    /// Entry-args block — also the top of the usable stack handed to
    /// `svcCreateThread`.
    fn entry_args(&self) -> NonNull<ThreadEntryArgs> {
        self.region(self.usable_stack).cast()
    }

    /// Thread Control Block at the start of the TCB span.
    fn tcb(&self) -> NonNull<Tcb> {
        self.region(self.tcb_offset()).cast()
    }

    /// ELF TLS block, immediately after the TCB span.
    fn tls_start(&self) -> NonNull<u8> {
        self.region(self.tls_offset())
    }

    /// Per-thread newlib `_reent` slot, between the ELF TLS block and the TSD
    /// array. Only the `ffi` build provisions a real block here.
    #[cfg(feature = "ffi")]
    fn reent(&self) -> NonNull<c_void> {
        self.region(self.reent_offset()).cast()
    }

    /// Flat per-thread TSD slot array.
    fn tsd(&self) -> NonNull<*mut c_void> {
        self.region(self.tsd_offset()).cast()
    }

    /// Per-thread [`ThreadRuntime`] record.
    fn runtime(&self) -> NonNull<ThreadRuntime> {
        self.region(self.runtime_offset()).cast()
    }

    /// Byte offset of the TCB span: past the usable stack and entry-args block.
    fn tcb_offset(&self) -> usize {
        self.usable_stack + size_of::<ThreadEntryArgs>()
    }

    /// Byte offset of the ELF TLS block: past the TCB span.
    fn tls_offset(&self) -> usize {
        self.tcb_offset() + tls_block::tls_start_offset()
    }

    /// Byte offset of the `_reent` slot: past the ELF TLS block.
    fn reent_offset(&self) -> usize {
        self.tls_offset() + tls_block::tls_size()
    }

    /// Byte offset of the TSD array: past the `_reent` slot.
    fn tsd_offset(&self) -> usize {
        self.reent_offset() + reent_size()
    }

    /// Byte offset of the [`ThreadRuntime`] record: past the TSD array.
    fn runtime_offset(&self) -> usize {
        self.tsd_offset() + NUM_TSD_KEYS * size_of::<*mut c_void>()
    }

    /// Pointer to the mirror region starting `offset` bytes into the mapping.
    fn region(&self, offset: usize) -> NonNull<u8> {
        // SAFETY: `new`'s contract guarantees the mirror spans the usable stack
        // followed by every control region, so `offset` — always within that
        // mapped extent — keeps the resulting pointer in bounds.
        unsafe { self.base.add(offset) }
    }
}

/// Rounds `value` up to the next multiple of `align`, which must be a power of
/// two.
///
/// Callers must ensure `value + align - 1` cannot overflow `usize`; use
/// [`checked_align_up`] when `value` is derived from untrusted input.
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Rounds `value` up to the next multiple of `align`, which must be a power of
/// two, returning `None` if the rounding would overflow `usize`.
const fn checked_align_up(value: usize, align: usize) -> Option<usize> {
    match value.checked_add(align - 1) {
        Some(sum) => Some(sum & !(align - 1)),
        None => None,
    }
}
