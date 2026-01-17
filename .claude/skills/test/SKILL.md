---
name: test
description: Run the test suite by building tests and deploying to Nintendo Switch. Use for running tests, verifying changes on hardware, or validating implementations.
allowed-tools: Bash(just --list:*), Bash(just build-tests:*)
---

# Test Skill

This skill orchestrates the full test workflow for the nx-std project. Tests run on actual Nintendo Switch hardware.

## When to Use This Skill

Use this skill when you need to:

- Validate code changes on real hardware
- Run the test suite after implementing features
- Verify FFI correctness between Rust and C code
- Confirm changes before creating a PR

## Workflow

This skill orchestrates the full test workflow by invoking other skills:

### Step 1: Build Tests

Use the `/build` skill to build the test NRO:

```bash
just build-tests
```

This compiles the test suite into `buildDir/subprojects/tests/nx-tests.nro`.

### Step 2: Deploy to Switch

Use the `/deploy` skill to deploy the test NRO to the Nintendo Switch:

```bash
just deploy buildDir/subprojects/tests/nx-tests.nro
```

The deploy skill handles network transfer to the Switch via cargo-nx.

### Step 3: Verify Results

🚨 **MANDATORY**: Ask the user to confirm the tests passed on the console.

Do NOT assume tests passed. The test output is only visible on the Switch screen.

## Automatic Orchestration

When you invoke `/test`, you should:

1. First invoke `/build` skill with `just build-tests`
2. Then invoke `/deploy` skill with the built NRO path
3. Finally ask the user to confirm test results

This ensures the complete test workflow is automated.

## Test Architecture

Tests are C code that link against Rust crates to verify FFI correctness. Located in `subprojects/tests/`:

- `source/main.c` - Test harness entry point
- `source/harness.h` - Test framework macros
- `source/sync/` - Synchronization primitive tests
- `source/rand/` - Random number generation tests

## Prerequisites

See `/deploy` skill for:

- Nintendo Switch setup requirements
- Network connectivity requirements
- cargo-nx installation

## Related Skills

- `/build` - Building targets (including `just build-tests`)
- `/deploy` - Deploying to Switch hardware
- `/format` - Formatting code before testing
