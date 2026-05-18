---
name: code-debug
description: Trap and debug crashes in deployed Switch homebrew (NRO) using Atmosphère's dmnt.gen2 GDB stub. Use when a deployed NRO faults — whether the console shows a 2XXX-YYYY fatal screen or only a generic "software was closed" dialog — to attach GDB and capture a backtrace, decode a crash address (PC/LR) to a source location, or decode a 2XXX-YYYY fatal code.
allowed-tools: "Bash(just --list:*), Bash(just gdb:*), Bash(just addr2line:*), Bash(just objdump:*), Bash(just nm:*), Bash(just readelf:*), Bash(just cxxfilt:*), Bash(just size:*), Bash(just deploy:*), Bash(ping:*), Bash(timeout:*), Bash(grep:*), Bash(mkdir:*), Write, Read"
---

# Code Debug Skill

Trap and debug crashes in deployed Switch homebrew (NRO) builds using Atmosphère's standalone
`dmnt.gen2` GDB stub. All toolchain access goes through `just` recipes in the `devkitpro` group —
never invoke `aarch64-none-elf-*` binaries directly.

**Authoritative reference:** [`docs/debugging_gdb_stub.md`](../../../docs/debugging_gdb_stub.md).
This skill is the operational procedure; the doc holds the full rationale, the worked example, and
the complete fatal-code tables. Consult it for anything this skill summarizes.

## When to Use This Skill

- A deployed `nx-tests.nro` (or other homebrew) faults on the console — whether it shows an
  Atmosphère `2XXX-YYYY` fatal screen or only a generic "The software was closed because an error
  occurred" dialog (see [Run Modes & Error Reporting](#run-modes--error-reporting)).
- You need to find *where* a deployed build faulted — capture a live backtrace and register state.
- You have a crash `pc`/`x30` and need to decode it to a source location.
- You need to decode an Atmosphère `2XXX-YYYY` fatal code to a fault kind.

## Prerequisites

1. **Console setup (one-time).** On the SD card, `atmosphere/config/system_settings.ini` must set
   `enable_standalone_gdbstub = u8!0x1` and `enable_htc = u8!0x0`, followed by a **reboot**. The
   stub then listens on **TCP 22225**. See the doc's *Console Setup* section.
2. **Same LAN.** Host and console must share a LAN. GDB needs the console's **explicit IP** (it
   cannot auto-discover) — find it under *System Settings → Internet → Connection Status*.
3. **An unstripped host ELF.** Decoding addresses needs the ELF that produced the NRO (e.g.
   `buildDir/subprojects/tests/nx-tests.elf`). Build it first via `/code-build` if absent.

## Run Modes & Error Reporting

How the homebrew was launched decides **what the console can tell you on a crash** — and whether a
`2XXX-YYYY` fatal code exists at all. The GDB stub does **not** care about the mode: `dmnt.gen2`
traps the fault at the process level, so the capture workflow below works identically in every
mode. The mode only changes what the *on-screen* report is worth.

| Run mode        | How the homebrew is launched                                       | On crash, the console shows…                                                          | `2XXX-YYYY`?       |
|-----------------|--------------------------------------------------------------------|----------------------------------------------------------------------------------------|--------------------|
| **Applet**      | hbmenu opened via the Album applet (the default)                   | Generic *"The software was closed because an error occurred"* — qlaunch closes the applet | **No** (suppressed) |
| **Application** | hbmenu opened by overriding a game title (hold **R**) or a forwarder | The full Atmosphère fatal screen with the `2XXX-YYYY` result code                       | **Yes**            |
| **Sysmodule**   | Background module — no UI                                          | Nothing on screen                                                                       | No                 |

**Implications for this skill:**

- **Applet mode is the common case** (hbmenu via the Album applet) and is exactly where the GDB
  stub matters most: the fatal screen is suppressed, so the stub capture is your *only* reliable
  source of `pc`, registers, and module bases. Do **not** wait for a `2XXX-YYYY` code that applet
  mode will never produce.
- **For a `2XXX-YYYY` code, run in application mode** — launch hbmenu via title override so a crash
  surfaces the full fatal screen. This is optional: the GDB capture already pinpoints the fault
  without it.
- Applet mode also has a **much smaller heap** than application mode. An OOM-class fault that only
  reproduces in applet mode may be a symptom of the reduced heap rather than a logic bug.
- Whichever mode, the homebrew still runs inside the **`hbloader`** process — the attach workflow
  is unchanged.

## Critical Gotchas

Each of these silently wastes a debugging session if missed:

- **Single-session stub.** Only one GDB client at a time. End any stale session (`disconnect`)
  before attaching.
- **Attach to `hbloader`, not the NRO.** Homebrew launched from hbmenu runs inside the `hbloader`
  process — there is no `nx-tests` process. Attach to the **`hbloader` PID**.
- **Re-probe the PID after every crash.** `hbloader` exits when the NRO crashes; relaunch hbmenu
  and re-probe — the PID changes on each launch.
- **Attach *before* deploying.** The NRO can fault within milliseconds of loading. GDB must already
  be attached and inside `continue` before `just deploy` runs.
- **Never use inline multi-word `-ex`.** The `just gdb` recipe expands `*ARGS` unquoted, so the
  shell word-splits `-ex 'set architecture aarch64'`. **Always** drive GDB with a command file
  (`-x cmds.gdb`), one command per line.

## Workflow

**First, triage.** Confirm how the homebrew was launched and what the console displayed (see
[Run Modes & Error Reporting](#run-modes--error-reporting)). In applet mode there is no
`2XXX-YYYY` code to decode — rely on the GDB capture below. The capture steps are identical in
every mode.

Substitute throughout: `<ip>` (console LAN IP), `<pid>` (`hbloader` PID), `<elf>` (unstripped host
ELF), `<workdir>` (scratch dir, e.g. `/tmp/claude/gdb`).

### Step 1 — Verify connectivity

Wake the console and re-open hbmenu (Wi-Fi drops on sleep), then:

```bash
mkdir -p <workdir>
ping -c2 <ip>
timeout 5 bash -c '</dev/tcp/<ip>/22225 && echo open'
```

If `open` is not printed, the stub is not listening — recheck `system_settings.ini` and confirm the
console was rebooted after the change.

### Step 2 — Probe for the `hbloader` PID

Write `<workdir>/probe.gdb`:

```gdb
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote <ip>:22225
info os processes
disconnect
```

```bash
just gdb --batch -x <workdir>/probe.gdb | grep hbloader
```

The matching line gives the current `<pid>`. The `disconnect` releases the single-session stub for
the real attach.

### Step 3 — Write the crash-trap command file

Write `<workdir>/cmds.gdb`, substituting `<ip>`, `<elf>`, `<pid>`, `<workdir>`:

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

`handle SIGTRAP nostop noprint pass` suppresses synthetic entry-point / DLL-load traps. `continue`
**blocks** until the NRO faults. `monitor get modules` prints runtime module bases — required to
rebase a crash PC.

### Step 4 — Launch GDB backgrounded, then deploy

GDB must be attached and inside `continue` *before* the NRO loads:

```bash
just gdb --batch -x <workdir>/cmds.gdb > <workdir>/stdout.log 2>&1 &
just deploy buildDir/subprojects/tests/nx-tests.nro --address <ip>
```

On fault, `continue` returns, the script dumps registers / backtrace / modules to `session.log`,
prints `--- end ---`, and GDB detaches.

### Step 5 — Read `session.log`

Read `<workdir>/session.log` and extract:

- The fault line — `Thread N received signal SIGSEGV` (or `SIGBUS`, `SIGILL`, …).
- `info registers` — note `pc` (fault site) and `x30` (link register / caller return address).
- The `bt` / `thread apply all bt` backtrace — **see the caveat below**.
- The `Modules:` block — runtime base addresses, needed for the next section.

## Decoding a Crash Address to Source

Runtime addresses are randomized — rebase to the ELF before symbols mean anything.

1. From `monitor get modules`, find the NRO module's base, e.g.
   `0x6ff7874000 - 0x6ff7b70fff nx-tests.elf` → `MODULE_BASE = 0x6ff7874000`.
2. Compute the file offset: `OFF = PC - MODULE_BASE`.
3. Resolve and disassemble:

```bash
just addr2line -f -i -p -C -e <elf> <OFF>
just objdump -d --start-address=<OFF-32> --stop-address=<OFF+16> -C <elf>
```

4. Repeat for `x30`/`lr` to find the **caller's** return address — essential when frame `#0` is a
   leaf function.

See the doc's *Decoding a Crash PC to Source* section for a fully worked example.

## Reading the Backtrace — Caveat

Switch homebrew threads carry **no frame pointer and no unwind info**. **Trust only frame `#0`.**
Higher frames are frequently `?? ()`, or worse, plausible-looking but bogus (GDB scans the stack for
values that *look* like return addresses). A higher frame is trustworthy only if independently
corroborated — it matches `x30`, or fits the expected code path.

When the backtrace is untrustworthy, inspect the stack manually inside the session:

```gdb
info registers
x/32a $sp
```

Scan `x/32a $sp` for words inside the NRO module's address range, then rebase and resolve each as
above.

## Decoding Atmosphère Fatal Codes

**Only available in application mode** — applet mode suppresses the fatal screen (see
[Run Modes & Error Reporting](#run-modes--error-reporting)). If the console only showed a generic
"software was closed" dialog, there is no code to decode; rely on the GDB capture above.

A fatal screen shows `2XXX-YYYY`: `2XXX` is the result module, `YYYY` the description. Look up the
module header vendored in the repo:

```text
vendor/Atmosphere--Atmosphere-NX/libraries/libvapours/include/vapours/results/<module>_results.hpp
```

E.g. `2168` is `ams::creport`; in `creport_results.hpp`, description `0002` is **DataAbort** — so
`2168-0002` is a data abort caught by the crash reporter.

## Fallback — SD-Card Crash Reports

Atmosphère's `creport` writes a crash report to the SD card on every process crash, **regardless of
run mode** — including applet mode, where no fatal screen ever appears. Use it when GDB was not
attached in time, or to corroborate a GDB capture:

- Path on the SD card: `atmosphere/crash_reports/`.
- The report is a text dump with the faulting `pc`, register state, and a stack trace — rebase and
  decode its addresses exactly as in
  [Decoding a Crash Address to Source](#decoding-a-crash-address-to-source).

The GDB stub is still preferred — it gives a live session and lets you inspect the stack manually —
but a crash report needs no attach race and survives across reboots.

## Toolchain Recipes

All routed through `just` so the `aarch64-none-elf-*` prefix stays consistent:

| Recipe           | Use                                                   |
|------------------|-------------------------------------------------------|
| `just gdb`       | Attach to the `dmnt.gen2` stub (always with `-x`)     |
| `just addr2line` | Resolve a file offset to a source location            |
| `just objdump`   | Disassemble around a crash offset                     |
| `just nm`        | List symbols in an ELF                                |
| `just readelf`   | Inspect ELF headers / segments                        |
| `just cxxfilt`   | Demangle symbols                                      |
| `just size`      | Section size summary                                  |

## Reminders

- Stub: **TCP 22225**, single client; requires `enable_standalone_gdbstub` + `enable_htc=0` + reboot.
- Attach to the **`hbloader` PID** — re-probe after every crash.
- Always `-x cmds.gdb`; never multi-word inline `-ex`.
- Attach **before** `just deploy`.
- Trust frame `#0` only.
- Applet mode shows no `2XXX-YYYY` code — the stub works regardless of mode; don't wait for a fatal
  screen. SD `atmosphere/crash_reports/` is a no-GDB fallback.
