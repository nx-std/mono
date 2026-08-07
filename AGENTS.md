# nx-std - Technical Overview for Coding Agents

## Project Summary

nx-std is a Meson-based monorepo implementing a Rust replacement for `libnx` (the C homebrew library for Nintendo Switch).

**Vision**: Provide a Rust `std` implementation for the Nintendo Switch's Horizon OS.

**Strategy**: Incremental replacement. Rust crates expose C-FFI bindings that replace `libnx` functions at link time, allowing gradual migration from C to Rust while maintaining compatibility with existing homebrew code.

**The `std` port is a goal, not a side effect.** Every crate under `sys/` is a building block for it, and this shapes design decisions throughout the workspace:

- **`sys/` mirrors `std::sys`.** The `nx-sys-*` crates track the platform-abstraction-layer (PAL) module tree inside Rust's `library/std/src/sys/` — `nx-sys-sync` ↔ `std::sys::sync`, `nx-sys-thread` ↔ `std::sys::thread`, and so on. When adding a `sys/` crate, name it after the `std::sys` module it will eventually back.
- **The substrate is newlib, and `std` already targets one.** Rust's Unix PAL has a `target_os = "horizon"` branch for the 3DS — devkitPro + newlib + libsysbase, the same C substrate used here. It works because devkitPro supplies libsysbase's implementation of that interface. Providing the same interface in Rust is what puts a Switch `std` within reach.
- **Prefer the shape `std` expects.** Where a design choice is open, pick the one the Unix PAL can consume unmodified (integer file descriptors, `errno` conventions, POSIX-shaped signatures) over a more elegant Rust-native abstraction that `std` would have to be taught about. Novel abstractions belong above the PAL boundary, not below it.

This is why the C-FFI surface is not merely a compatibility shim: the interface that lets existing C homebrew link against these crates is the same interface a future `std` port consumes.

**How it works**:

1. Rust crates implement Switch OS functionality (memory, threads, sync primitives, etc.)
2. Each crate has a public `ffi` module with C-compatible functions (`__nx_*` prefix)
3. `nx-std` is the only staticlib; it re-exports FFI symbols from enabled crates via `src/ffi.rs`
4. Linker scripts (`*_override.ld`) redirect `libnx` symbols to Rust implementations
5. At link time, code calling `libnx` functions transparently uses the Rust implementations

**Configuration**: Meson setup-time options (`use_nx_alloc`, `use_nx_svc`, etc.) control which crates are enabled, selecting corresponding Cargo features for `nx-std`.

## Quick Start

**If you're an AI agent working on this codebase, here's what you need to know immediately:**

1. **Invoke `/code-rules` FIRST** → Before planning OR coding (applies in Plan mode too), load the `docs/code/` rules that govern the affected crate(s). Skipping this leads to plans that violate project conventions and cause rework.
2. **Use Skills for operations** → Invoke skills (`/code-format`, `/code-check`, `/code-build`, `/code-test`, `/code-deploy`, `/code-rules-check`, `/code-review`) instead of running commands directly.

3. **Skills wrap justfile/meson tasks** → Skills provide the interface to `just` and `meson` commands with proper guidance.
4. **Follow the workflow** → Format → Check → Clippy → Build → Hardware Test (when applicable) → Rules check.

5. **Fix ALL warnings** → Zero tolerance for clippy warnings.

**Your first action**: Invoke `/code-rules` to load the governing rules before drafting a plan or writing any code. For commands, invoke the relevant Skill.

**Testing default**: Hardware tests run only after `/code-format` and `/code-check` are green. Tests are compiled as Switch homebrew (NRO) and validated on-device or emulator via `/code-deploy` + `/code-test`.

## Table of Contents

1. [Principles](#1-principles) — Core design principles guiding this codebase
2. [Code Rules](#2-code-rules) — Understanding coding standards via the `/code-rules` skill
3. [Architecture](#3-architecture) — Crate hierarchy, FFI integration, libnx integration
4. [Build System](#4-build-system) — Hybrid Meson + Cargo, prerequisites, cross-compilation
5. [Development Workflow](#5-development-workflow) — How to develop with this codebase
6. [Additional Resources](#6-additional-resources) — Links to documentation


## 1. Principles

The `principle-*` documents in `docs/code/` are the design rules. Their `/code-rules` catalog descriptions
state each rule in full, so loading the catalog gives you all of them; read a full document when you need its
examples or its Pragmatism Caveat to argue a decision.

Use `/code-rules principles` to read them all and summarise.


## 2. Code Rules

The code rules live in `docs/code/`, with YAML frontmatter for dynamic discovery. "Code rules" and "code
guidelines" name the same corpus.

**Rule documents are authoritative**: they define how code should be written. All implementations MUST follow
them. If code diverges from a rule, either fix the code or update the rule (with team approval).

### Rule Types

| Type               | Scope          | Purpose                                                          |
|--------------------|----------------|------------------------------------------------------------------|
| **Principle**      | `global`       | Universal software principles and best practices                 |
| **Core**           | `global`       | Fundamental coding standards (error handling, modules, docs)     |
| **Architectural**  | `global`       | High-level patterns (workspace structure, crate layout)          |
| **Pattern**        | `global`       | Reusable design patterns (builder, newtype, typestate)           |
| **Crate-specific** | `crate:<name>` | Rules for specific crates (nx-svc, nx-alloc, etc.)               |
| **Meta**           | `global`       | Documentation format specifications (`docs/__meta__/`)           |

### Skill Invocation

| When You Need To                                                  | Invoke This Skill    |
|-------------------------------------------------------------------|----------------------|
| Load the code rules before implementing                           | `/code-rules`        |
| "How should I handle errors?", "What's the pattern for X?"        | `/code-rules`        |
| Load crate-specific rules for `nx-svc`, `nx-alloc`, etc.          | `/code-rules`        |
| Check a finished changeset against the rules                      | `/code-rules-check`  |
| Deep review before a PR (bugs, regressions, soundness)            | `/code-review`       |
| Validate a rule document's own format                             | `/docs-fmt-check`    |

**Navigation:**

- Need to understand a convention? → `/code-rules`
- All rule documents live in `docs/code/`
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
    ├── nx-svc         - Supervisor calls (SVC) interface to Horizon OS
    ├── nx-cpu         - CPU utilities
    ├── nx-sys-mem     - Low-level memory management
    ├── nx-sys-sync    - Low-level synchronization primitives
    └── nx-sys-virtmem - Heap-free virtual-memory page substrate (reservation bitmap)
```

### Dependency Flow

`nx-svc` is the foundation — it provides raw SVC bindings. Higher-level crates build on it:

- `nx-alloc` depends on `nx-svc`, `nx-sys-sync`
- `nx-sys-virtmem` depends on `nx-svc`, `nx-rand`, `nx-sys-sync` — the heap-free page substrate; `nx-alloc` is deliberately absent from its graph so heap-freedom is compiler-enforced
- `nx-sys-mem` depends on `nx-alloc`, `nx-svc`, `nx-sys-virtmem`
- `nx-std` optionally depends on `nx-sys-virtmem` (folded into its `sys-mem` feature) to re-export the `virtmem` FFI directly

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

The project targets `aarch64-nintendo-horizon.json`, the repo-local target spec. Cargo configuration in `.cargo/config.toml` enables:

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
| **Code rules** (`docs/code/`)  | **HOW** (implementation) | Code implementation rules and standards (see [Code Rules](#2-code-rules)). Answers "How do I write quality, conventional code?"|

**Navigation Guide for AI Agents:**

- Need to understand the project? → Read this file (AGENTS.md)
- Need to run a command? → Invoke the appropriate Skill (`/code-format`, `/code-check`, `/code-build`, `/code-test`, `/code-deploy`)
- Need to write code? → Use `/code-rules` to load the governing rules

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
- **Code rules**: `/code-rules` — Load the code rules that govern the work at hand
- **Rules check**: `/code-rules-check` — Check a finished changeset against those rules
- **Doc format**: `/docs-fmt-check` — Validate a rule document's own format
- **Reviewing**: `/code-review` — Deep review for bugs, regressions, security, and soundness

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
2. **🚨 MANDATORY: Load the code rules FIRST** — Invoke `/code-rules` before drafting any plan or writing any code. This applies equally in Plan mode: the plan itself MUST be grounded in the loaded rules, not in assumptions about conventions.
3. **Follow crate-specific guidelines** — Guideline discovery loads crate-specific and core guidelines automatically
4. **Rationale** — Skipping this step leads to plans that violate conventions (e.g., module layout, error handling patterns, FFI surface design), causing avoidable rework.

### Typical Development Workflow

**Follow this workflow when implementing features or fixing bugs:**

#### 1. Research Phase

- Understand the codebase and existing guidelines
- Identify related modules and dependencies (use the [Crate Hierarchy](#crate-hierarchy))
- Review test files and usage examples in `subprojects/tests/`
- Use `/code-rules` to load the rules that govern the work

#### 2. Planning Phase

**🚨 MANDATORY FIRST STEP (including in Plan mode):** Invoke `/code-rules` to load the rules for the affected crate(s) BEFORE drafting the plan. The plan's structure, module layout, error handling, and type design decisions MUST reflect the loaded rules. Never draft a plan from assumptions about project conventions.

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
- [ ] Step 1: Write code following the rules (use /code-rules)
- [ ] Step 2: Format code (use /code-format skill)
- [ ] Step 3: Check compilation (use /code-check skill)
- [ ] Step 4: Fix all compilation errors
- [ ] Step 5: Run clippy (use /code-check skill)
- [ ] Step 6: Fix ALL clippy warnings
- [ ] Step 7: Build artifacts when applicable (use /code-build skill)
- [ ] Step 8: Run hardware tests when warranted (use /code-test skill)
- [ ] Step 9: Check the changeset against the rules (use /code-rules-check skill)
- [ ] Step 10: Fix every violation, or record why the deviation is deliberate
- [ ] Step 11: All required checks pass ✅
```

**Detailed workflow for each work chunk (and before committing):**

1. **Write code** following the rules from [Code Rules](#2-code-rules) (loaded via `/code-rules`)

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

7. **Rules check (once the implementation is complete)**:
   - **Use**: `/code-rules-check` skill — it checks the changeset against `docs/code/` and reports each violation with the document behind it.
   - **Must pass**: Fix every finding, or record why the deviation is deliberate.

8. **Iterate**: If any validation fails → fix → return to step 2

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
    /code-rules-check skill (audit the changeset)
          ↓              ↓ Violation?
    All Pass ✅     (loop back to /code-format)
```

**Remember**: Invoke Skills for all operations. If unsure which skill to use, refer to the Command-Line Operations Reference above.

#### 4. Completion Phase

- Ensure all required checks pass (format, check, clippy, build, any hardware tests run, and `/code-rules-check`)
- If hardware tests were skipped, document why and the risk assessment
- Use `/code-review` when the change is large or risky enough to need bugs, regressions, security, and soundness examined as well
- Document any warnings you couldn't fix and why

### Core Development Principles

**ALL AI agents MUST follow these principles:**

- **Research → Plan → Implement**: Never jump straight to coding
- **Rules before planning**: `/code-rules` is a prerequisite for both planning AND Plan mode, not just implementation. A plan written without the loaded rules is considered incomplete.
- **Rule compliance**: Follow the rules from [Code Rules](#2-code-rules), and audit the result with `/code-rules-check`
- **Zero tolerance for errors**: All automated checks must pass
- **Clarity over cleverness**: Choose clear, maintainable solutions
- **FFI safety first**: The C/Rust boundary is unforgiving — uphold soundness invariants and validate at the edge

**Essential conventions:**

- **Maintain type safety**: Leverage Rust's type system fully (see [Type-Driven Design](docs/code/principle-type-driven-design.md))
- **No-std discipline**: Code targets `aarch64-nintendo-horizon.json` — no `std`, `panic = "abort"`
- **Validate at FFI boundaries**: Hard shell on the C-facing surface (`__nx_*`); soft core inside (see [Validate at the Edge](docs/code/principle-validate-at-edge.md))
- **Format code before checks/commit**: Use `/code-format` skill
- **Fix all warnings**: Use `/code-check` skill for clippy
- **Test FFI changes on hardware**: Use `/code-test` skill after checks are green

### Summary: Key Takeaways for AI Agents

| What                | Where                                  | When                                                  |
|---------------------|----------------------------------------|-------------------------------------------------------|
| **Plan work**       | `/code-rules`                          | BEFORE creating any plan                              |
| **Run commands**    | `.agents/skills/`                      | Check Skills BEFORE any command                       |
| **Write code**      | [Code Rules](#2-code-rules)            | Load the rules before implementation                  |
| **Format**          | `/code-format`                         | Before checks or before committing                    |
| **Check**           | `/code-check`                          | After formatting                                      |
| **Lint**            | `/code-check`                          | Fix ALL warnings                                      |
| **Build artifacts** | `/code-build`                          | When NRO/NSP outputs are needed                       |
| **Deploy**          | `/code-deploy`                         | Push built NRO to the Switch                          |
| **Test (hardware)** | `/code-test`                           | Only after checks green; on FFI/foundation changes    |
| **Rules check**     | `/code-rules-check`                    | Once the implementation is complete                   |
| **Doc format**      | `/docs-fmt-check`                      | After editing `docs/code/` or `docs/__meta__/`        |
| **Review**          | `/code-review`                         | Before commits / PRs, when the change is large or risky |

**Golden Rules:**

1. ✅ Invoke Skills for all common operations
2. ✅ Skills wrap just/meson tasks with proper guidance
3. ✅ Follow the workflow: Format → Check → Clippy → Build → Hardware Test (when needed) → Rules check
4. ✅ Zero tolerance for errors and warnings
5. ✅ Every change improves the codebase

**Remember**: When in doubt, invoke the appropriate Skill!


## 6. Additional Resources

For more detailed information about the project:

- **Build system**: See [`docs/build_system.md`](docs/build_system.md)
- **libnx symbol overrides**: See [`docs/libnx_overrides.md`](docs/libnx_overrides.md)
- **Code rules**: Browse `docs/code/` (load via `/code-rules`)
- **Documentation format specs**: See `docs/__meta__/`
- **Tests**: C tests in `subprojects/tests/` link against Rust crates to verify FFI correctness
