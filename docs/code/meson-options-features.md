---
name: "meson-options-features"
description: "The `use_nx_*` feature options: Cargo-shaped declaration, the `use_nx` master switch, the dependency table, and the pull-up/push-down/refuse resolution every consuming `meson.build` runs for itself. Load when adding or removing a `use_nx_*` option, declaring what a feature depends on, reading a feature in a `meson.build`, or changing which features the prebuilt devkitPro archive supports"
type: "arch"
scope: "global"
---

# Meson Feature Options

**MANDATORY for ALL `use_nx_*` options and the `meson.build` files that read them in the workspace**

## Table of Contents

1. [What a Feature Is](#1-what-a-feature-is)
2. [Declaring a Feature](#2-declaring-a-feature)
3. [The Dependency Table](#3-the-dependency-table)
4. [Resolution](#4-resolution)
5. [Availability](#5-availability)
6. [One Resolver; Everything Below Exports Data](#6-one-resolver-everything-below-exports-data)
7. [Checklist](#checklist)

---

## 1. What a Feature Is

A feature is one `use_nx_<name>` Meson option, and enabling it does two things at once: it adds the matching
Cargo feature to `nx-std`'s build, and it links a `*_override.ld` fragment that aliases a family of `libnx`
symbols onto the Rust implementation behind it.

Both halves land together or the build is broken in a way that links cleanly. The C side keeps calling the
symbols it always called; if only some of them were aliased, the Rust half and the C half hold state neither
can see, and the failure surfaces at run time as a call that returns the wrong answer rather than as a missing
symbol.

That is the whole reason this document exists. A feature is not an independent switch — it is one end of a
dependency edge, and the edges are what keep a half-aliased family unconfigurable rather than merely
discouraged.

**The model is Cargo's `[features]`**, deliberately: a feature is asked for by name, and asking for it also
asks for what it depends on. Read an unfamiliar edge the way the equivalent Cargo entry would read.

---

## 2. Declaring a Feature

Feature options follow the `meson.options` shape that [meson-subproject](meson-subproject.md) defines. Three
rules are specific to features:

- **`type : 'feature'`, never `'boolean'`.** The `auto` state is what the master switch resolves; a boolean
  cannot express "not asked either way".
- **`value : 'auto'`** when the feature is ready to be turned on in bulk. A feature that is unfinished or
  known-unstable defaults to `'disabled'` with a trailing comment saying why, and the master switch then
  leaves it alone.
- **`yield : true`**, always. This is what makes the root's value the one that arrives at every consumer, at
  any depth of the subproject tree.

```meson
# ✅ Good — ready for the master switch to decide, so it is left on `auto`
option(
    'use_nx_sys_clock',
    type : 'feature', value : 'auto',
    description : 'Override all libnx functions with nx-sys-clock functions',
    yield : true,
)

# ✅ Good — the trailing comment is what keeps a reader from "fixing" the default
option(
    'use_nx_service_audio',
    type : 'feature', value : 'disabled', # Disable by default, WIP
    description : 'Enable nx-service-audio (Audio service)',
    yield : true,
)

# ❌ Bad — a boolean collapses `auto` into `false`, so the master switch can no longer
# tell "leave this to me" from "the user said no", and every feature it turns on
# becomes indistinguishable from one the user named by hand
option(
    'use_nx_sys_clock',
    type : 'boolean', value : false,
    description : 'Override all libnx functions with nx-sys-clock functions',
    yield : true,
)
```

Declare every option a `meson.build` reads in the `meson.options` beside it, copied verbatim from the
workspace root. A subproject that reads an option it does not declare is relying on a parent to have declared
it, which makes it unbuildable on its own.

---

## 3. The Dependency Table

Each feature and the features it cannot work without go in one table, written as direct edges only:

```meson
# ✅ Good — direct edges, each carrying the reason it exists
nx_features_deps = {
    'sys_clock' : [],
    'service_audio' : ['rt'],
    'audio_out' : ['service_audio', 'sys_fd'], # dispatches through audout, registers with the fd table
    'rt' : [],
}
```

```meson
# ❌ Bad — the closure is written out by hand, so `audio_out` now names `rt` twice.
# When `service_audio` stops depending on `rt`, one of the two goes stale and the
# build keeps enabling a runtime nothing needs
nx_features_deps = {
    'service_audio' : ['rt'],
    'audio_out' : ['service_audio', 'sys_fd', 'rt'],
}
```

**Every entry carries a comment saying what breaks without the dependency**, unless the edge is self-evident
from the two names. The edge is a claim about run-time behaviour, and a reader deciding whether to remove it
needs the claim, not the graph.

**A feature with no dependencies is still an entry.** The table is the list of features as well as the list of
edges; a feature missing from it is a feature nothing resolves.

---

## 4. Resolution

Four rules, applied in order. They are not independent — each exists to stop the previous one from doing
damage.

### 4.1 The Master Switch Decides Every `auto`

`use_nx` is the table's `default` feature set. It turns on every feature still on `auto` and leaves explicit
choices alone.

```meson
# ✅ Good — an `auto` the master switch does not reach stays off
use_nx = get_option('use_nx')
enabled = get_option('use_nx_' + name).enable_auto_if(use_nx.enabled()).enabled()
```

### 4.2 What Was Asked For Stays Separate From What It Resolves To

Keep the raw option values in their own dict and resolve into a second one.

This is the load-bearing rule of the whole resolution. A feature the master switch turned on may be turned off
again by a dependency that is absent; a feature the user named may not. Collapse the two and a dependency edge
can no longer refuse anything, because after the master switch every `auto` looks exactly like an explicit
`enabled`.

**The asked dict is an explicit map, one literal `get_option` per feature.** Grepping an option's name must
find every read of it; a name assembled at run time hides the read from every search.

```meson
# ✅ Good — each read is literal, greppable by the option's full name
nx_features_asked = {
    'sys_clock' : get_option('use_nx_sys_clock'),
    'service_audio' : get_option('use_nx_service_audio'),
}

# ❌ Bad — the option name exists only at setup time, so a rename sweep over
# `use_nx_sys_clock` finds the declaration and misses every read
nx_features_asked = {}
foreach name, _ : nx_features_deps
    nx_features_asked += {name : get_option('use_nx_' + name)}
endforeach
```

### 4.3 Edges Apply in Both Directions

- **Pull up** — an enabled feature enables what it depends on, so `-Duse_nx_audio_out=enabled` produces a
  build that works rather than one that needs two more flags to be told about.
- **Push down** — a feature whose dependency is off is turned off again, rather than left half-aliased.

Neither moves a feature the user named: pull-up only fills an `auto`, push-down only retracts one.

Take the closure by repeating the pass once per table entry. The graph is a DAG, so that reaches a fixed point
whatever order the table is written in.

```meson
# ❌ Bad — one pass, so an edge only resolves when the table happens to list the
# dependency after its dependent. Reordering the table silently changes the build
foreach name, deps : nx_features_deps
    ...
endforeach

# ✅ Good — the table's order stops being load-bearing
foreach _ : range(nx_features_deps.keys().length())
    foreach name, deps : nx_features_deps
        ...
    endforeach
endforeach
```

### 4.4 Two Explicit Choices That Contradict Each Other Are Refused

When a feature the user enabled by name depends on one that is disabled, `error()` out and name both sides and
the consequence. Do not reconcile it silently in either direction, and do not warn: a warning scrolls past on
the way to a build that faults at startup.

```meson
# ❌ Bad — the user asked for two things that cannot both hold, and the build
# says so in a line nobody reads before the linker succeeds
warning('use_nx_audio_out works best with use_nx_service_audio')

# ✅ Good — names both sides, the failure mode, and both ways out
error(
    'use_nx_@0@ depends on use_nx_@1@, which is disabled: the aliases would be emitted '.format(name, dep)
    + 'for one half of the pair and not the other, which links cleanly and fails at '
    + 'run time. Enable use_nx_@1@ or disable use_nx_@0@.'.format(name, dep),
)
```

### 4.5 Every Effect Site Logs Its Feature

Each block that acts on an enabled feature opens with a literal `debug('<feature> feature: enabled')` — the
feature's name written out, not derived in a loop. A configure log then answers "what did this build wire"
without re-running the resolution by hand, and a feature an edge decided is as visible as one the user named.

The repetition is the point: one debug per site is greppable next to the effect it announces, and a loop that
manufactures the lines from the table logs features this file never acts on.

```meson
# ✅ Good — the literal name, at the site whose effect it announces
if nx_features['service_audio']
    debug('service-audio feature: enabled')
    cargo_features += 'service-audio'
    ld_overrides += meson.current_source_dir() / 'overrides' / 'rt_nro_libnx_service_audio.ld'
endif

# ❌ Bad — a loop over the table logs every resolved feature, including the ones
# this file takes no action on, so the log claims effects that never happened
foreach name, enabled : nx_features
    if enabled
        debug(name + ' feature: enabled')
    endif
endforeach
```

---

## 5. Availability

Which features can be wired at all is decided by the choice of `libnx` archive, and it is a **third state**,
distinct from enabled and disabled.

The prebuilt devkitPro archive brings its own runtime and services. A feature that would alias over them has
nothing to attach to, so it is *unavailable* rather than merely disabled, and the difference changes what the
edges do:

- An edge pointing at an **unavailable** feature is already satisfied, by the C archive. Neither pull-up nor
  push-down applies to it, and it is not a contradiction.
- An edge pointing at a **disabled** feature is unmet, and rules 4.3 and 4.4 apply.

```meson
# ❌ Bad — pull-up is not guarded on availability, so enabling a feature the prebuilt
# archive does support drags in one it does not, and the Rust runtime aliases over
# the C runtime that is already there
if features[name] and asked[dep].auto()
    features += {dep : true}
endif

# ✅ Good — an unavailable dependency is the archive's to satisfy, so the edge is skipped
if features[name] and available[dep] and asked[dep].auto()
    features += {dep : true}
endif
```

**Every place an edge is consulted is guarded on availability** — pull-up, push-down, and the refusal in 4.4.
A guard on two of the three is the same bug as a guard on none.

Enabling an unavailable feature by name is refused, for the same reason 4.4 refuses a contradiction: the user
asked for something the archive makes impossible.

---

## 6. One Resolver; Everything Below Exports Data

The full table and its resolution live in exactly one `meson.build`: the crate that assembles the staticlib
(`nx-std`). That file is where both consequences of a feature land — the Cargo feature that compiles the
symbols and the `*_override.ld` fragment that aliases them — so resolving there means one answer selects
both, and they cannot disagree.

**Everything below the resolver exports data and decides nothing.** A crate that owns per-feature override
fragments exports each one unconditionally, as a plain path variable (`<crate>_service_apm_ld_override`,
…) — no feature reads, no conditionals, no `meson.options` mirror. The resolver picks which to link inside
its own per-feature blocks. A fragment path is just a string; exporting it costs nothing when the feature is
off.

```meson
# ✅ Good — the fragment-owning crate exports paths unconditionally
overrides_dir = meson.current_source_dir() / 'overrides'
nx_rt_nro_service_apm_ld_override = overrides_dir / 'rt_nro_libnx_service_apm.ld'

# ❌ Bad — the crate resolves features to build a pre-filtered list, so the
# resolution now exists twice, and the two copies can drift until a fragment is
# linked whose initializer was never compiled
if nx_features['service_apm']
    ld_overrides += overrides_dir / 'rt_nro_libnx_service_apm.ld'
endif
```

Two narrower reads stay outside the resolver, because neither needs the table:

- **A file that only needs to know whether *any* feature is on** derives it from the master switch and the
  explicitly-named features alone: pull-up only enables a feature behind one already enabled, so it cannot
  turn that answer from false to true.
- **A binary subproject gating on a single bottom-of-graph feature** applies the master switch to the one
  option it reads (next section). A feature with dependency edges cannot take this shortcut.

And neither shortcut around resolving works for the resolver itself:

- **Reading `get_option` without resolving** gives the *root's* value, which is `auto` for everything the
  master switch or an edge decided. A consumer that trusts it emits nothing for a feature that is on.
- **Forwarding through `subproject(default_options : ...)`** is rejected for any option the target does not
  itself declare, so every call site grows a per-consumer list, and those lists drift.

### Binary Subprojects Assert the Runtime Kind

The runtime override is kind-specific: `nx_rt_kind` selects which entry crate `nx-std` bundles, and the entry
crate must match the artifact being produced. A binary-producing subproject that bundles NROs therefore
asserts, when the `rt` feature is on, that `nx_rt_kind` is `nro` — and fails configuration otherwise, because
an NRO carrying the pm-launch runtime links cleanly and never boots.

```meson
# ✅ Good — a feature at the bottom of the graph needs no table; the master
# switch is the whole resolution, and the mismatch is refused with both ways out.
# The availability guard is still owed: under the prebuilt archive the runtime
# feature has nothing to gate.
nx_rt_available = not get_option('use_libnx_dkp').enabled()
use_nx = get_option('use_nx')
use_nx_rt = get_option('use_nx_rt').enable_auto_if(use_nx.enabled())
if nx_rt_available and use_nx_rt.enabled() and get_option('nx_rt_kind') != 'nro'
    error(
        'nx-tests bundles NRO artifacts, but nx_rt_kind is \'@0@\': the runtime linked '.format(
            get_option('nx_rt_kind'),
        )
        + 'into them would be the pm-launch one, which never boots under the homebrew '
        + 'loader. Set -Dnx_rt_kind=nro or disable use_nx_rt.',
    )
endif
```

The same shape serves any binary subproject that needs one feature rather than the set: apply the master
switch to the option it reads, and gate. A feature with dependency edges cannot take this shortcut — it needs
the full table, because an edge from a feature the file never reads can still decide the one it does.

---

## Checklist

Before committing a change to a feature option or its resolution, verify:

### Declaration

- [ ] The option is `type : 'feature'` with `yield : true`
- [ ] `value` is `'auto'`, or `'disabled'` with a trailing comment giving the reason
- [ ] Every option the `meson.build` reads is declared in the `meson.options` beside it, verbatim from the root

### Table

- [ ] The feature has an entry, even if its dependency list is empty
- [ ] Only direct edges are listed; no edge restates one reachable through another
- [ ] Each edge carries a comment saying what breaks without it, unless the two names say it

### Resolution

- [ ] `use_nx` resolves every `auto`, and explicit choices survive it
- [ ] What was asked for is kept in a separate dict from what it resolved to
- [ ] The asked dict is an explicit map of literal `get_option('use_nx_<name>')` reads, never a name built by concatenation
- [ ] The closure pass repeats once per table entry, so the table's order is not load-bearing
- [ ] Pull-up fills only an `auto`; push-down retracts only an `auto`
- [ ] A contradiction between two explicit choices calls `error()` naming both sides, never `warning()`
- [ ] Pull-up, push-down, and the refusal are each guarded on availability

### Consumers

- [ ] The table and the resolution exist in exactly one `meson.build`: the crate assembling the staticlib
- [ ] A crate owning per-feature fragments exports each as an unconditional path variable and reads no features
- [ ] A file needing only "is any feature on" derives it from the master switch and explicit names, without the table
- [ ] No consumer reads a resolved feature set out of another subproject, or takes one through `default_options`
- [ ] A binary subproject bundling NROs refuses `nx_rt_kind != 'nro'` when the `rt` feature is on
- [ ] Every block acting on a feature opens with a literal `debug('<feature> feature: enabled')`, never a loop over the table

## References

- [meson-subproject](meson-subproject.md) - Related: `meson.options` shape, section banners, and variable naming
- [meson-linker-script](meson-linker-script.md) - Related: the `*_override.ld` fragments a feature links
- [principle-symmetry](principle-symmetry.md) - Foundation: identical copies of an idea stay identical
