#!/bin/bash
set -e

# Tcl compilation - create executable script
# Note: For a true standalone binary, you'd need to build jimtcl statically
# or use tclkit. This creates an interpreter script.

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Create executable Tcl script
cat > "$TEMP_DIR/output" << 'HEADER'
#!/usr/bin/env tclsh
HEADER

cat "$SOURCE_PATH" >> "$TEMP_DIR/output"
chmod +x "$TEMP_DIR/output"

cp "$TEMP_DIR/output" "$OUTPUT_PATH"

echo "Warning: Tcl compilation creates an interpreter script, not a native binary" >&2
