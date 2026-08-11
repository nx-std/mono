---
name: code-test
description: Run nx-std tests on Nintendo Switch hardware after format/check/clippy are green. Builds the test NROs, launches the runner once, pushes suites to it back to back without a person at the console, and reads their TAP results off the wire.
allowed-tools: "Bash(just build-tests:*), Bash(just deploy:*), Bash(just list-options-configured:*), Bash(just reconfigure:*), Bash(.agents/skills/code-test/read-test-results.sh:*)"
---

# Code Testing Skill

Runs the nx-std test suite. Tests are C-code linked against the Rust crates that exercise FFI correctness; they execute on **real Nintendo Switch hardware** (or an emulator with nxlink), not via `cargo test`.

## Prerequisite

**`/code-format` and `/code-check` must be green first.** If dirty, return there — compile/clippy issues are faster to surface than a full NRO build.

## Scope Selection

There are no per-crate Rust unit-test profiles — coverage is exercised via the NRO binaries under
`subprojects/tests/`, one per area (`nx-tests-rand`, `nx-tests-rt`, `nx-tests-thread`,
`nx-tests-sync`, `nx-tests-fs`, `nx-tests-net`, and the interactive `nx-tests-applet-*`). Pick the
binaries whose area the change touches.

`nx-tests` itself is **not** a suite: it is the runner that receives a suite over the netloader
protocol and launches it (see Step 3).

| Blast radius                                                          | Action                                            |
|-----------------------------------------------------------------------|---------------------------------------------------|
| None (docs/comments only)                                             | Skip; state why                                   |
| Pure-Rust changes with no FFI surface impact                          | `/code-check` is sufficient — no NRO test run     |
| FFI surface change, foundation crate, or behavior change              | Build + deploy the affected suites; confirm       |
| Linker scripts, `nx-std` crate, or `use_nx*` Meson option behaviour   | Build + deploy the affected suites; confirm       |

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

### Step 2 — build the test NROs

```bash
just build-tests
```

Compiles the runner and every suite into `buildDir/subprojects/tests/`. To build one binary on its
own, `just build nx-tests-sync.nro`; see `/code-build`.

### Step 3 — start the runner

Ask the user to arm hbmenu's netloader, then push the runner to it **once**:

```bash
just deploy buildDir/subprojects/tests/nx-tests.nro --retries 40
```

The runner comes up showing the console's address and waits. From here it is the thing listening,
and it survives the whole run: each suite it launches hands control back when it finishes, so the
runner returns to the address screen and listens again on its own.

### Step 4 — push the suites

One command per suite, back to back, with nobody at the console:

```bash
just deploy buildDir/subprojects/tests/nx-tests-rand.nro --retries 60
just deploy buildDir/subprojects/tests/nx-tests-sync.nro --retries 60
```

A suite launched by the runner runs unattended: it prints `Running unattended`, runs its cases,
reports its tally back, and leaves — it does not wait for `+`. `--retries` is what absorbs the gap
while the previous suite is still running, so a push aimed at a busy console keeps knocking rather
than failing.

**A push that is refused means nothing is listening**, which means the run broke: either the runner
never came back (a suite crashed or hung) or it was exited. Say so rather than retrying blindly; a
suite that took the console down is a test result.

Pushing to **hbmenu's netloader** instead sends one binary per visit to the menu, and a suite landed
that way waits at `Press + to exit` like it always did. That is the path to take when you want the
per-case detail of Step 5.

See `/code-deploy` for prerequisites (Atmosphère, network).

### Step 5 — read the results

Suites report in [TAP 14](https://testanything.org/), to three readers that cannot reach each other.
**Prefer the host stream** — it is the only one you can read yourself.

**TAP to the host — add `--server` to the push.**

```bash
just deploy buildDir/subprojects/tests/nx-tests-sync.nro --retries 60 --server
```

`--server` keeps a stdio server up after the transfer, and the suite connects to it once its cases
are over and writes its document. You get per-case results in the terminal, so **a run can be
confirmed without asking the user anything**:

```
TAP version 14
# suite: sync
# build: 0.1.0
# hos: 20.1.5 (AMS)
# mode: unattended
ok 1 - mutex_lock_unlock_single_thread
not ok 2 - remutex_reentrancy_single_thread
  ---
  rc: 0xFFFFFF9B
  ...
ok 3 - condvar_basic_wait_wake_one # SKIP
1..3
```

`# SKIP` is a case that declined to run (missing console state); `# TODO` is one not written yet —
a TAP harness counts neither as a failure. A `not ok` with no directive is a real failure, and the
indented block carries the result code.

**TAP on the SD card** — the same document at `sdmc:/switch/nx-tests/<suite>.tap`, written whether
or not a host is listening. It is the record when a suite was launched by hand; retrieval is out of
band.

**The runner's screen — the run at a glance.** The per-suite tallies accumulate into a table (suite,
ok, failed, skipped, totals, and a `PASSED`/`N FAILED` verdict), kept in
`sdmc:/switch/nx-tests/results.log` so it survives the runner's own restarts. Ask the user what it
shows when you have no host stream. `-` clears it and deletes the log; `+` exits.

**The recording table — a fallback with no network in it.**

```bash
.agents/skills/code-test/read-test-results.sh <console-ip> buildDir/subprojects/tests/<binary>.elf
```

It attaches over Atmosphère's GDB stub and reads `g_test_results` out of the **running** process, so
it needs that process to still be there — and a suite the runner launched has already exited. Reach
for it when the console has no network to report over, or to inspect a suite still sitting on screen
after being launched from hbmenu. Otherwise the TAP stream carries the same per-case detail with far
less ceremony.

Works for any harness-based binary. The `nx-tests` runner runs no cases, and the interactive applet
binaries (`nx-tests-applet-*`) record none, so those still need a person watching the screen.

**Never assume tests passed**: the result reaches you over TAP, from the console, or not at all.

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
afterwards — which is why the same TAP document is also written to the SD card and sent
to the host, and why a diagnostic worth capturing goes in a `volatile` global or a TAP
line, never a bare print.

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
- `source/runner/` — the `nx-tests` runner: serves the netloader protocol, launches what it receives,
  and keeps the run's results (`ledger.c`) across its own restarts
- `source/runner/handback.h` — the two arguments the runner and a suite say things to each other with
- `source/suites/handback.h` — the suite side of them: am I unattended, and reporting on the way out
- `source/suites/tap.h`, `tap.c` — TAP 14 reporting: the console as cases finish, the SD card and the
  host once they are all over. The only place that knows what the protocol looks like
- `source/suites/harness.h` — test framework macros, which report through `tap.h`
- `source/suites/rand/` — RNG tests
- `source/suites/rt/` — runtime tests (working directory derived from the command line)
- `source/suites/thread/` — thread tests
- `source/suites/sync/` — synchronization primitive tests
- `source/suites/fs/` — SD card and savedata tests (own binary; needs a card)
- `source/suites/net/` — socket driver and resolver tests (own binary; brings up a network stack)

Each area is its own binary, with its `main.c` beside its cases.

### How a run holds together

The runner tells every suite it launches where to come back to (`--nx-tests-runner=<path>`). That one
argument carries two things:

- **Where to return.** The suite asks the process loader to run the runner next, so the runner comes
  back instead of the homebrew menu and the run continues past the first suite.
- **That nobody is watching.** A suite launched with it runs unattended and exits on its own; a suite
  launched without it waits at `Press + to exit` as before. Nothing else distinguishes the two.

On the way back the suite reports its tally (`--nx-tests-result=<suite>:<passed>:<failed>:<skipped>`),
which the runner appends to `sdmc:/switch/nx-tests/results.log`. The runner is a fresh process after
every hand-back, so a result it is not told at startup is one it never learns — and a runner started
any other way is a new run, which is why **pushing the runner itself clears the table and the log**.

A new suite joins the run by opening with `tap_begin("<name>", VERSION, unattended)`, closing with
`tap_plan()` and `tap_report("<name>", …)`, calling `handback_to_runner("<name>")` on its way out of
`main`, and breaking out of its loop when `suite_is_unattended()`. A suite that does none of that
still runs; it just reports nothing and ends the run at itself.

The one name it passes to all four is what it is known by everywhere: the TAP document, the `.tap`
file, the runner's table, and the ledger.

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
- Building the suites without `use_nx=enabled` — they would link against libnx and not exercise the Rust replacements.
- Deploying `nx-tests.nro` expecting test results — it is the runner, and it runs no cases. Pushing
  it mid-run also clears the table, since a new runner is a new run.
- Reading a suite the runner launched with `read-test-results.sh` — it exited as soon as its cases
  finished, so there is no process to attach to. Use `--server` and read its TAP instead.
- Pushing a suite without `--server` and then asking the user what the screen said — the TAP stream
  is there for the asking and carries every case.
- Printing a diagnostic with a bare `printf` inside a suite — it lands in the middle of a TAP
  document and corrupts it. Use `tap_comment()`, which a TAP reader ignores.
- Treating a refused push as a flaky network without checking the console — mid-run it means the
  runner never came back, which is a test result.
- Assuming a deploy succeeded without on-console confirmation.
- Skipping the NRO run on FFI-surface changes.
- Reporting a suite as passing without either reading the result table or asking the user.
- Putting a diagnostic in a `printf` and expecting to read it back — only recorded state survives the run.

## Pre-approved Commands

Runnable without user permission:
- `just list-options-configured`
- `just build-tests`
- `just deploy <nro-path>`, `just deploy <nro-path> --retries <n> --server`
- `just reconfigure -Duse_nx=enabled`
- `.agents/skills/code-test/read-test-results.sh <ip> <elf>` — attaches read-only over the GDB stub and detaches.

## Related Skills

- `/code-format` — Format before testing.
- `/code-check` — Must be green before running this skill.
- `/code-build` — Building targets (including `just build-tests`).
- `/code-deploy` — Deploying NRO files to the Switch.
