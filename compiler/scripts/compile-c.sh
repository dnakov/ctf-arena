#!/bin/bash
set -e

# C compilation with musl for static binary
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_COMPILER=gcc|clang   - Compiler to use (default: gcc via musl-gcc)
#   FLAG_STD=c89|c99|c11|c17|c23 - C standard (default: c17)
#   FLAG_OPT=0|1|2|3|s|z|fast - Optimization level (overrides OPTIMIZATION)
#   FLAG_WARNINGS=all|extra|pedantic|none - Warning level
#   FLAG_LTO=true|false       - Enable LTO
#   FLAG_STRIP=true|false     - Strip symbols
#   FLAG_MARCH=native|...     - Target architecture
#   FLAG_FREESTANDING=true    - Freestanding mode (no libc)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT="0"
        DEFAULT_STRIP="false"
        DEFAULT_LTO="false"
        DEBUG_FLAGS="-g"
        ;;
    release)
        # High optimization: -O3 with LTO for best instruction count
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        DEBUG_FLAGS=""
        ;;
    size)
        # Size optimization: -Os with LTO
        DEFAULT_OPT="s"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        DEBUG_FLAGS=""
        ;;
    *)
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        DEFAULT_LTO="true"
        DEBUG_FLAGS=""
        ;;
esac

# Parse flags
COMPILER="${FLAG_COMPILER:-gcc}"
STD="${FLAG_STD:-c17}"
OPT="${FLAG_OPT:-$DEFAULT_OPT}"
WARNINGS="${FLAG_WARNINGS:-}"
LTO="${FLAG_LTO:-$DEFAULT_LTO}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
MARCH="${FLAG_MARCH:-}"
FREESTANDING="${FLAG_FREESTANDING:-false}"

# Build compiler flags
FLAGS="-O$OPT $DEBUG_FLAGS"

# Standard
FLAGS="$FLAGS -std=$STD"

# Warnings
case "$WARNINGS" in
    all)
        FLAGS="$FLAGS -Wall"
        ;;
    extra)
        FLAGS="$FLAGS -Wall -Wextra"
        ;;
    pedantic)
        FLAGS="$FLAGS -Wall -Wextra -Wpedantic"
        ;;
    none)
        FLAGS="$FLAGS -w"
        ;;
esac

# LTO
if [ "$LTO" = "true" ]; then
    FLAGS="$FLAGS -flto"
fi

# Strip
if [ "$STRIP" = "true" ]; then
    FLAGS="$FLAGS -s"
fi

# Architecture
if [ -n "$MARCH" ]; then
    FLAGS="$FLAGS -march=$MARCH"
fi

# Freestanding mode
if [ "$FREESTANDING" = "true" ]; then
    FLAGS="$FLAGS -ffreestanding -nostdlib -nostartfiles"
    LIBS=""
else
    LIBS="-lm"
fi

# Select compiler
case "$COMPILER" in
    clang)
        CC="clang"
        FLAGS="$FLAGS --target=x86_64-linux-musl -static"
        ;;
    gcc|*)
        CC="musl-gcc"
        FLAGS="$FLAGS -static"
        ;;
esac

exec $CC $FLAGS -o "$OUTPUT_PATH" "$SOURCE_PATH" $LIBS
