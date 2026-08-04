//! Command-line argument parsing (NSO)
//!
//! Ports `libnx`'s `argvSetup` for the NSO output kind: an NSO receives its
//! command line through the `__argdata__` linker symbol — a page-aligned
//! region the process manager maps at the end of the executable — rather than
//! through a homebrew-loader `argv` pointer.
//!
//! [`get_nso_args`] is the NSO-specific `__argdata__` reader; the kind-agnostic
//! scanner, the parsed-argument store, and the [`Args`] iterator are shared
//! from [`nx_rt_core::argv`].

use core::{
    ptr,
    slice,
};

pub use nx_rt_core::argv::{
    Args,
    args,
};
use nx_svc::mem::query_memory;

/// Byte offset of the argument string within the `__argdata__` region.
const ARGS_OFFSET: usize = 0x20;

/// Size of the `__argdata__` header — two `u32`s: the total allocation size
/// and the argument-string length.
const HEADER_SIZE: usize = 2 * size_of::<u32>();

/// Sets up argv parsing.
///
/// This function can be called multiple times safely — initialization only
/// happens once. Subsequent calls are no-ops.
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
pub unsafe fn setup() {
    // An NSO sources its arguments from the `__argdata__` region.
    // SAFETY: called during initialization, after the allocator is up.
    let args_str = match unsafe { get_nso_args() } {
        Some(args_str) => args_str,
        None => return, // No arguments available.
    };

    nx_rt_core::argv::setup_from(args_str);

    // Publish the C-style argc/argv globals for C consumers.
    #[cfg(feature = "ffi")]
    if let Some((argc, argv)) = nx_rt_core::argv::system_argv() {
        // SAFETY: argc/argv describe the leaked argument allocation owned by
        // `nx_rt_core::argv`, which lives for the rest of the program.
        unsafe { crate::ffi::set_system_argv(argc, argv) };
    }
}

/// Reads the NSO command-line arguments from the `__argdata__` region.
///
/// Returns the argument string truncated at its first NUL byte, or `None`
/// when no argument data is mapped or the `__argdata__` header is empty or
/// malformed.
///
/// # Safety
///
/// Must be called during initialization.
unsafe fn get_nso_args() -> Option<&'static str> {
    unsafe extern "C" {
        /// Linker symbol for the NSO argument data, page-aligned at the end
        /// of the executable.
        static __argdata__: u8;
    }

    let argdata_ptr = ptr::addr_of!(__argdata__);

    // The process manager maps `__argdata__` read-write only when it has
    // argument data to deliver; an unmapped or non-RW region means none.
    let (meminfo, _pageinfo) = query_memory(argdata_ptr as usize).ok()?;
    if !meminfo.perm.is_read_write() {
        return None;
    }

    // Locate `__argdata__` within its mapped region before dereferencing
    // anything: the header read below is sound only once the region is proven
    // to hold `HEADER_SIZE` readable bytes from `argdata_ptr`.
    let argdata_addr = argdata_ptr as usize;
    if argdata_addr < meminfo.addr {
        return None;
    }
    let region_offset = argdata_addr - meminfo.addr;

    // Hard shell — reject a region that cannot hold the fixed-size header at
    // `argdata_ptr` before reading it, rather than relying on the unchecked
    // page-alignment of the `__argdata__` linker symbol.
    if region_offset + HEADER_SIZE > meminfo.size {
        return None;
    }

    // The `__argdata__` header is two `u32`s: the total allocation size and
    // the argument-string length.
    let header = argdata_ptr as *const u32;
    // SAFETY: the check above proves the mapped region holds `HEADER_SIZE`
    // bytes from `argdata_ptr`, and the read-write check proves they are
    // readable.
    let argdata_allocsize = unsafe { *header.add(0) } as usize;
    let argdata_strsize = unsafe { *header.add(1) } as usize;
    if argdata_allocsize == 0 || argdata_strsize == 0 {
        return None;
    }

    // Reject a header whose declared allocation runs past the mapped region.
    if region_offset + argdata_allocsize > meminfo.size {
        return None;
    }

    // Reject a header whose declared string length overflows the allocation;
    // the string lives past the fixed-size header.
    if ARGS_OFFSET + argdata_strsize > argdata_allocsize {
        return None;
    }

    // SAFETY: the two checks above bound `ARGS_OFFSET + argdata_strsize`
    // within the mapped allocation, so the string bytes are all readable.
    let args_ptr = unsafe { argdata_ptr.add(ARGS_OFFSET) };
    let args_slice = unsafe { slice::from_raw_parts(args_ptr, argdata_strsize) };

    // `argdata_strsize` is the upper bound on the readable region, not the
    // exact content length — it may count the NUL terminator or be a buffer
    // capacity. `libnx`'s `argvSetup` walks the argument string with a
    // `while (*i)` loop, treating the NUL as authoritative; mirror that here.
    //
    // Locate the content terminator in the raw bytes *before* UTF-8
    // validation, then validate only the content prefix. Bytes past the NUL
    // are buffer capacity, not arguments, and may be uninitialized or garbage;
    // validating them would let a single non-UTF-8 trailing byte fail
    // `from_utf8` and silently drop *every* argument. The bounds checks above
    // keep `argdata_strsize` as the memory-safety bound; the NUL is the
    // content terminator.
    let end = args_slice
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(args_slice.len());
    core::str::from_utf8(&args_slice[..end]).ok()
}
