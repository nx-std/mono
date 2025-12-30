# AGENTS.md

This file provides guidance to Coding Agents when working with code in this repository.

## Project Overview

This is a Meson-based monorepo implementing a Rust replacement for `libnx` (the C homebrew library for Nintendo Switch).

**Vision**: Provide a Rust `std` implementation for the Nintendo Switch's Horizon OS.

**Current Strategy**: Incremental replacement. Rust crates expose C-FFI bindings that can replace `libnx` functions at link time, allowing gradual migration from C to Rust while maintaining compatibility with existing homebrew code.

**How it works**:
1. Rust crates implement Switch OS functionality (memory, threads, sync primitives, etc.)
2. When built with the `ffi` feature, crates export C-compatible functions (`__nx_*` prefix)
3. Linker scripts (`*_override.ld`) redirect `libnx` symbols to Rust implementations
4. At link time, code calling `libnx` functions transparently uses the Rust implementations instead

**Configuration**: Meson setup-time options (`use_nx_alloc`, `use_nx_svc`, etc.) control which Rust implementations are linked, enabling selective or full replacement.

## Build System

The project uses a **hybrid build system**:
- **Meson** - Orchestrates overall project builds, linking Rust and C code
- **Cargo** - Manages Rust crates (via `just` tasks)

For detailed build system documentation, see [`docs/build_system.md`](docs/build_system.md).

### Prerequisites

- devkitPro toolchain at `/opt/devkitpro` (configurable via `meson.options`)
- Rust nightly toolchain (specified in `rust-toolchain.toml`)
- Meson >= 1.4.0
- `just` command runner

### Build Directory

Build artifacts go to `buildDir/`:
- `buildDir/` - Meson output (NRO/NSP bundles, C objects)
- `buildDir/cargo-target/` - Rust target directory

### Cross-Compilation

The project targets `aarch64-nintendo-switch-freestanding`. Cargo configuration in `.cargo/config.toml` enables:
- `build-std` for `core`, `compiler_builtins`, `alloc`
- `panic = "abort"` for both dev and release profiles

## Architecture

### Crate Hierarchy

```
nx-std (umbrella crate)
├── nx-alloc     - Global allocator using SVC memory management
├── nx-rand      - Random number generation
├── nx-std-sync  - High-level sync primitives (Mutex, RwLock, etc.)
├── nx-time      - Time utilities
└── sys/
    ├── nx-svc        - Supervisor calls (SVC) interface to Horizon OS
    ├── nx-cpu        - CPU utilities
    ├── nx-sys-mem    - Low-level memory management
    ├── nx-sys-sync   - Low-level synchronization primitives
    └── nx-sys-thread - Thread management
```

### Dependency Flow

`nx-svc` is the foundation - provides raw SVC bindings. Higher-level crates build on it:
- `nx-alloc` depends on `nx-svc`, `nx-sys-sync`
- `nx-sys-mem` depends on `nx-alloc`, `nx-svc`, `nx-rand`, `nx-std-sync`
- `nx-sys-thread` depends on most other crates

### FFI Integration

Each Rust crate can expose C-compatible FFI via the `ffi` feature flag. When enabled, Rust implementations replace corresponding `libnx` functions at link time. This is controlled via Meson options:

```
use_nx         - Master switch for all replacements
use_nx_alloc   - Replace libnx allocation functions
use_nx_svc     - Replace libnx SVC functions
...
```

### libnx Integration

Two modes exist:
1. Build from source (`subprojects/libnx/`) - default
2. Use devkitPro's prebuilt (`subprojects/libnx-dkp/`) - via `use_libnx_dkp` option

The custom libnx build links against Rust crates when `use_nx*` options are enabled.

## Code Style

- Uses unstable `rustfmt` features (nightly required)
- Imports grouped: std, external crates, local
- Import granularity at crate level

## Testing

Tests are compiled as Switch homebrew (NRO) that run on-device or emulator. Located in `subprojects/tests/`:

```bash
just build-tests  # Build test NRO
```

Test files are C code that link against Rust crates to verify FFI correctness.