# AGENTS.md

This file provides guidance to Coding Agents when working with code in this repository.

## Project Overview

This is a Meson-based monorepo implementing a Rust replacement for `libnx` (the C homebrew library for Nintendo Switch).

**Vision**: Provide a Rust `std` implementation for the Nintendo Switch's Horizon OS.

**Current Strategy**: Incremental replacement. Rust crates expose C-FFI bindings that can replace `libnx` functions at link time, allowing gradual migration from C to Rust while maintaining compatibility with existing homebrew code.

**How it works**:
1. Rust crates implement Switch OS functionality (memory, threads, sync primitives, etc.)
2. Each crate has a public `ffi` module with C-compatible functions (`__nx_*` prefix)
3. `nx-std` is the only staticlib; it re-exports FFI symbols from enabled crates via `src/ffi.rs`
4. Linker scripts (`*_override.ld`) redirect `libnx` symbols to Rust implementations
5. At link time, code calling `libnx` functions transparently uses the Rust implementations instead

**Configuration**: Meson setup-time options (`use_nx_alloc`, `use_nx_svc`, etc.) control which crates are enabled, selecting corresponding Cargo features for `nx-std`.

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

The `nx-std` crate is the single staticlib that exports all FFI symbols. Individual crates compile as rlib and expose their FFI functions via public `ffi` modules. When `nx-std` builds, it re-exports these modules based on enabled Cargo features, ensuring symbols compile once without duplication.

Meson options control which crates are included:

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

## Development Workflow

Follow this workflow when implementing features or fixing bugs.

### 1. Research Phase

- Understand the codebase and existing patterns
- Identify related modules and dependencies
- Review test files and usage examples
- Consult `docs/` for implementation guidance

### 2. Planning Phase

- Create detailed implementation plan
- Identify validation checkpoints
- Consider edge cases and error handling
- Ask user questions if requirements are unclear

### 3. Implementation Phase

🚨 **CRITICAL: Before running ANY command, consult the relevant Skill file in `.claude/skills/`.**

**Development checklist:**

```
- [ ] Write code following project conventions
- [ ] Format code (just fmt-rs for Rust, just fmt-meson for Meson)
- [ ] Check compilation (just check-rs)
- [ ] Run clippy (just clippy)
- [ ] Fix ALL warnings
- [ ] Build target (just meson-compile <target>)
- [ ] Run tests if applicable
- [ ] All checks pass ✅
```

**Workflow for EVERY code change:**

1. **Write code** following patterns from `docs/`

2. **Format immediately** (MANDATORY after EVERY edit):
   - Rust files: `just fmt-rs`
   - Meson files: `just fmt-meson`
   - See `.claude/skills/format/SKILL.md`

3. **Check compilation**:
   - Rust: `just check-rs` or `just check-crate <crate>`
   - Meson: `just meson-configure` then `just meson-compile`
   - See `.claude/skills/build/SKILL.md`

4. **Lint with clippy**:
   - Command: `just clippy` or `just clippy-crate <crate>`
   - Fix ALL warnings before proceeding

5. **Build and test**:
   - Build targets: `just meson-compile <target>`
   - Build tests: `just build-tests`

6. **Iterate**: If any validation fails → fix → return to step 2

### 4. Completion Phase

- Ensure all automated checks pass (format, check, clippy, build)
- Review changes against project conventions
- Document any warnings you couldn't fix and why

### 5. Hardware Validation (Optional)

To validate changes on actual hardware, deploy the test suite to a Nintendo Switch:

1. Build tests: `just build-tests`
2. Deploy to Switch: `just deploy buildDir/subprojects/tests/nx-tests.nro`
3. Ask user to confirm tests PASSED on the console
4. See `.claude/skills/deploy/SKILL.md` for details

## Testing

Tests are compiled as Switch homebrew (NRO) that run on-device or emulator. Located in `subprojects/tests/`:

```bash
just build-tests  # Build test NRO
```

Test files are C code that link against Rust crates to verify FFI correctness.