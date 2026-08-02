---
name: "meson-subproject"
description: "Generic Meson subproject conventions: directory layout, `meson.build` section banners, `meson.options` format, project header, variable naming, and setup-time option style. Load when creating or editing any `subprojects/<name>/meson.build` or `meson.options`, regardless of whether the subproject wraps Rust, C, or a prebuilt library"
type: "arch"
scope: "global"
---

# Meson Subproject Structure

**MANDATORY for ALL `subprojects/<name>/` directories that ship a `meson.build`**

## Table of Contents

1. [Scope](#1-scope)
2. [Directory Layout](#2-directory-layout)
3. [Naming Conventions](#3-naming-conventions)
4. [`meson.build` Skeleton](#4-mesonbuild-skeleton)
5. [`meson.options` Format](#5-mesonoptions-format)
6. [Variable Naming](#6-variable-naming)
7. [Comments and Section Headers](#7-comments-and-section-headers)
8. [Checklist](#checklist)

Workspace-level orchestration (root `meson.build`, root `meson.options`, top-level subproject ordering) is documented in [../workspace](../workspace.md). The Rust-crate specialization of this layout — Cargo manifest, `custom_target` wiring, Cargo/Meson dependency mirror — lives in [meson-subproject-crate](meson-subproject-crate.md).

---

## 1. Scope

A "Meson subproject" is any directory under `subprojects/<name>/` whose root contains a `meson.build`. The shape of the subproject depends on what it wraps:

| Subproject kind                       | Typical contents                                              | Examples                              |
|---------------------------------------|---------------------------------------------------------------|---------------------------------------|
| Rust crate                            | `Cargo.toml`, `src/`, `meson.build` wrapping `cargo build`    | `nx-svc`, `nx-std`, `nx-alloc`        |
| C library built from source           | `src/`, `meson.build` with `static_library(...)`              | `libnx`, `vendor`                     |
| Prebuilt library wrapper              | `meson.build` exposing `declare_dependency(...)` over a `.a`  | `libnx-dkp`, `libdeko3d-dkp`          |
| Sysroot manifest                      | `meson.build` declaring system include / link paths           | `sysroot`, `sysroot-dkp`              |
| Binary / NRO producer                 | `source/`, `meson.build` with `executable(...)` + bundling    | `tests`, `examples`, `nx-hbmenu`      |

The conventions in this document apply to **all** of them. Kind-specific rules (e.g., the Cargo invocation pattern, NRO bundling, linker overrides) live in companion guidelines and refer back here.

---

## 2. Directory Layout

Every subproject root holds, at minimum:

```
subprojects/<kebab-name>/
├── meson.build                # REQUIRED — subproject entry point
├── meson.options              # OPTIONAL — only when the subproject exposes setup-time options
└── ...                        # Kind-specific contents (src/, source/, Cargo.toml, ...)
```

### Required Files

- `meson.build` — the entry point Meson loads when this subproject is pulled in via `subproject('<name>')`.

### Optional Files

- `meson.options` — declare setup-time options consumed by this subproject. Subproject options live next to the `meson.build` that reads them; do NOT move them to the root `meson.options` (which is reserved for workspace-wide knobs).

Files used by other build systems (e.g., a Cargo manifest, C source trees) live at the subproject root according to their own conventions and are not part of the Meson contract per se.

---

## 3. Naming Conventions

| Artifact                            | Convention                                  | Example                              |
|-------------------------------------|---------------------------------------------|--------------------------------------|
| Subproject directory                | `kebab-case`                                | `nx-sys-thread-tls`, `libnx-dkp`     |
| Meson `project()` name              | matches the directory exactly               | `project('nx-sys-thread-tls', ...)`  |
| Setup-time options                  | `snake_case`                                | `use_nx`, `use_libnx_dkp`            |
| Exposed Meson variables             | `snake_case` mirror of the subproject name  | `nx_sys_thread_tls_dep`              |

**Rules:**

- The directory name, the `project()` name, and the variable prefix MUST all derive from the same slug. The Meson side flattens hyphens to underscores for variables (`nx-svc` → `nx_svc_*`); the directory and `project()` keep the hyphenated form.
- Subprojects whose primary artifact is a C library / executable MAY use a shorter `project()` name when there is a long-standing convention (e.g., `libnx` declares `project('nx', ...)`). When deviating, the directory still uses the full kebab-case name; document the deviation at the top of the `meson.build`.

---

## 4. `meson.build` Skeleton

Every subproject `meson.build` follows the same shape — a `project(...)` header, optional setup-time option resolution, dependencies, the actual target(s), and a final dependency declaration. Sections are separated by `#----` banner comments (Section 7).

### 4.1 Project Header

`project(...)` is always the first non-comment line:

```meson
project('<kebab-name>', version : '<x.y.z>')
```

- **Name** matches the directory.
- **Version** is mandatory. Rust crates use `'0.1.0'` to mirror their `Cargo.toml`; native libraries use their upstream semver.
- **Language list** is included when the subproject compiles C / C++ / assembly — `project('nx', 'c', ...)`. Rust-only subprojects omit the language list because Cargo handles compilation.
- **`meson_version`** is set only when the subproject relies on a feature newer than the workspace baseline pinned by the root `meson.build` (see [../workspace](../workspace.md)). Repeating the constraint is allowed when the subproject genuinely needs a newer Meson.
- **`default_options`** is reserved for native code: warning level, error treatment, C/C++ dialect. Do NOT put project options there — those go in `meson.options`.

```meson
# C library — explicit language list and compile defaults
project('nx', 'c',
        version : '4.9.0',
        meson_version : '>= 1.4.0',
        default_options : [
            'warning_level=1', # -Wall
            'werror=true',     # -Werror
            'cpp_std=gnu++11',
        ])
```

### 4.2 Canonical Section Order

After the header, sections appear in this order. Omit any that do not apply.

| #   | Section banner                  | Purpose                                                                              |
|-----|---------------------------------|--------------------------------------------------------------------------------------|
| 1   | `Options`                       | Resolve `get_option(...)` calls into local variables when the same value is reused.  |
| 2   | `Dependencies`                  | Pull sibling subprojects via `subproject(...)`, collect them into a `deps` list.     |
| 3   | `Compilation specific files`    | Linker scripts, spec files, default assets exposed to consumers.                     |
| 4   | `Source files`                  | `files(...)` lists for native sources (skip for Rust crates — Cargo discovers them). |
| 5   | `Data files`                    | `files(...)` for embedded data, plus any `bin2s` / `generator` plumbing.             |
| 6   | `Static library` / `ELF` / `Target` | The actual build target: `static_library`, `executable`, or `custom_target`.     |
| 7   | `Post-processing and bundling`  | Per-output transforms (NSP/NRO bundling, listings, post-link tools).                 |
| 8   | `Dependency declaration`        | Final `declare_dependency(...)` exposing the target to downstream consumers.         |

A binary-producing subproject (`tests`, `examples`, `nx-hbmenu`) typically stops at section 7 because nothing downstream consumes it as a dependency. A library subproject typically stops at section 6 plus section 8 and omits 7.

### 4.3 Options Resolution Block

When a subproject reads the same `get_option(...)` value in multiple places, lift it to a named local at the top:

```meson
#---------------------------------------------------------------------------------
# Options
#---------------------------------------------------------------------------------
use_nx = get_option('use_nx')
use_nx_alloc = get_option('use_nx_alloc').enable_auto_if(use_nx.enabled()).disable_auto_if(use_nx.disabled())
```

- Use `enable_auto_if` / `disable_auto_if` to derive child toggles from a master switch (the `auto` feature pattern below).
- One option per line; align by reader preference but keep the option name as the variable name (`use_nx_alloc = get_option('use_nx_alloc')`).
- If an option is read exactly once, calling `get_option(...)` at the use site is fine — skip the block.

### 4.4 Dependency Declaration

End with a `declare_dependency(...)` that exposes everything a downstream consumer needs:

```meson
#---------------------------------------------------------------------------------
# Dependency declaration
#---------------------------------------------------------------------------------
<name>_dep = declare_dependency(
    include_directories : inc,
    link_with : <name>_lib,
    link_args : deps_override_link_args,   # only when overrides flow through
    dependencies : deps,
)
```

- Always assign the dependency to a `<name>_dep` variable so consumers can `get_variable('<name>_dep')`.
- Order fields consistently: `include_directories`, `link_with` / `sources`, `link_args`, `dependencies`. Omit fields that do not apply.
- Sibling variables that escape `declare_dependency` (linker override paths, special assets, derived link-args lists) live immediately above this block; see Section 6.

---

## 5. `meson.options` Format

Subproject options live in `subprojects/<name>/meson.options`. Every option follows the same shape:

```meson
option(
    '<snake_case_name>',
    type : '<type>', value : '<default>',
    description : '<single-sentence description>',
    yield : true,
)
```

### Rules

- **Name** is `snake_case`. Use the same slug Meson would use in `-D<name>=...`.
- **`type`** matches Meson's option types (`feature`, `boolean`, `string`, `combo`, `array`, `integer`). Prefer `feature` over `boolean` when the option has an `auto` resolution path — `feature` supports `enabled` / `disabled` / `auto`.
- **`value`** sets the default. For `feature` options use `'auto'` unless the option is intentionally off by default (`'disabled'`) — annotate the latter with a trailing comment explaining why (`# Disable by default, WIP`).
- **`description`** is a single sentence, present tense, no trailing period. Mention the user-facing effect ("Enable the `service-apm` feature", "Override all libnx functions with nx-alloc functions").
- **`yield : true`** is the default. It lets a parent project propagate the option down to this subproject; omit only when the option is genuinely subproject-local.
- One option per `option(...)` block, separated by a blank line.

### Grouping and Ordering

- Group related options together (e.g., all `use_nx_service_*` toggles in one block, all `use_nx_sys_*` toggles in another). Inside a group, prefer the order the project's documentation already establishes — strict alphabetical sorting is NOT required.
- Master / specialization pairs: the master switch (`use_nx`) appears first; its specializations follow.
- Keep the file flat — no conditionals, no helper functions. Logic lives in `meson.build`.

### Cross-Subproject Mirroring

When two subprojects need to expose the same toggle (typical for the `use_nx*` feature flags consumed by both `subprojects/libnx/` and `subprojects/nx-std/`), declare the option in both `meson.options` files with **identical** name, type, default, and description. The matching `yield : true` setting lets the root `meson.options` value flow into both subprojects.

---

## 6. Variable Naming

All Meson variables exposed by a subproject use the `snake_case` mirror of the subproject name as the prefix:

| Variable                       | Convention                                       | Example                              |
|--------------------------------|--------------------------------------------------|--------------------------------------|
| Subproject handle              | `<name>_proj`                                    | `nx_svc_proj`                        |
| Dependency declaration         | `<name>_dep`                                     | `nx_svc_dep`                         |
| Primary build target           | `<name>_tgt` / `<name>_lib` / `<name>_elf`       | `nx_svc_tgt`, `nx_lib`               |
| Linker script(s)               | `<name>_ld_override` / `<name>_ld_overrides`     | `nx_svc_ld_override`                 |
| Aggregated linker args         | `<name>_dep_override_link_args`                  | `nx_std_dep_override_link_args`      |
| Misc exported asset            | `<name>_<asset>`                                 | `nx_switch_specs`, `nx_default_icon` |

Consumers retrieve these variables via `subproject('<kebab-name>').get_variable('<name>_dep')`. The `proj` / `dep` pair is the minimum surface: every library subproject exposes at least those two.

**Rules:**

- The snake_case prefix MUST come from the directory name, not from the artifact name. `nx-svc` → `nx_svc_*`, even if it produces `libnx_svc.rlib`.
- Plural list variables use the plural form (`nx_rt_ld_overrides`); single-path variables stay singular.
- Variables that are NOT part of the consumer contract (loop scratch, intermediate `deps` accumulators) need not follow the prefix rule.

---

## 7. Comments and Section Headers

### Section Banner

Every top-level section is introduced by a three-line banner of `#` characters. The middle line names the section; the two surrounding rules are exactly 81 `#` characters wide:

```meson
#---------------------------------------------------------------------------------
# Dependencies
#---------------------------------------------------------------------------------
```

- The section name is `Title Case` (e.g., `Static library`, `Dependency declaration`).
- Do not nest banners. Use plain `# Subsection` comments for finer structure.
- Keep section names from the canonical list in Section 4.2 when applicable so grep-based audits across the workspace remain stable.

### Inline Comments

- Use `#` comments to explain non-obvious wiring (e.g., why a `subproject(...)` call is gated, what an option does at this site).
- Keep one-line comments above the line they describe, not at the end. Reserve trailing comments for unit/value annotations (`# -Wall`, `# Disable by default, WIP`).
- Avoid restating what the code does. A comment like `# Get the cargo program` above `cargo = find_program('cargo')` adds nothing.

---

## Checklist

Before committing a new or modified `subprojects/<name>/meson.build` or `meson.options`, verify:

### Directory and naming

- [ ] Directory name is `kebab-case`.
- [ ] `meson.build` exists at the subproject root.
- [ ] `meson.options` is present only when the subproject exposes setup-time options.
- [ ] `project()` name matches the directory (or, for legacy native libraries, deviates with a comment).

### `meson.build`

- [ ] First non-comment line is `project('<name>', version : '...')` with the right language list and `default_options` for the subproject kind.
- [ ] Sections appear in the canonical order (Options → Dependencies → … → Dependency declaration) and are separated by `#----` banners.
- [ ] Repeated `get_option(...)` reads are lifted into an `Options` block; one-shot reads stay inline.
- [ ] `declare_dependency(...)` is the final section for library subprojects; binary subprojects stop at the build target / bundling block.

### `meson.options`

- [ ] Every option uses the `option('<name>', type : ..., value : ..., description : ..., yield : true)` shape.
- [ ] `feature` options default to `'auto'` unless intentionally off, in which case a trailing `# ...` comment explains why.
- [ ] Descriptions are single-sentence, present tense, no trailing period.
- [ ] Options shared with another subproject have identical name / type / default / description on both sides.
- [ ] Options are grouped by domain; master switches precede their specializations.

### Variables and comments

- [ ] Exported variables use the `<name>_` snake_case prefix (`<name>_proj`, `<name>_dep`, `<name>_tgt`, `<name>_ld_override`, …).
- [ ] Section banners use the 81-char `#---` form and Title Case section names from the canonical list.
- [ ] Inline comments explain why, not what; trailing comments are limited to unit / annotation use.

## References

- [meson-subproject-crate](meson-subproject-crate.md) - Related: Rust-crate specialization of this layout
- [meson-linker-script](meson-linker-script.md) - Related: `*_override.ld` linker scripts exposed as subproject variables
- [../workspace](../workspace.md) - See also: Workspace-level Meson root and crate categorization
