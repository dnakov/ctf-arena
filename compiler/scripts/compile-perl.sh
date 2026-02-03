#!/bin/bash
set -e

# Perl compilation with PAR::Packer (pp)

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/script.pl"
cd "$TEMP_DIR"

# Pack into standalone executable using PAR::Packer
pp -o output script.pl 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
