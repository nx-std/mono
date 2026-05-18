# Rust libnx: Linker Options and Output Targets

This document collects the context and a proposed structure for evolving `nx-std` from
"Rust crates that override individual `libnx` symbols" toward shipping a **complete Rust
`libnx`** that can produce every kind of Switch executable. It answers two related
questions:

1. How should linker options be described for Rust once Rust drives the link?
2. How should the foundations be structured so a single codebase can target homebrew
   apps, sysmodules, dynamically loadable modules, and installable packages?

> **Status**: forward-looking design. None of the crates or target files described in the
> *Proposed Solution* exist yet; the *Context* sections describe the project and platform
> as they are today.

## Table of Contents

1. [Context](#1-context)
2. [Describing Linker Options for Rust](#2-describing-linker-options-for-rust)
3. [The Four Axes of Variation](#3-the-four-axes-of-variation)
4. [Proposed Solution](#4-proposed-solution)
5. [Design Rationale](#5-design-rationale)
6. [Open Questions](#6-open-questions)
7. [References](#7-references)

---

## 1. Context

### 1.1 Where the project is today

- The build is a **hybrid Meson + Cargo** system. Individual Rust crates compile to
  `rlib`; only `nx-std` produces a staticlib (`libnx_std.a`).
- The final link of an NRO/NSP is performed by the **devkitA64 GCC toolchain**,
  orchestrated by Meson, against the C `libnx` in `subprojects/libnx/`.
- The linker script in use is the C `libnx` script,
  `subprojects/libnx/src/nx/switch.ld`, passed to the linker via `-T` in Meson link args.
- The Rust default target is `aarch64-nintendo-switch-freestanding` (`.cargo/config.toml`),
  built with `build-std` for `core`/`compiler_builtins`/`alloc` and `panic = "abort"`.
- `cargo-nx` is used today only to **deploy** built NROs to hardware (the `/code-deploy`
  workflow), not to build them.

The consequence worth calling out: today the Rust target's *own* linker behavior is
effectively bypassed — `rustc` never drives the final link, so the Rust artifacts are just
`.a`/`.rlib` inputs to a C-toolchain link. "How should linker options be described for
Rust" has therefore never had to be answered. It becomes a live question the moment Rust
drives the link — i.e. when a Rust binary crate produces an NRO directly.

### 1.2 `switch.ld` is a linker *script*, not linker *options*

The content of `switch.ld` splits into two categories:

- **Genuine options** — `OUTPUT_ARCH(aarch64)`, `ENTRY(_start)`. These *are* expressible as
  flags or target-spec fields.
- **A memory/section layout** — the `PHDRS` + `SECTIONS` body: the RELRO ordering, the
  `PROVIDE_HIDDEN` symbols (`__tls_start`, `__bss_start__`, …), the `KEEP(*(.crt0))`
  input, the `.main.tls` reservation. This is **not reducible to flags**. No combination of
  Cargo `rustflags`, `[profile]`, or `[target]` keys can produce it. It must remain a
  script file fed to the linker via `-T`.

So the real question is not "how do we translate the script" — it is "how does a Rust
build *deliver* that script to the linker." Cargo has no native linker-script concept; the
closest primitive is `cargo:rustc-link-arg`.

### 1.3 Switch executable formats and the output kinds

| Format    | What it is                                                          | Produced by                       |
|-----------|---------------------------------------------------------------------|------------------------------------|
| **NRO**   | Relocatable homebrew object loaded by the Homebrew Loader (hbmenu)   | `elf2nro`                          |
| **NSO**   | Object the OS process manager loads for real titles / exefs modules | `elf2nso`                          |
| **KIP**   | Kernel Initial Process; sysmodule loaded early, caps in KIP header  | `elf2kip` (+ caps JSON)            |
| **NRR**   | Registration blob that authorizes an NRO's hash for dynamic loading | `linkle` / tooling                 |
| **NPDM**  | Process metadata: program ID, kernel capabilities, service ACLs     | `npdmtool` (from JSON)             |
| **NACP**  | Application control property: name, author, version                 | `nacptool`                         |
| **PFS0/NSP** | Container bundling a `main` NSO + `main.npdm` + RomFS/logo        | `build_pfs0` / `linkle`            |

The output **kinds** the project cares about map onto these as follows:

| Output kind                 | ELF post-processing                          | Extra metadata        |
|-----------------------------|-----------------------------------------------|-----------------------|
| Homebrew app                | `elf2nro` (+ optional icon / NACP / RomFS)    | NACP                  |
| Sysmodule (installed)       | `elf2nso` → exefs → PFS0/NSP                   | NPDM                  |
| Sysmodule (boot-time)       | `elf2kip`                                      | KIP capability JSON   |
| Dynamically loadable module | `elf2nro` of a module ELF + NRR registration  | NRR                   |
| NSP                         | container — wraps a sysmodule or an app        | NPDM (+ NACP)         |

### 1.4 How libnx + devkitPro handle this today

- C `libnx` ships a **single** `switch_crt0.s` and **detects the hbl environment at
  runtime**; sysmodules customize behavior through **weak symbols** (`__appInit`,
  `__nx_applet_type`, `__libnx_initheap`, …).
- The **same** `switch.ld` serves NRO and NSO — the ELF is identical; only the
  post-processor (`elf2nro` vs `elf2nso`) and the metadata differ.
- A boot-time sysmodule is produced with `elf2kip` plus a JSON describing its kernel
  capabilities.

### 1.5 How cargo-nx handles this today

- `cargo-nx` exposes three package types — `lib`, `nro`, `nsp` — with `nro` the default.
  Configuration lives in `[package.metadata.nx]` in `Cargo.toml`.
- An `nsp` (sysmodule exefs) package **requires** an NPDM, supplied inline as `npdm` or as
  an external file via `npdm_json`.
- It builds against `aarch64-nintendo-switch-freestanding` by default, or a custom target
  JSON if provided.
- Limitation worth noting: runtime/ABI knowledge **and** packaging logic both live inside
  the tool. Adding a new kind means threading new logic through `cargo-nx` itself, rather
  than adding it at the edge.

---

## 2. Describing Linker Options for Rust

### 2.1 The three delivery mechanisms

There are exactly three ways a Rust build can hand a linker script to the linker:

1. **Target spec `link-script` field.** A `rustc` target specification (built-in or a
   custom JSON) can embed the script *inline*. This is the **only** mechanism that applies
   automatically to every binary built for the target, with no per-crate opt-in.
2. **Crate asset + `build.rs`.** The crate carries the `.ld` file, and its `build.rs`
   copies it into `OUT_DIR` and emits `cargo:rustc-link-search` so the linker can find it.
   This is the `cortex-m-rt` model (`link.x`).
3. **`.cargo/config.toml` `rustflags`.** `-C link-arg=-T….ld` on the target. Simplest, but
   it is a per-workspace setting — a published crate cannot carry it.

### 2.2 Constraint: a `build.rs` cannot ship a `-T` to consumers

A subtle but decisive limitation: `cargo:rustc-link-arg` emitted by a **dependency's**
build script does **not** propagate to the final binary crate (a different crate). Only
`cargo:rustc-link-search` and `cargo:rustc-link-lib` propagate down the dependency graph.

Therefore mechanism 2 makes the script *findable* but the final-binary crate must still
pass `-T….ld` itself (conventionally via `.cargo/config.toml`). This is a known Cargo gap;
see [cargo#7984](https://github.com/rust-lang/cargo/issues/7984) and
[rust#72034](https://github.com/rust-lang/rust/issues/72034) — the latter (the Sony PSP
target) was ultimately resolved by adding a *built-in target with an embedded
`link_script`*, the same path the Switch target took.

### 2.3 The built-in target already embeds a Switch linker script

The tier-3 `aarch64-nintendo-switch-freestanding` target spec sets `link_script` via
`include_str!` of a Switch linker script, and also sets `linker = "rust-lld"`,
`linker_flavor = Gnu(Cc::No, Lld::Yes)`, `position_independent_executables = true`,
`dynamic_linking = true`, `relro_level = Off`, and `os = Horizon`.

Because the built-in target already passes its embedded `link_script` as `-T`, supplying
an additional `-C link-arg=-Tswitch.ld` does **not** cleanly "override" it — the linker
receives **two** `SECTIONS` scripts, with order-dependent results. To genuinely own the
layout, use a custom target JSON with your own `link-script` (or one that omits it and you
pass `-T` deliberately), rather than fighting the built-in one.

### 2.4 Recommended split

Separate genuine flags from the layout, by owner:

- **Custom target JSON** owns the platform-stable, flag-shaped options *and* the
  `link-script` itself: `relocation-model: pic`, `position-independent-executables`,
  `dynamic-linking`, `relro-level`, `--build-id`, `os: horizon`, the entry symbol — and the
  `SECTIONS` layout. The layout belongs here, not in a runtime crate's `build.rs`, because
  Cargo offers **no propagating mechanism** for a library to hand a `SECTIONS` script to a
  downstream binary.
- The `.crt0` input section is deliberately **not** owned by the script's content — see
  [§4.1](#41-foundation-1--a-kind-agnostic-compilelink-layer).

The C `libnx` only gets away with a loose `switch.ld` file because Meson explicitly wires
`-T` into every link; a Rust-driven build has no equivalent implicit step, so the script
must ride in the target spec.

---

## 3. The Four Axes of Variation

The output kind ("NRO vs sysmodule vs module vs NSP") looks pervasive, but it actually
varies along only **four axes**. The structuring discipline is to give each axis exactly
one owner (Single Responsibility) so the kind does not leak into every layer.

| Axis                                   | Varies by kind? | Owner                          |
|-----------------------------------------|-----------------|--------------------------------|
| Compile + link layout                   | **No** — NRO/NSO/KIP all come from the same PIE ELF | Target spec + one linker script |
| Startup ABI (`_start`, env bring-up)    | **Yes** — hbloader vs `pm` vs kernel hand off differently | Per-kind runtime crate |
| Runtime profile (heap source, services) | **Yes**         | Per-kind runtime crate         |
| Packaging + metadata (NACP/NPDM/NRR)     | **Yes**         | Build tool (`cargo-nx` + `linkle`) |

The leverage: **two of the four axes do not vary by kind.** If the compile/link layer is
kept kind-agnostic, the kind collapses to "which runtime crate + which packaging recipe" —
which is exactly the surface a build tool should drive.

---

## 4. Proposed Solution

### 4.1 Foundation 1 — a kind-agnostic compile/link layer

One target spec and one linker script, shared by every kind:

- NRO and NSO are byte-identical ELFs — only the post-processor differs. `elf2kip` also
  consumes the standard ELF's `PT_LOAD` segments.
- A dynamically loadable module needs exported `.dynsym` entries, but that is controlled by
  **symbol visibility** (a version script / `--export-dynamic`), not by a different
  `SECTIONS` layout.

The script already does the right thing: `KEEP(*(.crt0))` at the entry is a **fill-in
slot**. The script reserves the entry; whichever runtime crate is linked supplies the
`.crt0` input section. The kind therefore forks the **crt0**, never the layout.

### 4.2 Foundation 2 — the runtime crate set

The startup ABI and runtime profile are the axes that genuinely vary. They are isolated
into a small family of runtime crates (names proposed):

```
nx-rt-core       env-agnostic runtime: .dynamic relocation processing, .bss clear,
                 TLS bring-up, init_array / fini_array, panic glue.
                 No _start, no environment assumptions.

nx-rt-nro        homebrew app: _start for hbloader's ABI, parse hbl config entries
                 (heap override, argv, applet type, stdio). -> nx-rt-core
nx-rt-sysmodule  exefs/NSP sysmodule: _start for pm's launch ABI, own heap via
                 svcSetHeapSize, minimal service profile.            -> nx-rt-core
nx-rt-module     dynamically loadable NRO: no _start, exported dynsym surface,
                 init/fini, NRR-registerable.                        -> nx-rt-core
(future)
nx-rt-kip        raw KIP sysmodule: kernel-launch ABI, caps from KIP header.
```

A final binary depends on **exactly one** `nx-rt-*` crate — and that dependency choice
*is* the output kind at the Rust level. Each `nx-rt-*` crate's job is narrow: supply the
`.crt0` section, implement the kind's entry ABI, and hand off to `nx-rt-core`.

### 4.3 Foundation 3 — declarative, composed metadata

NACP / NPDM / NRR / KIP capability descriptors are **data**, not code, and stay
declarative (in `[package.metadata.nx]` and/or sidecar JSON).

There is, however, a real coupling: an NPDM's **kernel capabilities (allowed SVCs)** and
**service access control** must match what the runtime *and* the app actually use. The
NPDM must therefore not be hand-maintained in isolation. The proposal:

- Each `nx-rt-sysmodule` / `nx-rt-kip` crate exposes a **machine-readable capability
  fragment** — the minimum SVCs and services *its own startup* requires.
- The build tool **merges** that runtime fragment with the app-declared capabilities to
  emit the final NPDM.

The runtime crate owns "what my startup needs", the app owns its own requirements, and
nobody hand-writes the union.

### 4.4 The build tool as thin orchestration

With the three foundations in place, the build tool (an extended `cargo-nx`, or Meson)
carries **no runtime or ABI logic**. Its job becomes pure orchestration:

1. Read `[package.metadata.nx]` and resolve the package type.
2. Map the package type 1:1 to a runtime crate dependency (`nro` → `nx-rt-nro`,
   `nsp` → `nx-rt-sysmodule`, `lib` → `nx-rt-module`); inject or verify that dependency.
3. Build the kind-agnostic ELF.
4. Pick the post-processor — reuse [`linkle`](https://lib.rs/crates/linkle) /
   `switch-tools` (`elf2nro`/`elf2nso`/`elf2kip`/`build_pfs0`) rather than reimplementing
   them.
5. Merge the runtime capability fragment with app-declared capabilities → NPDM; assemble
   the NRO / NSP / NRR as required.

Adding a new kind then touches **a crate and a packaging branch** — it never threads new
logic through the tool. This layering is also driver-agnostic: it works whether the build
is ultimately driven by Meson (as today) or by an extended `cargo-nx`.

### 4.5 End-to-end layering

```
Platform layer  (shared, kind-agnostic)
  custom target JSON  ──  one linker script  ──  .crt0 reserved as a fill-in slot

Runtime layer   (the variation point)
  nx-rt-core  +  exactly one of { nx-rt-nro | nx-rt-sysmodule | nx-rt-module | nx-rt-kip }

Application code
  the homebrew / sysmodule / module crate — identical regardless of kind

Packaging layer (orchestration only)
  pick post-processor (linkle / switch-tools)  ──  merge caps → NPDM  ──  assemble NRO/NSP/NRR
```

### 4.6 How a binary selects its kind

A binary crate declares its kind once, in two aligned places:

- A dependency on exactly one `nx-rt-*` crate in `Cargo.toml`.
- A matching `[package.metadata.nx]` package type.

The build tool reads the package type and can inject or verify the `nx-rt-*` dependency,
keeping the two consistent. There is no separate "build for sysmodule" flag threaded
through the codebase — the kind is the dependency.

---

## 5. Design Rationale

The proposed structure follows the workspace principles directly:

- **Type-Driven Design / crates over Cargo features.** The kinds are mutually exclusive — a
  binary is never both an NRO and a sysmodule. Cargo features are *additive* and unify
  across a dependency graph, so mutually-exclusive features are a footgun. Separate
  `nx-rt-*` crates make the illegal "both kinds" state unrepresentable.
- **Inversion of Control.** The binary injects its runtime profile by *choosing a
  dependency*, instead of the runtime detecting its environment at boot (the C `libnx`
  approach with one crt0 + weak symbols).
- **Open/Closed.** A new output kind is a new crate plus a packaging branch, with zero
  edits to existing runtimes or to the build tool's core.
- **Single Responsibility.** Each of the four axes has exactly one owner: the section
  layout in the target spec, the startup ABI and runtime profile in `nx-rt-*`, packaging
  and metadata in the build tool.

---

## 6. Open Questions

- **Built-in target vs custom target JSON.** Stay on the tier-3 triple and accept its
  embedded `link_script`, or ship a custom JSON to own the `SECTIONS` layout outright?
- **Relationship to the override staticlib.** Does `nx-std`'s libnx-symbol-override
  staticlib continue in parallel with a full-Rust `libnx`, or is the full-Rust path a
  separate track? This intersects the incremental-replacement strategy in `AGENTS.md`.
- **KIP scope.** Is boot-time sysmodule delivery (`elf2kip`) in scope, or only installed
  exefs NSP sysmodules?
- **Metadata location and fragment form.** Where do manifests live —
  `[package.metadata.nx]`, sidecar JSON, or both — and how is the `nx-rt-*` capability
  fragment expressed (a `const`, build-script output, or a static data file)?
- **Final-link ownership.** During the transition, when does `rustc`/Cargo take over the
  final link from the devkitA64 GCC toolchain?

---

## 7. References

Internal:

- [`docs/crt0-and-mod0.md`](crt0-and-mod0.md) — startup mechanics: `_start`, `.crt0`, MOD0
- [`docs/build_system.md`](build_system.md) — current hybrid Meson + Cargo build system
- [`docs/libnx_overrides.md`](libnx_overrides.md) — current link-time symbol override mechanism
- [`docs/code/meson-linker-script.md`](code/meson-linker-script.md) — linker script wiring in Meson
- [`subprojects/libnx/src/nx/switch.ld`](../subprojects/libnx/src/nx/switch.ld) — the linker script in use today

External:

- [`aarch64-nintendo-switch-freestanding` — rustc book](https://doc.rust-lang.org/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
- [`TargetOptions` (`link_script` field) — rustc_target docs](https://doc.rust-lang.org/stable/nightly-rustc/rustc_target/spec/struct.TargetOptions.html)
- [Add Nintendo Switch as tier 3 target — rust#88991](https://github.com/rust-lang/rust/pull/88991/)
- [Allow adding a linker script in build.rs — cargo#7984](https://github.com/rust-lang/cargo/issues/7984)
- [Including a linker script with a built-in target — rust#72034](https://github.com/rust-lang/rust/issues/72034)
- [`cortex-m-rt` `build.rs`](https://github.com/rust-embedded/cortex-m-rt/blob/master/build.rs)
- [`cargo-nx` — package types and `[package.metadata.nx]`](https://github.com/aarch64-switch-rs/cargo-nx)
- [`linkle` — Rust NRO/NSO/PFS0/NSP/NACP tooling](https://lib.rs/crates/linkle)
- [`switch-tools` — `elf2nro` / `elf2nso`](https://github.com/switchbrew/switch-tools)
- [`libnx` `switch_crt0.s`](https://github.com/switchbrew/libnx/blob/master/nx/source/runtime/switch_crt0.s)
