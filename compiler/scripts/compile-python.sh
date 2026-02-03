#!/bin/bash
set -e

# Python compilation with Nuitka

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.py"
cd "$TEMP_DIR"

# Nuitka compilation to standalone binary
python3 -m nuitka \
    --standalone \
    --onefile \
    --remove-output \
    --output-dir="$TEMP_DIR/out" \
    --output-filename=output \
    main.py 2>&1

cp "$TEMP_DIR/out/output" "$OUTPUT_PATH"
