//! # Thread-Local Storage (TLS) for Horizon OS (Nintendo Switch)
//!
//! This crate provides foundational types and utilities for working with **Thread-Local Storage (TLS)**
//! on Nintendo Switch's Horizon OS. It consolidates all basic TLS initialization and access code
//! that was previously duplicated across `nx-sys-sync`, `nx-sys-thread`, and `nx-rt-env`.
//!
//! ## Horizon OS Thread-Local Storage Architecture
//!
//! ### Overview
//!
//! Every user-mode thread running on Horizon OS receives a dedicated 0x200-byte (512-byte) TLS region.
//! The **base address** of this region is exposed to user code via the AArch64 system register
//! **`TPIDRRO_EL0`** (Thread Pointer ID Register, Read-Only, Exception Level 0).
//!
//! The kernel initializes `TPIDRRO_EL0` during thread creation to point to the first byte of the
//! thread's TLS block. This register can be read by user code but not written to (read-only).
//!
//! Multiple threads' TLS regions are typically allocated within the same memory page. The first
//! TLS slot in a page (offset 0x000) is reserved for user-mode exception handling, so the first
//! usable thread TLS region typically begins at page base + 0x200.
//!
//! ### Memory Layout
//!
//! The complete 0x200-byte TLS block has the following layout:
//!
//! ```text
//! TLS base (TPIDRRO_EL0)
//! 0x000  ┌────────────────────────────┐
//!        │ IPC Message Buffer         │ 0x100 Bytes (256 bytes)
//!        │                            │ Used by kernel for IPC message passing
//! 0x100  ├────────────────────────────┤
//!        │ <Kernel reserved>          │ 8 bytes
//!        │                            │ Purpose unknown, reserved by Horizon OS
//! 0x108  ├────────────────────────────┤  ╮
//!        │ Dynamic TLS Slots (27)     │  │ 0xD8 bytes (216 bytes = 27 × 8)
//!        │   Slot  0                  │  │ Runtime-allocated thread-local storage
//!        │   Slot  1                  │  │ Used by pthread_key_create, C locale, etc.
//!        │   ...                      │  │ Each slot is pointer-sized (*mut c_void)
//!        │   Slot 26                  │  │
//! 0x1E0  ├────────────────────────────┤  ├ User TLS Region (0xD8 bytes)
//!        │ ThreadVars (32 bytes)      │  │
//!        │   0x00: magic     (u32)    │  │ Magic value "!TV$" (0x21545624)
//!        │   0x04: handle    (u32)    │  │ Kernel thread handle
//!        │   0x08: thread_ptr         │  │ Pointer to language-specific thread struct
//!        │   0x10: reent              │  │ newlib reentrancy state pointer
//!        │   0x18: tls_ptr            │  │ Thread pointer for __aarch64_read_tp()
//! 0x200  └────────────────────────────┘  ╯
//! ```
//!
//! ### Region Breakdown
//!
//! #### IPC Message Buffer (0x000 – 0x0FF)
//!
//! The first 256 bytes are used by the Horizon OS kernel for **Inter-Process Communication (IPC)**.
//! When a thread makes IPC calls to system services, request/response data is marshaled through
//! this buffer. User code should not directly access this region except through kernel syscalls.
//!
//! #### Dynamic TLS Slots (0x108 – 0x1DF)
//!
//! An array of **27 runtime-allocated pointers** (`NUM_TLS_SLOTS`) used by libnx to implement
//! thread-local storage that is *not known at link-time*. This is used for:
//!
//! - `pthread_key_create()` / `pthread_setspecific()` API
//! - C locale data (`setlocale()`)
//! - Dynamic thread-local variables allocated at runtime
//!
//! **How it works:**
//! - Each slot can hold a `*mut c_void` value (pointer or small integer cast to usize)
//! - Slot IDs are **process-global**: all threads share the same slot ID → data mapping
//! - Each **thread** has its own copy of the 27 slots in its TLS region
//! - A process-global bitmask tracks which slot IDs are in use
//! - Optional *destructor* functions can be registered to run cleanup when a thread exits
//! - Access is purely arithmetic: `TPIDRRO_EL0 + 0x108 + (slot_id * 8)` — no syscalls needed
//!
//! In libnx C API: `threadTlsAlloc()`, `threadTlsSet()`, `threadTlsGet()`
//!
//! #### ThreadVars Structure (0x1E0 – 0x1FF)
//!
//! A fixed 32-byte footer containing per-thread metadata that both the kernel and userspace
//! runtime consult frequently. See [`ThreadVars`] for detailed field descriptions.
//!
//! **Critical invariant:** The `tls_ptr` field at offset 0x1F8 (last 8 bytes) **must** point
//! to the thread-local data segment base to satisfy the AArch64 ABI requirement for
//! `__aarch64_read_tp()`.
//!
//! ## libnx Thread Initialization Sequence
//!
//! Understanding when and how TLS is initialized is critical because the allocator depends on
//! TLS being properly set up (allocator uses mutexes → mutexes read thread handle from TLS).
//!
//! ### Initialization Order (from libnx C runtime)
//!
//! When a homebrew application launches on Nintendo Switch, the following sequence occurs:
//!
//! ```text
//! 1. __nx_start (crt0)              Entered from kernel
//!    ├─ Set up stack pointer
//!    └─ Call __libnx_init()
//!
//! 2. __libnx_init()
//!    ├─ [a] envSetup()              Parse homebrew environment (loader config)
//!    ├─ [b] newlibSetup()           ← INITIALIZE TLS ThreadVars (THIS CRATE)
//!    ├─ [c] virtmemSetup()          Set up virtual memory mappings
//!    └─ [d] __libnx_initheap()      Allocate heap (REQUIRES TLS!)
//!                                    └─ Internally uses mutexes
//!                                       └─ Mutexes read thread handle from TLS
//!
//! 3. main()                          User code begins execution
//! ```
//!
//! **Why order matters:**
//!
//! - **envSetup() FIRST**: Must run before anything else to parse the loader configuration
//!   and discover the main thread handle (stored in the environment)
//!
//! - **newlibSetup() SECOND**: Must run *before* heap/allocator initialization because:
//!   1. The allocator uses `nx-sys-sync` mutexes for thread-safety
//!   2. Mutexes need to read the current thread's handle from `ThreadVars.handle` (TLS offset 0x1E4)
//!   3. If `ThreadVars` is not initialized, mutex operations will read **garbage data** → undefined behavior
//!
//! - **virtmemSetup() THIRD**: Sets up address space mappings for heap
//!
//! - **__libnx_initheap() FOURTH**: Now safe to initialize the allocator
//!
//! ### newlibSetup() - Main Thread TLS Initialization
//!
//! The Rust port of `newlibSetup()` lives in `nx-rt-env::main_thread::setup()`. It performs
//! these tasks:
//!
//! 1. **Read TLS base**: Via `TPIDRRO_EL0` register (already set up by kernel)
//! 2. **Calculate ThreadVars location**: TLS base + 0x1E0
//! 3. **Write ThreadVars fields**:
//!    - `magic`: Set to `0x21545624` ("!TV$" magic value)
//!    - `handle`: Main thread handle from `envGetMainThreadHandle()`
//!    - `thread_ptr`: Set to null (filled later by thread registry)
//!    - `reent`: Pointer to newlib's global reentrancy structure (`_impure_ptr`)
//!    - `tls_ptr`: Calculated as `__tls_start - getTlsStartOffset()` (for AArch64 ABI)
//! 4. **Copy .tdata section**: Copy initialized thread-local data from ELF to TLS block
//!
//! ### Subsequent Thread Creation
//!
//! When spawning a new thread (not the main thread), the initialization sequence is:
//!
//! 1. Allocate stack memory
//! 2. Allocate TLS region (0x200 bytes)
//! 3. Copy `.tdata` section (initialized TLS variables) to new TLS block
//! 4. Zero `.tbss` section (uninitialized TLS variables)
//! 5. Initialize `ThreadVars` structure using [`init_thread_vars()`]
//! 6. Call `svcCreateThread()` with entry point, stack, and TLS
//! 7. Call `svcStartThread()`
//!
//! ## AArch64 ABI Requirement: `__aarch64_read_tp()`
//!
//! The **AArch64 Procedure Call Standard (AAPCS64)** defines a standard mechanism for accessing
//! thread-local variables (`__thread` in C, `thread_local!` in Rust).
//!
//! Compilers generate calls to a runtime function **`__aarch64_read_tp()`** to obtain a base
//! pointer for thread-local variable access. This function **must** return the address of the
//! thread's TLS data segment.
//!
//! ### Implementation in libnx
//!
//! The `__aarch64_read_tp()` function is implemented in assembly at
//! `libnx/nx/source/runtime/readtp.s`:
//!
//! ```assembly
//! .global __aarch64_read_tp
//! .type __aarch64_read_tp, %function
//! __aarch64_read_tp:
//!     mrs x0, tpidrro_el0     // Read TLS base from system register
//!     ldr x0, [x0, #0x1F8]    // Load value from [TLS_base + 0x1F8]
//!     ret                     // Return the thread pointer
//! ```
//!
//! **This is why `ThreadVars.tls_ptr` MUST be at offset 0x1F8:**
//!
//! - The assembly hardcodes the load from offset `0x1F8`
//! - `ThreadVars` is exactly 32 bytes (0x20), starting at offset 0x1E0
//! - Field layout: `[magic:4][handle:4][thread_ptr:8][reent:8][tls_ptr:8]`
//! - `tls_ptr` is the last field → offset = `0x1E0 + 0x18 = 0x1F8` ✓
//!
//! The value stored in `tls_ptr` typically points to `__tls_start` (the beginning of the
//! thread's TLS data segment), adjusted for the Thread Control Block (TCB) alignment.
//!
//! ## Safety and Undefined Behavior
//!
//! **CRITICAL:** Accessing TLS before it is initialized leads to **undefined behavior**:
//!
//! - Reading `ThreadVars.handle` before `init_thread_vars()` → garbage data → mutex corruption
//! - Using allocator before TLS is ready → mutex reads garbage → deadlock or corruption
//! - Calling `__aarch64_read_tp()` with uninitialized `tls_ptr` → segfault or data corruption
//!
//! **Safe initialization order:**
//! 1. Kernel sets `TPIDRRO_EL0` (done automatically during thread creation)
//! 2. Call `init_thread_vars()` **once** during thread startup
//! 3. Now safe to use allocator, mutexes, thread-local variables
//!
//! ## References
//!
//! - [Switchbrew Wiki: Thread Local Region](https://switchbrew.org/wiki/Thread_Local_Region)
//! - [switchbrew/libnx: tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)
//! - [ARM: Thread-Local Storage](https://developer.arm.com/documentation/100748/0624/Thread-Local-Storage)

#![no_std]

extern crate nx_panic_handler; // Provides #[panic_handler]

use core::{cell::UnsafeCell, ffi::c_void, mem::offset_of, ptr, ptr::NonNull};

use nx_cpu::control_regs;
use nx_svc::thread::Handle as ThreadHandle;
use static_assertions::{assert_not_impl_any, const_assert_eq};

#[cfg(feature = "ffi")]
pub mod ffi;

/// Size of the Thread Local Storage (TLS) region in bytes.
///
/// Every thread on Horizon OS receives a 0x200-byte (512-byte) TLS region. This is a
/// kernel-enforced constant and cannot be changed.
pub const TLS_REGION_SIZE: usize = 0x200;

/// Start offset of the user-mode TLS region within the TLS block.
///
/// The first 0x108 bytes (264 bytes) are reserved by the kernel:
/// - 0x000–0x0FF: IPC message buffer (256 bytes)
/// - 0x100–0x107: Kernel reserved (8 bytes)
///
/// From offset 0x108 onward, the TLS region is available for user-mode code.
pub const USER_TLS_REGION_BEGIN: usize = 0x108;

/// End offset of the user-mode TLS region (exclusive).
///
/// The user-mode TLS region extends from 0x108 to 0x1E0 (where `ThreadVars` begins).
/// This gives 0xD8 bytes (216 bytes) of user-accessible TLS storage.
pub const USER_TLS_REGION_END: usize = TLS_REGION_SIZE - THREAD_VARS_SIZE;

/// Number of dynamic TLS slots available per thread.
///
/// Each thread has 27 pointer-sized slots (0x108 bytes = 27 × 8) for runtime-allocated
/// thread-local storage. These slots are located at TLS offsets 0x108–0x1DF.
pub const NUM_TLS_SLOTS: usize = 27;

/// Size of the [`ThreadVars`] structure in bytes.
///
/// The `ThreadVars` structure is exactly 32 bytes (0x20) and is located at the end
/// of the thread's TLS region (offset 0x1E0).
pub const THREAD_VARS_SIZE: usize = 0x20;

/// Magic value used to verify that the [`ThreadVars`] structure is initialized.
///
/// The value `0x21545624` corresponds to the ASCII string "!TV$" (little-endian).
/// During thread initialization, this magic value is written to `ThreadVars.magic`
/// to indicate that the structure has been properly set up.
pub const THREAD_VARS_MAGIC: u32 = 0x21545624;

/// Size of the IPC message buffer in bytes.
///
/// The IPC buffer occupies the first [`IPC_BUFFER_SIZE`] bytes of the Thread Local
/// Region. The Horizon OS kernel reads request data from and writes response data
/// to this region when a thread makes an IPC call.
pub const IPC_BUFFER_SIZE: usize = 0x100;

/// Returns the base address of this thread's Thread-Local Storage (TLS) block as a plain `usize`.
///
/// On AArch64, the per-thread TLS pointer is exposed to user-mode code via the read-only
/// system register `TPIDRRO_EL0`. Horizon OS initializes this register during thread creation
/// to point at the first byte of the 0x200-byte TLS block.
///
/// This function is a thin, safe wrapper around a single `mrs` instruction that reads that
/// register. Because merely reading the register cannot violate any safety guarantees, the
/// function is safe to call; however, any *use* of the returned address (e.g., by dereferencing
/// it) must observe the TLS layout documented in this module.
///
/// If you need a raw pointer instead of an integer address, use [`get_ptr()`].
#[inline]
pub fn get_base_addr() -> usize {
    // SAFETY: Reading TPIDRRO_EL0 is a side-effect-free operation that returns the
    // kernel-initialized TLS base address. The register is read-only in user mode.
    unsafe { control_regs::tpidrro_el0() }
}

/// Returns a raw pointer to the 512-byte Thread Local Storage (TLS) for the current thread.
///
/// This is simply [`get_base_addr()`] cast to a pointer, so obtaining the value is completely
/// safe. **Dereferencing** the pointer, however, requires `unsafe` code and must respect the
/// TLS layout documented in this module.
#[inline]
pub fn get_ptr() -> *mut ThreadLocalRegion {
    get_base_addr() as *mut ThreadLocalRegion
}

/// Returns a pointer to the current thread's IPC buffer.
///
/// The IPC buffer is a [`IPC_BUFFER_SIZE`]-byte region at the start of the Thread
/// Local Region used by the Horizon OS kernel for inter-process communication
/// messages. When a thread makes IPC calls to system services, request/response
/// data is marshaled through this buffer.
///
/// # Returns
///
/// A `NonNull<u8>` pointing to the IPC buffer. The pointer is valid for the
/// lifetime of the current thread.
#[inline]
pub fn ipc_buffer_ptr() -> NonNull<u8> {
    let tls = get_ptr();

    // SAFETY: get_ptr() returns a valid pointer to the current thread's TLS block.
    // The TLS region is always valid for the current thread, so ipc_buffer is non-null.
    unsafe { NonNull::new_unchecked((*tls).ipc_buffer.as_mut_ptr()) }
}

/// Borrows the current thread's IPC buffer as an [`IpcBuffer`].
///
/// # Safety
///
/// No other [`IpcBuffer`] may be live on this thread for the lifetime of
/// the returned value. Two live tokens on the same thread would allow
/// aliased `&mut [u8; IPC_BUFFER_SIZE]` to be constructed via
/// [`IpcBuffer::as_array_mut`], which is undefined behavior.
///
/// Note that — unlike earlier revisions of this API — the token itself
/// may legally outlive a kernel write to the buffer (e.g. across
/// `svcSendSyncRequest`). See [`IpcBuffer`] for the model.
#[inline]
pub unsafe fn ipc_buffer() -> IpcBuffer {
    // SAFETY: ipc_buffer_ptr always points to a valid IPC_BUFFER_SIZE-byte
    // region in the current thread's TLS. Reinterpreting that pointer as
    // `*mut UnsafeCell<[u8; N]>` is layout-compatible because `UnsafeCell`
    // is `#[repr(transparent)]` over its inner type. Caller upholds the
    // singleton-per-thread contract documented above.
    let ptr = ipc_buffer_ptr().cast::<UnsafeCell<[u8; IPC_BUFFER_SIZE]>>();
    IpcBuffer { ptr }
}

/// Thread-affine token granting access to the current thread's IPC buffer.
///
/// Constructed via [`ipc_buffer`]; flowed into IPC marshaling code
/// (`nx-sf`) so the rest of the IPC layer operates on a typed, sized
/// view and never re-derives the buffer.
///
/// # Soundness model
///
/// The IPC buffer is a 0x100-byte region in this thread's TLS that the
/// **kernel mutates** during syscalls such as `svcSendSyncRequest`. That
/// makes its memory model fundamentally different from ordinary Rust
/// storage and requires careful type-level encoding:
///
/// 1. **Thread affinity.** The buffer lives in this thread's TLS only.
///    `IpcBuffer`'s sole field is a [`NonNull`], which wraps a raw
///    pointer and therefore inherits `!Send + !Sync` via auto-trait
///    inference. The compiler prevents the token from being moved to or
///    shared with another thread. A static assertion below pins this
///    guarantee so an accidental future field cannot silently restore
///    [`Send`] or [`Sync`].
///
/// 2. **Singleton per thread.** Constructing two tokens on one thread
///    would allow aliased `&mut [u8; IPC_BUFFER_SIZE]` references to the
///    same memory — instant UB. This invariant is delegated to the
///    `unsafe` constructor [`ipc_buffer`]; the type cannot enforce it
///    without runtime tracking.
///
/// 3. **Kernel writes do not break the `&` contract.** A plain
///    `&[u8; N]` (or `&mut`) promises the compiler that the bytes are
///    stable for the borrow's lifetime — nothing else, anywhere, will
///    read or write them. The kernel's store during a syscall violates
///    that promise, so naively holding `&self.bytes` across the syscall
///    would be UB even though no Rust code runs in the meantime.
///
///    To express "these bytes may be mutated through paths other than
///    this reference," the underlying array is wrapped in
///    [`UnsafeCell`]. `UnsafeCell` is the *only* language-level mechanism
///    that lifts the stability promise from a shared reference; every
///    interior-mutability primitive (`Cell`, `Mutex`, atomics, …) is
///    built on it. Because `UnsafeCell` is layout-transparent, the TLS
///    pointer can be reinterpreted as `*mut UnsafeCell<[u8; N]>` even
///    though Rust does not own the underlying memory.
///
///    Consequently, `&IpcBuffer` (and `&mut IpcBuffer`) **may legally
///    outlive a kernel write to the buffer**. The plumbing the token
///    holds — a `NonNull` to an `UnsafeCell` — makes no promise the
///    kernel would falsify.
///
/// 4. **Short-lived `&[u8; N]` borrows still must not span a kernel
///    write.** [`as_array`], [`as_array_mut`], and the [`Deref`] /
///    [`DerefMut`] impls hand out plain references to the array. Those
///    references *do* carry the standard stability/exclusivity promise
///    and would be UB if held across a syscall that mutates the buffer.
///    In practice the IPC syscall wrapper takes `&mut IpcBuffer`, which
///    reborrows the token uniquely and statically invalidates any
///    outstanding `as_array`/`Deref` borrow at the call site — so this
///    obligation is discharged by the borrow checker, not by hand.
///
/// # Summary of obligations
///
/// | Concern                              | Discharged by                                       |
/// |--------------------------------------|-----------------------------------------------------|
/// | Thread affinity                      | `NonNull` field → `!Send + !Sync` (asserted below)  |
/// | Single token per thread              | `unsafe fn ipc_buffer()` precondition               |
/// | Token may outlive a kernel write     | `UnsafeCell<[u8; N]>` interior (no stability promise) |
/// | No `&[u8; N]` borrow across syscall  | syscall wrapper takes `&mut IpcBuffer`              |
pub struct IpcBuffer {
    // `UnsafeCell` is what tells the compiler the bytes here may be mutated
    // through paths other than any live `&` to them — specifically, the
    // kernel during an IPC syscall. Without it, holding `&IpcBuffer` across
    // such a syscall would violate the shared-reference stability promise.
    //
    // The pointee lives in TLS (not in this struct); `NonNull` is just the
    // handle. `UnsafeCell` is `#[repr(transparent)]`, so reinterpreting the
    // raw TLS address as `*mut UnsafeCell<[u8; N]>` is layout-correct.
    // `NonNull` wraps a raw pointer, so it is neither `Send` nor `Sync`;
    // that auto-trait blocking propagates to `IpcBuffer` and enforces
    // thread-affinity without a dedicated `PhantomData` marker. The
    // `assert_not_impl_any!` below pins the guarantee.
    ptr: NonNull<UnsafeCell<[u8; IPC_BUFFER_SIZE]>>,
}

// Thread-affinity guard: if a future field accidentally restores `Send`
// or `Sync` (e.g. wrapping `ptr` in an `Arc`), this assertion fails at
// compile time. See the type docs for why both must remain blocked.
assert_not_impl_any!(IpcBuffer: Send, Sync);

impl IpcBuffer {
    /// Borrowed view of the underlying fixed-size array.
    ///
    /// The returned reference carries the standard `&T` stability promise
    /// and therefore **must not be held across an operation that lets the
    /// kernel mutate the buffer**. In normal use this is enforced
    /// automatically: the IPC syscall wrapper takes `&mut IpcBuffer`,
    /// which invalidates this `&self`-derived borrow at the call site.
    #[inline]
    pub fn as_array(&self) -> &[u8; IPC_BUFFER_SIZE] {
        // SAFETY:
        // - `ptr` references the current thread's TLS IPC buffer, valid
        //   for the thread's lifetime (`!Send`/`!Sync` confines use to it).
        // - The `UnsafeCell` interior makes the conversion to `&` sound
        //   even though the kernel may write the bytes through a side
        //   channel; only the returned reference's *own* lifetime carries
        //   the stability promise, and the caller-side borrow-check
        //   prevents that lifetime from spanning a syscall (see doc above).
        // - `&self` excludes a concurrent `as_array_mut`/`DerefMut`, so
        //   no aliased `&mut` to the same bytes can exist.
        unsafe { &*self.ptr.as_ref().get() }
    }

    /// Borrowed mutable view of the underlying fixed-size array.
    ///
    /// The returned reference carries the standard `&mut T`
    /// exclusivity/stability promise. The same "must not span a kernel
    /// write" obligation as [`as_array`] applies, and is enforced the
    /// same way at IPC call sites.
    #[inline]
    pub fn as_array_mut(&mut self) -> &mut [u8; IPC_BUFFER_SIZE] {
        // SAFETY: Validity argument as in `as_array`. `&mut self` provides
        // unique access to the token, and the singleton-per-thread
        // invariant on `ipc_buffer()` ensures no other token can hand out
        // an overlapping borrow.
        unsafe { &mut *self.ptr.as_ref().get() }
    }
}

impl core::ops::Deref for IpcBuffer {
    type Target = [u8; IPC_BUFFER_SIZE];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_array()
    }
}

impl core::ops::DerefMut for IpcBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_array_mut()
    }
}

/// Returns a raw pointer to the TLS dynamic slots array for the current thread.
///
/// The dynamic slots are an array of [`NUM_TLS_SLOTS`] (27) pointer-sized entries located
/// at TLS base + 0x108. Each slot can hold a `*mut c_void` value and is used for
/// runtime-allocated thread-local storage.
///
/// ```text
/// TLS base + 0x108 ──┐
///                    ├─ Slot 0  (*mut c_void)  ← returned pointer
///                    ├─ Slot 1  (*mut c_void)
///                    ├─ Slot 2  (*mut c_void)
///                    ┆        ...
///                    └─ Slot 26 (*mut c_void)
/// ```
///
/// # Returns
///
/// A `NonNull<*mut c_void>` pointing to the first slot. The pointer is valid for the
/// lifetime of the current thread and points to an array of [`NUM_TLS_SLOTS`] entries.
#[inline]
pub fn slots_ptr() -> NonNull<*mut c_void> {
    let tls = get_ptr();

    // SAFETY: The TLS region pointer is valid for the lifetime of the current thread.
    unsafe { NonNull::new_unchecked((*tls).slots.as_mut_ptr()) }
}

/// Returns a raw pointer to the [`ThreadVars`] for the current thread.
///
/// The `ThreadVars` structure is located at the end of the TLS block, at offset
/// `TLS_REGION_SIZE - THREAD_VARS_SIZE` (0x1E0).
#[inline]
pub fn thread_vars_ptr() -> *mut ThreadVars {
    let tls = get_ptr();

    // SAFETY: The TLS region pointer is valid for the lifetime of the current thread.
    unsafe { &raw mut (*tls).thread_vars }
}

/// Initializes the current thread's [`ThreadVars`] TLS footer.
///
/// Internally, the OS reserves the final 0x20 bytes of each 0x200-byte TLS block for a
/// small structure containing per-thread metadata that both the kernel and userspace runtime
/// consult frequently.
///
/// This function must be invoked exactly **once** during thread start-up, after a fresh TLS
/// block has been allocated and before any user code attempts to read thread metadata. The
/// supplied values are copied verbatim into the footer located at `TLS_base + 0x1E0`.
///
/// The three pointers the footer carries are distinguished by type rather than by
/// position, because as bare `*mut c_void` any two of them could be exchanged here
/// without the compiler objecting, and the resulting corruption would surface in
/// unrelated thread-local state long after this call.
///
/// # Safety
///
/// This routine mutates the TLS memory that the CPU is *actively* using for the current thread.
/// Callers must guarantee the following:
///
/// 1. The executing core is indeed running the thread whose TLS is being modified.
/// 2. No other code is concurrently accessing an uninitialized [`ThreadVars`].
/// 3. This function is called exactly **once** per thread during initialization.
///
/// Failing to uphold these requirements will lead to **undefined behavior**, up to and
/// including memory corruption in unrelated threads.
#[inline]
pub unsafe fn init_thread_vars(
    handle: ThreadHandle,
    thread_info_ptr: ThreadInfoPtr,
    reent: ReentPtr,
    tls_ptr: ThreadPointer,
) {
    let thread_vars = thread_vars_ptr();

    // SAFETY: Caller guarantees this is called exactly once during thread initialization,
    // on the thread whose TLS is being modified, with no concurrent access to ThreadVars.
    // The pointer from `thread_vars_ptr()` is valid for the current thread's TLS block.
    unsafe {
        thread_vars.write(ThreadVars {
            magic: THREAD_VARS_MAGIC,
            handle,
            thread_info_ptr,
            reent,
            tls_ptr,
        });
    }
}

/// Returns a type-safe pointer to the current thread's language-specific thread object.
///
/// This is a generic accessor for the `ThreadVars.thread_info_ptr` field. It returns
/// `*mut T` instead of `*mut c_void`, allowing type-safe access to thread objects.
#[inline]
pub fn get_thread_info_ptr<T>() -> *mut T {
    let tv_ptr = thread_vars_ptr();
    // SAFETY: Reading the thread_info_ptr field from ThreadVars with volatile semantics
    // to prevent compiler optimizations from reordering the load.
    unsafe { ptr::read_volatile(&raw const (*tv_ptr).thread_info_ptr) }
        .to_raw()
        .cast()
}

/// Sets the current thread's language-specific thread object pointer.
///
/// This is a generic setter for the `ThreadVars.thread_info_ptr` field.
///
/// # Safety
///
/// - The pointer must be valid for the lifetime of the thread
/// - Must not be called concurrently with reads/writes to the same field
#[inline]
pub unsafe fn set_thread_info_ptr<T>(ptr: *mut T) {
    let tv_ptr = thread_vars_ptr();
    // SAFETY: Writing the thread_info_ptr field with volatile semantics
    // SAFETY: The caller's contract is that `ptr` is this thread's thread object.
    let thread_info_ptr = ThreadInfoPtr::from_ptr_unchecked(ptr.cast());
    unsafe { ptr::write_volatile(&raw mut (*tv_ptr).thread_info_ptr, thread_info_ptr) };
}

/// Returns the [`Handle`] of the current thread.
///
/// This function reads the kernel handle from the `ThreadVars.handle` field at TLS offset 0x1E4.
/// A `read_volatile` is used to prevent the compiler from tearing or caching the value across
/// multiple calls, which is important when the scheduler might reschedule the thread to a
/// different core between two loads.
///
/// The returned handle identifies the thread to the kernel and is used by synchronization
/// primitives (mutexes, condition variables) to identify the owning thread.
#[inline]
pub fn get_current_thread_handle() -> ThreadHandle {
    let tv = thread_vars_ptr();

    // SAFETY: `tv` points to a valid `ThreadVars` inside the current thread's TLS block.
    // The field access is performed with `read_volatile` to avoid the compiler re-ordering
    // or eliminating the read.
    unsafe { ptr::read_volatile(&raw const (*tv).handle) }
}

/// Complete Thread-Local Storage (TLS) region layout.
///
/// This struct represents the exact memory layout of the 0x200-byte TLS block
/// that Horizon OS allocates for each thread. It is `#[repr(C)]` to ensure
/// the memory layout matches libnx's expectations.
///
/// ## Memory Layout
///
/// ```text
/// 0x000  ┌────────────────────────────┐
///        │ ipc_buffer                 │ 0x100 bytes (256 bytes)
/// 0x100  ├────────────────────────────┤
///        │ kernel_reserved            │ 8 bytes
/// 0x108  ├────────────────────────────┤
///        │ slots[27]                  │ 0xD8 bytes (216 bytes = 27 × 8)
/// 0x1E0  ├────────────────────────────┤
///        │ thread_vars                │ 0x20 bytes (32 bytes)
/// 0x200  └────────────────────────────┘
/// ```
///
/// ## Note on Kernel vs libnx Layout
///
/// The kernel defines a different layout at 0x100–0x200 (see Switchbrew wiki). This struct
/// uses libnx's homebrew interpretation since this crate replaces libnx functions.
#[derive(Debug)]
#[repr(C)]
pub struct ThreadLocalRegion {
    /// IPC message buffer used by kernel for inter-process communication.
    ///
    /// When a thread makes IPC calls to system services, request/response
    /// data is marshaled through this buffer.
    pub ipc_buffer: [u8; 0x100],

    /// Kernel-reserved 8 bytes.
    ///
    /// In the actual kernel layout, this region contains `DisableCounter` (u16),
    /// `InterruptFlag` (u16), and other fields. libnx treats it as reserved.
    _kernel_reserved: u64,

    /// Dynamic TLS slots for runtime-allocated thread-local storage.
    ///
    /// Each slot can hold a `*mut c_void` value and is used for:
    /// - `pthread_key_create()` / `pthread_setspecific()` API
    /// - C locale data (`setlocale()`)
    /// - Dynamic thread-local variables allocated at runtime
    pub slots: [*mut c_void; NUM_TLS_SLOTS],

    /// Per-thread variables structure containing thread metadata.
    pub thread_vars: ThreadVars,
}

// Compile-time layout assertions for ThreadLocalRegion
const_assert_eq!(size_of::<ThreadLocalRegion>(), TLS_REGION_SIZE);
const_assert_eq!(offset_of!(ThreadLocalRegion, ipc_buffer), 0x000);
const_assert_eq!(offset_of!(ThreadLocalRegion, _kernel_reserved), 0x100);
const_assert_eq!(offset_of!(ThreadLocalRegion, slots), USER_TLS_REGION_BEGIN);
const_assert_eq!(
    offset_of!(ThreadLocalRegion, thread_vars),
    TLS_REGION_SIZE - THREAD_VARS_SIZE
);

/// Per-thread variables located at the end of the TLS area.
///
/// The `ThreadVars` structure occupies exactly [`THREAD_VARS_SIZE`] bytes (32 bytes)
/// and is located at TLS offset 0x1E0. It contains essential per-thread metadata
/// that both the kernel and userspace runtime consult frequently.
///
/// ## Memory Layout
///
/// ```text
/// TLS base + 0x1E0
/// 0x1E0 ┌────────────────────────────┐
///       │ magic       (u32)          │ 4 bytes  - Magic value "!TV$"
/// 0x1E4 ├────────────────────────────┤
///       │ handle      (u32)          │ 4 bytes  - Kernel thread handle
/// 0x1E8 ├────────────────────────────┤
///       │ thread_ptr  (*mut c_void)  │ 8 bytes  - Thread object pointer
/// 0x1F0 ├────────────────────────────┤
///       │ reent       (*mut c_void)  │ 8 bytes  - newlib reentrancy state
/// 0x1F8 ├────────────────────────────┤
///       │ tls_ptr     (*mut c_void)  │ 8 bytes  - Thread pointer (MUST be at 0x1F8!)
/// 0x200 └────────────────────────────┘
/// ```
///
/// ## Field Descriptions
///
/// ### `magic: u32`
///
/// Magic value used to check if the structure is initialized. Set to [`THREAD_VARS_MAGIC`]
/// (`0x21545624`, ASCII "!TV$") during initialization.
///
/// ### `handle: Handle`
///
/// Kernel handle identifying the thread. This is the handle returned by `svcCreateThread()`
/// and is used by synchronization primitives (mutexes, condition variables) to identify
/// the owning thread.
///
/// **CRITICAL:** This field is read by mutex operations in `nx-sys-sync`. If not properly
/// initialized, mutex locks will read garbage data and cause undefined behavior.
///
/// ### `thread_info_ptr: *mut c_void`
///
/// Pointer to the language-specific thread object. In Rust, this typically points to
/// a `Thread` structure (from `nx-sys-thread`). In C, it might point to a `struct Thread`.
///
/// Initially set to null during TLS initialization. Later filled by the thread registry
/// when the thread is registered.
///
/// **Type-safe access:** Use [`get_thread_info_ptr::<T>()`] to retrieve this pointer as `*mut T`.
///
/// ### `reent: *mut c_void`
///
/// Pointer to the thread's newlib reentrancy state (`struct _reent` in C). This is used
/// by newlib's C standard library functions to maintain per-thread state for things like
/// `errno`, `strtok()`, and file descriptors.
///
/// For the main thread, this points to the global `_impure_ptr`. For spawned threads,
/// this points to a dynamically allocated `_reent` structure.
///
/// ### `tls_ptr: *mut c_void`
///
/// Pointer to this thread's thread-local segment (TP). This field **MUST** be located at
/// exactly offset 0x1F8 within the TLS block to comply with the **AArch64 ABI**.
///
/// **AArch64 ABI Requirement:**
///
/// The AArch64 Procedure Call Standard (AAPCS64) defines `__aarch64_read_tp()` as the
/// standard function for obtaining the thread pointer. Compilers generate calls to this
/// function when accessing `__thread` / `thread_local!` variables.
///
/// The libnx implementation of `__aarch64_read_tp()` (in assembly) is:
///
/// ```assembly
/// __aarch64_read_tp:
///     mrs x0, tpidrro_el0     ; Read TLS base from system register
///     ldr x0, [x0, #0x1F8]    ; Load value from [TLS_base + 0x1F8]
///     ret                     ; Return the thread pointer
/// ```
///
/// **This hardcoded offset 0x1F8 is why `tls_ptr` must be the last field at offset 0x1F8.**
///
/// The value stored here typically points to `__tls_start` (the beginning of the TLS
/// data segment), adjusted for the Thread Control Block (TCB) alignment.
/// Pointer to the language-specific thread object for a thread.
///
/// Null while the thread has no object yet; the main thread is started this way and
/// has the pointer filled in once the thread registry reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadInfoPtr(*mut c_void);

impl ThreadInfoPtr {
    /// A thread with no language-specific object yet.
    pub const NULL: Self = Self(ptr::null_mut());

    /// Wraps a raw pointer without checking what it addresses.
    ///
    /// The caller must ensure `ptr` addresses the thread object of the thread whose
    /// `ThreadVars` it will be stored in, or is null. Nothing distinguishes it from
    /// the other two pointers in that footer once it is a bare `*mut c_void`.
    #[inline]
    pub const fn from_ptr_unchecked(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    /// Returns the raw pointer.
    #[inline]
    pub const fn to_raw(self) -> *mut c_void {
        self.0
    }
}

/// Pointer to a thread's newlib re-entrancy state (`struct _reent`).
///
/// Null in builds without the C runtime, where nothing consults it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ReentPtr(*mut c_void);

impl ReentPtr {
    /// A thread with no newlib re-entrancy state.
    pub const NULL: Self = Self(ptr::null_mut());

    /// Wraps a raw pointer without checking what it addresses.
    ///
    /// The caller must ensure `ptr` addresses this thread's `struct _reent`, or is
    /// null. newlib reads through it without validating it.
    #[inline]
    pub const fn from_ptr_unchecked(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    /// Returns the raw pointer.
    #[inline]
    pub const fn to_raw(self) -> *mut c_void {
        self.0
    }
}

/// The AArch64 thread pointer (TP) for a thread.
///
/// This is the value `__aarch64_read_tp()` returns, and the base every thread-local
/// access is resolved against. It is the thread's TLS segment walked back over the
/// Thread Control Block, not the base of the TLS block itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadPointer(*mut c_void);

impl ThreadPointer {
    /// Wraps a raw pointer without checking what it addresses.
    ///
    /// The caller must ensure `ptr` is this thread's thread-pointer value. Every
    /// thread-local access on this thread is resolved against it, so a pointer that
    /// addresses anything else corrupts unrelated thread-local state rather than
    /// failing at the point it was set.
    #[inline]
    pub const fn from_ptr_unchecked(ptr: *mut c_void) -> Self {
        Self(ptr)
    }

    /// Returns the raw pointer.
    #[inline]
    pub const fn to_raw(self) -> *mut c_void {
        self.0
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ThreadVars {
    /// Magic value used to check if the struct is initialized.
    pub magic: u32,

    /// Kernel handle identifying the thread.
    pub handle: ThreadHandle,

    /// Pointer to the current thread object (if any).
    ///
    /// Type-safe access: use [`get_thread_info_ptr::<T>()`].
    pub thread_info_ptr: ThreadInfoPtr,

    /// Pointer to the thread's newlib reentrancy state.
    pub reent: ReentPtr,

    /// Pointer to this thread's thread-local segment (TP).
    ///
    /// **MUST** be at offset 0x1F8 for AArch64 ABI compliance.
    pub tls_ptr: ThreadPointer,
}

// Ensure the layout stays consistent with Horizon expectations.
const_assert_eq!(size_of::<ThreadVars>(), THREAD_VARS_SIZE);
