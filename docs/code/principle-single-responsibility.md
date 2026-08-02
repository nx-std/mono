---
name: "principle-single-responsibility"
description: "Single Responsibility — one struct, one reason to change; split when it spans external systems or mixes I/O with pure computation. Load when designing structs, splitting modules, or reviewing types that do both"
type: "principle"
scope: "global"
---

# Single Responsibility Principle (SRP)

## Rule

A struct or module owns one responsibility. Split when any of these observable signals is present:

1. **Multiple external systems**: the methods touch several distinct outside worlds (the kernel via SVCs, a
   service session over IPC, the global allocator, the virtual-memory reservation map, the C-FFI boundary).
   Each boundary is its own concern. Three is a strong signal; two warrants a split when they change
   independently.
2. **Disjoint field access**: the methods partition into groups that touch non-overlapping sets of fields. The
   groups are separate types sharing a struct by coincidence.
3. **Mixed I/O and transformation**: the same unit both performs effects (an SVC, an IPC round trip, an
   allocation) and does pure computation (request encoding, response decoding, page and alignment arithmetic,
   result-code mapping, buffer layout planning). Extract the pure part — it is the part worth unit testing, and
   effects make that impossible.

When a signal fires, split into focused units and compose them.

## Examples

1. **One concern per module in a crate**
   A service-client crate is four modules, each with exactly one job: a name table (which interface is served
   under which service name, is it available), a session (one handle, one request, one response), a shared-memory
   region (reserve, map, unmap, dropped as a unit), and the FFI shim that exposes the client to C.

```rust
// ❌ Bad — one struct owns the name table, the session, the shared-memory mapping, and the
// FFI surface. Signals 1 and 2 both fire: it touches `sm:` over IPC + the kernel via SVCs +
// the C-FFI boundary, and `names`/`available` are never read by the same methods that read
// `session`/`shmem`.
pub struct PadService {
    names: [ServiceName; MAX_INTERFACES],
    available: [bool; MAX_INTERFACES],
    session: SessionHandle,
    shmem: SharedMemoryHandle,
    shmem_addr: *mut u8,
}

impl PadService {
    pub fn is_available(&self, iface: Interface) -> bool {}
    pub fn name_for(&self, iface: Interface) -> Option<ServiceName> {}
    pub fn connect(&mut self, name: ServiceName) -> Result<(), ConnectError> {}
    pub fn read_state(&self, pad: PadId) -> Result<PadState, DispatchError> {}
    pub fn decode_state(&self, raw: &[u8]) -> Result<PadState, DecodeError> {}
    pub fn map_shmem(&mut self) -> Result<(), MapError> {}
    pub unsafe extern "C" fn __nx_pad_read_state(out: *mut RawPadState) -> u32 {}
}
```

```rust
// ✅ Good — four units, each describable in one sentence, composed by the caller.
// The name-table module — "which service name serves this interface, and is it available"
pub fn name_for(iface: Interface) -> Option<ServiceName>;
pub fn is_available(name: ServiceName) -> bool;

// The session module — "one handle, one request, one response"
pub struct PadSession { /* ... */ }

impl PadSession {
    pub fn open(name: ServiceName) -> Result<Self, ConnectError>;
    pub fn read_state(&self, pad: PadId) -> Result<PadState, DispatchError>;
}

// The shared-memory module — "one region: reserved, mapped, unmapped on drop"
pub struct PadSharedMemory { /* ... */ }

impl PadSharedMemory {
    pub fn map(handle: SharedMemoryHandle, size: usize) -> Result<Self, MapError>;
    pub fn states(&self) -> &[RawPadState];
}

// The ffi module — "expose the client to C"
pub unsafe extern "C" fn __nx_pad_read_state(out: *mut RawPadState) -> u32;
```

2. **Separate the pure transformation from the effect**
   Writing a large payload over IPC splits in two: one function does the session round trips, another is a pure
   planning function. Only the pure one holds the tricky invariant (chunks are cut on page boundaries, never
   mid-page), and only the pure one can be unit tested directly.

```rust
// ❌ Bad — the chunking rule is trapped inside the IPC round trip. Testing "a chunk never
// starts mid-page" now requires a console and a live session, and the first version of this
// cut on `MAX_CHUNK_LEN` instead of the page boundary below it — the server mapped the second
// send-buffer from the containing page and overwrote the tail of the first, which showed up
// as corrupted data on hardware and nowhere else.
pub fn write_payload(session: &Session, bytes: &[u8]) -> Result<usize, DispatchError> {
    let mut written = 0;
    while written < bytes.len() {
        let end = (written + MAX_CHUNK_LEN).min(bytes.len());
        let chunk = &bytes[written..end];
        session.dispatch_in(CMD_WRITE, InputBuffer::new(chunk, BufferMode::Normal))?;
        written = end;
    }
    Ok(written)
}
```

```rust
// ✅ Good — the effect is a thin shell over a pure core.
/// Cut a `base`-anchored payload of `len` bytes into chunks on page boundaries.
///
/// A chunk never starts mid-page: the kernel maps a send-buffer by whole pages,
/// so two chunks sharing a page would alias in the server's address space.
pub fn plan_page_chunks(base: usize, len: usize, max_chunk: usize) -> ArrayVec<Range<usize>, MAX_CHUNKS> {
    // pure: two integers in, ranges out — tested with an assertion, no session, no kernel
}

/// Send each planned chunk as one request. Returns the number of bytes written.
pub fn write_payload(session: &Session, bytes: &[u8]) -> Result<usize, DispatchError> {
    let base = bytes.as_ptr() as usize;
    for range in plan_page_chunks(base, bytes.len(), MAX_CHUNK_LEN) {
        let chunk = &bytes[range];
        session.dispatch_in(CMD_WRITE, InputBuffer::new(chunk, BufferMode::Normal))?;
    }
    Ok(bytes.len())
}
```

## Why It Matters

A type with one responsibility has one reason to change. A session changing how it retries a request cannot
break response decoding, because it does not contain any. A name table gaining an interface cannot break the
shared-memory mapping.

The testability consequence is concrete: anything fused to an SVC or a session can only be exercised on
hardware — built as an NRO, deployed to a console or an emulator, and read off the screen. Separable pure logic
compiles for the host and is tested with an assertion, in milliseconds, at the point of the change. Every
invariant left inside an effect is an invariant that gets checked once a day instead of once an edit, and the
cases that are awkward to reach on a console end up unchecked.

## Pragmatism Caveat

Small structs that touch two systems are not automatically wrong. A session that owns both the kernel handle
and the server's pointer-buffer size is one concern, because the whole point of the type is the coupling
between them (the size is queried over the same handle at construction and governs every later request on it) —
splitting them would put the invariant in neither half. An allocator owns its heap region handle because the
region's lifetime _is_ the allocator's lifetime.

When a signal fires and you keep the concerns together, say why in the module `//!` docs or a comment
(transactional atomicity, a shared invariant, a lifetime that cannot be split). An undocumented violation is
always wrong.

## Checklist

Before committing code, verify:

- [ ] Each struct or module can be described in one sentence without "and"
- [ ] No type both performs I/O — an SVC, an IPC round trip, an allocation — and holds a non-trivial pure
      algorithm; the algorithm is a free function
- [ ] Fields partition into one cohesive group, not two groups touched by disjoint method sets
- [ ] A change to one concern (name-table data, result-code mapping, handle lifecycle) touches one module
- [ ] Deliberate co-location of concerns is explained in the module docs

## References

- [principle-law-of-demeter](principle-law-of-demeter.md) - Related: Overloaded types are the ones callers end
  up navigating through
- [principle-open-closed](principle-open-closed.md) - Related: Extension points require variants that each own
  one concern
- [principle-inversion-of-control](principle-inversion-of-control.md) - Related: Separating pure logic is what
  makes injecting collaborators worthwhile
- [principle-dry-wet](principle-dry-wet.md) - Related: An abstraction serving two responsibilities is the wrong
  abstraction
- [principle-rate-of-change](principle-rate-of-change.md) - Related: Two rates of change are two reasons to
  change; the same split reached from the other side

## External References

- [SOLID: The Single Responsibility Principle (Uncle Bob)](https://blog.cleancoder.com/uncle-bob/2014/05/08/SingleReponsibilityPrinciple.html)
- [Single Responsibility Principle with a Rust Example](https://medium.com/@dogabudak/single-responsibility-principle-with-a-rust-example-2940504e3ebd)
