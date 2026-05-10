#!/usr/bin/env bash
set -euo pipefail
exec cargo nx tool build_pfs0 "$@"
