#!/bin/bash
set -e

# PHP compilation - create executable script with embedded interpreter path
# Note: This creates a script, not a native binary. For native compilation,
# you would need static-php-cli which is complex to set up.

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Create executable PHP script
cat > "$TEMP_DIR/output" << 'HEADER'
#!/usr/bin/env php
HEADER

cat "$SOURCE_PATH" >> "$TEMP_DIR/output"
chmod +x "$TEMP_DIR/output"

cp "$TEMP_DIR/output" "$OUTPUT_PATH"

# Note: This won't work in the sandbox without PHP interpreter.
# For true standalone PHP binaries, we'd need static-php-cli or similar.
echo "Warning: PHP compilation creates an interpreter script, not a native binary" >&2
