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
1. Rust crates implement Nintendo Switch OS functionality
2. When built with the `ffi` feature, crates export C-compatible functions
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
| `__libnx_exception_entry` | `__nx_rt_env__exception_entry` |

## Rust Crate Structure

### lib.rs

Feature-gate the FFI module in `src/lib.rs`:

```rust
#[cfg(feature = "ffi")]
mod ffi;
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

Each crate's `meson.build` must:

1. Build Rust crate with `ffi` feature
2. Declare linker override file variable
3. Export dependency

**Example**: `subprojects/nx-svc/meson.build`

```meson
# Build static library with FFI enabled
nx_svc_tgt = custom_target(
    'nx-svc',
    command : [
        cargo, 'build',
        '--package', meson.project_name(),
        '--features', 'ffi',  # Enable FFI exports
        '--target-dir', meson.global_build_root() / 'cargo-target',
        '--artifact-dir', '@OUTDIR@',
    ],
    output : ['libnx_svc.a', 'libnx_svc.rlib'],
    build_by_default : true,
    build_always_stale : true,
)

# Declare linker override script variable
nx_svc_ld_override = meson.current_source_dir() / 'svc_override.ld'

# Export dependency
nx_svc_dep = declare_dependency(
    link_with : nx_svc_tgt[0],
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

The `nx-std` umbrella crate collects override link args from all enabled dependencies.

**Example**: `subprojects/nx-std/meson.build`

```meson
deps_override_link_args = []

# Conditionally add each crate's override based on meson options
if get_option('use_nx_svc').enabled()
    nx_svc_proj = subproject('nx-svc')
    deps += nx_svc_proj.get_variable('nx_svc_dep')
    deps_override_link_args += ['-T', nx_svc_proj.get_variable('nx_svc_ld_override')]
endif

# ... repeat for other crates ...

# Export collected link args
nx_std_dep_override_link_args = deps_override_link_args
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
