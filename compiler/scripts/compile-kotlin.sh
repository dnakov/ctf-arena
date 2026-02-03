#!/bin/bash
set -e

# Kotlin compilation with GraalVM native-image

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/Main.kt"
cd "$TEMP_DIR"

# Compile to JVM bytecode with runtime included
kotlinc Main.kt -include-runtime -d main.jar 2>&1

# Build native image
native-image --no-fallback --static -O2 -jar main.jar -o output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
