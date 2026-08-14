---
name: "crates-rt"
description: "One runtime entry crate per launch path, with the launch path resolved by which crate the code lives in rather than by a runtime check, and the path-agnostic rule binding every layer below. Load when porting a function that branches on how the process was launched, deciding which nx-rt-* crate an entry point belongs to, reaching for envIsNso from outside the runtime, or looking for where the command line is stored"
type: "arch"
scope: "global"
---

# Runtime Launch Paths

## Table of Contents

1. [One Crate Per Launch Path](#1-one-crate-per-launch-path)
2. [The Launch Path Is Answered by Placement](#2-the-launch-path-is-answered-by-placement)
3. [Path-Agnostic Work Belongs to the Core](#3-path-agnostic-work-belongs-to-the-core)
4. [Layers Below the Runtime Never Ask](#4-layers-below-the-runtime-never-ask)
5. [Checklist](#checklist)

---

## 1. One Crate Per Launch Path

The runtime is one entry crate per **launch path** — how the process is brought into existence, and the
startup ABI it therefore wakes up on — over a path-agnostic `nx-rt-core`:

| Entry crate    | Launch path                                                    |
|----------------|----------------------------------------------------------------|
| `nx-rt-hbapp`  | homebrew loader handoff                                        |
| `nx-rt-nso`    | `pm` process launch                                            |
| `nx-rt-kip`    | kernel launch                                                  |
| `nx-rt-module` | `ro` dynamic load into a host process — no `_start` of its own |

**The axis is the launch path, not the output format.** Two of these four crates produce an NRO:
`nx-rt-hbapp` builds the homebrew application the loader hands control to, and `nx-rt-module` builds a
relocatable NRO, registered through an NRR and loaded by `ro` into a process that is already running. A
container name therefore does not identify a crate, while the way the process is entered does — and it is
the startup ABI that genuinely varies from one member to the next, which is why each carries its own
`.crt0`. What that `.crt0` contains, and how the link pipeline assembles it, are design questions answered
in [`docs/rustc-link-pipeline.md`](../rustc-link-pipeline.md) and
[`docs/rust-libnx-linker-and-targets.md`](../rust-libnx-linker-and-targets.md). This document does not
restate them.

What the rules below stand on is the one consequence: **a binary links exactly one entry crate, and that
dependency is the launch path.** By the time any code in it runs, the question "how was this process
launched?" has been answered by the build, so nothing at run time may ask it again.

---

## 2. The Launch Path Is Answered by Placement

**An entry crate does not check how its own process was launched.** The check has one possible outcome, and
writing it invites a reader to believe otherwise.

Upstream C keeps one translation unit for every launch path and branches at run time, so a port arrives
carrying that branch. Drop it, and say in a doc comment why it is gone.

```rust
// ❌ Bad — in the homebrew-loader entry crate this is always false, so the early return is dead
// code that reads as a live case; the next reader adds a `pm`-launch branch beside it and it
// never runs either.
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
/// every launch path there. Here the guard is gone: this crate is only ever entered through the
/// homebrew loader handoff, so the branch could only ever go one way.
pub fn record_program_name() {
    let Some(program) = argv::args().next() else {
        return;
    };
    diag::set_program_name(&program);
}
```

The same applies to an entry point a launch path does not have. A `pm`-launched process is not started from
a file path, so its runtime has no directory to derive from one — it omits the symbol rather than defining
one that returns early.

---

## 3. Path-Agnostic Work Belongs to the Core

Placement answers the launch-path question only if it is asked once. A function whose behaviour does not
depend on the launch path belongs to `nx-rt-core`, and the entry crates re-export it; copying it into each
of them puts the same rule in several places, where the copies drift and none of them is authoritative.

The test is whether the function reads a launch-specific fact at all:

| The function…                                               | Lives in                               |
|-------------------------------------------------------------|----------------------------------------|
| Reads the loader's command line, or a per-path config block | That launch path's entry crate         |
| Sequences startup steps every launch path performs          | `nx-rt-core`                           |
| Differs only in a constant the launch path supplies         | `nx-rt-core`, taking it as a parameter |

The last row is the one worth stating: a difference small enough to pass as an argument is not a reason
to fork a function across entry crates.

---

## 4. Layers Below the Runtime Never Ask

The runtime knows the launch path. **Nothing below it may ask.** Device, filesystem, synchronization and
service crates must not branch on how the process was launched, and must not depend on an `nx-rt-*` crate.

A function that needs the answer is in the wrong crate. Move it up to the runtime rather than teaching the
layer below to ask.

The command line is not one of those answers. Each entry crate reads it from the source its own launch path
provides and installs it in `nx-sys-args`, which holds it the way `std::sys::args` does: below every caller.
A crate that wants the arguments calls `nx_sys_args::args()` and needs nothing from the runtime.

```rust
// ❌ Bad — a device crate reaching through the C ABI twice over: for the launch path, which
// only the runtime knows, and for arguments `nx_sys_args::args()` would have handed it.
// It builds either way, so nothing catches that this reads libnx's state rather than the
// workspace's whenever the runtime override happens to be off.
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
// globals and the launch-path check both disappear and the dependency is one Cargo can see.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_set_report_name() {
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

- [ ] No entry crate branches on its own launch path; a dropped upstream guard is explained in a doc comment
- [ ] An entry point a launch path does not have is omitted from that crate, not defined as an early return
- [ ] A function that reads no launch-specific fact lives in `nx-rt-core` and is re-exported, not copied per crate
- [ ] A per-path difference small enough to pass as an argument is a parameter, not a forked function
- [ ] No crate below the runtime branches on how the process was launched
- [ ] A crate that needs the command line calls `nx_sys_args::args()`, not an entry crate's `__system_argv`
- [ ] No crate below the runtime depends on an `nx-rt-*` crate, in its manifest or through an `extern "C"` declaration
- [ ] A workspace symbol is reached through its Rust API, never through the `__nx_*` name the linker exports

## References

- [rust-ffi](rust-ffi.md) - Related: the `__nx_<aspect>__*` symbols this document forbids as an intra-workspace interface
- [meson-linker-script](meson-linker-script.md) - Related: the override scripts that alias one upstream symbol to exactly one crate
