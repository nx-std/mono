# Updating the devkitPro newlib Patch in `subprojects/sysroot`

`subprojects/sysroot` is a manually-cloned `newlib-cygwin` working tree (not a git
submodule) tracked at <https://github.com/nx-std/newlib-cygwin>. It carries a
single devkitPro patch on top of an upstream newlib snapshot, plus a stable tag
that the build system consumes.

This document describes how to roll the patch forward when devkitPro publishes a
new newlib release.

## Repository layout

The sysroot repo has two remotes:

- `origin` → `git@github.com:nx-std/newlib-cygwin.git` (the fork the project
  consumes)
- `upstream` → `https://sourceware.org/git/newlib-cygwin.git` (canonical newlib
  source)

Naming schema (kept consistent across releases):

| Ref kind                | Pattern                              | Example                                |
|-------------------------|--------------------------------------|----------------------------------------|
| Per-release branch      | `devkitpro-newlib-X.Y.Z.YYYYMMDD`    | `devkitpro-newlib-4.6.0.20260123`      |
| Per-release tag         | `newlib-X.Y.Z.YYYYMMDD-devkitpro`    | `newlib-4.6.0.20260123-devkitpro`      |
| Rolling pointer branch  | `devkitpro`                          | always points to the latest patch tip  |

`X.Y.Z.YYYYMMDD` is the devkitPro `NEWLIB_VER` (newlib upstream version + the
snapshot date). The branch is rooted at the corresponding upstream annotated
tag `newlib-X.Y.Z` and contains a single commit applying the devkitPro patch.

The patch filename in `devkitPro/buildscripts` carries an additional pkgrel
suffix (e.g. `newlib-4.6.0.20260123-4.patch`); the pkgrel is **not** encoded in
the branch or tag name, only in the commit message.

## When to update

Whenever <https://github.com/devkitPro/buildscripts> ships a new
`patches/newlib-*.patch` for the `devkitA64` toolchain (the Switch target).

To check the current pinned version, look at `select_toolchain.sh` in
`devkitPro/buildscripts`, in the `case 3` block (devkitA64):

```sh
NEWLIB_VER=4.6.0.20260123
NEWLIB_PKGREL=4
```

The combined patch file is therefore `newlib-${NEWLIB_VER}-${NEWLIB_PKGREL}.patch`.

## Procedure

All commands run from the repo root, against `subprojects/sysroot`. Replace the
example version (`4.6.0.20260123`, pkgrel `4`) with the new values.

### 1. Refresh remotes

```sh
git -C subprojects/sysroot fetch upstream --tags
git -C subprojects/sysroot fetch origin
```

If the local clone was shallow (it ships shallow by default), unshallow it now;
GitHub will reject pushes from a shallow clone with a missing-object error:

```sh
git -C subprojects/sysroot fetch --unshallow upstream   # only if .git/shallow exists
```

### 2. Identify the base commit

devkitPro snapshots are rooted at the upstream `newlib-X.Y.Z` annotated tag,
which resolves to the upstream "Changes for X.Y.Z snapshot" commit:

```sh
git -C subprojects/sysroot rev-parse newlib-4.6.0^{commit}
git -C subprojects/sysroot log --oneline -1 newlib-4.6.0
# 8ba4275b8 Changes for 4.6.0 snapshot
```

Cross-check against the previous release to confirm the convention:

```sh
git -C subprojects/sysroot log --oneline -1 newlib-4.5.0
# 5e5e51f1d Changes for 4.5.0 snapshot   ← parent of devkitpro-newlib-4.5.0.20241231
```

### 3. Fetch the devkitPro patch

Pull the patch directly from `devkitPro/buildscripts` via `gh`:

```sh
mkdir -p /tmp/dkp
gh api repos/devkitPro/buildscripts/contents/patches/newlib-4.6.0.20260123-4.patch \
  | python3 -c "import sys,json,base64; sys.stdout.buffer.write(base64.b64decode(json.load(sys.stdin)['content']))" \
  > /tmp/dkp/newlib-4.6.0.20260123-4.patch
```

### 4. Create the per-release branch and apply the patch

```sh
git -C subprojects/sysroot checkout -b devkitpro-newlib-4.6.0.20260123 newlib-4.6.0^{commit}
git -C subprojects/sysroot apply --check /tmp/dkp/newlib-4.6.0.20260123-4.patch
git -C subprojects/sysroot apply        /tmp/dkp/newlib-4.6.0.20260123-4.patch
git -C subprojects/sysroot add -A
git -C subprojects/sysroot -c commit.gpgsign=false commit \
  -m "Apply devkitpro/buildscripts newlib-4.6.0.20260123-4.patch"
```

The pkgrel suffix (`-4`) is preserved in the commit message so the source patch
is unambiguous; the branch name itself omits it (see schema table).

Whitespace warnings from `git apply` mirror what the upstream patch contains —
do not edit them out.

### 5. Tag the patch tip

```sh
git -C subprojects/sysroot tag newlib-4.6.0.20260123-devkitpro
```

### 6. Move the rolling `devkitpro` branch

`devkitpro` always points to the latest patch tip. It is intentionally a
non-fast-forward move from one release branch to the next:

```sh
git -C subprojects/sysroot checkout devkitpro
git -C subprojects/sysroot reset --hard devkitpro-newlib-4.6.0.20260123
```

### 7. Push everything to `origin`

Push the new release branch, the new tag, and the upstream `newlib-X.Y.Z` tag
(so `origin` can serve the base independently of `upstream`):

```sh
git -C subprojects/sysroot push origin \
  devkitpro-newlib-4.6.0.20260123 \
  newlib-4.6.0.20260123-devkitpro \
  newlib-4.6.0
```

Force-update the rolling pointer with `--force-with-lease` so a stale local
view cannot clobber concurrent work:

```sh
PREV=$(git -C subprojects/sysroot rev-parse origin/devkitpro)
git -C subprojects/sysroot push --force-with-lease=devkitpro:"$PREV" origin devkitpro
```

## Verification

After pushing, confirm the remote state:

```sh
git -C subprojects/sysroot ls-remote origin | grep -E 'devkitpro|newlib-4'
```

Expected: `refs/heads/devkitpro` and `refs/heads/devkitpro-newlib-X.Y.Z.YYYYMMDD`
both resolve to the new patch commit, and the matching tag exists.

## Notes

- **pkgrel bumps**: if devkitPro publishes a new pkgrel for the *same* snapshot
  date, the current schema has no slot to express it. Decide per-incident: roll
  the existing release branch forward (force-update branch and tag) or extend
  the schema. Document the choice in the commit message.
- **Multiple newlib patches in `buildscripts/patches/`**: that repo holds
  patches for all three toolchains (devkitARM/PPC/A64). Always pull the
  filename matched by `select_toolchain.sh` `case 3` (devkitA64), not the
  highest-numbered file.
- **Never** delete or rewrite published per-release branches/tags; they are the
  audit trail. Only the rolling `devkitpro` branch is force-updated.
