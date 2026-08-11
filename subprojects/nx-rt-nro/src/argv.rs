//! Command-line argument parsing (NRO)
//!
//! Ports `libnx`'s `argvSetup` for the homebrew NRO output kind: an NRO
//! receives its command line as a single string through the homebrew loader's
//! configuration block. [`get_nro_args`] reads that string; the kind-agnostic
//! scanner, the parsed-argument store, and the [`Args`] iterator are shared
//! from [`nx_rt_core::argv`].
//!
//! Homebrew-specific handling stays here: [`strip_nxlink_suffix`] removes the
//! trailing `XXXXXXXX_NXLINK_` token that nxlink appends and records the host
//! address through the FFI surface.

use core::ffi::CStr;

pub use nx_rt_core::argv::{
    Args,
    args,
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
/// # Safety
///
/// Must be called after the global allocator is initialized.
pub unsafe fn setup() -> Option<u32> {
    // A homebrew NRO sources its arguments from the loader configuration.
    // SAFETY: called during initialization, after the allocator is up.
    // No arguments available.
    let args_str = unsafe { get_nro_args() }?;

    // Strip the homebrew-only nxlink host suffix before the shared scanner
    // sees the string.
    let (args_str, nxlink_host) = strip_nxlink_suffix(args_str);

    nx_rt_core::argv::setup_from(args_str);

    nxlink_host
}

/// Reads the NRO command-line arguments from the homebrew loader config.
///
/// Returns the loader-supplied argument string, or `None` when the loader
/// passed no `argv` pointer or an empty string.
///
/// # Safety
///
/// Must be called during initialization. The `argv` pointer from
/// [`env::argv`] must point to a valid, null-terminated UTF-8 string that
/// remains valid for the lifetime of the program.
unsafe fn get_nro_args() -> Option<&'static str> {
    let argv_ptr = env::argv()?;

    // SAFETY: argv_ptr comes from the homebrew loader and is a valid
    // null-terminated string.
    let argv_str = unsafe { CStr::from_ptr(argv_ptr) };
    if argv_str.is_empty() {
        return None;
    }

    argv_str.to_str().ok()
}

/// Strips the trailing `XXXXXXXX_NXLINK_` token nxlink appends to the argument
/// string.
///
/// Returns the argument string with the suffix removed and the host address it
/// carried, or the string unchanged and `None` when no well-formed nxlink
/// token is present. The token is the last whitespace-delimited word: 8
/// hexadecimal digits followed by the `_NXLINK_` marker, stripped only when a
/// real argument precedes it.
///
/// The host is handed back rather than published from here, so that this
/// module does not have to reach into the crate's C boundary to announce it.
fn strip_nxlink_suffix(args: &str) -> (&str, Option<u32>) {
    /// Marker that terminates the nxlink token, after the 8-hex-digit host.
    const NXLINK_MARKER: &str = "_NXLINK_";
    /// Full nxlink token length: 8 hexadecimal digits plus the marker.
    const NXLINK_TOKEN_LEN: usize = 8 + NXLINK_MARKER.len();

    // nxlink appends the host as the last whitespace-delimited token; with no
    // separator there is no real argument before it, so leave `args` alone.
    let Some((rest, token)) = args.trim_end().rsplit_once(char::is_whitespace) else {
        return (args, None);
    };
    // nxlink only appends its host alongside real arguments; if nothing
    // precedes the token, treat it as an ordinary argument.
    if rest.trim().is_empty() {
        return (args, None);
    }
    if token.len() != NXLINK_TOKEN_LEN || !token.ends_with(NXLINK_MARKER) {
        return (args, None);
    }
    // The token's first 8 characters are the hexadecimal host address.
    let Ok(host) = u32::from_str_radix(&token[..8], 16) else {
        return (args, None);
    };
    (rest, Some(host))
}
