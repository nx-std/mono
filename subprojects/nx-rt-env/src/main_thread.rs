//! Main thread TLS initialization.
//!
//! This module provides [`setup_main_thread_tls`], the Rust port of libnx's `newlibSetup()`
//! function. It initializes the main thread's `ThreadVars` structure and copies the `.tdata`
//! section into the main thread's TLS block.
//!
//! ## Initialization Order Requirement
//!
//! This function **must** be called before the allocator is initialized because the allocator
//! depends on `nx-sys-sync`, which in turn reads the thread handle from `ThreadVars.handle`
//! at TLS offset `0x1E4`. If `ThreadVars` is not properly initialized before allocator setup,
//! mutex operations will read garbage data from TLS.
//!
//! The correct initialization sequence (matching libnx C runtime) is:
//! 1. `envSetup()` - Parse homebrew environment
//! 2. `newlibSetup()` / `setup_main_thread_tls()` - Initialize ThreadVars ← **THIS FUNCTION**
//! 3. `virtmemSetup()` - Virtual memory setup
//! 4. `__libnx_initheap()` - Allocate heap ← Requires ThreadVars to be initialized
//!
//! ## Why Not in `nx-sys-thread`?
//!
//! The full `ThreadVars` type and related utilities live in `nx-sys-thread`, but that crate
//! depends on `nx-alloc` (creating a dependency chain: `nx-sys-thread` → `nx-alloc` →
//! `nx-sys-sync`). Since we need to initialize `ThreadVars` *before* the allocator, this
//! function must live in a crate that doesn't depend on the allocator.
//!
//! Following the same pattern as `nx-sys-sync::tls`, we use hardcoded offset constants
//! rather than importing types from `nx-sys-thread`. These offsets must be kept in sync
//! with the `ThreadVars` definition in `nx-sys-thread::tls_region`.
//!
//! ## TLS Memory Layout
//!
//! ```text
//! TLS base (TPIDRRO_EL0)
//! 0x000  ┌────────────────────────────┐
//!        │ IPC Message Buffer         │ 0x100 bytes
//! 0x100  ├────────────────────────────┤
//!        │ <Unknown>                  │
//! 0x108  ├────────────────────────────┤
//!        │ Dynamic TLS slots (27)     │ 27 × 8 = 0xD8 bytes
//! 0x1E0  ├────────────────────────────┤ ← THREAD_VARS_OFFSET
//!        │ ThreadVars (32 bytes)      │
//!        │   0x00: magic     (u32)    │ ← MAGIC_OFFSET
//!        │   0x04: handle    (u32)    │ ← HANDLE_OFFSET
//!        │   0x08: thread_ptr         │ ← THREAD_INFO_PTR_OFFSET
//!        │   0x10: reent              │ ← REENT_OFFSET
//!        │   0x18: tls_tp             │ ← TLS_PTR_OFFSET
//! 0x200  └────────────────────────────┘
//! ```
//!
//! ## References
//!
//! - C implementation: `libnx/nx/source/runtime/newlib.c::newlibSetup()`
//! - `nx-sys-thread::tls_region::ThreadVars` - Canonical ThreadVars definition
//! - `nx-sys-sync::tls` - Similar hardcoded offset approach

use core::{ffi::c_void, ptr};

use nx_cpu::control_regs;

// ThreadVars layout constants (must match nx-sys-thread::tls_region::ThreadVars)
/// Size of the Thread Local Storage (TLS) region
const TLS_REGION_SIZE: usize = 0x200;

/// Size of the ThreadVars structure
const THREAD_VARS_SIZE: usize = 0x20;

/// Offset of ThreadVars from TLS base (0x200 - 0x20 = 0x1E0)
const THREAD_VARS_OFFSET: usize = TLS_REGION_SIZE - THREAD_VARS_SIZE;

// ThreadVars field offsets (from ThreadVars base address)
/// Offset of the magic field (u32)
const MAGIC_OFFSET: usize = 0x00;

/// Offset of the handle field (u32)
const HANDLE_OFFSET: usize = 0x04;

/// Offset of the thread_info_ptr field (*mut c_void)
const THREAD_INFO_PTR_OFFSET: usize = 0x08;

/// Offset of the reent field (*mut c_void)
const REENT_OFFSET: usize = 0x10;

/// Offset of the tls_ptr field (*mut c_void)
const TLS_PTR_OFFSET: usize = 0x18;

/// Magic value used to verify ThreadVars is initialized ("!TV$")
const THREAD_VARS_MAGIC: u32 = 0x21545624;

// Linker symbols for TLS block management
unsafe extern "C" {
    /// Start address of the main thread's TLS block
    static __tls_start: u8;

    /// Start address of the .tdata section (initialized thread-local data)
    static __tdata_lma: u8;

    /// End address of the .tdata section
    static __tdata_lma_end: u8;

    /// Alignment requirement for TLS blocks
    static __tls_align: usize;
}

#[cfg(feature = "ffi")]
unsafe extern "C" {
    /// Newlib's global reentrancy structure pointer
    ///
    /// Only available when the `ffi` feature is enabled, as this symbol is provided by newlib
    /// which is linked when using the C FFI override functionality.
    static _impure_ptr: *mut c_void;
}

/// Initializes the main thread's ThreadVars structure and copies the .tdata section.
///
/// This function is the Rust port of libnx's `newlibSetup()`. It performs two critical
/// initialization tasks:
///
/// 1. **ThreadVars initialization**: Writes the ThreadVars structure at the end of the
///    main thread's TLS block (offset 0x1E0), setting the magic value, thread handle,
///    newlib reentrancy pointer, and thread pointer (TP).
///
/// 2. **TLS data copy**: Copies the `.tdata` section (initialized thread-local data)
///    from the ELF image to the main thread's TLS block at `__tls_start`.
///
/// ## Initialization Order
///
/// **CRITICAL**: This function MUST be called BEFORE the allocator is initialized,
/// because the allocator uses mutexes from `nx-sys-sync`, which read the thread handle
/// from `ThreadVars.handle` at TLS offset 0x1E4.
///
/// Correct call order (matching libnx C runtime):
/// ```text
/// 1. envSetup()               - Parse environment
/// 2. setup()                  ← THIS FUNCTION
/// 3. virtmemSetup()           - Virtual memory
/// 4. __libnx_initheap()       - Allocator (requires ThreadVars)
/// ```
///
/// ## What Gets Initialized
///
/// The ThreadVars structure fields are set as follows:
///
/// - `magic` (`0x00`): Set to `0x21545624` ("!TV$")
/// - `handle` (`0x04`): Set to the main thread's kernel handle from `envGetMainThreadHandle()`
/// - `thread_info_ptr` (`0x08`): Set to null (filled later by `__libnx_init_thread()`)
/// - `reent` (`0x10`): Set to `_impure_ptr` (newlib's global reentrancy state)
/// - `tls_ptr` (`0x18`): Set to `__tls_start - getTlsStartOffset()` (for `__aarch64_read_tp()`)
///
/// The `.tdata` section contains initialized values for `__thread` variables declared
/// in the program. These are copied byte-for-byte from the ELF image's `.tdata` section
/// (at `__tdata_lma`) to the runtime TLS block (at `__tls_start`).
///
/// ## Safety
///
/// This function is `unsafe` because:
///
/// - It must be called exactly **once** during process initialization
/// - It performs raw pointer writes to TLS memory
/// - It directly manipulates thread-local storage layout
/// - The calling thread must be the actual main thread
/// - TLS must be in a valid state (TPIDRRO_EL0 set by kernel)
///
/// Calling this function more than once or from the wrong context will corrupt
/// ThreadVars and cause undefined behavior.
///
/// ## C Equivalent
///
/// This is the Rust port of:
/// ```c
/// void newlibSetup(void) {
///     ThreadVars* tv = getThreadVars();
///     tv->magic = THREADVARS_MAGIC;
///     tv->thread_ptr = NULL;
///     tv->reent = _impure_ptr;
///     tv->tls_tp = __tls_start - getTlsStartOffset();
///     tv->handle = envGetMainThreadHandle();
///
///     u32 tls_size = __tdata_lma_end - __tdata_lma;
///     if (tls_size)
///         memcpy(__tls_start, __tdata_lma, tls_size);
/// }
/// ```
pub unsafe fn setup() {
    // SAFETY: TPIDRRO_EL0 is set by the kernel and guaranteed to point to a valid
    // 0x200-byte TLS block for the current thread
    let tls_base = unsafe { control_regs::tpidrro_el0() as *mut u8 };

    // ThreadVars structure is located at the end of the TLS block (offset 0x1E0)
    // SAFETY: tls_base is valid for 0x200 bytes, and THREAD_VARS_OFFSET (0x1E0)
    // is within bounds
    let thread_vars = unsafe { tls_base.add(THREAD_VARS_OFFSET) };

    // Calculate the thread pointer (TP) value for __aarch64_read_tp()
    // This matches the C code: __tls_start - getTlsStartOffset()
    let tls_start = &raw const __tls_start as usize;
    let tls_start_offset = {
        // Thread Control Block (TCB) is 2 pointer-sized slots (16 bytes on AArch64)
        let tcb_sz = 2 * size_of::<*mut c_void>();

        // SAFETY: __tls_align is a linker-provided symbol guaranteed to be valid
        let align = unsafe { __tls_align };

        // Take the maximum of TCB size and required alignment
        if align > tcb_sz { align } else { tcb_sz }
    };
    let tls_tp = (tls_start - tls_start_offset) as *mut c_void;

    // Initialize ThreadVars fields by writing to each offset

    // Field: magic (u32 at offset 0x00)
    // SAFETY: thread_vars + MAGIC_OFFSET is within the 0x20-byte ThreadVars structure
    unsafe {
        ptr::write(thread_vars.add(MAGIC_OFFSET) as *mut u32, THREAD_VARS_MAGIC);
    }

    // Field: handle (u32 at offset 0x04)
    // SAFETY: thread_vars + HANDLE_OFFSET is within the ThreadVars structure
    unsafe {
        ptr::write(
            thread_vars.add(HANDLE_OFFSET) as *mut u32,
            crate::main_thread_handle().to_raw(),
        );
    }

    // Field: thread_info_ptr (*mut c_void at offset 0x08)
    // Set to null - will be filled later by __libnx_init_thread()
    // SAFETY: thread_vars + THREAD_INFO_PTR_OFFSET is within bounds
    unsafe {
        ptr::write(
            thread_vars.add(THREAD_INFO_PTR_OFFSET) as *mut *mut c_void,
            ptr::null_mut(),
        );
    }

    // Field: reent (*mut c_void at offset 0x10)
    // Point to newlib's global reentrancy structure if available (with ffi feature),
    // otherwise set to NULL
    // SAFETY: thread_vars + REENT_OFFSET is within bounds
    #[cfg(feature = "ffi")]
    unsafe {
        // SAFETY: _impure_ptr is a valid global provided by newlib when ffi is enabled
        ptr::write(
            thread_vars.add(REENT_OFFSET) as *mut *mut c_void,
            _impure_ptr,
        );
    }
    #[cfg(not(feature = "ffi"))]
    unsafe {
        ptr::write(
            thread_vars.add(REENT_OFFSET) as *mut *mut c_void,
            ptr::null_mut(),
        );
    }

    // Field: tls_ptr (*mut c_void at offset 0x18)
    // This is the value returned by __aarch64_read_tp() for thread-local variable access
    // SAFETY: thread_vars + TLS_PTR_OFFSET is within bounds
    unsafe {
        ptr::write(thread_vars.add(TLS_PTR_OFFSET) as *mut *mut c_void, tls_tp);
    }

    // Copy .tdata section (initialized thread-local data) to the TLS block
    // SAFETY: The linker guarantees __tdata_lma and __tdata_lma_end are valid pointers
    // that delimit the .tdata section in the ELF image
    let tdata_size = (&raw const __tdata_lma_end as usize) - (&raw const __tdata_lma as usize);

    if tdata_size > 0 {
        // SAFETY:
        // - __tdata_lma points to the .tdata section in the ELF image (read-only)
        // - __tls_start points to the main thread's TLS block (writable, accessed via mutable ptr)
        // - tdata_size bytes are guaranteed valid at both locations
        // - The regions do not overlap (source is in .rodata-like section, dest is in TLS)
        unsafe {
            ptr::copy_nonoverlapping(
                &raw const __tdata_lma,
                &raw const __tls_start as *mut u8,
                tdata_size,
            );
        }
    }
}
