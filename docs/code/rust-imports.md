---
name: "rust-imports"
description: "Module prologue order: doc, std, external, mod declarations, then self/crate/super; extension traits as _. Load when adding imports or declaring submodules"
type: "core"
scope: "global"
---

# Import and Module Declaration Order

## 1. The Module Prologue

Every module opens with the same five parts, in this order, separated by blank lines:

0. **Module documentation** — the `//!` block, before any item.
1. **`core` and `alloc` imports**.
2. **External crate imports**.
3. **`mod` declarations**, `pub` and private together in one alphabetical block; feature-gated ones form a
   second block after it.
4. **Local imports and re-exports** — `use`/`pub use` of `self::`, `crate::`, and `super::`.

The declarations come **before** the local imports because the local imports refer to them: a reader meets the
module's structure first, then what it pulls out of that structure.

When some of the sub-modules are feature-gated, the unconditional ones come first as one alphabetical block,
and the `#[cfg(...)]`-gated ones follow as a second. Alphabetical order across the whole set would interleave
the two, and a reader scanning for what a default build contains would have to read every attribute to find
out. The split is what makes the always-present set legible at a glance.

```rust
// ✅ Good — the unconditional block, then the gated one.
pub mod sm;

#[cfg(feature = "service-applet")]
pub mod applet;
```

Every crate here is `#![no_std]`, so the first group is `core` and `alloc` — there is no `std` group to write.
A crate root carries two more lines before the groups: the inner attributes (`#![no_std]`) and the
`extern crate` pulls that opt into `alloc` or wire in a `#[panic_handler]` or `#[global_allocator]`. Those are
covered in section 6.

```rust
// ❌ Bad — the doc block is a comment, the groups are interleaved, and the mod
// declarations are buried under the imports that depend on them.
// HIPC message framing.
use crate::service::Session;
mod buffer;
use core::mem::size_of;
pub use self::buffer::BufferMode;
use zerocopy::IntoBytes as _;
pub(crate) mod raw;
```

```rust
// ✅ Good — documentation, core/alloc, external, declarations, then local imports.
//! HIPC message framing.
//!
//! Lays out the buffer descriptors, handles, and raw data words that make up a request.

use alloc::vec::Vec;
use core::mem::size_of;

use zerocopy::IntoBytes as _;

mod buffer;
mod descriptor;
pub(crate) mod raw;

use crate::service::Session;

pub use self::{
    buffer::BufferMode,
    descriptor::HandleDescriptor,
};
```

## 2. What rustfmt Does and Does Not Do

The formatter splits `use` statements into the three groups (core/alloc, external, local), merges them per
crate, lays out multi-item braces vertically, and sorts `mod` declarations alphabetically — none of it worth
arguing about in review. What it does **not** do is place the `mod` block: item order is preserved as written,
so a prologue with its declarations in the wrong place formats cleanly and stays wrong. Section 1 is the
human's part.

## 3. Submodule Types Travel Through `self::`

A type declared in a submodule and re-exported by its parent is referenced from that parent through `self::` —
in the re-export and in any import of it. A bare module name relies on a path resolution the 2018+ editions
dropped; `crate::` for something one level down states a longer path than the truth and breaks when the module
moves.

```rust
// ❌ Bad — the parent reaches for its own child through the crate root. Moving this
// module anywhere in the tree breaks every one of these lines, for no benefit.
mod buffer;

pub use crate::hipc::buffer::BufferMode;
use crate::hipc::buffer::BufferModeParseError;
```

```rust
// ✅ Good — the child is addressed relative to the parent that owns it.
mod buffer;

pub use self::buffer::BufferMode;
use self::buffer::BufferModeParseError;
```

This applies to the private form too: a parent that consumes a submodule type without re-exporting it still
writes `use self::buffer::BufferModeParseError;`.

## 4. Import From the Defining Module

Import an item from the module that **declares** it, not from a module that happens to re-export it. A
re-export is a convenience for consumers outside the crate; inside the crate it hides where an item lives and
produces two paths to the same type in the same codebase.

```rust
// ✅ Good — the path names the defining module. Through the crate root's re-export
// (`use crate::Session;`) nothing says where Session is defined, and a second module
// importing it the other way makes the two look unrelated.
use crate::service::Session;
```

## 5. Siblings Use `super::`; `super::super::` Is Prohibited

A module reaching a sibling goes up one level: `use super::sibling::Item;`. That is the whole allowance for
relative upward paths. Which edges may exist at all — a sibling module yes, an item declared by the parent
file no — is owned by [rust-mods-graph](rust-mods-graph.md).

**`super::super::` is prohibited**, in `use` statements, in inline paths, and in intra-doc links. A path that
climbs two or more levels is unreadable at the use site — the reader has to reconstruct the file's position in
the tree — and it breaks silently when either module moves. Address the item from the crate root instead.

```rust
// ❌ Bad — the reader cannot tell what this names without knowing where the file sits,
// and moving either module changes what it resolves to without a compile error.
use super::super::DispatchError;

//! [`dispatch`](super::super::Session) owns the handle every request is sent on.
```

```rust
// ✅ Good — an absolute path inside the crate, readable in isolation.
use crate::cmif::DispatchError;

//! [`dispatch`](crate::service::Session) owns the handle every request is sent on.
```

A `super::super::` that feels necessary is usually a placement problem: the item the deep path reaches for
belongs closer to its users, or the two modules belong under a shared parent.

## 6. Extension Traits Are Imported As `_`

A trait imported only so its methods resolve is imported **without binding its name**. The same applies to a
crate pulled in purely for a side effect — a `#[panic_handler]` or a `#[global_allocator]` that the linker
needs and no source line ever names:

```rust
#![no_std]

extern crate alloc;
extern crate nx_alloc as _; // provides #[global_allocator]
extern crate nx_panic_handler as _; // provides #[panic_handler]

use bitflags::Flags as _;
use zerocopy::IntoBytes as _;
```

Two things follow from dropping the name, and both are the point:

- **Same-named traits can coexist.** `use core::fmt::Write; use alloc::fmt::Write;` is a hard error (`E0252`),
  and the same collision appears with any pair of `*Ext` traits sharing a name across crates. Imported `as _`,
  both resolve their methods and neither claims the name.
- **The import's liveness is tied to method calls alone.** A named import stays live if the name appears
  anywhere else — a bound, an `impl`, a qualified call — so deleting the last method call leaves it in place,
  meaning something other than what it was added for. With `as _` there is no other way for it to be used, so
  the day the methods go, the compiler reports it unused.

Import the trait **by name** when the name is needed: as a bound (`fn f<T: Flags>(..)`), in an
`impl Trait for Type`, or in a qualified call (`<T as Flags>::empty`). Needing the name is the signal that this
is not a method-only import. `core` traits are imported by name in that case — `use core::ops::Deref;` and then
`impl Deref for Session`, never `use core::ops;` and `impl ops::Deref`.

`zerocopy` is the standing exception: its traits are never imported by name. A bound spells the trait out
(`fn decode<T: zerocopy::FromBytes>(..)`), and the trait enters scope only where a method call needs it, as
`use zerocopy::FromBytes as _;`. The derives are written `#[derive(zerocopy::FromBytes)]` for the same reason —
one path, no prologue entry, nothing to keep in sync when the derive list changes.

```rust
// ❌ Bad — the name is bound but never referenced. It blocks any other `Flags` or
// `IntoBytes` the module might need, and it will outlive the calls that justified it.
use bitflags::Flags;
use zerocopy::IntoBytes;

fn encode(perms: BufferPerms, out: &mut [u8]) -> Result<(), EncodeError> {
    out[..4].copy_from_slice(perms.bits().as_bytes());
    if perms.contains(BufferPerms::all()) { /* .. */ }
}
```

Both imports above are method-only: written `use bitflags::Flags as _;` and `use zerocopy::IntoBytes as _;`,
they take no name and nothing but a method call can keep them alive.

## 7. What Not to Import

Not every path needs a `use`. Three cases stay inline:

- **One-off `core` items** used once or twice in a file: spell `core::mem::size_of` and `core::cmp::Ordering`
  fully qualified at the use site rather than importing them. The qualification is the documentation. Two
  `core` modules are never imported from at all, however often they appear: `core::fmt`
  ([rust-fmt](rust-fmt.md)) and `core::str::FromStr` ([rust-parse](rust-parse.md)).
- **Attribute and derive macros**: write `#[derive(thiserror::Error)]`, not an import of the macro.
- **Glob imports**: `use x::*` does not appear in production code. The one accepted glob is `use super::*;` at
  the top of a `#[cfg(test)] mod tests` block, which pulls in the module under test.

```rust
// ✅ Good — qualified at the use site; nothing to look up, and the prologue carries
// no name the rest of the file never mentions again.
fn wider(a: &BufferDescriptor, b: &BufferDescriptor) -> core::cmp::Ordering {
    b.size.cmp(&a.size)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {}
```

## Checklist

Before committing code, verify:

- [ ] The module opens with its `//!` documentation, before any item
- [ ] `core`/`alloc` imports, then external crate imports, each in its own group
- [ ] `mod` declarations form one block after the external imports and before the local imports, with any
      feature-gated ones in a second block after the unconditional set
- [ ] Local `use`/`pub use` of `self::`, `crate::`, and `super::` come last
- [ ] Submodule types are imported and re-exported through `self::`, never through `crate::` or a bare name
- [ ] Items are imported from the module that declares them, not through a re-export
- [ ] A sibling is reached with `super::`; no path contains `super::super::`, including in doc links
- [ ] Traits imported only for their methods are imported `as _`; a trait is imported by name only when the
      name appears in a bound, an `impl`, or a qualified call
- [ ] `extern crate` pulls that exist only for a `#[panic_handler]` or `#[global_allocator]` are bound `as _`
- [ ] `zerocopy` traits are spelled out in bounds and derives, and brought into scope only as `as _`
- [ ] One-off `core` items and attribute macros are written fully qualified instead of imported
- [ ] No glob import outside `use super::*;` in a test module
- [ ] The file is formatted, so grouping, granularity, and `mod` sorting are the formatter's output

## References

- [rust-mods-files](rust-mods-files.md) - Related: Module file layout; this doc owns the prologue inside each file
- [rust-mods-graph](rust-mods-graph.md) - Related: Which references between modules are legal; this doc owns the form those paths take
- [rust-mods-members](rust-mods-members.md) - Related: Ordering of the items that follow the prologue
- [rust-fmt](rust-fmt.md) - Related: Owns the never-import rule for `core::fmt`
- [rust-parse](rust-parse.md) - Related: Owns the never-import rule for `core::str::FromStr`
- [rust-docs-rustdoc](rust-docs-rustdoc.md) - Related: The `//!` block that opens the prologue
