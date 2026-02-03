#!/bin/bash
set -e

# C++ compilation with static linking
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_COMPILER=g++|clang++ - Compiler to use (default: g++)
#   FLAG_STD=c++11|c++14|c++17|c++20|c++23 - C++ standard (default: c++20)
#   FLAG_OPT=0|1|2|3|s|z|fast - Optimization level
#   FLAG_WARNINGS=all|extra|pedantic|none - Warning level
#   FLAG_LTO=true|false       - Enable LTO
#   FLAG_STRIP=true|false     - Strip symbols
#   FLAG_RTTI=true|false      - Enable RTTI (default: true)
#   FLAG_EXCEPTIONS=true|false - Enable exceptions (default: true)

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
COMPILER="${FLAG_COMPILER:-g++}"
STD="${FLAG_STD:-c++20}"
OPT="${FLAG_OPT:-$DEFAULT_OPT}"
WARNINGS="${FLAG_WARNINGS:-}"
LTO="${FLAG_LTO:-$DEFAULT_LTO}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
RTTI="${FLAG_RTTI:-true}"
EXCEPTIONS="${FLAG_EXCEPTIONS:-true}"

# Build compiler flags
FLAGS="-O$OPT $DEBUG_FLAGS -static"

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

# RTTI
if [ "$RTTI" = "false" ]; then
    FLAGS="$FLAGS -fno-rtti"
fi

# Exceptions
if [ "$EXCEPTIONS" = "false" ]; then
    FLAGS="$FLAGS -fno-exceptions"
fi

# Select compiler
case "$COMPILER" in
    clang++)
        CC="clang++"
        ;;
    g++|*)
        CC="g++"
        ;;
esac

exec $CC $FLAGS -o "$OUTPUT_PATH" "$SOURCE_PATH" -lm
