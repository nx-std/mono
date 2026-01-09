---
name: "error-reporting"
description: "Error handling patterns using thiserror. Use when defining error types, writing Result-returning functions, or reviewing error handling code. Covers thiserror derive, #[source] attribute, map_err patterns, and error type design."
---

# Error Reporting Patterns

**MANDATORY for ALL error handling in this project**

## Table of Contents

1. [Purpose](#purpose)
2. [Core Principles](#core-principles)
   - [1. Use `thiserror::Error` Derive Macro](#1-use-thiserrorerror-derive-macro)
   - [2. Choose Error Type Structure Based on Error Sources](#2-choose-error-type-structure-based-on-error-sources)
   - [3. Error Variant Forms](#3-error-variant-forms)
   - [4. Wrap Source Errors to Provide Context](#4-wrap-source-errors-to-provide-context)
   - [5. Avoid `#[from]` Attribute](#5-avoid-from-attribute)
   - [6. Always Use `#[source]` Attribute](#6-always-use-source-attribute)
   - [7. Closure Parameter Naming Convention](#7-closure-parameter-naming-convention)
   - [8. One Variant Per Error Source](#8-one-variant-per-error-source)
   - [9. One Error Enum Per Fallible Function](#9-one-error-enum-per-fallible-function)
   - [10. No Unused Error Variants](#10-no-unused-error-variants)
   - [11. Error Documentation](#11-error-documentation)
   - [12. Unknown Error Variants](#12-unknown-error-variants)
3. [Complete Example](#complete-example)
4. [Checklist](#checklist)

## Purpose

This document establishes consistent error reporting patterns across the codebase. These patterns ensure:

- **Explicit error propagation** - Clear visibility of where errors originate and are transformed
- **Rich error context** - Detailed information for debugging
- **Type-safe error handling** - Leverage Rust's type system to prevent error handling mistakes
- **Error chain preservation** - Maintain full error causality via `std::error::Error::source()`

## Core Principles

### 1. Use `thiserror::Error` Derive Macro

**ALWAYS** use the fully qualified form `#[derive(Debug, thiserror::Error)]` to avoid name clashes with user-defined `Error` types.

```rust
// CORRECT - Fully qualified form
#[derive(Debug, thiserror::Error)]
pub enum CreateThreadError {
    // ...
}

// WRONG - May clash with custom Error types
use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    // ...
}
```

**Note:** `thiserror` supports `no_std` with `default-features = false`.

### 2. Choose Error Type Structure Based on Error Sources

#### Enums: Multiple Error Sources

Use **enums** when an operation has multiple distinct error sources or failure modes.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
```

#### Structs: Single Error Source

Use **structs** when wrapping a single underlying error type.

```rust
#[derive(Debug, thiserror::Error)]
#[error("failed to query metadata")]
pub struct QueryError(#[source] pub ipc::SendSyncError);
```

### 3. Error Variant Forms

#### Tuple Form: Single Field (Default)

**ALWAYS** use tuple form when an error variant has a single field.

```rust
// CORRECT - Tuple form for single source error
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing move handle in response")]
    MissingHandle,
}
```

#### Named Fields: Multiple Fields or Context

Use named fields when providing additional context alongside the source error.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("failed to connect to service '{name}'")]
    Connect {
        name: &'static str,
        #[source]
        source: ipc::SendSyncError,
    },
}
```

### 4. Wrap Source Errors to Provide Context

**ALWAYS** wrap underlying error types in domain-specific error variants. This provides:
- Clear error origin
- Domain-specific error messages
- Ability to add context
- Type-safe error handling

```rust
// CORRECT - Wrapping with context
pub fn clone_current_object(session: SessionHandle) -> Result<SessionHandle, CloneObjectError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT, 0) };

    ipc::send_sync_request(session).map_err(CloneObjectError::SendRequest)?;

    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectError::ParseResponse)?;

    if resp.move_handles.is_empty() {
        return Err(CloneObjectError::MissingHandle);
    }

    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

// WRONG - Propagating generic errors without context
pub fn clone_current_object(session: SessionHandle) -> Result<SessionHandle, ipc::SendSyncError> {
    // Lost context about what operation failed
}
```

### 5. Avoid `#[from]` Attribute

**DO NOT** use `#[from]` attribute or manual `From` implementations unless explicitly required.

**Why?** Explicit `.map_err()` calls:
- Show exactly where error wrapping happens
- Make error flow more visible
- Prevent accidental implicit conversions
- Aid debugging and code comprehension

```rust
// CORRECT - Explicit error mapping
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("IPC operation failed")]
    Ipc(#[source] ipc::SendSyncError),  // No #[from]
}

pub fn my_operation(session: SessionHandle) -> Result<(), MyError> {
    ipc::send_sync_request(session)
        .map_err(MyError::Ipc)?;  // Explicit mapping
    Ok(())
}

// WRONG - Using #[from]
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("IPC operation failed")]
    Ipc(#[from] ipc::SendSyncError),  // Implicit conversion
}

pub fn my_operation(session: SessionHandle) -> Result<(), MyError> {
    ipc::send_sync_request(session)?;  // Where did wrapping happen?
    Ok(())
}
```

### 6. Always Use `#[source]` Attribute

**MANDATORY**: Use `#[source]` attribute on all wrapped error types to preserve the error chain.

```rust
// CORRECT - Using #[source]
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),  // #[source] preserves chain
}

// WRONG - Missing #[source]
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    #[error("failed to send IPC request")]
    SendRequest(ipc::SendSyncError),  // Error chain broken!
}
```

#### Special Case: Named Field `source`

When using named fields, if the field is named `source`, the `#[source]` attribute is redundant (but harmless). If the field has a different name, you **MUST** annotate it with `#[source]`.

```rust
// CORRECT - Field named 'source', #[source] is redundant
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("operation failed")]
    Failed {
        context: &'static str,
        source: ipc::SendSyncError,  // Automatically treated as source
    },
}

// CORRECT - Field named 'inner', #[source] is required
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("operation failed")]
    Failed {
        context: &'static str,
        #[source]  // Required because field is not named 'source'
        inner: ipc::SendSyncError,
    },
}
```

### 7. Closure Parameter Naming Convention

**ALWAYS** name the closure parameter in `.map_err()` as `err`, **NEVER** shortened to `e`.

```rust
// CORRECT - Full 'err' parameter name
ipc::send_sync_request(session)
    .map_err(|err| MyError::Ipc(err))?;

// CORRECT - Simple case (can omit closure when variant is tuple)
ipc::send_sync_request(session)
    .map_err(MyError::Ipc)?;

// WRONG - Shortened parameter name
ipc::send_sync_request(session)
    .map_err(|e| MyError::Ipc(e))?;
```

### 8. One Variant Per Error Source

**NEVER** reuse the same error variant for multiple error sources. Each variant should describe a single, specific error condition.

```rust
// CORRECT - Distinct variants for different error sources
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("failed to begin transaction")]
    BeginTransaction(#[source] ipc::SendSyncError),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] ipc::SendSyncError),
    #[error("failed to rollback transaction")]
    RollbackTransaction(#[source] ipc::SendSyncError),
}

// WRONG - Reusing single variant for multiple sources
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("IPC error")]
    IpcError(#[source] ipc::SendSyncError),  // Used everywhere - no context!
}
```

### 9. One Error Enum Per Fallible Function

**Prefer** one error type per fallible function or closely related operation. Only reuse error types when functions share **ALL** error variants.

```rust
// CORRECT - Dedicated error type per operation
pub fn create(/* ... */) -> Result<Handle, CreateThreadError> { /* ... */ }
pub fn start(handle: Handle) -> Result<(), StartThreadError> { /* ... */ }
pub fn exit() -> Result<(), ExitThreadError> { /* ... */ }

// ACCEPTABLE - Shared error type when ALL variants are common
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// Both functions use exactly these two variants
pub fn dispatch_a(/* ... */) -> Result<(), DispatchError> { /* ... */ }
pub fn dispatch_b(/* ... */) -> Result<(), DispatchError> { /* ... */ }
```

### 10. No Unused Error Variants

**MANDATORY**: Every error variant **MUST** be actually used in code. Remove unused variants immediately.

### 11. Error Documentation

Document each error variant with a doc comment explaining when the error occurs.

```rust
#[derive(Debug, thiserror::Error)]
pub enum CreateThreadError {
    #[error("out of memory")]
    OutOfMemory,
    /// The kernel ran out of generic thread-related resources - maps to
    /// `KernelError::OutOfResource` (raw code `0x267`).
    #[error("out of generic thread resources")]
    OutOfResource,
    /// The per-process thread quota has been exhausted -
    /// `KernelError::LimitReached` (raw code `0x284`).
    #[error("thread limit reached for process")]
    LimitReached,
    /// The process handle table contains no free slots -
    /// `KernelError::OutOfHandles` (raw code `0x269`).
    #[error("handle table full")]
    OutOfHandles,
    /// The supplied priority is outside `0..=0x3F` or not permitted by the
    /// process - `KernelError::InvalidPriority` (raw code `0x270`).
    #[error("invalid priority")]
    InvalidPriority,
    /// The requested CPU core is invalid or outside the process affinity mask -
    /// `KernelError::InvalidCoreId` (raw code `0x271`).
    #[error("invalid core id")]
    InvalidCoreId,
    /// Any unforeseen kernel error.
    #[error("unknown error: {0}")]
    Unknown(Error),
}
```

### 12. Unknown Error Variants

For kernel/SVC operations, include an `Unknown` variant to capture unforeseen error codes. This allows callers to inspect the raw result code if needed.

```rust
#[derive(Debug, thiserror::Error)]
pub enum StartThreadError {
    /// The supplied handle is not a valid thread handle.
    #[error("invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("unknown error: {0}")]
    Unknown(Error),
}
```

Implement `ToRawResultCode` to convert error types back to raw result codes when needed:

```rust
impl ToRawResultCode for StartThreadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}
```

## Complete Example

```rust
use crate::ipc::{self, SessionHandle};
use crate::cmif;

/// Error returned by [`clone_current_object`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    /// Failed to send the IPC request.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the service response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Clones the current session object via control request 2.
pub fn clone_current_object(session: SessionHandle) -> Result<SessionHandle, CloneObjectError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to valid IPC buffer with sufficient space.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT, 0) };

    ipc::send_sync_request(session).map_err(CloneObjectError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectError::ParseResponse)?;

    if resp.move_handles.is_empty() {
        return Err(CloneObjectError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}
```

## Checklist

Before committing error handling code, verify:

- [ ] All error types use `#[derive(Debug, thiserror::Error)]`
- [ ] Enums used for multiple error sources, structs for single sources
- [ ] Tuple form used for single-field variants
- [ ] All underlying errors are wrapped with domain-specific variants
- [ ] No `#[from]` attributes (unless explicitly required)
- [ ] All wrapped errors use `#[source]` attribute
- [ ] Closure parameters in `.map_err()` are named `err` (not `e`)
- [ ] Each error variant is used for a single, distinct error source
- [ ] One error type per function (or shared only when all variants are common)
- [ ] No unused error variants exist
- [ ] All error variants are documented
- [ ] Kernel errors have an `Unknown` variant for unforeseen codes
