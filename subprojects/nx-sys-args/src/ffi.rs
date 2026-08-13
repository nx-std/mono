//! C-facing view of the process command line.
//!
//! A C caller wants nul-terminated strings and a pointer array. The strings are
//! already there: [`crate::command_line`] terminated each argument in place
//! while scanning, so all this module adds is the array of pointers into them.
//! Nothing is copied and nothing is allocated.
//!
//! This crate exports no `__nx_*` symbol of its own. Each runtime entry crate
//! publishes what [`system_argv`] returns into its own
//! `__nx_<aspect>__system_argc` and `__nx_<aspect>__system_argv` globals,
//! because those globals belong to the output kind that defines them.

use core::{
    cell::UnsafeCell,
    ffi::c_char,
    ptr,
};

use nx_sys_sync::Once;

use crate::command_line::{
    self,
    MAX_ARGS,
};

/// Initialization guard: ensures the pointer array is filled once.
static C_ARGV_INIT: Once = Once::new();

/// C-style argv: a pointer per argument, then a NULL terminator.
///
/// Every entry starts null, so the terminator after the last argument is
/// already in place and the array reads as empty until it is filled.
static C_ARGV: CArgv = CArgv(UnsafeCell::new([ptr::null_mut(); MAX_ARGS + 1]));

/// C-style `(argc, argv)` for the installed command line, or `None` when no
/// command line was installed.
///
/// The returned `argv` points into the buffer the loader supplied, which lives
/// for the rest of the program, so a C caller may hold it indefinitely.
pub fn system_argv() -> Option<(i32, *mut *mut c_char)> {
    let count = command_line::args().len();
    if count == 0 {
        return None;
    }

    C_ARGV_INIT.call_once(|| {
        for (index, arg) in command_line::args().enumerate() {
            let entry = arg.as_ptr().cast_mut().cast::<c_char>();
            // SAFETY: this runs inside `C_ARGV_INIT`, so it is the only writer,
            // and `index` is below the argument count, which `command_line`
            // bounds by `MAX_ARGS`. No reader can be running: every read of the
            // array is ordered after this `call_once` returns.
            unsafe { (*C_ARGV.0.get())[index] = entry };
        }
    });

    // The count fits an `i32` because `MAX_ARGS` bounds it far below `i32::MAX`.
    let argc = count as i32;
    Some((argc, C_ARGV.0.get() as *mut *mut c_char))
}

/// `Sync` wrapper for a process-lifetime, null-terminated empty argv array.
///
/// Backs each entry crate's `__nx_<aspect>__system_argv` global before a
/// command line is installed. Shared here so the entry crates do not each
/// redefine the same empty-argv backing.
pub struct EmptyArgv([*mut c_char; 1]);

// SAFETY: the array is immutable and holds only a null pointer; sharing it
// across threads cannot observe a data race.
unsafe impl Sync for EmptyArgv {}

impl EmptyArgv {
    /// Pointer to the null-terminated empty argv array.
    pub const fn as_ptr(&self) -> *mut *mut c_char {
        self.0.as_ptr().cast_mut()
    }
}

/// Shared empty argv: a null-terminated array with zero arguments.
pub static EMPTY_ARGV: EmptyArgv = EmptyArgv([ptr::null_mut()]);

/// `Sync` wrapper for the write-once C-style argv array.
struct CArgv(UnsafeCell<[*mut c_char; MAX_ARGS + 1]>);

// SAFETY: the array is written only inside `C_ARGV_INIT`, and every read is
// ordered after that `call_once` has returned. The pointers it holds address a
// buffer that outlives the program and is never written after setup.
unsafe impl Sync for CArgv {}
