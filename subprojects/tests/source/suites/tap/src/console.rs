//! The screen, written to as each case finishes.
//!
//! The only destination that is written to while the run is happening, because it is the only one
//! with somebody in front of it. The card and the host are written once at the end, out of what the
//! document accumulated.
//!
//! # Why the text goes out through the C library
//!
//! The cases print too, and they print through the C library's `stdout`, which buffers. Writing
//! this alongside them through the descriptor underneath would put two producers on one stream with
//! only one of them buffered, and the lines would come out interleaved in an order neither of them
//! chose. So the document goes out the same way the cases' own output does, and the buffering
//! orders both.

use alloc::string::String;
use core::ffi::{
    c_char,
    c_int,
};

unsafe extern "C" {
    /// Writes formatted text to the C library's standard output.
    ///
    /// Declared rather than reached through a descriptor so that this shares the stream, and
    /// therefore the ordering, with everything a case prints.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Writes `text` to the screen, exactly as given.
///
/// The text is passed as an argument rather than as the format, so a title containing a percent
/// sign is a title and not a directive.
pub fn write(text: &str) {
    // The C library reads to a terminator, which a `&str` does not carry, so the text is copied
    // into a buffer that ends in one.
    let mut owned = String::with_capacity(text.len() + 1);
    owned.push_str(text);
    owned.push('\0');

    // SAFETY: `owned` ends in a nul byte and holds no interior one, since it was built from a
    // `&str` with the terminator appended, so the C library reads exactly the text and stops. The
    // format string is a literal with one `%s` matching the one argument. Both live until the call
    // returns, and neither pointer is retained.
    unsafe { printf(c"%s".as_ptr(), owned.as_ptr().cast::<c_char>()) };
}
