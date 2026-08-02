---
name: "principle-information-hiding"
description: "Information Hiding — reveal as little as possible; every item takes the most restrictive visibility that works. Load when choosing pub or pub(crate), or reviewing a crate's surface"
type: "principle"
scope: "global"
---

# Information Hiding (Reveal As Little As Possible)

## Rule

A module is defined by the design decision it **hides**. Its surface reveals as little as possible about how it
works, so the decision can change without any caller learning that it did.

Operationally: **every item carries the most restrictive visibility that still lets it do its job.** Private is
the default, and each widening is a decision that needs a reason:

| Widen to     | When                                    |
|--------------|-----------------------------------------|
| private      | Always, unless something below applies  |
| `pub(crate)` | A sibling module in this crate needs it |
| `pub`        | A consumer outside the crate needs it   |

There is no fourth case. `pub(super)` and `pub(in path)` are not a middle ground — they signal that the module
tree is wrong, because an item exactly one parent may see is an item that belongs to that parent.

Two consequences make this a principle rather than a style rule. **Widening is one-way**: once an item is
`pub`, retracting it is a breaking change, so the cost of guessing wrong is asymmetric — guess small. And
**encapsulation here is module-level, not type-level**: Rust's privacy boundary is the module, so hiding is
achieved by what a module declares and re-exports, not by wrapping fields in accessors.

## Examples

1. **Private by default, gated at the module declaration**
   An item made public "in case someone needs it" is a promise nobody can withdraw, and in this workspace the
   promise outlives the crate: a `pub` item is reachable from the `ffi` module, and anything the C surface
   names is frozen by every homebrew binary that links the override script. When a whole module is internal,
   say so once where it is declared, and let `#[cfg(feature = "ffi")] pub mod ffi;` be the only gate that is
   deliberately open.

```rust
// ❌ Bad — the `mod` line publishes the module and the restriction is repeated on every
// item. A reader has to check each item to learn what escapes, and the next item added
// defaults to `pub` and quietly escapes — here the raw TLS pointer, which a linked
// homebrew can now reach and which no later release can take back.
pub mod tls;

// inside the module
pub(super) fn ipc_buffer_offset() -> usize {}
pub(super) fn command_buffer_words() -> usize {}
pub(super) struct ThreadVars { /* ... */ }
pub struct RawTlsPtr(*mut u8); // ...like this one, exported forever by accident
```

```rust
// ✅ Good — the boundary is stated once, at the declaration. Items inside are plain
// `pub`, so the module reads normally, and nothing leaks past the gate into the crate's
// permanent `__nx_thread_tls__*` surface.
pub(crate) mod tls;

// inside the module
pub fn ipc_buffer_offset() -> usize {}
pub fn command_buffer_words() -> usize {}
pub struct ThreadVars { /* ... */ }
```

2. **Hide the decision, not just the data**
   A type whose surface mirrors its representation has hidden nothing, however private its fields are.

```rust
// ❌ Bad — the fields are private, but every accessor re-exposes the page-slot table one
// method at a time. Replacing the sorted array with a reservation bitmap is a breaking
// change to four signatures, which is exactly what encapsulation was meant to prevent.
pub struct PageReservations {
    slots: [Option<(PageIndex, RegionId)>; MAX_REGIONS],
}

impl PageReservations {
    pub fn slots(&self) -> &[Option<(PageIndex, RegionId)>] {}
    pub fn slots_mut(&mut self) -> &mut [Option<(PageIndex, RegionId)>] {}
    pub fn sort(&mut self) {}
    pub fn binary_search(&self, page: PageIndex) -> Result<usize, usize> {}
}
```

```rust
// ✅ Good — the surface is the question callers actually ask. The slot layout, the
// ordering, and the search strategy are all one module's business, and any of them can
// become a bitmap without a caller noticing.
pub struct PageReservations { /* ... */ }

impl PageReservations {
    pub fn reserve(&mut self, pages: PageCount) -> Option<PageIndex> {}
    pub fn owner(&self, page: PageIndex) -> Option<RegionId> {}
}
```

## Why It Matters

Parnas's argument has not aged: you decompose a system by what is **likely to change**, and each module hides
one of those decisions. A module that reveals its representation has published a decision instead of hiding
one, and every consumer becomes a reason not to revise it.

The cost is asymmetric in a way that makes caution cheap. Keeping an item private costs one `pub(crate)` the
day a sibling needs it. Making it public costs a major version bump the day you want it back, plus a
coordinated edit across every crate that reached for it in the meantime.

## Pragmatism Caveat

`pub(crate)` is a normal, healthy visibility and needs no defence: internal helpers, shared constants, and
cross-module types live there. It is `pub(super)` and `pub(in path)` that are the smell — reach for one and the
question to ask is whether the item belongs in the module it is being shown to.

Test-only access is not a reason to widen. A private item is testable from a `#[cfg(test)]` module inside the
same file or module; widening it so an integration test can reach it exports an implementation detail to every
consumer in order to satisfy one test.

The rule is about what a module **reveals**, not about ceremony. A plain data type with no invariant — a
`#[repr(C)]` request header about to be written into the IPC buffer, a build-time configuration struct — may
have public fields, because no decision is being hidden and accessors would add nothing but noise. Where an
invariant does exist — a raw kernel handle that must never be zero, a word offset that must stay inside the
command buffer — the field stays private and the constructor enforces it.

When you deliberately expose more than a caller strictly needs — a field a sibling crate reads on a hot path, an
internal type a macro must name, a symbol the override script has to alias — say why at the declaration. An
undocumented `pub` on something the crate does not intend to support is indistinguishable from an oversight, and
on the C surface it is indistinguishable from a supported API.

## Checklist

Before committing code, verify:

- [ ] Every item is private unless a specific caller requires otherwise
- [ ] `pub(crate)` is used for cross-module needs inside the crate; `pub` only for items a consumer outside
      the crate calls
- [ ] No `pub(super)` or `pub(in path)` was introduced; where one felt necessary, the module tree was
      reconsidered instead
- [ ] A module that is internal is gated at its `mod` declaration, not by annotating each of its items
- [ ] A type's public surface is the questions callers ask, not a mirror of its representation
- [ ] No item was made public solely so a test could reach it
- [ ] Public fields appear only on types with no invariant to protect
- [ ] Any deliberate over-exposure is documented at the declaration

## References

- [principle-single-responsibility](principle-single-responsibility.md) - Related: A module can only own one
  responsibility if callers cannot reach past its surface
- [principle-open-closed](principle-open-closed.md) - Related: Internals can only be restructured freely while
  nothing depends on them
- [principle-law-of-demeter](principle-law-of-demeter.md) - Related: A surface that reveals its internals is
  what makes reach-through possible
- [principle-type-driven-design](principle-type-driven-design.md) - Related: A private field is what lets a
  constructor be the only way to build a valid value
- [rust-ffi](rust-ffi.md) - Related: The `ffi` module is the one deliberately public, permanent surface, and
  the `ffi` feature is what keeps it out of every other build

## External References

- [On the Criteria To Be Used in Decomposing Systems into Modules — D. L. Parnas](https://dl.acm.org/doi/10.1145/361598.361623)
- [Effective Java, Item 15 — Minimize the accessibility of classes and members](https://github.com/clxering/Effective-Java-3rd-edition-Chinese-English-bilingual/blob/dev/Chapter-4/Chapter-4-Item-15-Minimize-the-accessibility-of-classes-and-members.md)
- [Effective Rust, Item 22 — Minimize visibility](https://effective-rust.com/visibility.html)
- [Information Hiding and Encapsulation — David Gries](https://www.cs.cornell.edu/courses/JavaAndDS/files/infoHiding.pdf)
- [Least Privilege Principle — OWASP](https://owasp.org/www-community/controls/Least_Privilege_Principle)
