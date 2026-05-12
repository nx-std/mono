//! Wire-layout types for async IPC sub-objects.

use static_assertions::const_assert_eq;

/// Error context returned by `GetErrorContext` commands (`[4.0.0+]`).
///
/// Opaque blob carrying service-specific error details. The `kind` field
/// selects the interpretation; callers match on [`ErrorContextKind`] to
/// decide how to read `data`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ErrorContext {
    /// Discriminant selecting the data interpretation.
    pub kind: u8,
    _pad: [u8; 7],
    /// Opaque payload whose format depends on `kind`.
    pub data: [u8; 0x1F4],
    /// HOS result code associated with this error context.
    pub result: u32,
}

const_assert_eq!(core::mem::size_of::<ErrorContext>(), 0x200);

/// Known error-context type discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorContextKind {
    None = 0,
    Http = 1,
    FileSystem = 2,
    WebMediaPlayer = 3,
    LocalContentShare = 4,
}
