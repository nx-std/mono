#!/usr/bin/env bash
# Shared helpers for driving a running process on the console through Atmosphère's
# `dmnt.gen2` GDB stub.
#
# The stub serves one client at a time, so every helper here opens its own session and
# disconnects before returning. Sourcing this file gives you:
#
#   console_pid <ip> <process>          the process's PID, e.g. `hbloader`
#   module_base <ip> <pid> <module>     where a module is loaded this launch (ASLR moves it)
#   symbol_offset <elf> <symbol>        a symbol's file offset, to be added to the base
#   gdb_batch <script>                  run a GDB command file, returning its output
#
# Every one of them writes diagnostics to stderr and the answer alone to stdout, so a
# caller can capture it with `$( )`.

set -euo pipefail

# The repository root, so the `just` recipes resolve wherever a caller runs this from.
GDB_LIB_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

# Port Atmosphère's standalone GDB stub listens on.
readonly GDB_STUB_PORT=22225

# Runs a GDB command file against the console, returning everything GDB printed.
#
# Always a command file rather than inline `-ex`: the `just gdb` recipe expands its
# arguments unquoted, so a multi-word `-ex` would be split by the shell.
gdb_batch() {
    local script="$1"

    just --justfile "${GDB_LIB_REPO_ROOT}/justfile" \
         --working-directory "${GDB_LIB_REPO_ROOT}" \
         gdb --batch -x "${script}" 2>&1
}

# Fails unless the console is reachable and the stub is listening.
require_stub() {
    local ip="$1"

    if ! timeout 5 bash -c "</dev/tcp/${ip}/${GDB_STUB_PORT}" 2>/dev/null; then
        echo "error: nothing is listening on ${ip}:${GDB_STUB_PORT}." >&2
        echo "       The console may be asleep (Wi-Fi drops with it), or the stub is off:" >&2
        echo "       atmosphere/config/system_settings.ini needs enable_standalone_gdbstub=u8!0x1" >&2
        echo "       and enable_htc=u8!0x0, followed by a reboot." >&2
        return 1
    fi
}

# Prints the PID of the named process.
#
# Homebrew launched from hbmenu runs inside `hbloader`, so that is the process to ask for
# rather than the NRO's own name. The PID changes on every launch, so this is re-probed
# rather than remembered.
console_pid() {
    local ip="$1" process="$2"
    local script output pid

    script="$(mktemp)"
    cat > "${script}" <<EOF
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote ${ip}:${GDB_STUB_PORT}
info os processes
disconnect
EOF

    output="$(gdb_batch "${script}")"
    rm -f "${script}"

    # The process list is one `<pid> <name>` row per line.
    pid="$(awk -v want="${process}" '$2 == want { print $1; exit }' <<< "${output}")"
    if [[ -z "${pid}" ]]; then
        echo "error: no process named '${process}' is running on ${ip}." >&2
        return 1
    fi

    echo "${pid}"
}

# Prints the address the named module is loaded at, as `0x...`.
#
# Modules are relocated on every launch, so a symbol's runtime address is this plus its
# file offset and nothing may be cached across launches.
module_base() {
    local ip="$1" pid="$2" module="$3"
    local script output base

    script="$(mktemp)"
    cat > "${script}" <<EOF
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote ${ip}:${GDB_STUB_PORT}
attach ${pid}
monitor get modules
detach
disconnect
EOF

    output="$(gdb_batch "${script}")"
    rm -f "${script}"

    # Each module is reported as `<base> - <end> <name>`.
    base="$(awk -v want="${module}" '$4 == want { print $1; exit }' <<< "${output}")"
    if [[ -z "${base}" ]]; then
        echo "error: '${module}' is not loaded in process ${pid}." >&2
        echo "       Modules found:" >&2
        sed -n '/Modules:/,/^[^ ]/p' <<< "${output}" >&2
        return 1
    fi

    echo "${base}"
}

# Prints the address range the named module is loaded over, as `<start> <end>`.
module_range() {
    local ip="$1" pid="$2" module="$3"
    local script output range

    script="$(mktemp)"
    cat > "${script}" <<EOF
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote ${ip}:${GDB_STUB_PORT}
attach ${pid}
monitor get modules
detach
disconnect
EOF

    output="$(gdb_batch "${script}")"
    rm -f "${script}"

    # Each module is reported as `<start> - <end> <name>`.
    range="$(awk -v want="${module}" '$4 == want { print $1, $3; exit }' <<< "${output}")"
    if [[ -z "${range}" ]]; then
        echo "error: '${module}' is not loaded in process ${pid}." >&2
        return 1
    fi

    echo "${range}"
}

# Prints the address of `padUpdate` in a loaded module, found by its own code.
#
# The alternative is an offset out of an ELF, which only holds when that ELF is the very
# build the console is running. It is for anything this repository deploys, and it is not
# for the homebrew menu on somebody's SD card — and an offset from the wrong build points
# at unrelated code, where a breakpoint simply never comes.
#
# What both builds do share is libnx, compiled the same way, so the function's first
# instructions are the same bytes wherever it ended up. Those bytes are what is searched
# for, which asks nothing of the layout around them.
scan_pad_update() {
    local ip="$1" pid="$2" start="$3" end="$4"
    local script output found

    # `padUpdate`'s opening instructions, little-endian. Two of them, because the function
    # comes from libnx and which libnx decides what it compiles to: a release build saves
    # registers and keeps the argument in one, while the build this repository produces
    # spills it to the stack. Searching for one and not the other finds nothing at all in
    # half the binaries worth pressing a button in.
    local patterns=(
        # devkitPro's prebuilt libnx, which a released homebrew menu links:
        #   stp x29,x30,[sp,#-128]! / mov x29,sp / stp x19,x20 / stp x21,x22 / mov x22,x0
        "0xfd, 0x7b, 0xb8, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0xf3, 0x53, 0x01, 0xa9, 0xf5, 0x5b, 0x02, 0xa9, 0xf6, 0x03, 0x00, 0xaa"
        # This repository's own build:
        #   stp x29,x30,[sp,#-112]! / mov x29,sp / str x0,[sp,#24] / ldr x0,[sp,#24]
        "0xfd, 0x7b, 0xb9, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0xe0, 0x0f, 0x00, 0xf9, 0xe0, 0x0f, 0x40, 0xf9, 0x1f, 0x04, 0x00, 0x39"
    )

    local pattern
    for pattern in "${patterns[@]}"; do
        script="$(mktemp)"
        cat > "${script}" <<EOF
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote ${ip}:${GDB_STUB_PORT}
attach ${pid}
find /b ${start}, ${end}, ${pattern}
detach
disconnect
EOF

        output="$(gdb_batch "${script}")"
        rm -f "${script}"

        if grep -qE '^0x[0-9a-f]+$' <<< "${output}"; then
            break
        fi
    done

    # A hit is a line holding an address and nothing else. Anchored at both ends on
    # purpose: attaching prints the address execution was stopped at, followed by the frame
    # it was in, and a search that reported that instead would be reporting the program
    # counter — a different number on every attach, and not a function at all.
    found="$(grep -E '^0x[0-9a-f]+$' <<< "${output}" | head -1)"
    if [[ -z "${found}" ]]; then
        echo "error: padUpdate's code was not found between ${start} and ${end}." >&2
        echo "       The module may not link libnx, or may link a different build of it." >&2
        return 1
    fi

    if [[ "$(grep -cE '^0x[0-9a-f]+$' <<< "${output}")" -gt 1 ]]; then
        echo "warning: the pattern matched more than once; using the first." >&2
    fi

    echo "${found}"
}

# Prints a symbol's file offset within `elf`, as `0x...`.
symbol_offset() {
    local elf="$1" symbol="$2"
    local offset

    if [[ ! -f "${elf}" ]]; then
        echo "error: no such ELF: ${elf}" >&2
        return 1
    fi

    offset="$(just --justfile "${GDB_LIB_REPO_ROOT}/justfile" \
                   --working-directory "${GDB_LIB_REPO_ROOT}" \
                   nm "${elf}" 2>/dev/null \
              | awk -v want="${symbol}" 'tolower($3) == tolower(want) { print $1; exit }')"

    if [[ -z "${offset}" ]]; then
        echo "error: '${symbol}' is not in ${elf}." >&2
        return 1
    fi

    echo "0x${offset}"
}
