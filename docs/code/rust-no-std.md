---
name: "rust-no-std"
description: "`no_std` crate anatomy for subproject crates: the `#![no_std]` marker, the lib.rs preamble, and the `extern crate <foo> as _;` pulls that wire in `#[panic_handler]` and `#[global_allocator]`. Load when creating a new subproject crate, editing the top of a `src/lib.rs`, or deciding whether to add `nx-panic-handler` / `nx-alloc` / `alloc` to a crate"
type: arch
scope: "global"
---

# `no_std` Crate Anatomy

**MANDATORY for ALL `src/lib.rs` files under `subprojects/<crate>/` in this workspace**

## Table of Contents

1. [Why `#![no_std]`](#1-why-no_std)
2. [`lib.rs` Skeleton](#2-librs-skeleton)
3. [`extern crate ... as _;` and Linkage Attributes](#3-extern-crate--as-_-and-linkage-attributes)
4. [`#[panic_handler]` via `nx-panic-handler`](#4-panic_handler-via-nx-panic-handler)
5. [`#[global_allocator]` via `nx-alloc`](#5-global_allocator-via-nx-alloc)
6. [Bringing in the `alloc` Crate](#6-bringing-in-the-alloc-crate)
7. [Decision Heuristic](#7-decision-heuristic)
8. [Checklist](#checklist)

---

## 1. Why `#![no_std]`

Every Rust crate in this workspace targets `aarch64-nintendo-switch-freestanding`, a custom Tier-3 target with no
operating system in the conventional sense — only Horizon OS supervisor calls. The Rust `std` library is not available
because it assumes Unix-like syscalls, a thread runtime, file descriptors, and a host allocator, none of which exist on
a freestanding Switch homebrew.

The first non-doc line of **every** `src/lib.rs` in `subprojects/` MUST be:

```rust
#![no_std]
```

This disables the prelude that brings `std` into scope and limits the crate to the `core` crate. Anything from `std`
that you actually need — `Box`, `Vec`, `String`, formatting machinery — comes from the `alloc` crate (Section 6), which
is a sub-crate of `std` that has no OS dependency, only a global allocator dependency.

If you need to assert a target invariant (e.g., aarch64 only), follow `#![no_std]` immediately with a guarded
`compile_error!`:

```rust
#![no_std]

#[cfg(not(target_arch = "aarch64"))]
compile_error!("nx-cpu only supports aarch64 CPUs");
```

---

## 2. `lib.rs` Skeleton

The canonical skeleton, in order, for a subproject crate that has all four optional pieces:

```rust
//! # nx-<aspect>
//!
//! <one-paragraph crate summary>
#![no_std]

// 1. Panic handler — pulled in for its linkage attribute.
extern crate nx_panic_handler as _;     // provides #[panic_handler]

// 2. The `alloc` crate — needed if this crate uses Box/Vec/String/etc.
extern crate alloc;

// 3. Global allocator — only when this crate sits at the link boundary
//    (typically the umbrella `nx-std`).
extern crate nx_alloc as _;              // provides #[global_allocator]

// 4. C-FFI surface — gated behind the `ffi` Cargo feature.
#[cfg(feature = "ffi")]
pub mod ffi;

// 5. Rust-facing modules (always available).
pub mod foo;
pub mod bar;

// 6. Re-exports.
pub use self::foo::Foo;
```

A minimal `sys/*` crate (e.g., `nx-cpu`) drops items 2, 3, and 4 and ends up with just `#![no_std]`, the
`nx-panic-handler` pull, and module declarations. A service crate that needs heap types but is not a link boundary adds
item 2 (`extern crate alloc;`) and skips item 3.

Module organization inside `src/` follows [rust-modules](rust-modules.md); the FFI module
follows [rust-ffi](rust-ffi.md).

---

## 3. `extern crate ... as _;` and Linkage Attributes

In Rust 2018+, `extern crate` is almost never needed — `use` and `Cargo.toml` together make the dependency reachable.
The exception is crates whose value lies in a **linkage attribute** rather than in any item you import by name:

- `#[panic_handler]` (defined inside `nx-panic-handler`)
- `#[global_allocator]` (defined inside `nx-alloc`)

These attributes are picked up by the compiler **only when the defining crate is in the dependency graph as a referenced
extern crate**, not merely as a Cargo dependency. Without an `extern crate` declaration, the rlib that holds the
attribute is dropped by the linker as dead code.

### The `as _;` Idiom

Use `extern crate <name> as _;` whenever you pull a crate in only for its linkage attribute. The `as _` rename binds the
crate to the anonymous identifier `_`, which:

- Makes the **intent** clear ("I'm pulling this in for side effects, not to call anything").
- Prevents the crate name from shadowing or polluting the local namespace.
- Allows `cargo machete` / `cargo udeps` to recognize the dependency as intentional rather than flagging it as unused.

```rust
extern crate nx_panic_handler as _;  // provides #[panic_handler]
extern crate nx_alloc as _;          // provides #[global_allocator]
```

The trailing comment naming the linkage attribute is mandatory — future readers should not have to grep
`nx-panic-handler/src/lib.rs` to understand why the `extern crate` line exists.

### When NOT to Use `as _;`

- `extern crate alloc;` — `alloc` is a standard crate brought in for its **types** (`Box`, `Vec`, `String`). You will
  reference it by name (`alloc::vec::Vec`, `alloc::boxed::Box`), so the binding must stay. No `as _`.
- Re-exports — if a crate intends to publicly re-export another crate (e.g., the umbrella re-exposing `nx_alloc`), use
  `pub extern crate nx_alloc;` without `as _`.

---

## 4. `#[panic_handler]` via `nx-panic-handler`

### When to Add

**Every `no_std` crate in this workspace adds `extern crate nx_panic_handler as _;`.** The rule is uniform: do not try
to predict which compilation graphs need the handler — always declare it.

The Cargo manifest MUST declare the dependency:

```toml
[dependencies]
nx-panic-handler = { version = "0.1.0", path = "../nx-panic-handler" }
```

### Why

`#[panic_handler]` is a one-per-binary linkage attribute. It MUST be present exactly once in the final link or the
program does not compile. The `nx-panic-handler` crate defines it (in `subprojects/nx-panic-handler/src/lib.rs`) and
forwards panics to `svcBreak`.

Even though only the **final staticlib** needs to surface the panic handler, every intermediate rlib that compiles with
`#![no_std]` is itself a unit where Rust may emit panic-triggering code (slice bounds checks, integer overflow checks,
`unwrap`/`expect`, formatting). Declaring `extern crate nx_panic_handler as _;` in every crate ensures:

1. The rlib carries the panic-handler dependency in its metadata, so downstream consumers (and tests linked directly
   against the rlib) get the symbol.
2. The dead-code stripper does not drop `nx-panic-handler` between a leaf crate and the final staticlib.
3. New crates start "ready to link" without later debugging undefined `rust_begin_unwind` errors.

### Cost

`nx-panic-handler` is intentionally minimal — it includes no `nx-svc` dependency and contains only the SVC bytes it
needs (`svcBreak`). The cost of always pulling it in is negligible.

### Producer-Module Convention

`nx-panic-handler` itself does NOT carry an `extern crate nx_panic_handler as _;` line — it **defines** the handler. Its
own `lib.rs` is just `#![no_std]` plus the `#[panic_handler]` function.

---

## 5. `#[global_allocator]` via `nx-alloc`

### When to Add

Add `extern crate nx_alloc as _;` (or `pub extern crate nx_alloc;` for the umbrella's re-export) **only when the crate
is at, or near, the link boundary** — i.e., when this crate's compilation unit will provide the single
`#[global_allocator]` for the final staticlib.

In this workspace the relevant link boundary is the umbrella crate (`nx-std`). The umbrella declares the dependency and
the `extern crate` line behind its `alloc` feature:

```rust
#[cfg(feature = "alloc")]
pub extern crate nx_alloc; // Provides #[global_allocator]
```

Some intermediate crates (`nx-sys-mem`, `nx-std-sync`, …) also declare `extern crate nx_alloc;` because their own
compilation may need to **prove** that a global allocator exists when they use `Box`/`Vec`. In those cases the
`extern crate nx_alloc;` is a *visibility* shim, not a registration — the umbrella still owns the singular
`#[global_allocator]` at link time.

The Cargo dependency in the umbrella:

```toml
[dependencies]
nx-alloc = { version = "0.1.0", path = "../nx-alloc", optional = true }

[features]
alloc = ["dep:nx-alloc", "nx-alloc/global-allocator"]
```

The `global-allocator` feature on `nx-alloc` is what actually flips the `#[global_allocator]` registration on; without
it the crate compiles as a plain allocator implementation without claiming the global slot.

### When NOT to Add

- **Pure `sys/*` crates** (`nx-cpu`, `nx-svc`, `nx-sys-sync`, `nx-sys-thread-tls`) — these neither
  allocate nor sit at a link boundary. Adding `extern crate nx_alloc as _;` would force every consumer to inherit an
  unwanted allocator dependency.
- **Pure Rust IPC service crates** (`nx-service-*`) — they expose a Rust API and let the final binary choose its
  allocator. They may add `extern crate alloc;` (Section 6) to use heap types, but never claim a global allocator.
- **`nx-alloc` itself** — it **defines** the allocator. Its own `lib.rs` does not pull itself in.

### Cost of Getting It Wrong

- **Too many `#[global_allocator]` registrations** → multi-definition link error in the final NRO.
- **Zero `#[global_allocator]` registrations** when any crate in the link uses `alloc` →
  `error: no global memory allocator found` from `rustc`.

The umbrella is the only place that should make this choice; intermediate crates should defer to it via Cargo feature
plumbing, not by adding their own `#[global_allocator]`.

---

## 6. Bringing in the `alloc` Crate

`alloc` is the standard library's heap-types sub-crate. It is shipped with `core` by `rustc` and is always available —
it just isn't in the implicit prelude when `#![no_std]` is set.

### When to Add

Add `extern crate alloc;` whenever the crate uses any heap-allocated standard type:

- `alloc::boxed::Box`, `alloc::vec::Vec`, `alloc::string::String`, `alloc::collections::*`
- `alloc::sync::Arc`, `alloc::rc::Rc`
- Formatting machinery beyond what `core::fmt` provides (e.g., `format!`)

The `extern crate alloc;` line by itself is enough — Cargo doesn't list `alloc` as a `[dependencies]` entry because it
ships with the toolchain.

```rust
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]
extern crate alloc;                 // Box, Vec, String, …

use alloc::vec::Vec;
```

### Pair with a Global Allocator

`alloc`-using code links only when the final binary registers a `#[global_allocator]`. In practice this means a crate
that adds `extern crate alloc;` MUST end up in a link graph that also pulls in `nx-alloc` (typically via the umbrella's
`alloc` feature). The producer crate does NOT need to declare `nx-alloc` itself — the umbrella's feature plumbing
handles activation.

### When to Skip

If the crate only uses `core` (stack-only types, `&str`, fixed arrays, etc.), do NOT add `extern crate alloc;`. Bringing
it in needlessly forces downstream consumers into a link graph that requires `#[global_allocator]`, which is a real
burden for tiny utilities like `nx-cpu`.

---

## 7. Decision Heuristic

When writing a new `src/lib.rs`, walk this list top-to-bottom:

| Decision                                             | If yes…                                                                                                                                       | If no…                                                    |
|------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------|
| Is this a subproject crate in this workspace?        | `#![no_std]` and `extern crate nx_panic_handler as _;` are **mandatory**.                                                                     | (Not applicable here.)                                    |
| Does the crate use `Box`/`Vec`/`String`/`Arc`/etc.?  | Add `extern crate alloc;`. No Cargo dep needed (ships with toolchain).                                                                        | Skip `alloc`; you stay on pure `core`.                    |
| Is this the umbrella (`nx-std`) or a runtime entry?  | Add `extern crate nx_alloc as _;` (or `pub extern crate nx_alloc;`). Add `nx-alloc` to `[dependencies]` (typically optional + feature-gated). | Skip `nx-alloc`. The umbrella owns `#[global_allocator]`. |
| Does the crate export `__nx_<aspect>__*` symbols?    | Gate `pub mod ffi;` with `#[cfg(feature = "ffi")]`. Add `ffi = []` to `[features]` (see [rust-ffi](rust-ffi.md)).                             | Omit the `ffi` feature entirely.                          |
| Does the crate target only aarch64 (e.g., uses asm)? | Add a guarded `compile_error!` immediately after `#![no_std]`.                                                                                | Skip the guard.                                           |

### Examples in This Workspace

| Crate               | `#![no_std]` | `nx_panic_handler as _` | `alloc` | `nx_alloc`  | Notes                                           |
|---------------------|:------------:|:-----------------------:|:-------:|:-----------:|-------------------------------------------------|
| `nx-cpu`            |      ✅       |            ✅            |    ❌    |      ❌      | Pure `sys/*` utility; stack-only.               |
| `nx-svc`            |      ✅       |            ✅            |    ❌    |      ❌      | SVC bindings; no heap.                          |
| `nx-sys-mem`        |      ✅       |            ✅            |    ✅    |  ✅ (shim)   | Uses heap; needs visibility into the allocator. |
| `nx-std-sync`       |      ✅       |            ✅            |    ✅    |  ✅ (shim)   | High-level sync; uses `Arc`.                    |
| `nx-service-time`   |      ✅       |            ✅            |    ❌    |      ❌      | Pure Rust API; no heap.                         |
| `nx-std` (umbrella) |      ✅       |            ✅            |    ✅    |  ✅ (owner)  | Single `#[global_allocator]` registration.      |
| `nx-panic-handler`  |      ✅       |       ❌ (defines)       |    ❌    |      ❌      | Defines `#[panic_handler]`; standalone.         |
| `nx-alloc`          |      ✅       |            ✅            |    ❌    | ❌ (defines) | Defines the allocator implementation.           |

---

## References

- [rust-ffi](rust-ffi.md) - Related: `ffi` Cargo feature for the FFI module declared in `lib.rs`
- [rust-crate](rust-crate.md) - Related: `Cargo.toml` baseline for `nx-panic-handler` and `nx-alloc` dependencies
- [meson-subproject-crate](meson-subproject-crate.md) - Related: Per-crate Meson wrapper; mirrors `nx-panic-handler` into the Meson dependency list
- [rust-modules](rust-modules.md) - Foundation: Module layout within `src/`

## Checklist

Before committing a new or modified `src/lib.rs`, verify:

- [ ] `#![no_std]` is the first non-doc line.
- [ ] `extern crate nx_panic_handler as _;` is present with the `// provides #[panic_handler]` comment, and
  `nx-panic-handler` is in `[dependencies]`.
- [ ] `extern crate alloc;` is present **iff** the crate uses heap types from `alloc::*`. No `as _`.
- [ ] `extern crate nx_alloc as _;` (or `pub extern crate nx_alloc;`) is present **only** at the umbrella or at
  link-boundary crates that own the global allocator, and `nx-alloc` is declared in `[dependencies]`.
- [ ] The producer crate does NOT register `#[global_allocator]` if it is not the umbrella.
- [ ] Linkage-attribute pulls use the `as _;` idiom with a trailing comment naming the attribute.
- [ ] `extern crate alloc;` is omitted when the crate uses only `core`.
- [ ] Aarch64-only crates carry a guarded `compile_error!` immediately after `#![no_std]`.
