#!/bin/bash
set -e

# Erlang compilation to escript (self-contained executable script)

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Copy source and determine module name
cp "$SOURCE_PATH" "$TEMP_DIR/main.erl"
cd "$TEMP_DIR"

# Extract module name from -module() directive
MODULE_NAME=$(grep -oP '^\s*-module\s*\(\s*\K\w+' main.erl | head -1)
if [ -z "$MODULE_NAME" ]; then
    MODULE_NAME="main"
fi

# Rename file to match module name if needed
if [ "$MODULE_NAME" != "main" ]; then
    mv main.erl "${MODULE_NAME}.erl"
fi

# Compile to BEAM
erlc "${MODULE_NAME}.erl"

# Create escript
cat > output << EOF
#!/usr/bin/env escript
%%! -pa .
main(_Args) ->
    ${MODULE_NAME}:main().
EOF

# Make it executable
chmod +x output

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
