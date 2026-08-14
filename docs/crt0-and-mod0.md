# Switch Startup: `_start`, `.crt0`, and MOD0

This document explains the three pieces that bring a Nintendo Switch executable
(NRO/NSO/KIP) to life: the `.crt0` startup section, the `_start` entry point,
and the MOD0 relocation descriptor. It describes the mechanism as `libnx`
implements it today — `subprojects/libnx/src/nx/switch_crt0.s` and
`subprojects/libnx/src/nx/switch.ld` — which is the mechanism every `nx-std`
artifact currently inherits.

> **Status**: reference. Describes the platform and the C `libnx` runtime as
> they are today. For where `nx-std` is *heading* — owning `_start` and the
> linker script from Rust — see
> [`rust-libnx-linker-and-targets.md`](rust-libnx-linker-and-targets.md).

## Table of Contents

1. [The bootstrapping problem](#1-the-bootstrapping-problem)
2. [The three pieces at a glance](#2-the-three-pieces-at-a-glance)
3. [`.crt0` — the startup section](#3-crt0--the-startup-section)
4. [`_start` — the entry point](#4-_start--the-entry-point)
5. [MOD0 — the relocation descriptor](#5-mod0--the-relocation-descriptor)
6. [How they chain together](#6-how-they-chain-together)
7. [NSO vs NRO: one mechanism, two entry ABIs](#7-nso-vs-nro-one-mechanism-two-entry-abis)
8. [Relationship to `nx-std`](#8-relationship-to-nx-std)
9. [References](#9-references)

---

## 1. The bootstrapping problem

A Switch executable is a **position-independent** ELF (PIE). The loader maps it
at a randomized (ASLR) base address and jumps to its entry point with **no
relocations applied yet** and almost no input — just a couple of registers.

This creates a chicken-and-egg situation: the startup code must **relocate
itself** before any normal code (anything referencing a global, a GOT entry,
or an absolute address) can run. It also must do this without consulting its
own ELF program headers — a running process sees a flat in-memory image, not a
parseable ELF file.

`.crt0` + `_start` + MOD0 exist precisely to solve this:

- `.crt0` guarantees the startup code lands at a known place in the image.
- `_start` carries a tiny fixed-layout header that is readable *before*
  relocation.
- MOD0 is a self-describing, relocation-free descriptor of which regions to
  relocate and zero.

## 2. The three pieces at a glance

| Piece     | Kind             | Defined in        | Role                                                              |
|-----------|------------------|-------------------|-------------------------------------------------------------------|
| `.crt0`   | Linker section   | `switch.ld`       | Reserved slot at the front of `.text`; holds the startup code     |
| `_start`  | Entry symbol     | `switch_crt0.s`   | ELF entry point; a fixed header + the startup routine             |
| MOD0      | Data descriptor  | `switch_crt0.s`   | Relocation-free struct describing `.dynamic`, `.bss`, `.eh_frame` |

`.crt0` is the *section* (a reserved slot in the layout). `_start` is the
*entry symbol/header* placed inside it. MOD0 is the *data* that `_start` hands
to the self-relocator. The linker script ties all three together.

## 3. `.crt0` — the startup section

`.crt0` is an input **section** holding the cold-start code. The linker script
places it first in `.text`, with `KEEP`:

```ld
OUTPUT_ARCH(aarch64)
ENTRY(_start)                   /* switch.ld:1-2 */
...
.text :
{
    KEEP (*(.crt0))             /* switch.ld:22 */
    *(.text.unlikely ...)
    ...
} :code
```

Two properties matter:

- **`KEEP`** stops `--gc-sections` (enabled via `switch.specs`) from discarding
  `.crt0`. Nothing *calls* `_start`, so without `KEEP` the linker would garbage
  collect the entry code.
- **First in `.text`** — and `.text` begins at `__start__ = 0x0`
  (`switch.ld:14-17`) — so the entry header sits at the very front of the code
  segment. That fixed, predictable position is what makes the `_start` header
  discoverable by the loader and by `_start` itself.

The four `PT_LOAD` / `PT_DYNAMIC` segments are declared in `switch.ld`'s
`PHDRS` block (`code`, `rodata`, `data`, `dyn`); `.crt0` lives in the
read+execute `code` segment.

## 4. `_start` — the entry point

`switch.ld` names `_start` as the ELF entry (`ENTRY(_start)`). It is defined in
`switch_crt0.s`, in the `.crt0` section — and it is **not** plain code at offset
0. It opens with a fixed-layout header:

```asm
.section .crt0, "ax", %progbits
.global _start
_start:
    b 1f                        // +0x00: branch over the header
    .word __nx_mod0 - _start    // +0x04: RELATIVE offset to MOD0
    .ascii "HOMEBREW"           // +0x08: magic for the homebrew loader
.org _start+0x80; 1:            // real entry code begins at a fixed +0x80
```

The header is the contract between the image and whoever inspects it:

- `b 1f` jumps execution over the embedded data to the real routine.
- The `.word` at `+0x04` is a **relative** offset to MOD0 (see §5 for why
  relative). Anyone with the load address can read this word and locate MOD0.
- `"HOMEBREW"` is a magic string identifying the image format.
- `.org _start+0x80` pads so the routine starts at a guaranteed `+0x80` offset,
  leaving fixed room for the header.

The startup routine at `+0x80` then runs (see `switch_crt0.s:10-54`):

1. **Distinguish exception entry from normal launch.** The kernel reuses the
   entry point for user-mode exception callbacks. `cmp`/`ccmn` tests
   `x0 != 0 && x1 != UINT64_MAX`; if so it branches to
   `__libnx_exception_entry`. Otherwise it falls through to normal launch.
2. **Stash loader-supplied state** into callee-saved registers: `x25 = arg0`,
   `x26 = arg1`, `x27 = loader return address` (`x30`), `x28 = initial sp`.
   The meaning of arg0/arg1 is *kind-specific* — NSO entry passes
   `x0 = 0, x1 = main thread handle`; NRO (homebrew ABI) passes
   `x0 = env context ptr, x1 = -1`. This per-kind ABI is the axis along which
   the runtime forks into separate entry crates (see §7).
3. **Self-relocate**: `__nx_dynamic(aslr_base, &__nx_mod0)`. This applies the
   dynamic relocations, after which normal code is safe to run. It *needs*
   MOD0 to know where `.dynamic` and `.bss` are.
4. **Save the stack pointer** into `__stack_top`.
5. **System init**: `__libnx_init(arg0, arg1, loader_return)` — heap, services,
   TLS, etc.
6. **Hand off to `main`**: load `argc`/`argv`, set the return address (`x30`)
   to `exit`, and `b main`.

`switch_crt0.s` also defines `__nx_exit` (restores `sp` from `__stack_top` and
branches back to the loader) and reserves the `__stack_top` word in
`.bss.__stack_top`.

## 5. MOD0 — the relocation descriptor

Because a running process sees a flat image rather than a parseable ELF, it
needs a small self-describing struct telling the startup code which regions to
relocate and zero. That struct is MOD0 (`__nx_mod0` in `switch_crt0.s`):

```asm
.global __nx_mod0
__nx_mod0:
    .ascii "MOD0"
    .word  _DYNAMIC             - __nx_mod0   // dynamic section -> relocations
    .word  __bss_start__        - __nx_mod0   // BSS region to zero
    .word  __bss_end__          - __nx_mod0
    .word  __eh_frame_hdr_start - __nx_mod0   // unwind / exception tables
    .word  __eh_frame_hdr_end   - __nx_mod0
    .word  0                                  // runtime module object (unused)

    // libnx homebrew extensions:
    .ascii "LNY0"
    .word  __got_start__        - __nx_mod0   // GOT range
    .word  __got_end__          - __nx_mod0
    .ascii "LNY1"
    .word  __relro_start        - __nx_mod0   // RELRO range
    .word  __data_start         - __nx_mod0
    .ascii "LNY2"
    .word  0x1                                // version/fix field
    .word  0x0                                // reserved
```

Two design points are worth calling out:

- **Every field is a 32-bit *relative* offset** (`X - __nx_mod0`). Absolute
  addresses are unknown until load time, and — decisively — these values must
  be usable *before relocations are applied*. MOD0 cannot itself depend on the
  relocations it describes, so relative offsets are the only option. The
  `_start` header points at MOD0 the same way, for the same reason.
- **Every target is a linker-script symbol.** `_DYNAMIC`, `__bss_start__`,
  `__bss_end__`, `__eh_frame_hdr_start/end`, `__got_start__/end__`,
  `__relro_start`, `__data_start` are all `PROVIDE_HIDDEN` in `switch.ld`. The
  linker script and the crt0 are tightly coupled through these names — the
  script defines the boundaries, MOD0 quotes them.

The `"LNY0"`/`"LNY1"`/`"LNY2"` blocks are `libnx`-specific extensions appended
after the standard MOD0 fields; the standard MOD0 layout ends at the unused
"runtime module object" word.

## 6. How they chain together

```
loader maps the PIE image at a random base, jumps to _start
        │
   _start  (.crt0, at image offset 0x0)
        │   header:  word @ +0x04  ->  __nx_mod0
        │            "HOMEBREW"     ->  format magic
        ▼
   startup routine  @ _start + 0x80
        │   stash loader args (x25..x28)
        │   __nx_dynamic(base, &__nx_mod0)
        │        └─ read MOD0 -> _DYNAMIC : apply relocations
        │                     -> __bss_* : zero BSS
        ▼
   __libnx_init   ->   main   ->   exit  (==  __nx_exit  ->  back to loader)
```

The linker script underpins every arrow: `ENTRY(_start)` selects the entry,
`KEEP(*(.crt0))` at address `0x0` fixes its location, and the `PROVIDE_HIDDEN`
boundary symbols give MOD0 something concrete to point at.

## 7. NSO vs NRO: one mechanism, two entry ABIs

The `_start` header carries a `"HOMEBREW"` magic and MOD0 carries `"LNY*"`
blocks — so is this whole scheme homebrew-only? It is not.

**MOD0 is Nintendo's native format.** Every official NSO carries a standard
MOD0; the self-relocation bootstrap (§1) is a property of the PIE platform, not
of homebrew. devkitA64's `switch_crt0.s` is a *single* file that serves both
outputs. The only homebrew-specific additions are:

- the `.ascii "HOMEBREW"` magic at `_start+0x08`, and
- the `"LNY0"`/`"LNY1"`/`"LNY2"` MOD0 extension blocks.

A genuine Nintendo NSO carries just the six standard MOD0 fields — no
`"HOMEBREW"`, no `"LNY*"`. So `.crt0`, the `_start` header shape, the relative
MOD0 pointer, `__nx_dynamic` self-relocation and BSS zeroing are **identical**
for NSO and NRO.

**What differs is the entry ABI** — who launches the image, and what arrives in
`x0`/`x1`:

| Aspect            | NRO (homebrew)                     | NSO                                       |
|-------------------|------------------------------------|-------------------------------------------|
| Launched by       | hbloader                           | `pm` (process manager), at process creation |
| `x0` at entry     | ptr to homebrew env context        | `0`                                       |
| `x1` at entry     | `-1` (`UINT64_MAX` sentinel)        | main-thread handle                        |
| Capabilities from | inherited from hbloader's process + env context | the process **NPDM** (ACID/ACI0) given to `pm` |
| File wrapper      | NRO header + asset blob (icon/NACP) | NSO header + LZ4-compressed segments      |

The crt0 exception check (`cmp x0,#0` / `ccmn x1,#1` / `beq`) routes **both**
NSO and NRO to the normal-launch path — only a real user-mode exception
callback (`x0 != 0 && x1 != -1`) peels off to `__libnx_exception_entry`.

The NSO-vs-NRO fork happens **later**, inside `__libnx_init` (step 5 of §4),
not in the crt0: it inspects `x1` — `-1` means "parse the homebrew env context
out of `x0`", any other value means "`x1` is the main-thread handle, NSO path".
That `ctx`-vs-`NULL` branch belongs to the kind-specific entry crate; it is
kept *out* of the kind-agnostic `nx-rt-core`.

This is also why no separate entry crate is needed per applet type: all six
NSO identities (`Application`, `SystemApplet`, … `None`) share one NSO startup
ABI, so a single `nx-rt-nso` crate owns the `pm`-launch `_start`; the applet
type is a build-time runtime-profile sub-axis, not a new `.crt0`.

## 8. Relationship to `nx-std`

Today **all three pieces come from the C `libnx`**. The final link of an
NRO/NSP is performed by the devkitA64 GCC toolchain (orchestrated by Meson),
and `_start` is `libnx`'s `switch_crt0.s`; `rustc`/Cargo do not drive the link
(see [`rust-libnx-linker-and-targets.md`](rust-libnx-linker-and-targets.md)
§1.1).

The forward-looking design isolates the parts that vary:

- **`.crt0` is a fill-in slot, not fixed content.** `KEEP(*(.crt0))` reserves
  the entry; whichever runtime crate is linked supplies the `.crt0` input
  section. The output kind therefore forks the *crt0*, never the `SECTIONS`
  layout (`rust-libnx-linker-and-targets.md` §4.1).
- **The startup ABI varies by kind.** `_start` differs for the hbloader, the
  `pm` launch path, and kernel launch — so each entry crate owns its own:
  `nx-rt-hbapp` (hbloader `_start`), `nx-rt-nso` (`pm` launch `_start`),
  `nx-rt-kip` (kernel `_start`). `nx-rt-module` is a loadable module and has
  **no `_start`** — it inherits the host process's entry. All hand off to the
  kind-agnostic `nx-rt-core`.
- This forward-looking split — porting `switch.ld` into a target-JSON
  `link-script` that keeps `.crt0` as the slot, then giving each entry crate
  its own per-kind `.crt0` — is currently **on hold** behind the linker
  takeover (the point at which `rustc` drives the final link).

Until that takeover happens, every `nx-std` artifact runs on the `_start` /
`.crt0` / MOD0 described above, exactly as C `libnx` produces them.

## 9. References

Internal:

- [`rust-libnx-linker-and-targets.md`](rust-libnx-linker-and-targets.md) — the
  linker-options / output-kind design this document underpins
- [`libnx_overrides.md`](libnx_overrides.md) — link-time symbol override mechanism
- [`build_system.md`](build_system.md) — hybrid Meson + Cargo build
- [`subprojects/libnx/src/nx/switch_crt0.s`](../subprojects/libnx/src/nx/switch_crt0.s) — `_start`, `.crt0`, `__nx_mod0`
- [`subprojects/libnx/src/nx/switch.ld`](../subprojects/libnx/src/nx/switch.ld) — the linker script
- [`subprojects/libnx/src/nx/switch.specs`](../subprojects/libnx/src/nx/switch.specs) — linker spec (`-T`, `--gc-sections`, …)

External:

- [`libnx` `switch_crt0.s`](https://github.com/switchbrew/libnx/blob/master/nx/source/runtime/switch_crt0.s)
- [Switchbrew Wiki — NSO / homebrew ABI](https://switchbrew.org/wiki/NSO)
- [Switchbrew Wiki — Homebrew ABI (NRO loader)](https://switchbrew.org/wiki/Homebrew_ABI)
