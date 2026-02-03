#!/bin/bash
set -e

# Bun platform - compile JavaScript/TypeScript to standalone binary

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Detect file extension based on content (TypeScript vs JavaScript)
if grep -q "^import\s.*from\s" "$SOURCE_PATH" || grep -q ": \(string\|number\|boolean\|any\)" "$SOURCE_PATH"; then
    cp "$SOURCE_PATH" "$TEMP_DIR/main.ts"
    SOURCE_NAME="main.ts"
else
    cp "$SOURCE_PATH" "$TEMP_DIR/main.js"
    SOURCE_NAME="main.js"
fi

cd "$TEMP_DIR"

# Bun compile to standalone binary
bun build --compile --target=bun-linux-x64 "$SOURCE_NAME" --outfile output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
