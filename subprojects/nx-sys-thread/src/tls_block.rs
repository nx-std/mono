//! Thread-Local Storage (TLS) Block
//!
//! An individual thread's copy of a TLS segment. There is one TLS block per TLS
//! segment per thread.
//!
//! ## Terminology
//!
//! - **TLS Segment**: This is the image of data in each module and specified by the
//!   ELF TLS ABI. Not every program has a TLS segment and thus not every program
//!   has a TLS block. Each program has at most one TLS segment and correspondingly
//!   at most one TLS block.
//!
//! - **TLS Block**: This is the runtime copy of a TLS segment. There is one TLS
//!   block per TLS segment per thread.
//!
//! # References
//! - [ARM: Thread-Local Storage](https://developer.arm.com/documentation/100748/0624/Thread-Local-Storage)
//! - [ELF Handling For Thread-Local Storage](https://www.akkadia.org/drepper/tls.pdf)
//! - [Switchbrew Wiki: Thread Local Region](https://switchbrew.org/wiki/Thread_Local_Region)
//! - [Fuchsia: Thread Local Storage (TLS)](https://fuchsia.dev/fuchsia-src/development/kernel/threads/tls)
//! - [Android: ELF Thread Local Storage (TLS)](https://android.googlesource.com/platform/bionic/+/HEAD/docs/elf-tls.md)
//! - [MaskRay: All about Thread Local Storage](https://maskray.me/blog/2021-02-14-all-about-thread-local-storage)

// SAFETY: The symbols are defined in the linker script and are guaranteed to
// be valid.
unsafe extern "C" {
    /// Start address of the memory reserved for the main thread's Thread-Local
    /// Storage (TLS) block.
    ///
    /// The linker emits this via:
    /// ```text
    /// PROVIDE_HIDDEN( __tls_start = ADDR(.main.tls) );
    /// ```
    ///
    /// Together with `__tls_end` this symbol delimits the TLS area that holds
    /// `.tdata` followed by `.tbss` for the initial thread.
    static __tls_start: u8;

    /// End address (one-past-the-last byte) of the main thread's TLS block.
    ///
    /// Linker source:
    /// ```text
    /// PROVIDE_HIDDEN( __tls_end = ADDR(.main.tls) + SIZEOF(.main.tls) );
    /// ```
    static __tls_end: u8;

    /// Alignment requirement (in bytes) for a TLS block.
    ///
    /// The value is emitted in the `.tls.align` section using:
    /// ```text
    /// QUAD( MAX( ALIGNOF(.tdata), ALIGNOF(.tbss) ) )
    /// ```
    /// And then exposed via:
    /// ```text
    /// PROVIDE_HIDDEN( __tls_align = ADDR(.tls.align) );
    /// ```
    ///
    /// Runtime code that allocates TLS for new threads should honour this
    /// alignment.
    static __tls_align: usize;
}

/// Address of the start of the TLS segment.
///
/// The start address is the address of the first byte of the TLS segment.
///
/// # Safety
///
/// The caller must ensure that the linker script defines the `__tls_start` symbol
/// and that it points to a valid memory location.
#[inline(always)]
pub unsafe fn start_addr() -> usize {
    &raw const __tls_start as usize
}

/// Address of the end of the TLS segment.
///
/// The end address is the address of the last byte of the TLS segment.
///
/// # Safety
///
/// The caller must ensure that the linker script defines the `__tls_end` symbol
/// and that it points to a valid memory location.
#[inline(always)]
pub unsafe fn end_addr() -> usize {
    &raw const __tls_end as usize
}

/// Size of the TLS segment.
///
/// The size is the difference between the end and the beginning of the TLS segment.
#[inline(always)]
pub fn size() -> usize {
    // SAFETY: The symbols are defined in the linker script and are guaranteed to
    // be valid.
    unsafe { end_addr() - start_addr() }
}

/// Returns the alignment of the TLS segment.
///
/// The alignment is the value of the `__tls_align` symbol.
#[inline(always)]
pub fn align() -> usize {
    // SAFETY: The symbols are defined in the linker script and are guaranteed to
    // be valid.
    unsafe { __tls_align }
}

pub mod tdata {
    use core::{ffi::c_void, ptr};

    use super::__tls_align;

    // SAFETY: The symbols are defined in the linker script and are guaranteed to
    // be valid.
    unsafe extern "C" {
        /// Start (Load Memory Address) of the `.tdata` section as provided by the
        /// linker script.
        ///
        /// In `switch.ld` you will find the following line:
        /// ```text
        /// PROVIDE_HIDDEN( __tdata_lma = ADDR(.tdata) );
        /// ```
        ///
        /// At runtime this symbol points to the first byte of the initialised
        /// thread-local data that needs to be copied into each thread's TLS area.
        static __tdata_lma: u8;

        /// End (one-past-the-last byte) address of the `.tdata` section.
        ///
        /// Defined by the linker via:
        /// ```text
        /// PROVIDE_HIDDEN( __tdata_lma_end = ADDR(.tdata) + SIZEOF(.tdata) );
        /// ```
        ///
        /// The difference between the end and the beginning of the `.tdata` section
        /// yields the size of the initialised TLS data block.
        static __tdata_lma_end: u8;
    }

    /// Address of the start of the `.tdata` section.
    ///
    /// Returns the address of the first byte of the `.tdata` section in the ELF file.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the linker script defines the `__tdata_lma` symbol
    /// and that it points to a valid memory location.
    #[inline(always)]
    pub unsafe fn lma_start_addr() -> usize {
        &raw const __tdata_lma as usize
    }

    /// Address of the end of the `.tdata` section.
    ///
    /// Returns the address of the last byte of the `.tdata` section in the ELF file.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the linker script defines the `__tdata_lma_end` symbol
    /// and that it points to a valid memory location.
    #[inline(always)]
    pub unsafe fn lma_end_addr() -> usize {
        &raw const __tdata_lma_end as usize
    }

    /// Size of the `.tdata` section.
    ///
    /// In the ELF file, the data corresponding to the `.tdata` section is located
    /// at `__tdata_lma` and ends at `__tdata_lma_end`.
    ///
    /// It is used to calculate the size of the `.tdata` section that needs to be
    /// copied into every new thread's TLS area.
    #[inline(always)]
    pub fn lma_size() -> usize {
        // SAFETY: The symbols are defined in the linker script and are guaranteed
        // to be valid.
        unsafe { lma_end_addr() - lma_start_addr() }
    }

    /// Returns the offset (in bytes) from the beginning of the TLS block to the
    /// start of the *static* thread-local data (.tdata / .tbss).
    ///
    /// The TLS area begins with the *Thread Control Block* (TCB), which on Horizon is defined as two
    /// pointer-sized fields (16 bytes on AArch64).  
    ///
    /// The actual thread–local data must be placed after this TCB, but it might also require a stricter
    /// alignment as communicated by the linker via the [`__tls_align`] symbol. At runtime we therefore
    /// take the maximum of the natural TCB size and the linker-supplied alignment value.
    #[inline]
    pub fn start_offset() -> usize {
        // The Horizon TCB consists of two pointer-sized slots.
        let tcb_sz = 2 * size_of::<*mut c_void>();

        // SAFETY: `__tls_align` is set up by the linker and guaranteed to point to a valid `usize`
        // that contains the required alignment of the TLS block.
        let align = unsafe { __tls_align };

        if align > tcb_sz { align } else { tcb_sz }
    }

    /// Copies the `.tdata` section into the given pointer.
    ///
    /// The `.tdata` section is located at `__tdata_lma` and ends at `__tdata_lma_end`.
    ///
    /// The size of the `.tdata` section is the difference between the end,
    /// `__tdata_lma_end`, and the beginning, `__tdata_lma`, of the `.tdata` section.
    ///
    /// # Safety
    /// - The caller must ensure that the pointer is valid, aligned and points
    ///   to the start of the `.tdata` section in the thread's TLS block.
    /// - The caller must ensure that the `.tdata` section is not modified
    ///   concurrently.
    pub unsafe fn copy_nonoverlapping<T>(dst: *mut T, size: usize) {
        if size == 0 {
            return; // No data to copy
        }

        // SAFETY: The symbols are defined in the linker script and are guaranteed
        // to be valid.
        let src = unsafe { lma_start_addr() as *const _ };

        // SAFETY: The caller must ensure that the pointer is valid, aligned and
        // points to the start of the `.tdata` section in the thread's TLS block.
        unsafe { ptr::copy_nonoverlapping(src, dst, size) }
    }
}

pub mod tbss {
    use core::ptr;

    /// Initializes the `.tbss` section of the TLS block with zeros.
    ///
    /// The `.tbss` section must be located at the end of the `.tdata` section
    /// in the thread's TLS block.
    ///
    /// The function writes zeros to the `.tbss` section using `ptr::write_bytes`.
    ///
    /// # Safety
    /// - The caller must ensure that the pointer is valid, aligned and points
    ///   to the start of the `.tbss` section in the thread's TLS block.
    /// - The caller must ensure that the `.tbss` section is not modified
    ///   concurrently.
    pub unsafe fn init_zeroed<T>(dst: *mut T, size: usize) {
        if size == 0 {
            return; // No data to initialise
        }

        // SAFETY: The caller must ensure that the pointer is valid, aligned and
        // points to the start of the `.tbss` section in the thread's TLS block.
        unsafe { ptr::write_bytes(dst, 0, size) }
    }
}
