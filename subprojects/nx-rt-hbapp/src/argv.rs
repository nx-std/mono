//! Command-line argument reader (NRO)
//!
//! An NRO receives its command line as a single string through the homebrew
//! loader's configuration block. [`get_nro_args`] reads that string and hands
//! it to `nx-sys-args`, which scans it and holds the result; this crate keeps
//! only the part that depends on being an NRO.
//!
//! Homebrew-specific handling stays here too: [`strip_nxlink_suffix`] removes
//! the trailing `XXXXXXXX_NXLINK_` token that nxlink appends and hands back
//! the host address it carried.

use core::{
    ffi::CStr,
    slice,
};

use crate::env;

/// Sets up argv parsing.
///
/// This function can be called multiple times safely: initialization only
/// happens once, and subsequent calls are no-ops.
///
/// Parsing is all this does. Publishing the result to the C-facing globals is
/// the caller's step, because the globals belong to the C boundary and this
/// module sits below it; a module that reached up into its own crate's `ffi`
/// to announce itself would make the two depend on each other.
///
/// Returns the nxlink host address the loader appended, when it appended one.
///
/// Nothing here allocates, so this needs no heap and may run before one exists.
pub fn setup() -> Option<u32> {
    // A homebrew NRO sources its arguments from the loader configuration.
    // SAFETY: this runs during initialization, before any other thread exists,
    // and the loader's argument buffer is this process's to write.
    let args = unsafe { get_nro_args() }?;

    // Strip the homebrew-only nxlink host suffix before the shared scanner sees
    // the string. The borrow ends here, so the slice below is the only one live.
    let content = &args[..args.len() - 1];
    let (content_len, nxlink_host) = strip_nxlink_suffix(content);

    // One byte past the content is the terminator slot: the whitespace that
    // preceded a stripped nxlink token, or the loader's own nul when nothing
    // was stripped.
    nx_sys_args::setup_from(&mut args[..=content_len]);

    nxlink_host
}

/// Reads the NRO command-line arguments from the homebrew loader config.
///
/// Returns the loader-supplied argument string **including its terminating
/// nul**, or `None` when the loader passed no `argv` pointer or an empty
/// string. The bytes are handed on unvalidated: an argument the encoding rules
/// do not describe is still an argument the loader meant to pass.
///
/// The slice stops at the terminator rather than spanning whatever the loader
/// sized the buffer to, because bytes past it were never written.
///
/// # Safety
///
/// Must be called during initialization, before any other thread exists. The
/// `argv` pointer from [`env::argv`] must address a valid, nul-terminated
/// string that stays valid for the lifetime of the program, and this process
/// must be the only writer of it.
unsafe fn get_nro_args() -> Option<&'static mut [u8]> {
    let argv_ptr = env::argv()?;

    // SAFETY: argv_ptr comes from the homebrew loader and addresses a valid
    // nul-terminated string. The borrow ends with this statement, leaving the
    // pointer as the only way into the buffer.
    let len = unsafe { CStr::from_ptr(argv_ptr.as_ptr()) }
        .to_bytes_with_nul()
        .len();

    // A one-byte string is the terminator alone: no arguments were passed.
    if len <= 1 {
        return None;
    }

    // SAFETY: `len` counts the string and its terminator, all of them written
    // by the loader, and the caller guarantees this process is their only
    // writer for as long as it runs.
    Some(unsafe { slice::from_raw_parts_mut(argv_ptr.as_ptr().cast::<u8>(), len) })
}

/// Strips the trailing `XXXXXXXX_NXLINK_` token nxlink appends to the argument
/// string.
///
/// Returns how much of `args` to keep and the host address the token carried,
/// or the full length and `None` when no well-formed nxlink token is present.
/// The token is the last whitespace-delimited word: 8 hexadecimal digits
/// followed by the `_NXLINK_` marker, stripped only when a real argument
/// precedes it.
///
/// A length rather than a subslice, because the caller goes on to write into
/// the same buffer and cannot be holding a borrow of it.
///
/// The host is handed back rather than published from here, so that this
/// module does not have to reach into the crate's C boundary to announce it.
fn strip_nxlink_suffix(args: &[u8]) -> (usize, Option<u32>) {
    /// Marker that terminates the nxlink token, after the 8-hex-digit host.
    const NXLINK_MARKER: &[u8] = b"_NXLINK_";
    /// Full nxlink token length: 8 hexadecimal digits plus the marker.
    const NXLINK_TOKEN_LEN: usize = 8 + NXLINK_MARKER.len();

    // nxlink appends the host as the last whitespace-delimited token; with no
    // separator there is no real argument before it, so leave `args` alone.
    let trimmed = args.trim_ascii_end();
    let Some(separator) = trimmed.iter().rposition(|byte| byte.is_ascii_whitespace()) else {
        return (args.len(), None);
    };
    let rest = &trimmed[..separator];
    let token = &trimmed[separator + 1..];

    // nxlink only appends its host alongside real arguments; if nothing
    // precedes the token, treat it as an ordinary argument.
    if rest.trim_ascii().is_empty() {
        return (args.len(), None);
    }
    if token.len() != NXLINK_TOKEN_LEN || !token.ends_with(NXLINK_MARKER) {
        return (args.len(), None);
    }
    // The token's first 8 bytes are the hexadecimal host address, so they are
    // ASCII whenever the token is well-formed and this rejects it when not.
    let Ok(digits) = core::str::from_utf8(&token[..8]) else {
        return (args.len(), None);
    };
    let Ok(host) = u32::from_str_radix(digits, 16) else {
        return (args.len(), None);
    };
    (rest.len(), Some(host))
}
