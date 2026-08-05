---
name: code-test
description: Run nx-std tests on Nintendo Switch hardware after format/check/clippy are green. Builds the nx-tests NRO, deploys via cargo-nx, and asks the user to confirm results on the console.
allowed-tools: "Bash(just build-tests:*), Bash(just deploy:*), Bash(just list-options-configured:*), Bash(just clean-all:*), Bash(just configure:*)"
---

# Code Testing Skill

Runs the nx-std test suite. Tests are C-code linked against the Rust crates that exercise FFI correctness; they execute on **real Nintendo Switch hardware** (or an emulator with nxlink), not via `cargo test`.

## Prerequisite

**`/code-format` and `/code-check` must be green first.** If dirty, return there — compile/clippy issues are faster to surface than a full NRO build.

## Scope Selection

This project has **one** test target: the `nx-tests` NRO. There are no per-crate Rust unit-test profiles — coverage is exercised via the integrated NRO suite under `subprojects/tests/`.

| Blast radius                                                          | Action                                          |
|-----------------------------------------------------------------------|-------------------------------------------------|
| None (docs/comments only)                                             | Skip; state why                                 |
| Pure-Rust changes with no FFI surface impact                          | `/code-check` is sufficient — no NRO test run   |
| FFI surface change, foundation crate, or behavior change              | Build + deploy nx-tests NRO; confirm on console |
| Linker scripts, `nx-std` crate, or `use_nx*` Meson option behaviour   | Build + deploy nx-tests NRO; confirm on console |

**Signals that require running the NRO test suite:**
- Changed any `__nx_*` FFI function signature, return value, or behavior.
- Edited `nx-std/src/ffi.rs` re-exports.
- Edited foundation crates (`nx-svc`, `nx-sys-mem`, `nx-sys-sync`).
- Changed linker scripts (`*_override.ld`) or Meson `use_nx*` wiring.
- Changed test code under `subprojects/tests/`.

If none fire, `/code-check` clean is sufficient — skip the NRO run and state why.

## Workflow

### Step 1 — verify configuration

Tests must be built with `use_nx=enabled` (unless the user explicitly requests otherwise):

```bash
just list-options-configured | grep use_nx
```

If not enabled (a fresh configure, not reconfigure — flipping a `use_nx*` feature leaves stale override link args behind on reconfigure):
```bash
just clean-all && just configure -Duse_nx=enabled
```

### Step 2 — build the test NRO

```bash
just build-tests
```

Compiles `buildDir/subprojects/tests/nx-tests.nro`. (Equivalent to `just build nx-tests.nro`; see `/code-build`.)

### Step 3 — deploy to the Switch

```bash
just deploy buildDir/subprojects/tests/nx-tests.nro
```

Network transfer via `cargo nx link`. See `/code-deploy` for prerequisites (Atmosphère, nxlink, network).

**On deploy failure:** retry up to 3 times with a 10-second delay (Switch may not yet be on the network or nxlink not running).

### Step 4 — verify results

🚨 **MANDATORY**: Ask the user to confirm tests passed on the console.

Test output is only visible on the Switch screen. **Do NOT assume tests passed.**

## Test Architecture

Tests live in `subprojects/tests/`:
- `source/main.c` — test harness entry point
- `source/harness.h` — test framework macros
- `source/sync/` — synchronization primitive tests
- `source/rand/` — RNG tests

C code links against the Rust crates to verify FFI correctness; the linker scripts (`*_override.ld`) redirect `libnx` symbols to the Rust implementations.

## Anti-patterns

- Running tests before `/code-check` is green.
- Using `cargo test` — there is no host-runnable cargo test suite for the Switch target. Tests are NRO-based.
- Building `nx-tests.nro` without `use_nx=enabled` — the suite would link against libnx and not exercise the Rust replacements.
- Assuming a deploy succeeded without on-console confirmation.
- Skipping the NRO run on FFI-surface changes.

## Pre-approved Commands

Runnable without user permission:
- `just list-options-configured`
- `just build-tests`
- `just deploy <nro-path>`
- `just clean-all`, `just configure -Duse_nx=enabled`

## Related Skills

- `/code-format` — Format before testing.
- `/code-check` — Must be green before running this skill.
- `/code-build` — Building targets (including `just build-tests`).
- `/code-deploy` — Deploying NRO files to the Switch.
- `/code-test` — Equivalent hardware-test workflow (this skill is the more detailed variant).
