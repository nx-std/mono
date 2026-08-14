---
name: "rust-ffi"
description: "The `ffi` Cargo feature contract for subproject crates: declaration, source gating, `__nx_<aspect>__*` symbol naming, and producer/consumer activation split. Load when designing or modifying a crate's C-FFI surface, adding an `ffi` module, or defining new `__nx_*` symbols"
type: "arch"
scope: "global"
---

# Subproject Crate — `ffi` Cargo Feature

**MANDATORY for ALL crates under `subprojects/<crate>/` that expose a C-FFI surface**

## Table of Contents

1. [Contract Summary](#1-contract-summary)
2. [Declaration](#2-declaration)
3. [Source Gating](#3-source-gating)
4. [Symbol Naming](#4-symbol-naming)
5. [Producer Build vs Consumer Activation](#5-producer-build-vs-consumer-activation)
6. [Crates Without an FFI Surface](#6-crates-without-an-ffi-surface)
7. [An `ffi` Module May Export Nothing of Its Own](#7-an-ffi-module-may-export-nothing-of-its-own)
8. [Checklist](#checklist)

A crate's C-FFI surface and the linker scripts that redirect upstream archive symbols (`libnx`, `newlib`, etc.) to it are deliberately separated. This doc covers only the Rust-side feature contract. The linker-side override scripts that consume the symbols defined here live in [meson-linker-script](meson-linker-script.md).

---

## 1. Contract Summary

| Aspect                     | Rule                                                                                      |
|----------------------------|-------------------------------------------------------------------------------------------|
| Declaration                | `ffi = []` in `[features]`, plus any `<dep>?/<feature>` gating an item only this crate's C boundary calls, or a `#[no_mangle]` definition the C link must own ([§2](#2-declaration)). |
| Source gating              | `#[cfg(feature = "ffi")] pub mod ffi;` in `src/lib.rs`.                                   |
| Symbol naming              | `__nx_<aspect>__<symbol>` with `#[unsafe(no_mangle)] pub extern "C"`.                     |
| Direct producer build      | The producer's own `meson.build` does NOT pass `--features ffi` to `cargo build`.         |
| Crates without an `ffi` module | Service crates (`nx-service-*`) and pure Rust utilities OMIT the `ffi` feature entirely ([§6](#6-crates-without-an-ffi-surface)). |

---

## 2. Declaration

```toml
[features]
# Enable the __nx_<aspect> FFI
ffi = []
```

- The feature value is empty (`[]`) — it gates a compile-time `#[cfg(...)]` branch in the producer crate and contributes nothing else.
- No transitive feature activation belongs in this list. The `ffi` feature MUST NOT pull in additional crates or enable other features. In particular it takes no `dep:` entry: a crate the C surface needs is a dependency of the crate, not of the feature.
- The comment above the declaration opens `# Enable the __nx_<aspect> FFI`, with `<aspect>` the crate's slug ([§4](#4-symbol-naming)), so the line reads the same in every crate that exports a surface of its own. Further lines may follow where the crate has something specific to say, as the exceptions below do. A crate whose `ffi` feature gates something *other* than its own `__nx_*` surface — a result-code mapping, C-shaped backing for another crate's exports ([§7](#7-an-ffi-module-may-export-nothing-of-its-own)) — says what it actually gates instead, because the canonical line would name symbols that do not exist. [rust-crates](rust-crates.md) requires *a* comment on every feature; this document fixes what this one says.

### First exception: a dependency that gates its own mapping on `ffi`

A crate may list `<dep>?/ffi` when that dependency compiles something **only this crate's C boundary calls**,
and gates it on its own `ffi` feature. The result-code mapping is the case in practice: `nx-sf` and the
`nx-service-*` crates put `ToResultCode` behind their `ffi` feature, and the only caller of `to_rc` is an
adapter below a `__nx_*` entry point. A pure-Rust consumer of those crates then compiles none of it.

Activating that from the consumer instead would be wrong, not merely different: the consumer would have to
name every service crate the producer happens to depend on, and would re-name them each time the producer's
dependency list changed. The producer is the crate that knows.

```toml
[features]
# Enable the __nx_rt_core FFI
# `nx-service-applet?/ffi` arrives here rather than under `service-applet`: that
# crate gates its result-code mapping on the feature, and only this crate's C
# boundary calls `to_rc`.
ffi = ["nx-sf/ffi", "nx-service-applet?/ffi"]
```

Each such entry carries a comment saying which item the dependency gates and why this crate is the one that
needs it. An entry without that comment is the violation this section otherwise describes. The exception does
not extend to pulling in a `dep:` or enabling a sibling feature of the producer's own — those still belong to
whichever feature owns them.

### Second exception: a dependency whose feature gates a `#[no_mangle]` definition

A crate may list `<dep>?/<feature>` when that feature defines a **`#[no_mangle]` symbol**. Such a symbol is
defined once per program or the link fails, and `ffi` is already the switch that says which archive carries
the program's unmangled definitions ([§5](#5-producer-build-vs-consumer-activation)). Putting the two under
one switch is what makes "exactly one" hold by construction; under separate switches a build can enable one
and not the other, and the failure surfaces as an undefined or duplicate symbol far from the manifest that
caused it.

`nx-rand` is the case in practice. Its `getrandom-backend` feature defines `__getrandom_v03_custom`, the
symbol `getrandom` resolves this target's entropy through, and the umbrella activates it beside the C surface:

```toml
[features]
# Enable the __nx_std FFI
# `nx-rand?/getrandom-backend` arrives here rather than under `rand`: it defines
# `__getrandom_v03_custom`, and one archive per program defines that.
ffi = ["nx-rand?/ffi", "nx-rand?/getrandom-backend"]
```

The gated symbol is **not** a C-FFI export, and the rest of this document does not reach it: it is
`extern "Rust"`, no linker script redirects it, and its name belongs to the crate that resolves it rather
than to this workspace, so [§3](#3-source-gating) does not place it in `src/ffi.rs` and
[§4](#4-symbol-naming) does not rename it. What this exception governs is the manifest entry alone. A
producer declaring such a feature names it for what it defines, never `ffi`, since a crate has at most one C
surface and this is not it.

---

## 3. Source Gating

The `ffi` module is the **only** module that should be gated by the `ffi` feature. The Rust-facing API of the crate is always available; the exported symbols are only emitted when a downstream consumer opts in.

```rust
// src/lib.rs
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;

// Rust-facing modules below are always available, regardless of `ffi`.
pub mod code;
pub mod error;
// ...
```

```rust
// src/ffi.rs — only compiled with `--features ffi`.
// Holds all `__nx_<aspect>__*` exports.

#[unsafe(no_mangle)]
pub extern "C" fn __nx_<aspect>__<symbol>(/* ... */) -> u32 {
    // delegate to the Rust-facing API in sibling modules
}
```

Keep all `extern "C"` exports inside `src/ffi.rs` (or submodules under `src/ffi/` for larger surfaces). Do NOT scatter `#[cfg(feature = "ffi")]` `pub extern "C" fn` definitions across other modules — the linker-override authors need a single place to look.

When a crate's override surface spans more than one upstream archive — or is organized that way for clarity — group the `ffi` submodules by **override target**: one submodule per archive, `src/ffi/<archive>.rs` (or `src/ffi/<archive>/` for a larger per-target surface). Target-agnostic FFI helpers stay in a sibling `src/ffi/common.rs`, not under a target submodule. This mirrors the override-script fragment families one-for-one ([Section 4](#4-symbol-naming), [meson-linker-script](meson-linker-script.md)). The `nx-rt-*` runtime family uses this layout — every override targets `libnx` today, so each crate exposes a single `src/ffi/libnx/` submodule tree.

---

## 4. Symbol Naming

Every C-FFI symbol uses the prefix `__nx_<aspect>__` followed by the original upstream name preserved verbatim:

```rust
// nx-svc — overriding the libnx `svcSetHeapSize` SVC
#[unsafe(no_mangle)]
pub extern "C" fn __nx_svc__svc_set_heap_size(/* ... */) -> u32 { /* ... */ }

// nx-alloc — overriding newlib's `_malloc_r`
#[unsafe(no_mangle)]
pub extern "C" fn __nx_alloc__newlib_malloc_r(/* ... */) -> *mut c_void { /* ... */ }
```

### Rules

- Prefix with `__nx_<aspect>__` where `<aspect>` is the crate's snake_case slug — the trailing segment of the crate name with hyphens flattened (e.g., `nx-svc` → `svc`, `nx-sys-mem` → `sys_mem`, `nx-sys-thread-tls` → `thread_tls`, `nx-alloc` → `alloc`).
- Preserve the original upstream name (case and all) after the prefix so the linker alias side ([meson-linker-script](meson-linker-script.md)) is mechanical: `<symbol> = __nx_<aspect>__<symbol>;`.
- **Group by override target.** When a crate's override surface spans more than one upstream archive — or is organized that way for clarity — insert the archive as a segment: `__nx_<aspect>__<archive>_<symbol>` (e.g., `__nx_alloc__newlib_malloc_r`, `__nx_rt_core__libnx_env_get_loader_info`). The archive segment matches the `src/ffi/<archive>/` submodule ([Section 3](#3-source-gating)) and the `<archive>_<axis>.ld` override-script fragment family ([meson-linker-script](meson-linker-script.md)). The `nx-rt-*` runtime family follows this layout — every fragment targets `libnx` today, so its symbols are `__nx_rt_<entry>__libnx_*` and its fragments `libnx_*.ld`.
- Apply `#[unsafe(no_mangle)]` so the symbol survives mangling.
- Apply `extern "C"` so the function uses the C ABI expected by the upstream callers.
- Use integer types that match the upstream prototype exactly (width, signedness, pointer mutability). Audit each signature against the upstream archive's headers (e.g., `subprojects/libnx/nx/include/switch/.../` for libnx) rather than guessing.

### Naming Examples

| Crate                  | `<aspect>` slug    | Example symbol                          |
|------------------------|--------------------|-----------------------------------------|
| `nx-svc`               | `svc`              | `__nx_svc__svc_set_heap_size`           |
| `nx-alloc`             | `alloc`            | `__nx_alloc__newlib_malloc_r`           |
| `nx-rt-core`           | `rt_core`          | `__nx_rt_core__libnx_initheap`          |
| `nx-sys-mem`           | `sys_mem`          | `__nx_sys_mem__virtmem_reserve`         |
| `nx-sys-thread-tls`    | `thread_tls`       | `__nx_thread_tls__get_thread_vars`      |
| `nx-std-sync`          | `std_sync`         | `__nx_std_sync__mutex_lock`             |
| `nx-sf`                | `sf`               | `__nx_sf__service_close`                |

Note that `<aspect>` is not always the trailing segment — it is the linker-friendly slug that matches the override script filename (`<aspect>_override.ld`). When the trailing segment alone would be ambiguous (`tls` could collide with other thread-local symbols), the slug expands to include enough context (`thread_tls`).

---

## 5. Producer Build vs Consumer Activation

The producer's own `meson.build` invokes `cargo build` **without** `--features ffi`. The producer crate is, by default, a Rust-only `rlib` with no exported override symbols. This keeps each producer minimal and avoids accidentally pulling FFI exports into binaries that never asked for them.

The `ffi` feature only takes effect when a downstream consumer (the staticlib that links the final NRO) re-enables it via Cargo's feature unification — typically through an `nx-<aspect>?/ffi` entry in its own `ffi` feature, gated by a matching Meson setup-time option (`use_nx_<aspect>`):

```toml
# Downstream consumer's Cargo.toml
[features]
ffi = [
    "nx-alloc?/ffi",
    "nx-svc?/ffi",
    # ...
]
```

The `?` makes the activation conditional on the consumer having also enabled `nx-alloc` / `nx-svc` as a dependency, so consumers can pick the exact subset of FFI surfaces they need.

---

## 6. Crates Without an FFI Surface

**The `ffi` feature exists to gate `src/ffi.rs`.** A crate with no such module MUST OMIT the feature entirely. Typical examples:

- `nx-service-*` IPC clients — expose a pure Rust API; the upstream services they replace are wrapped by callers in their own bindings.
- Pure Rust utilities such as `nx-cpu` and `nx-panic-handler`.

Do NOT add `ffi = []` "for symmetry" — its presence is a strong signal that an `ffi` module exists, and downstream `nx-<aspect>?/ffi` references will fail to resolve if the feature is declared without a corresponding `src/ffi.rs`.

Having no symbols of its own is not the same as having no module ([§7](#7-an-ffi-module-may-export-nothing-of-its-own)).

## 7. An `ffi` Module May Export Nothing of Its Own

Defining `__nx_*` symbols is the usual reason to have an `ffi` module, not the only one. A crate may hold the C-shaped data that **another** crate's exports address, and that data belongs behind the same feature: it is built for a C boundary, and a pure-Rust link should not pay for it.

The alternative is worse in both directions. Compiling the backing unconditionally bills every Rust-only consumer for a shape only C reads; moving it up to the exporting crate splits ownership of one data structure across a crate boundary, so the store and the view of it drift.

The split follows ownership. `nx-sys-args` stores the process command line, so the nul-terminated copies and the pointer array behind an entry crate's `__nx_rt_<entry>__libnx_system_argv` sit there too: they are a second shape for data that crate already holds. The symbols stay with the entry crates, because the launch path is what defines them ([crates-rt](crates-rt.md)).

Such a crate declares `ffi = []` and gates `src/ffi.rs` exactly as an exporting crate does. What it must not do is invent `__nx_<aspect>__*` names for items no linker script redirects — [§4](#4-symbol-naming) governs symbols, and this crate has none to name. Consumers activate it through their own `ffi` feature, unchanged ([§5](#5-producer-build-vs-consumer-activation)).

---

## Checklist

Before committing changes to a crate's `ffi` feature or `src/ffi.rs`, verify:

- [ ] `[features]` contains `ffi = []`, commented `# Enable the __nx_<aspect> FFI` when the crate exports a
      surface of its own, or with what the feature actually gates when it does not.
- [ ] The `ffi` feature value is `[]`, except for `<dep>?/<feature>` entries that each carry the comment
      saying which item the dependency gates and why this crate is the one that needs it — either an item
      only this crate's C boundary calls, or a `#[no_mangle]` definition one archive per program must own
- [ ] A feature gating a `#[no_mangle]` definition that is not a C-FFI export is named for what it defines,
      not `ffi`, and is activated from the consumer's `ffi` rather than beside the dependency it belongs to
- [ ] `src/lib.rs` gates the FFI module with `#[cfg(feature = "ffi")] pub mod ffi;`.
- [ ] All `extern "C"` exports live inside `src/ffi.rs` (or submodules under `src/ffi/`), not scattered across other modules.
- [ ] Every exported symbol uses the `__nx_<aspect>__<symbol>` naming with `#[unsafe(no_mangle)] pub extern "C"`.
- [ ] Symbol signatures match the upstream archive's prototype exactly (integer widths, signedness, pointer mutability).
- [ ] The producer's `meson.build` does NOT pass `--features ffi` to `cargo build`.
- [ ] A crate with no `src/ffi.rs` does NOT declare an `ffi` feature.
- [ ] An `ffi` module holding only C-shaped backing for another crate's exports defines no `__nx_*` symbols of
      its own.

## References

- [crates-rt](crates-rt.md) - Related: Owns which crate defines a runtime symbol, when the backing for it lives in another
- [rust-process-wide-state](rust-process-wide-state.md) - Related: Owns the one-definition-per-program
  discipline the second exception in §2 rides on, and why a build carrying `extern-state` never enables `ffi`
- [meson-linker-script](meson-linker-script.md) - Related: `*_override.ld` linker scripts that consume the symbols defined here
- [meson-subproject-crate](meson-subproject-crate.md) - Related: Rust-crate subproject layout and `meson.build` Cargo wiring
- [rust-crates](rust-crates.md) - Related: `Cargo.toml` feature naming and ordering rules. It owns the general
  requirements every feature meets; this document owns the `ffi` feature's value and comment, and governs
  where the two appear to differ
