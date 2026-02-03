#!/bin/bash
set -e

# Racket compilation to standalone executable

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.rkt"
cd "$TEMP_DIR"

# Compile to standalone executable using raco
raco exe -o output main.rkt 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
