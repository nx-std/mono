#!/bin/bash

set -e

# Function to convert relative paths to absolute paths
make_absolute_path() {
    echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --out-dir)
            OUT_DIR=$(make_absolute_path "$2")
            shift 2
            ;;
        --input)
            INPUT=$(make_absolute_path "$2")
            shift 2
            ;;
        --output)
            OUTPUT=$(make_absolute_path "$2")
            shift 2
            ;;
        --tmp-dir)
            TMP_DIR=$(make_absolute_path "$2")
            shift 2
            ;;
        --no-nacp)
            NO_NACP=true
            shift
            ;;
        --name)
            NAME="$2"
            shift 2
            ;;
        --author)
            AUTHOR="$2"
            shift 2
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --icon)
            ICON=$(make_absolute_path "$2")
            shift 2
            ;;
        --romfs)
            ROMFS=$(make_absolute_path "$2")
            shift 2
            ;;
        --npdm-json)
            NPDM_JSON=$(make_absolute_path "$2")
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Ensure TMP_DIR exists
mkdir -p "$TMP_DIR"
cd "$TMP_DIR"

STEM=$(basename "$INPUT" .${INPUT##*.})

if [[ -n "$NPDM_JSON" ]]; then
    cargo nx tool npdmtool "$NPDM_JSON" "$STEM.npdm"
    cargo nx tool elf2nso "$INPUT" "$STEM.nso"

    EXEFS_DIR="$TMP_DIR/exefs"
    mkdir -p "$EXEFS_DIR"
    cp "$STEM.nso" "$EXEFS_DIR/main"
    cp "$STEM.npdm" "$EXEFS_DIR/main.npdm"

    cargo nx tool build_pfs0 "$EXEFS_DIR" "$OUTPUT"

    rm -rf "$EXEFS_DIR"
else
    CMD="cargo nx tool elf2nro \"$INPUT\" \"$OUTPUT\""

    if [[ -n "$ICON" ]]; then
        CMD+=" --icon=\"$ICON\""
    fi

    if [[ "$NO_NACP" != true ]]; then
        cargo nx tool nacptool --create "$NAME" "$AUTHOR" "$VERSION" "$STEM.nacp"
        CMD+=" --nacp=\"$STEM.nacp\""
    fi

    if [[ -n "$ROMFS" ]]; then
        CMD+=" --romfsdir=\"$ROMFS\""
    fi

    eval "$CMD"
fi
