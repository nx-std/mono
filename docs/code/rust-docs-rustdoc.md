---
name: "rust-docs-rustdoc"
description: "Rustdoc: crate, module and item levels; mandatory # Panics and # Errors; never # Returns, # Arguments or # Examples. Load when writing a /// or //! block"
type: "core"
scope: "global"
---

# Rustdoc

Rustdoc is the consumer's channel ([rust-docs](rust-docs.md)): the hover text at every call site and the
rendered API page. It carries contracts and domain, in the timeless present. History, process, and the
argument for an exception belong in a `//` comment ([rust-docs-comments](rust-docs-comments.md)).

## 1. Three Levels, Three Questions

Each level answers a different question, and a block that answers the wrong one is in the wrong place.

| Level      | Written as            | Answers                                                        |
|------------|-----------------------|----------------------------------------------------------------|
| **Crate**  | `//!` in `src/lib.rs` | What is this crate for, and why would I depend on it?          |
| **Module** | `//!` at the top of a module | Why does this module exist, and what does it defend against? |
| **Item**   | `///` on the item     | What does calling this give me, and what must I uphold?        |

```rust
// ✅ Good — the crate root, in the terms of someone deciding whether to depend on it.
//! Kernel wait-object multiplexing for Horizon OS.
//!
//! Turns a set of borrowed kernel handles into a single blocking wait that
//! reports which handle signalled, with cancellation delivered through a
//! dedicated event handle rather than a timeout.

// ✅ Good — the module says why it exists and what it protects, which is the fact
// the next editor most needs and no signature carries.
//! Reservation planning: where a page reservation is cut into map requests.
//!
//! Requests are cut on 2 MiB block boundaries only. The kernel maps a block as a
//! unit, so a seam inside one leaves every page after it unmapped.
```

A module `//!` block is where an invariant that the module relies on, or upholds, is stated. That is a fact
about this code, so it belongs here rather than in a rule document: the reader who needs it is editing this
file.

## 2. Say the Contract, Briefly

Every public item carries a description: one or two sentences, contract first. Non-obvious behavior — edge
cases, ordering, atomicity — earns another sentence. Nothing else does.

```rust
// ❌ Bad — a paragraph per parameter, restating the signature in prose. It says
// nothing a reader cannot see, and it goes stale the moment an argument moves.
/// Maps a shared memory object into the current process
///
/// # Arguments
/// * `handle` - The shared memory handle returned by the create call
/// * `addr` - The address at which the mapping should be placed
/// * `size` - The number of bytes to map
pub unsafe fn map_shared(handle: SharedHandle, addr: *mut u8, size: usize) -> Result<(), MapError> {}

// ✅ Good — the contract, then the obligation the signature cannot carry: a raw
// pointer says nothing about who owns the range it points into.
/// Map `size` bytes of `handle` at `addr`. All-or-nothing: on failure the address
/// space is left exactly as it was.
///
/// # Safety
///
/// `addr` must be page-aligned and name a reserved, unmapped range of at least
/// `size` bytes that the caller owns for as long as the mapping lives.
pub unsafe fn map_shared_at(handle: SharedHandle, addr: *mut u8, size: usize) -> Result<(), MapError> {}
```

`# Arguments` is never written. A parameter that needs explaining needs a better name or a type that carries
the meaning ([pattern-newtype](pattern-newtype.md)).

## 3. Sections That Are Never Written

- **`# Returns`** — the return type says it. A sentence restating `-> MemoryPermission` as "returns the current
  memory permission" is noise that survives until the signature changes and then becomes wrong.
- **`# Arguments`** — see above.
- **`# Examples`** — usage examples are what tests are for, and a hand-written example compiles only until it
  doesn't. The exception is a doctest that **pins a contract**: [rust-fmt](rust-fmt.md) requires one on every
  formatting impl, because the exact rendering is the promise and an assertion is the only form of it that
  cannot drift. A doctest asserting a contract is required; one demonstrating typical usage is not written.
  Be honest about the reach of that exception here: these crates are `no_std` and target
  `aarch64-nintendo-horizon.json`, so a doctest only runs where the item builds and runs on the host.
  Most of this workspace's surface calls SVCs, so a code block in rustdoc is the rare case, not the habit.

Every one of those bans rests on the same premise: the reader is holding the signature. The return type is
visible, the parameter names are visible, and a section restating them adds a second place for the same fact
to rot.

**Where the reader has no signature, the premise fails and the sections come back.** An item whose rustdoc is
**product surface** — read by someone invoking it from C across the linker-override boundary rather than
calling it from Rust — has a reader with no Rust types to consult and no parameter names to improve. An
`extern "C"` symbol called from libnx-era C as `__nx_virtmem_reserve(len, align)` is the case this workspace
has. There, `# Arguments` is the only place a parameter's units and ownership can live, and `# Returns` the
only place the null-on-failure convention can. Such items are governed by their own crate-scoped FFI rule,
which requires these sections and is right to.

The test is not "is this public" but **"can the reader see the signature?"** For ordinary Rust items the
answer is yes, and the bans stand.

```rust
// ❌ Bad — a usage demo, duplicating a test and rotting on the next signature change.
// `/// Validate a service name against the port naming rules.` is the whole doc needed.
/// Validates a service name
///
/// # Examples
/// ```
/// assert!(validate_service_name("fsp-srv").is_ok());
/// ```
pub fn validate_service_name(name: &str) -> Result<(), NameError> {}
```

## 4. Sections That Are Mandatory

**`# Panics`** — any function that can panic says so, and names the condition. That includes a function whose
body reaches an `unwrap`, an `expect`, a `panic!`, an indexing operation on a data-derived index, or a call to
something that panics. The stakes are higher here than in a hosted crate: every crate builds with
`panic = "abort"`, so a reachable panic takes the process down rather than unwinding, and a caller who lets one
cross an `extern "C"` boundary has no recovery to write. The section is the only warning they get.

```rust
// ✅ Good — the condition, not the mechanism.
/// The highest page covered by `regions`.
///
/// # Panics
///
/// Panics if `regions` is empty.
pub fn max_page(regions: &[PageRange]) -> PageIndex {}
```

The section documents a panic a caller can actually reach. An `unwrap` or `expect` that a code invariant makes
unreachable is the exception: it carries a `// SAFETY:` comment on its statement, stated where the call sits,
and gets **no** `# Panics` section — documenting a panic that cannot occur would mislead the caller. That
comment and the proof behind it are owned by [rust-errors-handling](rust-errors-handling.md).

**`# Errors`** — any fallible public function describes what its failures mean. The variants themselves are
documented on the error type, which [rust-errors-reporting](rust-errors-reporting.md) governs; this section
says which of them a caller can expect here, and what they imply.

**`# Safety`** — required on every `unsafe` item, including a validation-skipping constructor whose misuse is
undefined behaviour. A `_unchecked` constructor that only skips a check whose violation misbehaves without
being unsound stays safe and states its precondition in prose instead; that split, and the call-site comment
each half carries, are owned by [rust-fn-unchecked](rust-fn-unchecked.md). In this workspace the
same section is what every `unsafe fn` owes its caller, because most of the public surface wraps an SVC or a
raw pointer: it states the obligations the compiler cannot — pointer validity and provenance, page alignment
and size, and who owns a kernel handle and until when — as in the mapping example in §2.

Two adjacent requirements live elsewhere and are not restated here: the documentation template for error enums
and their variants ([rust-errors-reporting](rust-errors-reporting.md)), and the `Cargo.toml` feature comment
([rust-crates](rust-crates.md)).

## Checklist

Before committing code, verify:

- [ ] The crate root `//!` says what the crate is for, in the terms of someone deciding whether to depend on it
- [ ] Every module has a `//!` block saying why it exists and stating any invariant it relies on or upholds
- [ ] Every public item has a one-or-two-sentence description, contract first
- [ ] No `# Returns`, `# Arguments`, or usage-demo `# Examples` section was added
- [ ] Any doctest present pins a contract rather than demonstrating typical usage
- [ ] Every function that can panic has a `# Panics` section naming the condition
- [ ] A provably-unreachable `unwrap`/`expect` has a `// SAFETY:` comment on its statement and no `# Panics`
      section
- [ ] Every fallible public function has an `# Errors` section saying what its failures mean
- [ ] Every `unsafe` item has a `# Safety` section; a safe validation-skipping constructor states its
      precondition in prose instead

## References

- [rust-docs](rust-docs.md) - Extends: The intent rule, the audience routing, and the shared voice, applied to
  the consumer's channel
- [rust-docs-comments](rust-docs-comments.md) - Related: The editor's channel, and the leading comment that
  sits after a doc comment
- [rust-fn-unchecked](rust-fn-unchecked.md) - Related: Owns which unchecked constructors are `unsafe`, and the
  call-site comment each half carries
- [rust-errors-handling](rust-errors-handling.md) - Related: Owns the `// SAFETY:` comment on a
  provably-unreachable unwrap/expect, which stands in for the omitted `# Panics` section
- [rust-fmt](rust-fmt.md) - Related: The one place a doctest is mandatory, because the rendering is the contract
- [rust-errors-reporting](rust-errors-reporting.md) - Related: Owns the error enum and variant documentation template
- [rust-crates](rust-crates.md) - Related: Owns `Cargo.toml` feature documentation
