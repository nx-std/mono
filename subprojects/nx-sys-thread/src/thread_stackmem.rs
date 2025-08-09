//! Thread stack memory management.
//!
//! This module provides structures and utilities for managing thread stack memory
//! in the Horizon OS environment. It handles both owned and provided stack memory
//! configurations, supporting the Nintendo Switch's unique memory model with
//! stack mirroring.
//!
//! ## Stack Memory Models
//!
//! The module supports two stack memory ownership models:
//! - **Owned**: Thread owns and manages its stack memory allocation
//! - **Provided**: Stack memory is provided externally (e.g., main thread)
//!
//! ## Memory Mirroring
//!
//! Horizon OS uses stack mirroring for security and debugging purposes:
//! - The actual stack memory (`mem`) is the original allocation
//! - The mirror (`mirror`) is a mapped view used for execution
//! - This separation provides guard pages and overflow detection

use core::{ffi::c_void, ptr::NonNull};

/// Thread stack memory information.
///
/// This enum represents the two possible stack memory configurations for
/// threads in the Horizon OS environment. It tracks ownership, pointers,
/// and size information necessary for proper stack management.
///
/// ## Memory Layout
///
/// Stack memory in Horizon OS follows this layout:
/// ```text
/// [Guard Page] [Stack Space] [TLS] [Reent] [Guard Page]
///                    ^
///                    |
///              Stack grows down
/// ```
///
/// ## Ownership Models
///
/// - **Owned**: Thread allocated its own stack (typical for created threads)
/// - **Provided**: Stack was provided externally (typical for main thread)
#[derive(Debug, Clone)]
pub enum ThreadStackMem {
    /// The stack memory is owned by the thread.
    ///
    /// This variant is used when a thread allocates its own stack memory,
    /// typically during thread creation via [`crate::thread_create::create`].
    /// The thread is responsible for deallocating this memory on exit.
    Owned {
        /// Pointer to the original stack memory allocation.
        ///
        /// This points to the base of the allocated memory region that
        /// backs the stack. It's used for deallocation when the thread
        /// exits.
        mem: NonNull<c_void>,

        /// Pointer to stack memory mirror used for execution.
        ///
        /// This is the mapped view of the stack that the thread actually
        /// uses during execution. It may have different permissions or
        /// guard pages compared to the original allocation.
        mirror: NonNull<c_void>,

        /// Stack memory size in bytes.
        ///
        /// This is the usable stack size, not including guard pages or
        /// other metadata. It must be page-aligned (4096 bytes).
        size: usize,
    },

    /// The stack memory is not owned by the thread.
    ///
    /// This variant is used when stack memory is provided externally,
    /// such as for the main thread where the kernel provides the stack.
    /// The thread should not attempt to deallocate this memory.
    Provided {
        /// Pointer to the provided stack memory mirror.
        ///
        /// For provided stacks, we only track the mirror pointer since
        /// we don't own the underlying allocation and won't deallocate it.
        mirror: NonNull<c_void>,

        /// Stack memory size in bytes.
        ///
        /// The size of the provided stack space. This is informational
        /// since the thread doesn't manage this memory.
        size: usize,
    },
}

impl ThreadStackMem {
    /// Creates a new owned thread stack memory.
    ///
    /// Use this when creating a thread that owns its stack allocation.
    ///
    /// # Arguments
    ///
    /// * `mem` - Pointer to the original allocated memory
    /// * `mirror` - Pointer to the mapped/mirrored memory for execution
    /// * `size` - Size of the stack in bytes (must be page-aligned)
    pub fn new_owned(mem: NonNull<c_void>, mirror: NonNull<c_void>, size: usize) -> Self {
        Self::Owned { mem, mirror, size }
    }

    /// Creates a new thread stack memory from a provided stack.
    ///
    /// Use this when the stack memory is provided externally and not
    /// owned by the thread (e.g., main thread, or threads with custom stacks).
    ///
    /// # Arguments
    ///
    /// * `mirror` - Pointer to the provided stack memory
    /// * `size` - Size of the stack in bytes
    pub fn new_provided(mirror: NonNull<c_void>, size: usize) -> Self {
        Self::Provided { mirror, size }
    }

    /// Returns true if the stack memory is owned by the thread.
    ///
    /// This is important for cleanup - only owned stacks should be
    /// deallocated when the thread exits.
    pub fn is_owned(&self) -> bool {
        matches!(self, ThreadStackMem::Owned { .. })
    }

    /// Returns a pointer to the original thread stack memory allocation.
    ///
    /// Returns `Some(ptr)` for owned stacks, `None` for provided stacks.
    /// This pointer is used for deallocation purposes.
    ///
    /// # Safety Notes
    ///
    /// The returned pointer should only be used for deallocation when
    /// the thread is exiting. Using it while the thread is running may
    /// cause undefined behavior.
    pub fn memory_ptr(&self) -> Option<NonNull<c_void>> {
        match self {
            ThreadStackMem::Owned { mem, .. } => Some(*mem),
            ThreadStackMem::Provided { .. } => None,
        }
    }

    /// Returns a pointer to the thread stack memory mirror.
    ///
    /// This is the actual memory region used for the thread's stack
    /// during execution. It's available for both owned and provided stacks.
    ///
    /// # Safety Notes
    ///
    /// This pointer points to active stack memory. Modifying it while
    /// the thread is running will cause undefined behavior. It should
    /// only be used for:
    /// - Initial stack setup before thread start
    /// - Debugging/inspection while thread is paused
    /// - Cleanup after thread exit
    pub fn mirror_ptr(&self) -> NonNull<c_void> {
        match self {
            ThreadStackMem::Owned { mirror, .. } => *mirror,
            ThreadStackMem::Provided { mirror, .. } => *mirror,
        }
    }

    /// Returns the size of the thread stack memory in bytes.
    ///
    /// This is the usable stack size, not including guard pages or
    /// other metadata. The size is the same whether the stack is
    /// owned or provided.
    ///
    /// # Stack Size Guidelines
    ///
    /// Common stack sizes for Nintendo Switch:
    /// - **Minimal**: 0x1000 (4KB) - Very simple threads
    /// - **Small**: 0x4000 (16KB) - Light processing
    /// - **Default**: 0x10000 (64KB) - General purpose
    /// - **Large**: 0x40000 (256KB) - Complex processing
    /// - **Huge**: 0x100000 (1MB) - Heavy recursion/allocation
    pub fn size(&self) -> usize {
        match self {
            ThreadStackMem::Owned { size, .. } => *size,
            ThreadStackMem::Provided { size, .. } => *size,
        }
    }

    /// Checks if the stack has sufficient size for safe operation.
    ///
    /// This is a convenience method that checks against a minimum
    /// recommended size for typical thread operations.
    ///
    /// # Arguments
    ///
    /// * `min_size` - Minimum required stack size in bytes
    pub fn has_minimum_size(&self, min_size: usize) -> bool {
        self.size() >= min_size
    }

    /// Returns information about the stack memory as a tuple.
    ///
    /// Returns `(is_owned, memory_ptr, mirror_ptr, size)` for convenient
    /// access to all stack properties at once.
    pub fn as_tuple(&self) -> (bool, Option<NonNull<c_void>>, NonNull<c_void>, usize) {
        (
            self.is_owned(),
            self.memory_ptr(),
            self.mirror_ptr(),
            self.size(),
        )
    }
}

/// Stack memory utilities and helpers.
pub mod utils {
    use super::*;

    /// Calculates the stack pointer for initial thread entry.
    ///
    /// Given a stack memory configuration, returns the initial stack
    /// pointer that should be used when creating a thread. This accounts
    /// for stack growth direction (downward on ARM).
    ///
    /// # Arguments
    ///
    /// * `stack_mem` - The thread stack memory configuration
    /// * `reserved` - Bytes to reserve at top of stack (for arguments, etc.)
    pub fn calculate_initial_sp(stack_mem: &ThreadStackMem, reserved: usize) -> *mut u8 {
        let mirror = stack_mem.mirror_ptr();
        let size = stack_mem.size();

        unsafe { (mirror.as_ptr() as *mut u8).add(size).sub(reserved) }
    }

    /// Validates stack alignment requirements.
    ///
    /// Checks that the stack memory pointers and size meet the alignment
    /// requirements for the Horizon OS (must be page-aligned).
    ///
    /// # Arguments
    ///
    /// * `stack_mem` - The stack memory to validate
    ///
    /// # Returns
    ///
    /// `true` if alignment requirements are met, `false` otherwise.
    pub fn is_properly_aligned(stack_mem: &ThreadStackMem) -> bool {
        const PAGE_SIZE: usize = 0x1000;

        let mirror_aligned = stack_mem.mirror_ptr().as_ptr() as usize & (PAGE_SIZE - 1) == 0;
        let size_aligned = stack_mem.size() & (PAGE_SIZE - 1) == 0;

        if let Some(mem) = stack_mem.memory_ptr() {
            let mem_aligned = mem.as_ptr() as usize & (PAGE_SIZE - 1) == 0;
            mirror_aligned && size_aligned && mem_aligned
        } else {
            mirror_aligned && size_aligned
        }
    }
}
