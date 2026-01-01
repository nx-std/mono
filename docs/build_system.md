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

### Auto-Enable/Disable Pattern

Individual options use conditional logic based on the master `use_nx` switch:

```meson
use_nx = get_option('use_nx')

use_nx_alloc = get_option('use_nx_alloc')
    .enable_auto_if(use_nx.enabled())
    .disable_auto_if(use_nx.disabled())
```

**Examples**:

| `use_nx` | `use_nx_alloc` | Result                             |
|----------|----------------|------------------------------------|
| enabled  | auto           | **enabled** (follows master)       |
| disabled | auto           | **disabled** (follows master)      |
| enabled  | disabled       | **disabled** (override forces off) |
| disabled | enabled        | **enabled** (override forces on)   |

### Option Propagation

Options are declared in `meson.options` with `yield: true`, allowing them to propagate to subprojects:

```meson
option('use_nx_alloc',
    type : 'feature', value : 'auto',
    description : 'Override libnx allocation with nx-alloc',
    yield : true  # Propagate to subprojects
)
```

Subprojects receive these options and can pass them further:

```meson
# In libnx/meson.build
nx_std_proj = subproject('nx-std',
    default_options : {
        'use_nx_alloc' : '@0@'.format(use_nx_alloc),
        'use_nx_svc' : '@0@'.format(use_nx_svc),
    }
)
```

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

1. **Rust crates** implement libnx functions with C FFI (when built with `--features ffi`)
2. **Linker override scripts** (`*_override.ld`) redirect libnx symbols to Rust implementations
3. **Meson** collects override scripts from enabled crates based on `use_nx_*` options
4. **At link time**, the linker applies all override scripts transparently

### Quick Example

**Setup**: Build with SVC overrides enabled

```bash
just configure -Duse_nx_svc=enabled
```

**What happens**:

1. **Cargo builds** `nx-svc` with `--features ffi` → produces `libnx_svc.a`
2. **Meson collects** `-T svc_override.ld` from nx-svc subproject
3. **Link args** propagate: `nx-svc` → `nx-std` → `libnx` → final executable
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

Each Rust crate uses a standard `custom_target()` invocation:

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
        '--features', 'ffi',
    ],
    output : ['libnx_svc.a', 'libnx_svc.rlib'],
    console : true,
    build_by_default : true,
    build_always_stale : true,  # Delegate incremental compilation to Cargo
)
```

**Key options**:

- `--package` - Build specific crate (Meson project name matches crate name)
- `--profile` - Map Meson `buildtype` to Cargo profile (`dev` or `release`)
- `--target-dir` - Shared Cargo target directory (`buildDir/cargo-target/`)
- `--artifact-dir` - Place output (`.a`, `.rlib`) in Meson's output directory
- `--features ffi` - Enable FFI feature for C interop (see [libnx_overrides.md](libnx_overrides.md#rust-crate-structure))
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
