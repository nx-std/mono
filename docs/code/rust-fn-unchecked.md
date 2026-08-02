---
name: "rust-fn-unchecked"
description: "Unchecked constructors that skip FromStr/TryFrom: when the bypass stays a safe fn, when it is unsafe with # Safety and a // SAFETY: call site. Load when adding or reviewing a *_unchecked call"
type: "core"
scope: "global"
---

# Unchecked Constructors

## 1. Validation Lives in `FromStr` and `TryFrom`

A newtype carries a proof: a value of this type has been checked. That check has exactly one home — `FromStr`
for string input, `TryFrom` for everything else. Every other constructor delegates to it or bypasses it, and
the bypasses are this document's subject.

An unchecked constructor does not weaken the invariant. It **moves the obligation to prove it** from the type
to the caller — sound only when the caller genuinely holds the proof, and only visible when the call site says
so.

`_unchecked` in a name says a check was skipped. It does **not**, on its own, say the function is `unsafe fn`.
Which it is depends on one question, asked of the invariant rather than of the constructor:

**If the caller breaks the precondition, does the program exhibit undefined behavior, or does it merely
misbehave?**

- **Misbehaves → the constructor stays a safe `fn`.** A service name longer than the wire field, a command id
  outside the range a sysmodule documents, a page count that overflows a request: the worst outcome is a
  rejected IPC request, a `ResultCode` nobody expected, or a wrong number in a log. The obligation is real and
  is documented in prose, but the compiler's `unsafe` keyword is not the tool for it, and spending it here
  makes it worthless where it matters.
- **Undefined behavior → the constructor is `unsafe fn`.** A raw pointer nothing has validated, a `zerocopy`
  cast over a buffer that may be shorter or less aligned than the target type, a handle asserted live that the
  kernel may already have closed, a `'static` lifetime asserted over a mapping that can be torn down. Here
  misuse is not a wrong answer; it is a fault, a use-after-close, or a silently corrupted address space.

The rest of this document is that split applied to declarations ([§4](#4-the-declaration-states-what-the-caller-must-uphold))
and to call sites ([§5](#5-the-call-site-records-why-the-proof-exists)).

## 2. When an Unchecked Constructor Is Warranted

There are three honest reasons, and they share a shape: the value's invariant was established somewhere the
type cannot see.

1. **Reading back a field the kernel or a sysmodule already guarantees.** Re-validating a value the SVC ABI
   documents as in-range pays for a check the producer already made, and turns an ABI mismatch into a decode
   error at a random reply site rather than at the boundary that decoded the reply.
2. **Re-wrapping a value taken from an already-valid instance.** Borrowing the bytes out of an existing
   newtype, or converting between the borrowed and owned forms of one, cannot produce an invalid value.
3. **Literals in tests**, where the value is visible in the same expression and the check would obscure the
   test's actual subject.

Anything else is validation avoidance. If the reason is "this is on the IPC hot path", measure first: checking
a value that is already correct is rarely what a profile blames.

## 3. Naming and Visibility

The name states both the input and the bypass, so a reader never has to open the definition to know a check was
skipped:

- `from_bytes_unchecked` / `from_str_unchecked` for the borrowed and owned forms of a fixed-width name newtype
- `from_raw_unchecked`, `from_u32_unchecked` for typed sources

Keep the constructor as narrow as its callers allow. A `pub(crate)` unchecked constructor cannot be reached by
a consumer who does not hold the proof; a `pub` one is part of the API and every downstream crate — every
`nx-service-*` built on it — inherits the obligation.

## 4. The Declaration States What the Caller Must Uphold

The declaration states what the caller must guarantee in terms of the invariant, not "the value must be
valid", which says nothing. The same constructor written as a plain `new` hides the bypass twice over: the
name does not admit it, and a caller reasonably assumes it validates, because every other constructor does.

**The non-UB case is a safe `fn`.** Its obligation goes in an ordinary prose paragraph — "the caller must
ensure …" — and it gets **no** `# Safety` section, because there is no memory-safety contract to state and a
`# Safety` block would tell the reader to search for a soundness argument that does not exist.

```rust
// ✅ Good — the name admits the bypass and the prose names the obligation, with no `# Safety`
// section to imply a soundness contract the type does not have. Breaking the precondition
// costs a rejected `Connect` request, not a fault.
impl ServiceName {
    /// Wrap eight raw name bytes without checking them.
    ///
    /// The caller must ensure the bytes hold a NUL-padded ASCII service name of one to
    /// eight characters, as `sm` expects it on the wire. This constructor performs no
    /// validation; an ill-formed name reaches the server and is answered with
    /// `ResultCode::NOT_REGISTERED`.
    pub fn from_bytes_unchecked(raw: [u8; 8]) -> Self {
        Self(raw)
    }
}
```

**The UB case is an `unsafe fn` with a `# Safety` section** ([rust-docs-rustdoc](rust-docs-rustdoc.md#4-sections-that-are-mandatory)).
The section names the state of the world the caller is asserting, not the shape of the argument.

```rust
// ✅ Good — `unsafe` is spent where misuse is a real fault: the wrapper closes the handle on
// drop, so a stale or borrowed value closes a descriptor another owner is still using.
impl SessionHandle {
    /// Adopt a raw kernel handle as an owned session.
    ///
    /// # Safety
    ///
    /// `raw` must name a live session handle that the kernel has not closed, and the caller
    /// must be its sole owner: this wrapper closes the handle on drop, so a second owner
    /// observes a closed descriptor and a later `send_sync_request` faults on it.
    pub unsafe fn from_raw_unchecked(raw: u32) -> Self {
        Self(raw)
    }
}
```

A type that offers either kind of unchecked constructor also says so at the module level: a `//!` block stating
which invariants the type maintains, and where validation actually happens.

## 5. The Call Site Records Why the Proof Exists

A call site names **why the invariant already holds** here. "This is fine", "we know it's valid", and a
restatement of the function's own docs are all failures — they record that someone thought about it, not what
they concluded. Without the note, a later change to the invariant has no searchable list of sites to
re-examine, and the invalid value enters the domain from the one path nobody audits.

The marker follows the split. A call to an `unsafe` unchecked constructor sits in an `unsafe` block under a
`// SAFETY:` comment. A call to a safe one gets a plain `//` comment justifying the bypass
([rust-docs-comments](rust-docs-comments.md#2-comments-justify-decisions)) and **never** a `// SAFETY:` marker:
that marker is the searchable index of the workspace's soundness obligations, and diluting it with
non-UB preconditions is what makes an audit of it useless.

```rust
// ✅ Good — each note names the fact that makes the bypass correct, and the markers stay
// distinct, so grepping `SAFETY:` still returns only the sites where misuse is UB.
impl SmClient {
    pub fn connect(&self, name: &ServiceName) -> Result<Session, ConnectError> {
        // The bytes come out of an already-validated `ServiceName`, so re-checking them
        // would re-derive a proof this value carries by construction.
        let request = ConnectRequest::new(ServiceName::from_bytes_unchecked(name.as_bytes()));

        let raw = self.dispatch(request)?.into_raw_handle();
        // SAFETY: `sm` returns a freshly created session handle owned by this process, and
        // `dispatch` yields it exactly once, so this wrapper is its sole owner.
        Ok(Session::new(unsafe { SessionHandle::from_raw_unchecked(raw) }))
    }
}
```

**Test code is the one exception.** A call in a `#[cfg(test)]` module needs no justification comment: the value
is a literal in the same expression, visible to anyone reading the assertion, and the comment would bury the
thing the test is actually about. An `unsafe` block still needs its `// SAFETY:` comment even there, because
UB in a test is UB.

`From`/`Into` impls are the other common site, and the rule does not soften there: a conversion that re-wraps
an already-valid value writes out why the input already upholds the invariants rather than relying on the
reader to reconstruct it.

## 6. Where Unchecked Construction Is Never Acceptable

Anything that arrives from outside is parsed, never wrapped — and this holds for both categories, since a
rejected request that surfaces three sysmodules away is no easier to diagnose than a fault. A name passed in
over an FFI boundary from C, a field read out of an untrusted NRO's asset header, a value taken from a mapped
buffer another process can write: wrapping one asserts the fact the boundary exists to establish, in a place
with no error path ([principle-validate-at-edge](principle-validate-at-edge.md)).

```rust
// ❌ Bad — a C caller's name wrapped, not parsed. A nine-byte name silently truncates to
// eight, and the first sign of it is a session opened against the wrong service.
let name = ServiceName::from_bytes_unchecked(raw_name);
```

```rust
// ✅ Good — the boundary parses, and the caller gets a typed error to turn into a ResultCode.
let name = ServiceName::try_from(raw_name).map_err(ConnectError::from)?;
```

## Checklist

Before committing code, verify:

- [ ] The type's validating constructor is `FromStr` or `TryFrom`, and it is the only place the invariant is
      checked
- [ ] Every constructor that skips validation has `_unchecked` in its name
- [ ] The constructor is `unsafe fn` if and only if breaking its precondition is undefined behavior — a raw
      pointer, an unchecked cast, a handle asserted live, an asserted lifetime
- [ ] An `unsafe` unchecked constructor carries a `# Safety` section; a safe one states its precondition in
      prose and carries no `# Safety` section
- [ ] The constructor's visibility is as narrow as its callers allow
- [ ] Every call site outside `#[cfg(test)]` carries a note immediately above it naming why the invariant
      already holds: `// SAFETY:` for the `unsafe` case, a plain `//` comment for the safe one
- [ ] No `// SAFETY:` comment sits above a safe unchecked call, and none merely restates the function's docs or
      asserts that the value is valid
- [ ] No value from an FFI caller, an untrusted asset header, or a shared buffer is wrapped rather than parsed
- [ ] The module documents which invariants the type maintains and where validation actually occurs

## References

- [principle-validate-at-edge](principle-validate-at-edge.md) - Foundation: Where the invariant is established,
  and why a kernel-produced value is a different case from an FFI caller's buffer
- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: The newtype carries the proof
  that an unchecked constructor asserts
- [principle-least-surprise](principle-least-surprise.md) - Foundation: `FromStr`/`TryFrom` are the constructors a
  reader expects; a bypass must announce itself in its name
- [rust-docs-rustdoc](rust-docs-rustdoc.md) - Related: The rustdoc sections a `# Safety` block sits among, and
  module-level invariant docs
- [rust-docs-comments](rust-docs-comments.md) - Related: The voice of the plain `//` comment that justifies a
  safe unchecked call
