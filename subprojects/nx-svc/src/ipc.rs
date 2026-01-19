//! IPC session management for Horizon OS.
//!
//! This module provides safe wrappers around the kernel's IPC session SVCs.
//! A session handle represents the client side of an IPC connection to a
//! service port (named port or anonymous port).
//!
//! ## Horizon OS Terminology
//!
//! - **Session**: A bidirectional IPC channel between a client and server.
//! - **Named Port**: A kernel object registered with a string name (e.g., `"sm:"`)
//!   that clients can connect to by name.
//! - **Client Session**: The handle held by the client side of an IPC session,
//!   used to send requests to the server.

use core::ffi::CStr;

use crate::{
    error::{KernelError as KError, ToRawResultCode},
    raw,
    result::{Error, ResultCode, raw::Result as RawResult},
};

define_waitable_handle_type! {
    /// A handle to a client session kernel object.
    ///
    /// This represents the client side of an IPC session. It is obtained by
    /// connecting to a named port via [`connect_to_named_port`] or other
    /// session-creation SVCs.
    ///
    /// In Horizon OS, IPC sessions are the primary mechanism for inter-process
    /// communication. The client holds a session handle and uses it to send
    /// synchronous requests to the server via [`send_sync_request`].
    pub struct Handle
}

/// Connects to a registered named port and returns a session handle.
pub fn connect_to_named_port(name: &CStr) -> Result<Handle, ConnectError> {
    let mut handle = raw::INVALID_HANDLE;
    // SAFETY: `name` is a valid null-terminated C string (guaranteed by CStr),
    // and `handle` is a valid mutable pointer to receive the output handle.
    let rc = unsafe { raw::connect_to_named_port(&mut handle, name.as_ptr()) };

    RawResult::from_raw(rc).map(Handle(handle), |rc| match rc.description() {
        desc if KError::OutOfRange == desc => ConnectError::OutOfRange,
        desc if KError::NotFound == desc => ConnectError::NotFound,
        desc if KError::OutOfHandles == desc => ConnectError::OutOfHandles,
        desc if KError::OutOfResource == desc => ConnectError::OutOfResource,
        desc if KError::OutOfSessions == desc => ConnectError::OutOfSessions,
        desc if KError::LimitReached == desc => ConnectError::LimitReached,
        _ => ConnectError::Unknown(rc.into()),
    })
}

/// Error returned by [`connect_to_named_port`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Port name exceeds 11 characters or is not null-terminated.
    #[error("Port name out of range")]
    OutOfRange,
    /// No port registered with the given name.
    #[error("Port not found")]
    NotFound,
    /// Process handle table is full.
    #[error("Out of handles")]
    OutOfHandles,
    /// Failed to allocate session object.
    #[error("Out of resource")]
    OutOfResource,
    /// Port's maximum session limit reached.
    #[error("Out of sessions")]
    OutOfSessions,
    /// Process session resource limit exceeded.
    #[error("Limit reached")]
    LimitReached,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for ConnectError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::OutOfRange => KError::OutOfRange.to_rc(),
            Self::NotFound => KError::NotFound.to_rc(),
            Self::OutOfHandles => KError::OutOfHandles.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::OutOfSessions => KError::OutOfSessions.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Connects to an anonymous port handle and returns a session handle.
pub fn connect_to_port(port: Handle) -> Result<Handle, ConnectToPortError> {
    let mut session = raw::INVALID_HANDLE;
    // SAFETY: `session` is a valid mutable pointer to receive the output handle.
    // The kernel validates the port handle and returns an error if invalid.
    let rc = unsafe { raw::connect_to_port(&mut session, port.to_raw()) };

    RawResult::from_raw(rc).map(Handle(session), |rc| match rc.description() {
        desc if KError::OutOfSessions == desc => ConnectToPortError::OutOfSessions,
        desc if KError::OutOfResource == desc => ConnectToPortError::OutOfResource,
        desc if KError::OutOfHandles == desc => ConnectToPortError::OutOfHandles,
        desc if KError::InvalidHandle == desc => ConnectToPortError::InvalidHandle,
        desc if KError::LimitReached == desc => ConnectToPortError::LimitReached,
        _ => ConnectToPortError::Unknown(rc.into()),
    })
}

/// Error returned by [`connect_to_port`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectToPortError {
    /// Port's maximum session limit reached.
    #[error("Out of sessions")]
    OutOfSessions,
    /// Failed to allocate session object.
    #[error("Out of resource")]
    OutOfResource,
    /// Process handle table is full.
    #[error("Out of handles")]
    OutOfHandles,
    /// Invalid port handle.
    #[error("Invalid handle")]
    InvalidHandle,
    /// Process session resource limit exceeded.
    #[error("Limit reached")]
    LimitReached,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for ConnectToPortError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::OutOfSessions => KError::OutOfSessions.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::OutOfHandles => KError::OutOfHandles.to_rc(),
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Sends a synchronous IPC request on a session.
pub fn send_sync_request(handle: Handle) -> Result<(), SendSyncError> {
    // SAFETY: The kernel validates the session handle and returns an error if invalid.
    // The IPC message is read from the thread-local storage buffer.
    let rc = unsafe { raw::send_sync_request(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::TerminationRequested == desc => SendSyncError::TerminationRequested,
        desc if KError::OutOfResource == desc => SendSyncError::OutOfResource,
        desc if KError::InvalidHandle == desc => SendSyncError::InvalidHandle,
        desc if KError::SessionClosed == desc => SendSyncError::SessionClosed,
        _ => SendSyncError::Unknown(rc.into()),
    })
}

/// Error returned by [`send_sync_request`].
#[derive(Debug, thiserror::Error)]
pub enum SendSyncError {
    /// Thread is terminating.
    #[error("Termination requested")]
    TerminationRequested,
    /// Failed to allocate session request.
    #[error("Out of resource")]
    OutOfResource,
    /// Invalid session handle.
    #[error("Invalid handle")]
    InvalidHandle,
    /// Session closed by server.
    #[error("Session closed")]
    SessionClosed,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SendSyncError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::TerminationRequested => KError::TerminationRequested.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::SessionClosed => KError::SessionClosed.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Sends a light synchronous IPC request on a session.
///
/// Light IPC uses registers instead of the TLS buffer for small messages.
pub fn send_sync_request_light(handle: Handle) -> Result<(), SendSyncLightError> {
    // SAFETY: The kernel validates the session handle and returns an error if invalid.
    // Light IPC uses registers for message passing, no memory pointers involved.
    let rc = unsafe { raw::send_sync_request_light(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::TerminationRequested == desc => SendSyncLightError::TerminationRequested,
        desc if KError::InvalidHandle == desc => SendSyncLightError::InvalidHandle,
        desc if KError::Cancelled == desc => SendSyncLightError::Cancelled,
        desc if KError::SessionClosed == desc => SendSyncLightError::SessionClosed,
        desc if KError::InvalidState == desc => SendSyncLightError::InvalidState,
        _ => SendSyncLightError::Unknown(rc.into()),
    })
}

/// Error returned by [`send_sync_request_light`].
#[derive(Debug, thiserror::Error)]
pub enum SendSyncLightError {
    /// Thread is terminating.
    #[error("Termination requested")]
    TerminationRequested,
    /// Invalid session handle.
    #[error("Invalid handle")]
    InvalidHandle,
    /// Wait was cancelled.
    #[error("Cancelled")]
    Cancelled,
    /// Session closed by server.
    #[error("Session closed")]
    SessionClosed,
    /// Invalid state for operation.
    #[error("Invalid state")]
    InvalidState,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SendSyncLightError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::TerminationRequested => KError::TerminationRequested.to_rc(),
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Cancelled => KError::Cancelled.to_rc(),
            Self::SessionClosed => KError::SessionClosed.to_rc(),
            Self::InvalidState => KError::InvalidState.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Sends a synchronous IPC request using a user-provided buffer.
///
/// The buffer must be page-aligned and its size must be page-aligned and non-zero.
pub fn send_sync_request_with_user_buffer(
    buffer: &mut [u8],
    handle: Handle,
) -> Result<(), SendSyncWithBufferError> {
    // SAFETY: The buffer pointer and length are derived from a valid slice.
    // The kernel validates alignment, size, and memory region, returning
    // appropriate errors if requirements are not met.
    let rc = unsafe {
        raw::send_sync_request_with_user_buffer(
            buffer.as_mut_ptr().cast(),
            buffer.len() as u64,
            handle.to_raw(),
        )
    };

    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::TerminationRequested == desc => {
            SendSyncWithBufferError::TerminationRequested
        }
        desc if KError::InvalidSize == desc => SendSyncWithBufferError::InvalidSize,
        desc if KError::InvalidAddress == desc => SendSyncWithBufferError::InvalidAddress,
        desc if KError::OutOfResource == desc => SendSyncWithBufferError::OutOfResource,
        desc if KError::InvalidCurrentMemory == desc => {
            SendSyncWithBufferError::InvalidCurrentMemory
        }
        desc if KError::InvalidHandle == desc => SendSyncWithBufferError::InvalidHandle,
        desc if KError::InvalidCombination == desc => SendSyncWithBufferError::InvalidCombination,
        desc if KError::SessionClosed == desc => SendSyncWithBufferError::SessionClosed,
        desc if KError::MessageTooLarge == desc => SendSyncWithBufferError::MessageTooLarge,
        _ => SendSyncWithBufferError::Unknown(rc.into()),
    })
}

/// Error returned by [`send_sync_request_with_user_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum SendSyncWithBufferError {
    /// Thread is terminating.
    #[error("Termination requested")]
    TerminationRequested,
    /// Buffer size is zero or not page-aligned.
    #[error("Invalid size")]
    InvalidSize,
    /// Buffer address is not page-aligned.
    #[error("Invalid address")]
    InvalidAddress,
    /// Failed to lock the user buffer.
    #[error("Out of resource")]
    OutOfResource,
    /// Buffer overflow or invalid memory region.
    #[error("Invalid current memory")]
    InvalidCurrentMemory,
    /// Invalid session handle.
    #[error("Invalid handle")]
    InvalidHandle,
    /// Invalid message descriptor combination.
    #[error("Invalid combination")]
    InvalidCombination,
    /// Session closed by server.
    #[error("Session closed")]
    SessionClosed,
    /// Message exceeds maximum allowed size.
    #[error("Message too large")]
    MessageTooLarge,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SendSyncWithBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::TerminationRequested => KError::TerminationRequested.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidCombination => KError::InvalidCombination.to_rc(),
            Self::SessionClosed => KError::SessionClosed.to_rc(),
            Self::MessageTooLarge => KError::MessageTooLarge.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

define_waitable_handle_type! {
    /// A handle to an IPC completion event.
    ///
    /// This handle is returned by [`send_async_request_with_user_buffer`] and can
    /// be waited on to determine when the asynchronous request completes.
    pub struct EventHandle
}

/// Sends an asynchronous IPC request using a user-provided buffer.
///
/// The buffer must be page-aligned and its size must be page-aligned and non-zero.
/// Returns an event handle that signals when the request completes.
pub fn send_async_request_with_user_buffer(
    buffer: &mut [u8],
    session: Handle,
) -> Result<EventHandle, SendAsyncWithBufferError> {
    let mut event_handle = raw::INVALID_HANDLE;
    // SAFETY: `event_handle` is a valid mutable pointer to receive the output handle.
    // The buffer pointer and length are derived from a valid slice. The kernel
    // validates alignment, size, and memory region, returning appropriate errors.
    let rc = unsafe {
        raw::send_async_request_with_user_buffer(
            &mut event_handle,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u64,
            session.to_raw(),
        )
    };

    RawResult::from_raw(rc).map(EventHandle(event_handle), |rc| match rc.description() {
        desc if KError::TerminationRequested == desc => {
            SendAsyncWithBufferError::TerminationRequested
        }
        desc if KError::InvalidSize == desc => SendAsyncWithBufferError::InvalidSize,
        desc if KError::InvalidAddress == desc => SendAsyncWithBufferError::InvalidAddress,
        desc if KError::OutOfResource == desc => SendAsyncWithBufferError::OutOfResource,
        desc if KError::InvalidCurrentMemory == desc => {
            SendAsyncWithBufferError::InvalidCurrentMemory
        }
        desc if KError::InvalidHandle == desc => SendAsyncWithBufferError::InvalidHandle,
        desc if KError::InvalidCombination == desc => SendAsyncWithBufferError::InvalidCombination,
        desc if KError::NotFound == desc => SendAsyncWithBufferError::NotFound,
        desc if KError::SessionClosed == desc => SendAsyncWithBufferError::SessionClosed,
        desc if KError::InvalidState == desc => SendAsyncWithBufferError::InvalidState,
        desc if KError::LimitReached == desc => SendAsyncWithBufferError::LimitReached,
        desc if KError::ReceiveListBroken == desc => SendAsyncWithBufferError::ReceiveListBroken,
        desc if KError::MessageTooLarge == desc => SendAsyncWithBufferError::MessageTooLarge,
        _ => SendAsyncWithBufferError::Unknown(rc.into()),
    })
}

/// Error returned by [`send_async_request_with_user_buffer`].
#[derive(Debug, thiserror::Error)]
pub enum SendAsyncWithBufferError {
    /// Thread is terminating.
    #[error("Termination requested")]
    TerminationRequested,
    /// Buffer size is zero or not page-aligned.
    #[error("Invalid size")]
    InvalidSize,
    /// Buffer address is not page-aligned.
    #[error("Invalid address")]
    InvalidAddress,
    /// Failed to allocate event or lock buffer.
    #[error("Out of resource")]
    OutOfResource,
    /// Buffer overflow or invalid memory region.
    #[error("Invalid current memory")]
    InvalidCurrentMemory,
    /// Invalid session handle.
    #[error("Invalid handle")]
    InvalidHandle,
    /// Invalid message descriptor combination.
    #[error("Invalid combination")]
    InvalidCombination,
    /// No pending request found.
    #[error("Not found")]
    NotFound,
    /// Session closed by server.
    #[error("Session closed")]
    SessionClosed,
    /// Invalid request state.
    #[error("Invalid state")]
    InvalidState,
    /// Event resource limit exceeded.
    #[error("Limit reached")]
    LimitReached,
    /// Receive list was corrupted.
    #[error("Receive list broken")]
    ReceiveListBroken,
    /// Message exceeds maximum allowed size.
    #[error("Message too large")]
    MessageTooLarge,
    /// Unexpected kernel error.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SendAsyncWithBufferError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::TerminationRequested => KError::TerminationRequested.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidCombination => KError::InvalidCombination.to_rc(),
            Self::NotFound => KError::NotFound.to_rc(),
            Self::SessionClosed => KError::SessionClosed.to_rc(),
            Self::InvalidState => KError::InvalidState.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::ReceiveListBroken => KError::ReceiveListBroken.to_rc(),
            Self::MessageTooLarge => KError::MessageTooLarge.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Closes a session handle, decrementing the kernel reference count.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    // SAFETY: The kernel validates the handle and returns an error if invalid.
    // Closing an already-closed handle is safe (returns InvalidHandle error).
    let rc = unsafe { raw::close_handle(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(rc.into()),
    })
}

/// Error returned by [`close_handle`].
#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    /// The supplied handle is not a valid session handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CloseHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}
