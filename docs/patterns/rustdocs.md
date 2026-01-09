---
name: "rustdocs"
description: "Documentation standards for Rust code. Use when writing doc comments, documenting unsafe functions, adding Safety/Panics sections, or reviewing documentation. Covers succinct style, Safety sections, SAFETY comments, FFI documentation, and Panics sections."
---

# Rustdoc Patterns

**MANDATORY for ALL documentation in this project**

## Table of Contents

1. [Purpose](#purpose)
2. [Core Principles](#core-principles)
   - [1. Succinct Documentation Philosophy](#1-succinct-documentation-philosophy)
   - [2. Document Safety-Critical Information](#2-document-safety-critical-information)
   - [3. Add Value Beyond Code](#3-add-value-beyond-code)
3. [Function Documentation Requirements](#function-documentation-requirements)
   - [1. Brief Description Required](#1-brief-description-required)
   - [2. Document Key Behaviors](#2-document-key-behaviors)
   - [3. No Returns Section](#3-no-returns-section)
   - [4. No Examples Section](#4-no-examples-section)
   - [5. No Arguments Section](#5-no-arguments-section)
4. [Safety Documentation](#safety-documentation)
   - [1. Safety Section in Unsafe Functions](#1-safety-section-in-unsafe-functions)
   - [2. Safety Section in _unchecked Functions](#2-safety-section-in-_unchecked-functions)
   - [3. SAFETY Comments at Callsites](#3-safety-comments-at-callsites)
5. [Panics Documentation](#panics-documentation)
6. [FFI Documentation](#ffi-documentation)
   - [1. FFI Function Documentation](#1-ffi-function-documentation)
   - [2. C Type Documentation](#2-c-type-documentation)
7. [Complete Examples](#complete-examples)
8. [Checklist](#checklist)

## Purpose

This document establishes consistent, succinct documentation standards across the codebase. These patterns ensure:

- **Rustdoc generation** - All public APIs documented for `cargo doc`
- **Succinct clarity** - Concise documentation that adds value without verbosity
- **Safety guarantees** - Explicit safety documentation for all unsafe operations
- **FFI clarity** - Clear documentation of C-compatible interfaces

## Core Principles

### 1. Succinct Documentation Philosophy

**BE SUCCINCT**: Write concise documentation that adds value. Code should be self-documenting, but rustdocs need the obvious info for `cargo doc` generation. Keep it brief and clear.

```rust
// WRONG - Overly verbose
/// This function creates a new thread by invoking the svcCreateThread supervisor
/// call. It will allocate the necessary kernel resources and return a handle
/// to the newly created thread, or an error if the operation fails due to
/// resource exhaustion or invalid parameters.
pub fn create_thread(/* ... */) -> Result<ThreadHandle, CreateThreadError> {
    // ...
}

// CORRECT - Succinct with value-added info
/// Creates a new thread via `svcCreateThread`. Does not start the thread.
pub fn create_thread(/* ... */) -> Result<ThreadHandle, CreateThreadError> {
    // ...
}
```

### 2. Document Safety-Critical Information

**ALWAYS** document safety requirements, invariants, and preconditions. This is non-negotiable for `unsafe fn` and `_unchecked` functions.

### 3. Add Value Beyond Code

Documentation should explain **behavior, edge cases, and important details** that aren't immediately obvious from the signature.

```rust
// WRONG - Merely repeating signature
/// Sends an IPC request to the session.
pub fn send_sync_request(session: SessionHandle) -> Result<(), SendSyncError> {
    // ...
}

// CORRECT - Documenting important behavior
/// Sends an IPC request and blocks until the response arrives.
/// The IPC buffer in TLS must be prepared before calling.
pub fn send_sync_request(session: SessionHandle) -> Result<(), SendSyncError> {
    // ...
}
```

## Function Documentation Requirements

### 1. Brief Description Required

**REQUIRED**: Every public function must have a brief description. Keep it to one or two sentences maximum.

```rust
// CORRECT - Brief, informative description
/// Allocates memory from the global heap with the specified layout.
pub fn allocate(layout: Layout) -> *mut u8 {
    // ...
}

// CORRECT - With important behavioral note
/// Closes the handle and releases associated kernel resources.
/// No-op if the handle is invalid.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    // ...
}
```

### 2. Document Key Behaviors

**RECOMMENDED**: Document important behaviors, edge cases, or non-obvious details succinctly.

```rust
// CORRECT - Succinct with key behavior noted
/// Queries the memory region containing the given address.
/// Returns `InvalidAddress` if the address is not mapped.
pub fn query_memory(addr: usize) -> Result<MemoryInfo, QueryMemoryError> {
    // ...
}

// CORRECT - Documenting atomicity
/// Updates the thread's core affinity mask. Takes effect on next reschedule.
pub fn set_thread_affinity(handle: ThreadHandle, mask: u64) -> Result<(), SetAffinityError> {
    // ...
}
```

### 3. No Returns Section

**FORBIDDEN**: Do not include `# Returns` sections. Return types are self-documenting.

```rust
// WRONG - Unnecessary returns section
/// Gets the current process handle.
///
/// # Returns
/// Returns the pseudo-handle for the current process.
pub fn current_process() -> Handle {
    // ...
}

// CORRECT - No returns section
/// Gets the pseudo-handle for the current process.
pub fn current_process() -> Handle {
    // ...
}
```

### 4. No Examples Section

**FORBIDDEN**: Do not include `# Examples` or usage examples sections in documentation. Tests serve as examples.

````rust
// WRONG - Unnecessary examples section
/// Validates a service name.
///
/// # Examples
/// ```
/// let name = ServiceName::new("sm:").unwrap();
/// ```
pub fn new(name: &str) -> Result<ServiceName, InvalidNameError> {
    // ...
}

// CORRECT - No examples section
/// Validates and creates a service name. Names must be 1-8 characters.
pub fn new(name: &str) -> Result<ServiceName, InvalidNameError> {
    // ...
}
````

### 5. No Arguments Section

**FORBIDDEN**: Do not include `# Arguments` sections. Parameter names and types are self-documenting.

```rust
// WRONG - Unnecessary arguments section
/// Creates a thread with the given parameters.
///
/// # Arguments
/// * `entry` - The thread entry point function
/// * `arg` - Argument passed to the entry function
/// * `stack_top` - Pointer to the top of the stack
/// * `priority` - Thread priority (0-63)
/// * `core_id` - Preferred CPU core
pub fn create_thread(
    entry: ThreadFunc,
    arg: usize,
    stack_top: *mut u8,
    priority: i32,
    core_id: i32,
) -> Result<ThreadHandle, CreateThreadError> {
    // ...
}

// CORRECT - Key info in description, no arguments section
/// Creates a thread. Priority must be 0-63; core_id of -2 uses default affinity.
pub fn create_thread(
    entry: ThreadFunc,
    arg: usize,
    stack_top: *mut u8,
    priority: i32,
    core_id: i32,
) -> Result<ThreadHandle, CreateThreadError> {
    // ...
}
```

## Safety Documentation

### 1. Safety Section in Unsafe Functions

**MANDATORY**: All `unsafe fn` declarations MUST include a `# Safety` section explaining the caller's responsibilities.

```rust
// CORRECT - Safety section in unsafe function
/// Reads a value from the given memory address.
///
/// # Safety
/// - `addr` must be properly aligned for type `T`
/// - `addr` must point to a valid, initialized value of type `T`
/// - The memory must not be concurrently modified
pub unsafe fn read_volatile<T>(addr: *const T) -> T {
    // ...
}

// WRONG - Missing safety section
pub unsafe fn read_volatile<T>(addr: *const T) -> T {
    // ...
}
```

### 2. Safety Section in _unchecked Functions

**MANDATORY**: All functions with `_unchecked` suffix MUST include a `# Safety` section, even if they are not `unsafe fn`.

```rust
// CORRECT - Safety section in _unchecked function
/// Creates a handle from a raw value without validation.
///
/// # Safety
/// The caller must ensure the raw value is a valid kernel handle.
/// Using an invalid handle may cause undefined behavior in SVCs.
pub fn from_raw_unchecked(raw: u32) -> Handle {
    // ...
}
```

**Safety Section Template:**

```rust
/// # Safety
/// - [First invariant the caller must uphold]
/// - [Second invariant the caller must uphold]
/// - [Additional invariants as needed]
```

### 3. SAFETY Comments at Callsites

**MANDATORY**: All callsites of `unsafe` blocks and `_unchecked` functions (except in test code) MUST be preceded by a `// SAFETY:` comment explaining why the call is safe.

```rust
// CORRECT - SAFETY comment at unsafe block
let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
// SAFETY: ipc_buf points to the thread-local IPC buffer which is always
// valid and properly aligned for IPC operations.
unsafe { cmif::make_request(ipc_buf, cmd_id, 0) };

// CORRECT - SAFETY comment for _unchecked call
let raw_handle = response.move_handles[0];
// SAFETY: Kernel returned this handle in the IPC response, guaranteeing validity.
let session = SessionHandle::from_raw_unchecked(raw_handle);

// WRONG - Missing SAFETY comment
let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
unsafe { cmif::make_request(ipc_buf, cmd_id, 0) };
```

**Exception**: Test code does not require `// SAFETY:` comments.

```rust
// CORRECT - Test code without SAFETY comments
#[test]
fn test_handle_creation() {
    let handle = Handle::from_raw_unchecked(0x1234);
    assert_eq!(handle.raw(), 0x1234);
}
```

## Panics Documentation

**MANDATORY**: If a function can panic (uses `.unwrap()`, `.expect()`, `panic!()`, indexing, or calls functions that panic), it MUST include a `# Panics` section.

```rust
// CORRECT - Panics section for function that can panic
/// Returns the first handle from the response.
///
/// # Panics
/// Panics if the response contains no move handles.
pub fn first_handle(&self) -> Handle {
    Handle::from_raw_unchecked(self.move_handles[0])
}
```

**Panics Section Template:**

```rust
/// # Panics
/// Panics if [condition that causes panic].
```

**Note**: Prefer returning `Result` or `Option` over panicking when feasible.

## FFI Documentation

### 1. FFI Function Documentation

**REQUIRED**: All `pub extern "C"` functions in `ffi` modules must be documented, including:

- What the function does
- Safety requirements (always unsafe from C perspective)
- Error return conventions (if applicable)

```rust
// CORRECT - FFI function with full documentation
/// Creates a new thread.
///
/// # Safety
/// - `entry` must be a valid function pointer
/// - `stack_top` must point to a valid stack with sufficient size
/// - `out_handle` must be a valid pointer to write the result
///
/// # Returns
/// Returns 0 on success, or a raw result code on failure.
/// On success, the new thread handle is written to `out_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_thread_create(
    out_handle: *mut u32,
    entry: ThreadFunc,
    arg: usize,
    stack_top: *mut c_void,
    priority: i32,
    core_id: i32,
) -> u32 {
    // ...
}
```

### 2. C Type Documentation

**REQUIRED**: Document C-compatible types with their intended usage and any invariants.

```rust
// CORRECT - C type with documentation
/// Thread entry function signature for C callers.
///
/// The function receives a single `usize` argument and should not return
/// (call `__nx_thread_exit` instead).
pub type ThreadFunc = unsafe extern "C" fn(arg: usize);
```

## Complete Examples

### Example 1: SVC Wrapper Function

```rust
/// Outputs a debug string to the kernel log.
///
/// The string is truncated to 256 bytes if longer. Requires debug mode enabled
/// in the kernel; otherwise returns `NotAllowed`.
pub fn output_debug_string(s: &str) -> Result<(), OutputDebugStringError> {
    let bytes = s.as_bytes();
    let len = bytes.len().min(256);

    // SAFETY: bytes points to valid UTF-8 data, len is within bounds.
    let rc = unsafe { svc::output_debug_string(bytes.as_ptr(), len) };

    match rc {
        0 => Ok(()),
        0xF001 => Err(OutputDebugStringError::NotAllowed),
        _ => Err(OutputDebugStringError::Unknown(Error::from_raw(rc))),
    }
}
```

### Example 2: Unsafe Function with Safety Section

```rust
/// Writes a value to the IPC buffer at the specified word offset.
///
/// # Safety
/// - `ipc_buf` must point to a valid IPC buffer (at least 256 bytes)
/// - `offset` must be less than 64 (buffer is 64 words)
/// - No other code may access the same buffer region concurrently
pub unsafe fn write_raw(ipc_buf: *mut u32, offset: usize, value: u32) {
    debug_assert!(offset < 64, "IPC buffer offset out of bounds");
    ipc_buf.add(offset).write_volatile(value);
}
```

### Example 3: FFI Function

```rust
/// Allocates memory with the specified size and alignment.
///
/// # Safety
/// - `size` must be non-zero
/// - `align` must be a power of two
///
/// # Returns
/// Returns a pointer to the allocated memory, or null on failure.
/// The caller is responsible for freeing with `__nx_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_memalign(align: usize, size: usize) -> *mut c_void {
    // ...
}
```

## Checklist

Before committing code, verify:

### Function Documentation
- [ ] All public functions have succinct documentation (1-2 sentences max)
- [ ] Documentation includes key behaviors and edge cases
- [ ] No `# Arguments`, `# Returns`, or `# Examples` sections
- [ ] Documentation adds value beyond what the signature conveys

### Safety Documentation
- [ ] All `unsafe fn` have `# Safety` section in rustdocs
- [ ] All `_unchecked` functions have `# Safety` section in rustdocs
- [ ] All `unsafe` blocks (except in tests) have `// SAFETY:` comments
- [ ] All `_unchecked` callsites (except in tests) have `// SAFETY:` comments
- [ ] Safety comments explain why the operation is safe

### Panics Documentation
- [ ] Functions that can panic have `# Panics` section
- [ ] Panic conditions are clearly documented

### FFI Documentation
- [ ] All `pub extern "C"` functions are documented
- [ ] FFI safety requirements are documented
- [ ] Return value conventions are documented