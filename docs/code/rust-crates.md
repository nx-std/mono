---
name: "rust-crates"
description: "Cargo.toml section ordering, feature flag rules, kebab-case naming. Load when editing Cargo.toml or adding features to a crate"
type: "arch"
scope: "global"
---

# Rust Crate Manifest Patterns

**MANDATORY for ALL `Cargo.toml` files in the nx-std workspace**

## 1. Cargo.toml Section Ordering

Sections MUST appear in this exact order, with no other section mixed between them:

1. `[package]` — crate metadata
2. **Target definitions** — `[lib]`, `[[bin]]`, `[[bench]]`: what this manifest builds
3. `[features]` — feature flags and their dependencies
4. `[dependencies]` — runtime dependencies
5. `[dev-dependencies]` — development and test dependencies
6. `[build-dependencies]` — build-time dependencies, and `[target.'cfg(…)'.build-dependencies]` in the same
   slot, since it is the same section under a gate
7. `[lints.<tool>]` — lint configuration

Every section except `[package]` is optional. Dependencies within each section MUST be alphabetically ordered,
and sections are separated by a blank line.

The order reads as **what the crate is, what it builds, what it needs, and how it is checked**. Target
definitions sit high because they answer "is this a library, a binary, or both" — the question a reader asks
before any dependency matters. Lints sit last because they configure the build rather than compose it, and a
cfg-gated `[lints.rust]` declaring a `check-cfg` belongs beside the gate it names.

```toml
# ✅ Good — a reviewer knows where to look for a new dependency, and two branches adding
# one each conflict on different lines instead of the same one.
[package]
name = "nx-service-timesrv"
version = "0.1.0"
edition = "2024"

[lib]
bench = false

[[bin]]
name = "nx-timesrv-dump"
path = "src/bin/dump.rs"

[features]
default = []

[dependencies]
bitflags = "2.6"
nx-sf = { path = "../nx-sf" }
zerocopy = { version = "0.8", features = ["derive"] }

[dev-dependencies]
static_assertions = "1.1"

[target.'cfg(gen_cmif_tables)'.build-dependencies]
nx-build-cmif = { path = "../nx-build-cmif" }

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ["cfg(gen_cmif_tables)"] }
```

Named dependency groups are allowed when a set of dependencies has shared external update ownership, lockstep
compatibility requirements, or operational tooling rules such as Renovate groups. Each group MUST carry a
concise comment explaining why it is grouped, and dependencies MUST stay alphabetically ordered inside it.

## 2. Features Section Rules

**Features sections are OPTIONAL. Do NOT add a `[features]` section if the crate doesn't already have one. The
`default` feature is implicit and optional when empty.**

When a `[features]` section exists:

- All features MUST be ordered alphabetically. The one exception: `default` MUST be listed FIRST.
- Feature names MUST use kebab-case — lowercase letters and hyphens only.
- Names MUST be descriptive rather than abbreviated. `kernel-debug-svcs` says what it enables; `debug` does
  not say whether it adds debug SVC wrappers, extra assertions, or log output. Likewise `handle-tracking` over
  `tracking`, `result-decoding` over `results`, `virtmem-stats` over `stats`, `tipc-dispatch` over `tipc`.
- Every feature MUST have a `#` comment above it explaining its purpose.

```toml
# ❌ Bad — unordered, undocumented, and named so that no reader can tell what turning one
# on actually pulls in; `dbg` was assumed to gate only assertions and silently shipped the
# debug SVC wrappers into a retail build.
[features]
handle-tracking = []
FFI = []
default = ["result-decoding"]
result_decoding = ["dep:thiserror"]
# stuff
dbg = []
```

```toml
# ✅ Good — `default` first, the rest alphabetical, kebab-case, and each line says what
# enabling it buys.
[features]
# Default features, enabled unless default-features = false
default = ["result-decoding"]
# C-FFI surface: compiles the `__nx_*` symbols the libnx override linker scripts redirect to.
# `ffi` is the workspace-canonical name for this feature and is spelled the same in every crate.
ffi = ["dep:nx-sf"]
# Tracks every live kernel handle so leaks are reported instead of exhausting the handle table
handle-tracking = []
# Kernel debug SVC wrappers (process memory inspection); excluded from retail builds
kernel-debug-svcs = []
# Typed Result decoding: kernel error codes surface as values instead of raw u32 words
result-decoding = ["dep:thiserror"]
```

## Checklist

Before committing Cargo.toml changes, verify:

- [ ] Sections appear in the correct order: `[package]` → target definitions → `[features]` → `[dependencies]`
      → `[dev-dependencies]` → `[build-dependencies]` → `[lints.<tool>]`
- [ ] All dependencies within each section are alphabetically ordered, or split into documented named groups with alphabetical ordering inside each group
- [ ] Features use kebab-case naming
- [ ] `default` feature is listed first (if present)
- [ ] All remaining features are alphabetically ordered
- [ ] Every feature has a descriptive `#` comment above it
- [ ] No `[features]` section added unnecessarily

## References

- [meson-subproject-crate](meson-subproject-crate.md) - Related: How a subproject crate's manifest is wired into
  the Meson build
- [rust-ffi](rust-ffi.md) - Related: The `ffi` feature contract a subproject crate's manifest declares
- [rust-no-std](rust-no-std.md) - Related: The `no_std` dependencies a subproject crate's manifest pulls in
