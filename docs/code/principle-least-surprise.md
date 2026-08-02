---
name: "principle-least-surprise"
description: "Principle of Least Surprise — code behaves as its name and shape predict; deviations are documented. Load when naming things, designing constructors, or reviewing an API surface"
type: "principle"
scope: "global"
---

# Principle of Least Surprise (Follow Rust Idioms and Conventions)

## Rule

Code must behave the way a reader predicts from its name, signature, and shape. A name is a contract: what a
function returns, what it touches, and whether it can fail should be guessable without opening it. **The Rust
standard library is the primary reference**: where `std` establishes a pattern for naming, trait usage, or
method semantics, follow it. The conventions here:

1. **Names follow the standard library.** Which constructor, which prefix, which receiver, and what each
   promises about cost and ownership are settled by `std` and collected in the `rust-fn` rule document; this
   document owns the cases that are about behavior rather than naming.
2. **Construction reveals its cost.** A value is never half-initialized: anything that issues an SVC, opens a
   session, or maps pages says so in its name.
3. **Teardown**: the inverse of a named constructor is `close()` or `disconnect()`. Pick one per type, stay
   consistent, and make it safe to call twice.
4. **Paired names**: if the codebase uses `map`/`unmap`, do not introduce `attach`/`release`. The inverse of
   `acquire` is `release`, not `drop_ref`. The shape the pair shares is owned by `principle-symmetry`; this
   rule owns its vocabulary.
5. **Parameters**: more than two or three related inputs go in a config struct or a builder, never a positional
   `bool`. `map(dst, src, size, true, false)` cannot be reviewed.
6. **No hidden effects**: a function named for a computation does not issue an SVC, change a page mapping, or
   mutate global state. Where a lookup and an effect both exist, they are two functions.

## Examples

1. **`new` does not issue an SVC; acquiring construction is a named constructor**
   `new` cannot fail, so a type that must open a session before it is usable cannot be built by one.

```rust
// ❌ Bad — `new` defers the session handshake and returns immediately. Every method
// then needs an "is the port open yet?" guard, and a caller who forgets one
// dispatches a command on INVALID_HANDLE and gets a bare 0xF201 back.
impl ClkRstClient {
    pub fn new(port: ServiceName) -> Self {
        Self { port, session: OnceCell::new() }
    }
    pub fn core_clock_rate(&self) -> Result<ClockRate, DispatchError> {
        let session = self.session.get().ok_or(DispatchError::NotConnected)?; // ...on every method
    }
}
```

```rust
// ✅ Good — the named constructor performs the connect and hands back a client
// that is, by construction, usable. No method needs a readiness guard.
impl ClkRstClient {
    pub fn connect(port: ServiceName) -> Result<Self, ConnectError> {
        let session = SessionHandle::connect_to_named_port(port)?;
        Ok(Self { port, session })
    }
    pub fn core_clock_rate(&self) -> Result<ClockRate, DispatchError> {}
}
```

2. **Separate the lookup from the effect**
   Only one of "which region covers this address" and "unmap it" issues an SVC.

```rust
// ❌ Bad — a name that reads like a query, a body that unmaps. A caller probing
// "is this address in a reserved region?" inside a filter unmaps every region it
// walks, and the next read of an already-live page faults with a data abort.
pub fn region_for(addr: PageAddr) -> Option<RegionId> {
    let spec = REGIONS.iter().find(|r| r.contains(addr))?;
    spec.unmap_tail(); // hidden effect: an SVC
    Some(spec.id)
}
```

```rust
// ✅ Good — the query is pure; the effect names the SVC it issues.
pub fn region_for(addr: PageAddr) -> Option<&'static RegionSpec> {
    REGIONS.iter().find(|r| r.contains(addr))
}

/// Unmap the region's trailing pages. Returns the region that was unmapped.
pub fn unmap_region_tail(addr: PageAddr) -> Result<Option<RegionId>, UnmapError> {
    let Some(spec) = region_for(addr) else { return Ok(None) };
    spec.unmap_tail()?;
    Ok(Some(spec.id))
}
```

## Why It Matters

Every broken convention forces a reader to open the implementation, and across a codebase that cost is paid
mostly in bugs: a caller who assumes `as_bytes()` is a borrow calls it once per dispatched command.

Consistency also compounds. Because every handle-owning type here is built by a named fallible constructor
(`open`, `connect`, `map`), `Type::new(..)` tells a reviewer the type owns no kernel resource — or that
something is wrong. Standard traits buy composition on top: `?`, `.parse()`, and `.into()` all compose with
`FromStr`, `From`, and `TryFrom`, while a custom constructor requires custom glue at every boundary.

## Pragmatism Caveat

A domain term beats a convention when it is genuinely clearer. `dispatch` on a command builder beats
`into_dispatched` even though it consumes `self`, because dispatch is the established verb. Prefer the domain
word only when it is _more_ predictable, not merely more clever. Some deviations are imposed from outside: the
C ABI an `extern "C"` entry point replaces dictates its own name and argument order, so match the foreign
convention at the FFI boundary and the workspace convention everywhere else.

When you deviate deliberately — a method that never fails, a fire-and-forget signal, a name the ABI forced —
say why in a doc comment at the declaration. An undocumented deviation is always wrong; the next reader cannot
tell it from a mistake.

## Checklist

Before committing code, verify:

- [ ] Names follow the standard library's vocabulary; the concrete forms are checked against `rust-fn`
- [ ] No value is observable half-initialized; anything that issues an SVC or opens a session says so in
      its name
- [ ] Every named constructor that acquires a handle or a mapping has a matching `close()`/`disconnect()`,
      and calling it twice is safe
- [ ] No function named for a query performs an effect; lookup and effect are separate functions
- [ ] More than two or three related parameters are a config struct or a builder; no positional `bool`
- [ ] Any intentional deviation (a domain verb, an ABI-imposed shape) is documented at the declaration

## References

- [principle-type-driven-design](principle-type-driven-design.md) - Related: The same discipline, for types
- [principle-validate-at-edge](principle-validate-at-edge.md) - Related: `FromStr`/`TryFrom` parse at the edge
- [principle-idempotency](principle-idempotency.md) - Related: `connect`/`close` are safe to call twice
- [principle-single-responsibility](principle-single-responsibility.md) - Related: A type that cannot be named
  in one sentence cannot have a predictable API
- [principle-symmetry](principle-symmetry.md) - Related: A prediction is only available where the same idea
  keeps the same shape

## External References

- [Rust API Guidelines — Naming](https://rust-lang.github.io/api-guidelines/naming.html)
- [Principle of Least Surprise (principles-wiki.net)](https://principles-wiki.net/principles:principle_of_least_surprise)
- [The Principle of Least Astonishment](https://dev.to/notmattlucas/the-principle-of-least-astonishment-3f9k)
- [What is the Principle of Least Astonishment?](https://softwareengineering.stackexchange.com/a/187462)
