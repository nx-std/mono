# nx-std - Technical Overview for Coding Agents

## Project Summary

nx-std is a Meson-based monorepo implementing a Rust replacement for `libnx` (the C homebrew library for Nintendo Switch).

**Vision**: Provide a Rust `std` implementation for the Nintendo Switch's Horizon OS.

**Strategy**: Incremental replacement. Rust crates expose C-FFI bindings that replace `libnx` functions at link time, allowing gradual migration from C to Rust while maintaining compatibility with existing homebrew code.

**How it works**:

1. Rust crates implement Switch OS functionality (memory, threads, sync primitives, etc.)
2. Each crate has a public `ffi` module with C-compatible functions (`__nx_*` prefix)
3. `nx-std` is the only staticlib; it re-exports FFI symbols from enabled crates via `src/ffi.rs`
4. Linker scripts (`*_override.ld`) redirect `libnx` symbols to Rust implementations
5. At link time, code calling `libnx` functions transparently uses the Rust implementations

**Configuration**: Meson setup-time options (`use_nx_alloc`, `use_nx_svc`, etc.) control which crates are enabled, selecting corresponding Cargo features for `nx-std`.

## Quick Start

**If you're an AI agent working on this codebase, here's what you need to know immediately:**

1. **Invoke `/code-guidelines` FIRST** → Before planning OR coding (applies in Plan mode too), load the relevant guidelines for the affected crate(s). Skipping this leads to plans that violate project conventions and cause rework.
2. **Use Skills for operations** → Invoke skills (`/code-format`, `/code-check`, `/code-build`, `/code-test`, `/code-deploy`, `/code-review`) instead of running commands directly.
3. **Skills wrap justfile/meson tasks** → Skills provide the interface to `just` and `meson` commands with proper guidance.
4. **Follow the workflow** → Format → Check → Clippy → Build → Hardware Test (when applicable).
5. **Fix ALL warnings** → Zero tolerance for clippy warnings.

**Your first action**: Invoke `/code-guidelines` to load guidelines before drafting a plan or writing any code. For commands, invoke the relevant Skill.

**Testing default**: Hardware tests run only after `/code-format` and `/code-check` are green. Tests are compiled as Switch homebrew (NRO) and validated on-device or emulator via `/code-deploy` + `/code-test`.

## Table of Contents

1. [Principles](#1-principles) — Core design principles guiding this codebase
2. [Code Guidelines](#2-code-guidelines) — Understanding coding standards via `/code-guidelines` skill
3. [Architecture](#3-architecture) — Crate hierarchy, FFI integration, libnx integration
4. [Build System](#4-build-system) — Hybrid Meson + Cargo, prerequisites, cross-compilation
5. [Development Workflow](#5-development-workflow) — How to develop with this codebase
6. [Additional Resources](#6-additional-resources) — Links to documentation


## 1. Principles

**MANDATORY**: Before writing any code, read and internalize these core design principles. Full details, examples, and checklists are in the linked docs — read them every time.

| Principle                   | One-liner                                                        | Full doc                                              |
|-----------------------------|------------------------------------------------------------------|-------------------------------------------------------|
| **Single Responsibility**   | One struct = one reason to change                                | @docs/code/principle-single-responsibility.md         |
| **Open/Closed**             | Extend via new types/trait impls, don't modify existing code     | @docs/code/principle-open-closed.md                   |
| **Law of Demeter**          | Only talk to immediate collaborators — no `a.b().c().d()` chains | @docs/code/principle-law-of-demeter.md                |
| **Validate at the Edge**    | Hard shell (boundary validates), soft core (domain trusts)       | @docs/code/principle-validate-at-edge.md              |
| **Type-Driven Design**      | Make illegal states unrepresentable via the type system          | @docs/code/principle-type-driven-design.md            |
| **Idempotency**             | Operations safe to retry — same effect whether run once or N×    | @docs/code/principle-idempotency.md                   |
| **Inversion of Control**    | Depend on abstractions, not concretions                          | @docs/code/principle-inversion-of-control.md          |
| **Least Surprise**          | Code behaves the way readers expect from its name and signature  | @docs/code/principle-least-surprise.md                |
| **DRY/WET balance**         | Deduplicate real knowledge; tolerate incidental similarity       | @docs/code/principle-dry-wet.md                       |

Use `/code-guidelines principles` to load these on demand when relevant to your task.


## 2. Code Guidelines

Code guideline documentation lives in `docs/code/` with YAML frontmatter for dynamic discovery.

**Guideline docs are authoritative**: Guideline docs define how code should be written. All implementations MUST follow the patterns. If code doesn't follow a pattern, either fix the code or update the pattern (with team approval).

### Guideline Types

| Type               | Scope          | Purpose                                                          |
|--------------------|----------------|------------------------------------------------------------------|
| **Principle**      | `global`       | Universal software principles and best practices                 |
| **Core**           | `global`       | Fundamental coding standards (error handling, logging, modules)  |
| **Architectural**  | `global`       | High-level patterns (workspace structure, crate layout)          |
| **Pattern**        | `global`       | Reusable design patterns (builder, typestate)                    |
| **Crate-specific** | `crate:<name>` | Patterns for specific crates (nx-svc, nx-alloc, etc.)            |
| **Meta**           | `global`       | Documentation format specifications (`docs/__meta__/`)           |

### Skill Invocation

| When You Need To                                                  | Invoke This Skill   |
|-------------------------------------------------------------------|---------------------|
| Understand code guidelines before implementing                    | `/code-guidelines`  |
| "How should I handle errors?", "What's the pattern for X?"        | `/code-guidelines`  |
| Load crate-specific guidelines for `nx-svc`, `nx-alloc`, etc.     | `/code-guidelines`  |
| Review code changes for guideline compliance                      | `/code-review`      |

**Navigation:**

- Need to understand patterns? → `/code-guidelines`
- All guidelines located in `docs/code/`
- Documentation format specs in `docs/__meta__/`

### Code Style

- Uses unstable `rustfmt` features (nightly required)
- Imports grouped: std, external crates, local
- Import granularity at crate level


## 3. Architecture

### Crate Hierarchy

```
nx-std (umbrella crate)
├── nx-alloc     - Global allocator using SVC memory management
├── nx-rand      - Random number generation
├── nx-std-sync  - High-level sync primitives (Mutex, RwLock, etc.)
├── nx-time      - Time utilities
└── sys/
    ├── nx-svc        - Supervisor calls (SVC) interface to Horizon OS
    ├── nx-cpu        - CPU utilities
    ├── nx-sys-mem    - Low-level memory management
    ├── nx-sys-sync   - Low-level synchronization primitives
    └── nx-sys-thread - Thread management
```

### Dependency Flow

`nx-svc` is the foundation — it provides raw SVC bindings. Higher-level crates build on it:

- `nx-alloc` depends on `nx-svc`, `nx-sys-sync`
- `nx-sys-mem` depends on `nx-alloc`, `nx-svc`, `nx-rand`, `nx-std-sync`
- `nx-sys-thread` depends on most other crates

### FFI Integration

The `nx-std` crate is the single staticlib that exports all FFI symbols. Individual crates compile as rlib and expose their FFI functions via public `ffi` modules. When `nx-std` builds, it re-exports these modules based on enabled Cargo features, ensuring symbols compile once without duplication.

Meson options control which crates are included:

```
use_nx         - Master switch for all replacements
use_nx_alloc   - Replace libnx allocation functions
use_nx_svc     - Replace libnx SVC functions
...
```

### libnx Integration

Two modes exist:

1. Build from source (`subprojects/libnx/`) — default
2. Use devkitPro's prebuilt (`subprojects/libnx-dkp/`) — via `use_libnx_dkp` option

The custom libnx build links against Rust crates when `use_nx*` options are enabled.


## 4. Build System

The project uses a **hybrid build system**:

- **Meson** — Orchestrates overall project builds, linking Rust and C code
- **Cargo** — Manages Rust crates (via `just` tasks)

For detailed build system documentation, see [`docs/build_system.md`](docs/build_system.md).

### Prerequisites

- devkitPro toolchain at `/opt/devkitpro` (configurable via `meson.options`)
- Rust nightly toolchain (specified in `rust-toolchain.toml`)
- Meson >= 1.4.0
- `just` command runner

### Build Directory

Build artifacts go to `buildDir/`:

- `buildDir/` — Meson output (NRO/NSP bundles, C objects)
- `buildDir/cargo-target/` — Rust target directory

### Cross-Compilation

The project targets `aarch64-nintendo-switch-freestanding`. Cargo configuration in `.cargo/config.toml` enables:

- `build-std` for `core`, `compiler_builtins`, `alloc`
- `panic = "abort"` for both dev and release profiles


## 5. Development Workflow

**This section provides guidance for AI agents on how to develop with this codebase.**

### Documentation Structure: Separation of Concerns

This project uses three complementary documentation systems. Understanding their roles helps AI agents navigate efficiently:

| Documentation                  | Purpose                  | Content Focus                                                                                                                                |
|--------------------------------|--------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| **AGENTS.md** (this file)      | **WHY** and **WHAT**     | Project architecture, policies, goals, and principles. Answers "What is this project?" and "Why do we do things this way?"                   |
| **Skills** (`.agents/skills/`) | **HOW** and **WHEN**     | Command-line operations and just/meson usage. Answers "How do I run commands?" and "When should I use each command?"                         |
| **Guidelines** (`docs/code/`)  | **HOW** (implementation) | Code implementation guidelines and standards (see [Code Guidelines](#2-code-guidelines)). Answers "How do I write quality, conventional code?"|

**Navigation Guide for AI Agents:**

- Need to understand the project? → Read this file (AGENTS.md)
- Need to run a command? → Invoke the appropriate Skill (`/code-format`, `/code-check`, `/code-build`, `/code-test`, `/code-deploy`)
- Need to write code? → Use `/code-guidelines` to load relevant guidelines

### Core Operating Principle

**🚨 MANDATORY: USE Skills for all common operations. Skills wrap just/meson tasks with proper guidance.**

#### The Golden Rule

**USE Skills (`/code-format`, `/code-check`, `/code-build`, `/code-test`, `/code-deploy`, `/code-review`) for all common operations. Only use `cargo`, `just`, `meson`, or other tools directly when the operation is NOT covered by a skill.**

**Decision process:**

1. **First**: Check if a skill exists for your operation
2. **If exists**: Invoke the skill (provides proper flags, setup, and error handling)
3. **If not exists**: You may run the tool directly (e.g., one-off `cargo` introspection commands)

#### Why Skills Are Preferred

- **Consistency**: Uniform command execution across all developers and AI agents
- **Correctness**: Skills ensure proper flags, target triple, feature selection, and error handling
- **Guidance**: Skills provide context on when and how to use commands
- **Pre-approved workflows**: Skills document which commands can run without user permission

#### Examples

- ✅ **Use skill**: `/code-format` (formatting Rust + Meson)
- ✅ **Use skill**: `/code-check` (compile check, clippy)
- ✅ **Use skill**: `/code-build` (configure/reconfigure/build NROs)
- ✅ **Use skill**: `/code-test` (build + run tests on Switch hardware)
- ✅ **Use skill**: `/code-deploy` (push NRO to Switch via cargo-nx link)
- ✅ **Direct tool OK**: `cargo tree -p nx-svc` (introspection not in justfile)
- ✅ **Direct tool OK**: One-off Meson queries not covered by `just` tasks

#### Command Execution Hierarchy (Priority Order)

When determining which command to run, follow this strict hierarchy:

1. **Priority 1: Skills** (`.agents/skills/`)
   - Skills are the **SINGLE SOURCE OF TRUTH** for all command execution
   - If a Skill documents a command, use it EXACTLY as shown
   - Skills override any other guidance in AGENTS.md or elsewhere

2. **Priority 2: AGENTS.md workflow**
   - High-level workflow guidance (when to format, check, build, test)
   - Refers you to Skills for specific commands

3. **Priority 3: Everything else**
   - Other documentation is supplementary
   - When in conflict, Skills always win

#### Workflow Gate: Use Skills First

**Before running ANY command:**

1. Ask yourself: "Which Skill covers this operation?"
2. Invoke the appropriate skill (e.g., `/code-format`, `/code-check`, `/code-build`, `/code-test`)
3. Let the skill guide you through the operation

**Example decision tree:**

- Need to format a file? → Use `/code-format` skill
- Need to check a crate? → Use `/code-check` skill
- Need to build an NRO or (re)configure Meson? → Use `/code-build` skill
- Need to run tests on hardware? → Use `/code-test` skill (after format + check are green)
- Need to push an NRO to the Switch? → Use `/code-deploy` skill

### Command-Line Operations Reference

**🚨 CRITICAL: Use skills for all operations — invoke them before running commands.**

Available skills and their purposes:

- **Formatting**: `/code-format` — Format Rust and Meson code after editing files
- **Checking/Linting**: `/code-check` — Validate compilation and lint with clippy
- **Building**: `/code-build` — Configure/reconfigure Meson and build NRO/NSP artifacts
- **Testing**: `/code-test` — Build the test NRO and run on Switch hardware
- **Deploying**: `/code-deploy` — Push built artifacts to the Switch via cargo-nx link
- **Guidelines**: `/code-guidelines` — Load relevant code guidelines and patterns
- **Reviewing**: `/code-review` — Review changes for bugs, guideline violations, security

Each Skill provides:

- ✅ **When to use** — Clear guidance on appropriate usage
- ✅ **Available operations** — All supported tasks with proper execution
- ✅ **Examples** — Real-world usage patterns
- ✅ **Pre-approved workflows** — Operations that can run without user permission
- ✅ **Workflow integration** — How operations fit into the development flow

**Remember: If you don't know which operation to perform, invoke the appropriate Skill.**

### Pre-Implementation Checklist

**BEFORE drafting a plan OR writing ANY code, you MUST:**

1. **Understand the task** — Research the codebase and identify affected crate(s)
2. **🚨 MANDATORY: Load implementation guidelines FIRST** — Invoke `/code-guidelines` before drafting any plan or writing any code. This applies equally in Plan mode: the plan itself MUST be grounded in the loaded guidelines, not in assumptions about conventions.
3. **Follow crate-specific guidelines** — Guideline discovery loads crate-specific and core guidelines automatically
4. **Rationale** — Skipping this step leads to plans that violate conventions (e.g., module layout, error handling patterns, FFI surface design), causing avoidable rework.

### Typical Development Workflow

**Follow this workflow when implementing features or fixing bugs:**

#### 1. Research Phase

- Understand the codebase and existing guidelines
- Identify related modules and dependencies (use the [Crate Hierarchy](#crate-hierarchy))
- Review test files and usage examples in `subprojects/tests/`
- Use `/code-guidelines` to load relevant implementation guidelines

#### 2. Planning Phase

**🚨 MANDATORY FIRST STEP (including in Plan mode):** Invoke `/code-guidelines` to load the guidelines for the affected crate(s) BEFORE drafting the plan. The plan's structure, module layout, error handling, and type design decisions MUST reflect the loaded guidelines. Never draft a plan from assumptions about project conventions.

- Create the implementation plan on top of the loaded guidelines
- Ensure plan follows required patterns (error handling, type design, module structure, FFI surface)
- Identify validation checkpoints
- Consider edge cases and error handling according to guidelines
- Ask user questions if requirements are unclear

#### 3. Implementation Phase

**🚨 CRITICAL: Before running ANY command in this phase, invoke the relevant Skill.**

**Copy this checklist and track your progress:**

```
Development Progress:
- [ ] Step 1: Write code following guidelines (use /code-guidelines)
- [ ] Step 2: Format code (use /code-format skill)
- [ ] Step 3: Check compilation (use /code-check skill)
- [ ] Step 4: Fix all compilation errors
- [ ] Step 5: Run clippy (use /code-check skill)
- [ ] Step 6: Fix ALL clippy warnings
- [ ] Step 7: Build artifacts when applicable (use /code-build skill)
- [ ] Step 8: Run hardware tests when warranted (use /code-test skill)
- [ ] Step 9: All required checks pass ✅
```

**Detailed workflow for each work chunk (and before committing):**

1. **Write code** following guidelines from [Code Guidelines](#2-code-guidelines) (loaded via `/code-guidelines`)

2. **Format before checks/commit**:
   - **Use**: `/code-format` skill when you finish a coherent chunk of work
   - **Validation**: Verify no formatting changes remain

3. **Check compilation**:
   - **Use**: `/code-check` skill after changes
   - **Must pass**: Fix all compilation errors
   - **Validation**: Ensure zero errors before proceeding

4. **Lint with clippy**:
   - **Use**: `/code-check` skill for linting
   - **Must pass**: Fix all clippy warnings
   - **Validation**: Re-run until zero warnings before proceeding

5. **Build artifacts (when applicable)**:
   - **Use**: `/code-build` skill to (re)configure Meson and build NROs
   - **Validation**: Build succeeds with the relevant `use_nx*` options enabled

6. **Hardware tests (when warranted, after checks are green)**:
   - **Prerequisite**: `/code-format` and `/code-check` must both be clean — do not run hardware tests before.
   - **Use**: `/code-test` skill — builds the test NRO, deploys via `/code-deploy`, asks the user to confirm results on the console.
   - **When to run**: FFI surface changes, allocator/memory/sync/thread changes, or anything that crosses the C/Rust boundary.
   - **Validation**: Fix failures or record why tests were skipped.

7. **Iterate**: If any validation fails → fix → return to step 2

**Visual Workflow:**

```
Edit File → /code-format skill
          ↓
    /code-check skill (compile) → Fix errors?
          ↓                            ↓ Yes
    /code-check skill (clippy) → (loop back)
          ↓
    ALL CHECKS GREEN ─ gate
          ↓
    /code-build skill (build NRO when applicable)
          ↓
    /code-test skill (hardware validation when warranted)
          ↓              ↓ Fix failure?
    All Pass ✅     (loop back to /code-format)
```

**Remember**: Invoke Skills for all operations. If unsure which skill to use, refer to the Command-Line Operations Reference above.

#### 4. Completion Phase

- Ensure all required checks pass (format, check, clippy, build, and any hardware tests run)
- If hardware tests were skipped, document why and the risk assessment
- Review changes against guidelines (use `/code-review`)
- Document any warnings you couldn't fix and why

### Core Development Principles

**ALL AI agents MUST follow these principles:**

- **Research → Plan → Implement**: Never jump straight to coding
- **Guidelines before planning**: `/code-guidelines` is a prerequisite for both planning AND Plan mode, not just implementation. A plan written without loaded guidelines is considered incomplete.
- **Guideline compliance**: Follow guidelines from [Code Guidelines](#2-code-guidelines)
- **Zero tolerance for errors**: All automated checks must pass
- **Clarity over cleverness**: Choose clear, maintainable solutions
- **FFI safety first**: The C/Rust boundary is unforgiving — uphold soundness invariants and validate at the edge

**Essential conventions:**

- **Maintain type safety**: Leverage Rust's type system fully (see [Type-Driven Design](docs/code/principle-type-driven-design.md))
- **No-std discipline**: Code targets `aarch64-nintendo-switch-freestanding` — no `std`, `panic = "abort"`
- **Validate at FFI boundaries**: Hard shell on the C-facing surface (`__nx_*`); soft core inside (see [Validate at the Edge](docs/code/principle-validate-at-edge.md))
- **Format code before checks/commit**: Use `/code-format` skill
- **Fix all warnings**: Use `/code-check` skill for clippy
- **Test FFI changes on hardware**: Use `/code-test` skill after checks are green

### Summary: Key Takeaways for AI Agents

| What                | Where                                  | When                                                  |
|---------------------|----------------------------------------|-------------------------------------------------------|
| **Plan work**       | `/code-guidelines`                     | BEFORE creating any plan                              |
| **Run commands**    | `.agents/skills/`                      | Check Skills BEFORE any command                       |
| **Write code**      | [Code Guidelines](#2-code-guidelines)  | Load guidelines before implementation                 |
| **Format**          | `/code-format`                         | Before checks or before committing                    |
| **Check**           | `/code-check`                          | After formatting                                      |
| **Lint**            | `/code-check`                          | Fix ALL warnings                                      |
| **Build artifacts** | `/code-build`                          | When NRO/NSP outputs are needed                       |
| **Deploy**          | `/code-deploy`                         | Push built NRO to the Switch                          |
| **Test (hardware)** | `/code-test`                           | Only after checks green; on FFI/foundation changes    |
| **Review**          | `/code-review`                         | Before commits / PRs                                  |

**Golden Rules:**

1. ✅ Invoke Skills for all common operations
2. ✅ Skills wrap just/meson tasks with proper guidance
3. ✅ Follow the workflow: Format → Check → Clippy → Build → Hardware Test (when needed)
4. ✅ Zero tolerance for errors and warnings
5. ✅ Every change improves the codebase

**Remember**: When in doubt, invoke the appropriate Skill!


## 6. Additional Resources

For more detailed information about the project:

- **Build system**: See [`docs/build_system.md`](docs/build_system.md)
- **libnx symbol overrides**: See [`docs/libnx_overrides.md`](docs/libnx_overrides.md)
- **Code guidelines**: Browse `docs/code/` (load via `/code-guidelines`)
- **Documentation format specs**: See `docs/__meta__/`
- **Tests**: C tests in `subprojects/tests/` link against Rust crates to verify FFI correctness
