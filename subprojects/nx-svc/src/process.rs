//! Process handle types.

use crate::raw;

define_handle_type! {
    /// A handle to a process kernel object.
    pub struct Handle
}

impl Handle {
    /// Returns a pseudo-handle for the current process.
    ///
    /// This handle can be used in syscalls that accept the current process,
    /// such as when opening applet proxy sessions.
    pub const fn current_process() -> Self {
        Self(raw::CUR_PROCESS_HANDLE)
    }

    /// Returns `true` if this is the current process pseudo-handle.
    pub const fn is_current_process(&self) -> bool {
        self.0 == raw::CUR_PROCESS_HANDLE
    }
}
