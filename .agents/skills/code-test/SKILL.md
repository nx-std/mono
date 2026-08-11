---
name: code-test
description: Run nx-std tests on Nintendo Switch hardware after format/check/clippy are green. Builds the nx-tests NRO, deploys via cargo-nx, and asks the user to confirm results on the console.
allowed-tools: "Bash(just build-tests:*), Bash(just deploy:*), Bash(just list-options-configured:*), Bash(just reconfigure:*), Bash(.agents/skills/code-test/read-test-results.sh:*)"
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

If not enabled:
```bash
just reconfigure -Duse_nx=enabled
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

### Step 4 — read the results

The harness records every case's verdict in a table (`g_test_results` in
`source/harness.h`) precisely so the run can be read back afterwards. Prefer reading it:

```bash
.agents/skills/code-test/read-test-results.sh <console-ip> buildDir/subprojects/tests/<binary>.elf
```

Works for any harness-based binary — `nx-tests`, `nx-tests-sync`, `nx-tests-fs`,
`nx-tests-net`. The interactive applet binaries (`nx-tests-applet-*`) have no test cases
and record nothing, so they still need a person watching the screen.

It prints one line per case and a pass/fail tally. Run it any time after the suite has
finished — it attaches over Atmosphère's GDB stub and reads memory, so there is no
breakpoint and no race with the run.

**Requires** `enable_standalone_gdbstub=u8!0x1` and `enable_htc=u8!0x0` in
`atmosphere/config/system_settings.ini` (reboot after changing). Without the stub, fall
back to asking the user.

**Fallback — ask the user.** If the stub is unavailable or the script cannot find the
module, ask the user to confirm what the console shows. **Never assume tests passed**:
the result reaches you from the console or not at all.

#### Why not read the console text

The console has no text buffer to dump. libnx draws each character straight to the
framebuffer through `renderer->drawChar`; `PrintConsole` holds a font, cursor, colours
and dimensions and nothing else. Anything a test only `printf`s is unreadable
afterwards — so a diagnostic worth capturing goes in a `volatile` global, not a print.

#### Why the script works the way it does

Three constraints shaped it, and each will bite anyone who reimplements it:

- **The module base moves every launch.** ASLR relocates the NRO, so the script reads
  `monitor get modules` each run rather than caching an address.
- **`monitor` output goes to stderr.** Capturing it needs `2>&1`; discarding stderr
  silently yields "module not loaded".
- **The recording table must be `volatile`.** Nothing in the process reads it, so
  without `volatile` the stores are dead and the whole array is optimised out of the
  binary. `volatile` also stops the compiler dropping *unused* copies, which is why the
  table is declared `extern` in `harness.h` and defined once per binary — a `static`
  definition in the header gave `nx-tests-sync` twenty-six tables to search.

This gdb build has no Python, so the address arithmetic is done in the shell.

## Test Architecture

Tests live in `subprojects/tests/`:
- `source/main.c` — test harness entry point
- `source/harness.h` — test framework macros
- `source/sync/` — synchronization primitive tests
- `source/rand/` — RNG tests
- `source/net/` — socket driver and resolver tests (own binary; brings up a network stack)

Suites that need console state — a network, savedata, a particular firmware — report
`TEST_SKIPPED` from `//* Given` rather than failing, since that state is a property of
the console rather than of the code under test.

A new test binary joins the reader by expanding `TEST_RESULTS_STORAGE` once at file scope
in its `main.c`. Without it the link fails on the undefined table, which is the intended
outcome: a binary that runs cases has to record them.

C code links against the Rust crates to verify FFI correctness; the linker scripts (`*_override.ld`) redirect `libnx` symbols to the Rust implementations.

## Anti-patterns

- Running tests before `/code-check` is green.
- Using `cargo test` — there is no host-runnable cargo test suite for the Switch target. Tests are NRO-based.
- Building `nx-tests.nro` without `use_nx=enabled` — the suite would link against libnx and not exercise the Rust replacements.
- Assuming a deploy succeeded without on-console confirmation.
- Skipping the NRO run on FFI-surface changes.
- Reporting a suite as passing without either reading the result table or asking the user.
- Putting a diagnostic in a `printf` and expecting to read it back — only recorded state survives the run.

## Pre-approved Commands

Runnable without user permission:
- `just list-options-configured`
- `just build-tests`
- `just deploy <nro-path>`
- `just reconfigure -Duse_nx=enabled`
- `.agents/skills/code-test/read-test-results.sh <ip> <elf>` — attaches read-only over the GDB stub and detaches.

## Related Skills

- `/code-format` — Format before testing.
- `/code-check` — Must be green before running this skill.
- `/code-build` — Building targets (including `just build-tests`).
- `/code-deploy` — Deploying NRO files to the Switch.
- `/code-test` — Equivalent hardware-test workflow (this skill is the more detailed variant).
