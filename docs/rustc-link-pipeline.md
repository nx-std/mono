# The `rustc`-Driven Link Pipeline

This document describes the **opt-in `rustc`-driven link pipeline** — a second
way to produce a Switch executable, in which `rustc`/Cargo (rather than the
devkitA64 GCC toolchain) drives the final link and owns the section layout. It
documents the new compilation targets that pipeline introduces: the custom
target specification, the embedded linker script, and the per-kind `.crt0`.

> **Status**: partially implemented. The custom target JSON
> (`aarch64-nintendo-horizon.json`) with its embedded linker script exists, and
> the per-kind `.crt0` sections described below exist in the entry crates
> (`nx-rt-nro`, `nx-rt-nso`, `nx-rt-kip`), feature-gated behind `rt-link`. The
> build orchestration that selects the pipeline per build is future work.

The `rustc` link is **additive and opt-in**, not a cutover. It does not replace
the GCC pipeline — both coexist.

## Table of Contents

1. [The Two Pipelines](#1-the-two-pipelines)
2. [Why a Custom Target Is Required](#2-why-a-custom-target-is-required)
3. [The Custom Target Specification](#3-the-custom-target-specification)
4. [The Embedded Linker Script](#4-the-embedded-linker-script)
5. [The Per-Kind `.crt0` and Feature Gating](#5-the-per-kind-crt0-and-feature-gating)
6. [Selecting a Pipeline](#6-selecting-a-pipeline)
7. [Keeping the Two Layouts in Sync](#7-keeping-the-two-layouts-in-sync)
8. [References](#8-references)

---

## 1. The Two Pipelines

The project builds Switch executables two ways. They share all Rust source —
`nx-rt-core`, the entry crates, every service crate — and diverge only in who
drives the final link.

| Aspect            | GCC pipeline (default, today)                         | `rustc` pipeline (opt-in, new)                              |
|-------------------|-------------------------------------------------------|-------------------------------------------------------------|
| Link driver       | devkitA64 GCC, orchestrated by Meson                  | `rustc` / Cargo                                             |
| Compilation target| built-in tier-3 `aarch64-nintendo-switch-freestanding`| custom target JSON (this document)                          |
| Linker script     | C `libnx` `switch.ld`, passed via `-T`                | embedded in the target JSON's `link-script` field           |
| `_start` / `.crt0`| `libnx`'s `switch_crt0.s`                             | per-kind `.crt0` from the entry crate (`rt-link`-gated)     |
| Rust artifacts    | `rlib` inputs; `nx-std` is the override `staticlib`   | the entry-crate binary is linked directly                   |
| Output kinds      | homebrew NRO                                          | NRO, NSO, KIP, module                                       |

**The GCC pipeline is unchanged.** It remains the default and is exercised by
every existing `just` target. The `rustc` pipeline is selected explicitly
(see [§6](#6-selecting-a-pipeline)) and is the only path that consumes the
targets documented here.

The leverage that makes coexistence cheap: the two pipelines diverge along
exactly **three** axes — the compilation target, the linker-script delivery,
and the `.crt0`. Everything above the link layer is shared, so a single Rust
codebase feeds both.

## 2. Why a Custom Target Is Required

The built-in tier-3 `aarch64-nintendo-switch-freestanding` target *already
embeds* a Switch linker script: its target spec sets `link_script` via
`include_str!`. Because the built-in target passes that embedded script to the
linker as `-T`, supplying an additional `-C link-arg=-Tswitch.ld` does **not**
cleanly override it — the linker receives **two** `SECTIONS` scripts, with
order-dependent results.

Cargo also offers **no propagating mechanism** for a library crate to hand a
`SECTIONS` script to a downstream binary: `cargo:rustc-link-arg` emitted by a
dependency's `build.rs` does not reach the final-binary crate. A linker script
that must apply to every binary therefore has to ride in the *target spec*.

So owning the layout means owning the whole target: a **custom target JSON**
with its own `link-script`, not the built-in triple plus a `-T` override.

## 3. The Custom Target Specification

The custom target JSON is the `rustc` pipeline's compilation target. It is
**not** wired as the default in `.cargo/config.toml` — the GCC pipeline keeps
the built-in triple — and is instead selected per build (see
[§6](#6-selecting-a-pipeline)).

It owns the platform-stable, flag-shaped linker options *and* the section
layout:

| Field                              | Value      | Purpose                                                        |
|------------------------------------|------------|----------------------------------------------------------------|
| `arch`                             | `aarch64`  | Target architecture                                            |
| `os`                               | `horizon`  | Horizon OS                                                     |
| `relocation-model`                 | `pic`      | Position-independent code — NRO/NSO/KIP are all PIE ELFs       |
| `position-independent-executables` | `true`     | Emit a PIE                                                     |
| `dynamic-linking`                  | `true`     | `.dynamic` / `.dynsym` for relocation processing and modules   |
| `relro-level`                      | (per ELF)  | RELRO segment ordering                                         |
| `linker`                           | `rust-lld` | Use the bundled LLD                                            |
| `linker-flavor`                    | GNU LLD    | `rustc` drives LLD directly, no C compiler driver              |
| entry symbol                       | `_start`   | The `.crt0` fill-in slot (see [§5](#5-the-per-kind-crt0-and-feature-gating)) |
| `link-script`                      | (inline)   | The full `PHDRS` / `SECTIONS` layout — see [§4](#4-the-embedded-linker-script) |

One target spec serves **every** output kind. NRO and NSO are byte-identical
ELFs; `elf2kip` also consumes the standard `PT_LOAD` segments; a dynamically
loadable module differs only in exported symbol visibility, not in layout. The
kind forks the `.crt0`, never the section layout.

### 3.1 One Target, Not One Per Kind

The single kind-agnostic target spec is deliberate. It is worth stating why a
*per-kind* set of JSONs — one each for NRO, NSO, KIP, and module — would be the
wrong design.

Every field the target spec holds is **kind-invariant**:

- `arch`, `os`, `relocation-model`, the PIE / `dynamic-linking` flags, `linker`,
  and `linker-flavor` are platform constants — identical for every kind.
- `link-script` is the `switch.ld`-derived layout, itself **kind-agnostic**
  (see [§4](#4-the-embedded-linker-script)): NRO and NSO are byte-identical
  ELFs, `elf2kip` post-processes the same `PT_LOAD` segments, and a module
  differs only in exported-symbol visibility — none of which is a section-layout
  difference.

Four per-kind JSONs would therefore be **byte-identical copies of each other**.
That is not separation of concerns; it is duplication — and worse than the
`switch.ld` case, because it multiplies the embedded layout into four copies,
each of which `just check-target-json` (see
[§7](#7-keeping-the-two-layouts-in-sync)) would have to keep in sync. A per-kind
split *reintroduces* the very DRY hazard [§7](#7-keeping-the-two-layouts-in-sync)
exists to remove.

The only thing that genuinely varies per kind is the **startup ABI** — hbloader
vs `pm`-launch vs kernel-launch `_start`. That is deliberately **not** a
target-spec field: it is the `.crt0` input section, forked per entry crate and
gated by `rt-link` (see [§5](#5-the-per-kind-crt0-and-feature-gating)). Kind
variation rides in the *crate being linked*, not the target — one target plus N
entry crates.

A separate target would be justified only if a kind needed a different value for
an actual target-spec field — `arch`, `os`, the relocation model, or the section
layout. No kind does. Even if one ever did, that would call for *one* extra
target for that single kind, never a blanket one-per-kind split where most files
are duplicates.

## 4. The Embedded Linker Script

`switch.ld` splits into two categories of content:

- **Genuine options** — `OUTPUT_ARCH(aarch64)`, `ENTRY(_start)` — expressible
  as target-spec fields.
- **A memory/section layout** — the `PHDRS` + `SECTIONS` body: RELRO ordering,
  the `PROVIDE_HIDDEN` symbols (`__tls_start`, `__bss_start__`, …), the
  `KEEP(*(.crt0))` input, the `.main.tls` reservation. This is **not**
  reducible to flags.

The layout lives in the custom target JSON's `link-script` field as an inline
script. It is **kind-agnostic** — one script shared by NRO, NSO, KIP, and
module — and it reserves `.crt0` as a fill-in slot:

```ld
KEEP(*(.crt0))
```

The script reserves the entry section; whichever entry crate is linked supplies
the `.crt0` input that fills it. With one shared layout, NRO and NSO ELFs come
out byte-identical except for that `.crt0`.

## 5. The Per-Kind `.crt0` and Feature Gating

Each entry crate supplies its own `.crt0` input section implementing its
startup ABI, then hands off to `nx-rt-core`'s kind-agnostic init:

| Entry crate    | `.crt0` startup ABI                       |
|----------------|-------------------------------------------|
| `nx-rt-nro`    | hbloader `_start`                         |
| `nx-rt-nso`    | `pm` process-launch `_start`              |
| `nx-rt-kip`    | kernel-launch `_start`                    |
| `nx-rt-module` | none — a module has no own `_start`       |

**The `.crt0` MUST be feature-gated.** Because the GCC pipeline still links
`libnx`'s `switch_crt0.s`, an entry crate that *unconditionally* emitted a
`.crt0` with `_start` would produce a **duplicate-`_start` link error** on the
GCC path. The `.crt0` is therefore gated behind a Cargo feature — `rt-link` —
that is enabled **only** on the `rustc` pipeline:

```rust
#[cfg(feature = "rt-link")]
core::arch::global_asm!(include_str!("crt0.s"));
```

| Pipeline | `rt-link` | Source of `_start`                  |
|----------|-----------|-------------------------------------|
| GCC      | off       | `libnx` `switch_crt0.s`             |
| `rustc`  | on        | the entry crate's gated `.crt0`     |

The gate guarantees **exactly one `_start`** in every configuration. This is
the single sharpest hazard of running the two pipelines in parallel; the
feature gate is what makes the parallel model sound.

## 6. Selecting a Pipeline

The GCC pipeline is the default — no opt-in needed, and `.cargo/config.toml`'s
default `target` is left pointing at the built-in triple. The `rustc` pipeline
is selected explicitly:

- **Cargo target**: pass `--target <path-to-custom>.json` (with `build-std`, as
  the project already does for the freestanding target).
- **`.crt0` gate**: enable the entry crate's `rt-link` feature.
- **Build orchestration**: a Meson option chooses which pipeline a given build
  runs; the `cargo nx` build tool wires the entry-crate dependency, the
  `rt-link` feature, and the post-processor (`elf2nro` / `elf2nso` / `elf2kip`)
  consistently from the declared package kind.

Because the selection is per build, both pipelines remain runnable from the
same checkout at all times.

## 7. Keeping the Two Layouts in Sync

Running both pipelines means the `PHDRS`/`SECTIONS` layout exists in two
places: `switch.ld` (consumed by the GCC link) and the custom target JSON's
`link-script` (consumed by the `rustc` link). That is duplicated knowledge — a
DRY hazard: an edit to one that misses the other yields two pipelines that
silently produce different ELFs.

Mitigation: treat **`switch.ld` as the single source of truth** and embed it
*verbatim* into the JSON's `link-script` field, rather than hand-porting the
layout into a second, independently-maintained copy. The two layouts are then
the same knowledge with one authoritative representation.

`just check-target-json` enforces this: it re-embeds `switch.ld` into the
JSON in memory and fails if the result differs from the committed file, so the
embedded copy cannot silently rot. There is no regeneration recipe — `switch.ld`
is a vendored `libnx` file that effectively never changes; in the rare event of
drift, the check's own failure message prints the one-line `jq` command that
re-embeds it.

## 8. References

Internal:

- [`docs/rust-libnx-linker-and-targets.md`](rust-libnx-linker-and-targets.md) —
  the four-axes design and the linker-option analysis behind the custom target
- [`docs/crt0-and-mod0.md`](crt0-and-mod0.md) — startup mechanics: `_start`,
  `.crt0`, MOD0
- [`docs/build_system.md`](build_system.md) — the current hybrid Meson + Cargo
  build (the GCC pipeline)
- [`docs/libnx_overrides.md`](libnx_overrides.md) — the link-time symbol
  override mechanism the GCC pipeline relies on
- [`subprojects/libnx/src/nx/switch.ld`](../subprojects/libnx/src/nx/switch.ld) —
  the linker script in use today, and the source the embedded `link-script` is
  derived from

External:

- [`aarch64-nintendo-switch-freestanding` — rustc book](https://doc.rust-lang.org/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
- [`TargetOptions` (`link_script` field) — rustc_target docs](https://doc.rust-lang.org/stable/nightly-rustc/rustc_target/spec/struct.TargetOptions.html)
- [Custom targets — rustc book](https://doc.rust-lang.org/rustc/targets/custom.html)
- [Allow adding a linker script in build.rs — cargo#7984](https://github.com/rust-lang/cargo/issues/7984)
- [Including a linker script with a built-in target — rust#72034](https://github.com/rust-lang/rust/issues/72034)
