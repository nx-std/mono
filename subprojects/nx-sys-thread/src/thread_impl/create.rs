//! Thread creation implementation
//!
//! This module provides the core thread creation functionality, porting the
//! `threadCreate` function from libnx's `thread.c` to safe Rust.

use core::{
    ffi::c_void,
    ptr::{self, NonNull},
};

use nx_svc::{
    debug::{BreakReason, break_event},
    thread as svc,
};
use nx_sys_mem::stack::{self as stack_mem, StackMemory, Unmapped};

use super::info::{Thread, ThreadStackMem};
use crate::{registry, slots::Slots, tls_block, tls_region};

/// Thread entry point function type
pub type ThreadFunc = unsafe extern "C" fn(*mut c_void);

/// Page size constant
const PAGE_SIZE: usize = 0x1000;

/// Size of newlib's `struct _reent` structure
///
/// This must match `sizeof(struct _reent)` from newlib. The value is determined
/// by the newlib build configuration and target architecture.
///
/// For AArch64 with the current newlib configuration, this is 352 bytes.
/// This value should be verified whenever newlib is updated.
///
/// ## Verification
///
/// To verify this size, create a test program:
///
/// ```c
/// #include <stdio.h>
/// #include <sys/reent.h>
///
/// int main() {
///     printf("sizeof(struct _reent) = %zu bytes\n", sizeof(struct _reent));
///     printf("Aligned size (16-byte) = %zu bytes\n", (sizeof(struct _reent) + 0xF) & ~0xF);
///     return 0;
/// }
/// ```
///
/// Compile and run with:
/// ```bash
/// # Using host compiler (should work since both x86_64 and AArch64 are 64-bit)
/// gcc -I subprojects/sysroot/newlib/libc/include test_reent_size.c -o test_reent_size
/// ./test_reent_size
///
/// # Or using the target compiler for Nintendo Switch (AArch64)
/// /opt/devkitpro/devkitA64/bin/aarch64-none-elf-gcc -I subprojects/sysroot/newlib/libc/include test_reent_size.c -o test_reent_size_aarch64
/// # (Cannot run directly on x86_64 host, but the struct size should be the same)
/// ```
///
/// Expected output:
/// ```text
/// sizeof(struct _reent) = 352 bytes
/// Aligned size (16-byte) = 352 bytes
/// ```
const REENT_SIZE: usize = 352;

/// Creates a new thread
///
/// This is the Rust port of libnx's `threadCreate` function.
///
/// # Arguments
/// * `thread` - Mutable reference to Thread structure to initialize
/// * `entry` - Thread entry point function
/// * `arg` - Argument to pass to the entry point
/// * `stack_mem` - Optional stack memory (if None, will allocate)
/// * `stack_sz` - Stack size (must be page-aligned)
/// * `prio` - Thread priority (0x00~0x3F)
/// * `cpuid` - CPU core ID (0~3, or -2 for default)
///
/// Returns `Ok(())` on success, `Err(ThreadCreateError)` on failure.
pub fn create(
    thread: &mut Thread,
    entrypoint: ThreadFunc,
    arg: *mut c_void,
    stack_mem: Option<NonNull<c_void>>,
    stack_sz: usize,
    prio: i32,
    cpuid: i32,
) -> Result<(), ThreadCreateError> {
    // Calculate sizes for reent and TLS, aligned to 16 bytes
    // This matches the C implementation: (sizeof(struct _reent)+0xF) &~ 0xF
    let reent_sz = (REENT_SIZE + 0xF) & !0xF;
    let tls_sz = (tls_block::size() + 0xF) & !0xF;

    // Verify stack size alignment
    if stack_sz & (PAGE_SIZE - 1) != 0 {
        return Err(ThreadCreateError::InvalidStackSize);
    }

    let (stack_memory, effective_stack_sz) = match stack_mem {
        Some(provided_stack) => {
            // Using provided stack memory
            if provided_stack.as_ptr() as usize & (PAGE_SIZE - 1) != 0 {
                return Err(ThreadCreateError::InvalidStackAlignment);
            }

            // Ensure we don't go out of bounds
            let align_mask = tls_block::tdata::start_offset() - 1;
            let needed_sz = (tls_sz + reent_sz + align_mask) & !align_mask;
            if stack_sz <= needed_sz + size_of::<ThreadEntryArgs>() {
                return Err(ThreadCreateError::StackTooSmall);
            }

            let effective_sz = stack_sz - needed_sz;
            let total_size = stack_sz + tls_sz + reent_sz;

            // Create StackMemory from provided memory
            let stack_mem =
                unsafe { StackMemory::<Unmapped>::from_raw(provided_stack.as_ptr(), total_size)? };
            (stack_mem, effective_sz)
        }
        None => {
            // Allocate new memory for stack, TLS, and reent
            let total_size = stack_sz + tls_sz + reent_sz;
            let stack_mem = StackMemory::<Unmapped>::alloc_owned(total_size)?;
            (stack_mem, stack_sz)
        }
    };

    // Map the stack memory
    let mapped_stack = unsafe { stack_mem::map(stack_memory)? };
    let stack_mirror = mapped_stack.addr();

    // Calculate memory layout
    let stack_top = unsafe {
        stack_mirror
            .as_ptr()
            .add(effective_stack_sz)
            .sub(size_of::<ThreadEntryArgs>())
    };
    let tls_ptr = unsafe { stack_mirror.as_ptr().add(effective_stack_sz) };
    let reent_ptr = unsafe { tls_ptr.add(tls_sz) };

    // Update the thread structure with stack information
    thread.stack_mem = if mapped_stack.is_owned() {
        ThreadStackMem::new_owned(mapped_stack.backing_ptr(), stack_mirror, effective_stack_sz)
    } else {
        ThreadStackMem::new_provided(stack_mirror, effective_stack_sz)
    };
    thread.tls_slots = None; // Will be set in entry_wrap

    // Set up thread entry arguments and write them to the top of the stack
    let args_ptr = stack_top as *mut ThreadEntryArgs;
    unsafe {
        args_ptr.write(ThreadEntryArgs {
            thread: thread as *mut Thread,
            entrypoint,
            arg,
            reent: reent_ptr as *mut c_void,
            tls: tls_ptr as *mut c_void,
            _pad: ptr::null_mut(),
        })
    };

    // Create the kernel thread
    let handle = svc::create(
        thread_entrypoint_wrapper,
        args_ptr,
        stack_top as *mut c_void,
        prio,
        cpuid,
    )?;

    thread.handle = handle;

    // Set up child thread's reent struct, inheriting standard file handles
    unsafe {
        ptr::write_bytes(reent_ptr, 0, reent_sz);

        // TODO: Initialize the newlib reent structure and inherit file handles
    }

    // Set up child thread's TLS block
    // - Copy the `.tdata` section into the TLS block (if any)
    // - Initialize the `.tbss` section with zeros (if any)
    let tls_data_start = tls_ptr;
    let tls_data_size = tls_block::tdata::lma_size();
    unsafe { tls_block::tdata::copy_nonoverlapping(tls_data_start, tls_data_size) };

    let tls_bss_start = unsafe { tls_ptr.add(tls_data_size) };
    let tls_bss_size = tls_sz - tls_data_size;
    unsafe { tls_block::tbss::init_zeroed(tls_bss_start, tls_bss_size) };

    // Success! Transfer ownership of the mapped stack to the thread.
    // The stack memory will be cleaned up when the thread exits and
    // threadClose() is called.
    mapped_stack.leak();

    Ok(())
}

/// Thread entrypoint wrapper function
///
/// This is the actual thread entry point that:
/// - Initializes the TLS thread vars
/// - Initializes the thread info and registers with the global thread list
/// - Calls the user's thread function
/// - Exits the thread
fn thread_entrypoint_wrapper(args: *mut ThreadEntryArgs) {
    let args = unsafe { &*args };

    // Initialize the TLS thread vars
    unsafe {
        tls_region::init_thread_vars(
            (*args.thread).handle,
            args.thread as *mut c_void,
            args.reent,
            (args.tls as *mut u8).sub(tls_block::tdata::start_offset()) as *mut c_void,
        );
    }

    // Initialize thread info and register with global thread list
    let thread_ref = unsafe { &mut *args.thread };
    thread_ref.tls_slots = Some(unsafe {
        // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
        Slots::from_ptr(tls_region::slots_ptr())
    });

    // Register thread with global registry
    unsafe { registry::insert(thread_ref) };

    // Launch thread entrypoint
    unsafe { (args.entrypoint)(args.arg) };

    // Exit thread (this will clean up TLS and unregister)
    unsafe { exit(thread_ref) };
}

/// Thread creation arguments structure
///
/// Keep this struct's size 16-byte aligned, matching the C version
#[repr(C, align(16))]
struct ThreadEntryArgs {
    thread: *mut Thread,
    entrypoint: ThreadFunc,
    arg: *mut c_void,
    reent: *mut c_void,
    tls: *mut c_void,
    _pad: *mut c_void,
}

pub unsafe fn exit(thread: &mut Thread) -> ! {
    // Run the destructors for the slots that are currently in use.
    let Some(slots) = thread.tls_slots.as_mut() else {
        // TODO: Handle this case.
        break_event(BreakReason::Assert, 0, 0)
    };
    slots.run_destructors();

    // Remove thread from the global registry
    // SAFETY: `thread` was previously inserted during creation; removing it
    // now is valid and ensures the registry is kept consistent.
    unsafe { registry::remove(thread) };

    // Clear pointer fields to catch use-after-free bugs in debug builds.
    thread.tls_slots = None;

    // Terminate the thread via svcExitThread (never returns)
    svc::exit();
}

/// Thread creation errors
#[derive(Debug, thiserror::Error)]
pub enum ThreadCreateError {
    /// The provided stack size is not page-aligned.
    ///
    /// Stack sizes must be a multiple of the page size (4096 bytes / 0x1000).
    /// This check occurs early in the thread creation process before any
    /// memory allocation or mapping.
    #[error("Invalid stack size (must be page-aligned)")]
    InvalidStackSize,

    /// The provided stack memory pointer is not page-aligned.
    ///
    /// When providing external stack memory (non-None `stack_mem` parameter),
    /// the memory address must be aligned to page boundaries (4096 bytes).
    /// This ensures compatibility with the kernel's memory management.
    #[error("Invalid stack memory alignment")]
    InvalidStackAlignment,

    /// Generic out-of-memory error.
    ///
    /// This is a fallback error for memory allocation failures that don't
    /// fit into the more specific `StackAlloc` category.
    #[error("Out of memory")]
    OutOfMemory,

    /// The provided stack size is too small to hold required thread data.
    ///
    /// The stack must be large enough to accommodate:
    /// - The actual stack space for the thread
    /// - Thread-local storage (TLS) data
    /// - newlib reentrancy structure (`struct _reent`)
    /// - Thread entry arguments structure
    /// - Proper alignment padding
    ///
    /// This error occurs when using provided stack memory that is insufficient.
    #[error("Stack too small for required data")]
    StackTooSmall,

    /// Stack memory allocation failed.
    ///
    /// This error occurs when attempting to allocate new stack memory
    /// (when `stack_mem` parameter is None). The underlying error provides
    /// more details about the allocation failure.
    #[error("Stack allocation failed: {0}")]
    StackAlloc(#[from] stack_mem::AllocError),

    /// Stack memory mapping failed.
    ///
    /// This error occurs when the allocated stack memory cannot be mapped
    /// into the process address space. This could be due to:
    /// - Virtual address space exhaustion
    /// - Kernel memory mapping failures
    /// - Invalid memory regions
    #[error("Stack mapping failed: {0}")]
    StackMap(#[from] stack_mem::MapError),

    /// Kernel thread creation failed.
    ///
    /// This error occurs when the final step of creating the kernel thread
    /// object fails. This happens after all memory has been allocated and
    /// mapped successfully, but the kernel rejects the thread creation
    /// request. Possible causes include:
    /// - Invalid priority values
    /// - Invalid CPU ID values  
    /// - Kernel resource exhaustion
    /// - Permission issues
    #[error("Thread creation failed: {0}")]
    SvcCreateThread(#[from] svc::CreateThreadError),
}
