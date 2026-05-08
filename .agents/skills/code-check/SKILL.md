---
name: code-check
description: Validate and lint Rust code after changes in the nx-std monorepo. Use after editing .rs files, when user mentions compilation errors, type checking, linting, clippy warnings, or before commits/PRs. Prefers IDE/rust-analyzer diagnostics when available, defaults to per-crate `just` commands, and auto-fixes clippy lints with `--fix`.
allowed-tools: "Bash(just check:*), Bash(just check-rs:*), Bash(just check-crate:*), Bash(just clippy:*), Bash(just clippy-crate:*), mcp__ide__getDiagnostics, LSP"
---

# Code Checking Skill

Code validation and linting for the nx-std Rust workspace. Optimized for **minimum wall-clock time to first error**: cheapest signal first, per-crate over workspace whenever possible, auto-fix before hand-fix.

## When to Use This Skill

- Validate Rust code after making changes
- Surface compilation errors without a full Meson build
- Lint code with clippy (and auto-apply machine-applicable fixes)
- Ensure code quality before commits or PRs

## Command Selection: Three-Stage Funnel

Run the stages in order. Move to the next stage only after the current one is clean.

```
Stage 0 (optional)                   Stage 1 — mandatory             Stage 2 — mandatory
rust-analyzer-lsp diagnostics   →    just check-crate (per crate) →  just clippy-crate --fix … (per crate)
(via mcp__ide__getDiagnostics)       ↑ escalate to check-rs only     ↑ escalate to clippy --fix … only
                                       on blast-radius signals          on blast-radius signals
```

**Default to per-crate.** Iterate over edited crates — do not collapse them into a workspace command. `just check-rs` / `just clippy` compile every crate in the workspace and are only faster when a blast-radius signal fires.

**Crate derivation.** Map edited file paths to crates by reading the nearest enclosing `Cargo.toml` (workspace layout: `subprojects/<name>/`). Include every edited crate.

### Blast-radius escalation (Stages 1 and 2)

Escalate from per-crate to workspace commands only when one of these signals is present:

| Signal                           | Meaning                                                                                                         |
|----------------------------------|-----------------------------------------------------------------------------------------------------------------|
| **Foundation crate edited**      | Crate that many others depend on (`nx-svc`, `nx-sys-mem`, `nx-sys-sync`). Changes may break unseen dependents.  |
| **Public API change**            | Trait signature change, removed/renamed public item, breaking type change. Downstream consumers break silently. |
| **FFI surface change**           | Added/removed/renamed `__nx_*` functions in any `ffi` module — `nx-std` re-exports may break.                   |
| **Cargo manifest change**        | Workspace-level `Cargo.toml`/`Cargo.lock`, or a crate's `[dependencies]`/`[features]` altered.                  |
| **Suspected dependent breakage** | Per-crate check passed but you have concrete reason to think other crates may now fail to compile.              |

If no signal fires, stay per-crate.

## Stage 0 — Rust-analyzer Diagnostics (Fast Path)

Stage 0 reads diagnostics rust-analyzer has already computed in the background. It is near-instant because it does not invoke cargo.

**Plugin:** The Claude Code plugin that provides Rust language-server capabilities is **`rust-analyzer-lsp`** (from the `claude-plugins-official` marketplace). When installed it powers the `LSP` tool for `.rs` files and feeds rust-analyzer diagnostics into the conversation via `mcp__ide__getDiagnostics`.

**Probe availability.** Call `mcp__ide__getDiagnostics` once with no `uri`. If it returns (even an empty array), diagnostics are live — proceed. If it errors or is not available, skip Stage 0 and go to Stage 1.

**Per-file diagnostics.** For each edited Rust file, call `mcp__ide__getDiagnostics` with `uri=file://<absolute-path>`. Fix reported errors and warnings before moving on.

**Supplementary LSP operations.** While fixing, the `LSP` tool exposes `hover`, `documentSymbol`, `goToDefinition`, `findReferences`, etc., which help locate related code. These are navigational — not a substitute for diagnostics.

**Stage 0 is advisory, never terminal.** Stage 1 remains mandatory even when Stage 0 is clean: rust-analyzer may be stale, and per-crate `cargo check` catches everything rust-analyzer's current configuration ignores.

## Available Commands

### Check Rust Code (Workspace)
```bash
just check-rs [EXTRA_FLAGS]
```
Checks all Rust code (`cargo check`). Use only when a blast-radius signal fires. **Alias**: `just check`.

Examples:
- `just check-rs` — check all Rust code
- `just check-rs --features ffi` — check with FFI bindings enabled
- `just check-rs --all-features` — check with every feature

### Check Specific Crate
```bash
just check-crate <CRATE> [EXTRA_FLAGS]
```
Checks one crate (`cargo check -p <CRATE>`). **Default Stage 1 command.**

Examples:
- `just check-crate nx-svc`
- `just check-crate nx-alloc --features ffi`

### Lint Rust Code (Workspace) with Auto-fix
```bash
just clippy [EXTRA_FLAGS]
```
Lints all Rust code (`cargo clippy`). Use only when a blast-radius signal fires.

Examples:
- `just clippy --fix --allow-dirty --allow-staged` — standard escalated auto-fix pass
- `just clippy` — residue pass (after `--fix`) to surface remaining warnings
- `just clippy --features ffi` — lint with FFI feature enabled
- `just clippy -- -D warnings` — treat warnings as errors

### Lint Specific Crate with Auto-fix
```bash
just clippy-crate <CRATE> [EXTRA_FLAGS]
```
Lints one crate (`cargo clippy -p <CRATE> --no-deps`). **Default Stage 2 command.**

`--no-deps` (already wired into the justfile recipe) restricts analysis to the specified crate:
- **Faster execution**: skip dependency code you don't control
- **Focused output**: only warnings from your crate
- **Actionable results**: every warning is in code you can fix

Examples:
- `just clippy-crate nx-svc --fix --allow-dirty --allow-staged` — standard auto-fix pass
- `just clippy-crate nx-svc` — residue pass after `--fix`
- `just clippy-crate nx-alloc --all-features`

#### Auto-fix semantics

`cargo clippy --fix` automatically rewrites the source to apply all *machine-applicable* suggestions (unused imports, redundant clones, idiomatic rewrites, etc.). After the auto-fix pass:
- **Residual warnings remain for hand-fixing.** Re-run the same command **without `--fix`** to list the residue, then hand-fix.
- **Formatting may shift.** Re-run `/code-format` after `--fix` applies changes.
- **`--allow-dirty --allow-staged` are required** because the dev workflow always has uncommitted changes when this skill runs; without them cargo refuses to modify files.

## Important Guidelines

### MANDATORY: Run Checks After Changes

You MUST run checks after making code changes. Follow the three-stage funnel above.

Before considering a task complete: all checks MUST pass AND all clippy warnings MUST be fixed (either auto-fixed or hand-fixed).

### Example Workflows

**Common case (leaf-crate edits, no escalation):**
Edits in `subprojects/nx-alloc/src/…` and `subprojects/nx-time/src/…`. Neither is foundational; no Cargo manifest changes.

1. Format changes: use `/code-format`.
2. **Stage 0** — probe `mcp__ide__getDiagnostics`. If available, call with each edited file's `file://` URI. Fix reported issues.
3. **Stage 1** — per-crate check:
   - `just check-crate nx-alloc` → fix errors → repeat until clean.
   - `just check-crate nx-time` → fix errors → repeat until clean.
4. **Stage 2** — per-crate clippy auto-fix:
   - `just clippy-crate nx-alloc --fix --allow-dirty --allow-staged`
   - `just clippy-crate nx-time --fix --allow-dirty --allow-staged`
   - If warnings remain: re-run each without `--fix`, hand-fix the residue.
5. Re-run `/code-format` if `--fix` changed source.
6. Done when: zero errors AND zero warnings across both crates.

**Escalation case (FFI surface change in foundation crate):**
Edit in `subprojects/nx-svc/src/ffi.rs` adding a new `__nx_*` function. Foundation + FFI surface signals fire.

1. Format changes: use `/code-format`.
2. **Stage 0** — probe `mcp__ide__getDiagnostics`; fix surfaced issues.
3. **Stage 1 (escalated)** — workspace check (with FFI):
   - `just check-rs --features ffi` → fix errors in `nx-svc` and every dependent that broke → repeat until clean.
4. **Stage 2 (escalated)** — workspace clippy auto-fix:
   - `just clippy --features ffi --fix --allow-dirty --allow-staged`
   - If warnings remain: `just clippy --features ffi` (no `--fix`), hand-fix.
5. Re-run `/code-format` if `--fix` changed source.
6. Done when: zero errors AND zero warnings across the workspace.

## Common Mistakes to Avoid

### Anti-patterns
- **Never run `cargo check` directly** — use `just check-crate` or `just check-rs`.
- **Never run `cargo clippy` directly** — the justfile recipes add proper flags like `--no-deps`.
- **Never default to `just check-rs` / `just clippy` for convenience** — workspace commands compile every crate. Use only when a blast-radius signal fires.
- **Never skip Stage 1 just because Stage 0 is clean** — rust-analyzer may be stale.
- **Never skip FFI feature checks for foundation crates** — `nx-std` re-exports FFI symbols only when `--features ffi` is set.
- **Never run clippy without `--fix` on the first pass** — wastes cycles on machine-applicable lints.
- **Never pass `--fix` without `--allow-dirty --allow-staged`** — cargo refuses to modify files in a dirty tree.
- **Never ignore clippy warnings** — fix-all-warnings is mandatory.

### Best practices
- Start with Stage 0 when `rust-analyzer-lsp` is installed and `mcp__ide__getDiagnostics` responds.
- Default to per-crate commands; escalate only on a blast-radius signal.
- Always use `--fix --allow-dirty --allow-staged` on the first clippy pass; re-run without `--fix` to list residue.
- Fix compilation errors (Stage 1) before running clippy (Stage 2).
- Run with `--features ffi` (or `--all-features`) for any change touching `ffi` modules.
- Run the full funnel when you finish a coherent chunk of work or before committing.

## Pre-approved Commands

These commands can run without user permission:
- `mcp__ide__getDiagnostics` — read-only.
- `LSP` tool operations against `.rs` files — read-only.
- `just check-rs`, `just check-crate <crate>` — safe, read-only.
- `just clippy`, `just clippy-crate <crate>` — safe, read-only.
- `just clippy --fix --allow-dirty --allow-staged` and `just clippy-crate <crate> --fix --allow-dirty --allow-staged` — auto-apply of machine-applicable fixes; affects only source files already being edited in the current workflow.

## Related Skills

- `/code-format` — Format code before/after running checks.
- `/code-build` — Build NRO/Meson targets after validation passes.
- `/code-test` — Hardware tests after checks are green.
