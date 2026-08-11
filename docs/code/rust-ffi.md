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
7. [Checklist](#checklist)

A crate's C-FFI surface and the linker scripts that redirect upstream archive symbols (`libnx`, `newlib`, etc.) to it are deliberately separated. This doc covers only the Rust-side feature contract. The linker-side override scripts that consume the symbols defined here live in [meson-linker-script](meson-linker-script.md).

---

## 1. Contract Summary

| Aspect                     | Rule                                                                                      |
|----------------------------|-------------------------------------------------------------------------------------------|
| Declaration                | `ffi = []` in `[features]`, plus any `<dep>?/ffi` whose gated item only this crate's C boundary calls ([§2](#2-declaration)). |
| Source gating              | `#[cfg(feature = "ffi")] pub mod ffi;` in `src/lib.rs`.                                   |
| Symbol naming              | `__nx_<aspect>__<symbol>` with `#[unsafe(no_mangle)] pub extern "C"`.                     |
| Direct producer build      | The producer's own `meson.build` does NOT pass `--features ffi` to `cargo build`.         |
| Crates without overrides   | Service crates (`nx-service-*`) and pure Rust utilities OMIT the `ffi` feature entirely.  |

---

## 2. Declaration

```toml
[features]
# Enable the __nx_<aspect> FFI
ffi = []
```

- The feature value is empty (`[]`) — it gates a compile-time `#[cfg(...)]` branch in the producer crate and contributes nothing else.
- No transitive feature activation belongs in this list. The `ffi` feature MUST NOT pull in additional crates or enable other features.
- The `# Enable the __nx_<aspect> FFI` comment is the canonical descriptor and must accompany the feature declaration ([rust-crates](rust-crates.md)).

### The one exception: a dependency that gates its own mapping on `ffi`

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
- **Group by override target.** When a crate's override surface spans more than one upstream archive — or is organized that way for clarity — insert the archive as a segment: `__nx_<aspect>__<archive>_<symbol>` (e.g., `__nx_alloc__newlib_malloc_r`, `__nx_rt_core__libnx_env_get_loader_info`). The archive segment matches the `src/ffi/<archive>/` submodule ([Section 3](#3-source-gating)) and the `<archive>_<axis>.ld` override-script fragment family ([meson-linker-script](meson-linker-script.md)). The `nx-rt-*` runtime family follows this layout — every fragment targets `libnx` today, so its symbols are `__nx_rt_<kind>__libnx_*` and its fragments `libnx_*.ld`.
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

A crate that has no `__nx_<aspect>__*` symbols MUST OMIT the `ffi` feature entirely. Typical examples:

- `nx-service-*` IPC clients — expose a pure Rust API; the upstream services they replace are wrapped by callers in their own bindings.
- Pure Rust utilities such as `nx-cpu` and `nx-panic-handler`.

Do NOT add `ffi = []` "for symmetry" — its presence is a strong signal that an `ffi` module exists, and downstream `nx-<aspect>?/ffi` references will fail to resolve if the feature is declared without a corresponding `src/ffi.rs`.

---

## Checklist

Before committing changes to a crate's `ffi` feature or `src/ffi.rs`, verify:

- [ ] `[features]` contains `ffi = []` with the `# Enable the __nx_<aspect> FFI` comment.
- [ ] The `ffi` feature value is `[]`, except for `<dep>?/ffi` entries that each carry the comment saying
      which item the dependency gates and why this crate is the caller
- [ ] `src/lib.rs` gates the FFI module with `#[cfg(feature = "ffi")] pub mod ffi;`.
- [ ] All `extern "C"` exports live inside `src/ffi.rs` (or submodules under `src/ffi/`), not scattered across other modules.
- [ ] Every exported symbol uses the `__nx_<aspect>__<symbol>` naming with `#[unsafe(no_mangle)] pub extern "C"`.
- [ ] Symbol signatures match the upstream archive's prototype exactly (integer widths, signedness, pointer mutability).
- [ ] The producer's `meson.build` does NOT pass `--features ffi` to `cargo build`.
- [ ] Crates without a C-FFI surface do NOT declare an `ffi` feature.

## References

- [meson-linker-script](meson-linker-script.md) - Related: `*_override.ld` linker scripts that consume the symbols defined here
- [meson-subproject-crate](meson-subproject-crate.md) - Related: Rust-crate subproject layout and `meson.build` Cargo wiring
- [rust-crates](rust-crates.md) - Related: `Cargo.toml` feature naming and ordering rules
