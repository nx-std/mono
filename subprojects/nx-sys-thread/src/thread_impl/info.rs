use core::ffi::c_void;

use nx_svc::thread::Handle;

/// Thread information structure
#[repr(C)]
pub struct Thread {
    /// The kernel thread handle
    pub handle: Handle,

    /// Whether the stack memory is owned by the thread.
    pub stack_mem_owned: bool,

    /// Alignment padding
    _align: [u8; 3],

    /// Stack memory information.
    pub stack_mem: ThreadStackMem,

    /// Pointer to the TLS slot array.
    pub tls_slot_array: *mut *mut c_void,

    /// Pointer to the next thread.
    pub next: *mut Thread,

    /// Pointer to the previous thread.
    pub prev_next: *mut *mut Thread,
}

/// Thread stack memory information
#[repr(C)]
pub struct ThreadStackMem {
    /// Pointer to stack memory
    pub mem: *mut c_void,

    /// Pointer to stack memory mirror
    pub mirror: *mut c_void,

    /// Stack memory size
    pub size: usize,
}

impl ThreadStackMem {
    /// Returns a pointer to the thread stack memory.
    pub fn memory_ptr(&self) -> *mut c_void {
        self.mem
    }

    /// Returns a pointer to the thread stack memory mirror.
    pub fn mirror_ptr(&self) -> *mut c_void {
        self.mirror
    }

    /// Returns the size of the thread stack memory.
    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert;

    use super::{Thread, ThreadStackMem};

    // Assert that the size and alignment of the `Thread` struct is correct
    const_assert!(size_of::<Thread>() == 0x38);
    const_assert!(align_of::<Thread>() == 0x8);

    // Assert that the size and alignment of the `ThreadStackMem` struct is correct
    const_assert!(size_of::<ThreadStackMem>() == 0x18);
    const_assert!(align_of::<ThreadStackMem>() == 0x8);
}
