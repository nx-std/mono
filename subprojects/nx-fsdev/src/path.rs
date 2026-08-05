//! Building the path buffer every `fsp-srv` command takes.
//!
//! A path command carries a fixed 0x301-byte buffer rather than a pointer and a length, so a path
//! is copied into one before anything is dispatched. Building it is also where a relative path
//! becomes an absolute one, because the working directory it is relative to belongs to the device
//! and nothing below this crate knows about it.
//!
//! What arrives here has already had its `"name:"` prefix removed by the descriptor table, so a
//! second colon is a caller writing a path this device cannot serve rather than a device name. It
//! is rejected, matching what libnx does with the same input.

use core::ffi::CStr;

use nx_service_fs::FS_MAX_PATH;
use nx_sys_fd::device::DeviceError;

/// A path in the form a `fsp-srv` command takes: nul-terminated, in a fixed buffer.
pub(crate) struct FsPath {
    buf: [u8; FS_MAX_PATH],
}

impl FsPath {
    /// Builds the path a command takes by resolving `path` against `cwd`.
    ///
    /// A path starting with `/` is absolute and used as it stands; anything else is joined onto
    /// `cwd`. `cwd` is expected to be absolute and without a trailing slash, which is how a device
    /// keeps it.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveError::TooLong`] when the result does not fit the buffer with room for its
    /// nul terminator, and [`ResolveError::InvalidPath`] when `path` carries a colon.
    pub(crate) fn create(cwd: &[u8], path: &CStr) -> Result<Self, ResolveError> {
        let path = path.to_bytes();
        if path.contains(&b':') {
            return Err(ResolveError::InvalidPath);
        }

        let mut buf = [0u8; FS_MAX_PATH];
        let mut len = 0;

        let mut push = |bytes: &[u8]| -> Result<(), ResolveError> {
            // One byte is held back for the terminator, which the zeroed buffer already provides.
            if len + bytes.len() >= FS_MAX_PATH {
                return Err(ResolveError::TooLong);
            }
            buf[len..len + bytes.len()].copy_from_slice(bytes);
            len += bytes.len();
            Ok(())
        };

        if !path.starts_with(b"/") {
            push(cwd)?;
            push(b"/")?;
        }
        push(path)?;

        Ok(Self { buf })
    }

    /// Returns the buffer, for a command to send.
    pub(crate) fn as_buf(&self) -> &[u8; FS_MAX_PATH] {
        &self.buf
    }

    /// Returns the path, without its nul terminator.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        let end = self
            .buf
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.buf.len());
        &self.buf[..end]
    }
}

/// Errors returned by [`FsPath::create`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum ResolveError {
    /// The resolved path does not fit the buffer a command carries
    ///
    /// Occurs when the working directory and the path together exceed what `fsp-srv` accepts.
    /// Nothing was dispatched: rejecting here is what keeps a truncated path, which names a
    /// different entry, from reaching the server.
    #[error("The resolved path does not fit the command buffer")]
    TooLong,

    /// The path carries a colon
    ///
    /// Occurs when a caller writes a device name where a path belongs. The descriptor table
    /// removed the prefix already, so whatever is left is not a name this device can act on.
    #[error("The path is not one this device can serve")]
    InvalidPath,
}

/// Both rejections are the path being wrong for the operation, which is the one thing a device can
/// say about a path it never dispatched. libnx distinguishes the over-long case as
/// `ENAMETOOLONG`; there is no [`DeviceError`] carrying that, so the length shows up as an invalid
/// path instead.
impl From<ResolveError> for DeviceError {
    fn from(err: ResolveError) -> Self {
        match err {
            ResolveError::TooLong | ResolveError::InvalidPath => Self::InvalidPath,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Working directory the relative fixtures are resolved against.
    const CWD: &[u8] = b"/nx-tests-fs";

    #[test]
    fn absolute_path_is_used_as_it_stands() {
        //* Given
        let path = c"/switch/app.nro";

        //* When
        let resolved = FsPath::create(CWD, path).expect("an absolute path should resolve");

        //* Then
        assert_eq!(
            resolved.as_bytes(),
            b"/switch/app.nro",
            "an absolute path must not be joined onto the working directory"
        );
    }

    #[test]
    fn relative_path_is_joined_onto_the_working_directory() {
        //* Given
        let path = c"save.dat";

        //* When
        let resolved = FsPath::create(CWD, path).expect("a relative path should resolve");

        //* Then
        assert_eq!(
            resolved.as_bytes(),
            b"/nx-tests-fs/save.dat",
            "a relative path must be joined onto the working directory with one separator"
        );
    }

    #[test]
    fn resolved_path_is_nul_terminated_in_the_command_buffer() {
        //* Given
        let path = c"save.dat";

        //* When
        let resolved = FsPath::create(CWD, path).expect("a relative path should resolve");

        //* Then
        let buf = resolved.as_buf();
        assert_eq!(
            buf[resolved.as_bytes().len()],
            0,
            "the byte past the path must terminate it for the command"
        );
    }

    #[test]
    fn path_carrying_a_colon_is_rejected() {
        //* Given
        // The descriptor table strips the device prefix, so a colon that survives is a caller
        // writing a path this device cannot serve.
        let path = c"sdmc:/save.dat";

        //* When
        let result = FsPath::create(CWD, path);

        //* Then
        assert!(
            matches!(result, Err(ResolveError::InvalidPath)),
            "a path carrying a colon must be refused as invalid"
        );
    }

    #[test]
    fn path_that_overruns_the_command_buffer_is_rejected() {
        //* Given
        // One byte longer than the buffer can hold with its terminator.
        let long = alloc::vec![b'a'; FS_MAX_PATH];
        let path = alloc::ffi::CString::new(long).expect("the fixture holds no interior nul");

        //* When
        let result = FsPath::create(b"/", &path);

        //* Then
        assert!(
            matches!(result, Err(ResolveError::TooLong)),
            "a path that cannot fit the buffer must be refused rather than truncated"
        );
    }

    #[test]
    fn working_directory_counts_towards_the_buffer_limit() {
        //* Given
        // The name fits on its own; joined onto the working directory it does not.
        let cwd = alloc::vec![b'a'; FS_MAX_PATH - 4];
        let path = c"file";

        //* When
        let result = FsPath::create(&cwd, path);

        //* Then
        assert!(
            matches!(result, Err(ResolveError::TooLong)),
            "the join must be measured, not just the path handed in"
        );
    }
}
