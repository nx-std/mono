---
name: build
description: Build specific targets in the nx-std monorepo using just tasks. Use when compiling NRO files, libraries, or other build artifacts.
allowed-tools: Bash(just --list:*), Bash(just list-targets:*), Bash(just list-options:*), Bash(just list-options-configured:*), Bash(just list-dependencies:*), Bash(just meson-compile:*), Bash(just meson-configure:*), Bash(just build-tests:*)
---

# Build Target Skill

**MANDATORY**: Use `just` tasks for all build operations in this project.

## Important: This is a Meson Project

This project uses the Meson build system with Cargo integration. **Configuration must be done first** before building any targets.

### Initial Configuration

If `buildDir/` doesn't exist or you need to reconfigure:

```bash
just configure
```

This runs `meson setup buildDir` with default options from `meson.options`.

### Listing Available Options

**List all project options** (from `meson.options`):
```bash
just list-options
```

**List currently configured options** (requires configured build):
```bash
just list-options-configured
```

### Configuring with Options

**Configure with specific options**:
```bash
just configure -Duse_nx=enabled
```

**Configure with multiple options**:
```bash
just configure -Duse_nx=enabled -Duse_libnx_dkp=disabled
```

**Tip**: Run `just list-options` first to see available options and their defaults.

## Listing Available Targets

After configuration, list available targets to see what can be compiled:

```bash
just list-targets
```

This shows all build targets with their names, types, and locations.

## Building a Target

**Primary command**: `just build <target-name>`

Aliases: `just meson-compile <target-name>` or `just compile <target-name>`

### Examples

**Build hbmenu**:
```bash
just build hbmenu.nro
```

**Build tests**:
```bash
just build nx-tests.nro
```

Or use the dedicated test task:
```bash
just build-tests
```

**Build multiple targets**:
```bash
just build hbmenu.nro nx-tests.nro
```

**Build all targets** (no arguments):
```bash
just build
```

## Critical Rules

1. **Configure first** - Run `just configure` before building if `buildDir/` doesn't exist
2. **NEVER use `ninja` directly** - always use `just build`
3. **NEVER use `meson compile` directly** - always use `just build`
4. **NEVER use `meson setup` directly** - always use `just configure`
5. **List targets first** if unsure what can be built
6. **Use exact target names** from `just list-targets` output

## Build Output

Build artifacts are located in:
- `buildDir/` - Meson output (NRO/NSP bundles, C objects, ELF files)
- `buildDir/cargo-target/` - Rust compilation artifacts

## Complete Workflow

1. **Initial setup** (first time or clean build):
   ```bash
   just list-options                    # See available options
   just configure -Duse_nx=enabled      # Configure with desired options
   ```

2. **List targets** (if unsure):
   ```bash
   just list-targets
   ```

3. **Build target**:
   ```bash
   just build <target-name>
   ```

4. **Verify build** (check for symbol overrides):
   ```bash
   /opt/devkitpro/devkitA64/bin/aarch64-none-elf-nm buildDir/path/to/file.elf | rg '__nx_'
   ```

5. **Deploy** (if building NRO):
   ```bash
   just deploy buildDir/path/to/file.nro
   ```

## Reconfiguring

To change build options after initial configuration:

```bash
just configure -Duse_nx=disabled
```

Or clean and reconfigure from scratch:

```bash
just clean-all
just configure -Duse_nx=enabled
```
