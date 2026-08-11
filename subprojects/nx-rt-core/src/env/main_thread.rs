//! Main-thread TLS bring-up.
//!
//! [`setup`] fills in the thread bookkeeping that the C runtime and the
//! synchronization primitives read out of thread-local storage, and copies the
//! initialized thread-local data from the image into the main thread's TLS
//! block.
//!
//! # Where it sits in startup
//!
//! It runs before the heap. Allocating takes a mutex, and a mutex reads the
//! current thread's handle out of the bookkeeping this writes; the other way
//! round, the first allocation arbitrates on whatever the TLS block happened
//! to contain. It runs after the environment is parsed, because the handle it
//! records comes from there.
//!
//! The layout of the bookkeeping and the offsets it lives at belong to
//! [`nx_sys_thread_tls`].

use core::{
    ffi::c_void,
    ptr,
};

use nx_sys_thread_tls::{
    ReentPtr,
    ThreadInfoPtr,
    ThreadPointer,
};

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

/// Fills in the main thread's bookkeeping and its thread-local data.
///
/// Two steps. First the thread bookkeeping is written at the end of the main
/// thread's TLS block: the magic the C runtime checks for, the thread's kernel
/// handle, the reentrancy state newlib keeps per thread, and the thread
/// pointer that thread-local reads are resolved against. Then the initialized
/// thread-local data is copied out of the image into the block, which is what
/// gives `#[thread_local]` variables their declared values.
///
/// The thread-info slot is deliberately left null: it is filled later, by the
/// step that registers this thread with the thread bookkeeping.
///
/// # Safety
///
/// Must be called exactly once, on the main thread, before anything allocates
/// or takes a lock, and with thread-local storage as the kernel left it.
/// Calling it twice, or from another thread, overwrites bookkeeping that is
/// already in use.
pub unsafe fn setup() {
    // The thread pointer is the TLS block start walked back over the control
    // block that sits in front of it, which is what a thread-local read adds
    // its offset to.
    let tls_start = &raw const __tls_start as usize;
    let tls_start_offset = {
        // Thread Control Block (TCB) is 2 pointer-sized slots (16 bytes on AArch64)
        let tcb_sz = 2 * size_of::<*mut c_void>();

        // SAFETY: __tls_align is a linker-provided symbol guaranteed to be valid
        let align = unsafe { __tls_align };

        // Take the maximum of TCB size and required alignment
        if align > tcb_sz { align } else { tcb_sz }
    };
    // SAFETY: `tls_start` walked back over the TCB span is the main thread's
    // thread-pointer value.
    let tls_tp = ThreadPointer::from_ptr_unchecked((tls_start - tls_start_offset) as *mut c_void);

    // Get the reent pointer (newlib reentrancy state)
    // SAFETY: `_impure_ptr` is newlib's own re-entrancy state for this thread.
    #[cfg(feature = "ffi")]
    let reent = ReentPtr::from_ptr_unchecked(unsafe { _impure_ptr });
    // Without the C runtime there is no `_reent` for the footer to point at.
    #[cfg(not(feature = "ffi"))]
    let reent = ReentPtr::NULL;

    // SAFETY: called exactly once during main-thread bring-up, with the TLS
    // block the kernel pointed the thread register at.
    unsafe {
        nx_sys_thread_tls::init_thread_vars(
            super::main_thread_handle(),
            // Filled later by the thread registry.
            ThreadInfoPtr::NULL,
            reent,
            tls_tp,
        );
    }

    // The initialized thread-local data, which the linker delimits.
    let tdata_size = (&raw const __tdata_lma_end as usize) - (&raw const __tdata_lma as usize);

    if tdata_size > 0 {
        // SAFETY: the linker places both ends, so the span is live at the
        // source and fits at the destination; the image the data is read from
        // and the TLS block it lands in are separate regions, so they cannot
        // overlap.
        unsafe {
            ptr::copy_nonoverlapping(
                &raw const __tdata_lma,
                &raw const __tls_start as *mut u8,
                tdata_size,
            );
        }
    }
}
