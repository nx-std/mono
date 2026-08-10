//! How an operation reports that it did not happen.

/// Errors returned by the device, file, and directory operations.
///
/// Shared by all of them because any operation can fail either way: one the implementation does not
/// offer, or one it attempted and could not complete.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// The operation is not implemented here
    ///
    /// Occurs when a caller reaches for an operation left at its default. Nothing was attempted, so
    /// whatever the call addressed is unchanged and the call is safe to skip.
    #[error("Operation is not supported by the device")]
    Unsupported,

    /// The operation was attempted and failed
    ///
    /// Occurs when the underlying device rejected the request. How much was completed before the
    /// failure is the device's business; callers should treat any position as unknown.
    #[error("Device reported an I/O failure")]
    Io,

    /// The path names nothing
    ///
    /// Occurs when an entry was looked up and does not exist. Distinguished from [`Self::Io`]
    /// because the C standard library reports it as its own error number, and callers routinely
    /// branch on it.
    #[error("No entry exists at that path")]
    NotFound,

    /// The path already names something
    ///
    /// Occurs when creating an entry that is already there. Reported separately for the same reason
    /// as [`Self::NotFound`].
    #[error("An entry already exists at that path")]
    AlreadyExists,

    /// The path is not shaped the way the operation requires
    ///
    /// Occurs when a file operation names a directory, a directory operation names a file, or a
    /// path is malformed. Nothing was changed.
    #[error("The path is not valid for this operation")]
    InvalidPath,

    /// The operation would have had to wait
    ///
    /// Occurs on a device set not to block, such as a non-blocking socket with nothing to receive.
    /// Nothing was transferred and the call is meant to be retried, which is why it is separate
    /// from [`Self::Io`]: a caller that reads it as a failure spins on an error that is not one.
    #[error("Operation would block")]
    WouldBlock,

    /// The operation was cut short before it did anything
    ///
    /// Occurs when a wait was broken off. Nothing was transferred, and the C standard library
    /// reports it as its own error number because callers routinely retry on it.
    #[error("Operation was interrupted")]
    Interrupted,

    /// The peer is gone
    ///
    /// Occurs when the far end of a connection reset it or is no longer reachable. Distinguished
    /// from [`Self::Io`] because it is the ordinary way a connection ends rather than a fault, and
    /// callers branch on it to close their side.
    #[error("Connection was reset by the peer")]
    ConnectionReset,

    /// There is no connection to operate on
    ///
    /// Occurs when an operation that needs an established connection runs on one that has none.
    #[error("Not connected")]
    NotConnected,

    /// The caller may not do this
    ///
    /// Occurs when the device recognised the request and refused it on permission grounds.
    #[error("Permission denied")]
    PermissionDenied,

    /// The operation ran out of time
    ///
    /// Occurs when the device gave the operation a deadline and it expired. How much was completed
    /// is unknown, as with [`Self::Io`].
    #[error("Operation timed out")]
    TimedOut,
}
