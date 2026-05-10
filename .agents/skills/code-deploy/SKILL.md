---
name: code-deploy
description: Deploy build artifacts (e.g., NRO files) to a Nintendo Switch via cargo-nx link. Use when sending built homebrew to the console for testing or hardware validation.
allowed-tools: "Bash(just --list:*), Bash(just deploy:*), Bash(just gdb:*), Bash(just addr2line:*), Bash(just objdump:*), Bash(just nm:*), Bash(just readelf:*), Bash(just cxxfilt:*), Bash(just size:*)"
---

# Code Deploy Skill

Deploy NRO/NSP artifacts to a Nintendo Switch console via `cargo nx link` (nxlink).

## When to Use This Skill

- Send a build artifact (NRO) to the Switch for testing.
- Deploy the nx-tests NRO as part of a hardware-test workflow.
- Verify built homebrew runs correctly on real hardware.

## Prerequisites

1. **Switch** running a homebrew environment (e.g., Atmosphère).
2. **nxlink server** running on the Switch (typically launched via hbmenu's netloader).
3. **Network connectivity** between dev machine and Switch.
4. **`cargo-nx`** installed. If missing, **ask the user** to run `just install-cargo-nx` — do NOT run it yourself.

## Workflow

### Step 1 — Build the artifact

Use `/code-build` to compile the target first. Common outputs:

| Target   | Path                                         |
|----------|----------------------------------------------|
| hbmenu   | `buildDir/subprojects/nx-hbmenu/hbmenu.nro`  |
| nx-tests | `buildDir/subprojects/tests/nx-tests.nro`    |

### Step 2 — Deploy

```bash
just deploy <path-to-file.nro>
```

Examples:
```bash
just deploy buildDir/subprojects/nx-hbmenu/hbmenu.nro
just deploy buildDir/subprojects/tests/nx-tests.nro
just deploy buildDir/subprojects/tests/nx-tests.nro --address 192.168.1.100
```

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
- `just gdb`, `just addr2line`, `just objdump`, `just nm`, `just readelf`, `just cxxfilt`, `just size` — devkitPro toolchain passthroughs for debugging/inspection (read-only against local ELFs and the user's own Switch GDB stub).

**Not pre-approved**: `just install-cargo-nx` (mutates the host's cargo install state — ask the user).

## Debugging a Crash with the dmnt.gen2 GDB Stub

When a deployed NRO crashes (Atmosphère fatal `2XXX-YYYY`), capture a live backtrace via Atmosphère's GDB stub on **TCP 22225**.

**Console config** (`atmosphere/config/system_settings.ini`): `enable_standalone_gdbstub=u8!0x1`, `enable_htc=u8!0x0`.

**Decode fatal codes**: `2XXX` = result module → `vendor/Atmosphere--Atmosphere-NX/libraries/libvapours/include/vapours/results/<module>_results.hpp`. `2168` = `ams::creport` (e.g. `0002` = DataAbort).

### Critical gotchas

- **Single-session stub** — disconnect any existing GDB client first.
- **NROs run inside `hbloader`** — attach to the `hbloader` PID (`info os processes`), not the NRO name.
- **`hbloader` exits on NRO crash** — relaunch hbmenu and re-find the new PID after every fatal.
- **Suppress synthetic SIGTRAPs** (entry-point BP + DLL-load notifications) with `handle SIGTRAP nostop noprint pass`, otherwise the script stops at the wrong place.
- **Attach BEFORE `just deploy`** — the NRO can crash within milliseconds of load.

### Workflow

All toolchain access goes through `just` tasks in the `devkitpro` group: `gdb`, `addr2line`, `objdump`, `nm`, `readelf`.

1. Probe for hbloader's PID:
   ```bash
   just gdb --batch \
     -ex 'set architecture aarch64' \
     -ex 'target extended-remote <ip>:22225' \
     -ex 'info os processes' -ex 'disconnect' | grep hbloader
   ```

2. Write `/tmp/claude/gdb/cmds.gdb` (substitute `<ip>`, `<elf>`, `<pid>`):
   ```gdb
   set pagination off
   set architecture aarch64
   set logging file /tmp/claude/gdb/session.log
   set logging overwrite on
   set logging redirect on
   set logging enabled on
   handle SIGTRAP nostop noprint pass
   target extended-remote <ip>:22225
   file <elf>
   attach <pid>
   continue
   echo \n--- stop ---\n
   info registers
   bt 40
   thread apply all bt 30
   monitor get modules
   ```

3. Run it backgrounded, then `just deploy` the NRO:
   ```bash
   just gdb --batch -x /tmp/claude/gdb/cmds.gdb > /tmp/claude/gdb/stdout.log 2>&1 &
   just deploy buildDir/.../foo.nro
   ```

4. After the fault, read `/tmp/claude/gdb/session.log`. Resolve PC → source (`MODULE_BASE` from `monitor get modules`):
   ```bash
   OFF=$(printf '0x%x' $((PC - MODULE_BASE)))
   just addr2line -f -i -p -e <elf> $OFF
   just objdump -d --start-address=$((OFF-32)) --stop-address=$((OFF+16)) <elf>
   ```

Frames past #0 are usually `??` (no FP, no unwind info) — inspect `x30`/`lr` and `x/32a $sp` manually.

## Related Skills

- `/code-build` — Build artifacts before deploying.
- `/code-format` — Format code before building.
- `/code-test` — Full hardware-test workflow (build + deploy + confirm).
