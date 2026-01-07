# libnx Override System

This document describes how Rust crates in this project can override `libnx` functions at link time using C FFI and GNU linker scripts.

## Table of Contents

- [Overview](#overview)
- [FFI Naming Convention](#ffi-naming-convention)
- [Rust Crate Structure](#rust-crate-structure)
- [Linker Override Files](#linker-override-files)
- [Meson Build Integration](#meson-build-integration)
- [nx-std Integration](#nx-std-integration)

## Overview

The libnx override system allows Rust crates to replace libnx C functions transparently at link time. This enables gradual migration from C to Rust while maintaining compatibility with existing homebrew code.

**How it works**:
1. Rust crates implement Nintendo Switch OS functionality with public `ffi` modules
2. `nx-std` is the only staticlib; it re-exports FFI symbols from enabled crates
3. Linker scripts redirect libnx symbols to Rust implementations
4. At link time, code calling libnx functions transparently uses Rust implementations

## FFI Naming Convention

All Rust FFI exports follow this pattern:

```
__{{ crate-name | snake_case }}__{{ fn-name | snake_case }}
```

### Rules

- **Crate name**: Rust crate name with hyphens converted to underscores (e.g., `nx-svc` → `nx_svc`)
- **Function name**: libnx function name converted from CamelCase to snake_case
- **Separator**: Double underscore (`__`) separates crate name from function name

### Examples

| libnx Function | Rust FFI Function |
|---|---|
| `svcSetHeapSize` | `__nx_svc__svc_set_heap_size` |
| `svcMapMemory` | `__nx_svc__svc_map_memory` |
| `mutexLock` | `__nx_std_sync__mutex_lock` |

### Special Case: `__libnx_*` Functions

For functions starting with `__libnx_`, strip the prefix and keep the rest as-is:

| libnx Function | Rust FFI Function |
|---|---|
| `__libnx_initheap` | `__nx_alloc__initheap` |
| `__libnx_exception_entry` | `__nx_rt__exception_entry` |

## Rust Crate Structure

### lib.rs

Expose the FFI module publicly in `src/lib.rs` so `nx-std` can re-export it:

```rust
pub mod ffi;
```

### ffi.rs

Implement C-compatible functions in `src/ffi.rs`:

```rust
use core::ffi::c_void;

/// Override for libnx svcSetHeapSize
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_svc__svc_set_heap_size(
    out_addr: *mut *mut c_void,
    size: usize
) -> u32 {
    // Implementation
}
```

**Requirements**:
- `#[unsafe(no_mangle)]`: Prevents name mangling so the linker can find the symbol
- `extern "C"`: Uses C calling convention for ABI compatibility
- Match libnx function signature exactly
- Module must be `pub` for `nx-std` re-export visibility

## Linker Override Files

### File Format

Linker override files use GNU ld script syntax. File naming: `<short_name>_override.ld`

**Example**: `subprojects/nx-svc/svc_override.ld`

```ld
/* Ensure Rust symbols are pulled in */
EXTERN(__nx_svc__svc_set_heap_size);
EXTERN(__nx_svc__svc_map_memory);

/* Redirect libnx symbols to Rust implementations */
svcSetHeapSize = __nx_svc__svc_set_heap_size;
svcMapMemory = __nx_svc__svc_map_memory;
```

### Structure

1. **`EXTERN()` declarations**: Tell the linker to pull in Rust symbols from the static library
2. **Symbol assignments**: Create aliases redirecting all references from libnx symbols to Rust symbols

**Result**: Code calling `svcSetHeapSize()` transparently calls `__nx_svc__svc_set_heap_size()` from Rust.

## Meson Build Integration

### meson.build

Individual crates build as rlib only. Each crate's `meson.build` must:

1. Build Rust crate as rlib (no `--features ffi`)
2. Declare linker override file variable
3. Export dependency

**Example**: `subprojects/nx-svc/meson.build`

```meson
# Build rlib (no staticlib, no FFI feature)
nx_svc_tgt = custom_target(
    'nx-svc',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_svc.rlib'],
    build_by_default : true,
    build_always_stale : true,
)

# Declare linker override script variable
nx_svc_ld_override = meson.current_source_dir() / 'svc_override.ld'

# Export dependency
nx_svc_dep = declare_dependency(
    sources : nx_svc_tgt,
    dependencies : deps,
)
```

### meson.options

Add a feature option to control whether the override is enabled:

```meson
option(
    'use_nx_svc',
    type : 'feature', value : 'auto',
    description : 'Override libnx SVC functions with nx-svc',
    yield : true
)
```

**Values**:
- `enabled`: Always use the override
- `disabled`: Never use the override
- `auto`: Use if `use_nx` is enabled (default)

## nx-std Integration

The `nx-std` crate is the single staticlib. It collects Cargo features and override link args from all enabled dependencies.

**Key files**:
- `nx-std/src/ffi.rs` - Re-exports FFI modules from dependent crates
- `nx-std/meson.build` - Builds staticlib with `--features ffi,<enabled-crates>`

**Example**: `subprojects/nx-std/meson.build`

```meson
deps_override_link_args = []
deps_cargo_features = []

# Conditionally add each crate's override based on meson options
if get_option('use_nx_svc').enabled()
    nx_svc_proj = subproject('nx-svc')
    deps += nx_svc_proj.get_variable('nx_svc_dep')
    deps_override_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]
    deps_cargo_features += ['svc']
endif

# ... repeat for other crates ...

# Build staticlib with FFI and enabled crate features
nx_std_tgt = custom_target(
    'nx-std',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--no-default-features',
        '--features', ','.join(['ffi'] + deps_cargo_features),
        ...
    ],
    output : ['libnx_std.a', 'libnx_std.rlib'],
    ...
)

# Export collected link args
nx_std_dep_override_link_args = deps_override_link_args
```

**FFI re-export pattern** (`nx-std/src/ffi.rs`):

```rust
#[cfg(feature = "svc")]
pub use nx_svc::ffi as svc;

#[cfg(feature = "alloc")]
pub use nx_alloc::ffi as alloc;
// ... other crates
```

### Usage in Applications

Applications link against `nx-std` and apply the override link args:

```meson
executable(
    'my_app',
    sources,
    dependencies : nx_std_dep,
    link_args : nx_std_dep_override_link_args,
)
```

The linker applies all override scripts, redirecting libnx symbols to Rust implementations.

## Master Override Option

The `use_nx` option enables all overrides at once:

```bash
meson setup buildDir -Duse_nx=enabled
```

Individual crates can still be controlled separately:

```bash
meson setup buildDir -Duse_nx_svc=disabled -Duse_nx_alloc=enabled
```
