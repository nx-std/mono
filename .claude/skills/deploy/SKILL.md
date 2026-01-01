---
name: deploy
description: Deploy build artifacts (e.g., NRO files) to a Nintendo Switch using just deploy. Use when deploying built homebrew to the console for testing.
allowed-tools: Bash(just --list:*), Bash(just deploy:*)
---

# Deploy Skill

This skill provides deployment operations for sending build artifacts to a Nintendo Switch console.

## When to Use This Skill

Use this skill when you need to:

- Deploy build artifacts (e.g., NRO files) to a Nintendo Switch for testing
- Send test builds to the console after compilation
- Verify that built homebrew runs correctly on hardware

## Prerequisites

1. **Nintendo Switch** must be running a homebrew environment (e.g., Atmosphère)
2. **nxlink** server must be running on the Switch (usually via hbmenu's netloader)
3. **Network connectivity** between development machine and Switch
4. **cargo-nx** must be installed (see installation section below)

## Available Commands

### Deploy Build Artifact

```bash
just deploy <path-to-file>
```

Deploys a build artifact to the Nintendo Switch via `cargo nx link`.

### Examples

**Deploy hbmenu**:

```bash
just deploy buildDir/subprojects/nx-hbmenu/hbmenu.nro
```

**Deploy test suite**:

```bash
just deploy buildDir/subprojects/tests/nx-tests.nro
```

**Deploy with extra flags**:

```bash
just deploy buildDir/subprojects/tests/nx-tests.nro --address 192.168.1.100
```

### Install cargo-nx

If `cargo nx` is not available, **ask the user to install it**:

```bash
just install-cargo-nx
```

🚨 **Do NOT run this command yourself. Ask the user to run it manually.**

This builds and installs cargo-nx from the submodule.

## Common Deployment Paths

After building with `just meson-compile`, build artifacts are located at:

| Target   | Path                                        |
|----------|---------------------------------------------|
| hbmenu   | `buildDir/subprojects/nx-hbmenu/hbmenu.nro` |
| nx-tests | `buildDir/subprojects/tests/nx-tests.nro`   |

## Workflow

**Complete build-and-deploy workflow:**

1. **Build the target**:
   ```bash
   just meson-compile hbmenu.nro
   ```

2. **Deploy to Switch**:
   ```bash
   just deploy buildDir/subprojects/nx-hbmenu/hbmenu.nro
   ```

3. **Ask user for confirmation**:
    - For general deployments: Ask user to confirm the deployment worked as expected
    - For test deployments: Ask user to confirm the tests PASSED on the console

## Retry on Failure

🚨 **If deployment fails (e.g., Switch not found on network), retry up to 3 times with a 10-second delay between
attempts.**

```bash
# Attempt 1
just deploy buildDir/path/to/file.nro

# If failed, wait 10 seconds and retry
sleep 10
just deploy buildDir/path/to/file.nro

# If failed again, wait 10 seconds and retry one more time
sleep 10
just deploy buildDir/path/to/file.nro
```

Common reasons for deployment failure:

- Switch not connected to the network
- nxlink server not running on the Switch
- Network timeout or connectivity issues

## Troubleshooting

### Connection Issues

- Ensure Switch and development machine are on the same network
- Check that nxlink server is running on the Switch

### cargo-nx Not Found

If you see an error about `cargo nx` not being found, **ask the user to run**:

```bash
just install-cargo-nx
```

🚨 **Do NOT run this command yourself.**

## Next Steps

Before deploying, ensure your build is complete:

1. **Build targets** → See `.claude/skills/build/SKILL.md`
2. **Format code** → See `.claude/skills/format/SKILL.md`