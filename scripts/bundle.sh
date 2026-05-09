#!/usr/bin/env bash
set -euo pipefail
exec cargo nx bundle "$@"
