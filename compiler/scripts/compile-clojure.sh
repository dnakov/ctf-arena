#!/bin/bash
set -e

# Clojure compilation with GraalVM native-image

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

mkdir -p "$TEMP_DIR/src"

# Extract namespace from source file
NS_NAME=$(grep -oP '\(ns\s+\K[\w.-]+' "$SOURCE_PATH" | head -1)
if [ -z "$NS_NAME" ]; then
    NS_NAME=$(basename "$SOURCE_PATH" .clj)
fi

# Convert namespace with dots to path with slashes (e.g., foo.bar -> foo/bar.clj)
NS_PATH=$(echo "$NS_NAME" | tr '.' '/')
mkdir -p "$TEMP_DIR/src/$(dirname "$NS_PATH")" 2>/dev/null || true
cp "$SOURCE_PATH" "$TEMP_DIR/src/${NS_PATH}.clj"
cd "$TEMP_DIR"

# Create deps.edn for AOT compilation
cat > deps.edn << 'EOF'
{:paths ["src" "classes"]
 :deps {org.clojure/clojure {:mvn/version "1.11.1"}}}
EOF

mkdir -p classes

# AOT compile (use clojure instead of clj to avoid rlwrap requirement)
clojure -M -e "(compile '$NS_NAME)" 2>&1

# Build native image
native-image --no-fallback --static -O2 -cp "classes:$(clojure -Spath)" "$NS_NAME" -o output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
