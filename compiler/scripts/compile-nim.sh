#!/bin/bash
set -e

# Nim compilation with static linking via musl
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=speed|size|none - Optimization mode
#   FLAG_STRIP=true|false - Strip debug info (default: true for release)
#   FLAG_LTO=true|false - Enable LTO (default: true for release)
#   FLAG_DANGER=true|false - Enable dangerous optimizations

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT=""
        DEFAULT_STRIP="false"
        DEFAULT_LTO="false"
        FLAGS="--debugger:native"
        ;;
    release)
        # High optimization: max speed with LTO
        DEFAULT_OPT="speed"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        FLAGS="-d:release -d:danger"
        ;;
    size)
        DEFAULT_OPT="size"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        FLAGS="-d:release -d:danger"
        ;;
    *)
        DEFAULT_OPT="speed"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        FLAGS="-d:release -d:danger"
        ;;
esac

# Parse flags
OPT="${FLAG_OPT:-$DEFAULT_OPT}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
LTO="${FLAG_LTO:-$DEFAULT_LTO}"
DANGER="${FLAG_DANGER:-true}"

# Build optimization flags
if [ -n "$OPT" ]; then
    FLAGS="$FLAGS --opt:$OPT"
fi
if [ "$LTO" = "true" ]; then
    FLAGS="$FLAGS --passC:-flto --passL:-flto"
fi
if [ "$STRIP" = "true" ]; then
    FLAGS="$FLAGS --passL:-s"
fi

# Nim needs musl-gcc for static linking
exec nim c $FLAGS --gcc.exe:musl-gcc --gcc.linkerexe:musl-gcc --passL:"-static" -o:"$OUTPUT_PATH" "$SOURCE_PATH"
