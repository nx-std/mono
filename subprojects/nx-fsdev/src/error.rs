//! Turning a failed `fsp-srv` command into what a device reports.
//!
//! A command fails with a result code the server chose, and a device reports a
//! [`DeviceError`] the descriptor table turns into an error number. Only a handful of codes have
//! an error number of their own; the rest are an I/O failure as far as a C caller can tell.
//!
//! The mapping is libnx's, kept deliberately: a homebrew binary that branches on `errno` today
//! must branch the same way after the switch. libnx also maps its name-too-long code to
//! `ENAMETOOLONG`, which has no [`DeviceError`] to carry it and therefore arrives as an I/O
//! failure here. A path that long is rejected by [`crate::path`] before a command is ever built,
//! so the server only produces that code for a path this crate did not construct.

use core::sync::atomic::{
    AtomicU32,
    Ordering,
};

use nx_sf::{
    error::ToResultCode as _,
    service::DispatchError,
};
use nx_sys_fd::device::DeviceError;

/// The path names nothing.
const PATH_NOT_FOUND: u32 = 0x202;

/// The path already names something.
const PATH_ALREADY_EXISTS: u32 = 0x402;

/// The path is not one the filesystem accepts.
const INVALID_PATH: u32 = 0x2EE202;

/// Result code of the most recent failed command.
///
/// libnx keeps this so that a caller who has already lost the code to an error number can ask for
/// it again, which is what `fsdevGetLastResult` returns. It is process-wide rather than
/// per-thread, matching what it replaces.
static LAST_RESULT: AtomicU32 = AtomicU32::new(0);

/// Records that `err` happened and returns what the device reports for it.
///
/// Recording and mapping are one call because every failure has to do both, and a failure that
/// reached the server without updating the cache would leave `fsdevGetLastResult` describing an
/// older one.
pub(crate) fn report(err: DispatchError) -> DeviceError {
    let code = err.to_rc();
    LAST_RESULT.store(code, Ordering::Relaxed);

    device_error_for(code)
}

/// Returns what the device reports for the result code `code`.
fn device_error_for(code: u32) -> DeviceError {
    match code {
        PATH_NOT_FOUND => DeviceError::NotFound,
        PATH_ALREADY_EXISTS => DeviceError::AlreadyExists,
        INVALID_PATH => DeviceError::InvalidPath,
        _ => DeviceError::Io,
    }
}

/// Returns the result code of the most recent failed command, or zero when none has failed.
pub(crate) fn last_result() -> u32 {
    LAST_RESULT.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_not_found_maps_to_the_missing_entry_error() {
        //* Given / When
        let err = device_error_for(PATH_NOT_FOUND);

        //* Then
        assert!(
            matches!(err, DeviceError::NotFound),
            "the C caller branches on ENOENT, which only this variant produces"
        );
    }

    #[test]
    fn path_already_exists_maps_to_the_occupied_entry_error() {
        //* Given / When
        let err = device_error_for(PATH_ALREADY_EXISTS);

        //* Then
        assert!(
            matches!(err, DeviceError::AlreadyExists),
            "the C caller branches on EEXIST, which only this variant produces"
        );
    }

    #[test]
    fn invalid_path_maps_to_the_invalid_path_error() {
        //* Given / When
        let err = device_error_for(INVALID_PATH);

        //* Then
        assert!(
            matches!(err, DeviceError::InvalidPath),
            "EINVAL is its own answer"
        );
    }

    #[test]
    fn unknown_code_maps_to_a_plain_io_failure() {
        //* Given
        // A code the mapping does not name, such as the directory-not-empty the server reports.
        let code = 0x1002;

        //* When
        let err = device_error_for(code);

        //* Then
        assert!(
            matches!(err, DeviceError::Io),
            "a code without an error number of its own must arrive as a plain I/O failure"
        );
    }
}
