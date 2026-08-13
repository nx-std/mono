//! Shared pieces of the C boundary.
//!
//! The names below are what the entry points return. They are built from the shared libnx
//! vocabulary rather than assembled here, so the encoding lives in one place for the whole
//! workspace and this module only says which description each failure deserves.

use nx_sf::error::{
    LibnxError,
    ResultCode,
    libnx_error,
};

/// Nothing is mounted under that name.
pub(crate) const NOT_FOUND: ResultCode = libnx_error(LibnxError::NotFound);

/// The registry has no slot left.
///
/// libnx reports a failed `AddDevice` this way, and a caller that branches on the code should not
/// have to learn a new one.
pub(crate) const OUT_OF_MEMORY: ResultCode = libnx_error(LibnxError::OutOfMemory);

/// The bytes are not an image, or could not be read.
pub(crate) const IO_ERROR: ResultCode = libnx_error(LibnxError::IoError);

/// The arguments do not describe anything this crate can act on.
pub(crate) const BAD_INPUT: ResultCode = libnx_error(LibnxError::BadInput);
