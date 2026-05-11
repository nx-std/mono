# Workspace

This document describes the workspace-level organization of this repository: the hybrid Cargo + Meson root, the crate category hierarchy, dependency direction rules, and ordering requirements for the root manifests.

## Table of Contents

1. [Hybrid Cargo + Meson Workspace](#1-hybrid-cargo--meson-workspace)
2. [Crate Category Hierarchy](#2-crate-category-hierarchy)
3. [Dependency Rules per Category](#3-dependency-rules-per-category)
4. [Root `Cargo.toml`](#4-root-cargotoml)
5. [Root `meson.build`](#5-root-mesonbuild)
6. [Ordering Requirements](#6-ordering-requirements)
7. [Checklist](#checklist)

Per-crate Meson/Cargo wiring (the producer-side `meson.build` skeleton, Cargo manifest baseline, and Cargo/Meson dependency mirror) is documented in [code/meson-subproject-crate](code/meson-subproject-crate.md), which extends the generic [code/meson-subproject](code/meson-subproject.md) conventions. This doc covers only the workspace-level concerns above the per-subproject layer.

---

## 1. Hybrid Cargo + Meson Workspace

The repository is **simultaneously a Cargo workspace and a Meson project**. Both rooted at the repository top level, but they index different sets of files:

| System    | Root manifest      | What it sees                                                                              |
|-----------|--------------------|-------------------------------------------------------------------------------------------|
| **Cargo** | `Cargo.toml`       | Every Rust crate under `subprojects/<crate>/` listed in `[workspace].members`             |
| **Meson** | `meson.build`      | Every Meson subproject under `subprojects/<name>/` (Rust crates + C libraries + binaries) |

The two views overlap on the Rust crates (each Cargo member also has a `meson.build`) but diverge on the C side: `subprojects/libnx/`, `subprojects/libnx-dkp/`, `subprojects/libdeko3d-dkp/`, `subprojects/sysroot/`, `subprojects/sysroot-dkp/`, `subprojects/tests/`, `subprojects/examples/`, `subprojects/nx-hbmenu/`, and `subprojects/vendor/` are visible only to Meson.

```
mono/
├── Cargo.toml                # Cargo workspace root
├── meson.build               # Meson project root
├── meson.options             # Setup-time options (e.g., use_libnx_dkp)
└── subprojects/
    ├── nx-svc/               # Rust crate — both Cargo member and Meson subproject
    │   ├── Cargo.toml
    │   └── meson.build
    ├── nx-std/               # Rust crate — umbrella staticlib
    │   ├── Cargo.toml
    │   └── meson.build
    ├── libnx/                # C library — Meson-only
    │   └── meson.build
    ├── tests/                # Test NROs — Meson-only, links against Rust + libnx
    │   └── meson.build
    └── ...
```

### Build Flow

1. `meson setup buildDir` reads the root `meson.build`, recursively enters each declared subproject, and stitches dependency graphs.
2. Inside each Rust subproject's `meson.build`, a `custom_target` shells out to `cargo build --package <name>` (see [code/meson-subproject-crate](code/meson-subproject-crate.md)).
3. Cargo resolves the Rust dependency graph from the root `Cargo.toml` and shares a single `target/` directory across the workspace (`buildDir/cargo-target/`).
4. The umbrella crate (`nx-std`) is the only Rust target that produces a `staticlib`; final NRO binaries (defined under `subprojects/tests/`, `subprojects/examples/`, …) link against that `.a`, against `libnx`, and against any per-crate `<aspect>_override.ld` scripts (see [code/meson-linker-script](code/meson-linker-script.md)).

The two graphs deliberately stay in lockstep but serve different needs: Cargo drives the actual Rust link order, Meson schedules `custom_target` invocations and propagates C include directories. The detailed mirror rule lives in [code/meson-subproject-crate](code/meson-subproject-crate.md#5-cargo--meson-dependency-mirroring).

---

## 2. Crate Category Hierarchy

Cargo workspace members live under `subprojects/<crate>/` and are split into the following architectural layers, from foundation to leaf:

### `sys/*` — Foundation Layer

**Purpose**: Direct interface to Horizon OS primitives. Foundation for everything else.

**What belongs here:**

- **`nx-svc`**: Raw Supervisor Call (SVC) bindings — the layer everything else depends on.
- **`nx-cpu`**: CPU-level utilities (cache, registers).
- **`nx-sys-mem`**: Low-level memory management on top of `nx-svc`.
- **`nx-sys-sync`**: Low-level synchronization primitives on top of `nx-svc`.
- **`nx-sys-thread`**, **`nx-sys-thread-tls`**: Thread management.

### Higher-level Crates — Standard-library-style abstractions

**Purpose**: `std`-flavoured abstractions built on the `sys/*` layer.

**What belongs here:**

- **`nx-alloc`**: Global allocator (uses `nx-svc` + `nx-sys-sync`).
- **`nx-rand`**: Random number generation.
- **`nx-time`**: Time utilities.
- **`nx-std-sync`**: High-level sync primitives (`Mutex`, `RwLock`, …).
- **`nx-rt`**: Runtime support.
- **`nx-panic-handler`**: Panic handler.

### Service Crates (`nx-service-*`) — Horizon OS Services

**Purpose**: Bindings to specific Horizon OS services exposed via IPC.

**What belongs here:**

- **`nx-sf`**: Service framework primitives.
- **`nx-service-sm`**, **`nx-service-time`**, **`nx-service-applet`**, **`nx-service-hid`**, **`nx-service-vi`**, **`nx-service-set`**, **`nx-service-apm`**, **`nx-service-nv`**: Per-service IPC clients.

### `nx-std` — Umbrella Staticlib

**Purpose**: Single `staticlib` crate that re-exports the FFI symbols (`__nx_*`) consumed by linker overrides. Each enabled higher-level / `sys/*` / service crate exposes its FFI surface via a public `ffi` module behind an `ffi` Cargo feature; `nx-std` re-exports them based on enabled features.

This is the only Rust crate that produces a linkable artifact for the C side.

### Meson-only Subprojects

These ship `meson.build` files but no `Cargo.toml` and are NOT Cargo workspace members:

- **`libnx`** / **`libnx-dkp`**: The C homebrew library, either built from source or sourced as a devkitPro prebuilt (toggled by `use_libnx_dkp`).
- **`sysroot`** / **`sysroot-dkp`**: System root manifest mapping devkitPro and toolchain headers/libs for Meson.
- **`libdeko3d-dkp`**: devkitPro graphics library prebuilt.
- **`vendor`**: Vendored third-party C sources.
- **`tests`**: Switch-hardware NRO test suite (C code linking against the Rust crates to verify FFI correctness).
- **`examples`**: Example NROs.
- **`nx-hbmenu`**: The homebrew menu binary.

---

## 3. Dependency Rules per Category

| From \ To              | `sys/*` | higher-level | service | `nx-std` |
|------------------------|:-------:|:------------:|:-------:|:--------:|
| **`sys/*`**            | ✅       | ❌            | ❌       | ❌        |
| **higher-level**       | ✅       | ✅            | ❌       | ❌        |
| **service**            | ✅       | ✅            | ✅       | ❌        |
| **`nx-std`** umbrella  | ✅       | ✅            | ✅       | ❌        |

**Key rules:**

- **`sys/*` crates** depend only on other `sys/*` crates and `nx-svc`. They NEVER depend on higher-level, service, or umbrella crates.
- **Higher-level crates** depend on `sys/*` and other higher-level crates. They MUST NOT depend on service or umbrella crates.
- **Service crates** depend on `sys/*`, higher-level, and other service crates as needed (e.g., service-applet depends on service-sm). They MUST NOT depend on the `nx-std` umbrella.
- **`nx-std`** is the sink: every other crate may flow into it; nothing depends on `nx-std`.
- **No circular dependencies** at any layer.
- The `ffi` feature on each crate gates its FFI module; `nx-std` enables exactly those `ffi` features that match the Meson `use_nx*` setup-time options.

Meson-only subprojects (libnx, tests, examples, …) sit outside this matrix — they consume the Rust side but are not consumed by it.

---

## 4. Root `Cargo.toml`

The root manifest declares the Cargo workspace and lists every Rust crate by path:

```toml
[workspace]
resolver = "2"
members = [
    "subprojects/nx-alloc",
    "subprojects/nx-cpu",
    "subprojects/nx-panic-handler",
    # ...alphabetical...
    "subprojects/nx-time",
]
```

### Rules

- **`resolver = "2"`** is mandatory.
- **`members` MUST be alphabetically ordered** (`subprojects/<name>` keys).
- New Rust crates MUST be added here when introduced under `subprojects/`. A missing member entry causes `cargo build --package <name>` (invoked by the per-crate `meson.build`) to fail with "package not found in workspace".
- The root manifest declares NO `[dependencies]` and NO `[package]` — it is workspace-only.
- Do NOT introduce a `[workspace.dependencies]` table without team alignment; the current convention is per-crate `version + path` pinning (see [code/meson-subproject-crate](code/meson-subproject-crate.md#3-cargotoml-conventions)).

Meson-only subprojects MUST NOT appear in the `members` list.

---

## 5. Root `meson.build`

The Meson root project is named `switchbrew` and orchestrates the high-level build:

```meson
project('switchbrew', meson_version: '>= 1.7.0')

## Subprojects
# Libraries
subproject('sysroot-dkp')

if get_option('use_libnx_dkp').disabled()
    subproject('libnx')
else
    subproject('libnx-dkp')
endif

# Executables
subproject('tests')
subproject('examples')
subproject('nx-hbmenu')
```

### Rules

- **Project name**: `switchbrew` (not `nx-std`, not the repo directory name). Do not rename.
- **`meson_version: '>= 1.7.0'`** is the workspace baseline.
- **Only declare the binaries / top-level C libraries here.** Per-crate Rust subprojects are pulled in transitively by `subprojects/nx-std/meson.build` (or by `subprojects/tests/`, etc.) — listing them at the root would double-load them.
- **Library / executable split** is preserved via the `# Libraries` / `# Executables` comment banners. Maintain the banners when adding new top-level subprojects.
- **Conditional libnx**: the libnx-from-source vs devkitPro-prebuilt switch happens here via `use_libnx_dkp`. Per-crate code never branches on this option directly.

Setup-time options for the workspace live in the root `meson.options`; per-subproject options (such as the `use_nx_*` toggles consumed by the umbrella) live in `subprojects/<name>/meson.options`. See [code/meson-subproject](code/meson-subproject.md) for the generic `meson.options` format and [code/meson-subproject-crate](code/meson-subproject-crate.md) for the per-crate layout.

---

## 6. Ordering Requirements

### Cargo Workspace Members

The root `Cargo.toml` `members` array MUST be ordered alphabetically by `subprojects/<name>` path.

**Rationale**: Consistent merge conflict resolution, predictable diffs, and easy visual scanning.

### Dependencies in `Cargo.toml`

All `Cargo.toml` dependency sections (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`) MUST be ordered alphabetically ([code/rust-crate](code/rust-crate.md)).

**Rationale**: Same as above. The Cargo/Meson mirror in each per-crate `meson.build` references these in the same order, making the relationship easy to audit.

### Meson Subproject Calls

Top-level `subproject(...)` calls in the root `meson.build` are grouped by purpose (`Libraries` first, `Executables` second), then ordered to match the build dependency direction within each group (sysroot before libnx; libnx before binaries). Strict alphabetical ordering is NOT required at the root because dependency direction must drive the order.

---

## See Also

- [code/meson-subproject](code/meson-subproject.md) — Generic Meson subproject conventions
- [code/meson-subproject-crate](code/meson-subproject-crate.md) — Rust-crate specialization of the Meson subproject layout
- [code/rust-crate](code/rust-crate.md) — Crate manifest conventions
- [code/rust-ffi](code/rust-ffi.md) — `ffi` Cargo feature contract for the FFI surface
- [code/meson-linker-script](code/meson-linker-script.md) — `*_override.ld` linker scripts

## Checklist

Before committing workspace-level changes, verify:

### Crate placement and direction

- [ ] New crates are placed in the correct architectural layer (`sys/*` foundation, higher-level, service, or umbrella).
- [ ] Dependency direction follows the rules (no upward edges, no cycles).
- [ ] `sys/*` crates have no dependencies on higher-level, service, or `nx-std` crates.
- [ ] Meson-only subprojects (libnx, tests, examples, …) are NOT Cargo workspace members.

### Root manifests

- [ ] Root `Cargo.toml` `members` array contains every Rust subproject and is alphabetically ordered.
- [ ] Root `Cargo.toml` declares no `[package]` and no global `[dependencies]`.
- [ ] Root `meson.build` keeps the `# Libraries` / `# Executables` grouping and does not list per-crate Rust subprojects.
- [ ] `meson_version` baseline (`>= 1.7.0`) is unchanged or bumped intentionally.

### Cargo manifests (per-crate)

- [ ] All `Cargo.toml` dependency sections are alphabetically ordered.

### FFI surface

- [ ] If the crate exposes a C-FFI surface, it lives behind an `ffi` Cargo feature and is re-exported by `nx-std` when the corresponding `use_nx*` Meson option is enabled (see [code/rust-ffi](code/rust-ffi.md)).
