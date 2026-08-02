---
name: "principle-law-of-demeter"
description: "Law of Demeter — a unit talks to its immediate collaborators, never through them to reach something further. Load when reviewing call chains, field access patterns, or coupling concerns"
type: "principle"
scope: "global"
---

# Law of Demeter (Principle of Least Knowledge)

## Rule

A function or method may only talk to its immediate collaborators. Do not reach through chains of values to get
at something buried deep in the graph. A method `m` of a type `T` may only call methods on `T` itself (`self`),
on values passed as arguments to `m`, on values `m` created, and on values held in `T`'s own fields.

If you write `a.b().c().do_something()` — or, with public fields, `a.b.c.do_something()` — you are violating
the principle. Stop at `a.b()`: if you need something from `c`, ask `a` (or `b`) to hand you the value, or
accept the value as a parameter.

**Not violations**: chains where every link is the same logical value. Builder chains
(`CmifRequestBuilder::new(CMD_GET_MODEL).with_context(token).add_in_handle(h).build()`), iterator adapters
(`descriptors.iter().filter(..).map(..).sum()`), `Result`/`Option` combinators
(`.map_err(DispatchError::Layout)?`), and matching on an enum a direct collaborator returned
(`let slot = session.acquire_object()?; slot.dispatch(..)`) are not reach-through — the enum is that
collaborator's own return value.

## Examples

1. **Ask the collaborator, don't navigate its internals**
   A service client owns its session, the pointer-buffer sizing rule, and the "is this object still live"
   decision.

```rust
// ❌ Bad — reaches through the client into its session and through the session into its raw
// handle. This caller now depends on the client keeping exactly one session, on the session
// exposing its kernel handle, and on the request being dispatchable without the client's
// pointer-buffer accounting. Any of the three changing breaks it, and nothing in the type
// system says this caller exists.
fn read_display_mode(client: &DisplayClient) -> Result<DisplayMode, DispatchError> {
    let handle = client.session.handle();
    raw_dispatch_in_out(handle, CMD_GET_DISPLAY_MODE, ())
}
```

```rust
// ✅ Good — one call to the immediate collaborator, which answers the question completely.
// This caller knows two things: ask the client for the mode, or report why the object is gone.
fn read_display_mode(client: &DisplayClient) -> Result<DisplayMode, DispatchError> {
    match client.get_display_mode()? {
        ModeReply::Active(mode) => Ok(mode),
        ModeReply::Detached(reason) => Err(DispatchError::ObjectDetached(reason)),
    }
}
```

2. **Receive the value, not the object graph that contains it**
   A page mapper needs a heap base and a page size. It should take exactly those two things.

```rust
// ❌ Bad — the mapper is handed the whole runtime environment and digs for what it needs.
// It is coupled to the environment's shape three levels down, and it cannot be exercised
// without a fully initialized runtime: loader block, address-space probe, service manager
// session and all.
struct PageMapper {
    env: &'static RuntimeEnv,
}

impl PageMapper {
    fn reserve(&self, count: usize) -> Option<NonNull<u8>> {
        let base = self.env.loader.address_space.heap.base_addr; // three levels of reach-through
        // ...
    }
}
```

```rust
// ✅ Good — declare the two values the mapper actually uses.
// The startup code that already holds the environment does the navigation once, at the seam.
// A test constructs this with a scratch region and a literal page size.
struct PageMapper {
    heap_base: NonNull<u8>,
    page_size: usize,
}

impl PageMapper {
    fn reserve(&self, count: usize) -> Option<NonNull<u8>> { /* uses only its own two fields */ }
}
```

## Why It Matters

Reach-through chains turn a private implementation detail into a public contract by accident. When a client
stops holding a bare session and starts holding a domain plus an object id, every caller that read
`client.session.handle()` breaks — and nothing in the type system told you those callers existed. Keeping to
immediate collaborators lets a crate restructure its internals as long as its methods keep their meaning.

The second cost is testability: a type that navigates `env.loader.address_space.heap.base_addr` can only be
exercised once a whole runtime is standing, so it ends up covered by an on-console integration run or not at
all. A type that takes a base pointer and a page size is tested with a scratch region and three lines.

## Pragmatism Caveat

A short reach-through is occasionally the honest choice: navigating a plain data structure you own (a decoded
wire header, a descriptor table you just validated) is reading data, not coupling. The rule targets navigation
through _behavioral_ values that could hide their internals. When you deliberately reach through one, add a
comment explaining why the alternatives (a delegating method on the direct collaborator, or passing the value
in) were rejected. An undocumented violation is always wrong.

## Checklist

Before committing code, verify:

- [ ] No expression navigates two or more levels into another type's fields to reach behavior
- [ ] Functions accept the values they use (a handle, a page size, a buffer) rather than a container to dig
      through
- [ ] Cross-crate access goes through public functions and methods, never through another crate's internal
      sessions, tables, or state
- [ ] Fluent chains on one logical value (builders, iterators, `Result`/`Option` combinators, matched enums) are
      not mistaken for violations
- [ ] Any deliberate reach-through is local and carries a comment with its rationale

## References

- [principle-single-responsibility](principle-single-responsibility.md) - Related: A type that must be navigated
  deeply usually owns too much
- [principle-inversion-of-control](principle-inversion-of-control.md) - Related: Injecting the value you need is
  the standard cure for reach-through
- [principle-type-driven-design](principle-type-driven-design.md) - Related: Returning an enum lets a
  collaborator answer a question completely instead of exposing its internals

## External References

- [Law of Demeter — Principle of Least Knowledge](https://dev.to/dazevedo/law-of-demeter-principle-of-least-knowledge-35l2)
