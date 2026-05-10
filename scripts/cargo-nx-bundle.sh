#!/usr/bin/env bash
# Package a pre-built ELF as an NRO or NSP via `cargo nx bundle`.
set -euo pipefail
exec cargo nx bundle "$@"
