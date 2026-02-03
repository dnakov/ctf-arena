#!/bin/bash
set -e

# JavaScript compilation with Bun (same as TypeScript)

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.js"
cd "$TEMP_DIR"

# Bun compile to standalone binary
bun build --compile --target=bun-linux-x64 main.js --outfile output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
