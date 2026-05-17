# Debugging Crashes with Atmosphère's GDB Stub

This document describes how to trap and debug crashes in deployed Switch homebrew (NRO) builds using
Atmosphère's standalone `dmnt.gen2` GDB stub.

## Table of Contents

- [Overview](#overview)
- [Console Setup](#console-setup)
- [Toolchain Access](#toolchain-access)
- [Critical Gotchas](#critical-gotchas)
- [Step-by-Step Workflow](#step-by-step-workflow)
- [Decoding a Crash PC to Source](#decoding-a-crash-pc-to-source)
- [Reading the Backtrace — Caveats](#reading-the-backtrace--caveats)
- [Decoding Atmosphère Fatal Codes](#decoding-atmosphère-fatal-codes)
- [Quick Reference](#quick-reference)

## Overview

Atmosphère's standalone `dmnt.gen2` GDB stub lets you attach a host GDB to a crashing NRO over TCP
and capture a live backtrace, register state, and module layout. It is the most reliable way to find
out *where* a deployed build faulted.

**When to use it**: a deployed `nx-tests.nro` (or other homebrew) faults on the console with an
Atmosphère fatal screen showing a `2XXX-YYYY` result code. The fatal screen alone only tells you the
*kind* of fault (e.g. DataAbort) — the GDB stub tells you the *location*.

The stub:

- Listens on **TCP port 22225**.
- Allows **one** GDB client at a time (single-session).
- Lets you `attach` to a running process, `continue` until it faults, and inspect registers,
  memory, threads, and module load addresses.

## Console Setup

1. On the SD card, edit `atmosphere/config/system_settings.ini` and set:

   ```ini
   [atmosphere]
   enable_standalone_gdbstub = u8!0x1
   enable_htc               = u8!0x0
   ```

   `enable_htc` must be disabled — HTC and the standalone GDB stub are mutually exclusive.

2. **Reboot the console** after changing `system_settings.ini`. The setting is only read at boot.

3. The stub now listens on **TCP port 22225**.

4. The console and the dev machine must be on the **same LAN**. Unlike `nxlink` (used by
   `just deploy`), GDB cannot auto-discover the console — it needs the console's **explicit IP**.
   Find it on the console under **System Settings → Internet → Connection Status**.

## Toolchain Access

All devkitPro tools are reached through `just` recipes in the `devkitpro` group — thin passthroughs
to the `aarch64-none-elf-*` binaries:

| Recipe           | Tool                    | Use                                                   |
|------------------|-------------------------|-------------------------------------------------------|
| `just gdb`       | `aarch64-none-elf-gdb`  | Attach to the dmnt.gen2 stub                          |
| `just addr2line` | `aarch64-none-elf-addr2line` | Resolve a file offset to a source location       |
| `just objdump`   | `aarch64-none-elf-objdump`   | Disassemble around a crash offset                |
| `just nm`        | `aarch64-none-elf-nm`        | List symbols in an ELF                           |
| `just readelf`   | `aarch64-none-elf-readelf`   | Inspect ELF headers / segments                   |

Never invoke `aarch64-none-elf-gdb` (or `meson` / `ninja`) directly — always go through the `just`
recipes so the toolchain prefix stays consistent.

> **Critical pitfall — the `just gdb` recipe expands `*ARGS` unquoted.** Multi-word arguments such as
> `-ex 'set architecture aarch64'` get word-split by the shell and fail (`set` and `architecture`
> arrive as separate tokens). **Always drive GDB with a command file** via `-x /path/to/cmds.gdb`,
> with one command per line. Do not rely on inline `-ex` for any multi-word command.

## Critical Gotchas

Read these before you start — every one of them will silently waste a debugging session if missed:

- **Single-session stub.** Only one GDB client can be connected at a time. Disconnect any existing
  client (`disconnect` / kill the stale process) before attaching.
- **NROs run inside `hbloader`.** Homebrew launched from hbmenu is hosted by the `hbloader`
  process — there is no process named `nx-tests`. Attach to the **`hbloader` PID**, found via
  `info os processes`.
- **`hbloader` exits when the NRO crashes.** After every fatal you must relaunch hbmenu on the
  console and **re-probe for the new `hbloader` PID** — it changes on each launch.
- **Suppress synthetic `SIGTRAP`s.** The entry-point breakpoint and DLL-load notifications raise
  `SIGTRAP`. Without `handle SIGTRAP nostop noprint pass`, GDB stops on those instead of the real
  fault, and the script dumps state at the wrong place.
- **Attach *before* deploying.** The NRO can fault within milliseconds of being loaded. GDB must
  already be attached and sitting in `continue` *before* `just deploy` runs, or the fault happens
  before you ever see it.

## Step-by-Step Workflow

Throughout this section, substitute the placeholders:

- `<ip>` — the console's LAN IP (e.g. `192.168.1.100`).
- `<pid>` — the `hbloader` process ID (changes every launch).
- `<elf>` — the unstripped host ELF that produced the NRO
  (e.g. `buildDir/subprojects/tests/nx-tests.elf`).
- `<workdir>` — a scratch directory for command files and logs (e.g. `/tmp/claude/gdb`).

### 1. Verify connectivity

The Switch drops Wi-Fi when asleep — wake it and re-open hbmenu first, then check the host can
reach both the console and the stub port:

```bash
ping -c2 <ip>
timeout 5 bash -c '</dev/tcp/<ip>/22225 && echo open'
```

If the port check does not print `open`, the stub is not listening — confirm `system_settings.ini`
is correct and the console was rebooted after the change.

### 2. Probe for the `hbloader` PID

Write a small `probe.gdb` command file:

```gdb
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote <ip>:22225
info os processes
disconnect
```

Run it in batch mode and filter the output for `hbloader`:

```bash
just gdb --batch -x <workdir>/probe.gdb | grep hbloader
```

The matching line gives you the current `<pid>`. The `disconnect` at the end releases the
single-session stub so the real attach can connect.

### 3. Write the crash-trap command file

Write `cmds.gdb`, substituting `<ip>`, `<elf>`, `<pid>`, and `<workdir>`:

```gdb
set pagination off
set architecture aarch64
set tcp connect-timeout 10
set logging file <workdir>/session.log
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
echo \n--- end ---\n
```

What the key lines do:

- `set logging redirect on` — sends GDB's output to `session.log` instead of stdout, so the
  registers and backtrace land in the file rather than scrolling past in the terminal.
- `handle SIGTRAP nostop noprint pass` — suppresses the synthetic traps (see Gotchas).
- `attach <pid>` — attaches to the running `hbloader` process.
- `continue` — **blocks** until the process faults. The script does not advance past this line
  until the NRO crashes.
- `monitor get modules` — prints the runtime module load addresses, which are required to rebase a
  crash PC back to the ELF (see [Decoding a Crash PC to Source](#decoding-a-crash-pc-to-source)).

### 4. Launch GDB backgrounded, then deploy

GDB must be attached and inside `continue` before the NRO loads. Start it in the background, then
deploy:

```bash
just gdb --batch -x <workdir>/cmds.gdb > <workdir>/stdout.log 2>&1 &
just deploy buildDir/subprojects/tests/nx-tests.nro --address <ip>
```

When the NRO faults, `continue` returns, the script dumps registers / backtrace / modules into
`session.log`, prints `--- end ---`, and GDB detaches.

### 5. Read `session.log`

Open `<workdir>/session.log` and look for:

- The fault line — `Thread N received signal SIGSEGV` (or `SIGBUS`, `SIGILL`, …).
- The `info registers` dump — note `pc` and `x30` (the link register / `lr`).
- The backtrace from `bt 40` / `thread apply all bt 30`.
- The `Modules:` block from `monitor get modules` — the runtime base addresses.

## Decoding a Crash PC to Source

Runtime addresses are **not** ELF addresses — the loader places each module at a randomized base.
A crash `pc` must be rebased to the ELF before symbols mean anything:

1. From the `monitor get modules` output, find the NRO module's base address — the line for your
   build's ELF:

   ```text
   0x6ff7874000 - 0x6ff7b70fff nx-tests.elf
   ```

   Here `MODULE_BASE = 0x6ff7874000`.

2. Compute the file offset:

   ```text
   OFF = PC - MODULE_BASE
   ```

3. Resolve the offset to a source location:

   ```bash
   just addr2line -f -i -p -C -e <elf> <OFF>
   ```

4. Disassemble around it to see the faulting instruction in context:

   ```bash
   just objdump -d --start-address=<OFF-32> --stop-address=<OFF+16> -C <elf>
   ```

5. Repeat steps 2–4 for `x30` / `lr` to find the **caller's** return address — useful when frame
   `#0` is in a leaf function and you need the call site.

### Worked example

Say `session.log` shows `pc = 0x6ff78a41c8` and `monitor get modules` reports the NRO based at
`0x6ff7874000`:

```bash
# OFF = 0x6ff78a41c8 - 0x6ff7874000 = 0x301c8
just addr2line -f -i -p -C -e buildDir/subprojects/tests/nx-tests.elf 0x301c8
just objdump -d --start-address=0x301a8 --stop-address=0x301d8 -C buildDir/subprojects/tests/nx-tests.elf
```

If the same dump shows `x30 = 0x6ff789f3b4`, its offset is `0x6ff789f3b4 - 0x6ff7874000 = 0x2b3b4`:

```bash
just addr2line -f -i -p -C -e buildDir/subprojects/tests/nx-tests.elf 0x2b3b4
```

## Reading the Backtrace — Caveats

Treat the backtrace with suspicion. Switch homebrew threads carry **no frame pointer and no unwind
info**, so GDB cannot reliably walk the stack:

- Frames past `#0` are frequently `?? ()` — GDB simply cannot find the previous frame.
- Worse, GDB may print **plausible-looking but bogus** frames. It scans the stack for values that
  *look* like return addresses and maps each one to whatever line-table entry covers it. This is
  deterministic — you will get the same wrong answer every time — but it is still wrong.
- **Trust only frame `#0`** unless a higher frame is independently corroborated (e.g. it matches
  `x30`/`lr`, or it is consistent with the code path you expect).

When the backtrace is untrustworthy, inspect the stack manually:

```gdb
info registers
x/32a $sp
```

- `info registers` — `pc` is the fault site; `x30` is the most-recent return address.
- `x/32a $sp` — dumps 32 stack words as addresses; scan for entries inside the NRO module's
  address range, then rebase and resolve them as in
  [Decoding a Crash PC to Source](#decoding-a-crash-pc-to-source).

## Decoding Atmosphère Fatal Codes

The fatal screen shows a code of the form `2XXX-YYYY`:

- `2XXX` identifies the **result module** that raised the fault.
- `YYYY` is the module-specific **description** (the kind of error).

Look up the module in the Atmosphère sources vendored in the repo:

```text
vendor/Atmosphere--Atmosphere-NX/libraries/libvapours/include/vapours/results/<module>_results.hpp
```

For example, `2168` is `ams::creport` (the crash-report module). Within `creport_results.hpp`,
description `0002` corresponds to **DataAbort** — so a fatal `2168-0002` is a data abort caught by
the crash reporter. Match the `YYYY` value against the result definitions in that header to get the
exact fault description.

## Quick Reference

| Step                    | Command                                                                       |
|-------------------------|-------------------------------------------------------------------------------|
| Ping the console        | `ping -c2 <ip>`                                                               |
| Check stub port         | `timeout 5 bash -c '</dev/tcp/<ip>/22225 && echo open'`                        |
| Probe for `hbloader`    | `just gdb --batch -x <workdir>/probe.gdb \| grep hbloader`                     |
| Trap the crash          | `just gdb --batch -x <workdir>/cmds.gdb > <workdir>/stdout.log 2>&1 &`         |
| Deploy the NRO          | `just deploy <nro> --address <ip>`                                            |
| Rebase a runtime addr   | `OFF = PC - MODULE_BASE` (from `monitor get modules`)                          |
| Resolve to source       | `just addr2line -f -i -p -C -e <elf> <OFF>`                                    |
| Disassemble around it   | `just objdump -d --start-address=<OFF-32> --stop-address=<OFF+16> -C <elf>`    |
| Dump the stack          | `x/32a $sp` (inside the GDB session)                                          |

**Reminders**

- Stub: **TCP 22225**, single client, requires `enable_standalone_gdbstub=u8!0x1` +
  `enable_htc=u8!0x0` and a reboot.
- Attach to the **`hbloader` PID**, not the NRO name — and re-probe after every crash.
- Always use a command file (`-x cmds.gdb`); never multi-word inline `-ex`.
- Attach **before** `just deploy`; the NRO can fault in milliseconds.
- Trust frame `#0` only — there is no unwind info on Switch homebrew threads.
