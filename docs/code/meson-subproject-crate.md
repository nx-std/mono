---
name: "meson-subproject-crate"
description: "Rust-crate specialization of the Meson subproject layout: required directory layout (Cargo.toml, src/lib.rs), Cargo manifest baseline, `custom_target` invocation of cargo, and Cargo/Meson dependency mirroring. Load when creating a new subproject crate, editing `subprojects/<crate>/meson.build`, or wiring a Cargo workspace member into Meson"
type: "arch"
scope: "global"
---

# Meson Subproject — Rust Crate Specialization

**MANDATORY for ALL Rust-crate subprojects under `subprojects/<crate>/` in this workspace**

This document refines the rules in [meson-subproject](meson-subproject.md) for the Rust-crate case (i.e., subprojects that are also Cargo workspace members). Read [meson-subproject](meson-subproject.md) first — the generic skeleton, section banner format, `meson.options` shape, and variable-naming rules are not repeated here.

The C-FFI surface (`ffi` Cargo feature) is documented in [rust-ffi](rust-ffi.md); the `*_override.ld` linker scripts that consume it are documented in [meson-linker-script](meson-linker-script.md).

## Table of Contents

1. [Directory Layout](#1-directory-layout)
2. [Crate-Specific Naming](#2-crate-specific-naming)
3. [`Cargo.toml` Conventions](#3-cargotoml-conventions)
4. [`meson.build` Cargo Wiring](#4-mesonbuild-cargo-wiring)
5. [Cargo / Meson Dependency Mirroring](#5-cargo--meson-dependency-mirroring)
6. [Checklist](#checklist)

---

## 1. Directory Layout

On top of the generic [meson-subproject](meson-subproject.md#2-directory-layout) requirements, a Rust-crate subproject also contains a Cargo manifest and a Rust source tree:

```
subprojects/<crate-kebab-name>/
├── Cargo.toml                 # REQUIRED — Cargo package manifest
├── meson.build                # REQUIRED — Meson subproject wrapper around cargo
├── src/                       # REQUIRED — Rust sources
│   ├── lib.rs                 # REQUIRED
│   └── ffi.rs                 # OPTIONAL — C-FFI surface, gated by `ffi` feature
├── include/                   # OPTIONAL — public C headers exposed to consumers
│   └── nx_<aspect>.h
├── <aspect>_override.ld       # OPTIONAL — linker fragment redirecting libnx symbols
├── overrides/                 # OPTIONAL — multi-fragment linker overrides
└── meson.options              # OPTIONAL — only when the crate exposes setup-time options
```

### Crate-Specific Required Files

- `Cargo.toml` — declared as a workspace member in the root `Cargo.toml` (see [../workspace](../workspace.md)).
- `src/lib.rs` — `no_std` library entry point.

### Crate-Specific Optional Files

- `include/` — present when the crate exposes a public C header. Must be a sibling of `src/`, registered through `include_directories('include')` and propagated via `declare_dependency(include_directories : inc, ...)`.
- `<aspect>_override.ld` / `overrides/` — present **only** for crates that override `libnx` (or other archive) symbols. Foundation and higher-level crates typically have one; service crates (`nx-service-*`) typically do not. See [meson-linker-script](meson-linker-script.md).
- `meson.options` — declare crate-specific setup-time options. For per-feature toggles consumed by `nx-std`, the option lives in `subprojects/nx-std/meson.options`, not in the producer crate.

---

## 2. Crate-Specific Naming

In addition to the [generic naming rules](meson-subproject.md#3-naming-conventions), Rust crates also expose Cargo-side names that MUST stay aligned with the Meson side:

| Artifact                    | Convention                              | Example                       |
|-----------------------------|-----------------------------------------|-------------------------------|
| Cargo `package.name`        | `kebab-case` (matches directory)        | `nx-sys-thread-tls`           |
| Cargo `lib.name`            | `snake_case` of the package name        | `nx_sys_thread_tls`           |
| Cargo build output          | `lib<lib.name>.rlib` (+ `.a` for umbrella) | `libnx_sys_thread_tls.rlib`|
| Linker override filename    | `<aspect>_override.ld` at crate root    | `thread_tls_override.ld`      |
| FFI symbol prefix           | `__nx_<aspect>__<libnx_symbol>`         | `__nx_svc__svc_set_heap_size` |

The `<aspect>` slug used by the linker override filename and the FFI prefix is the short, domain-meaningful tail of the crate name (`nx-sys-thread-tls` → `thread_tls`). It MUST match between the `.ld` filename, the `__nx_<aspect>__` symbol prefix, and the `<crate>_ld_override` Meson variable (see [meson-linker-script](meson-linker-script.md)).

---

## 3. `Cargo.toml` Conventions

Subproject `Cargo.toml` files follow [rust-crates](rust-crates.md) section ordering, with the additional constraints below.

### `[package]`

```toml
[package]
name = "nx-<aspect>"
version = "0.1.0"
edition = "2024"
```

- `name` MUST match the subproject directory name (kebab-case).
- All crates use `version = "0.1.0"` and `edition = "2024"`.

### `[lib]`

```toml
[lib]
name = "nx_<aspect>"            # snake_case mirror of package name
crate-type = ["rlib"]
test = false
doctest = false
bench = false
```

- Producer crates always declare `crate-type = ["rlib"]`. Only the umbrella crate is allowed to add `"staticlib"`, since a single `staticlib` per link is required for the final NRO; producer crates MUST NOT add it.
- `test`, `doctest`, and `bench` are disabled at the manifest level — tests run as hardware NROs through `subprojects/tests/`, not via `cargo test`.

### `[features]`

The `[features]` section is OPTIONAL ([rust-crates](rust-crates.md)). The canonical feature for this workspace is `ffi`; its declaration, source-gating, and symbol-naming rules live in [rust-ffi](rust-ffi.md).

### `[dependencies]` — Workspace Siblings

Workspace siblings MUST be declared with `path` AND `version`:

```toml
[dependencies]
nx-panic-handler = { version = "0.1.0", path = "../nx-panic-handler" }
nx-svc = { version = "0.1.0", path = "../nx-svc" }
```

- Always relative path `../<dep-kebab-name>`.
- Always pin `version = "0.1.0"` to match the workspace baseline.
- Dependency direction follows [../workspace](../workspace.md).
- Every Cargo workspace dependency MUST have a matching Meson `subproject(...)` entry. See [Cargo / Meson Dependency Mirroring](#5-cargo--meson-dependency-mirroring).

---

## 4. `meson.build` Cargo Wiring

The crate's `meson.build` follows the generic skeleton from [meson-subproject](meson-subproject.md#4-mesonbuild-skeleton). The crate-specific differences are concentrated in three places: the cargo discovery line, the `custom_target` that drives the build, and the absence of native source-file sections.

### 4.1 Skeleton

```meson
project('nx-<aspect>', version : '0.1.0')

cargo = find_program('cargo', required : true)

#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
# Rust dependencies here are just informative so Meson can build the dependencies in the correct order
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

deps = [
    nx_panic_handler_dep,
]

#---------------------------------------------------------------------------------
# Static library
#---------------------------------------------------------------------------------
inc = include_directories('include')          # omit when the crate has no public C header

nx_<aspect>_tgt = custom_target(
    'nx-<aspect>',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_<aspect>.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,
)

#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
nx_<aspect>_ld_override = meson.current_source_dir() / '<aspect>_override.ld'   # only when the crate overrides symbols

nx_<aspect>_dep = declare_dependency(
    include_directories : inc,                # omit when no `include/`
    sources : nx_<aspect>_tgt,
    dependencies : deps,
)
```

Source-file blocks (`Source files`, `Data files`) from the generic skeleton are omitted — Cargo discovers Rust sources via `Cargo.toml`, so Meson never needs `files(...)` for them.

### 4.2 Required Cargo Wiring Rules

- **Project name and Cargo package**: `project('nx-<aspect>', version : '0.1.0')` MUST match the directory and the Cargo `package.name`. `meson.project_name()` is then reused as the `--package` argument to `cargo`.
- **Cargo discovery**: `cargo = find_program('cargo', required : true)` appears immediately below the project header so the `custom_target` can reference it.
- **Build profile**: do NOT hardcode the profile. Always derive it from `get_option('buildtype')` with the ternary `get_option('buildtype') == 'release' ? 'release' : 'dev'` so debug/release tracks Meson's `buildtype`.
- **Target directories**: `--target-dir` MUST be `meson.global_build_root() / 'cargo-target'` so every crate writes into the shared workspace target directory. `--artifact-dir '@OUTDIR@'` writes the published rlib into Meson's per-target output directory.
- **`output`**: list `libnx_<aspect>.rlib` (and `libnx_<aspect>.a` for the umbrella). The filename MUST match the `lib.name` (`nx_<aspect>`) produced by Cargo, with a `lib` prefix and `.rlib` extension.
- **`build_always_stale : true`**: Cargo owns up-to-date detection for Rust sources; Meson must not skip the `custom_target`. This is non-negotiable.
- **`console : true`**: keep so Cargo's progress output streams to the user.

### 4.3 Linker Override Variable

When the crate ships an `<aspect>_override.ld`, expose its path as a sibling variable to `<crate>_dep` (outside `declare_dependency()`). The full contract — when to add a script, naming, file contents, and consumer wiring — is documented in [meson-linker-script](meson-linker-script.md).

### 4.4 Umbrella-Specific Wiring (`nx-std`)

The `nx-std` umbrella crate is the only Rust-crate subproject that:

- Produces both `libnx_std.a` and `libnx_std.rlib` (`output : ['libnx_std.a', 'libnx_std.rlib']`).
- Selects Cargo features at the `cargo build` command line via `--no-default-features --features ...`, with the feature list assembled from the enabled `use_nx_*` options.
- Uses `link_with : <name>_tgt[0]` in its `declare_dependency` (indexing the staticlib output).
- Exposes an additional `<name>_dep_override_link_args` variable that aggregates per-crate override `-T` arguments for the final NRO link.

Producer crates MUST NOT replicate any of these patterns.

---

## 5. Cargo / Meson Dependency Mirroring

Cargo and Meson maintain **two independent dependency graphs that MUST stay in lockstep**:

- **Cargo's graph** drives the actual Rust link — Cargo discovers transitive `rlib` dependencies and links them into the final `staticlib`. The Meson dependency entries do not produce any linker flags for the rlib chain.
- **Meson's graph** exists solely so Meson schedules `custom_target` invocations in the right order, and so include directories / `declare_dependency` chains propagate to downstream Meson consumers (the final NRO target, C tests).

### Rule: every workspace `[dependencies]` entry has a matching `subproject(...)` block

For each workspace-internal dependency in `Cargo.toml`, `meson.build` MUST contain:

1. A `subproject('<dep>')` call.
2. A `<dep>_dep` retrieved via `get_variable('<dep>_dep')`.
3. An entry in the `deps` list passed to `declare_dependency()`.

```toml
# Cargo.toml
[dependencies]
nx-panic-handler = { version = "0.1.0", path = "../nx-panic-handler" }
nx-svc = { version = "0.1.0", path = "../nx-svc" }
nx-sys-sync = { version = "0.1.0", path = "../nx-sys-sync" }
```

```meson
# meson.build  — mirrors the Cargo deps above
nx_panic_handler_proj = subproject('nx-panic-handler')
nx_panic_handler_dep = nx_panic_handler_proj.get_variable('nx_panic_handler_dep')

nx_svc_proj = subproject('nx-svc')
nx_svc_dep = nx_svc_proj.get_variable('nx_svc_dep')

nx_sys_sync_proj = subproject('nx-sys-sync')
nx_sys_sync_dep = nx_sys_sync_proj.get_variable('nx_sys_sync_dep')

deps = [
    nx_panic_handler_dep,
    nx_svc_dep,
    nx_sys_sync_dep,
]
```

### Scope of the Mirror

- **Workspace siblings only** — only crates under `subprojects/` participate in the mirror. External crates from crates.io (e.g., `thiserror`, `bitflags`) live in `Cargo.toml` alone; Meson is unaware of them.
- **Optional Cargo dependencies** — when a workspace dep is `optional = true` and gated behind a feature, the Meson mirror is conditional too: it lives inside an `if get_option('use_nx_<aspect>').enabled()` block (see `subprojects/nx-rt/meson.build` and `subprojects/nx-std/meson.build`).
- **`nx-panic-handler`** — every `no_std` crate transitively depends on the panic handler. It MUST appear in both Cargo `[dependencies]` and the Meson `deps` list (`extern crate nx_panic_handler as _;` in `src/lib.rs`).

### Why Both Graphs Exist

Cargo cannot order builds of sibling Meson targets (C objects, header generation, install rules). Meson cannot peek into Cargo's resolved graph. The informational comment `# Rust dependencies here are just informative so Meson can build the dependencies in the correct order` is part of every `meson.build` and should be preserved when copy-templating a new crate.

---

## Checklist

Before committing a new or modified Rust-crate subproject, verify:

### Generic Meson rules

- [ ] [meson-subproject](meson-subproject.md) checklist passes (project header, section banners, `meson.options` format, variable naming).

### Crate-specific directory layout

- [ ] `Cargo.toml` and `src/lib.rs` are present alongside `meson.build`.
- [ ] `include/` exists only when the crate ships a public C header.
- [ ] `<aspect>_override.ld` or `overrides/` exists only when the crate overrides libnx (or other archive) symbols.

### Cargo manifest

- [ ] `[package]` uses `version = "0.1.0"` and `edition = "2024"`.
- [ ] `[lib]` declares `name = "nx_<aspect>"` (snake_case mirror), `crate-type = ["rlib"]` (or `["rlib", "staticlib"]` for the umbrella), and disables `test`/`doctest`/`bench`.
- [ ] Workspace siblings use `{ version = "0.1.0", path = "../<dep>" }`.
- [ ] If the crate exposes a C-FFI surface, the `ffi` feature rules in [rust-ffi](rust-ffi.md) are followed.

### `meson.build` Cargo wiring

- [ ] `cargo` is discovered via `find_program('cargo', required : true)` immediately below the project header.
- [ ] `custom_target` uses `--package meson.project_name()`, `--target-dir meson.global_build_root() / 'cargo-target'`, `--artifact-dir '@OUTDIR@'`, and the `buildtype` ternary for `--profile`.
- [ ] `output` lists `libnx_<aspect>.rlib` (and `libnx_<aspect>.a` for `nx-std`).
- [ ] `build_always_stale : true` is set so Cargo controls staleness.
- [ ] No native `Source files` / `Data files` sections appear — Cargo owns Rust source discovery.

### Cargo / Meson mirror

- [ ] Every workspace `[dependencies]` entry has a corresponding `subproject(...)` block and `deps` list member in `meson.build`.
- [ ] Optional workspace deps in Cargo are guarded by the matching `if get_option('use_nx_*').enabled()` block in Meson.
- [ ] External crates from crates.io are NOT mirrored into `meson.build`.
- [ ] `nx-panic-handler` appears in both `[dependencies]` and `deps` for every `no_std` producer crate.

### FFI surface and linker overrides

- [ ] If the crate ships a C-FFI surface, the checklist in [rust-ffi](rust-ffi.md) passes.
- [ ] If the crate ships an `*_override.ld`, the checklist in [meson-linker-script](meson-linker-script.md) passes.

## References

- [meson-subproject](meson-subproject.md) - Related: Generic Meson subproject conventions (skeleton, banners, options, naming)
- [meson-linker-script](meson-linker-script.md) - Related: `*_override.ld` linker scripts that redirect `libnx` symbols
- [rust-crates](rust-crates.md) - Related: `Cargo.toml` section ordering and feature naming rules
- [rust-ffi](rust-ffi.md) - Related: `ffi` Cargo feature contract for the C-FFI surface
- [rust-mods-files](rust-mods-files.md) - Foundation: Module layout inside `src/`
- [../workspace](../workspace.md) - See also: Workspace crate categories and dependency direction
