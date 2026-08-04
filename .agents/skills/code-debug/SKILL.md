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
4. **An NRO built *without* `cargo-nx.txt`.** `just configure` passes
   `--cross-file cargo-nx.txt`, which routes the `bundle` step through `cargo nx bundle` and
   produces an NRO that faults at `module_base` before reaching any of your code. Debugging that
   build means chasing a crash that is not yours. Reconfigure with the devkitPro bundler first:

   ```bash
   meson setup --cross-file devkitpro.txt --cross-file cross.txt buildDir <options>
   ```

   Changing cross-files needs a fresh setup, not `--reconfigure`; move `buildDir/cargo-target`
   aside and restore it afterwards to keep the Rust rebuild incremental.

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
- **Never suppress `SIGTRAP` when hunting an abort.** Rust panics and libnx's
  `diagAbortWithResult` both terminate via `svcBreak`, which arrives as `SIGTRAP`. A capture with
  `handle SIGTRAP nostop noprint pass` returns an attach banner and nothing else, which reads
  exactly like "no fault occurred". Use `handle SIGTRAP stop print nopass`.
- **An empty capture is not evidence of no crash.** Between a stale PID and a suppressed `SIGTRAP`,
  the two most common failures both produce a clean, empty, successful-looking log. Confirm the
  PID is live and `SIGTRAP` is unsuppressed before concluding anything from silence.
- **Debug the right NRO.** A `cargo-nx.txt`-bundled build crashes at `module_base` regardless of
  your code (see [Prerequisites](#prerequisites)).

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

**Re-probe immediately before every attach.** `hbloader` exits when the NRO crashes and hbmenu
relaunches it with a fresh PID — a single debugging session routinely walks through several
(`138 → 144 → 146`). Attaching to a PID probed before the previous crash silently attaches to a
dead process: GDB reports success and then traps nothing.

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

# Do NOT use `handle SIGTRAP nostop noprint pass` here — see below.
handle SIGTRAP stop print nopass

target extended-remote <ip>:22225
file <elf>
attach <pid>

# Module load raises traps of its own, so the fault is rarely the first stop.
# Dump the break arguments at each of several stops and pick the informative one.
continue
echo \n=== STOP 1 ===\n
info registers x0 x1 x2 pc
printf "break reason=%d msg_len=%d\n", $x0, $x2
x/s $x1
bt 25
monitor get modules

continue
echo \n=== STOP 2 ===\n
info registers x0 x1 x2 pc
printf "break reason=%d msg_len=%d\n", $x0, $x2
x/s $x1
bt 25

continue
echo \n=== STOP 3 ===\n
info registers x0 x1 x2 pc
printf "break reason=%d msg_len=%d\n", $x0, $x2
x/s $x1
bt 25
echo \n=== END ===\n
```

> [!CAUTION]
> **Never suppress `SIGTRAP` when hunting an abort.** A Rust panic (`nx-panic-handler`) and libnx's
> `diagAbortWithResult` both terminate through **`svcBreak`**, which reaches GDB as `SIGTRAP`.
> `handle SIGTRAP nostop noprint pass` tells GDB to ignore exactly the event you are trying to
> catch: the capture completes, logs nothing but the attach banner, and looks indistinguishable
> from "the process never faulted". Suppress it only when chasing a genuine memory fault and the
> load-time traps are drowning the session.

`continue` **blocks** until the next stop. `monitor get modules` prints runtime module bases —
required to rebase a crash PC.

### Reading a `svcBreak` stop — the fast path

`svcBreak(reason, address, size)` follows the AArch64 C ABI, so at the stop the arguments are still
in registers:

| Register | Meaning                                                              |
|----------|----------------------------------------------------------------------|
| `x0`     | `BreakReason` — **`0` is `BREAK_REASON_PANIC`** (a Rust panic)        |
| `x1`     | Pointer to the message buffer                                         |
| `x2`     | Message length in bytes                                               |

So **`x/s $x1` prints the panic message directly**, including the `file:line:col` Rust records:

```
break reason=0 msg_len=97
0xc233a73a0: "panicked at subprojects/nx-service-fs/src/cmif/proxy.rs:211:10:
              server returned filesystem object"
```

That one line identifies the fault precisely, with no module rebasing and no reliance on a
backtrace — which is fortunate, because Switch threads carry no unwind info and `bt` past frame
`#0` is usually junk (see the caveat below).

**A stop with `reason=0` but `x1 == 0` and `x2 == 0` is not the interesting one.** `nx-panic-handler`
elects a single winner to format the shared message buffer; concurrent or nested panics take the
loser path and break with a null, zero-length message. Keep issuing `continue` until you find the
stop that carries a real pointer and length.

**`x0` also discriminates the two abort kinds**, which is often the whole question:

| Observation                          | Meaning                                                                 |
|--------------------------------------|--------------------------------------------------------------------------|
| `reason=0`, non-null `x1`            | Rust panic — the message names the `expect`/index and its source location |
| `reason` ≠ 0, or empty buffer        | libnx `diagAbortWithResult` — an error *return* was propagated, not a fault |

### When the program produces no output at all

If a crash happens during `__libnx_init` / `__appInit` — before `main()` — then **nothing** the
program prints can help: `stdout` is not wired up yet. Do not spend a cycle on
`cargo nx link -s` (the nxlink stdio server) expecting a panic message; the panic handler writes to
`svcBreak`, never to `stdout`. An empty stdio capture is evidence the fault is pre-`main`, nothing
more.

### Step 4 — Launch GDB backgrounded, then deploy

GDB must be attached and inside `continue` *before* the NRO loads. Backgrounding the shell job is
not enough on its own — wait until the log shows the attach actually landed, then deploy:

```bash
just gdb --batch -x <workdir>/cmds.gdb > <workdir>/stdout.log 2>&1 &

# Block until GDB is attached, so the deploy cannot race ahead of it.
until grep -q "New Thread" <workdir>/session.log 2>/dev/null; do sleep 2; done

just deploy buildDir/subprojects/tests/nx-tests.nro --address <ip>
```

On each stop `continue` returns, the script dumps that stop, and after the last one GDB detaches.

### Step 5 — Read `session.log`

Read `<workdir>/session.log` and extract, in this order:

- **The `svcBreak` arguments at each stop** — `break reason=` and the `x/s $x1` string. For an
  abort this is usually the whole answer; see
  [Reading a `svcBreak` stop](#reading-a-svcbreak-stop--the-fast-path).
- The fault line — `Thread N received signal SIGSEGV` (or `SIGBUS`, `SIGILL`, `SIGTRAP`, …).
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
