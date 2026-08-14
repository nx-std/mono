---
name: "pattern-builder-fluent"
description: "Fluent typestate builder: a sealed Set/Unset marker module per required field, a terminal method gated on all-Set, and an always-compiled exercise of the chain. Load when a builder has three or more independent required fields, or when a forgotten field must fail to compile"
type: "core"
scope: "global"
---

# Fluent Typestate Builder (Compile-Time Construction Completeness)

## Rule

When a builder has several **independent required fields** and a forgotten one must fail to compile rather than
come back as an error value, give the builder **one type parameter per required field**. Each parameter holds
its own field's `Set` or `Unset` marker. A required setter is reachable only while its field is `Unset`,
consumes `self`, and returns the builder with that one parameter advanced. The terminal method exists only
where every parameter is `Set`, so an incomplete builder has no way to finish.

Four requirements carry the pattern, and it is not the pattern without all four.

**1. Each required field owns its marker pair, in its own module.** A field named `payload` owns
`payload::Set`, `payload::Unset`, and a `payload::State` trait that only those two implement; the builder's
payload parameter is bounded by `payload::State` and by nothing else. The module form removes the name suffix
a shared vocabulary would need, and it is what makes a misplaced marker a compile error: with one `Set`/`Unset`
pair shared by every parameter, each parameter accepts every marker, so a setter whose return type puts a
marker in the **wrong parameter** is still well-typed. Per field, it is not: that setter fails to compile with
`the trait bound payload::Set: reply::State is not satisfied`, which names both the marker and the slot it
does not belong in.

**The marker modules are declared inline, in the module that defines the builder**, not as module files of
their own. They exist to serve one type, they are a handful of lines each, and putting them in separate files
separates a field's markers from the only builder that can use them. They sit after the builder's impls: the
subject first, then the vocabulary its parameters are drawn from.

Each module seals its own `State` trait, with its own `_priv` module and the ordering that document sets out
([pattern-trait-sealed](pattern-trait-sealed.md)) — the public trait, the two markers, the `State` impls, then
`_priv` last holding `Sealed` and its impls. A field module seals its own trait rather than reaching back into
its parent for a shared `Sealed` supertrait ([rust-mods-graph](rust-mods-graph.md)).

**2. The markers live in one `_`-prefixed `PhantomData` tuple.** One `_state: PhantomData<(C, P, R)>` field,
not one `PhantomData` per parameter, so advancing a field rewrites one field rather than `N`. A private
`retype` helper carries the data across a transition, so no setter restates the struct literal.

The name is `_`-prefixed because it is not a field in the sense the others are: it holds nothing, it is never
read, and it exists only to make the type parameters count towards variance and drop. The underscore says that
where the reader meets it, the same way it does for the `_priv` module that seals each marker trait
([pattern-trait-sealed](pattern-trait-sealed.md)).

**3. Setters are declared on the marker they consume.** A required setter lives on the impl block where its own
parameter is `Unset` and returns the builder with that parameter `Set`; optional setters live on one blanket
impl bounded by every field's `State` trait and return `Self`; the terminal method is declared on the all-`Set`
instantiation and nowhere else.

**4. Something always compiled exercises the whole chain.** Per-field markers close the cross-slot error. They
do **not** close the wrong-field error: a setter that leaves its own parameter alone and advances a different
one is internally consistent and compiles. Per-field markers alone do not make the setters self-checking. The
only thing that catches it is calling every required setter once and then the terminal method: if `payload()`
never advances the payload parameter, the chain never reaches all-`Set`, the terminal method does not exist,
and the exercise fails to compile.

That exercise has to live somewhere that is always compiled. A crate built with `test = false` and
`doctest = false` has neither a unit test nor a doctest to host it, so the requirement is on ordinary code: a
real constructor that runs the chain internally and returns the finished value is one way, a `pub(crate)`
convenience the crate's own callers go through is another, and a unit test or doctest serves where the crate
is built with them. What matters is not which mechanism: it is that deleting a setter's advance breaks the
build rather than a test run nobody schedules.

The data fields are ordinary `Option`s underneath, exactly as in [pattern-builder](pattern-builder.md). What
changes is who checks them: the type parameters track completeness, so the terminal method unwraps values the
compiler has already proved present instead of returning a `Result` nobody can trigger. Those unwraps are the
provably-unreachable case, and each one carries the `// SAFETY:` comment that case requires
([rust-errors-handling](rust-errors-handling.md)).

**Which document governs.** This pattern and [pattern-typestate](pattern-typestate.md) both encode state in
types, and they answer to different shapes of state:

| The state is                                                    | Model it as                           | Governed by         |
|-----------------------------------------------------------------|---------------------------------------|---------------------|
| An ordered lifecycle: few states, each holding different data   | distinct concrete structs             | `pattern-typestate` |
| Independent completeness flags: `N` fields over one set of data | `N` marker parameters + `PhantomData` | this document       |

A lifecycle is enumerable. `ReservedPages` → `MappedPages` → `ReleasedPages` is three states, they occur in one
order, and each holds fields the others do not, so each earns a named struct whose name appears in signatures
and diagnostics.

Construction completeness is not enumerable. `N` required fields, each independently set or unset, is a state
space of `2^N`: four required fields are sixteen states, seven are a hundred and twenty-eight, and none of them
differs from its neighbours in the data it holds. Writing them as concrete structs means writing that many
structs and that many transitions. The type parameters **are** the enumeration, and `PhantomData` is what
carries them without adding a field.

So the prohibition in `pattern-typestate` on "a generic wrapper parameterized by a `PhantomData` marker" is
scoped to the lifecycle case it governs, and does not reach a builder. There the marker replaces names that
should have existed; here it stands in for names that cannot.

## Examples

1. **Completeness proved by the compiler, not reported by the terminal method**
   A request descriptor has three required fields that arrive from three different places in the caller, in no
   fixed order. A runtime-checked builder turns a forgotten field into an error value on a path that may not be
   exercised until the console is holding a reply it cannot decode.

```rust
// ❌ Bad — the only thing stopping a request without a reply buffer is a check inside
// build(), and the one call site that forgot it returned MissingReply on the branch taken
// only when the service answered with a domain object. It shipped: the error surfaced as a
// generic dispatch failure two crates away from the builder that produced it.
pub struct RequestBuilder<'a> {
    command_id: Option<u32>,
    payload: Option<&'a [u8]>,
    reply: Option<&'a mut [u8]>,
    copy_handles: &'a [RawHandle],
}

impl<'a> RequestBuilder<'a> {
    pub fn command_id(mut self, id: u32) -> Self {
        self.command_id = Some(id);
        self
    }

    pub fn build(self) -> Result<Request<'a>, MissingField> {
        Ok(Request {
            command_id: self.command_id.ok_or(MissingField::CommandId)?,
            payload: self.payload.ok_or(MissingField::Payload)?,
            reply: self.reply.ok_or(MissingField::Reply)?,
            copy_handles: self.copy_handles,
        })
    }
}
```

```rust
// ✅ Good — build() is declared on one instantiation, so a request missing its reply buffer
// is a "no method named build" error at the call site that omitted it, with nothing left to
// return and no MissingField enum to keep in step with the field list. Each parameter is
// bounded by its own field's State trait, so the type reads as a list of outstanding fields.
use core::marker::PhantomData;

pub struct RequestBuilder<'a, C: command_id::State, P: payload::State, R: reply::State> {
    command_id: Option<u32>,
    payload: Option<&'a [u8]>,
    reply: Option<&'a mut [u8]>,
    copy_handles: &'a [RawHandle],
    _state: PhantomData<(C, P, R)>,
}

impl<'a> RequestBuilder<'a, command_id::Unset, payload::Unset, reply::Unset> {
    pub fn new() -> Self {
        Self {
            command_id: None,
            payload: None,
            reply: None,
            copy_handles: &[],
            _state: PhantomData,
        }
    }
}

// Declared on command_id::Unset, so it is gone once the command id is assigned.
impl<'a, P: payload::State, R: reply::State> RequestBuilder<'a, command_id::Unset, P, R> {
    pub fn command_id(mut self, id: u32) -> RequestBuilder<'a, command_id::Set, P, R> {
        self.command_id = Some(id);
        self.retype()
    }
}

// Optional fields are valid in every state, so they live on a blanket impl and return Self.
impl<'a, C: command_id::State, P: payload::State, R: reply::State> RequestBuilder<'a, C, P, R> {
    pub fn copy_handles(mut self, handles: &'a [RawHandle]) -> Self {
        self.copy_handles = handles;
        self
    }
}

// The whole point: no other instantiation has a build().
impl<'a> RequestBuilder<'a, command_id::Set, payload::Set, reply::Set> {
    pub fn build(self) -> Request<'a> {
        // SAFETY: this impl is reachable only where all three fields are Set, and the only
        // route to a field's Set is the setter that assigns it.
        Request {
            command_id: self.command_id.expect("command id assigned"),
            payload: self.payload.expect("payload assigned"),
            reply: self.reply.expect("reply buffer assigned"),
            copy_handles: self.copy_handles,
        }
    }
}
```

2. **One marker pair per field, each inline and sealed in its own module**
   A shared `Set`/`Unset` pair makes every parameter accept every marker, so the compiler cannot tell a
   correct setter from one that advances the wrong slot. Giving each field its own pair, behind its own sealed
   trait, is what turns that into a type error at the declaration.

```rust
// ❌ Bad — one Set/Unset pair shared by all three parameters. reply() writes its Set into
// the payload slot and leaves its own alone, and nothing here is ill-typed: the crate
// compiled green and the consumer two crates away was told there is no method named
// payload, pointing at the caller's line instead of at the return type that is wrong.
pub trait Position {}
pub struct Set;
pub struct Unset;
impl Position for Set {}
impl Position for Unset {}

impl<'a, C: Position, P: Position> RequestBuilder<'a, C, P, Unset> {
    pub fn reply(mut self, buf: &'a mut [u8]) -> RequestBuilder<'a, C, Set, Unset> {
        self.reply = Some(buf);
        self.retype()
    }
}
```

```rust
// ✅ Good — one marker pair per field, each in the field's own module behind its own sealed
// State trait. The reply parameter is bounded by reply::State, so the transposition above
// fails at the declaration with "the trait bound payload::Set: reply::State is not
// satisfied", naming both the marker and the slot it does not belong in. Subject first,
// mechanism last: the State impls precede `_priv`, which closes the module.
pub mod command_id {
    /// Whether the command id has been assigned.
    ///
    /// Sealed: [`Set`] and [`Unset`] are the only implementors, and no other field's
    /// marker can stand in this parameter.
    pub trait State: _priv::Sealed {}

    /// The command id has been assigned.
    pub struct Set;

    /// The command id has not been assigned yet.
    pub struct Unset;

    impl State for Set {}

    impl State for Unset {}

    mod _priv {
        pub trait Sealed {}

        impl Sealed for super::Set {}

        impl Sealed for super::Unset {}
    }
}

// `payload` and `reply` carry the same items in their own modules, declared inline beside
// this one. Each seals its own trait with its own `_priv`, rather than reaching up for a
// shared one.

impl<'a, C: command_id::State, P: payload::State> RequestBuilder<'a, C, P, reply::Unset> {
    pub fn reply(mut self, buf: &'a mut [u8]) -> RequestBuilder<'a, C, P, reply::Set> {
        self.reply = Some(buf);
        self.retype()
    }
}
```

3. **One `PhantomData` tuple, and one private helper that moves markers**
   Every transition rewrites the marker field, and every transition has to restate the struct literal because
   the return type is a different type. Both costs are paid once when the markers live in a single tuple and
   the rewrite lives in a private helper.

```rust
// ❌ Bad — one marker per parameter, and a full struct literal per setter. Adding a fourth
// required field edits every setter twice: once for the new PhantomData field, once for the
// new literal entry. The setter that was missed kept copy_handles from the wrong builder.
pub struct RequestBuilder<'a, C: command_id::State, P: payload::State, R: reply::State> {
    command_id: Option<u32>,
    payload: Option<&'a [u8]>,
    reply: Option<&'a mut [u8]>,
    copy_handles: &'a [RawHandle],
    _command_id_state: PhantomData<C>,
    _payload_state: PhantomData<P>,
    _reply_state: PhantomData<R>,
}

impl<'a, P: payload::State, R: reply::State> RequestBuilder<'a, command_id::Unset, P, R> {
    pub fn command_id(self, id: u32) -> RequestBuilder<'a, command_id::Set, P, R> {
        RequestBuilder {
            command_id: Some(id),
            payload: self.payload,
            reply: self.reply,
            copy_handles: self.copy_handles,
            _command_id_state: PhantomData,
            _payload_state: PhantomData,
            _reply_state: PhantomData,
        }
    }
}
```

```rust
// ✅ Good — the markers are one field, so a transition rewrites one field, and the data is
// moved by one helper that every setter shares. A fourth required field adds a tuple element
// and a field, and touches no setter it does not belong to.
pub struct RequestBuilder<'a, C: command_id::State, P: payload::State, R: reply::State> {
    command_id: Option<u32>,
    payload: Option<&'a [u8]>,
    reply: Option<&'a mut [u8]>,
    copy_handles: &'a [RawHandle],
    _state: PhantomData<(C, P, R)>,
}

impl<'a, C: command_id::State, P: payload::State, R: reply::State> RequestBuilder<'a, C, P, R> {
    /// Re-labels the markers, carrying the data across unchanged.
    ///
    /// Private: it can move any field in either direction, which only the required setters
    /// may legitimately do. Exposed, it would hand a caller an all-`Set` builder whose
    /// fields are all `None`, and `build` would abort on it.
    fn retype<C2, P2, R2>(self) -> RequestBuilder<'a, C2, P2, R2>
    where
        C2: command_id::State,
        P2: payload::State,
        R2: reply::State,
    {
        RequestBuilder {
            command_id: self.command_id,
            payload: self.payload,
            reply: self.reply,
            copy_handles: self.copy_handles,
            _state: PhantomData,
        }
    }
}
```

4. **A permanent exercise of the whole chain**
   Per-field markers stop a marker landing in the wrong parameter. They do not stop a setter from advancing
   the wrong field, because such a setter is internally consistent. Only running every setter once and then
   the terminal method proves each setter advances its own field, and that run has to be compiled every time.

```rust
// ❌ Bad — payload() advances the reply field and leaves its own alone. Every declaration
// here is well-typed, this crate builds with test = false and doctest = false, and nothing
// in it ever runs the chain, so it compiled green for a release. The consumer that wrote
// .command_id().payload().reply() was told there is no method named reply, three call
// frames from the return type that caused it.
impl<'a, C: command_id::State, R: reply::State> RequestBuilder<'a, C, payload::Unset, R> {
    pub fn payload(
        mut self,
        bytes: &'a [u8],
    ) -> RequestBuilder<'a, C, payload::Unset, reply::Set> {
        self.payload = Some(bytes);
        self.retype()
    }
}
```

```rust
// ✅ Good — the crate runs the chain itself, in ordinary code that is compiled whether or
// not tests are. A setter that advances the wrong field now fails here, in the crate that
// owns the mistake, because the chain never reaches the all-Set instantiation build() is
// declared on.
impl<'a> Request<'a> {
    /// Builds a request carrying no optional entries.
    ///
    /// Calls every required setter once, so the chain is type-checked on every build of
    /// this crate.
    pub fn simple(command_id: u32, payload: &'a [u8], reply: &'a mut [u8]) -> Self {
        RequestBuilder::new()
            .command_id(command_id)
            .payload(payload)
            .reply(reply)
            .build()
    }
}
```

## Why It Matters

**A required field that was forgotten never reaches a runtime check.** The failure a runtime-checked builder
produces is a value on an error path, and an error path is only as reliable as the call site that reads it.
Under `panic = "abort"` the fallback is worse than an ignored error: an `expect` on a `None` takes the process
down on the console. Gating the terminal method removes the path entirely, and the diagnostic lands at the
call site that is missing a line rather than in the caller that received the error.

**The diagnostic names the outstanding fields.** `RequestBuilder<'_, command_id::Set, payload::Unset,
reply::Unset>` says which two fields are still missing without the reader counting positions or consulting a
comment that maps parameter letters onto field names. That is the return on giving each field its own module,
and it is why the markers are not one shared pair.

**Setting a field twice is caught too.** A setter declared on its field's `Unset` marker vanishes once the
field is assigned, so the second call has no method to resolve. A runtime-checked builder accepts both writes
and keeps the last one, which is a bug that reads as correct code.

**The completeness check cannot drift out of step with the fields.** There is no `MissingField` enum to extend,
no `ok_or` chain to keep parallel to the struct, and no test needed for a state the compiler refuses to build.
Adding a required field is a marker module, a type parameter, a field, and a setter; every incomplete call site
then fails to compile, which is the review the change needs.

## Pragmatism Caveat

**One or two required fields do not justify this.** Take them as arguments to the constructor:
`Request::new(command_id, payload)` cannot be built incomplete either, and it costs nothing to read. The
pattern starts paying at roughly three independent required fields, and only where the fields genuinely arrive
separately; if the caller has all of them in one place, a plain `new` is the honest shape
([pattern-builder](pattern-builder.md)).

**The marker modules are the cost, and it is paid per field.** Each required field brings a module, a sealed
trait, two markers, and a type parameter that appears in every impl block's generics. The type is
self-describing but long, and the cost grows with `N`: past roughly five parameters the signatures and error
messages are the dominant maintenance expense, and the honest alternative is a runtime-checked
`build() -> Result<_, MissingFields>` reporting every absent field at once.

**Without a home for the chain exercise, the pattern is not available.** Requirement 4 is not optional
polish: a builder whose setters are never all run in an always-compiled path can ship a setter that advances
the wrong field, and the breakage lands on consumers. If nothing in the crate can host the exercise, use a
runtime-checked builder, which has no such failure mode.

**A builder crossing the C FFI boundary does not use this.** The boundary needs one concrete type with a stable
layout; markers are a Rust-side construct and there is no instantiation to name in a header. Build the value
behind the boundary and expose the finished type.

Where the pattern is skipped for one of these reasons, no note is needed. Where it is skipped despite three or
more independent required fields whose omission would abort the console, say why at the builder's declaration.

## Checklist

Before committing code, verify:

- [ ] Every required field has exactly one type parameter, and one marker module of its own holding `Set`,
      `Unset`, and the `State` trait that bounds that parameter
- [ ] No marker pair is shared between fields, so a marker written into another field's parameter is a
      trait-bound error rather than well-typed code
- [ ] The marker modules are declared inline in the module that defines the builder, after the builder's
      impls, rather than as module files of their own
- [ ] Each marker module seals its `State` trait with its own `_priv` module and reaches into no parent for a
      shared supertrait, and inside it the `State` impls come first and `_priv` last
      ([pattern-trait-sealed](pattern-trait-sealed.md))
- [ ] The markers are stored in a single `PhantomData<(..)>` field, not one `PhantomData` per parameter, and
      its name is `_`-prefixed to mark it as holding nothing
- [ ] Each required setter is declared on its own field's `Unset` marker, consumes `self`, and advances only
      that field
- [ ] Optional setters live on one blanket impl bounded by every field's `State` trait and return `Self`
- [ ] The terminal method is declared only on the all-`Set` instantiation and returns the built value, not a
      `Result`, and its unwraps carry the `// SAFETY:` comment naming the all-`Set` bound as the proof
- [ ] The retype helper is private, and no public method can move a field from `Set` back to `Unset`
- [ ] Something always compiled calls every required setter once and then the terminal method, so a setter
      that advances the wrong field fails the build rather than a consumer's
- [ ] A builder with fewer than three required fields uses constructor arguments instead
- [ ] A builder whose parameter count makes diagnostics unreadable, or that has nowhere to host the chain
      exercise, uses a runtime-checked `build` instead

## References

- [pattern-builder](pattern-builder.md) - Extends: The builder this specializes; the runtime-checked `build`
  and the rule that built types carry no `Option` for required data
- [pattern-typestate](pattern-typestate.md) - Related: The lifecycle case, modelled as concrete structs, and
  the boundary between the two documents
- [pattern-trait-sealed](pattern-trait-sealed.md) - Related: The `_priv::Sealed` idiom each marker module
  uses, its naming, and the rule that `_priv` is declared last
- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: An incomplete value is made
  unrepresentable rather than rejected
- [principle-information-hiding](principle-information-hiding.md) - Foundation: The sealed traits and the
  private retype helper are what keep each field's marker vocabulary closed
- [rust-mods-graph](rust-mods-graph.md) - Related: Why each marker module seals its own trait instead of
  reaching up for a shared supertrait
- [rust-errors-handling](rust-errors-handling.md) - Related: The `// SAFETY:` comment a provably-unreachable
  unwrap in the terminal method carries

## External References

- [Rust API Guidelines — Builders](https://rust-lang.github.io/api-guidelines/type-safety.html#builders-enable-construction-of-complex-values-c-builder)
- [Rust API Guidelines — Sealed Traits](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed)
- [The Typestate Pattern in Rust — Cliff L. Biffle](https://cliffle.com/blog/rust-typestate/)
