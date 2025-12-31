//! # nx-panic-handler
//!
//! Custom panic handler for Nintendo Switch homebrew applications.
//!
//! This module provides a panic handler that calls the Switch's debug break
//! system call with a Panic reason, allowing for better debugging and error
//! reporting in homebrew applications.
//!
//! The panic handler formats messages using Rust's standard "panicked at" format
//! and passes them to `svcBreak` via a 512-byte static buffer, following the same
//! approach as libnx's `fatalThrow` and `diagAbortWithResult` functions.
//!
//! ## Minimal SVC Implementation
//!
//! This crate contains only the minimal supervisor call code needed for the panic
//! handler, making it independent of the full `nx-svc` crate. This allows other
//! crates to link the panic handler without pulling in the entire SVC library.

#![no_std]

use core::{fmt::Write as _, panic::PanicInfo};

/// Maximum size for panic message buffer
const MSG_BUFFER_SIZE: usize = 512;

/// Custom panic handler that calls the Switch debug break system call.
///
/// When a panic occurs, this handler will:
/// 1. Format the panic message using Rust's standard "panicked at" format
/// 2. Call `svcBreak` with `BreakReason::Panic`
/// 3. Pass the formatted message buffer address and size to svcBreak
///
/// This follows the same approach as libnx's `fatalThrow` and `diagAbortWithResult`,
/// and uses Rust's standard panic message format for consistency.
#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    /// Static buffer for storing panic messages
    ///
    /// This buffer is used to store the formatted panic message so it can be
    /// passed to svcBreak via a pointer. The buffer is static to ensure it
    /// remains valid for the duration of the break event.
    static mut MSG_BUFFER: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];

    // Format the panic message using Rust's standard Display implementation
    // This gives us the standard "panicked at '<message>', <file>:<line>:<column>" format

    // SAFETY: Taking a raw pointer to static mut and creating a slice from it is safe.
    // The pointer is valid, properly aligned, and we have exclusive access during panic.
    let (buf_slice, buf_ptr) = unsafe {
        let raw_ptr = &raw mut MSG_BUFFER;
        let slice = core::slice::from_raw_parts_mut(raw_ptr as *mut u8, MSG_BUFFER_SIZE);
        (slice, raw_ptr)
    };

    // Create a cursor to write into the buffer
    let mut cursor = Cursor::new(buf_slice);

    // Write the panic info using Rust's standard Display format
    // This automatically handles the "panicked at" formatting
    let _ = write!(cursor, "{}", info);

    let written = cursor.position();
    let (msg_ptr, msg_len) = (buf_ptr as usize, written);

    // Call the debug break system call with panic reason.
    // Pass the panic message buffer address and size, following the same
    // pattern as libnx's `fatalThrow` and `diagAbortWithResult` functions.
    svc::break_event(svc::BreakReason::Panic, msg_ptr, msg_len);
}

/// A cursor implementation for writing to a byte buffer in no_std environments.
///
/// Wraps a mutable byte slice and tracks the current write position.
/// Provides `Write` trait implementation for formatting operations.
struct Cursor<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// Creates a new cursor wrapping the provided buffer.
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Returns the current write position in the buffer.
    fn position(&self) -> usize {
        self.pos
    }
}

impl<'a> core::fmt::Write for Cursor<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len().saturating_sub(self.pos);
        let to_write = bytes.len().min(remaining);

        if to_write > 0 {
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
        }

        Ok(())
    }
}

/// Minimal supervisor call (SVC) implementation for the panic handler.
///
/// This module contains only the bare minimum SVC functionality needed to trigger
/// a debug break when a panic occurs. It includes:
///
/// - [`BreakReason`] enum for specifying the type of break event
/// - [`break_event()`] function to trigger the break with a message buffer
///
/// This is intentionally minimal to avoid pulling in the full `nx-svc` crate,
/// allowing the panic handler to remain lightweight and dependency-free.
mod svc {
    /// Result code returned from supervisor calls.
    type ResultCode = u32;

    /// SVC number for `svcBreak` system call.
    const BREAK: u16 = 0x26;

    /// Reasons for triggering a debug break event.
    ///
    /// These values are passed to the `svcBreak` system call to indicate
    /// the reason for breaking into the debugger.
    #[repr(u32)]
    #[allow(dead_code)]
    pub(super) enum BreakReason {
        /// Program panic
        Panic = 0,
        /// Assertion failure
        Assert = 1,
        /// User-triggered break
        User = 2,
        /// Pre-DLL load event
        PreLoadDll = 3,
        /// Post-DLL load event
        PostLoadDll = 4,
        /// Pre-DLL unload event
        PreUnloadDll = 5,
        /// Post-DLL unload event
        PostUnloadDll = 6,
        /// C++ exception
        CppException = 7,
        /// Notification-only flag
        NotificationOnlyFlag = 0x80000000,
    }

    /// Trigger a debug event
    ///
    /// This function is used to trigger a debug event.
    /// It will cause the system to break into the debugger.
    pub(super) fn break_event(reason: BreakReason, address: usize, size: usize) -> ! {
        let _ = unsafe { svc_break(reason, address, size) };
        unreachable!()
    }

    /// Breaks execution.
    ///
    /// `Result svcBreak(BreakReason reason, uintptr_t address, uintptr_t size);`
    ///
    /// Syscall code: BREAK (`0x26`).
    ///
    /// | Arg | Name | Description |
    /// | --- | --- | --- |
    /// | IN | _reason_ | Break reason (see [BreakReason]) |
    /// | IN | _address_ | Address of the buffer to pass to the debugger |
    /// | IN | _size_ | Size of the buffer to pass to the debugger |
    ///
    /// Ref: <https://switchbrew.org/wiki/SVC#Break>
    #[unsafe(naked)]
    unsafe extern "C" fn svc_break(reason: BreakReason, address: usize, size: usize) -> ResultCode {
        core::arch::naked_asm!(
            "svc {code}", // Issue the SVC call with immediate value 0x26
            "ret",
            code = const BREAK,
        );
    }
}
