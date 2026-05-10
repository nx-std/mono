#!/usr/bin/env bash
set -euo pipefail
exec cargo nx tool bin2c "$@"
