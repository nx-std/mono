# Workspace

This document describes the workspace-level organization of this repository: the hybrid Cargo + Meson root, the crate families, the dependency rules that hold between them, and ordering requirements for the root manifests.

## Table of Contents

1. [Hybrid Cargo + Meson Workspace](#1-hybrid-cargo--meson-workspace)
2. [Crate Families](#2-crate-families)
3. [Dependency Rules](#3-dependency-rules)
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

The two views overlap on the Rust crates (each Cargo member also has a `meson.build`) but diverge on the C side: `subprojects/libnx/`, `subprojects/libnx-dkp/`, `subprojects/libdeko3d-dkp/`, `subprojects/sysroot-dkp/`, `subprojects/tests/`, `subprojects/examples/`, and `subprojects/nx-hbmenu/` are visible only to Meson.

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

## 2. Crate Families

Every Cargo workspace member sits directly at `subprojects/<crate>/`. The directory tree is **flat**: there is no `sys/` subdirectory, and a crate's family is read off its name.

### The Prefix Names Provenance, Not Layer

A prefix says **which part of `std` a crate is destined to become**, not where it sits in the dependency graph. The two are different questions, and reading one off the other is wrong in both directions.

| Prefix | Mirrors | Example |
|---|---|---|
| `nx-svc`, `nx-cpu` | no `std` counterpart — the Horizon substrate | `nx-svc` is the SVC surface everything reaches through |
| `nx-sys-*` | a module of `std::sys` | `nx-sys-args` ↔ `std::sys::args` |
| `nx-std-*` | a top-level module of `std` | `nx-std-env` ↔ `std::env` |
| `nx-sf`, `nx-service-*` | no `std` counterpart — Horizon IPC clients | `nx-service-sm` speaks to the service manager |
| device crates (`nx-net`, `nx-fsdev`, `nx-nv`, `nx-display`, `nx-pm`, `nx-wlaninf`, `nx-netloader`) | no `std` counterpart — a device or driver above IPC | `nx-fsdev` serves the SD card as a device |
| `nx-rt-*` | the process entry runtime: `std::rt`, plus what `crt0` and the C runtime do beneath it | one entry crate per output kind ([code/crates-rt](code/crates-rt.md)) |
| `nx-std` | the umbrella `staticlib`, not a member of the `nx-std-*` family | the single linkable artifact |

Two consequences worth stating outright, because both have been misread:

- **`nx-std` is not the parent of `nx-std-*`.** It is the sink every crate flows into, and it depends on some `nx-std-*` crates and not others. The shared prefix is a collision, not a hierarchy.
- **A `nx-std-*` crate is not automatically above a `nx-sys-*` one.** `nx-std-path` is a vocabulary crate near the bottom that several `nx-sys-*` crates depend on; `nx-std-fs` is genuinely a top tier. `std` has the same shape and hides it, because there the whole thing is one crate and the edges are invisible.

### The Actual Order

What the graph really looks like, foundation first. Only the load-bearing edges are drawn; `nx-panic-handler` is omitted because every crate takes it.

```
nx-panic-handler, nx-cpu                      no dependencies
└── nx-svc                                    the SVC surface
    ├── nx-sys-thread-tls, nx-sys-sync, nx-rand
    │   └── nx-alloc                          the global allocator
    │       ├── nx-std-path                   OsStr / Path vocabulary
    │       ├── nx-sys-args, nx-sys-env       process-wide platform state
    │       ├── nx-sys-virtmem → nx-sys-mem → nx-sys-thread
    │       ├── nx-sys-fd                     descriptor table and devices
    │       └── nx-std-sync                   Mutex, RwLock, …
    │           └── nx-sf → nx-service-*      the IPC layer
    │               ├── nx-sys-net, and the device crates
    │               │   └── nx-std-fs, nx-std-env   the std-facing API
    │               │       └── nx-rt-core → nx-rt-{nro,nso,kip,module}
    │               │           └── nx-std    the umbrella
```

### Meson-only Subprojects

These ship `meson.build` files but no `Cargo.toml`, and are NOT Cargo workspace members:

- **`libnx`** / **`libnx-dkp`**: the C homebrew library, built from source or taken as a devkitPro prebuilt (`use_libnx_dkp`).
- **`sysroot-dkp`**: system root manifest mapping devkitPro and toolchain headers and libraries for Meson.
- **`libdeko3d-dkp`**: devkitPro graphics library prebuilt.
- **`tests`**: the Switch-hardware NRO suites and the runner that drives them.
- **`examples`**: example NROs.
- **`nx-hbmenu`**: the homebrew menu binary.

---

## 3. Dependency Rules

There is no per-family permission matrix, because the families are not layers ([Section 2](#2-crate-families)). What holds instead:

- **No cycles**, at any level. Cargo enforces this, and a cycle usually means two crates were split along the wrong seam.
- **`nx-std` is the sink.** Every crate may flow into it; nothing depends on it.
- **Nothing below the runtime depends on an `nx-rt-*` crate**, in a manifest or through an `extern "C"` declaration. The runtime is the last crate before the sink, so anything owned there is out of reach of everything else ([code/crates-rt](code/crates-rt.md)).
- **A new `nx-sys-*` crate takes no `nx-std-*` dependency.** It deals in bytes, and the crate above applies the `OsStr` / `Path` vocabulary. `nx-sys-fd` and `nx-sys-net` predate this and carry the edge; the rule is about not adding more.
- **A `nx-sys-*` crate implements its resource in Rust** rather than calling the C library this workspace replaces. Wrapping newlib adopts a second copy of state the crate exists to provide, and the two then disagree.
- **The `ffi` feature gates a crate's `src/ffi.rs`**, and `nx-std` enables exactly those `ffi` features matching the enabled `use_nx*` Meson options ([code/rust-ffi](code/rust-ffi.md)).

Meson-only subprojects sit outside all of this: they consume the Rust side and are not consumed by it.

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

All `Cargo.toml` dependency sections (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`) MUST be ordered alphabetically ([code/rust-crates](code/rust-crates.md)).

**Rationale**: Same as above. The Cargo/Meson mirror in each per-crate `meson.build` references these in the same order, making the relationship easy to audit.

### Meson Subproject Calls

Top-level `subproject(...)` calls in the root `meson.build` are grouped by purpose (`Libraries` first, `Executables` second), then ordered to match the build dependency direction within each group (`sysroot-dkp` before libnx; libnx before binaries). Strict alphabetical ordering is NOT required at the root because dependency direction must drive the order.

---

## See Also

- [code/meson-subproject](code/meson-subproject.md) — Generic Meson subproject conventions
- [code/meson-subproject-crate](code/meson-subproject-crate.md) — Rust-crate specialization of the Meson subproject layout
- [code/rust-crates](code/rust-crates.md) — Crate manifest conventions
- [code/rust-ffi](code/rust-ffi.md) — `ffi` Cargo feature contract for the FFI surface
- [code/meson-linker-script](code/meson-linker-script.md) — `*_override.ld` linker scripts

## Checklist

Before committing workspace-level changes, verify:

### Crate placement and direction

- [ ] A new crate's prefix names the part of `std` it is destined to become, not where it sits in the graph.
- [ ] The crate sits directly at `subprojects/<crate>/`; no family has a directory of its own.
- [ ] There are no cycles, and nothing below the runtime depends on an `nx-rt-*` crate.
- [ ] A new `nx-sys-*` crate takes no `nx-std-*` dependency and implements its resource in Rust rather than calling the C library.
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
