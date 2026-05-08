---
name: code-format
description: Format Rust and Meson code in the nx-std monorepo. Use immediately after editing .rs/meson.build/meson.options/justfile, when user mentions formatting, code style, rustfmt, or before commits/PRs.
allowed-tools: "Bash(just fmt:*), Bash(just fmt-rs:*), Bash(just fmt-rs-check:*), Bash(just fmt-meson:*), Bash(just fmt-meson-check:*)"
---

# Code Formatting Skill

Code formatting operations for the nx-std monorepo (Rust workspace + Meson build system).

## When to Use This Skill

- Format code after editing Rust or Meson files
- Check if code meets formatting standards
- Ensure code formatting compliance before commits

## Command Selection Rules

| File Type                                     | Command          | Rationale                                  |
|-----------------------------------------------|------------------|--------------------------------------------|
| Rust (`.rs`)                                  | `just fmt-rs`    | Workspace-wide nightly rustfmt; standard.  |
| Meson (`meson.build`, `meson.options`, `justfile`) | `just fmt-meson` | Project Meson formatter.                   |

**Decision process:**
1. If you edited any Rust files → run `just fmt-rs`.
2. If you edited any Meson files → run `just fmt-meson`.
3. If both → run both (order does not matter).

## Available Commands

### Format Rust Code
```bash
just fmt-rs
```
Formats all Rust code using nightly `cargo fmt --all`. **Alias:** `just fmt`.

### Check Rust Formatting
```bash
just fmt-rs-check
```
Checks formatting without changes. **Alias:** `just fmt-check`.

### Format Meson Files
```bash
just fmt-meson
```
Formats `meson.build`, `meson.options`, and `justfile`.

### Check Meson Formatting
```bash
just fmt-meson-check
```

## Important Guidelines

### Format Before Checks/Commit

Format when you finish a coherent chunk of work and before running checks or committing.

### Example Workflows

**Single Rust file edit:**
1. Edit `subprojects/nx-alloc/src/lib.rs`.
2. Run `just fmt-rs`.
3. Run `/code-check`.

**Meson file edit:**
1. Edit `subprojects/libnx/meson.build`.
2. Run `just fmt-meson`.
3. Run `just configure` / `just reconfigure` if options changed.

## Common Mistakes to Avoid

### Anti-patterns
- **Never run `cargo fmt` or `rustfmt` directly** — use `just fmt-rs` (justfile selects nightly + project config).
- **Never skip formatting before checks/commit** — even minor edits.
- **Never commit unformatted code** — verify with `just fmt-rs-check` and `just fmt-meson-check`.

### Best Practices
- Format before running checks/tests or before committing.
- Run both `just fmt-rs-check` and `just fmt-meson-check` before commits.

## Formatting Configuration

- **Rust**: nightly rustfmt (pinned via `rust-toolchain.toml`), config in `rustfmt.toml` with unstable features (import grouping std/external/local, crate-level granularity).
- **Meson**: project-wide style applied to `meson.build`, `meson.options`, and `justfile`.

## Next Steps

After formatting:
1. **Check compilation** → `/code-check`
2. **Run clippy** → `/code-check`
3. **Build targets** → `/code-build`
4. **Run tests** → `/code-test`
