//! The C surface, as `include/nx_netloader.h` declares it.
//!
//! This adds a surface rather than replacing one: nothing in `libnx` serves the netloader protocol,
//! so there is no symbol to override and no linker script. A C caller reaches these by name.
//!
//! The server crosses the boundary as an opaque pointer, because what it holds -- two sockets that
//! close themselves when dropped -- has no C representation and no business being taken apart by a
//! caller. An outcome crosses as a value, because it is one: a caller reads it once and it owns
//! nothing.
//!
//! # Where the two shapes meet
//!
//! Inside, a transfer produces `String`s of whatever length it needs. The C contract is fixed
//! buffers. Everything that copies one into the other happens here, at the edge, and truncates
//! rather than failing: a path that will not fit is one the loader could not have launched anyway,
//! and a reason that will not fit is still worth most of.

use alloc::boxed::Box;
use core::{
    ffi::{
        CStr,
        c_char,
        c_void,
    },
    ptr,
};

use crate::{
    server::Server,
    transfer::Outcome,
};

/// How much room the received program's path gets.
const PATH_SIZE: usize = 256;

/// How much room the command line handed to the next program gets.
const CMDLINE_SIZE: usize = 2048;

/// How much room a failure reason gets.
const ERROR_SIZE: usize = 192;

/// What a receive attempt produced, as C declares it.
#[repr(C)]
pub struct NxNetloaderOutcome {
    /// Where the program was written.
    path: [c_char; PATH_SIZE],
    /// The command line to launch it with.
    cmdline: [c_char; CMDLINE_SIZE],
    /// Why the transfer did not complete.
    error: [c_char; ERROR_SIZE],
}

/// Called as a transfer advances, so the caller can show progress.
type ProgressFn = Option<
    unsafe extern "C" fn(name: *const c_char, received: usize, total: usize, ctx: *mut c_void),
>;

/// Nobody is connecting; nothing was received.
const IDLE: i32 = 0;
/// A program arrived and was written to the drop directory.
const RECEIVED: i32 = 1;
/// A host connected but the transfer did not complete.
const FAILED: i32 = 2;
/// The sockets can no longer be listened on.
const SERVER_LOST: i32 = 3;

/// Binds both sockets and starts listening.
///
/// Returns null when either socket could not be bound, in which case nothing is left open, and when
/// `drop_dir` is null or is not text.
///
/// # Safety
///
/// `drop_dir` must be a live nul-terminated string.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_netloader__server_open(drop_dir: *const c_char) -> *mut Server {
    if drop_dir.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: the caller guarantees `drop_dir` is a live nul-terminated string.
    let Ok(drop_dir) = (unsafe { CStr::from_ptr(drop_dir) }).to_str() else {
        return ptr::null_mut();
    };

    match Server::open(drop_dir) {
        Ok(server) => Box::into_raw(Box::new(server)),
        // The caller is told only that nothing is listening, which is all it can act on: it retries,
        // and the reason a bind failed while the network was down does not change that.
        Err(_) => ptr::null_mut(),
    }
}

/// Frees a server, closing both sockets.
///
/// # Safety
///
/// `server` must be null, or a pointer this module produced that has not already been freed or
/// consumed.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_netloader__server_free(server: *mut Server) {
    if server.is_null() {
        return;
    }
    // SAFETY: the caller guarantees `server` came from this module and has not been freed, so this
    // is the one box that owns it.
    drop(unsafe { Box::from_raw(server) });
}

/// Answers a pending discovery ping, if one has arrived.
///
/// Returns 0 while the socket is still good, and -1 once it has failed in a way waiting will not
/// mend.
///
/// # Safety
///
/// `server` must be a pointer this module produced that has not been freed or consumed.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_netloader__server_answer_discovery(server: *mut Server) -> i32 {
    let Some(server) = (unsafe { server.as_ref() }) else {
        return -1;
    };

    match server.answer_discovery() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Receives one program, if a host is connecting.
///
/// # Safety
///
/// `server` must be a pointer this module produced that has not been freed or consumed, `out` must
/// point to a writable [`NxNetloaderOutcome`], and `extra_arg` must be null or a live
/// nul-terminated string.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_netloader__server_receive(
    server: *mut Server,
    out: *mut NxNetloaderOutcome,
    extra_arg: *const c_char,
    on_progress: ProgressFn,
    progress_ctx: *mut c_void,
) -> i32 {
    let Some(server) = (unsafe { server.as_ref() }) else {
        return SERVER_LOST;
    };
    let Some(out) = (unsafe { out.as_mut() }) else {
        return IDLE;
    };

    // Everything the attempt reports lands in this one value, so it is cleared before anything is
    // written into it rather than each field being cleared where it is filled.
    out.path[0] = 0;
    out.cmdline[0] = 0;
    out.error[0] = 0;

    // An argument that is not text is one the runtime could not be handed, so it is dropped rather
    // than passed on half-decoded.
    let extra_arg = match extra_arg.is_null() {
        true => None,
        // SAFETY: the caller guarantees `extra_arg` is a live nul-terminated string.
        false => unsafe { CStr::from_ptr(extra_arg) }.to_str().ok(),
    };

    let mut report = |name: &str, received: usize, total: usize| {
        let Some(on_progress) = on_progress else {
            return;
        };

        // The name reaches C as a nul-terminated string, which the `&str` is not, so it is copied
        // into a buffer that ends in one. A name too long to fit is reported truncated, since the
        // alternative is reporting no progress at all.
        let mut buffer = [0 as c_char; PATH_SIZE];
        copy_into(&mut buffer, name);

        // SAFETY: `buffer` is nul-terminated and outlives the call, and the pointer is not retained.
        unsafe { on_progress(buffer.as_ptr(), received, total, progress_ctx) };
    };

    match server.receive(extra_arg, &mut report) {
        Ok(None) => IDLE,
        Ok(Some(Outcome::Received { path, cmdline })) => {
            copy_into(&mut out.path, &path);
            copy_into(&mut out.cmdline, &cmdline);
            RECEIVED
        }
        Ok(Some(Outcome::Failed { reason })) => {
            copy_into(&mut out.error, &reason);
            FAILED
        }
        Err(_) => SERVER_LOST,
    }
}

/// Copies text into a fixed C buffer, nul-terminated and truncated to fit.
///
/// The last byte is always the terminator, so what comes out is a string a C caller can read even
/// when what went in was longer than the buffer.
fn copy_into(buffer: &mut [c_char], text: &str) {
    let Some(room) = buffer.len().checked_sub(1) else {
        return;
    };

    // Truncation lands on a character boundary rather than in the middle of one, so what a C caller
    // reads is text even when the tail is missing.
    let mut end = core::cmp::min(room, text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    for (slot, byte) in buffer.iter_mut().zip(text.as_bytes()[..end].iter()) {
        *slot = *byte as c_char;
    }
    buffer[end] = 0;
}
