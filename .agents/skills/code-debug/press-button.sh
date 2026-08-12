#!/usr/bin/env bash
# Presses a button in a program running on the console, without anybody touching it.
#
# Usage:
#   press-button.sh <ip> <button> [--elf <path>] [--module <name>] [--process <name>]
#
#   press-button.sh 192.168.1.129 Plus --elf buildDir/subprojects/tests/nx-tests.elf \
#                                      --module nx-tests.elf
#
# ## How the press is delivered
#
# Not through HID: the input a program sees arrives in shared memory the system writes
# continuously, so a value poked in there is overwritten before the program looks, and the
# layout is a sampled ring buffer rather than a set of button bits.
#
# What the program actually reads is its own `PadState`, which `padUpdate` refreshes once a
# frame and `padGetButtonsDown` then reduces to `~buttons_old & buttons_cur`. So the press
# is injected there, between the two: break where `padUpdate` returns, set the button in
# `buttons_cur` and clear it in `buttons_old`, and the very next `padGetButtonsDown` reports
# it as newly pressed. One frame later `padUpdate` overwrites both fields with what the
# controller really says, so the press lasts exactly one frame and needs no releasing.
#
# `padUpdate`'s first argument is the `PadState` itself, which is what makes this work
# against any program without knowing where it keeps one: a local in the runner's `main`,
# a global in hbmenu. The only symbol that has to be found is `padUpdate`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=gdb-lib.sh
source "${SCRIPT_DIR}/gdb-lib.sh"

# Offsets into libnx's `PadState`, whose layout the injection writes through directly.
readonly PAD_BUTTONS_CUR_OFFSET=16
readonly PAD_BUTTONS_OLD_OFFSET=24

# Maps a button name to its bit in `HidNpadButton`.
button_bit() {
    case "${1,,}" in
        a)      echo $((1 << 0))  ;;
        b)      echo $((1 << 1))  ;;
        x)      echo $((1 << 2))  ;;
        y)      echo $((1 << 3))  ;;
        lstick) echo $((1 << 4))  ;;
        rstick) echo $((1 << 5))  ;;
        l)      echo $((1 << 6))  ;;
        r)      echo $((1 << 7))  ;;
        zl)     echo $((1 << 8))  ;;
        zr)     echo $((1 << 9))  ;;
        plus)   echo $((1 << 10)) ;;
        minus)  echo $((1 << 11)) ;;
        left)   echo $((1 << 12)) ;;
        up)     echo $((1 << 13)) ;;
        right)  echo $((1 << 14)) ;;
        down)   echo $((1 << 15)) ;;
        *)
            echo "error: unknown button '${1}'." >&2
            echo "       Known: A B X Y LStick RStick L R ZL ZR Plus Minus Left Up Right Down" >&2
            return 1
            ;;
    esac
}

main() {
    local ip="" button="" elf="" module="" process="hbloader"

    if [[ $# -lt 2 ]]; then
        sed -n '2,8p' "${BASH_SOURCE[0]}" >&2
        exit 2
    fi

    ip="$1"; shift
    button="$1"; shift

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --elf)     elf="$2";     shift 2 ;;
            --module)  module="$2";  shift 2 ;;
            --process) process="$2"; shift 2 ;;
            *)
                echo "error: unexpected argument '${1}'." >&2
                exit 2
                ;;
        esac
    done

    if [[ -z "${module}" ]]; then
        echo "error: --module is required; it names the loaded module to press in." >&2
        exit 2
    fi

    local bit
    bit="$(button_bit "${button}")"

    require_stub "${ip}"

    local pid
    pid="$(console_pid "${ip}" "${process}")"
    echo "process ${process} is ${pid}" >&2

    local pad_update_addr
    if [[ -n "${elf}" ]]; then
        # An ELF is only worth trusting when it is the build the console is running, which
        # is the case for anything this repository deploys.
        local base offset
        base="$(module_base "${ip}" "${pid}" "${module}")"
        offset="$(symbol_offset "${elf}" padUpdate)"
        # Both are hexadecimal, and the shell needs them decimal to add.
        pad_update_addr="$(printf '0x%x' $(( base + offset )))"
        echo "padUpdate is at ${pad_update_addr} (${module} at ${base} + ${offset})" >&2
    else
        # No ELF: find the function by its own code, which asks nothing of the layout and
        # so works against a build this machine does not have.
        local range start end
        range="$(module_range "${ip}" "${pid}" "${module}")"
        start="${range% *}"
        end="${range#* }"
        pad_update_addr="$(scan_pad_update "${ip}" "${pid}" "${start}" "${end}")"
        echo "padUpdate found at ${pad_update_addr} (searched ${module}: ${start}-${end})" >&2
    fi

    local script
    script="$(mktemp)"
    cat > "${script}" <<EOF
set pagination off
set architecture aarch64
set tcp connect-timeout 10
target extended-remote ${ip}:${GDB_STUB_PORT}
attach ${pid}

# Stop where the program is about to refresh its pad, so that its address can be taken
# from the first argument before the call runs.
break *${pad_update_addr}
continue
set \$pad = \$x0

# The press has to be written after the refresh, or it would be overwritten by it. At the
# entry stop the return address is still in x30, untouched, so this lands immediately after
# padUpdate returns to its caller.
tbreak *\$x30
continue

set var *(unsigned long *)(\$pad + ${PAD_BUTTONS_CUR_OFFSET}) = \
    *(unsigned long *)(\$pad + ${PAD_BUTTONS_CUR_OFFSET}) | ${bit}
set var *(unsigned long *)(\$pad + ${PAD_BUTTONS_OLD_OFFSET}) = \
    *(unsigned long *)(\$pad + ${PAD_BUTTONS_OLD_OFFSET}) & ~${bit}

printf "injected: pad=%#lx cur=%#lx old=%#lx\n", \$pad, \
    *(unsigned long *)(\$pad + ${PAD_BUTTONS_CUR_OFFSET}), \
    *(unsigned long *)(\$pad + ${PAD_BUTTONS_OLD_OFFSET})

# Every breakpoint has to go before detaching, or the program keeps trapping into a stub
# with nobody on the other end.
delete
detach
disconnect
EOF

    local output
    output="$(gdb_batch "${script}")"
    rm -f "${script}"

    if ! grep -q "^injected:" <<< "${output}"; then
        echo "error: the press was not delivered." >&2
        echo "${output}" >&2
        exit 1
    fi

    grep "^injected:" <<< "${output}" >&2
    echo "pressed ${button} in ${module}" >&2
}

main "$@"
