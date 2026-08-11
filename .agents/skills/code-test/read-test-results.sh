#!/usr/bin/env bash
# Reads a finished nx-tests run's results out of the console's memory.
#
# The console draws its text straight to the framebuffer and keeps no copy, so
# the harness records each result in `g_test_results` instead. Nothing in the
# process reads that table; this does, over Atmosphere's GDB stub, after the run
# is over. There is no timing pressure and no breakpoint involved.
#
# Usage: read-test-results.sh <console-ip> <path-to-elf>
#
# Works for any harness-based test binary: nx-tests, nx-tests-sync, nx-tests-fs,
# nx-tests-net. The interactive applet binaries have no test cases and record
# nothing.
set -euo pipefail

IP="${1:?console ip}"
ELF="${2:?path to elf}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

GDB=(just gdb --batch -x)
NM=/opt/devkitpro/devkitA64/bin/aarch64-none-elf-nm

# Static offsets of the table and its count, as linked.
# `harness.h` declares the table and each binary's `main.c` defines it exactly
# once via `TEST_RESULTS_STORAGE`, so there is one address to find.
# The symbol table is read once and filtered in-process: `awk` exits at the
# first match, and closing the pipe under it would kill `nm` with SIGPIPE and
# trip `pipefail` before anything is printed.
SYMS=$("$NM" "$ELF")
COUNT_OFF=$(awk '$3=="g_test_result_count"{print "0x"$1; exit}' <<<"$SYMS")
TABLE_OFF=$(awk '$3=="g_test_results"{print "0x"$1; exit}' <<<"$SYMS")
[ -n "$COUNT_OFF" ] && [ -n "$TABLE_OFF" ] || {
  echo "$(basename "$ELF") records no results — is it a harness-based suite built from this tree?" >&2
  exit 1
}

# The NRO is loaded at a fresh base every launch, so it has to be read back
# rather than assumed.
cat > "$TMP/probe.gdb" <<EOF
set pagination off
set confirm off
set architecture aarch64
target extended-remote $IP:22225
info os processes
disconnect
EOF
# `set -e` must not swallow a failed probe: gdb exits non-zero when the console
# is unreachable, and the reason it printed is the useful part.
PROBE=$("${GDB[@]}" "$TMP/probe.gdb" 2>&1 || true)
case "$PROBE" in
  *"No route to host"*|*"Connection refused"*|*"Connection timed out"*)
    echo "cannot reach the console at $IP:22225 — is it awake, and is the GDB stub enabled?" >&2
    exit 1
    ;;
esac
PID=$(awk '/hbloader/{print $1; exit}' <<<"$PROBE")
[ -n "$PID" ] || { echo "hbloader is not running — launch hbmenu or an NRO first" >&2; exit 1; }

cat > "$TMP/mods.gdb" <<EOF
set pagination off
set confirm off
set architecture aarch64
handle SIGTRAP nostop noprint pass
target extended-remote $IP:22225
attach $PID
monitor get modules
detach
disconnect
EOF
# The module is named after the ELF being read, so a caller pointing at one
# binary never picks up another's base by accident.
MODULE=$(basename "$ELF")
# `|| true` binds tighter than the pipe, so extracting the base has to be a
# separate step: `cmd || true | awk` reads as `cmd || (true | awk)` and leaves
# the whole gdb transcript in `BASE`.
MODS=$("${GDB[@]}" "$TMP/mods.gdb" 2>&1 || true)
BASE=$(awk -v m="$MODULE" 'index($0, m){print $1; exit}' <<<"$MODS")
[ -n "$BASE" ] || { echo "$MODULE is not loaded on the console" >&2; exit 1; }

COUNT_ADDR=$(printf '0x%x' $(( BASE + COUNT_OFF )))
TABLE_ADDR=$(printf '0x%x' $(( BASE + TABLE_OFF )))

# One entry is a `const char*` title followed by an `int` result, padded to 16.
cat > "$TMP/read.gdb" <<EOF
set pagination off
set confirm off
set architecture aarch64
handle SIGTRAP nostop noprint pass
target extended-remote $IP:22225
attach $PID
printf "COUNT %d\n", *(int *)$COUNT_ADDR
set \$i = 0
while \$i < *(int *)$COUNT_ADDR
  printf "RESULT %d %s\n", *(int *)($TABLE_ADDR + \$i * 16 + 8), *(char **)($TABLE_ADDR + \$i * 16)
  set \$i = \$i + 1
end
detach
disconnect
EOF

READ_OUT=$("${GDB[@]}" "$TMP/read.gdb" 2>&1 || true)
awk '
  /^COUNT/ { total = $2; next }
  /^RESULT/ {
    rc = $2
    title = ""
    for (i = 3; i <= NF; i++) title = title (i > 3 ? " " : "") $i
    if (rc == 0)          verdict = "OK"
    else if (rc == -501)  verdict = "TODO"
    else if (rc == -502)  verdict = "SKIPPED"
    else if (rc == -503)  verdict = "SETUP FAILED"
    else                  verdict = sprintf("FAILED (%d)", rc)
    printf "%-68s %s\n", title, verdict
    if (rc != 0 && rc != -502 && rc != -501) failed++
  }
  END {
    printf "\n%d recorded, %d failed\n", total, failed + 0
  }' <<<"$READ_OUT"
