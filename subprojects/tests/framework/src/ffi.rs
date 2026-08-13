//! The C surface, as `include/nx_tests_framework.h` declares it.
//!
//! This adds a surface rather than replacing one: nothing upstream reports in this protocol, so
//! there is no symbol to override and no linker script. A C caller reaches these by name.
//!
//! # What is decided here
//!
//! Two things arrive as numbers and leave as types, because this is the edge and the inside should
//! not have to keep asking what a number meant:
//!
//! - A case's **result code** becomes an [`Outcome`]. The protocol treats a skip, a todo and a
//!   failure differently, and past this point which one it is has already been settled.
//! - The host's **address** becomes an [`Ipv4Addr`], or nothing at all when the program was
//!   launched by hand.
//!
//! Everything the C runtime knows and this crate must not ask for -- the system version, the host,
//! the directory reports are filed in -- crosses here as an argument.

use alloc::string::{
    String,
    ToString as _,
};
use core::{
    ffi::{
        CStr,
        c_char,
    },
    net::Ipv4Addr,
};

use crate::document::{
    HosVersion,
    Outcome,
    Run,
};

/// What the case function returned when everything it asserted held.
///
/// This and the three below are `harness.h`'s, which is where a case gets them from.
const TEST_SUCCESS: i32 = 0;
/// What a case that is not written yet returns.
const TEST_TODO: i32 = -501;
/// What a case that declined to run returns.
const TEST_SKIPPED: i32 = -502;
/// What the harness reports when a fixture could not be built.
const TEST_SETUP_FAILED: i32 = -503;

/// What a reader needs in order to know which run it is looking at, as C declares it.
#[repr(C)]
pub struct NxTestsFrameworkRun {
    /// The name this suite reports under, which is also the name its file is written to.
    suite: *const c_char,
    /// The build this was compiled from.
    build: *const c_char,
    /// The directory the report is filed in.
    report_dir: *const c_char,
    /// The system version, already taken apart by the caller that asked the runtime for it.
    hos_major: u8,
    /// See [`NxTestsFrameworkRun::hos_major`].
    hos_minor: u8,
    /// See [`NxTestsFrameworkRun::hos_major`].
    hos_micro: u8,
    /// Whether the run is happening under a custom firmware.
    atmosphere: bool,
    /// Whether the runner launched this rather than a person.
    unattended: bool,
}

/// Opens the document. Called once, before any case runs.
///
/// Does nothing when `run` is null, which leaves every later call with nowhere to report to.
///
/// # Safety
///
/// `run` must be null or point to a live [`NxTestsFrameworkRun`] whose three strings are live
/// nul-terminated strings for the length of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tests_framework__begin(run: *const NxTestsFrameworkRun) {
    // SAFETY: the caller guarantees `run` is null or points to a live `NxTestsFrameworkRun`.
    let Some(run) = (unsafe { run.as_ref() }) else {
        return;
    };

    crate::begin(Run {
        // SAFETY: the caller guarantees `suite` is a live nul-terminated string.
        suite: unsafe { text(run.suite) },
        // SAFETY: the caller guarantees `build` is a live nul-terminated string.
        build: unsafe { text(run.build) },
        // SAFETY: the caller guarantees `report_dir` is a live nul-terminated string.
        report_dir: unsafe { text(run.report_dir) },
        hos: HosVersion {
            major: run.hos_major,
            minor: run.hos_minor,
            micro: run.hos_micro,
            atmosphere: run.atmosphere,
        },
        unattended: run.unattended,
    });
}

/// Writes a line the protocol carries and ignores.
///
/// # Safety
///
/// `text` must be null or a live nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tests_framework__comment(text: *const c_char) {
    // SAFETY: the caller guarantees `text` is null or a live nul-terminated string.
    let owned = unsafe { text_from(text) };
    crate::comment(&owned);
}

/// Reports one case, numbering it after the last one reported.
///
/// # Safety
///
/// `title` must be null or a live nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tests_framework__case(title: *const c_char, rc: i32) {
    // SAFETY: the caller guarantees `title` is null or a live nul-terminated string.
    let title = unsafe { text_from(title) };
    crate::case(&title, outcome_of(rc));
}

/// Reports a case the harness itself could not run.
///
/// Distinct from a case that failed: what went wrong was the machinery around the test rather than
/// the thing under test. It is still counted and numbered, because a case that did not run is not a
/// case that passed.
///
/// # Safety
///
/// `title` and `reason` must each be null or a live nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tests_framework__harness_error(
    title: *const c_char,
    reason: *const c_char,
) {
    // SAFETY: the caller guarantees `title` is null or a live nul-terminated string.
    let title = unsafe { text_from(title) };
    // SAFETY: the caller guarantees `reason` is null or a live nul-terminated string.
    let reason = unsafe { text_from(reason) };
    crate::case(&title, Outcome::HarnessError { reason });
}

/// Closes the document by stating how many cases there were.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tests_framework__plan() {
    crate::plan();
}

/// Files the document with the card and sends it to the host.
///
/// `host` is the address the program was pushed from, in the layout the C runtime holds it in, and
/// zero when the program was launched by hand. A caller that passes a non-zero address guarantees a
/// socket driver is already running.
///
/// Returns 0 when both the card and the host took the document, and -1 when either did not. There
/// is nothing a caller can do about a failure past showing it, and the console has already shown
/// every case the document holds.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_tests_framework__report(host: u32) -> i32 {
    // The address is held by the C runtime the way it arrived off the wire, so its first octet is
    // the byte at the lowest address; read as a native word on this machine that is the low byte.
    let host = match host {
        0 => None,
        raw => Some(Ipv4Addr::from(raw.to_le_bytes())),
    };

    let report = crate::report(host);
    match report.card.is_ok() && report.host.is_ok() {
        true => 0,
        false => -1,
    }
}

/// Maps a case's result code to what the protocol says about it.
fn outcome_of(rc: i32) -> Outcome {
    match rc {
        TEST_SUCCESS => Outcome::Passed,
        TEST_TODO => Outcome::Todo,
        TEST_SKIPPED => Outcome::Skipped,
        TEST_SETUP_FAILED => Outcome::SetupFailed,
        rc => Outcome::Failed { rc },
    }
}

/// Reads a C string that the caller has promised is there.
///
/// # Safety
///
/// `ptr` must be a live nul-terminated string for the length of the call.
unsafe fn text(ptr: *const c_char) -> String {
    // SAFETY: the caller guarantees `ptr` is a live nul-terminated string.
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string()
}

/// Reads a C string that may be absent.
///
/// Text that is not valid UTF-8 is reported with the offending bytes replaced rather than dropped: a
/// case that goes unreported renumbers every case after it, which is a worse thing to hand a reader
/// than a title with a question mark in it.
///
/// # Safety
///
/// `ptr` must be null or a live nul-terminated string for the length of the call.
unsafe fn text_from(ptr: *const c_char) -> String {
    match ptr.is_null() {
        true => String::new(),
        // SAFETY: `ptr` is not null, and the caller guarantees it is then a live nul-terminated
        // string.
        false => unsafe { text(ptr) },
    }
}
