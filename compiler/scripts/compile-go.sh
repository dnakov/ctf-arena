#!/bin/bash
set -e

# Go compilation - static by default
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_CGO=true|false       - Enable CGO (default: false for static)
#   FLAG_STRIP=true|false     - Strip debug info (default: true for release)
#   FLAG_RACE=true|false      - Enable race detector
#   FLAG_GCFLAGS=...          - Additional gc compiler flags
#   FLAG_TAGS=...             - Build tags (comma-separated)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_STRIP="false"
        ;;
    release|size|*)
        DEFAULT_STRIP="true"
        ;;
esac

# Parse flags
CGO="${FLAG_CGO:-false}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
RACE="${FLAG_RACE:-false}"
GCFLAGS="${FLAG_GCFLAGS:-}"
TAGS="${FLAG_TAGS:-}"

# Build ldflags
LDFLAGS=""
if [ "$STRIP" = "true" ]; then
    LDFLAGS="-s -w"
fi

# Build command args
BUILD_ARGS=""

if [ "$RACE" = "true" ]; then
    BUILD_ARGS="$BUILD_ARGS -race"
fi

if [ -n "$GCFLAGS" ]; then
    BUILD_ARGS="$BUILD_ARGS -gcflags=$GCFLAGS"
fi

if [ -n "$TAGS" ]; then
    BUILD_ARGS="$BUILD_ARGS -tags=$TAGS"
fi

# Set environment
if [ "$CGO" = "true" ]; then
    export CGO_ENABLED=1
else
    export CGO_ENABLED=0
fi
export GOOS=linux
export GOARCH=amd64

go build -ldflags="$LDFLAGS" $BUILD_ARGS -o "$OUTPUT_PATH" "$SOURCE_PATH"
