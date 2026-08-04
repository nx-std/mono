---
name: "handles"
description: "Kernel handles and server-side object ids: the raw/owned/borrowed split, one closer per resource, and where ManuallyDrop is allowed. Load when a type holds a handle or an object id, closes one on drop, or hands one out"
type: "core"
scope: "global"
---

# Kernel Handle Ownership

**MANDATORY for ALL types that hold a kernel handle or a server-side object id**

**Ownership of a kernel resource is a type, never a convention.** A handle is a number the kernel expects
back exactly once, and a number is trivially copied — so the thing that closes it must be a value that
cannot be, and everything else must be a borrow that carries the owner's lifetime. Every rule below follows
from that.

This document owns how a resource's ownership is *modelled*. [rust-fn-unchecked](rust-fn-unchecked.md) owns
when the constructor that adopts one is `unsafe` and the `// SAFETY:` note its call sites carry.
[pattern-newtype](pattern-newtype.md) owns the wrapper the three roles are built from.

## 1. Three Roles, Three Types

A kernel handle is a number the kernel issues and expects back exactly once. Three separate questions
attach to that number, and each gets its own type:

| Role | Shape | Closes? |
|---|---|---|
| **Name** the resource | `Copy`, no destructor | never |
| **Own** the resource | neither `Copy` nor `Clone`, `Drop` closes | exactly once |
| **Borrow** the resource | `Copy`, no destructor, carries the owner's lifetime | never |

The name is the bare newtype over the handle word. It is `Copy` on purpose: a number that closes nothing
can be passed around freely, and every SVC wrapper needs one.

```rust
// ✅ Good — three types, and only the middle one can close. A second owner needs a
// move, and the move checker rejects it.
pub struct PortHandle(u32);              // name: Copy, closes nothing

pub struct OwnedPortHandle(PortHandle);  // owner: !Copy, !Clone, Drop closes

pub struct BorrowedPortHandle<'h> {      // borrow: Copy, cannot outlive its owner
    handle: PortHandle,
    owner: PhantomData<&'h OwnedPortHandle>,
}
```

The lifetime on the borrowed form is the whole point: it is what makes use-after-close a compile error
rather than a request that reaches whatever now holds the reused number.

## 2. One Closer Per Resource

**No type pairs a `Copy` handle with a `Drop` that closes it.** The owning type is the only thing in the
workspace whose destructor closes that resource, and the compiler enforces "exactly once" because the type
cannot be duplicated.

```rust
// ❌ Bad — the handle is `Copy`, so `as_raw` hands out a second one and a second
// wrapper closes it again. The first close freed the number, the kernel reissued it
// to an unrelated event, and the second close tore that down: the owning thread
// blocked forever on a wait that would never be signalled.
pub struct ConnectionEvent {
    handle: EventHandle, // Copy
}

impl ConnectionEvent {
    pub fn as_raw(&self) -> EventHandle {
        self.handle
    }
}

impl Drop for ConnectionEvent {
    fn drop(&mut self) {
        let _ = svc::close_handle(self.handle);
    }
}
```

```rust
// ✅ Good — the owning field carries the close, so `ConnectionEvent` needs no `Drop`
// of its own and cannot hand out a second closer. Callers that only wait on the event
// get the borrowed form, which has no destructor to misuse.
pub struct ConnectionEvent {
    handle: OwnedEventHandle,
}

impl ConnectionEvent {
    pub fn as_borrowed(&self) -> BorrowedEventHandle<'_> {
        self.handle.as_borrowed()
    }
}
```

A double close is a **resource error, not undefined behaviour** — nothing faults, an unrelated object is
simply torn down. That is why the tool for it is the move checker rather than `unsafe`
([rust-fn-unchecked §1](rust-fn-unchecked.md#1-validation-lives-in-fromstr-and-tryfrom)); spending `unsafe`
here would devalue it where it marks real UB, and would not stop the duplication anyway.

Server-side object ids inside an IPC domain follow the same split, for the same reason: an id is closed by
a per-object request, and closing one the server has since reissued tears down someone else's object.

## 3. Functions Take the Borrowed Form

A function that only *uses* a resource takes the borrowed type. Only the points that mint a resource or
hand one on take the owning type.

```rust
// ❌ Bad — takes the owner to read a value from it. Every caller either gives up its
// handle or clones one, and the two crates that reached for the second option shipped
// a close-per-call: the port was torn down after the first query and every later one
// answered `InvalidHandle`.
pub fn query_max_sessions(port: OwnedPortHandle) -> Result<u32, QueryError>
```

```rust
// ✅ Good — the borrow says "I will not close this", the lifetime says "I will not
// outlive it", and callers pass `port.as_borrowed()` without giving anything up.
pub fn query_max_sessions(port: BorrowedPortHandle<'_>) -> Result<u32, QueryError>
```

Taking the owning type is the right signature exactly when the function consumes the resource — a teardown
that sends a close, or a conversion that hands the obligation somewhere else. A `self`-consuming method that
closes is correct; a `&self` method that opens a closer is the bug in the Bad example above.

## 4. `ManuallyDrop` Releases; It Does Not Borrow

`ManuallyDrop` appears only in a release path: moving a field out of a type whose destructor must not run.
That is what it is for.

**In a struct field or a parameter type, `ManuallyDrop` means a borrowed type is missing.** Wrapping an
owner to suppress its close is a borrow spelled in the one construction that also offers `into_inner` —
a safe function that hands the suppressed close straight back.

```rust
// ❌ Bad — `ManuallyDrop` standing in for a borrow. The field is a second owner whose
// destructor happens to be suppressed, `into_inner` reaches the double close from safe
// code, and the wrapper has no lifetime tying it to the session it names, so it can
// outlive it entirely.
pub struct SessionView {
    port: ManuallyDrop<OwnedPortHandle>,
}
```

```rust
// ✅ Good — the borrowed type says the same thing in the type system: no destructor to
// suppress, no `into_inner` to reach, and the lifetime keeps the view inside the
// owner's.
pub struct SessionView<'h> {
    port: BorrowedPortHandle<'h>,
}
```

```rust
// ✅ Good — the release path `ManuallyDrop` exists for: the close is being handed on
// rather than performed, so the destructor that would perform it is suppressed once,
// in the method that transfers the obligation.
impl OwnedPortHandle {
    pub fn into_handle(self) -> PortHandle {
        let this = ManuallyDrop::new(self);
        this.0
    }
}
```

A struct that must store a resource it cannot borrow — because the owner sits in a field beside it — stores
the **id** and rebuilds the borrowed view on demand. That keeps the struct non-self-referential, which is
what lets it be moved, and it is strictly cheaper than the `transmute` to `'static` the alternative invites.

## 5. Adoption and Release

Two operations cross the boundary the type system cannot see, and each gets one narrowly-scoped function.

**Adoption** wraps a number the kernel or a server just issued. It skips a check nothing local can perform —
only the issuer knows which numbers are live — so it is named `from_raw_unchecked`, stays a safe `fn`, and
states its obligation in prose ([rust-fn-unchecked §3](rust-fn-unchecked.md#3-naming-and-visibility)). Keep
it as narrow as its callers allow: a `pub(crate)` adopter cannot be reached by a consumer who does not hold
the proof.

**Release** gives up the obligation, returning the bare name. It is the `into_*` half, and it is where
`ManuallyDrop` belongs ([§4](#4-manuallydrop-releases-it-does-not-borrow)).

```rust
// ✅ Good — adoption vouches for one thing and says which; release hands the same
// obligation on. Between them, nothing can mint an owner for a resource that already
// has one.
impl OwnedPortHandle {
    /// Adopts a port handle as the sole owner.
    ///
    /// The caller must ensure `raw` names a live port this process owns and that
    /// nothing else will close, since this value closes it on drop. A second owner
    /// sends its close against a number the kernel may have reissued, tearing down an
    /// unrelated port rather than faulting, which is why this is a safe function.
    pub(crate) const fn from_raw_unchecked(raw: PortHandle) -> Self {
        Self(raw)
    }
}
```

There is no third operation. **A method that hands back a second owner of a live resource is not a design
point**, whatever it is named and whatever it wraps the result in: `as_borrowed` covers every legitimate
caller, and anything it cannot express is a signal that the *caller* should hold an id rather than a value
([§4](#4-manuallydrop-releases-it-does-not-borrow)).

## Checklist

Before committing code, verify:

- [ ] Each resource has exactly one owning type, and it is the only thing whose `Drop` closes that resource
- [ ] The owning type is neither `Copy` nor `Clone`
- [ ] The borrowed type is `Copy`, has no destructor, and carries the owner's lifetime via `PhantomData`
- [ ] No type pairs a `Copy` handle with a `Drop` that closes it
- [ ] Every function that only uses a resource takes the borrowed type; only minting, hand-off, and
      consuming teardown take the owning type
- [ ] No `ManuallyDrop` appears in a struct field or a parameter type
- [ ] Every `ManuallyDrop` sits in an `into_*` method that transfers the obligation
- [ ] A struct that cannot borrow its resource stores the id and rebuilds the view on demand, rather than
      transmuting a lifetime to `'static`
- [ ] The adopting constructor is named `from_raw_unchecked`, is a safe `fn`, states its obligation in
      prose, and is as narrow as its callers allow
- [ ] No API hands back a second owner of a live resource

## References

- [rust-fn-unchecked](rust-fn-unchecked.md) - Foundation: Why an adopting constructor stays safe, and the
  `// SAFETY:` note its every call site carries
- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: The illegal state this
  document makes unrepresentable is "two owners of one resource"
- [pattern-newtype](pattern-newtype.md) - Related: The wrapper the three roles are built from
- [principle-least-surprise](principle-least-surprise.md) - Related: Why a type that suppresses a
  destructor reads as an owner, not a borrow
