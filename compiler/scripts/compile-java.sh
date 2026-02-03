#!/bin/bash
set -e

# Java compilation with GraalVM native-image

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract class name from source (look for public class or just class)
CLASS_NAME=$(grep -oP '(public\s+)?class\s+\K\w+' "$SOURCE_PATH" | head -1)
if [ -z "$CLASS_NAME" ]; then
    CLASS_NAME="Main"
fi

cp "$SOURCE_PATH" "$TEMP_DIR/${CLASS_NAME}.java"
cd "$TEMP_DIR"

# Compile to bytecode
javac "${CLASS_NAME}.java"

# Build native image (without musl for compatibility)
native-image --no-fallback --static -O2 -o output "$CLASS_NAME" 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
