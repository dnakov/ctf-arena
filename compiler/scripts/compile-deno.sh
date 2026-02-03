#!/bin/bash
set -e

# Deno platform - compile TypeScript/JavaScript to standalone binary

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Detect file extension
if grep -q "^import\s.*from\s" "$SOURCE_PATH" || grep -q ": \(string\|number\|boolean\|any\)" "$SOURCE_PATH"; then
    cp "$SOURCE_PATH" "$TEMP_DIR/main.ts"
    SOURCE_NAME="main.ts"
else
    cp "$SOURCE_PATH" "$TEMP_DIR/main.js"
    SOURCE_NAME="main.js"
fi

cd "$TEMP_DIR"

# Deno compile to standalone binary
deno compile --allow-all --output output "$SOURCE_NAME" 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
