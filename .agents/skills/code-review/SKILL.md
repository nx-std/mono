---
name: code-review
description: Deep review of the working branch — rule compliance, bugs, regressions, security, soundness. Use before opening a PR, or when a change needs scrutiny beyond /code-rules-check.
---

# Code Review Skill

A thorough review of the current branch, run locally. It performs `/code-rules-check` at review depth — fanned
out across rule groups — and adds what a compliance check cannot see: logic gaps, regressions, security,
safety, and soundness.

## When to Use This Skill

- Before opening a PR
- After a large, risky, or long-running implementation
- Reviewing someone else's branch locally
- When `/code-rules-check` is clean but the change still warrants scrutiny

For the routine "does this follow the rules?" pass after finishing a piece of work, use `/code-rules-check`
alone — it is a fraction of the cost and is the gate in the development workflow.

## Review Checklist

Please review this code change and provide feedback on:

### 1. Security & Soundness Concerns

Review for memory-safety, FFI-soundness, and security issues:
- `unsafe` blocks: precondition documentation, invariant maintenance, raw pointer validity, lifetime soundness
- FFI boundaries: pointer validity, null checks, lifetime/ownership at the C boundary, ABI correctness (`extern "C"`, `#[repr(C)]`, `#[no_mangle]`), panic-across-FFI prevention
- Data races / unsynchronized access in `Sync`/`Send` impls
- Linker-script (`*_override.ld`) overrides: ensure renamed/removed `__nx_*` symbols are reflected
- Exposed secrets or credentials in code or test data
- Input validation at any C-FFI entry point

### 2. Principles Violations

The `principle-*` documents are the design rules, and the `/code-rules` catalog states each one in full — work
from the catalog rather than a list here, which would cover a subset and go stale on the next edit.

Judge the change against all of them. Read a full principle document when you need to *argue* a finding: the
examples and the Pragmatism Caveat are what separate a violation from a deliberate, documented exception, and
a principle finding that ignores the caveat will be rejected.

### 3. Potential Bugs

Look for common programming errors such as:
- Off-by-one errors
- Incorrect conditionals
- Use of wrong variable when multiple variables of same type are in scope
- `min` vs `max`, `first` vs `last`, flipped ordering
- Iterating over hashmap/hashset in order-sensitive operations

### 4. Panic Branches

Identify panic branches that cannot be locally proven to be unreachable:
- `unwrap` or `expect` calls
- Indexing operations
- Panicking operations on external data

**Note**: This overlaps with the error handling patterns documented in `docs/code/rust-errors-handling.md`. Verify compliance with the project's error handling standards.

### 5. Backwards Compatibility

Verify backwards compatibility is maintained:
- FFI surface (`__nx_*`): existing function signatures, calling convention, and ABI must not silently change — downstream homebrew links against these symbols
- Renamed/removed FFI symbols must be reflected in the corresponding `*_override.ld` linker script and `nx-std/src/ffi.rs` re-exports
- Cargo `[features]`: removing or renaming a feature breaks consumers; verify `meson.options` `use_nx*` switches still wire correctly to feature names
- `use_nx_*` feature resolution: the dependency table and resolution live only in `nx-std`'s `meson.build`; crates below it export fragment paths unconditionally and read no features (`docs/code/meson-options-features.md`) — a second resolution copy or a feature read below the resolver is a finding
- Changes to `#[repr(C)]` types crossing the FFI boundary: layout/size/alignment must remain compatible

### 6. Code Rules Compliance

Run `/code-rules-check`, forcing its fan-out path regardless of diff size: one agent per rule group, spawned
in a single message, each applying its documents' `## Checklist` items to the diff. That skill owns the
procedure — which documents govern a change, how groups are derived, and the report format. Do not restate
its rules here; they change when `docs/code/` changes.

A finding in this dimension is a rule violation with a document behind it. Anything a reviewer notices that no
document states belongs in the dimensions above and below, not here.

### 7. Testing

Evaluate test coverage and quality. Tests in this project are NRO-based, executed on Switch hardware via `subprojects/tests/`:
- Reduced test coverage without justification (especially for FFI-surface changes)
- Tests that don't actually exercise the FFI replacement path (must be built with `use_nx=enabled`)
- Tests with race conditions or non-deterministic behavior
- Changes to existing tests that weaken assertions
- Changes to tests that are actually a symptom of breaking changes to user-visible behaviour
- New `__nx_*` functions exposed without a corresponding C test in `subprojects/tests/source/`

### 8. Performance

Check for performance issues:
- Inefficient algorithms or data structures
- Unnecessary heap allocation in hot paths (this is a `no_std`-friendly OS-replacement context — prefer stack/static where possible)
- Lock granularity / lock-held-across-syscall patterns in sync primitives

### 9. Documentation

Ensure documentation is up-to-date:
- Public API doc-comments reflect new/changed signatures and `unsafe` preconditions
- `AGENTS.md` / `docs/build_system.md` reflect any new Meson option, justfile recipe, or build-flow change
- `docs/libnx_overrides.md` reflects added/removed `__nx_*` symbols and their linker-script wiring
- README and architectural docs reflect current behavior

### 10. Dead Code

Find dead code that is not caught by warnings:
- Overriding values that should be read first
- Silently dead code due to `pub`
- `todo!()` or `dbg!()` macros left in production code

### 11. Inconsistencies

Look for inconsistencies between comments and code:
- Documentation that doesn't match implementation
- Misleading variable names or comments
- Outdated comments after refactoring

## Important Guidelines

### Focus on Actionable Feedback

- Provide specific, actionable feedback on actual lines of code
- Avoid general comments without code references
- Reference specific file paths and line numbers
- Suggest concrete improvements

### Rule Compliance is Critical

Rule violations should be treated seriously as they:
- Reduce codebase consistency
- Make maintenance harder
- May introduce security vulnerabilities (in security-sensitive crates)
- Conflict with established architectural decisions

Always run the rules-compliance dimension (section 6) as part of every code review.

### Review Priority

Sections are ordered by priority — review from top to bottom:
1. **Security concerns** (§1, highest priority)
2. **Principles violations** (§2)
3. **Potential bugs** and **panic branches** (§3–4)
4. **Backwards compatibility** (§5)
5. **Code rules compliance** (§6)
6. **Testing** (§7)
7. **Performance** (§8)
8. **Documentation**, **dead code**, and **inconsistencies** (§9–11)

## Next Steps

After completing the code review:
1. Provide clear, prioritized feedback
2. Distinguish between blocking issues (bugs, soundness, FFI/ABI breaks) and suggestions (style, performance)
3. Cite the `docs/code/` document behind every rule finding
4. Suggest using `/code-format`, `/code-check`, and `/code-test` skills to validate fixes
5. Run `/docs-fmt-check` when the change touches `docs/code/` or `docs/__meta__/`
