---
name: code-deploy
description: Deploy build artifacts (e.g., NRO files) to a Nintendo Switch via cargo-nx link. Use when sending built homebrew to the console for testing or hardware validation.
allowed-tools: "Bash(just --list:*), Bash(just deploy:*)"
---

# Code Deploy Skill

Deploy NRO/NSP artifacts to a Nintendo Switch console via `cargo nx link` (nxlink).

## When to Use This Skill

- Send a build artifact (NRO) to the Switch for testing.
- Deploy the nx-tests NRO as part of a hardware-test workflow.
- Verify built homebrew runs correctly on real hardware.

## Prerequisites

1. **Switch** running a homebrew environment (e.g., Atmosphère).
2. **nxlink server** running on the Switch (typically launched via hbmenu's netloader).
3. **Network connectivity** between dev machine and Switch.
4. **`cargo-nx`** installed. If missing, **ask the user** to run `just install-cargo-nx` — do NOT run it yourself.

## Workflow

### Step 1 — Build the artifact

Use `/code-build` to compile the target first. Common outputs:

| Target   | Path                                         |
|----------|----------------------------------------------|
| hbmenu   | `buildDir/subprojects/nx-hbmenu/hbmenu.nro`  |
| nx-tests | `buildDir/subprojects/tests/nx-tests.nro`    |

### Step 2 — Deploy

```bash
just deploy <path-to-file.nro>
```

Examples:
```bash
just deploy buildDir/subprojects/nx-hbmenu/hbmenu.nro
just deploy buildDir/subprojects/tests/nx-tests.nro
just deploy buildDir/subprojects/tests/nx-tests.nro --address 192.168.1.100
```

### Step 3 — Confirm with the user

🚨 **Always ask the user to confirm** the deployment ran as expected on the console:
- General deploys: ask whether the homebrew launched and behaved correctly.
- Test deploys: ask whether the test suite **PASSED** on the console screen.

**Do NOT assume success** — output is only visible on the Switch.

## Retry on Failure

If deployment fails (Switch not on network, nxlink down), **retry up to 3 times with a 10-second delay** between attempts:

```bash
just deploy buildDir/path/to/file.nro
sleep 10
just deploy buildDir/path/to/file.nro
sleep 10
just deploy buildDir/path/to/file.nro
```

Common failure causes:
- Switch not connected to network.
- nxlink server not running on the Switch.
- Network timeout.

If three attempts fail, surface the error to the user — do not loop indefinitely.

## Anti-patterns

- **Never run `just install-cargo-nx` yourself** — ask the user to run it.
- **Never claim deployment succeeded** without on-console confirmation.
- **Never invoke `cargo nx link` directly** — use `just deploy` (the recipe handles flags).
- **Never deploy an artifact that hasn't been freshly rebuilt** if you just edited code — go through `/code-build` first.

## Pre-approved Commands

Runnable without user permission:
- `just --list` — read-only introspection.
- `just deploy <path>` — sends the artifact via nxlink; no other side effects on the dev machine.

**Not pre-approved**: `just install-cargo-nx` (mutates the host's cargo install state — ask the user).

## Related Skills

- `/code-build` — Build artifacts before deploying.
- `/code-format` — Format code before building.
- `/code-test` — Full hardware-test workflow (build + deploy + confirm).
