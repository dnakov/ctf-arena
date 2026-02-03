#!/bin/bash
set -e

# Scala 3 compilation with GraalVM native-image

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

BASENAME=$(basename "$SOURCE_PATH" .scala)
cp "$SOURCE_PATH" "$TEMP_DIR/$BASENAME.scala"
cd "$TEMP_DIR"

# Find Scala library jars
SCALA_LIB=$(find /usr/local/scala/lib -name "scala3-library_3-*.jar" | head -1)
SCALA_LIB2=$(find /usr/local/scala/lib -name "scala-library-*.jar" | head -1)

# Compile to JVM bytecode
scalac -d . "$BASENAME.scala" 2>&1

# Extract object name from source file (object XXX { or object XXX extends)
MAIN_CLASS=$(grep -oP 'object\s+\K\w+' "$BASENAME.scala" | head -1)
if [ -z "$MAIN_CLASS" ]; then
    MAIN_CLASS="$BASENAME"
fi

# Build native image with Scala libraries
native-image --no-fallback --static -O2 \
    -cp ".:$SCALA_LIB:$SCALA_LIB2" \
    --initialize-at-build-time=scala \
    "$MAIN_CLASS" -o output 2>&1

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
