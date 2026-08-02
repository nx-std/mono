---
name: "pattern-typestate"
description: "Typestate pattern — model state machines with distinct types to enforce valid transitions at compile time. Load when designing workflows, pipelines, or objects with lifecycle states"
type: "core"
scope: "global"
---

# Typestate Pattern (State Machines with Types)

## Rule

Use distinct types for each state to prevent invalid transitions at compile time. When an object has a lifecycle (reserved → mapped → released), each phase is a separate type, so only the operations valid in that state exist.

A struct that guards transitions with a status enum and runtime assertions catches invalid transitions only at runtime. In a `no_std`, `panic = "abort"` target such an assertion is not a recoverable error — it aborts the process on the console. Replace the enum with distinct types that consume `self` on transition, making an invalid transition a compile error.

Model each state as its own concrete struct (`ReservedPages` → `MappedPages` → `ReleasedPages`), not as a generic `Pages<S>` carrying a `PhantomData` marker. Concrete structs give each state a name that appears in signatures, error messages, and rustdoc, and let a state hold exactly the fields it owns without threading type parameters through every caller.

## Examples

```rust
// ❌ Bad — runtime state checking; a wrong-order call aborts the console instead of failing to compile
pub struct PageRange {
    state: RangeState,
    addr: usize,
    len: usize,
    handle: Option<MemoryHandle>,
    // ...
}

impl PageRange {
    pub fn map(&mut self, handle: MemoryHandle) {
        assert_eq!(self.state, RangeState::Reserved); // Aborts at runtime!
        self.handle = Some(handle);
        self.state = RangeState::Mapped;
    }

    pub fn release(&mut self) {
        assert_eq!(self.state, RangeState::Mapped); // Aborts at runtime!
        self.handle = None;
        self.state = RangeState::Reserved;
    }
}

// Nothing stops a caller from releasing a range that was never mapped, and the
// `Option<MemoryHandle>` forces every accessor to unwrap a value the state already implies
```

```rust
// ✅ Good — the type system rejects an unmap of a never-mapped range at compile time
pub struct ReservedPages { addr: usize, len: usize }
pub struct MappedPages { addr: usize, len: usize, handle: MemoryHandle, perm: Permission }
pub struct ReleasedPages { addr: usize, len: usize }

impl ReservedPages {
    pub fn map(self, handle: MemoryHandle, perm: Permission) -> Result<MappedPages, MapError> {
        svc_map_memory(self.addr, self.len, handle, perm)?;
        Ok(MappedPages { addr: self.addr, len: self.len, handle, perm })
    }
}

impl MappedPages {
    /// Only a mapped range exposes the backing memory.
    pub fn as_slice(&self) -> &[u8] { /* ... */ }

    pub fn unmap(self) -> Result<ReleasedPages, MapError> {
        svc_unmap_memory(self.addr, self.len, self.handle)?;
        Ok(ReleasedPages { addr: self.addr, len: self.len })
    }
}

// Usage:
let pages = ReservedPages::reserve(len)?;
let pages = pages.map(handle, Permission::ReadWrite)?; // ReservedPages -> MappedPages
let pages = pages.unmap()?;                            // MappedPages -> ReleasedPages
// pages.as_slice();                                   // Compile error — ReleasedPages has no as_slice()
```

```rust
// ✅ Good — shared fields factored into a private struct, keeping each state a distinct named type
/// Data every state carries; private, so the span cannot be rebuilt outside this module.
struct PageSpan { addr: usize, len: usize }

pub struct ReservedPages { span: PageSpan }
pub struct MappedPages { span: PageSpan, handle: MemoryHandle, perm: Permission }
pub struct ReleasedPages { span: PageSpan }

impl ReservedPages {
    pub fn map(self, handle: MemoryHandle, perm: Permission) -> Result<MappedPages, MapError> {
        svc_map_memory(self.span.addr, self.span.len, handle, perm)?;
        Ok(MappedPages { span: self.span, handle, perm })
    }
}

impl MappedPages {
    pub fn unmap(self) -> Result<ReleasedPages, MapError> {
        svc_unmap_memory(self.span.addr, self.span.len, self.handle)?;
        Ok(ReleasedPages { span: self.span })
    }
}
```

## Why It Matters

Runtime state assertions are invisible to the compiler: they fail only when the wrong code path executes, which may happen first on the console under specific conditions. Typestate turns an invalid transition into a compile error, eliminating an entire class of logic bugs, and the type signature documents which operations are valid in each state — enforcement and documentation in one.

## Pragmatism Caveat

Not every stateful object needs typestate. An object with two states, or simple well-tested transitions, may be simpler as a status enum with clear documentation. Apply typestate when invalid transitions would cause serious bugs, when the state machine is complex enough that runtime assertions are easy to forget, or when multiple callers might not know the correct transition order. For objects stored in a fixed-size collection or exposed across the C FFI boundary (where a single concrete type is needed), a status enum is often the practical choice — typestate works best for in-memory, linear workflows.

## Checklist

Before committing code, verify:

- [ ] State transitions consume `self` (move semantics) to prevent reuse of the old state
- [ ] Each state is a distinct concrete struct that only exposes operations valid for that state, not a generic wrapper parameterized by a `PhantomData` marker
- [ ] No runtime assertions (`assert!`, `panic!`) for state validity that the type system could enforce
- [ ] State-specific data is only present in the types where it exists (e.g., `handle` only in `MappedPages`)
- [ ] Simple two-state objects or FFI-facing types use status enums when typestate adds unnecessary complexity

## References

- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: Design principle this pattern implements
- [pattern-builder](pattern-builder.md) - Related: Builder pattern can use typestate for compile-time required field enforcement
