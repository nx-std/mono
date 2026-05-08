---
name: code-build
description: Configure, reconfigure, and build targets in the nx-std monorepo using just tasks. Use when configuring Meson build options, reconfiguring an existing build, or compiling NRO files and other build artifacts.
allowed-tools: "Bash(just --list:*), Bash(just configure:*), Bash(just reconfigure:*), Bash(just meson-configure:*), Bash(just meson-reconfigure:*), Bash(just list-targets:*), Bash(just list-options:*), Bash(just list-options-configured:*), Bash(just list-dependencies:*), Bash(just build:*), Bash(just meson-compile:*), Bash(just compile:*), Bash(just build-tests:*)"
---

# Code Build Skill

Configure and build Meson/Cargo targets in the nx-std monorepo. **MANDATORY**: use `just` tasks; never invoke `meson` or `ninja` directly.

## When to Use This Skill

- First-time Meson configuration (`buildDir/` does not exist).
- Changing Meson setup-time options (`use_nx*`, `use_libnx_dkp`, …) on an existing build.
- Listing available targets / options / configured options.
- Compiling NRO bundles, ELF objects, or the nx-tests test NRO.

## Prerequisite

If `buildDir/` does not exist, configure first; otherwise reconfigure to change options. Build commands fail with a confusing error if the directory is unconfigured.

## Workflow

### Step 1 — Configure (first time only)

```bash
just configure                       # default options
just configure -Duse_nx=enabled      # with options
```

Runs `meson setup buildDir`. **Use after `just clean-all` or on a fresh checkout.**

### Step 2 — Reconfigure (existing build, options changed)

```bash
just reconfigure -Duse_nx=enabled -Duse_nx_sf=disabled
```

Runs `meson setup --reconfigure`. **Use whenever `buildDir/` already exists and you need to flip a Meson option.**

### Step 3 — Discover targets and options

```bash
just list-options              # all options from meson.options
just list-options-configured   # currently configured option values
just list-targets              # all available build targets
just list-dependencies         # introspect dependency graph
```

Run `just list-options` before configuring an unfamiliar build to see defaults.

### Step 4 — Build a target

```bash
just build <target-name>             # primary form
just build hbmenu.nro nx-tests.nro   # multiple targets
just build                           # all targets (no args)
just build-tests                     # convenience for nx-tests.nro
```

Aliases: `just meson-compile`, `just compile`. Use **exact** target names from `just list-targets`.

## Build Output

- `buildDir/` — Meson output (NRO/NSP bundles, C objects, ELF files).
- `buildDir/cargo-target/` — Rust compilation artifacts.

## FFI Symbol Verification

After a build, confirm Rust replacements are linked into the C side:

```bash
/opt/devkitpro/devkitA64/bin/aarch64-none-elf-nm buildDir/path/to/file.elf | rg '__nx_'
```

## Anti-patterns

- **Never invoke `ninja` directly** — use `just build`.
- **Never invoke `meson compile` directly** — use `just build`.
- **Never invoke `meson setup` directly** — use `just configure` or `just reconfigure`.
- **Never run `just configure` on an already-configured build** — use `reconfigure` (Meson errors otherwise).
- **Never guess target names** — run `just list-targets` first.

## Pre-approved Commands

Runnable without user permission:
- `just --list`, `just list-targets`, `just list-options`, `just list-options-configured`, `just list-dependencies` — read-only introspection.
- `just configure [opts]`, `just reconfigure [opts]` — idempotent for already-correct configs.
- `just build [targets]`, `just build-tests`, `just meson-compile [targets]`, `just compile [targets]` — compile only; no deploy or external side effects.

## Related Skills

- `/code-format` — Format before building.
- `/code-check` — Validate compilation/clippy before a full build.
- `/code-deploy` — Deploy NRO artifacts to Switch hardware.
- `/code-test` — Build + deploy the nx-tests NRO and confirm results on console.
