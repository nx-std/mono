---
name: "rust-attrs-derived"
description: "Derive list ordering: std traits first from Debug to Hash, then fully-qualified external derive macros. Load when adding a #[derive], reordering one, or reviewing a type's derived traits"
type: "core"
scope: "global"
---

# Derive Attributes

## 1. Standard Traits First, External Macros Last

A derive list has two halves in a fixed order: the standard-library traits, then the derive macros that come
from dependencies — `#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::FromBytes, zerocopy::IntoBytes)]`.

The split is what makes a long list readable at a glance. The std half says what the type *is* — comparable,
copyable, hashable — and reads the same on every type in the workspace. The external half says what the type
*participates in*: byte-level layout, error reporting, wire decoding. Interleaving them means reading the whole
list to answer either question.

## 2. The Standard Order: `Debug` First, `Hash` Last

Within the std half, derives are written in this order, omitting whatever does not apply:

```
Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash
```

`Debug` leads because nearly every type has it and it is the one a reader scans for. `Hash` trails because it
must agree with `Eq`, so it reads last, after the equality traits it depends on. The comparison traits stay in
supertrait order, `PartialEq` before `Eq` and `PartialOrd` before `Ord`, so the list reflects the direction the
bounds actually point.

```rust
// ❌ Bad — no two of these agree, so nothing can be scanned for. Is this handle
// hashable? Comparable? Answering takes a full read of each list, every time.
#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct SessionHandle { /* ... */ }

bitflags::bitflags! {
    #[derive(Clone, PartialEq, Copy, Eq, Debug)]
    pub struct MapPermission: u32 { /* ... */ }
}

#[derive(thiserror::Error, Debug)]
pub enum DispatchError { /* ... */ }
```

```rust
// ✅ Good — one order everywhere, so the shape of the list carries the information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionHandle { /* ... */ }

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapPermission: u32 { /* ... */ }
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError { /* ... */ }
```

An error enum is the common case of the last line: `thiserror::Error` requires `Debug`, so the pair is written
`Debug, thiserror::Error` — std trait first, macro second, like every other list.

## 3. External Derive Macros Are Fully Qualified

Every derive macro from a dependency is written at its full path (`zerocopy::FromBytes`, `thiserror::Error`,
`zerocopy::KnownLayout`), and **the macro is never imported**. A derive macro almost always shares its name with
the trait it implements, so a bare `FromBytes` in a derive list identifies neither the crate it came from nor
which of the two it is. The qualified path answers both without a trip to the prologue, and it keeps a name out
of the import block that is used nowhere except inside an attribute.

```rust
// ❌ Bad — imports that exist only to be spelled inside `derive`, and a list that no
// longer says where anything comes from. The prologue now carries `FromBytes` twice
// over: once as the derive below, once as the trait a call site needs in scope for
// `read_from_prefix`, and nothing at either site says which is which.
use thiserror::Error;
use zerocopy::{
    FromBytes,
    Immutable,
    IntoBytes,
    KnownLayout,
};

#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct CmifInHeader { /* ... */ }

#[derive(Debug, Error)]
pub enum CmifDecodeError { /* ... */ }
```

```rust
// ✅ Good — no imports, and every macro names its origin.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct CmifInHeader { /* ... */ }

#[derive(Debug, thiserror::Error)]
pub enum CmifDecodeError { /* ... */ }
```

Within the external half, keep a crate's macros together and in the order the crate itself pairs them —
`zerocopy::FromBytes` before `zerocopy::IntoBytes`, matching the direction bytes are read in and written back
out, with the marker derives (`Immutable`, `KnownLayout`) trailing the pair they enable.

Attribute helpers that accompany a derive (`#[error(...)]`, `#[skip]`) are **not** qualified: they are
namespaced by the derive that registered them, and writing them any other way does not compile.

## Checklist

Before committing code, verify:

- [ ] Standard-library derives come first, external derive macros last
- [ ] The std half follows `Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash`, omitting what
      does not apply
- [ ] `Debug` is first when present; `Hash` is last of the std traits when present
- [ ] Every external derive macro is fully qualified (`zerocopy::FromBytes`, `thiserror::Error`,
      `zerocopy::KnownLayout`)
- [ ] No derive macro is imported; the prologue carries no name used only inside a `derive`
- [ ] A crate's macros are grouped, with `zerocopy::FromBytes` before `zerocopy::IntoBytes`
- [ ] Attribute helpers (`#[error(..)]`, `#[skip]`) are left unqualified

## References

- [rust-imports](rust-imports.md) - Related: The prologue these derives keep clear, and the same rule for
  one-off `std` paths
- [rust-attrs-lints](rust-attrs-lints.md) - Related: The other attribute family, and its `#[expect]` rule
- [rust-errors-reporting](rust-errors-reporting.md) - Related: Designing the error enums that carry
  `Debug, thiserror::Error`
- [principle-least-surprise](principle-least-surprise.md) - Foundation: A list read in a predictable order is
  one a reader does not have to parse
