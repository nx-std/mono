---
name: "meson-linker-script"
description: "`*_override.ld` linker scripts that redirect symbols (from `libnx`, `newlib`, or any other static archive) to a crate's `__nx_<aspect>__*` Rust implementations, plus the Meson wiring that exposes the script to downstream consumers. Load when adding, modifying, or auditing an `<aspect>_override.ld` file or its `<crate>_ld_override` Meson variable"
type: arch
scope: "global"
---

# Subproject Crate — Linker Symbol Override Scripts

**MANDATORY for ALL crates under `subprojects/<crate>/` that ship an `<aspect>_override.ld`**

## Table of Contents

1. [How Symbol Override Works](#1-how-symbol-override-works)
2. [When to Add an Override Script](#2-when-to-add-an-override-script)
3. [Filename Convention](#3-filename-convention)
4. [File Contents](#4-file-contents)
5. [Meson Wiring](#5-meson-wiring)
6. [Multi-Fragment Overrides (`overrides/` Layout)](#6-multi-fragment-overrides-overrides-layout)
7. [Checklist](#checklist)

The Rust-side counterpart — the `ffi` Cargo feature that defines the `__nx_<aspect>__*` symbols this script targets — is documented in [rust-ffi](rust-ffi.md). The two docs MUST stay in lockstep: a symbol referenced by an override script that is not exported by the Rust `ffi` module is a hard link error.

---

## 1. How Symbol Override Works

A linker override script lets us replace any symbol provided by an upstream archive (most commonly `libnx`, but also `newlib`/`libc`, devkitPro support libraries, or any other static library) with a Rust implementation **at link time**, without patching the upstream archive itself. The mechanism is purely symbol-name driven: if Rust exports a symbol with the same name the archive uses, the linker has a choice between the two definitions; the override script forces that choice.

The mechanism, in four steps:

1. The Rust crate compiles with `--features ffi`, producing exported symbols named `__nx_<aspect>__<symbol>` (see [rust-ffi](rust-ffi.md)). The `__nx_<aspect>__` prefix prevents accidental collisions with upstream archives that already define the bare name.
2. The override script's `EXTERN(...)` declarations force the linker to keep those Rust symbols even if no caller references them by their `__nx_*` name. Without this, dead-code elimination would drop the Rust replacement.
3. The override script aliases each upstream name to its Rust replacement: `<symbol> = __nx_<aspect>__<symbol>;`. The left-hand side is the original symbol as it appears in the upstream archive (e.g., `svcSetHeapSize` from `libnx`, `_malloc_r` from `newlib`).
4. When the final NRO is linked with both the upstream archive and the umbrella `staticlib`, the linker sees the name defined twice. The alias wins because the override script is passed via `-T <path>` and is processed before object scanning, marking the alias as the canonical definition.

Because each override is supplied as a separate `-T` argument, the mechanism is **opt-in per script**: a consumer may depend on a crate's Rust artifact (its rlib + Rust-facing API) without taking that crate's symbol overrides.

### Typical Override Targets

The dominant case in this workspace is `libnx`, but the same mechanism applies to any archive linked into the final binary. Current examples:

| Source archive                | Symbol style                     | Example                                         |
|-------------------------------|----------------------------------|-------------------------------------------------|
| `libnx` (kernel SVCs)         | `svcCamelCase`                   | `svcSetHeapSize`                                |
| `libnx` (services, IPC, sync) | `serviceCamelCase` / `mutexInit` | `smGetService`, `mutexLock`                     |
| `newlib` (libc)               | `_<name>_r` (reentrant)          | `_malloc_r`, `_free_r`, `_realloc_r`            |
| Any archive on the final link | whatever names it ships          | The mechanism is name-driven, not source-driven |

If you need to override symbols from a new archive (e.g., a future libdeko3d or devkitPro helper), the same recipe applies — only the names of the left-hand-side aliases change.

---

## 2. When to Add an Override Script

Add an `<aspect>_override.ld` **only** when the crate replaces one or more symbols that an upstream archive (most often `libnx`) would otherwise provide:

| Crate type                                 | Override script?                                  |
|--------------------------------------------|---------------------------------------------------|
| Foundation (`nx-svc`, `nx-sys-mem`, ...)   | YES                                               |
| Higher-level (`nx-alloc`, `nx-rand`, ...)  | YES — typically `libnx`, `nx-alloc` also `newlib` |
| Service framework (`nx-sf`)                | YES                                               |
| Service IPC client (`nx-service-*`)        | NO (pure Rust API, no upstream replacement)       |
| Runtime family (`nx-rt-*`)                 | YES — split under `overrides/` (Section 6)        |

If a crate has no override script, omit both the `.ld` file and the `<crate>_ld_override` Meson variable. The corresponding Rust crate should also omit the `ffi` Cargo feature (see [rust-ffi](rust-ffi.md#6-crates-without-an-ffi-surface)).

---

## 3. Filename Convention

`<aspect>_override.ld` at the crate root, where `<aspect>` is the same slug used in the `__nx_<aspect>__*` symbol prefix — the short, domain-meaningful name (often the trailing segment of the crate name with hyphens flattened):

| Crate                  | Override filename            |
|------------------------|------------------------------|
| `nx-svc`               | `svc_override.ld`            |
| `nx-alloc`             | `alloc_override.ld`          |
| `nx-rand`              | `rand_override.ld`           |
| `nx-time`              | `time_override.ld`           |
| `nx-sys-mem`           | `sys_mem_override.ld`        |
| `nx-sys-sync`          | `sys_sync_override.ld`       |
| `nx-sys-thread-tls`    | `thread_tls_override.ld`     |
| `nx-std-sync`          | `std_sync_override.ld`       |
| `nx-sf`                | `sf_override.ld`             |

The filename slug MUST match the `<aspect>` segment of `__nx_<aspect>__*` in the Rust FFI module. Mismatches break grep-based audits and confuse override-script reviewers.

---

## 4. File Contents

For every overridden symbol the script MUST contain two entries:

1. `EXTERN(__nx_<aspect>__<symbol>)` — force-pull the Rust symbol so dead-code elimination cannot drop it.
2. `<symbol> = __nx_<aspect>__<symbol>;` — alias the upstream name to the Rust implementation.

### Example: `libnx` SVCs (`nx-svc`)

```ld
/* Static linker script for SVC function symbols redirection */
/* Redirects libnx svc* functions to nx-svc __nx_svc__svc_* implementations */

/* Memory management */
EXTERN(__nx_svc__svc_set_heap_size);
EXTERN(__nx_svc__svc_set_memory_permission);
EXTERN(__nx_svc__svc_set_memory_attribute);
EXTERN(__nx_svc__svc_map_memory);
...

svcSetHeapSize        = __nx_svc__svc_set_heap_size;
svcSetMemoryPermission = __nx_svc__svc_set_memory_permission;
svcSetMemoryAttribute = __nx_svc__svc_set_memory_attribute;
svcMapMemory          = __nx_svc__svc_map_memory;
...

/* Process and thread management */
EXTERN(__nx_svc__svc_exit_process);
...
```

### Example: `newlib` allocator (`nx-alloc`)

The exact same pattern targets `newlib`'s reentrant allocator API. The left-hand-side names come from `newlib`, not `libnx`:

```ld
/* Static linker script for allocation function symbols redirection */
/* Redirects newlib reentrant allocator functions to nx-alloc __nx_alloc__* implementations */

EXTERN(__nx_alloc__newlib_malloc_r)
EXTERN(__nx_alloc__newlib_calloc_r)
EXTERN(__nx_alloc__newlib_realloc_r)
EXTERN(__nx_alloc__newlib_memalign_r)
EXTERN(__nx_alloc__newlib_free_r)

_malloc_r   = __nx_alloc__newlib_malloc_r;
_calloc_r   = __nx_alloc__newlib_calloc_r;
_realloc_r  = __nx_alloc__newlib_realloc_r;
_memalign_r = __nx_alloc__newlib_memalign_r;
_free_r     = __nx_alloc__newlib_free_r;
```

Notice that the Rust-side names embed the source archive (`__nx_alloc__newlib_*`) so a future override targeting a different allocator (e.g., `__nx_alloc__bionic_*`) can coexist without renaming.

The two examples above use the same EXTERN-plus-alias recipe — only the left-hand-side names differ, because they come from different upstream archives. The override mechanism makes no distinction between archives.

### Style Rules

- Group related symbols under `/* Block comment */` section headers (memory, threads, sync, IPC, …) for readability and easy diffing. Section ordering should match the Rust `ffi` module layout when possible.
- The `EXTERN(...)` form may or may not be terminated with a semicolon — GNU ld accepts both. Keep the choice consistent within a single script and match the surrounding crates' style.
- Put the entire `EXTERN(...)` block for a section first, then the alias assignments. Do not interleave the two — it makes auditing a section much harder.
- Two leading comment lines describe what the script redirects, including the **source archive** ("Redirects `<archive>` `<domain>` functions to `<crate>` `__nx_<aspect>__*` implementations"). Keep this header up to date when refactoring.

### Failure Modes

- **Symbol named in the script but not exported by the Rust crate** → undefined-reference link error on the final NRO. Confirm `src/ffi.rs` exports the matching `#[unsafe(no_mangle)] pub extern "C" fn __nx_<aspect>__...`.
- **Symbol exported by Rust but not listed in the override** → the upstream implementation is still used; the Rust replacement silently does nothing. Audit by grepping for `__nx_<aspect>__` in `src/ffi.rs` and confirming each appears in the `.ld` file.
- **Slug mismatch between script and Rust prefix** → undefined reference (the EXTERN names a symbol that does not exist). Make sure the filename slug, the `__nx_<aspect>__` prefix, and the Meson variable name all use the same slug.
- **Symbol name collision between two overrides** → if two scripts both alias the same upstream name, the order in which Meson appends `-T` arguments determines which Rust implementation wins, and there is no warning. Avoid this by partitioning the override surface so each upstream symbol is owned by exactly one crate.
- **Override passed `-T` but symbol referenced before script processing** → typically caught only by linking the final NRO; foundation overrides like `nx-svc` should always be passed before higher-level overrides for clean diagnostics.

---

## 5. Meson Wiring

The producer's `meson.build` exposes the override path as a sibling variable to `<crate>_dep` (outside `declare_dependency()`):

```meson
#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
# Linker script for overriding <archive> <aspect> functions
nx_<aspect>_ld_override = meson.current_source_dir() / '<aspect>_override.ld'

nx_<aspect>_dep = declare_dependency(
    sources : nx_<aspect>_tgt,
    dependencies : deps,
)
```

### Rules

- Place the `nx_<aspect>_ld_override` assignment immediately above `declare_dependency(...)`, separated by a one-line comment that names the override domain.
- Use `meson.current_source_dir() / '<aspect>_override.ld'` so the path is absolute and survives subproject relocation.
- Do NOT add the override to `link_args` or `link_with` inside the producer's own `declare_dependency()`. The override is consumer-driven: only the final binary chooses to apply it.
- The variable name MUST be `<crate>_ld_override` with snake_case mirror of the crate (`nx_<aspect>` becomes `nx_<aspect>_ld_override`). Downstream consumers locate the script via `subproject('nx-<aspect>').get_variable('nx_<aspect>_ld_override')`.

### Consumer-Side Wiring

Downstream consumers retrieve the path and add it as a `-T` linker argument:

```meson
# In the umbrella or NRO Meson file
nx_svc_proj = subproject('nx-svc')
override_args = ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]
```

The umbrella aggregates multiple `<crate>_ld_override` paths into a single list (e.g., `deps_override_link_args`) that is passed to the final NRO link.

---

## 6. Multi-Fragment Overrides (`overrides/` Layout)

When a crate's override surface is partitioned — by override target and/or an orthogonal per-feature axis — use an `overrides/` subdirectory and expose a **list** of paths instead of a single variable. Name each fragment `<crate>_<archive>_<axis>.ld`: the leading segment is the owning crate (so fragments stay unambiguous once several crates' `overrides/` directories are aggregated into one final link), the middle segment is the override target (matching the `__nx_<aspect>__<archive>_*` symbol prefix and the `src/ffi/<archive>/` submodule — see [rust-ffi](rust-ffi.md)), the trailing segment is the partition axis (`core` for the always-linked fragment, `service_<name>` for a per-feature one).

The `nx-rt-*` runtime family uses this layout. Every fragment targets `libnx`, so each crate's `overrides/` holds one `<crate>_libnx_*.ld` family, mirrored by a single `src/ffi/libnx/` submodule tree:

```
subprojects/nx-rt-nro/
├── Cargo.toml
├── meson.build
├── overrides/
│   ├── rt_nro_libnx_core.ld              # always present
│   ├── rt_nro_libnx_service_apm.ld
│   ├── rt_nro_libnx_service_applet.ld
│   └── rt_nro_libnx_service_<name>.ld    # one per service feature
└── src/
    └── ffi/
        └── libnx/                        # FFI symbols grouped by the same target
```

The Meson wiring conditionally appends fragments based on setup-time options and exposes them as a list:

```meson
ld_overrides = [meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_core.ld']

if get_option('use_nx_service_apm').enabled()
    # ... subproject + deps wiring ...
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_apm.ld'
endif

# ... one block per service feature ...

nx_rt_nro_ld_overrides = ld_overrides   # plural variable: list of paths
```

Downstream consumers iterate the list when building `-T` arguments:

```meson
foreach script : nx_rt_nro_proj.get_variable('nx_rt_nro_ld_overrides')
    deps_override_link_args += ['-T', script]
endforeach
```

Use this layout only when fragments are genuinely independent. If overrides simply grow large, a single well-sectioned `<aspect>_override.ld` remains preferable — it is easier to audit and less error-prone than a sprawl of `.ld` files.

---

## References

- [rust-ffi](rust-ffi.md) - Related: The `ffi` Cargo feature that defines the `__nx_<aspect>__*` symbols this script targets
- [meson-subproject-crate](meson-subproject-crate.md) - Related: Rust-crate subproject layout and `meson.build` Cargo wiring
- [meson-subproject](meson-subproject.md) - Related: Generic Meson subproject conventions

## Checklist

Before committing changes to an `<aspect>_override.ld` or its Meson wiring, verify:

### Override script

- [ ] Filename is `<aspect>_override.ld` at the crate root (or under `overrides/` for multi-fragment crates).
- [ ] `<aspect>` slug matches the `__nx_<aspect>__` prefix used in the Rust `ffi` module and the `nx_<aspect>_ld_override` Meson variable.
- [ ] Every overridden upstream symbol has both an `EXTERN(__nx_<aspect>__<symbol>)` declaration and an `<symbol> = __nx_<aspect>__<symbol>;` alias.
- [ ] The Rust crate exports each referenced `__nx_<aspect>__*` symbol behind `#[cfg(feature = "ffi")]` (see [rust-ffi](rust-ffi.md)).
- [ ] Related symbols are grouped under `/* ... */` block comments, with EXTERN block first and assignments second.
- [ ] Two-line header comment at the top of the script names the source archive and the override domain.
- [ ] No other override script in the workspace aliases the same upstream symbol name (Section 4 — Symbol name collision).

### Meson wiring

- [ ] `nx_<aspect>_ld_override = meson.current_source_dir() / '<aspect>_override.ld'` is declared as a sibling variable to `<crate>_dep`, NOT inside `declare_dependency()`.
- [ ] The variable is placed immediately above `declare_dependency(...)`, preceded by a one-line descriptive comment.
- [ ] The override is NOT added to `link_args` or `link_with` inside the producer's own `declare_dependency()`.
- [ ] Multi-fragment crates expose a plural `<crate>_ld_overrides` list and conditionally append per-feature fragments.
