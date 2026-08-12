---
name: code-deploy
description: Deploy build artifacts (e.g., NRO files) to a Nintendo Switch via cargo-nx link, arming hbmenu's netloader first when nothing is listening. Use when sending built homebrew to the console for testing or hardware validation.
allowed-tools: "Bash(just --list:*), Bash(just deploy:*), Bash(just arm-netloader:*), Bash(just deploy-armed:*), Bash(just gdb:*), Bash(just addr2line:*), Bash(just objdump:*), Bash(just nm:*), Bash(just readelf:*), Bash(just cxxfilt:*), Bash(just size:*)"
---

# Code Deploy Skill

Deploy NRO/NSP artifacts to a Nintendo Switch console via `cargo nx link` (nxlink).

## When to Use This Skill

- Send a build artifact (NRO) to the Switch for testing.
- Deploy the nx-tests NRO as part of a hardware-test workflow.
- Verify built homebrew runs correctly on real hardware.

## Prerequisites

1. **Switch** running a homebrew environment (e.g., Atmosphère).
2. **Something listening** for the transfer on the Switch: hbmenu's netloader, or the `nx-tests`
   runner, which serves the same protocol and launches each suite it receives. hbmenu's netloader
   disarms after every transfer, so a second push needs it armed again — which this skill can do
   itself, see [Arming the netloader](#arming-the-netloader).
3. **Network connectivity** between dev machine and Switch.
4. **`cargo-nx`** installed. If missing, **ask the user** to run `just install-cargo-nx` — do NOT run it yourself.

## Workflow

### Step 1 — Build the artifact

Use `/code-build` to compile the target first. Common outputs:

| Target          | Path                                             |
|-----------------|--------------------------------------------------|
| hbmenu          | `buildDir/subprojects/nx-hbmenu/hbmenu.nro`      |
| nx-tests runner | `buildDir/subprojects/tests/nx-tests.nro`        |
| a test suite    | `buildDir/subprojects/tests/nx-tests-<area>.nro` |

### Step 2 — Deploy

```bash
just deploy <path-to-file.nro>
```

Examples:
```bash
just deploy buildDir/subprojects/nx-hbmenu/hbmenu.nro
just deploy buildDir/subprojects/tests/nx-tests-sync.nro
just deploy buildDir/subprojects/tests/nx-tests-sync.nro --address 192.168.1.100
just deploy buildDir/subprojects/tests/nx-tests-sync.nro --retries 60
```

`--retries` keeps knocking instead of failing, which is what makes a push land on a console that is
busy right now but will be listening shortly — pushing one suite to the `nx-tests` runner while the
previous suite is still running is the usual case. Prefer it over sleeping between attempts.

`--server` keeps a stdio server up after the transfer, so anything the homebrew sends back lands in
your terminal. For a test suite that is its TAP report, which is how a run is confirmed without
asking anyone what the screen says — see `/code-test`.

### Arming the netloader

A push needs something listening, and hbmenu's netloader disarms after each transfer. Rather than
asking for Y to be pressed once per suite, press it through Atmosphère's debug stub:

```bash
just arm-netloader 192.168.1.129
just deploy buildDir/subprojects/tests/nx-tests-fs.nro --retries 60 --server

# Or as one step, for a console sitting at the menu:
just deploy-armed 192.168.1.129 buildDir/subprojects/tests/nx-tests-fs.nro --retries 60 --server
```

Three things this needs, each of which makes it fail cleanly rather than silently:

- **The console's IP**, spelled out. `cargo nx link` finds a console by broadcasting, but the debug
  stub is addressed directly and cannot be discovered.
- **The debug stub enabled** — `enable_standalone_gdbstub` and `enable_htc=0`, then a reboot. See
  `/code-debug`, which owns the mechanism and explains how the press is delivered.
- **hbmenu on screen.** Whichever build it is: the press site is found by searching the loaded
  module for the function's own code, so a stock menu is reached the same as one built here.
  Verified against hbmenu v3.6.1.

Arming is for **hbmenu**. The `nx-tests` runner listens continuously and has nothing to arm, so a
push aimed at it needs none of this — `--retries` covers the gap while the previous suite runs.

### Step 3 — Confirm with the user

🚨 **Always ask the user to confirm** the deployment ran as expected on the console:
- General deploys: ask whether the homebrew launched and behaved correctly.
- Test deploys: ask whether the test suite **PASSED** on the console screen.

**Do NOT assume success** — output is only visible on the Switch.

## Retry on Failure

If deployment fails (Switch not on network, nxlink down), **retry up to 3 times with a 10-second delay** between attempts:

```bash
just deploy buildDir/path/to/file.nro
sleep 10
just deploy buildDir/path/to/file.nro
sleep 10
just deploy buildDir/path/to/file.nro
```

Common failure causes:
- Switch not connected to network.
- nxlink server not running on the Switch.
- Network timeout.

If three attempts fail, surface the error to the user — do not loop indefinitely.

## Anti-patterns

- **Never run `just install-cargo-nx` yourself** — ask the user to run it.
- **Never claim deployment succeeded** without on-console confirmation.
- **Never invoke `cargo nx link` directly** — use `just deploy` (the recipe handles flags).
- **Never deploy an artifact that hasn't been freshly rebuilt** if you just edited code — go through `/code-build` first.

## Pre-approved Commands

Runnable without user permission:
- `just --list` — read-only introspection.
- `just deploy <path>` — sends the artifact via nxlink; no other side effects on the dev machine.
- `just arm-netloader <ip>`, `just deploy-armed <ip> <path>` — presses Y on the console's own hbmenu through the debug stub; touches nothing on the dev machine and nothing on the console beyond that press.
- `just gdb`, `just addr2line`, `just objdump`, `just nm`, `just readelf`, `just cxxfilt`, `just size` — devkitPro toolchain passthroughs for debugging/inspection (read-only against local ELFs and the user's own Switch GDB stub).

**Not pre-approved**: `just install-cargo-nx` (mutates the host's cargo install state — ask the user).

## Debugging a Crash

When a deployed NRO faults, `/code-debug` owns the procedure: attaching to Atmosphère's `dmnt.gen2`
stub, capturing the fault, and decoding an address back to a source location. It is also what the
netloader arming above goes through.

This skill deliberately keeps no copy of that procedure. It had one, and it had drifted into
advising the opposite of what `/code-debug` says on two points that decide whether a capture works
at all — whether `SIGTRAP` may be suppressed, and whether GDB can be driven with inline `-ex`
arguments.

## Related Skills

- `/code-build` — Build artifacts before deploying.
- `/code-format` — Format code before building.
- `/code-test` — Full hardware-test workflow (build + deploy + confirm).
- `/code-debug` — The debug stub: capturing a fault, and the button press behind `just arm-netloader`.
