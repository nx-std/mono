---
name: "principle-validate-at-edge"
description: "Validate at the Edge (hard shell, soft core) — parse untrusted input once at the boundary. Load when designing `__nx_*` FFI entry points, converting raw handles and pointers, or decoding IPC responses"
type: "principle"
scope: "global"
---

# Validate at the Edge (Hard Shell, Soft Core)

## Rule

Every value that enters from outside — a pointer and a length handed in by a C caller, a raw `u32` handle, a
`#[repr(C)]` struct filled in by libnx, an IPC response buffer written by a system service, a `ResultCode`
returned by the kernel — arrives untrusted and typed wider than reality. Parse it **once**, at the boundary,
into a type the rest of the code can trust. Past that point, no function re-checks. The boundary is the hard
shell; the domain is the soft core. Concretely:

- Parsing lives in `FromStr` or `TryFrom`, not in a standalone `parse` function and not in the body of the
  `extern "C"` shim. That is the one place a newtype's invariant is established, and it is what `?`, the
  `Option` combinators, and zerocopy's fallible readers all compose with.
- A shim's job is to turn raw arguments into domain types and hand them on. Domain functions take the parsed
  types, never the raw pointer, length or handle number.
- Malformed input degrades into a typed error at the edge — a `ResultCode` returned to the C caller, a refused
  mapping, a rejected response — never an abort three frames down.
- Unit and convention conversions happen once, at the boundary: raw `u32` to `Handle`, bytes to pages,
  `ResultCode` to typed error.

## Examples

1. **Parse into domain types at the boundary; the domain trusts them**
   A shim receives pointers and integers. Everything past it receives meaning.

```rust
// ❌ Bad — the domain function takes the raw pointer, length and handle and checks them
// itself. Every other caller must remember to do the same, and the one that forgot passed
// a null `out` that was dereferenced three frames down, aborting the process at an address
// with nothing in it to attribute the fault to a caller.
unsafe fn read_counters(out: *mut u64, count: usize, handle: u32) -> Result<(), CounterError> {
    if out.is_null() {
        return Err(CounterError::NullBuffer);
    }
    if count == 0 || count > MAX_COUNTERS {
        return Err(CounterError::BadCount);
    }
    if handle == INVALID_HANDLE {
        return Err(CounterError::BadHandle);
    }
    // ...domain logic, finally
}
```

```rust
// ✅ Good — the invariants live in TryFrom; the shim parses; the domain trusts. Every path
// into the domain goes through the same types, including the Rust-only callers that never
// touch the FFI surface.
impl TryFrom<u32> for EventHandle {
    type Error = ParseEventHandleError;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        if raw == INVALID_HANDLE {
            return Err(ParseEventHandleError::Invalid);
        }
        Ok(Self(raw))
    }
}

// Boundary — the only place a raw pointer, length or handle number is looked at
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_evt__read_counters(out: *mut u64, count: usize, handle: u32) -> ResultCode {
    let Ok(handle) = EventHandle::try_from(handle) else {
        return ParseEventHandleError::Invalid.to_raw_rc();
    };
    // SAFETY: `from_raw_parts` rejects a null base and a zero or oversized count; past that
    // the C contract documents `out` as writable for `count` elements.
    let Some(out) = (unsafe { OutBuf::from_raw_parts(out, count) }) else {
        return CounterError::BadBuffer.to_raw_rc();
    };
    read_counters(handle, out).map_or_else(|err| err.to_raw_rc(), |()| RESULT_SUCCESS)
}

// Domain — no defensive checks; the types carry the proof
fn read_counters(handle: EventHandle, out: OutBuf<'_, u64>) -> Result<(), CounterError> {}
```

2. **Cross-field constraints belong to a composite type**
   A relationship between two fields is an invariant of the pair, so the pair is the type.

```rust
// ❌ Bad — the wrap check sits in the one function that happened to need it, and the
// "is `size` bytes or pages?" convention is re-decided at every call site.
fn map_region(base: usize, size: usize) -> Result<(), MapError> {
    if base.checked_add(size).is_none() {
        return Err(MapError::AddressWraps);
    }
    // is `size` bytes here? the caller two modules up passed a page count
}
```

```rust
// ✅ Good — the pair is a type, alignment and non-wrapping are its invariants, and the
// unit is stated once and carried in the docs.
/// A page-aligned virtual range: `base` included, `base + len` excluded, both in **bytes**.
pub struct PageRange {
    base: usize,
    len: usize,
}

impl TryFrom<(usize, usize)> for PageRange {
    type Error = ParsePageRangeError;

    fn try_from((base, len): (usize, usize)) -> Result<Self, Self::Error> {
        if base % PAGE_SIZE != 0 || len % PAGE_SIZE != 0 {
            return Err(ParsePageRangeError::Misaligned { base, len });
        }
        if len == 0 {
            return Err(ParsePageRangeError::Empty);
        }
        base.checked_add(len)
            .ok_or(ParsePageRangeError::Wraps { base, len })?;
        Ok(Self { base, len })
    }
}

fn map_region(range: PageRange) -> Result<(), MapError> {}
```

3. **A `#[repr(C)]` struct from C is converted into validated types, once**
   A field checked wherever it is consumed is a field that is eventually consumed somewhere new.

```rust
// ❌ Bad — the raw struct is trusted, and every consumer re-checks the fields it uses.
// The consumer added last did not, and a zero stack size produced a mapping with no guard
// page, so the new thread's first deep call scribbled over the stack below it.
#[repr(C)]
pub struct ThreadAttrRaw {
    pub stack_size: usize,
    pub priority: i32,
    pub core_id: i32,
}

pub fn spawn(attr: &ThreadAttrRaw) -> Result<ThreadHandle, SpawnError> {
    if attr.stack_size == 0 {
        return Err(SpawnError::BadStackSize);
    }
    if !(0..=HIGHEST_PRIORITY).contains(&attr.priority) {
        return Err(SpawnError::BadPriority);
    }
    // ...and the next consumer re-checks whichever of these it happens to read
}
```

```rust
// ✅ Good — the conversion is the boundary. An invalid attribute block fails at the shim,
// naming the field and the reason, before a single page is reserved.
pub struct ThreadAttr {
    stack_size: StackSize,
    priority: Priority,
    core_id: CoreId,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseThreadAttrError {
    #[error("stack size must be a non-zero multiple of {PAGE_SIZE:#x}, got {0:#x}")]
    StackSize(usize),
    #[error("priority must be in 0..={HIGHEST_PRIORITY}, got {0}")]
    Priority(i32),
    #[error("core id {0} is not in the process core mask")]
    CoreId(i32),
}

impl TryFrom<&ThreadAttrRaw> for ThreadAttr {
    type Error = ParseThreadAttrError;

    fn try_from(raw: &ThreadAttrRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            stack_size: StackSize::try_from(raw.stack_size)?,
            priority: Priority::try_from(raw.priority)?,
            core_id: CoreId::try_from(raw.core_id)?,
        })
    }
}

pub fn spawn(attr: ThreadAttr) -> Result<ThreadHandle, SpawnError> {
    reserve_stack(attr.stack_size)
    // ...no defensive checks; the fields cannot hold a value spawn would have to reject
}
```

4. **A malformed IPC response degrades; it does not take the process down**
   A system service is a foreign process. It will eventually answer with something its own documentation does
   not describe.

```rust
// ❌ Bad — the response buffer is trusted to have the shape the type claims. A service that
// answered with a short error response instead of the documented payload was read past the
// end of the TLS IPC buffer, and the caller got a session handle made of whatever the last
// request had left in those words.
fn parse_open_session(buf: &[u8; IPC_BUFFER_SIZE]) -> OpenSession {
    // SAFETY: none — the header is assumed present and well-formed
    let header = unsafe { &*buf.as_ptr().cast::<OutHeader>() };
    OpenSession {
        session: unsafe { SessionHandle::from_raw(header.handles[0]) },
        object_id: u32::from_le_bytes(buf[0x20..0x24].try_into().unwrap()),
    }
}
```

```rust
// ✅ Good — decoding is fallible at the edge, and the failure carries what is needed to act
// on it: which field, what was expected, what arrived. The kernel's ResultCode becomes a
// typed error here and nowhere else.
#[derive(Debug, thiserror::Error)]
pub enum ParseOpenSessionError {
    #[error("response truncated: need {need} bytes, got {got}")]
    Truncated { need: usize, got: usize },
    #[error("bad out-header magic: expected {OUT_HEADER_MAGIC:#x}, got {got:#x}")]
    BadMagic { got: u32 },
    #[error("service returned an error")]
    Service(#[from] nx_svc::result::Error),
    #[error("response carried {got} move handles, expected 1")]
    MissingHandle { got: usize },
}

fn parse_open_session(buf: &[u8; IPC_BUFFER_SIZE]) -> Result<OpenSession, ParseOpenSessionError> {
    let (header, rest) = OutHeader::read_from_prefix(buf.as_slice()).map_err(|_| {
        ParseOpenSessionError::Truncated { need: size_of::<OutHeader>(), got: buf.len() }
    })?;
    if header.magic.get() != OUT_HEADER_MAGIC {
        return Err(ParseOpenSessionError::BadMagic { got: header.magic.get() });
    }
    nx_svc::result::Error::from_raw_rc(header.result.get())?;

    let raw_handle = *header
        .move_handles
        .first()
        .ok_or(ParseOpenSessionError::MissingHandle { got: header.move_handles.len() })?;
    let session = SessionHandle::try_from(raw_handle)?;
    let object_id = ObjectId::read_from_prefix(rest)
        .map_err(|_| ParseOpenSessionError::Truncated { need: size_of::<ObjectId>(), got: rest.len() })?
        .0;
    Ok(OpenSession { session, object_id })
}
```

## Why It Matters

This code sits between C homebrew that calls in through `__nx_*` with raw pointers and integers, and system
services that answer over IPC, either of which can produce a value the types do not describe: a null where a
buffer was promised, a length that wraps the address space, a response header whose magic is not the documented
one. Trusted where they land, the failure surfaces somewhere else entirely — an abort inside the allocator
caused by a length accepted three calls earlier, with nothing but a PC address tying the two together.

A single narrow point also localizes the fix. When a service changes the shape of a field, the one decode site
is what changes, and a failed decode is a `ResultCode` returned to the caller rather than a fault at an unmapped
address. Because the invariant lives in `TryFrom`, every entry point that reaches the same domain — the FFI
shim, the Rust API, the internal retry path — enforces it identically, for free. Scattered per-layer checks buy
the opposite: three partial contracts, and the union of them is nobody's job.

## Pragmatism Caveat

The signal for where a check belongs is what it depends on:

- **Depends only on the incoming value → the edge**: shape, null pointers, alignment, ranges, and cross-field
  constraints within one call or one response.
- **Depends on external state → the domain**: "is this handle still open?", "is this page range already
  reserved?", "is there room left in the session table?". These need the world, and the edge does not have it.

Do not push state-dependent checks into the boundary, and do not let shape checks leak past it. Do not
re-validate in the soft core: a function taking a parsed type does not check its fields again. An undocumented
re-validation is dead code that hides where the real contract lives.

## Checklist

Before committing code, verify:

- [ ] Every invariant is established in `FromStr` or `TryFrom`, not in an `extern "C"` shim body or a standalone
      `parse`
- [ ] Shims parse the raw arguments into domain types; domain functions take the parsed types, never raw
      pointers, lengths or handle numbers
- [ ] Cross-field constraints are invariants of a composite type, not checks in one function that needed them
- [ ] `#[repr(C)]` structs crossing the boundary are converted into validated types at entry; consumers do not
      re-check
- [ ] IPC responses are decoded fallibly, with errors naming the field and the context, never `unwrap` or a
      blind cast
- [ ] Downstream functions contain zero re-validation of what the boundary guaranteed
- [ ] Unit and convention conversions (bytes to pages, `ResultCode` to typed error) happen once, at the boundary
- [ ] Checks that require external state stay in the domain

## References

- [principle-type-driven-design](principle-type-driven-design.md) - Related: The edge produces the validated
  types that make illegal states unrepresentable
- [principle-idempotency](principle-idempotency.md) - Related: The boundary that accepts a request is where its
  identity key is established
- [principle-least-surprise](principle-least-surprise.md) - Related: A function whose parameter is a parsed type
  must behave as though it trusts it

## External References

- [Parse, Don't Validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [Using Types To Guarantee Domain Invariants](https://lpalmieri.com/posts/2020-12-11-zero-to-production-6-domain-modelling/)
- [Architecture Patterns with Python (O'Reilly)](https://www.oreilly.com/library/view/architecture-patterns-with/9781492052197/)
