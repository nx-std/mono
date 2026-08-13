//! Command-line argument parsing (NSO)
//!
//! Ports `libnx`'s `argvSetup` for the NSO output kind: an NSO receives its
//! command line through the `__argdata__` linker symbol (a page-aligned
//! region the process manager maps at the end of the executable) rather than
//! through a homebrew-loader `argv` pointer.
//!
//! [`get_nso_args`] is the NSO-specific `__argdata__` reader; the scanner and
//! the store that holds the result live in `nx-sys-args`.

use core::{
    ptr,
    slice,
};

use nx_svc::mem::query_memory;

/// Byte offset of the argument string within the `__argdata__` region.
const ARGS_OFFSET: usize = 0x20;

/// Size of the `__argdata__` header, two `u32`s: the total allocation size
/// and the argument-string length.
const HEADER_SIZE: usize = 2 * size_of::<u32>();

/// Sets up argv parsing.
///
/// This function can be called multiple times safely: initialization only
/// happens once. Subsequent calls are no-ops.
///
/// Nothing here allocates, so this needs no heap and may run before one exists.
pub fn setup() {
    // An NSO sources its arguments from the `__argdata__` region.
    // SAFETY: this runs during initialization, before any other thread exists,
    // and `__argdata__` is this process's to write.
    let args = match unsafe { get_nso_args() } {
        Some(args) => args,
        None => return, // No arguments available.
    };

    nx_sys_args::setup_from(args);
}

/// Reads the NSO command-line arguments from the `__argdata__` region.
///
/// Returns the argument string up to and including its first NUL byte, or
/// `None` when no argument data is mapped, the `__argdata__` header is empty or
/// malformed, or no NUL terminates the string. The bytes are handed on
/// unvalidated: an argument the encoding rules do not describe is still an
/// argument the process manager meant to pass.
///
/// The terminator is part of the slice because the scanner writes the last
/// argument's own terminator into it, and the slice stops there because bytes
/// past it are buffer capacity rather than argument text.
///
/// # Safety
///
/// Must be called during initialization, before any other thread exists, and
/// this process must be the only writer of `__argdata__`.
unsafe fn get_nso_args() -> Option<&'static mut [u8]> {
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

    // Hard shell: reject a region that cannot hold the fixed-size header at
    // `argdata_ptr` before reading it, rather than relying on the unchecked
    // page-alignment of the `__argdata__` linker symbol.
    if region_offset + HEADER_SIZE > meminfo.size {
        return None;
    }

    // The `__argdata__` header is two `u32`s: the total allocation size and
    // the argument-string length.
    let header = argdata_ptr as *const u32;
    // SAFETY: the check above proves the mapped region holds `HEADER_SIZE`
    // bytes from `argdata_ptr`, which covers this first `u32`, and the
    // read-write check proves they are readable.
    let argdata_allocsize = unsafe { *header.add(0) } as usize;
    // SAFETY: the check above proves the mapped region holds `HEADER_SIZE`
    // bytes from `argdata_ptr`, which is exactly the two `u32`s this is the
    // second of, and the read-write check proves they are readable.
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

    // SAFETY: the two checks above bound `ARGS_OFFSET + argdata_strsize` within
    // the mapped allocation, so this offset stays inside the same object as
    // `argdata_ptr`.
    let args_ptr = unsafe { argdata_ptr.add(ARGS_OFFSET) };

    // SAFETY: the same two checks prove `argdata_strsize` bytes from `args_ptr`
    // lie within the mapped allocation, and the read-write check above proves
    // they are both readable and writable. The process manager maps the region
    // before this process runs, so they are initialized, and the mapping lives
    // as long as the process does. This runs during initialization with no
    // other thread in existence, and `__argdata__` belongs to this process
    // alone, so the exclusive borrow aliases nothing.
    let args_slice = unsafe { slice::from_raw_parts_mut(args_ptr.cast_mut(), argdata_strsize) };

    // `argdata_strsize` is the upper bound on the readable region, not the
    // exact content length: it may count the NUL terminator or be a buffer
    // capacity. The startup ABI walks the argument string until its NUL,
    // treating that as authoritative; mirror it here. Bytes past the NUL are
    // buffer capacity, not arguments, and may be uninitialized or garbage.
    // The bounds checks above keep `argdata_strsize` as the memory-safety
    // bound; the NUL is the content terminator.
    //
    // A region with no NUL has no terminator to hand the scanner, and looking
    // past `argdata_strsize` for one would leave the bound established above.
    // Report no arguments instead.
    let end = args_slice.iter().position(|&byte| byte == 0)?;
    Some(&mut args_slice[..=end])
}
