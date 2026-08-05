---
name: code-build
description: Configure and build targets in the nx-std monorepo using just tasks. Use when configuring Meson build options, changing options on an existing build, or compiling NRO files and other build artifacts.
allowed-tools: "Bash(just --list:*), Bash(just clean-all:*), Bash(just configure:*), Bash(just reconfigure:*), Bash(just meson-configure:*), Bash(just meson-reconfigure:*), Bash(just list-targets:*), Bash(just list-options:*), Bash(just list-options-configured:*), Bash(just list-dependencies:*), Bash(just build:*), Bash(just meson-compile:*), Bash(just compile:*), Bash(just build-tests:*)"
---

# Code Build Skill

Configure and build Meson/Cargo targets in the nx-std monorepo. **MANDATORY**: use `just` tasks; never invoke `meson` or `ninja` directly.

## When to Use This Skill

- First-time Meson configuration (`buildDir/` does not exist).
- Changing Meson setup-time options (`use_nx*`, `use_libnx_dkp`, …) on an existing build.
- Listing available targets / options / configured options.
- Compiling NRO bundles, ELF objects, or the nx-tests test NRO.

## Prerequisite

If `buildDir/` does not exist, configure first. Build commands fail with a confusing error if the directory is unconfigured.

## Workflow

### Step 1 — Configure (first time)

```bash
just configure                       # default options
just configure -Duse_nx=enabled      # with options
```

Runs `meson setup buildDir`. **Use after `just clean-all` or on a fresh checkout.**

### Step 2 — Change options (fresh configure)

**The default way to change any Meson option is a clean configure:**

```bash
just clean-all && just configure -Duse_nx=enabled
```

This is mandatory for `use_nx*` and `use_libnx_dkp`: `meson setup --reconfigure` does not refresh the override link args a feature contributes, so the change appears accepted but the link line is stale — the build silently keeps the old feature set.

<details>
<summary>Exception: <code>just reconfigure</code> (rarely needed)</summary>

`just reconfigure -D<option>=<value>` (runs `meson setup --reconfigure`) is safe only for options that do not affect the feature set or linking — e.g. `link_pipeline`, `nso_applet_type`, `devkitpro`. When unsure, use the clean configure above; it is always correct, just slower.

</details>

## The `use_nx_*` Feature Model

The `use_nx_*` options resolve like Cargo `[features]` (see `docs/code/meson-options-features.md`):

- `-Duse_nx=enabled` is the master switch: it turns on every feature left on `auto`. Features defaulting to `disabled` (the WIP/unstable ones) are not affected and must be named explicitly.
- **Enabling a feature pulls in what it depends on** — `-Duse_nx_fsdev=enabled` also enables `sys_fd`, `service_fs`, and `rt`; there is no need to pass the dependencies by hand.
- **Two explicit choices that contradict each other fail configuration** with an error naming both sides (e.g. `-Duse_nx_fsdev=enabled -Duse_nx_service_fs=disabled`). Fix the flags rather than working around the error.
- Against the prebuilt archive (`-Duse_libnx_dkp=enabled`) only a subset of features is available; enabling one outside it is refused at configure time.

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
- **Never invoke `meson setup` directly** — use `just configure` (or, for the narrow exception above, `just reconfigure`).
- **Never run `just configure` on an already-configured build** — run `just clean-all` first (Meson errors otherwise).
- **Never flip `use_nx*` or `use_libnx_dkp` via `just reconfigure`** — the override link args go stale; use `just clean-all && just configure`.
- **Never guess target names** — run `just list-targets` first.

## Pre-approved Commands

Runnable without user permission:
- `just --list`, `just list-targets`, `just list-options`, `just list-options-configured`, `just list-dependencies` — read-only introspection.
- `just clean-all`, `just configure [opts]` — the standard path for configuring and changing options.
- `just reconfigure [opts]` — the narrow non-feature-option exception above.
- `just build [targets]`, `just build-tests`, `just meson-compile [targets]`, `just compile [targets]` — compile only; no deploy or external side effects.

## Related Skills

- `/code-format` — Format before building.
- `/code-check` — Validate compilation/clippy before a full build.
- `/code-deploy` — Deploy NRO artifacts to Switch hardware.
- `/code-test` — Build + deploy the nx-tests NRO and confirm results on console.
