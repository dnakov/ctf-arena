#!/bin/bash
set -e

# Zig compilation
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=Debug|ReleaseFast|ReleaseSmall|ReleaseSafe - Optimization mode
#   FLAG_STRIP=true|false - Strip debug info (default: true for release)
#   FLAG_SINGLE_THREADED=true|false - Single-threaded mode

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT="-ODebug"
        DEFAULT_STRIP="false"
        ;;
    release)
        DEFAULT_OPT="-OReleaseFast"
        DEFAULT_STRIP="true"
        ;;
    size)
        DEFAULT_OPT="-OReleaseSmall"
        DEFAULT_STRIP="true"
        ;;
    *)
        DEFAULT_OPT="-OReleaseFast"
        DEFAULT_STRIP="true"
        ;;
esac

# Parse flags
OPT="${FLAG_OPT:+"-O$FLAG_OPT"}"
OPT="${OPT:-$DEFAULT_OPT}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
SINGLE_THREADED="${FLAG_SINGLE_THREADED:-false}"

BUILD_ARGS=""
if [ "$STRIP" = "true" ]; then
    BUILD_ARGS="$BUILD_ARGS -fstrip"
fi
if [ "$SINGLE_THREADED" = "true" ]; then
    BUILD_ARGS="$BUILD_ARGS -fsingle-threaded"
fi

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/source.zig"
cd "$TEMP_DIR"

zig build-exe $OPT $BUILD_ARGS -target x86_64-linux-musl -femit-bin=output source.zig

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
