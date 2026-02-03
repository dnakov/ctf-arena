#!/bin/bash
set -e

# WebAssembly (WAT text format) to WASM binary compilation
# Note: The output is a .wasm file. To execute it, the sandbox
# needs a WASM runtime (wasmer, wasmtime, etc.)

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.wat"
cd "$TEMP_DIR"

# Compile WAT (text) to WASM (binary)
wat2wasm main.wat -o output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"

echo "Note: Output is a WASM binary. Requires a WASM runtime to execute." >&2
