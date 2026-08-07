---
name: "rust-fmt"
description: "core::fmt impls, fully qualified and never imported; Display/Debug/LowerHex/UpperHex with tests pinning the rendering. Load when implementing Display or a hex trait"
type: "core"
scope: "global"
---

# Formatting Traits (`core::fmt`)

## 1. Fully Qualified, Never Imported

Every `core::fmt` item is written at its full path: the trait in the `impl` header, `core::fmt::Formatter<'_>`
in the signature, `core::fmt::Result` as the return type. **Nothing from `core::fmt` is ever imported.**

The names are the problem. `Result` and `Error` from `core::fmt` collide with the crate's own — and in this
workspace nearly every crate defines both; `Write` collides with the `Write` a buffer writer already brings in;
and a bare `Result` in a `fmt` signature reads as the crate's `Result` to everyone who did not scroll to the
prologue. Qualifying costs eleven characters and removes the ambiguity permanently.

```rust
// ❌ Bad — `Result` in the signature is core::fmt's, but nothing at the use site says
// so, and the import shadows the crate's own Result for the rest of the file.
use core::fmt::{
    Display,
    Formatter,
    Result,
};

impl Display for CommandId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {}
}
```

```rust
// ✅ Good — no import, and every type in the signature names itself.
impl core::fmt::Display for CommandId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {}
}
```

## 2. Delegate Through the Trait, Not Through `write!`

A newtype that renders as its inner value delegates by calling the trait function directly:

```rust
// ❌ Bad — `write!` starts a fresh format spec for the inner value, so the outer
// formatter's flags are dropped: `{:>6}` pads nothing, `{:#x}` loses its prefix.
// The bug only appears at the one call site that used a flag.
fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
}
```

```rust
// ✅ Good — the same formatter is handed to the inner impl, so width, fill,
// precision, and the alternate flag all propagate.
fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    core::fmt::Display::fmt(&self.0, f)
}
```

Name the trait in the delegation rather than writing `self.0.fmt(f)`. When the inner type implements several
formatting traits — the normal case for a raw handle, a packed command id, or any integer — `self.0.fmt(f)`
resolves by inference and silently picks a different rendering the moment the surrounding impl changes.

## 3. The Type Documents Its Formatting Surface

A type with more than one formatting trait carries a `## Formatting` section in its own docs, stating what each
one renders and linking to the impls. A reader choosing between `{}`, `{:?}`, and `{:x}` should not have to
read three impl bodies.

```rust
/// A Horizon OS result code, packed as a module in bits 0-8 and a description in bits 9-21.
///
/// ## Formatting
///
/// The `ResultCode` type implements the following formatting traits:
///
/// - Use [`core::fmt::Display`] for the `2XXX-YYYY` module/description pair shown on the console.
/// - Use [`core::fmt::LowerHex`] (or [`core::fmt::UpperHex`]) for the packed 32-bit word.
///
/// See the [`Display`], [`LowerHex`], and [`UpperHex`] trait implementations for usage examples.
///
/// [`Display`]: #impl-Display-for-ResultCode
/// [`LowerHex`]: #impl-LowerHex-for-ResultCode
/// [`UpperHex`]: #impl-UpperHex-for-ResultCode
pub struct ResultCode(u32);
```

## 4. Every `fmt` Impl Carries a Doctest

The rustdoc goes **on the `fmt` method**, not only on the type, and it states the rendering with an example that
asserts the exact output.

A type's rendering is a contract: it lands on the console's fatal screen, in a panic message, in a debug-console
dump, and in the `Display` a homebrew developer reads when a call fails. Prose describing it drifts; an assertion
does not. Without one, a change to the encoding breaks every consumer without breaking a single test.

Crates here build for `aarch64-nintendo-horizon.json`, where `cargo test --doc` does not run, so the
rustdoc example documents the rendering but **does not pin it**. The pin is a plain `#[test]` over the formatted
string in the in-tree `tests` module — a host-runnable unit test with no `it_` prefix, since formatting needs no
kernel. Write both: the example for the reader, the test for CI.

```rust
// ✅ Good — the unit test is the specification, and it fails the moment the rendering
// changes, whether or not anyone remembered this impl had consumers.
impl core::fmt::Display for ResultCode {
    /// Format the `ResultCode` as its `2XXX-YYYY` module and description pair.
    ///
    /// ```rust
    /// let rc = ResultCode::from_parts(168, 2);
    ///
    /// assert_eq!(alloc::format!("{rc}"), "2168-0002");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04}-{:04}", 2000 + self.module(), self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_display_with_module_and_description_renders_console_code() {
        let rc = ResultCode::from_parts(168, 2);

        assert_eq!(alloc::format!("{rc}"), "2168-0002");
    }
}
```

## 5. Hex Traits Come in Pairs and Document the Alternate Flag

An integer-backed type that implements `LowerHex` implements `UpperHex` too. Callers pick the case at the format
site, and a type that offers only one forces the other half of the codebase into hand-rolled case conversion on a
formatted string.

Both impls document that the alternate flag `#` prepends `0x`, and both tests assert it — that behavior is
inherited from the inner type's impl, so it is easy to change by accident when the inner type is swapped.

```rust
impl core::fmt::LowerHex for ResultCode {
    /// Lowercase hex representation of the packed `ResultCode` word.
    ///
    /// Note that the alternate flag, `#`, adds a `0x` in front of the output.
    ///
    /// ```rust
    /// let rc = ResultCode::from_parts(168, 2);
    ///
    /// assert_eq!(alloc::format!("{rc:x}"), "4a8");
    /// assert_eq!(alloc::format!("{rc:#x}"), "0x4a8");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl core::fmt::UpperHex for ResultCode {
    /// Uppercase hex representation of the packed `ResultCode` word.
    ///
    /// Note that the alternate flag, `#`, adds a `0x` in front of the output.
    ///
    /// ```rust
    /// let rc = ResultCode::from_parts(168, 2);
    ///
    /// assert_eq!(alloc::format!("{rc:X}"), "4A8");
    /// assert_eq!(alloc::format!("{rc:#X}"), "0x4A8");
    /// ```
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self.0, f)
    }
}
```

## 6. `Debug` Is Written When the Derive Is Unhelpful

`#[derive(Debug)]` is the default and stays the default. It stops being right when the derived output is
unreadable — a newtype over a packed word derives into `ResultCode(1192)`, a decimal that nobody can match
against the `2168-0002` printed on the console's fatal screen or the `0x4a8` returned by an SVC.

A hand-written `Debug` picks one of two shapes and documents which:

- **Wrapped**: `ResultCode(2168-0002)`, keeping the type name visible in a struct dump.
- **Delegated**: whatever the inner type's `Debug` renders, when the type is meant to be indistinguishable
  from it in output.

A struct-shaped `Debug` built with `f.debug_struct` counts as wrapped: it names the type and then the decoded
fields, which is the right choice when a reader needs the module and the raw word side by side. Either way the
doc on the `fmt` method says what it produces, points readers at the hex traits when they want a specific case,
and asserts the result in a unit test like the ones above.

## 7. `Display` and `FromStr` Round-Trip

When a type implements both, `value.to_string().parse::<T>()` returns the same value. A `Display` that renders
a form its own `FromStr` rejects is a defect: it breaks every code a homebrew developer copies off the fatal
screen and feeds back into a decoder, and it silently breaks any tool that re-reads the rendering this pair
defines.

`FromStr` may accept **more** than `Display` produces — a type that renders `2168-0002` and parses either that
form or a raw `0x4a8` word is fine, and often desirable. It may never accept less.

## Checklist

Before committing code, verify:

- [ ] Formatting traits are implemented as `impl core::fmt::<Trait> for T`, fully qualified
- [ ] No file imports anything from `core::fmt`
- [ ] `f: &mut core::fmt::Formatter<'_>` and `-> core::fmt::Result` are written at full path
- [ ] Delegation calls the trait function (`core::fmt::Display::fmt(&self.0, f)`), never `write!(f, "{}", self.0)`
      and never `self.0.fmt(f)`
- [ ] A type with more than one formatting trait has a `## Formatting` section linking to each impl
- [ ] Every `fmt` impl has rustdoc on the method with an example asserting the exact output
- [ ] The rendering is pinned by a host-runnable `#[test]` over the formatted string, since doctests do not run
      for the freestanding target
- [ ] `LowerHex` and `UpperHex` are implemented together, and both document and assert the `#` alternate flag
- [ ] A hand-written `Debug` says which shape it produces and why the derive was not used
- [ ] `Display` output parses back through `FromStr` to the same value

## References

- [rust-parse](rust-parse.md) - Related: The `FromStr` half of the round-trip, and the same fully-qualified rule
- [rust-imports](rust-imports.md) - Related: The general rule that one-off `core` paths stay qualified
- [rust-docs-rustdoc](rust-docs-rustdoc.md) - Related: The rustdoc sections and the doctest carve-out this
  document relies on
- [pattern-newtype](pattern-newtype.md) - Related: The wrappers that need a formatting surface at all
