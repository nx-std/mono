//! The POSIX `TZ` specification newlib reads the local timezone from.
//!
//! Horizon reports the device's timezone as a short name plus an offset from UTC. newlib's
//! `localtime`, `mktime` and friends instead read the `TZ` environment variable, so the runtime
//! renders the one into the other at startup. [`TzSpec`] is that rendering and [`set`] publishes
//! it. This is the second half of libnx's `__libnx_init_time`.

use core::ffi::{
    c_char,
    c_int,
};

/// Longest timezone name a rendered spec quotes.
///
/// Horizon reports the name in an 8-byte field whose last byte is reserved for the terminator,
/// so at most seven bytes are ever meaningful.
const NAME_CAPACITY: usize = 7;

/// Width of the `±HH:MM:SS` offset that follows the quoted name.
const OFFSET_LEN: usize = 9;

/// Capacity of a rendered spec: `<`, the name, `>`, the offset, and the terminating NUL.
const SPEC_CAPACITY: usize = 1 + NAME_CAPACITY + 1 + OFFSET_LEN + 1;

/// Seconds in a minute.
const SECS_PER_MINUTE: u32 = 60;

/// Seconds in an hour.
const SECS_PER_HOUR: u32 = 60 * SECS_PER_MINUTE;

/// Hours in a day.
const HOURS_PER_DAY: u32 = 24;

/// A rendered POSIX `TZ` specification, such as `<JST>-09:00:00`.
///
/// The name is always angle-quoted: real timezone names contain digits and `-`/`+`, which POSIX
/// would otherwise read as the start of the offset.
pub struct TzSpec {
    /// The rendered spec followed by a NUL; bytes past the terminator are unused zeroes.
    buf: [u8; SPEC_CAPACITY],
    /// Length of the rendered spec, excluding the terminating NUL.
    len: usize,
}

impl TzSpec {
    /// Renders the timezone `name` and its `utc_offset_secs` as a POSIX `TZ` specification.
    ///
    /// `name` is read up to its first NUL and truncated to the seven bytes a spec can quote.
    /// `utc_offset_secs` is the offset Horizon reports — seconds to add to UTC to reach local
    /// time — and its hours field is reduced modulo a day, as libnx does.
    pub fn new(name: &[u8], utc_offset_secs: i32) -> Self {
        let mut buf = [0u8; SPEC_CAPACITY];
        let mut len = 0;

        buf[len] = b'<';
        len += 1;
        for &byte in name.iter().take(NAME_CAPACITY).take_while(|&&b| b != 0) {
            buf[len] = byte;
            len += 1;
        }
        buf[len] = b'>';
        len += 1;

        // POSIX states the offset as the value added to *local* time to reach UTC, so its sign is
        // the opposite of the UTC offset Horizon reports.
        buf[len] = if utc_offset_secs < 0 { b'+' } else { b'-' };
        len += 1;

        let magnitude = utc_offset_secs.unsigned_abs();
        let hours = (magnitude / SECS_PER_HOUR) % HOURS_PER_DAY;
        let minutes = (magnitude / SECS_PER_MINUTE) % SECS_PER_MINUTE;
        let seconds = magnitude % SECS_PER_MINUTE;

        let offset = [
            digit(hours / 10),
            digit(hours % 10),
            b':',
            digit(minutes / 10),
            digit(minutes % 10),
            b':',
            digit(seconds / 10),
            digit(seconds % 10),
        ];
        buf[len..len + offset.len()].copy_from_slice(&offset);
        len += offset.len();

        buf[len] = b'\0';
        Self { buf, len }
    }

    /// The rendered spec as a NUL-terminated byte string, ready to hand to C.
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.buf[..=self.len]
    }
}

/// Publishes `spec` as the process `TZ`, leaving an existing `TZ` untouched.
///
/// A `TZ` the program set for itself therefore wins over the device's timezone, which is the
/// precedence libnx establishes.
pub fn set(spec: &TzSpec) {
    unsafe extern "C" {
        // This is a newlib/libc function
        fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    }

    const NAME: &core::ffi::CStr = c"TZ";

    // The return value is discarded: it reports only that newlib could not record the entry, and
    // there is no fallback timezone to install instead.
    // SAFETY: Both arguments are NUL-terminated byte strings that outlive the call, which is all
    // `setenv` requires of them.
    let _ = unsafe { setenv(NAME.as_ptr(), spec.as_bytes_with_nul().as_ptr().cast(), 0) };
}

/// Renders `value` as a single ASCII digit.
///
/// Every caller has already reduced `value` below ten by dividing or taking a remainder, so the
/// cast cannot lose anything.
fn digit(value: u32) -> u8 {
    // The cast is lossless: `value` is below ten, so it fits a single byte.
    b'0' + value as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_east_of_utc_offset_flips_the_sign_to_minus() {
        //* Given
        let name = b"JST\0\0\0\0\0";

        //* When
        let spec = TzSpec::new(name, 9 * 3600);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<JST>-09:00:00\0",
            "an offset ahead of UTC must render as a POSIX minus offset"
        );
    }

    #[test]
    fn new_west_of_utc_offset_flips_the_sign_to_plus() {
        //* Given
        let name = b"PST\0\0\0\0\0";

        //* When
        let spec = TzSpec::new(name, -8 * 3600);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<PST>+08:00:00\0",
            "an offset behind UTC must render as a POSIX plus offset"
        );
    }

    #[test]
    fn new_offset_with_minutes_and_seconds_fills_every_field() {
        //* Given
        let name = b"NPT\0\0\0\0\0";

        //* When
        let spec = TzSpec::new(name, 5 * 3600 + 45 * 60 + 7);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<NPT>-05:45:07\0",
            "minutes and seconds must each render as two digits"
        );
    }

    #[test]
    fn new_utc_offset_renders_as_minus_zero() {
        //* Given
        let name = b"UTC\0\0\0\0\0";

        //* When
        let spec = TzSpec::new(name, 0);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<UTC>-00:00:00\0",
            "a zero offset is not negative, so it takes the minus sign"
        );
    }

    #[test]
    fn new_name_longer_than_the_quota_truncates_it() {
        //* Given
        let name = b"ABCDEFGH";

        //* When
        let spec = TzSpec::new(name, 0);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<ABCDEFG>-00:00:00\0",
            "only the seven bytes a spec can quote may survive"
        );
    }

    #[test]
    fn new_empty_name_still_yields_a_parsable_spec() {
        //* Given
        let name = b"\0\0\0\0\0\0\0\0";

        //* When
        let spec = TzSpec::new(name, 3600);

        //* Then
        assert_eq!(
            spec.as_bytes_with_nul(),
            b"<>-01:00:00\0",
            "a missing name must not shift the offset out of place"
        );
    }
}
