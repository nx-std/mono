---
name: check
description: Check Rust code compilation and lint with clippy. Use when checking if code compiles, running clippy, validating changes, or before building.
allowed-tools: Bash(just --list:*), Bash(just check:*), Bash(just check-rs:*), Bash(just check-crate:*), Bash(just clippy:*), Bash(just clippy-crate:*)
---

# Code Check and Lint Skill

Check Rust code compilation and run clippy linter.

## Commands

**Check all Rust code compiles**:
```bash
just check-rs
```
Alias: `just check`

**Check specific crate**:
```bash
just check-crate <crate-name>
```

**Lint all Rust code**:
```bash
just clippy
```

**Lint specific crate**:
```bash
just clippy-crate <crate-name>
```

## Checking with Features

Many crates have optional features (like `ffi` for FFI bindings) that should be checked.

**Check with FFI feature**:
```bash
just check-rs --features ffi
just clippy --features ffi
```

**Check with all features**:
```bash
just check-rs --all-features
just clippy --all-features
```

**Per-crate feature checks**:
```bash
just check-crate <crate-name> --features ffi
just clippy-crate <crate-name> --all-features
```

## Workflow

1. Format code → `/format`
2. Check crate compilation → `just check-crate <crate-name>`
3. Lint crate with clippy → `just clippy-crate <crate-name>`
4. Fix all warnings, repeat 2-3 until crate is clean
5. Check whole project → `just check-rs`
6. Lint whole project → `just clippy`
7. Fix any remaining warnings
8. Check with FFI features → `just check-rs --features ffi` and `just clippy --features ffi`
9. Check with all features → `just check-rs --all-features` and `just clippy --all-features`
10. Fix any feature-specific warnings

## Error Handling

- Compilation errors → Fix reported issues, format, re-check
- Clippy warnings → Fix all warnings (mandatory), format, re-run

## Critical Rules

- NEVER use `cargo check` or `cargo clippy` directly - use `just` commands
- FIX ALL WARNINGS - warnings are not acceptable
- Format before checking - run `/format` first

## Related Skills

- `/format` - Format code before running checks
- `/build` - Build targets after validation passes