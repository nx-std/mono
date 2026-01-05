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

define_handle_type! {
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

/// Sends a synchronous IPC request on a session.
pub fn send_sync_request(handle: Handle) -> Result<(), SendSyncError> {
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

/// Closes a session handle, decrementing the kernel reference count.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
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
