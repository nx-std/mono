//! Wire structures the error applet reads.
//!
//! Both structures are fixed-layout payloads written verbatim into an
//! `IStorage`, so they are modelled as `repr(C)` structs and converted with
//! zerocopy rather than serialised field by field.

use core::mem::size_of;

use static_assertions::const_assert_eq;

/// Common arguments every library applet reads as its **first** storage.
///
/// libnx calls this `LibAppletArgs`. The applet rejects a `version` other than
/// 1, and reads `size` to decide how much of the struct it can trust.
#[derive(Debug, Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C)]
pub struct CommonArguments {
    /// Struct version. Must be 1; version 0 is not supported.
    pub version: u32,
    /// Size of this struct.
    pub size: u32,
    /// Library applet API version.
    pub la_version: u32,
    /// Theme colour the caller expects the applet to render with.
    pub expected_theme_color: i32,
    /// Whether the applet plays its startup sound.
    pub play_startup_sound: u8,
    _padding: [u8; 7],
    /// System tick at the moment the arguments are pushed.
    pub tick: u64,
}

const_assert_eq!(size_of::<CommonArguments>(), 0x20);

impl CommonArguments {
    /// Builds the common arguments for a library applet launched at `tick`.
    ///
    /// `expected_theme_color` is left at zero. libnx sources it from
    /// `appletGetThemeColorType`, which costs another round trip and only
    /// affects the palette the applet renders with, never whether it runs.
    pub const fn new(la_version: u32, tick: u64) -> Self {
        Self {
            version: 1,
            size: size_of::<Self>() as u32,
            la_version,
            expected_theme_color: 0,
            play_startup_sound: 0,
            _padding: [0; 7],
            tick,
        }
    }
}

/// Which kind of error the applet should present.
///
/// Only [`Application`](Self::Application) is implemented here; the rest are
/// listed because they share [`ErrorCommonHeader`], and each needs its own arg
/// struct before it can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorType {
    /// Error code or result, with an optional backtrace.
    Normal = 0,
    /// System error with custom text.
    System = 1,
    /// Application error with custom text.
    Application = 2,
    /// EULA display.
    Eula = 3,
    /// Parental controls.
    Pctl = 4,
    /// Recorded error.
    Record = 5,
    /// System update EULA.
    SystemUpdateEula = 8,
}

/// Header shared by every error arg struct.
///
/// Packed, because it is the first member of a packed arg struct; nothing in it
/// is naturally aligned anyway.
#[derive(Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C, packed)]
pub struct ErrorCommonHeader {
    /// Which error variant follows, see [`ErrorType`].
    pub error_type: u8,
    /// When clear, the applet returns without jumping to the error viewer.
    pub jump_flag: u8,
    _unknown: [u8; 3],
    /// Set when an extra storage carrying a context or backtrace is pushed.
    pub context_flag: u8,
    /// When clear the applet uses the error code; otherwise it derives one.
    pub result_flag: u8,
    /// The `ErrorCommonArg` counterpart of `context_flag`.
    pub context_flag2: u8,
}

const_assert_eq!(size_of::<ErrorCommonHeader>(), 8);

/// Argument storage for an application error dialog.
///
/// Packed: `language_code` sits at offset 12, so the natural `u64` alignment
/// does not hold and the compiler must not insert padding to restore it.
#[derive(Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C, packed)]
pub struct ErrorApplicationArg {
    /// Common header; `error_type` is [`ErrorType::Application`].
    pub header: ErrorCommonHeader,
    /// Decimal number the dialog displays as the error code.
    pub error_number: u32,
    /// Language to render in. Zero lets the applet pick.
    pub language_code: u64,
    /// UTF-8 message shown in the dialog itself.
    pub dialog_message: [u8; 0x800],
    /// UTF-8 message shown behind the dialog's "Details" button.
    pub fullscreen_message: [u8; 0x800],
}

const_assert_eq!(size_of::<ErrorApplicationArg>(), 0x1014);

impl ErrorApplicationArg {
    /// Builds an application-error argument carrying `dialog_message`, and
    /// `fullscreen_message` behind "Details" when given.
    ///
    /// Messages longer than the 2 KB field are truncated at a UTF-8 character
    /// boundary; see [`copy_message`].
    pub fn new(dialog_message: &str, fullscreen_message: Option<&str>) -> Self {
        let mut arg = Self {
            header: ErrorCommonHeader {
                error_type: ErrorType::Application as u8,
                jump_flag: 1,
                _unknown: [0; 3],
                context_flag: 0,
                result_flag: 0,
                context_flag2: 0,
            },
            error_number: 0,
            language_code: 0,
            dialog_message: [0; 0x800],
            fullscreen_message: [0; 0x800],
        };

        copy_message(&mut arg.dialog_message, dialog_message);
        if let Some(message) = fullscreen_message {
            copy_message(&mut arg.fullscreen_message, message);
        }

        arg
    }
}

/// Copies `src` into `dest` as a NUL-terminated UTF-8 string, truncating to fit.
///
/// `dest` arrives zeroed, so stopping one byte short of the end leaves the
/// terminator in place. Truncation walks back to a character boundary: cutting
/// mid-codepoint would hand the applet a byte sequence that is not UTF-8, and it
/// renders the message rather than validating it.
fn copy_message(dest: &mut [u8; 0x800], src: &str) {
    let limit = dest.len() - 1;

    let mut end = src.len().min(limit);
    while end > 0 && !src.is_char_boundary(end) {
        end -= 1;
    }

    dest[..end].copy_from_slice(&src.as_bytes()[..end]);
}
