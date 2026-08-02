---
name: "rust-errors-reporting"
description: "Declaring error types with thiserror: variant forms, #[source], one enum per function, declared beside it. Load when defining an error type or variant"
type: "core"
scope: "global"
---

# Error Reporting Patterns

**MANDATORY for ALL error handling in the nx-std workspace**

## 1. Derive `thiserror::Error` Fully Qualified

**ALWAYS** write `#[derive(Debug, thiserror::Error)]`.

```rust
// ✅ Good — resolves to the derive macro no matter what `Error` means in this module
#[derive(Debug, thiserror::Error)]
pub enum MapSharedMemoryError { /* ... */ }

// ❌ Bad — the import collides with `svc::Error` re-exported into this module and the
// derive silently resolves to whichever won the name
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error { /* ... */ }
```

## 2. Enum for Several Sources, Struct for One

An enum when the operation has multiple distinct failure modes; a struct when it has exactly one.

```rust
// ✅ Good — enum: the operation fails in two distinguishable ways
#[derive(Debug, thiserror::Error)]
pub enum MapSharedMemoryError {
    #[error("Failed to reserve address space for the shared memory region")]
    ReserveAddressSpace(#[source] virtmem::ReserveError),

    #[error("Failed to map the shared memory region")]
    MapSharedMemory(#[source] svc::Error),
}

// ✅ Good — struct: a single source needs no variant to select between
#[derive(Debug, thiserror::Error)]
#[error("Failed to query the memory info for the region")]
pub struct QueryMemoryError(#[source] pub svc::Error);
```

## 3. Variant Forms: Tuple by Default, Named for Context

**ALWAYS** use tuple form for a single-field variant. Use named fields only when the message carries context
alongside the source.

```rust
// ✅ Good — tuple form; a lone source field gains nothing from a name
#[derive(Debug, thiserror::Error)]
pub enum GetSteadyClockError {
    #[error("Failed to build the CMIF request")]
    BuildRequest(#[source] cmif::RequestLayoutError),

    #[error("Failed to parse the CMIF response")]
    ParseResponse(#[source] cmif::ParseError),
}

// ✅ Good — named fields; the message needs values the source does not carry
#[derive(Debug, thiserror::Error)]
pub enum ReserveRegionError {
    #[error("Invalid page count {count} for the reservation")]
    InvalidPageCount { count: usize, source: PageCountError },

    #[error("Failed to reserve {count} pages at alignment {align:#x}")]
    Reserve { count: usize, align: usize, source: virtmem::ReserveError },
}
```

## 4. Wrap Source Errors in Domain Variants

**ALWAYS** wrap an underlying error in a variant that names what this layer was attempting. Returning a
dependency's error type propagates the failure without the step that caused it.

```rust
// ✅ Good — every failure arrives as a variant naming the step that produced it
pub fn map_shared_memory(
    &self,
    handle: SharedMemoryHandle,
    size: usize,
) -> Result<MappedRegion, MapSharedMemoryError> {
    let region = self
        .virtmem
        .reserve(size)
        .map_err(MapSharedMemoryError::ReserveAddressSpace)?;

    // SAFETY: `region` is a reservation this allocator owns and has not handed out yet,
    // so no other mapping covers the range passed to the SVC.
    unsafe { svc::map_shared_memory(handle.to_raw(), region.addr(), size, Permission::RW) }
        .map_err(MapSharedMemoryError::MapSharedMemory)?;

    // ...
}

// ❌ Bad — the caller receives a bare result code and cannot tell whether the address
// space reservation failed or the kernel rejected the mapping
pub fn map_shared_memory(
    &self,
    handle: SharedMemoryHandle,
    size: usize,
) -> Result<MappedRegion, svc::Error> {
    let region = self.virtmem.reserve(size)?;
    // ...
}
```

## 5. No `#[from]`, No `From` Implementations

**DO NOT** use `#[from]` or write a manual `From` impl unless explicitly required. Explicit `.map_err()` shows
where wrapping happens and prevents an unrelated call from silently converting into a variant that misnames it.

```rust
// ✅ Good — the wrapping site is visible at the call
#[derive(Debug, thiserror::Error)]
pub enum UnmapRegionError {
    #[error("Failed to unmap the shared memory region")]
    UnmapSharedMemory(#[source] svc::Error),
}

// SAFETY: `addr`/`size` describe the exact range this handle was mapped to.
unsafe { svc::unmap_shared_memory(handle.to_raw(), addr, size) }
    .map_err(UnmapRegionError::UnmapSharedMemory)?;

// ❌ Bad — every `?` on an `svc::Error` in the function becomes this variant, so a
// failed `svc::query_memory` before the unmap reports itself as a failed unmap
#[derive(Debug, thiserror::Error)]
pub enum UnmapRegionError {
    #[error("Failed to unmap the shared memory region")]
    UnmapSharedMemory(#[from] svc::Error),
}

unsafe { svc::unmap_shared_memory(handle.to_raw(), addr, size) }?;
```

## 6. Always Mark the Source With `#[source]`

**MANDATORY**: every wrapped error is reachable through `core::error::Error::source()`. A field that is not
annotated ends the chain, so the cause never reaches a log line or a fatal report.

```rust
// ✅ Good — the chain survives to the formatter
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("Failed to request the service handle from `sm`")]
    GetServiceHandle(#[source] svc::Error),
}

// ❌ Bad — the cause is stored but invisible: `.source()` returns `None` and the log
// says only "Failed to request the service handle from `sm`", never the `2001-1023`
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("Failed to request the service handle from `sm`")]
    GetServiceHandle(svc::Error),
}
```

With named fields, `thiserror` treats a field named `source` as the source automatically, so the attribute is
redundant (but harmless) there. **Prefer naming the field `source`.** Any other name (`error`, `inner`,
`cause`) **MUST** carry `#[source]`.

```rust
// ✅ Good — field named `source`; the attribute may be written or omitted
#[derive(Debug, thiserror::Error)]
pub enum MapRegionError {
    #[error("Failed to map the region at {addr:#x}")]
    Map { addr: usize, source: svc::Error },
}

// ❌ Bad — field not named `source` and not annotated, so the chain ends here
#[derive(Debug, thiserror::Error)]
pub enum MapRegionError {
    #[error("Failed to map the region at {addr:#x}")]
    Map { addr: usize, error: svc::Error },
}
```

## 7. Never Embed the Source in the Display String

**MANDATORY**: when a field is the `#[source]`, do **NOT** reference it from `#[error("...")]` via `{0}`,
`{1}`, or `{source}`. Chain formatters (`error_with_causes` and friends)
already append `.source()`, so embedding it prints the same sentence twice in every rendering.
Context fields other than the source are included as normal.

```rust
// ✅ Good — the message describes this level only; the cause arrives via the chain
#[derive(Debug, thiserror::Error)]
#[error("Invalid handle for the {name} session")]
pub struct InvalidHandleError {
    pub name: &'static str,
    #[source]
    pub source: HandleDecodeError,
}

// ❌ Bad — renders as "Invalid handle for the fsp-srv session: handle value is zero |
// Caused by: handle value is zero"
#[derive(Debug, thiserror::Error)]
#[error("Invalid handle for the {name} session: {source}")]
pub struct InvalidHandleError {
    pub name: &'static str,
    #[source]
    pub source: HandleDecodeError,
}
```

## 8. Name the Closure Parameter `err`

**ALWAYS** bind the error as `err` in `.map_err()`, **NEVER** `e`, unless it shadows a binding already in scope.

```rust
// ✅ Good — the closure names the value it binds
virtmem::reserve(page_count, align)
    .map_err(|err| ReserveRegionError::Reserve { count: page_count, align, source: err })?;

// ✅ Good — a tuple variant needs no closure at all
cmif::parse_response::<()>(&buf)
    .map_err(GetSteadyClockError::ParseResponse)?;

// ❌ Bad — a single letter that says nothing about what it holds, in a closure long
// enough that the reader has to scroll back to find out
virtmem::reserve(page_count, align)
    .map_err(|e| ReserveRegionError::Reserve { count: page_count, align, source: e })?;
```

## 9. One Variant Per Error Source

**NEVER** reuse one variant for more than one error source. Each variant describes a single, specific failure
condition.

```rust
// ✅ Good — the variant name identifies which step failed
#[derive(Debug, thiserror::Error)]
pub enum SetLanguageCodeError {
    #[error("Failed to build the CMIF request")]
    BuildRequest(#[source] cmif::RequestLayoutError),

    #[error("Failed to send the request on the `set` session")]
    SendRequest(#[source] svc::Error),

    #[error("Failed to parse the CMIF response")]
    ParseResponse(#[source] cmif::ParseError),
}

// ❌ Bad — every IPC step reports the same variant, so an operator reading "IPC error"
// cannot tell a malformed request buffer from a session the server closed
#[derive(Debug, thiserror::Error)]
pub enum SetLanguageCodeError {
    #[error("IPC error")]
    Ipc(#[source] svc::Error),
}
```

## 10. One Error Enum Per Fallible Function

**Prefer** one error type per fallible function or closely related operation. Reuse a type only when the
sharing functions can return **ALL** of its variants.

```rust
// ✅ Good — dedicated error type per operation
pub fn map_shared_memory(&self) -> Result<MappedRegion, MapSharedMemoryError> { /* ... */ }
pub fn unmap_shared_memory(&self) -> Result<(), UnmapRegionError> { /* ... */ }

// 🔶 Acceptable — shared type where both functions can return both variants
#[derive(Debug, thiserror::Error)]
pub enum SessionIoError {
    #[error("Failed to send the request on the session")]
    SendRequest(#[source] svc::Error),

    #[error("Failed to parse the response from the service")]
    ParseResponse(#[source] cmif::ParseError),
}

// ❌ Bad — variants are half-unreachable from each caller, so every `match` handles
// cases the function it called cannot produce
#[derive(Debug, thiserror::Error)]
pub enum SharedError {
    #[error("Failed to decode the CMIF response header")]
    ResponseHeader(#[source] cmif::ParseError), // only `read_setting` returns this

    #[error("Service is not registered with `sm`")]
    ServiceNotRegistered, // only `open_session` returns this
}
```

## 11. Errors Live With the Function That Returns Them

**MANDATORY**: an error type is declared in the same module as the function that returns it, immediately
**after** that function or `impl` block.

The error and the function are one unit: the variants enumerate exactly the ways that function fails, so a
change to either is a change to both. Splitting them across files means every edit to a failure path is a
two-file edit, and the compiler cannot tell you when they drift apart.

```rust
// ✅ Good — the error follows the function it belongs to
pub fn map_shared_memory(
    &self,
    handle: SharedMemoryHandle,
    size: usize,
) -> Result<MappedRegion, MapSharedMemoryError> {
    // ...
}

/// Errors returned by [`map_shared_memory`].
#[derive(Debug, thiserror::Error)]
pub enum MapSharedMemoryError {
    #[error("Failed to reserve address space for the shared memory region")]
    ReserveAddressSpace(#[source] virtmem::ReserveError),

    #[error("Failed to map the shared memory region")]
    MapSharedMemory(#[source] svc::Error),
}
```

**An `error.rs` module is not a home for a collection of error types.** It is permitted only for an error the
module itself owns — a crate-level `Error` that its public API returns, or a shared result-code wrapper — and
that file holds **one** type.

```rust
// ❌ Bad — error.rs as a bucket. Adding a failure path becomes a two-file edit, and a
// variant that stops being constructed is invisible because nothing nearby shows what
// still returns it.
//
// src/error.rs
pub enum MapSharedMemoryError { /* ... */ }
pub enum UnmapRegionError { /* ... */ }
pub enum QueryMemoryError { /* ... */ }

// 🔶 Acceptable — error.rs holding the one error the crate itself surfaces
//
// src/error.rs
/// Errors surfaced by this crate's public API.
#[derive(Debug, thiserror::Error)]
pub enum Error { /* ... */ }
```

The test is ownership, not the filename: ask which function's failure this type describes. If the answer names
one function, the type belongs next to it. If the answer is "the crate", `error.rs` is where it lives.

## 12. No Unused Error Variants

**MANDATORY**: every variant is constructed somewhere. Remove one that is not, immediately.

```rust
// ❌ Bad — `SessionClosed` is never constructed, so every caller writes a match arm for
// a failure that cannot happen, and the type lies about what the function does
#[derive(Debug, thiserror::Error)]
pub enum GetSteadyClockError {
    #[error("Failed to parse the CMIF response")]
    ParseResponse(#[source] cmif::ParseError),

    #[error("The session was closed by the server")]
    SessionClosed,
}
```

## 13. Error Documentation Template

**MANDATORY**: document each variant as brief description, when it occurs, optional causes, optional
guarantees (cleanup semantics, retry safety).

```rust
#[derive(Debug, thiserror::Error)]
pub enum MapCodeMemoryError {
    /// The requested range is not page aligned
    ///
    /// Occurs when the caller passes an address or size that is not a multiple of the
    /// 4 KiB page size. Detected before any SVC is issued, so no page range was
    /// reserved and no mapping was touched.
    #[error("Range at {0:#x} is not page aligned")]
    UnalignedRange(usize),

    /// Failed to map the code region after the address space was reserved
    ///
    /// The reservation is released before this variant is returned, so no page range is
    /// left claimed by this call and the process address space is unchanged.
    ///
    /// Possible causes:
    /// - The source range is not owned by this process
    /// - The destination range overlaps an existing mapping
    /// - The kernel resource limit has no memory left for the new page tables
    ///
    /// Safe to retry from the beginning: no partial mapping was left behind.
    #[error("Failed to map the code region")]
    MapCodeMemory(#[source] svc::Error),
}
```

## 14. No `BoxError` or `Box<dyn Error>`

**DO NOT** use `BoxError`, `Box<dyn Error>`, or similar type erasure in production code. It discards the type,
so callers cannot match on the failure and the structure of the error is invisible at the signature.

```rust
// ✅ Good — the source type is part of the contract, so a caller can match on it
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("Failed to open a session to service '{name}'")]
    GetServiceHandle { name: ServiceName, source: crate::sm::GetServiceError },
}

// ❌ Bad — a caller that must distinguish "service not registered" from "session limit
// reached", and retry only the first, is left with a string comparison on the message
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("Failed to open a session")]
    GetServiceHandle { name: ServiceName, source: BoxError },
}
```

**Exception:** prototyping and proof-of-concept work may use `BoxError` temporarily. It **MUST** be replaced
with concrete types before merging to main, carry a `TODO` naming the replacement, and be raised in review.

```rust
// 🔶 Acceptable — prototype only, and the TODO is what keeps it from shipping
#[derive(Debug, thiserror::Error)]
pub enum PrototypeError {
    // TODO: replace BoxError with `cmif::ParseError` before production
    #[error("Failed to decode the CMIF response header")]
    Decode(BoxError),
}
```

## Checklist

Before committing error handling code, verify:

- [ ] All error types use `#[derive(Debug, thiserror::Error)]`
- [ ] Enums used for multiple error sources, structs for single sources
- [ ] Tuple form used for single-field variants (unless named fields provide context)
- [ ] All underlying errors are wrapped with domain-specific variants
- [ ] No `#[from]` attributes or `From` implementations (unless explicitly required)
- [ ] All wrapped errors use `#[source]`, or are named `source` in a named-field variant
- [ ] Source fields are NOT referenced in `#[error("...")]` format strings (no `{0}`, `{source}` when `#[source]` is present)
- [ ] Closure parameters in `.map_err()` are named `err` (not `e`)
- [ ] Each error variant is used for a single, distinct error source
- [ ] One error type per function (or shared only when all variants are common)
- [ ] Each error type is declared in the same module as the function that returns it, immediately after it
- [ ] No `error.rs` holds a collection of per-function error types; it holds at most the one error the module
      itself owns
- [ ] No unused error variants exist
- [ ] All error variants are fully documented following the template
- [ ] No `BoxError` or `Box<dyn Error>` in production code

## References

- [rust-errors-handling](rust-errors-handling.md) - Related: Propagating and recovering from the errors declared here
