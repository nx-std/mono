#!/usr/bin/env bash
# Arms hbmenu's netloader, so a push can be sent with nobody at the console.
#
# Usage:
#   arm-netloader.sh <ip> [--elf <path>] [--module <name>]
#
# hbmenu arms its netloader when Y is pressed on the menu (`nx_main/main.c`), which is the
# one thing a run of several suites otherwise needs a person for: the netloader disarms
# after each transfer, so every suite costs another visit to the console. Pressing Y
# through the debug stub removes the person from that loop.
#
# ## Whichever hbmenu the console runs
#
# No ELF is asked for. The press goes to `padUpdate`, which is found by searching the
# loaded module for the function's own code, so a menu this machine has no copy of is
# reached the same as one built here. Confirmed against a stock hbmenu v3.6.1, where the
# function sits more than 0xB000 away from where this repository's build puts it — an
# offset taken from the wrong ELF would have addressed unrelated memory in the process
# drawing the menu.
#
# `--elf` remains for the case where the console runs a build from this repository: it
# skips the search, which is a little quicker and cannot be confused by a second match.
# Passing one that did not produce the running module is worse than passing none.
#
# Note that arming is rarely what a run needs. The test runner listens continuously and
# has nothing to arm, so suites are pushed straight to it; this is for getting back to a
# runner when there is none.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

readonly DEFAULT_HBMENU_MODULE="nx-hbmenu.elf"

main() {
    # No ELF by default: the search finds the press site in whichever build is loaded.
    local ip="" elf="" module="${DEFAULT_HBMENU_MODULE}"

    if [[ $# -lt 1 ]]; then
        sed -n '2,6p' "${BASH_SOURCE[0]}" >&2
        exit 2
    fi

    ip="$1"; shift

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --elf)    elf="$2";    shift 2 ;;
            --module) module="$2"; shift 2 ;;
            *)
                echo "error: unexpected argument '${1}'." >&2
                exit 2
                ;;
        esac
    done

    if [[ -n "${elf}" ]]; then
        if [[ ! -f "${elf}" ]]; then
            echo "error: no hbmenu ELF at ${elf}." >&2
            exit 1
        fi
        "${SCRIPT_DIR}/press-button.sh" "${ip}" Y --elf "${elf}" --module "${module}"
    else
        "${SCRIPT_DIR}/press-button.sh" "${ip}" Y --module "${module}"
    fi
    echo "netloader armed on ${ip}" >&2
}

main "$@"
