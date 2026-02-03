#!/bin/bash
set -e

# x86_64 Assembly compilation

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

as -o "$TEMP_DIR/prog.o" "$SOURCE_PATH"
ld -o "$OUTPUT_PATH" "$TEMP_DIR/prog.o"
