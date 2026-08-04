//! # Command-line argument scanner
//!
//! Kind-agnostic machinery shared by every Switch executable kind: the
//! argument-string scanner ([`parse_argv`]), the parsed-argument store
//! ([`ParsedArgs`]), and the [`Args`] iterator that mirrors `std::env::args`.
//!
//! Each output kind reads its command line from a different source — a
//! homebrew NRO from the loader-supplied `argv` pointer, an NSO from the
//! page-aligned `__argdata__` region — and hands the already-read argument
//! string to [`setup_from`]. The kind-specific reader, and the NRO-only
//! nxlink-suffix handling, stay with each entry crate; only the scanner,
//! storage, and iterator are shared here.

use alloc::{
    boxed::Box,
    ffi::CString,
    string::String,
    vec::Vec,
};
#[cfg(feature = "ffi")]
use core::ffi::c_char;
use core::{
    ptr,
    sync::atomic::{
        AtomicPtr,
        Ordering,
    },
};

use nx_sys_sync::Once;

/// Initialization guard — ensures [`setup_from`] installs arguments once.
static ARGV_INIT: Once = Once::new();

/// Parsed arguments (owns all argument memory).
static PARSED_ARGS: AtomicPtr<ParsedArgs> = AtomicPtr::new(ptr::null_mut());

/// Returns an iterator over command-line arguments (like `std::env::args`).
///
/// The first argument is typically the program name.
pub fn args() -> Args {
    Default::default()
}

/// Installs the process command line from an already-read argument string.
///
/// `source` is the raw, whitespace- and quote-delimited argument string the
/// entry crate's kind-specific reader produced. It is scanned into individual
/// arguments and published for [`args`] and the C-FFI [`system_argv`] surface.
///
/// Runs exactly once; subsequent calls are no-ops, which is what keeps the
/// unsynchronized [`Args`] reads sound.
pub fn setup_from(source: &str) {
    ARGV_INIT.call_once(|| {
        let parsed = parse_argv(source);
        if parsed.is_empty() {
            return;
        }

        // Convert to CStrings (owns the null-terminated argument data).
        let cstrings: Vec<CString> = parsed
            .into_iter()
            .filter_map(|s| CString::new(s).ok())
            .collect();

        // Build the C-style argv pointer array — pointers into `cstrings`
        // followed by a NULL terminator — backing the `__system_argv` export.
        #[cfg(feature = "ffi")]
        let argv_ptrs = {
            let mut argv_ptrs: Vec<*mut c_char> = cstrings
                .iter()
                .map(|cs| cs.as_ptr() as *mut c_char)
                .collect();
            argv_ptrs.push(ptr::null_mut()); // NULL terminator.
            argv_ptrs.into_boxed_slice()
        };

        let parsed_args = Box::new(ParsedArgs {
            cstrings,
            #[cfg(feature = "ffi")]
            argv_ptrs,
        });
        PARSED_ARGS.store(Box::into_raw(parsed_args), Ordering::Release);
    });
}

/// C-style `(argc, argv)` for the installed command line, or `None` when no
/// arguments were installed.
///
/// The returned `argv` points into the leaked [`ParsedArgs`] allocation — a
/// NULL-terminated pointer array that lives for the rest of the program. Each
/// entry crate's FFI surface publishes it into its own
/// `__nx_<aspect>__system_argv` global.
#[cfg(feature = "ffi")]
pub fn system_argv() -> Option<(i32, *mut *mut c_char)> {
    let parsed_ptr = PARSED_ARGS.load(Ordering::Acquire);
    if parsed_ptr.is_null() {
        return None;
    }
    // SAFETY: PARSED_ARGS is set once during setup_from() and never freed.
    let parsed = unsafe { &*parsed_ptr };
    let argc = parsed.cstrings.len() as i32;
    let argv = parsed.argv_ptrs.as_ptr() as *mut *mut c_char;
    Some((argc, argv))
}

/// `Sync` wrapper for a process-lifetime, null-terminated empty argv array.
///
/// Backs each entry crate's `__nx_<aspect>__system_argv` global before
/// [`setup_from`] installs the real command line. Shared here so the NRO and
/// NSO entry crates do not each redefine the same empty-argv backing.
#[cfg(feature = "ffi")]
pub struct EmptyArgv([*mut c_char; 1]);

// SAFETY: the array is immutable and holds only a null pointer; sharing it
// across threads cannot observe a data race.
#[cfg(feature = "ffi")]
unsafe impl Sync for EmptyArgv {}

#[cfg(feature = "ffi")]
impl EmptyArgv {
    /// Pointer to the null-terminated empty argv array.
    pub const fn as_ptr(&self) -> *mut *mut c_char {
        self.0.as_ptr().cast_mut()
    }
}

/// Shared empty argv — a null-terminated array with zero arguments.
#[cfg(feature = "ffi")]
pub static EMPTY_ARGV: EmptyArgv = EmptyArgv([ptr::null_mut()]);

/// Iterator over command-line arguments (like `std::env::Args`).
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

        // SAFETY: PARSED_ARGS is set once during setup_from() and never freed.
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
        // SAFETY: PARSED_ARGS is set once during setup_from() and never freed.
        let parsed = unsafe { &*parsed_ptr };
        let remaining = parsed.cstrings.len().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Args {}

/// Parsed command-line arguments.
///
/// Written exactly once during [`setup_from`], then read-only.
struct ParsedArgs {
    /// CString storage — owns the null-terminated argument data.
    cstrings: Vec<CString>,
    /// Pre-built C-style argv array — pointers into `cstrings` plus a NULL
    /// terminator. Exists to keep the allocation alive: each entry crate's
    /// `__nx_<aspect>__system_argv` points into it. Compiled only with the
    /// C-FFI surface; the Rust [`Args`] iterator reads `cstrings` directly.
    #[cfg(feature = "ffi")]
    argv_ptrs: Box<[*mut c_char]>,
}

// SAFETY: ParsedArgs is only written once during init, then read-only.
unsafe impl Sync for ParsedArgs {}

/// Parses an argv string into individual arguments.
///
/// Handles quoted strings and whitespace separation.
fn parse_argv(args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_arg = String::new();
    let mut in_quote = false;

    for ch in args.chars() {
        if in_quote {
            if ch == '"' {
                // End quote.
                in_quote = false;
                if !current_arg.is_empty() {
                    result.push(current_arg.clone());
                    current_arg.clear();
                }
            } else {
                // Inside quote, add character.
                current_arg.push(ch);
            }
        } else if ch == '"' {
            // Start quote.
            in_quote = true;
        } else if ch.is_whitespace() {
            // Whitespace separator.
            if !current_arg.is_empty() {
                result.push(current_arg.clone());
                current_arg.clear();
            }
        } else {
            // Regular character.
            current_arg.push(ch);
        }
    }

    // Push the final argument, if any.
    if !current_arg.is_empty() {
        result.push(current_arg);
    }

    result
}
