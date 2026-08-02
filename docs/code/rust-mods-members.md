---
name: "rust-mods-members"
description: "Member ordering within a module: main API first, errors after the function that returns them. Load when organizing module contents"
type: "core"
scope: "global"
---

# Module Member Ordering

**MANDATORY for ALL Rust code in the nx-std workspace**

A module reads top-down as its public interface first, then the code that supports it, so a reader meets the
API before the machinery and a reviewer finds the main logic without scrolling.

## 1. Member Ordering

Organize module members in this order:

1. **Module prologue** — doc comment, imports, `mod` declarations, re-exports ([rust-imports](rust-imports.md))
2. **Constants and statics** (`const`, `static`)
3. **Type aliases** (`type Foo = ...`)
4. **Main module members** — public types, main functions (`new`, `dispatch`, `parse_response`)
5. **Helper types and functions** — in dependency order: if A calls B, A comes first

The prologue itself — the `//!` block, the import groups, the `mod` declarations, and the local re-exports —
is owned by [rust-imports](rust-imports.md). This document covers the items that follow it.

```rust
// ❌ Bad — the reader meets a private header struct and a cursor helper before learning
// what `cmif::response` is for, and has to read the whole file to find `parse_response`.
struct OutHeader { ... }
fn read_out_header(cursor: &mut Cursor<'_>) -> Result<OutHeader, ParseError> { ... }
pub fn parse_response(buf: &[u8]) -> Result<Response<'_>, ParseError> { ... }
pub enum ParseError { ... }
```

```rust
// ✅ Good — the entry point first, then its failures, then the machinery it calls.
pub fn parse_response(buf: &[u8]) -> Result<Response<'_>, ParseError> { ... }
pub enum ParseError { ... }

struct OutHeader { ... }
fn read_out_header(cursor: &mut Cursor<'_>) -> Result<OutHeader, ParseError> { ... }
```

## 2. Error Types Follow Their Function

An error type is declared **immediately after** the function or `impl` block that returns it, never before it
and never in a separate module. The function is what a reader came for; its error is the detail they need
second, and keeping the pair adjacent means a change to a failure path is a single-file edit.

```rust
// ❌ Bad — the error precedes the function, so the reader meets six wire-level variants
// before learning that `dispatch` is what produces them.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError { /* ... */ }

pub fn dispatch(&self, cmd_id: u32, req: Request<'_>) -> Result<Response<'_>, DispatchError> {}
```

```rust
// ✅ Good — the function first, then the failures it can produce.
pub fn dispatch(&self, cmd_id: u32, req: Request<'_>) -> Result<Response<'_>, DispatchError> {}

/// Errors returned by [`dispatch`].
#[derive(Debug, thiserror::Error)]
pub enum DispatchError { /* ... */ }
```

Which module the error lives in, and why an `error.rs` collection is not one, is owned by
[rust-errors-reporting](rust-errors-reporting.md).

## 3. Common Violations

- Main public function (`new`, `dispatch`, `parse_response`) buried after helper functions
- An error type separated from the function that returns it, or collected at the end of the file
- Helper structs or functions appearing before the main types they support
- Private implementation details scattered before public API

## Checklist

Before committing Rust code, verify:

- [ ] Main public function (`new`, `dispatch`, etc.) appears early in the file
- [ ] Public structs/types appear before private helpers
- [ ] Each error type sits immediately after the function or `impl` that returns it
- [ ] Helper functions appear after the code that uses them
- [ ] No private implementation details scattered before public API

## References

- [rust-imports](rust-imports.md) - Related: The module prologue that precedes these members
- [rust-mods](rust-mods.md) - Extends: The module invariants these rules make operational
- [rust-mods-files](rust-mods-files.md) - Related: Module file layout and the no-`mod.rs` rule
- [rust-mods-graph](rust-mods-graph.md) - Related: Which references between module files are legal
