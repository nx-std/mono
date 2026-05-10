#!/usr/bin/env bash
# Package a pre-built ELF as an NRO or NSP using devkitPro standalone tools
# (nacptool, elf2nro/elf2nso, npdmtool, build_pfs0).
#
# Tool location is taken from $DEVKITPRO/tools/bin (default /opt/devkitpro).

set -euo pipefail

DKP_BIN="${DEVKITPRO:-/opt/devkitpro}/tools/bin"

abs_path() {
    echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
}

INPUT=""
OUTPUT=""
TMP_DIR=""
NAME=""
AUTHOR=""
VERSION=""
ICON=""
ROMFS=""
NPDM_JSON=""
NO_NACP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --out-dir)   shift 2 ;;
        --input)     INPUT=$(abs_path "$2"); shift 2 ;;
        --output)    OUTPUT=$(abs_path "$2"); shift 2 ;;
        --tmp-dir)   TMP_DIR=$(abs_path "$2"); shift 2 ;;
        --no-nacp)   NO_NACP=true; shift ;;
        --name)      NAME="$2"; shift 2 ;;
        --author)    AUTHOR="$2"; shift 2 ;;
        --version)   VERSION="$2"; shift 2 ;;
        --icon)      ICON=$(abs_path "$2"); shift 2 ;;
        --romfs)     ROMFS=$(abs_path "$2"); shift 2 ;;
        --npdm-json) NPDM_JSON=$(abs_path "$2"); shift 2 ;;
        *) echo "devkitpro-bundle.sh: unknown option '$1'" >&2; exit 1 ;;
    esac
done

mkdir -p "$TMP_DIR"
cd "$TMP_DIR"

STEM=$(basename "$INPUT" ".${INPUT##*.}")

if [[ -n "$NPDM_JSON" ]]; then
    "$DKP_BIN/npdmtool" "$NPDM_JSON" "$STEM.npdm"
    "$DKP_BIN/elf2nso"  "$INPUT" "$STEM.nso"

    EXEFS_DIR="$TMP_DIR/exefs"
    mkdir -p "$EXEFS_DIR"
    cp "$STEM.nso"  "$EXEFS_DIR/main"
    cp "$STEM.npdm" "$EXEFS_DIR/main.npdm"

    "$DKP_BIN/build_pfs0" "$EXEFS_DIR" "$OUTPUT"
    rm -rf "$EXEFS_DIR"
else
    nro_args=("$INPUT" "$OUTPUT")
    [[ -n "$ICON" ]] && nro_args+=("--icon=$ICON")

    if [[ "$NO_NACP" != true ]]; then
        "$DKP_BIN/nacptool" --create "$NAME" "$AUTHOR" "$VERSION" "$STEM.nacp"
        nro_args+=("--nacp=$STEM.nacp")
    fi

    [[ -n "$ROMFS" ]] && nro_args+=("--romfsdir=$ROMFS")

    "$DKP_BIN/elf2nro" "${nro_args[@]}"
fi
