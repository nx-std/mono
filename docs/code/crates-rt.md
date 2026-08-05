---
name: "crates-rt"
description: "One runtime crate per output kind, with the kind resolved by which crate the code lives in rather than by a runtime check, and the kind-agnostic rule binding every layer below. Load when porting a function that branches on output kind, deciding which nx-rt-* crate an entry point belongs to, or reaching for envIsNso or the argv globals from outside the runtime"
type: "arch"
scope: "global"
---

# Runtime Output Kinds

## Table of Contents

1. [One Crate Per Output Kind](#1-one-crate-per-output-kind)
2. [The Kind Is Answered by Placement](#2-the-kind-is-answered-by-placement)
3. [Kind-Agnostic Work Belongs to the Core](#3-kind-agnostic-work-belongs-to-the-core)
4. [Layers Below the Runtime Never Ask](#4-layers-below-the-runtime-never-ask)
5. [Checklist](#checklist)

---

## 1. One Crate Per Output Kind

The runtime is one entry crate per output kind — `nx-rt-nro`, `nx-rt-nso`, `nx-rt-kip`, `nx-rt-module` —
over a kind-agnostic `nx-rt-core`. Why the family is cut that way, which axes vary by kind, and how the
`.crt0` and the link pipeline follow from it are design questions, answered in
[`docs/rustc-link-pipeline.md`](../rustc-link-pipeline.md) and
[`docs/rust-libnx-linker-and-targets.md`](../rust-libnx-linker-and-targets.md). This document does not
restate them.

What the rules below stand on is the one consequence: **a binary links exactly one entry crate, and that
dependency is the output kind.** By the time any code in it runs, the question "which kind is this?" has
been answered by the build, so nothing at run time may ask it again.

---

## 2. The Kind Is Answered by Placement

**An entry crate does not check which kind it is running as.** The check has one possible outcome, and
writing it invites a reader to believe otherwise.

Upstream C keeps one translation unit for every kind and branches at run time, so a port arrives carrying
that branch. Drop it, and say in a doc comment why it is gone.

```rust
// ❌ Bad — in an NRO entry crate this is always false, so the early return is dead code that
// reads as a live case; the next reader adds an NSO branch beside it and it never runs either.
pub fn record_program_name() {
    if env::is_nso() {
        return;
    }

    let Some(program) = argv::args().next() else {
        return;
    };
    diag::set_program_name(&program);
}
```

```rust
// ✅ Good — the guard is gone and the doc says what makes it unnecessary, so nobody restores it.
/// Records the name of the file this process was loaded from, for a crash report to quote.
///
/// Upstream guards this on the process not being an NSO, because one translation unit serves
/// every kind there. Here the guard is gone: this is the NRO entry crate, so the branch could
/// only ever go one way.
pub fn record_program_name() {
    let Some(program) = argv::args().next() else {
        return;
    };
    diag::set_program_name(&program);
}
```

The same applies to an entry point a kind does not have. An NSO is not launched by a path, so an NSO
runtime has no directory to derive from one — it omits the symbol rather than defining one that returns
early.

---

## 3. Kind-Agnostic Work Belongs to the Core

Placement answers the kind question only if it is asked once. A function whose behaviour does not depend
on the kind belongs to `nx-rt-core`, and the entry crates re-export it; copying it into each of them puts
the same rule in several places, where the copies drift and none of them is authoritative.

The test is whether the function reads a kind-specific fact at all:

| The function…                                        | Lives in            |
|------------------------------------------------------|---------------------|
| Reads the loader's command line, or a per-kind config | The kind's entry crate |
| Sequences startup steps that every kind performs      | `nx-rt-core`        |
| Differs only in a constant the kind supplies          | `nx-rt-core`, taking it as a parameter |

The last row is the one worth stating: a difference small enough to pass as an argument is not a reason
to fork a function across entry crates.

---

## 4. Layers Below the Runtime Never Ask

The runtime knows the kind. **Nothing below it may ask.** Device, filesystem, synchronization and service
crates must not branch on the output kind, must not read the runtime's `argv` globals, and must not depend
on an `nx-rt-*` crate.

A function that needs the answer is in the wrong crate. Move it up to the runtime rather than teaching the
layer below to ask.

```rust
// ❌ Bad — a device crate reaching through the C ABI for a fact the runtime holds in Rust.
// It builds either way, so nothing catches that this reads libnx's state rather than the
// runtime's whenever the runtime override happens to be off.
unsafe extern "C" {
    fn envIsNso() -> bool;
    static __system_argc: c_int;
    static __system_argv: *mut *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_set_report_name() {
    // SAFETY: both are runtime globals, initialized before this runs.
    if unsafe { envIsNso() } || unsafe { __system_argc } == 0 {
        return;
    }
    // ...
}
```

```rust
// ✅ Good — the entry point sits in the runtime crate that owns the command line, so the
// globals and the kind check both disappear and the dependency is one Cargo can see.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_set_report_name() {
    diag::record_program_name()
}
```

The `extern "C"` block is the signal. A crate that declares one to reach a workspace symbol has taken a C
ABI round trip to a Rust function it could have called, and has acquired a dependency that appears in no
manifest — so the link order decides which implementation it gets, and a build with the override disabled
reads a second, stale copy of the same state.

`__nx_*` symbols exist for the C callers left behind in the archives being replaced. They are not an
interface between crates in this workspace ([rust-ffi](rust-ffi.md)); between crates, call the Rust API.

---

## Checklist

Before committing a runtime change, verify:

- [ ] No entry crate branches on its own output kind; a dropped upstream guard is explained in a doc comment
- [ ] An entry point a kind does not have is omitted from that kind's crate, not defined as an early return
- [ ] A function that reads no kind-specific fact lives in `nx-rt-core` and is re-exported, not copied per kind
- [ ] A per-kind difference small enough to pass as an argument is a parameter, not a forked function
- [ ] No crate below the runtime branches on the output kind or reads the runtime's `argv` globals
- [ ] No crate below the runtime depends on an `nx-rt-*` crate, in its manifest or through an `extern "C"` declaration
- [ ] A workspace symbol is reached through its Rust API, never through the `__nx_*` name the linker exports

## References

- [rust-ffi](rust-ffi.md) - Related: the `__nx_<aspect>__*` symbols this document forbids as an intra-workspace interface
- [meson-linker-script](meson-linker-script.md) - Related: the override scripts that alias one upstream symbol to exactly one crate
