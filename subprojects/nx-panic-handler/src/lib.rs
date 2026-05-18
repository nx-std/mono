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

use core::{
    fmt::Write as _,
    panic::PanicInfo,
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

/// Maximum size for the panic message buffer.
const MSG_BUFFER_SIZE: usize = 512;

/// Custom panic handler that calls the Switch debug break system call.
///
/// When a panic occurs, this handler will:
/// 1. Format the panic message using Rust's standard "panicked at" format
/// 2. Call `svcBreak` with a Panic break reason
/// 3. Pass the formatted message buffer address and size to `svcBreak`
///
/// This follows the same approach as libnx's `fatalThrow` and `diagAbortWithResult`,
/// and uses Rust's standard panic message format for consistency.
///
/// # Concurrency
///
/// `MSG_BUFFER` is a single shared `static`, so two situations could otherwise
/// race for `&mut` access to it: concurrent panics on separate threads, and a
/// nested panic (one raised while this thread is still formatting a message).
///
/// A one-shot atomic claim elects exactly one **winner** — the thread whose
/// `FORMATTER.swap` returns the non-null buffer pointer. Only the winner formats
/// the panic message into `MSG_BUFFER`; holding the unique pointer *is* its
/// exclusive-access token. Every **loser** (a concurrent or nested panic) reads
/// back a null pointer, skips formatting, and breaks with an empty buffer.
///
/// Losers break immediately instead of waiting for the winner: a panic handler
/// must never block, and a nested panic runs on the winner's own thread, so
/// waiting would deadlock. The claim guarantees `&mut MSG_BUFFER` is never
/// aliased across threads.
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    /// Static buffer for storing the formatted panic message.
    ///
    /// The buffer is `static` so its address stays valid for the debugger to
    /// read for the duration of the break event.
    static mut MSG_BUFFER: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];

    /// One-shot claim on `MSG_BUFFER`, guarding it against concurrent panics.
    ///
    /// Holds the buffer pointer until the first panic claims it, then null. The
    /// first panicking thread swaps the pointer out and gains exclusive access;
    /// later threads observe null and skip formatting entirely.
    static FORMATTER: AtomicPtr<[u8; MSG_BUFFER_SIZE]> = AtomicPtr::new(&raw mut MSG_BUFFER);

    // Only the thread that claims the buffer pointer may touch MSG_BUFFER. A
    // losing thread (a concurrent or nested panic) breaks with an empty buffer.
    let buf_ptr = FORMATTER.swap(ptr::null_mut(), Ordering::AcqRel);
    let (msg_ptr, msg_len) = if !buf_ptr.is_null() {
        // SAFETY: `swap` atomically transferred the unique non-null buffer
        // pointer to this thread; no other thread can observe it again.
        let buf = unsafe { &mut *buf_ptr };

        // Write the panic info using Rust's standard Display format, which
        // produces the standard "panicked at" message.
        let mut writer = SliceWriter::new(buf);
        let _ = write!(writer, "{info}");

        (buf_ptr as usize, writer.position())
    } else {
        // Lost the claim — break with a null, zero-length message.
        (0, 0)
    };

    // Call the debug break system call with the panic reason. Pass the panic
    // message buffer address and size, following the same pattern as libnx's
    // `fatalThrow` and `diagAbortWithResult` functions.
    svc::break_panic(msg_ptr, msg_len);
}

/// A write-only sink over a fixed-size byte buffer for `no_std` formatting.
///
/// Wraps a mutable byte slice and tracks the current write position. Writes that
/// exceed the buffer capacity are silently truncated, which is acceptable for
/// best-effort panic messages.
struct SliceWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceWriter<'a> {
    /// Creates a new writer over the provided buffer.
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Returns the number of bytes written so far.
    fn position(&self) -> usize {
        self.pos
    }
}

impl core::fmt::Write for SliceWriter<'_> {
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
/// a debug break when a panic occurs. It includes [`break_panic()`] to break with
/// a buffer describing the panic.
///
/// This is intentionally minimal to avoid pulling in the full `nx-svc` crate,
/// allowing the panic handler to remain lightweight and dependency-free.
mod svc {
    /// Result code returned from supervisor calls.
    type ResultCode = u32;

    /// SVC number for the `svcBreak` system call.
    const BREAK: u16 = 0x26;

    /// `BreakReason` value for a program panic.
    const BREAK_REASON_PANIC: u32 = 0;

    /// Triggers a panic debug break event and never returns.
    ///
    /// `address` and `size` describe the buffer handed to the debugger.
    ///
    /// `svcBreak` only terminates the process when no debugger is attached; if a
    /// debugger is attached and resumes the process, the call returns. The break
    /// is therefore reissued in a loop so this function upholds its `!` return
    /// type, mirroring libnx's `diagAbortWithResult`.
    pub(super) fn break_panic(address: usize, size: usize) -> ! {
        loop {
            // SAFETY: `address`/`size` describe the static panic-message buffer,
            // which stays readable for the debugger for the process's lifetime.
            let _ = unsafe { svc_break(BREAK_REASON_PANIC, address, size) };
        }
    }

    /// Breaks execution.
    ///
    /// `Result svcBreak(BreakReason reason, uintptr_t address, uintptr_t size);`
    ///
    /// Syscall code: BREAK (`0x26`).
    ///
    /// | Arg | Name | Description |
    /// | --- | --- | --- |
    /// | IN | _reason_ | Break reason |
    /// | IN | _address_ | Address of the buffer to pass to the debugger |
    /// | IN | _size_ | Size of the buffer to pass to the debugger |
    ///
    /// Ref: <https://switchbrew.org/wiki/SVC#Break>
    ///
    /// # Safety
    ///
    /// If a debugger is attached, `address` must point to `size` bytes of valid,
    /// readable memory for the debugger to inspect.
    #[unsafe(naked)]
    unsafe extern "C" fn svc_break(reason: u32, address: usize, size: usize) -> ResultCode {
        core::arch::naked_asm!(
            "svc {code}", // Issue the SVC call with immediate value 0x26
            "ret",
            code = const BREAK,
        );
    }
}
