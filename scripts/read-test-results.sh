#!/usr/bin/env bash
# Reads a finished nx-tests run's results out of the console's memory.
#
# The console draws its text straight to the framebuffer and keeps no copy, so
# the harness records each result in `g_test_results` instead. Nothing in the
# process reads that table; this does, over Atmosphere's GDB stub, after the run
# is over. There is no timing pressure and no breakpoint involved.
#
# Usage: scripts/read-test-results.sh <console-ip> <path-to-elf>
set -euo pipefail

IP="${1:?console ip}"
ELF="${2:?path to elf}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

GDB=(just gdb --batch -x)
NM=/opt/devkitpro/devkitA64/bin/aarch64-none-elf-nm

# Static offsets of the table and its count, as linked.
# `harness.h` declares the table `static`, so every translation unit that
# includes it emits its own copy and only the one holding the suite functions is
# ever filled. Collect every candidate pair and pick the populated one below.
mapfile -t COUNT_OFFS < <("$NM" "$ELF" | awk '$3=="g_test_result_count"{print "0x"$1}')
mapfile -t TABLE_OFFS < <("$NM" "$ELF" | awk '$3=="g_test_results"{print "0x"$1}')
[ "${#COUNT_OFFS[@]}" -gt 0 ] && [ "${#TABLE_OFFS[@]}" -gt 0 ] || { echo "symbols missing from $ELF" >&2; exit 1; }
[ "${#COUNT_OFFS[@]}" -eq "${#TABLE_OFFS[@]}" ] || { echo "table and count copies disagree" >&2; exit 1; }

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
PID=$(printf '%s\n' "$PROBE" | awk '/hbloader/{print $1; exit}')
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
BASE=$("${GDB[@]}" "$TMP/mods.gdb" 2>&1 || true \
  | awk -v m="$MODULE" 'index($0, m){print $1; exit}')
[ -n "$BASE" ] || { echo "$MODULE is not loaded on the console" >&2; exit 1; }

# Read each copy's count and keep the first that recorded anything.
COUNT_ADDR=""
for i in "${!COUNT_OFFS[@]}"; do
  probe=$(printf '0x%x' $(( BASE + ${COUNT_OFFS[$i]} )))
  cat > "$TMP/count.gdb" <<EOF
set pagination off
set confirm off
set architecture aarch64
handle SIGTRAP nostop noprint pass
target extended-remote $IP:22225
attach $PID
printf "N %d\n", *(int *)$probe
detach
disconnect
EOF
  n=$("${GDB[@]}" "$TMP/count.gdb" 2>&1 | awk '/^N /{print $2; exit}')
  if [ -n "$n" ] && [ "$n" -gt 0 ] 2>/dev/null; then
    COUNT_ADDR="$probe"
    TABLE_ADDR=$(printf '0x%x' $(( BASE + ${TABLE_OFFS[$i]} )))
    break
  fi
done
[ -n "$COUNT_ADDR" ] || { echo "no run recorded any results" >&2; exit 1; }

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

"${GDB[@]}" "$TMP/read.gdb" 2>&1 | awk '
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
  }'
