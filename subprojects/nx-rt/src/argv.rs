//! Command-line argument parsing
//!
//! This module ports libnx's `argvSetup` functionality to Rust, parsing
//! command-line arguments into the standard argc/argv format.

use alloc::{boxed::Box, ffi::CString, string::String, vec::Vec};
use core::{
    ffi::{CStr, c_char},
    ptr, slice,
    sync::atomic::{AtomicPtr, Ordering},
};

use nx_svc::mem::query_memory;
use nx_sys_sync::Once;

use crate::env;

/// Initialization guard - ensures setup runs exactly once
static ARGV_INIT: Once = Once::new();

/// Parsed arguments (owns all argument memory)
static PARSED_ARGS: AtomicPtr<ParsedArgs> = AtomicPtr::new(ptr::null_mut());

/// Returns an iterator over command-line arguments (like `std::env::args`)
///
/// The first argument is typically the program name.
pub fn args() -> Args {
    Default::default()
}

/// Setup argv parsing
///
/// This function can be called multiple times safely - initialization
/// only happens once. Subsequent calls are no-ops.
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
pub unsafe fn setup() {
    ARGV_INIT.call_once(|| {
        // Get argument data based on mode
        let args_result = if env::is_nso() {
            // NSO mode: Check __argdata__ memory mapping
            unsafe { get_nso_args() }
        } else {
            // NRO mode: Use argv from homebrew loader
            unsafe { get_nro_args() }
        };

        let (args_str, _alloc_size) = match args_result {
            Some(result) => result,
            None => return, // No arguments available
        };

        // Parse the arguments string
        let mut parsed = parse_argv(args_str);

        // Strip nxlink suffix if present (XXXXXXXX_NXLINK_ pattern)
        if parsed.len() > 1
            && let Some(last) = parsed.last()
            && last.len() == 16
            && last.ends_with("_NXLINK_")
        {
            // Parse the first 8 hex characters as the IP address
            if let Ok(_addr) = u32::from_str_radix(&last[..8], 16) {
                #[cfg(feature = "ffi")]
                crate::ffi::set_nxlink_host(_addr);
                parsed.pop();
            }
        }

        if parsed.is_empty() {
            return;
        }

        // Convert to CStrings (owns null-terminated data)
        let cstrings: Vec<CString> = parsed
            .into_iter()
            .filter_map(|s| CString::new(s).ok())
            .collect();

        // Build argv pointer array
        let mut argv_ptrs: Vec<*mut c_char> = cstrings
            .iter()
            .map(|cs| cs.as_ptr() as *mut c_char)
            .collect();
        argv_ptrs.push(ptr::null_mut()); // Null terminator

        let argv_ptrs = argv_ptrs.into_boxed_slice();

        // Store everything in a single struct
        let parsed_args = Box::new(ParsedArgs {
            cstrings,
            argv_ptrs,
        });
        PARSED_ARGS.store(Box::into_raw(parsed_args), Ordering::Release);

        // Update FFI statics for C compatibility
        #[cfg(feature = "ffi")]
        // SAFETY: Called with valid argc/argv pointers
        unsafe {
            let parsed = &*PARSED_ARGS.load(Ordering::Acquire);
            let argc = parsed.cstrings.len() as i32;
            let argv_ptr = parsed.argv_ptrs.as_ptr() as *mut *mut c_char;
            crate::ffi::set_system_argv(argc, argv_ptr);
        }
    });
}

/// Parsed command-line arguments
struct ParsedArgs {
    /// CString storage - owns the null-terminated string data
    cstrings: Vec<CString>,
    /// Pre-built argv array (pointers into cstrings, plus null terminator)
    ///
    /// This field must exist to keep the Box allocation alive, even though
    /// it's never read. SYSTEM_ARGV points into this allocation.
    #[allow(dead_code)]
    argv_ptrs: Box<[*mut c_char]>,
}

// SAFETY: ParsedArgs is only written once during init, then read-only
unsafe impl Sync for ParsedArgs {}

/// Get arguments from NSO mode (__argdata__ linker symbol)
///
/// # Safety
///
/// Must be called during initialization
unsafe fn get_nso_args() -> Option<(&'static str, usize)> {
    unsafe extern "C" {
        /// Linker symbol for NSO argument data (page-aligned at end of executable)
        static __argdata__: u8;
    }

    let argdata_ptr = ptr::addr_of!(__argdata__);

    // Query memory to check if __argdata__ is mapped
    let (meminfo, _pageinfo) = query_memory(argdata_ptr as usize).ok()?;

    // Check if memory has Read+Write permission
    if !meminfo.perm.is_read_write() {
        return None;
    }

    // Read argdata header
    let arg32 = argdata_ptr as *const u32;
    let argdata_allocsize = unsafe { *arg32.add(0) } as usize;
    let argdata_strsize = unsafe { *arg32.add(1) } as usize;

    if argdata_allocsize == 0 || argdata_strsize == 0 {
        return None;
    }

    // Validate bounds
    let argdata_addr = argdata_ptr as usize;
    if argdata_addr < meminfo.addr {
        return None;
    }
    if (argdata_addr - meminfo.addr) + argdata_allocsize > meminfo.size {
        return None;
    }

    // Arguments string starts at offset 0x20
    let args_ptr = unsafe { argdata_ptr.add(0x20) };
    let args_slice = unsafe { slice::from_raw_parts(args_ptr, argdata_strsize) };

    // Convert to str
    let args_str = core::str::from_utf8(args_slice).ok()?;

    Some((args_str, argdata_allocsize))
}

/// Get arguments from NRO mode (homebrew loader argv)
///
/// # Safety
///
/// Must be called during initialization. The argv pointer returned by `env::argv()`
/// must point to a valid, null-terminated UTF-8 string that remains valid for the
/// lifetime of the program.
unsafe fn get_nro_args() -> Option<(&'static str, usize)> {
    let argv_ptr = env::argv()?;

    // Convert null-terminated C string to Rust str
    // SAFETY: argv_ptr comes from homebrew loader and must be a valid null-terminated string
    let argv_str = unsafe { CStr::from_ptr(argv_ptr) };
    if argv_str.is_empty() {
        return None;
    }

    let argv_str = argv_str.to_str().ok()?;

    // Use same alloc size as NSO mode (0x9000)
    Some((argv_str, 0x9000))
}

/// Parse argv string into Vec<String>
///
/// Handles quoted strings and whitespace separation
fn parse_argv(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_arg = String::new();
    let mut in_quote = false;
    let mut chars = args.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quote {
            if ch == '"' {
                // End quote
                in_quote = false;
                if !current_arg.is_empty() {
                    result.push(current_arg.clone());
                    current_arg.clear();
                }
            } else {
                // Inside quote, add character
                current_arg.push(ch);
            }
        } else {
            if ch == '"' {
                // Start quote
                in_quote = true;
            } else if ch.is_whitespace() {
                // Whitespace separator
                if !current_arg.is_empty() {
                    result.push(current_arg.clone());
                    current_arg.clear();
                }
            } else {
                // Regular character
                current_arg.push(ch);
            }
        }
    }

    // Push final argument if any
    if !current_arg.is_empty() {
        result.push(current_arg);
    }

    result
}

/// Iterator over command-line arguments (like std::env::Args)
#[derive(Default)]
pub struct Args {
    index: usize,
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let parsed_ptr = PARSED_ARGS.load(Ordering::Acquire);
        if parsed_ptr.is_null() {
            return None;
        }

        // SAFETY: PARSED_ARGS is set once during setup() and never freed
        let parsed = unsafe { &*parsed_ptr };

        if self.index < parsed.cstrings.len() {
            let cstr = &parsed.cstrings[self.index];
            self.index += 1;
            Some(cstr.to_string_lossy().into_owned())
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let parsed_ptr = PARSED_ARGS.load(Ordering::Acquire);
        if parsed_ptr.is_null() {
            return (0, Some(0));
        }
        let parsed = unsafe { &*parsed_ptr };
        let remaining = parsed.cstrings.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Args {}
