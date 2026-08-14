---
name: "pattern-trait-sealed"
description: "Sealing a trait against downstream impls with a `_priv::Sealed` supertrait, and where the `_priv` module sits. Load when a trait's implementors must stay closed to this crate, or when naming the module that holds a Sealed supertrait"
type: "core"
scope: "global"
---

# Sealed Traits (Closing a Trait to Downstream Implementations)

## Rule

A public trait is open: any downstream crate may implement it for its own type. **When the crate reasons about
the set of implementors, that set must be closed**, and sealing is how it is closed. Seal a trait whose
implementors the crate enumerates, matches on, or relies on being exactly the ones it wrote.

The idiom is a supertrait the outside world cannot name. Written in the order it is declared:

1. The public trait takes that supertrait: `pub trait Role: _priv::Sealed {}`.
2. The permitted types follow, then their `impl Role for T {}`.
3. A private `_priv` module closes the enclosing module, holding `pub trait Sealed {}` and one
   `impl Sealed for T {}` per permitted type.

A downstream crate cannot path to `_priv::Sealed`, because the module is private. It therefore cannot satisfy
the supertrait, so it cannot implement the public trait, and the compiler says so at the attempted `impl`. The
trait stays callable and nameable from outside; only implementing it is closed.

**The module is named `_priv`, not `sealed`.** [rust-mods-naming](rust-mods-naming.md) forbids naming a module
for a mechanism, and `sealed` is exactly that: it describes the trick being played rather than a subject the
crate has. It also reads in paths and in rustdoc as though it were a domain module, which it is not. A leading
underscore says the opposite, and says it in the one place a reader looks: this is an implementation detail
with no meaning of its own. Any `_`-prefixed name satisfies the rule; `_priv` is the default, and a crate
should not carry two spellings.

**`_priv` goes last.** It is declared after the trait it seals, after the types, and after every impl of the
public trait. The order in the enclosing module is: the public trait, the types, the public trait's impls,
then `_priv` holding `Sealed` and its impls. Mechanism sits below subject, so a reader meets what the module
is for before meeting the machinery that closes it.

Keep `_priv` private. It widens to `pub(crate)` only when a macro or a sibling module has to write the
`Sealed` impl, which still excludes every downstream crate and so still seals the trait.

## Examples

1. **Closing a trait whose implementors the crate relies on**
   A trait that exists to name a fixed set of types is a promise the crate makes to itself. Left open, the
   promise is only as good as the absence of a downstream `impl`.

```rust
// ❌ Bad — the crate treats these two as the only implementors, but nothing says so. A
// downstream crate implements Role for its own type, passes it in, and reaches code written
// on the assumption that a Role is one of the two the crate defined.
pub trait Role {}

pub struct Client;

pub struct Server;

impl Role for Client {}

impl Role for Server {}
```

```rust
// ✅ Good — Role carries a supertrait that only this crate can satisfy, so the implementor
// set is exactly the two below. A downstream `impl Role for Theirs` fails with "the trait
// bound Theirs: _priv::Sealed is not satisfied", pointing at the impl that is not allowed.
pub trait Role: _priv::Sealed {}

pub struct Client;

pub struct Server;

impl Role for Client {}

impl Role for Server {}

mod _priv {
    pub trait Sealed {}

    impl Sealed for super::Client {}

    impl Sealed for super::Server {}
}
```

2. **Where the module sits, and what it is called**
   The sealing machinery is the least interesting thing in the module, so it is written where the least
   interesting thing belongs. A name that describes the trick, placed first, inverts that.

```rust
// ❌ Bad — the module is named for the mechanism and declared before the trait it serves, so
// the first thing the reader meets is the trick rather than the subject. `sealed::Sealed`
// also reads in rustdoc and in error messages as though `sealed` were a concept this crate
// has, and the Sealed impls are interleaved with the public trait's.
pub mod state {
    mod sealed {
        pub trait Sealed {}
    }

    pub trait Kind: sealed::Sealed {}

    pub struct Open;

    pub struct Closed;

    impl sealed::Sealed for Open {}

    impl Kind for Open {}

    impl sealed::Sealed for Closed {}

    impl Kind for Closed {}
}
```

```rust
// ✅ Good — subject first, mechanism last. The trait and its types lead, the public trait's
// impls follow, and `_priv` closes the module with Sealed and its impls together. The
// underscore marks it as an implementation detail rather than a concept.
pub mod state {
    pub trait Kind: _priv::Sealed {}

    pub struct Open;

    pub struct Closed;

    impl Kind for Open {}

    impl Kind for Closed {}

    mod _priv {
        pub trait Sealed {}

        impl Sealed for super::Open {}

        impl Sealed for super::Closed {}
    }
}
```

## Why It Matters

**An open trait makes the implementor set a guess.** Code that matches on implementors, or that is correct only
because the implementors are the ones the crate wrote, has an assumption the type system is not enforcing.
Sealing turns it into something the compiler checks, at the one place it can be violated.

**It keeps the trait an implementation choice rather than a published contract.** An unsealed public trait is
API: adding a method to it, or a supertrait, breaks every downstream implementor and is a major version bump.
Sealed, the trait can gain methods freely, because the crate is the only implementor. That is the difference
between a trait the crate may still evolve and one it has frozen by accident.

**The diagnostic names the rule.** `the trait bound Theirs: _priv::Sealed is not satisfied` lands on the
attempted `impl`, and the unnameable path in the message is itself the explanation: whatever `_priv::Sealed`
is, the caller has no way to write it, so this impl was never available.

## Pragmatism Caveat

**An extension trait meant for downstream implementation must not be sealed.** If the point of the trait is
that callers implement it for their own types, sealing defeats the trait. Sealing is for a closed set, not for
every public trait.

**A `pub(crate)` or private trait needs no seal.** Visibility already closes it; adding a supertrait adds a
module and a bound to say what `pub(crate)` said. Seal a trait when it is public and must still be closed.

**One or two implementors that nothing reasons about may not be worth it.** A public trait with two impls that
the crate never enumerates and never matches on can stay open; the cost of a downstream impl is nothing the
crate depends on. Seal when a wrong implementor would break an invariant, not on reflex.

## Checklist

Before committing code, verify:

- [ ] Every public trait whose implementor set the crate relies on carries a `Sealed` supertrait
- [ ] The module holding `Sealed` is named `_priv`, or at least begins with `_`, and never `sealed`
- [ ] `_priv` is declared last in its enclosing module, after the public trait, the types, and the public
      trait's impls
- [ ] The `Sealed` impls sit with the trait inside `_priv`, not interleaved with the public trait's impls
- [ ] `_priv` is private, widening to `pub(crate)` only where a macro or sibling module writes the impl
- [ ] No trait intended for downstream implementation is sealed
- [ ] The crate uses one spelling of the module name throughout

## References

- [principle-information-hiding](principle-information-hiding.md) - Foundation: The sealed supertrait is
  visibility used to close a set, revealing the trait while withholding the ability to implement it
- [rust-mods-naming](rust-mods-naming.md) - Related: Why the module is `_priv` and not `sealed`; a module is
  not named for a mechanism
- [pattern-builder-fluent](pattern-builder-fluent.md) - Related: Seals one marker trait per required field, so
  a field's positions stay closed to the two the builder defines
- [pattern-typestate](pattern-typestate.md) - Related: Where a state set is closed by concrete types instead,
  and needs no seal

## External References

- [Rust API Guidelines — Sealed Traits](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed)
- [Future Proofing — The Rust API Guidelines Book](https://rust-lang.github.io/api-guidelines/future-proofing.html)
