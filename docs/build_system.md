# Build System

This document describes the Meson-based build system that orchestrates dual C/Rust development for Nintendo Switch
homebrew.

## Overview

The build system is a **hybrid architecture** combining:

- **Meson** - Orchestrates cross-compilation, dependency management, and project structure
- **Cargo** - Manages Rust workspace and compilation
- **devkitPro** - Provides toolchain (GCC, binutils) and Switch-specific tools

This enables incremental replacement of C-based `libnx` functions with Rust implementations while maintaining full
compatibility with existing Switch homebrew development workflows.

## Cross-Compilation Setup

The build system uses Meson's cross-compilation files to target the Nintendo Switch (Horizon OS on ARM Cortex-A57).

### `devkitpro.txt`

Defines the devkitPro toolchain location and binaries. Located at: `devkitpro.txt`

**Purpose**: Maps all toolchain executables and Switch-specific tools.

**Key sections**:

```ini
[constants]
dkp = '/opt/devkitpro'

[binaries]
# GCC toolchain
c = dkp + '/devkitA64/bin/aarch64-none-elf-gcc'
cpp = dkp + '/devkitA64/bin/aarch64-none-elf-g++'
ar = dkp + '/devkitA64/bin/aarch64-none-elf-ar'

# Switch-specific tools
elf2nro = dkp + '/tools/bin/elf2nro'        # ELF → NRO converter
elf2nso = dkp + '/tools/bin/elf2nso'        # ELF → NSO converter
nacptool = dkp + '/tools/bin/nacptool'      # NACP metadata generator
npdmtool = dkp + '/tools/bin/npdmtool'      # NPDM metadata generator
build_pfs0 = dkp + '/tools/bin/build_pfs0'  # PFS0/NSP packager
```

**Customization**: Override the devkitPro path via meson option:

```bash
just configure -Ddevkitpro=/custom/path
```

### `cross.txt`

Defines the target machine and architecture-specific compiler flags. Located at: `cross.txt`

**Purpose**: Specifies Nintendo Switch hardware characteristics and compilation flags.

**Target machine**:

```ini
[host_machine]
system = 'horizon'          # Horizon OS
cpu_family = 'aarch64'      # ARMv8-A 64-bit
cpu = 'cortex-a57'          # Nintendo Switch CPU
endian = 'little'           # Little-endian
```

**Compiler flags**:

```ini
[built-in options]
c_args = [
         '-march=armv8-a+crc+crypto',  # ARMv8-A with CRC and crypto extensions
         '-mtune=cortex-a57',          # Optimize for Cortex-A57
         '-mtp=soft',                  # Software thread pointer (no hardware TLS)
         '-fPIE'                       # Position-independent executable
]
cpp_args = c_args + ['-fno-rtti', '-fno-exceptions']
```

**Custom tools**:

```ini
[binaries]
bundle = '@GLOBAL_SOURCE_ROOT@/scripts/bundle.sh'  # NRO/NSP bundler
```

### Build Invocation

```bash
# Configure with cross-compilation
just configure

# Compile
just build
```

## libnx Subprojects

The build system supports two modes for obtaining the `libnx` library, controlled by the `use_libnx_dkp` option.

### `libnx` - Build from Source (Default)

**Location**: `subprojects/libnx/`

Builds libnx from source, allowing integration with Rust implementations via the override mechanism.

**When to use**: Development, debugging, or when Rust function overrides are enabled.

**Setup**:

```bash
just configure
# Automatically uses source-built libnx
```

### `libnx-dkp` - Use Pre-built Libraries

**Location**: `subprojects/libnx-dkp/`

Links against pre-built libnx libraries provided by devkitPro.

**When to use**: Faster builds when Rust overrides are not needed, or for testing against official libnx releases.

**Setup**:

```bash
just configure -Duse_libnx_dkp=enabled
```

**How it works**: The `libnx-dkp` subproject wraps the pre-installed libnx:

```meson
nx_libdir = devkitpro / 'libnx/lib'
nx_incdir = devkitpro / 'libnx/include'

nx_dep = declare_dependency(
    include_directories : include_directories(nx_incdir),
    link_args : ['-L@0@'.format(nx_libdir), '-lnx'],
    dependencies : [sysroot_dep, nx_std_dep],  # Still supports Rust overrides
)
```

**Note**: Rust function overrides work with both modes. The `libnx-dkp` variant still allows selective replacement of
libnx functions with Rust implementations.

## Rust Library Selection (Setup-Time Configuration)

The build system uses Meson options to control which Rust implementations replace libnx C functions.

### Configuration Options

The build system provides `use_nx_*` Meson options to control which Rust crates replace libnx C implementations:

- **`use_nx`** - Master switch that controls all Rust overrides (default: `disabled`)
- **`use_nx_<crate>`** - Individual crate overrides (default: `auto`, follows master switch)

**List all available options**:

```bash
just list-options
```

**List configured options** (requires configured build):

```bash
just list-options-configured
```

### Option Behavior

- **`enabled`**: Force enable (always use Rust implementation)
- **`disabled`**: Force disable (always use C implementation)
- **`auto`** (default): Follow the `use_nx` master switch

### Feature Resolution (Cargo-Style)

The `use_nx_*` options resolve the way Cargo resolves `[features]`: a feature is asked for by name, and asking
for it also asks for what it depends on.

| You pass                                        | You get                                          |
|-------------------------------------------------|--------------------------------------------------|
| `-Duse_nx=enabled`                              | every feature left on `auto`                     |
| `-Duse_nx_fsdev=enabled`                        | `fsdev` plus `sys_fd`, `service_fs`, `rt`        |
| `-Duse_nx=enabled -Duse_nx_time=disabled`       | everything except `time` (and what needed it)    |
| `-Duse_nx_fsdev=enabled -Duse_nx_service_fs=disabled` | a configure error naming both sides        |

Pull-up and push-down only move features left on `auto`; two explicit choices that contradict each other fail
configuration rather than being silently reconciled. Against the prebuilt archive (`use_libnx_dkp`), features
that would alias over its own runtime and services are unavailable and refused when named.

The rules live in [`docs/code/meson-options-features.md`](code/meson-options-features.md).

### Option Propagation

Every `use_nx_*` option is declared at the workspace root and mirrored verbatim (with `yield : true`) into the
`meson.options` of each subproject that reads it, so the root's value reaches every depth. Each consuming
`meson.build` resolves the feature set for itself from those options — nothing is forwarded through
`subproject(default_options : ...)`, and no subproject reads another's resolution.

### Usage Examples

**Enable all Rust overrides**:

```bash
just configure -Duse_nx=enabled
```

**Selective overrides (only allocation and SVC)**:

```bash
just configure -Duse_nx_alloc=enabled -Duse_nx_svc=enabled
```

**Disable specific override while using master switch**:

```bash
just configure -Duse_nx=enabled -Duse_nx_time=disabled  # Everything except time
```

## Link-Time C API Replacement

Rust implementations transparently replace libnx C functions at link time using **linker override scripts**.

> **For detailed information** about FFI naming conventions, linker override file format, and meson integration patterns, see [libnx_overrides.md](libnx_overrides.md).

### High-Level Flow

1. **Rust crates** implement libnx functions with C FFI via public `ffi` modules
2. **`nx-std`** is the only staticlib; it re-exports FFI symbols from enabled crates
3. **Linker override scripts** (`*_override.ld`) redirect libnx symbols to Rust implementations
4. **Meson** collects override scripts from enabled crates based on `use_nx_*` options
5. **At link time**, the linker applies all override scripts transparently

### Quick Example

**Setup**: Build with SVC overrides enabled

```bash
just configure -Duse_nx_svc=enabled
```

**What happens**:

1. **Cargo builds** `nx-svc` as rlib, `nx-std` with `--features ffi,svc` → produces `libnx_std.a`
2. **Meson collects** `-T svc_override.ld` from nx-svc subproject
3. **Link args** propagate: `nx-std` → `libnx` → final executable
4. **At link time**: All `svcSetHeapSize()` calls execute the Rust implementation

**Verification**: Check the symbol table

```bash
nm buildDir/subprojects/tests/tests.elf | grep svcSetHeapSize
# Shows: svcSetHeapSize = __nx_svc__svc_set_heap_size
```

### Available Override Crates

Override crates correspond to the `use_nx_*` Meson options documented in the [Configuration Options](#configuration-options) table. When an option is enabled, its crate's override script redirects libnx symbols to Rust implementations.

To see which symbols a crate overrides, inspect its `*_override.ld` file in the crate's directory.

## Cargo Integration

Meson invokes Cargo to build Rust crates via `custom_target()` declarations.

### Rust Workspace

All Rust crates are part of a single Cargo workspace defined in `Cargo.toml`. The workspace includes:

- **`nx-std`** - Umbrella crate that re-exports functionality from other crates
- **`nx-svc`** - Foundation crate providing raw supervisor call (SVC) bindings to Horizon OS
- **`nx-sys-*`** - Low-level system crates (memory management, synchronization primitives, threading)
- **Higher-level crates** - Allocator (`nx-alloc`), time (`nx-time`), random (`nx-rand`), sync (`nx-std-sync`)
- **Utility crates** - CPU utilities, homebrew menu integration

**Benefits**:

- Shared `Cargo.lock` for consistent dependencies
- Incremental compilation across crates
- Unified toolchain configuration (`.cargo/config.toml`, `rust-toolchain.toml`)

### Meson Custom Target Pattern

Individual crates compile as rlib only. Only `nx-std` produces a staticlib:

**Individual crate pattern** (e.g., `nx-svc`):

```meson
cargo = find_program('cargo', required : true)

nx_svc_tgt = custom_target(
    'nx-svc',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_svc.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,  # Delegate incremental compilation to Cargo
)
```

**nx-std pattern** (staticlib with FFI):

```meson
nx_std_tgt = custom_target(
    'nx-std',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--profile', get_option('buildtype') == 'release' ? 'release' : 'dev',
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
        '--no-default-features',
        '--features', ','.join(['ffi'] + deps_cargo_features),
    ],
    output : ['libnx_std.a', 'libnx_std.rlib'],
    ...
)
```

**Key options**:

- `--package` - Build specific crate (Meson project name matches crate name)
- `--profile` - Map Meson `buildtype` to Cargo profile (`dev` or `release`)
- `--target-dir` - Shared Cargo target directory (`buildDir/cargo-target/`)
- `--artifact-dir` - Place output (`.rlib` or `.a`) in Meson's output directory
- `--features ffi,<crates>` - Enable FFI and crate features (nx-std only)
- `build_always_stale : true` - Always invoke Cargo (it handles incremental builds)

## Build Artifacts

The build produces two types of Switch homebrew packages.

### NRO (Homebrew Applications)

**Format**: Nintendo Relocatable Object
**Use case**: Homebrew applications launched via hbmenu or similar loaders

**Build process**:

1. Compile ELF executable
2. Generate NACP metadata (name, author, version)
3. Convert to NRO with optional icon and RomFS

**Output location**: `buildDir/subprojects/<name>/<name>.nro`

**Example**:

```bash
# Build generates:
buildDir/subprojects/tests/nx-tests.nro
```

### NSP (Installable Packages)

**Format**: Nintendo Submission Package
**Use case**: System modules, applications installed to home menu

**Build process**:

1. Compile ELF executable
2. Generate NPDM metadata from JSON config
3. Convert ELF to NSO (Nintendo Shared Object)
4. Create ExeFS structure (main NSO + NPDM)
5. Package as PFS0/NSP

**Output location**: `buildDir/subprojects/<name>/<name>.nsp`

**Trigger**: Presence of `config.json` (NPDM configuration) determines NSP build

### Bundle Script

Both formats are generated via `scripts/bundle.sh`, invoked by Meson custom targets:

```meson
custom_target('@0@.nro'.format(name),
    input : elf,
    output : '@0@.nro'.format(name),
    command : [
        bundle_sh,
        '--out-dir', '@OUTDIR@',
        '--input', '@INPUT@',
        '--output', '@OUTPUT0@',
        '--tmp-dir', '@PRIVATE_DIR@',
        '--icon', icon,
        '--name', name,
        '--author', author,
        '--version', version,
    ],
    build_by_default : true,
)
```

## Link Pipeline

The final executable link can be driven by one of two pipelines, selected by the
`link_pipeline` combo option:

- **`gcc`** (default) — the link is driven by the devkitA64 GCC toolchain, using
  libnx's `switch.ld` linker script and `switch_crt0.s` startup object.
- **`rustc`** — the opt-in rustc-driven link pipeline: a custom
  `aarch64-nintendo-horizon.json` target embeds the section layout, and each
  runtime entry crate (`nx-rt-nro`, `nx-rt-nso`, `nx-rt-kip`) supplies its own
  per-kind `.crt0` via the `rt-link` Cargo feature.

```bash
just configure -Dlink_pipeline=rustc
```

The rustc pipeline has no implicit `-T` step, so `aarch64-nintendo-horizon.json`
must embed `switch.ld`'s section layout verbatim in its `link-script` field.
`just check-target-json` verifies the two stay in sync.

See [rustc-link-pipeline.md](rustc-link-pipeline.md) for the full pipeline and
[crt0-and-mod0.md](crt0-and-mod0.md) for the `.crt0` / MOD0 startup mechanism.
