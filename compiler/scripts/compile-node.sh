#!/bin/bash
set -e

# Node.js platform - bundle JavaScript to standalone binary using pkg

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.js"
cd "$TEMP_DIR"

# Create minimal package.json
cat > package.json <<EOF
{
  "name": "app",
  "version": "1.0.0",
  "main": "main.js",
  "bin": "main.js",
  "pkg": {
    "targets": ["node22-linux-x64"],
    "outputPath": "."
  }
}
EOF

# Use pkg to create standalone binary
/usr/lib/node_modules/@yao-pkg/pkg/lib-es5/bin.js . --target node22-linux-x64 --output output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
