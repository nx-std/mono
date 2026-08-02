---
name: "pattern-newtype"
description: "Newtypes for identity, invariants, and units: private field, FromStr as the only validator. Load when a domain value is a bare String or integer, or two same-typed parameters sit side by side"
type: "core"
scope: "global"
---

# Newtype (Wrapped Primitive)

## Rule

A domain value carried as a bare `u32`, `usize`, or byte array makes every reader remember an invariant the
compiler could have remembered for them. When the value has an **identity**, an **invariant**, or a **unit**,
declare it as a newtype — `struct ProcessHandle(RawHandle)` — a primitive the compiler refuses to interchange
with its neighbours.

A value earns a newtype when it does at least one of three jobs:

1. **Identity** — same-typed values that must never be swapped: a process handle, a transfer-memory handle, and
   the session handle derived from a port, all meeting in one SVC wrapper.
2. **Invariant** — a constraint established once, at the edge, and never re-checked: a page-aligned address, a
   thread priority inside the legal range, a service name that fits in eight NUL-padded bytes.
3. **Unit** — a unit, base, or convention a bare primitive cannot state: bytes versus data words, kernel ticks
   versus nanoseconds, core index versus core mask.

Three signals a newtype is missing, all visible in a diff:

- Two parameters of the same primitive type sit side by side (`map_transfer_memory(process: u32, memory: u32)`).
- A `* 4` or `/ 4` whose meaning lives in a comment rather than in a type.
- A doc comment saying what the type should have said: _"must be page-aligned"_, _"length in words"_,
  _"0..=63"_.

**Declaring.** A tuple struct with a **private** field, so construction cannot bypass the invariant. The
validating constructor is `TryFrom` for numeric, address, and handle values, and `FromStr` for values that
genuinely arrive as text — a service name written in a config or a test — and nothing else; see
[rust-fn-unchecked](rust-fn-unchecked.md) for the narrow cases where a constructor may skip it. Add `Display`,
`AsRef<[u8]>`, and a raw accessor for the FFI edge, so the type is as convenient as the primitive it replaces,
and decode wire forms by running the raw `zerocopy` struct through the same `TryFrom`, so the check is not
skipped by a cast.

**Constructing.** At the boundary the value enters through, and nowhere else. A newtype constructed all over
the domain is a newtype whose invariant nobody can locate.

## Examples

1. **Two handles that are not the same handle**
   Mapping transfer memory takes the process it is mapped into and the memory block itself. Both are raw
   handles, they sit side by side, and transposing them produces a plausible-looking call that compiles.

```rust
// ❌ Bad — two bare handles in the same signature. `map_transfer_memory(memory, process, ..)`
// compiles, the kernel rejects it with `InvalidHandle`, and the error surfaces in a
// caller three crates away that never named either handle.
pub fn map_transfer_memory(process: RawHandle, memory: RawHandle, perm: Permission) -> Result<(), MapError> {}
```

```rust
// ✅ Good — two types, so the transposition is a compile error at every call site, forever.
pub struct ProcessHandle(RawHandle);
pub struct TransferMemoryHandle(RawHandle);

pub fn map_transfer_memory(
    process: &ProcessHandle,
    memory: &TransferMemoryHandle,
    perm: Permission,
) -> Result<(), MapError> {}
```

2. **A unit written into the type, and enforced at the edge**
   IPC payload sizes are counted in 4-byte data words inside the message header and in bytes everywhere else,
   but `usize` cannot say which, and the one caller that read words as bytes truncated every reply.

```rust
// ❌ Bad — the unit lives in a doc comment, and the `/ 4` that reconciles the two readings
// is copied to three call sites. The one that dropped it declared a payload four times
// too large, so the header claimed words past the end of the 0x100-byte TLS buffer and
// the request came back as a malformed-header error with no hint at the arithmetic.
/// Reserve `len` data words for the request payload.
pub fn reserve_payload(len: usize) -> Result<&mut [u8], BuildError> {}

let buf = reserve_payload(size_of::<Args>())?;
```

```rust
// ✅ Good — the unit is the type, and the one conversion between the two lives in one
// function. A missing conversion is now a missing call, not an absent `/ 4`.
pub struct ByteLen(usize);

/// A payload length counted in 4-byte IPC data words.
pub struct WordLen(usize);

impl TryFrom<ByteLen> for WordLen {
    type Error = NotWordAligned;

    fn try_from(len: ByteLen) -> Result<Self, Self::Error> {
        match len.0 % 4 {
            0 => Ok(Self(len.0 / 4)),
            rem => Err(NotWordAligned { remainder: rem }),
        }
    }
}

pub fn reserve_payload(len: WordLen) -> Result<&mut [u8], BuildError> {}
```

3. **Brand the subset a function actually requires**
   Every mapping address is an address, but not every address is one the kernel will map. When a _subset_ of a
   type is what a function requires, make the subset a type too, narrowed once where the value is computed
   rather than re-checked defensively at each call.

```rust
// ❌ Bad — every caller is trusted to have aligned the address, and the one that did not
// passes an allocator-returned pointer straight to the SVC, which fails with
// `InvalidAddress` — an error naming neither the allocation it came from nor its offset.
pub fn set_memory_permission(addr: usize, size: usize, perm: Permission) -> Result<(), MemError> {
    raw::set_memory_permission(addr, size, perm)
}
```

```rust
// ✅ Good — two types, because there are two claims: it is an address, and it is one the
// kernel will accept. The narrowing happens once, where the value is computed.
pub struct PageAlignedAddr(usize);

impl TryFrom<usize> for PageAlignedAddr {
    type Error = NotPageAligned;

    fn try_from(addr: usize) -> Result<Self, Self::Error> {
        match addr % PAGE_SIZE {
            0 => Ok(Self(addr)),
            offset => Err(NotPageAligned { offset }),
        }
    }
}

// Cannot be called with an unaligned address, so it needs no check and has no failure mode.
pub fn set_memory_permission(
    addr: PageAlignedAddr,
    size: PageAlignedLen,
    perm: Permission,
) -> Result<(), MemError> {}
```

## Why It Matters

**The compiler remembers, so the reader does not.** "The first argument is the process handle"; "this length is
in words"; "this address is page-aligned" — each is a fact a maintainer holds in their head at every call site,
and forgets exactly once.

**API misuse becomes a compile error instead of a plausible result.** The failures a newtype prevents are the
quiet ones: a transposed handle reaches the kernel as a well-formed SVC that returns a generic result code; a
dropped `/ 4` writes a header claiming four times the payload. Neither panics, neither fails a compile, and
both are ruled out once the types differ.

**An invariant checked at the edge stays checked.** A `ThreadPriority` cannot exist outside its legal range, so
nothing downstream re-validates it or has to trust a comment — that is
[principle-validate-at-edge](principle-validate-at-edge.md) in the type system rather than in discipline.

**It costs nothing at runtime**: a `#[repr(transparent)]` tuple struct has the primitive's layout, so it crosses
the FFI boundary unchanged, and only the validating constructor runs a check, once, at the boundary. **The
documentation cannot go stale**, because it is the signature.

## Pragmatism Caveat

Wrap a value that has an invariant, a unit, or a confusable sibling; a newtype with nothing to distinguish is ceremony:

- **No sibling, no invariant, no newtype.** A debug label, a panic message, a free-form module description:
  nothing to swap them with, nothing to check. Leave them `&str`.
- **Named struct fields blunt the swap hazard** positional parameters create. Two values that are genuinely the
  same kind playing different roles — the two handles of a session pair — want a struct with named fields, not
  two newtypes differing in name only.
- **A newtype is not a validator for kernel state.** "Is this handle still open?", "is this page currently
  mapped?" depend on the kernel, not the value. Those stay runtime checks against the result code.
- **Keep validation off hot paths.** The check runs on every construction: right at a boundary, wrong per
  request word in a marshalling loop.
- **If a newtype forces `*_unchecked` calls at ordinary call sites, the boundary is in the wrong place.** Move
  construction to the edge the value actually enters through — typically the `__nx_*` FFI shim; see
  [rust-fn-unchecked](rust-fn-unchecked.md).

## Checklist

Before committing code, verify:

- [ ] Every domain value with an invariant, a unit or base, or a confusable same-typed sibling is a newtype
- [ ] The wrapped field is private, so construction cannot bypass the invariant
- [ ] The validating constructor is `TryFrom` (or `FromStr` for genuinely textual values); there is no second, parallel validator
- [ ] Decoding a wire form runs the same check, rather than casting the raw bytes straight into the newtype
- [ ] The newtype is constructed at the boundary the value enters through — the FFI shim or the IPC decoder — and nowhere else
- [ ] A conversion between two newtypes (bytes to words, core index to core mask) exists in exactly one function
- [ ] `Display`, `AsRef`, and the primitive's other conveniences are provided, so callers never reach for the inner value
- [ ] No newtype was added to a value with nothing to confuse it with and no invariant to carry

## References

- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: A newtype makes an invalid value unrepresentable
- [principle-validate-at-edge](principle-validate-at-edge.md) - Foundation: Constructed at the edge, never re-checked downstream
- [principle-least-surprise](principle-least-surprise.md) - Foundation: A signature saying `u32` twice surprises the caller who transposes them
- [rust-fn-unchecked](rust-fn-unchecked.md) - Related: The narrow cases where construction may skip the validating constructor, and the comment that must accompany it
- [pattern-builder](pattern-builder.md) - Related: Construction with multiple required fields

## External References

- [Rust API Guidelines — Newtypes](https://rust-lang.github.io/api-guidelines/type-safety.html#newtypes-provide-static-distinctions-c-newtype)
- [Parse, Don't Validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [The Ultimate Guide to Rust Newtypes](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes)
