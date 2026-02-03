#!/bin/bash
set -e

# TypeScript/JavaScript compilation with Bun

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.ts"
cd "$TEMP_DIR"

# Bun compile to standalone binary
bun build --compile --target=bun-linux-x64 main.ts --outfile output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
