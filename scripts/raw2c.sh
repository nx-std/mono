#!/usr/bin/env bash
set -euo pipefail
exec cargo nx tool raw2c "$@"
