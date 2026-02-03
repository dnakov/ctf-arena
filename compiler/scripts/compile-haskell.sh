#!/bin/bash
set -e

# Haskell compilation with GHC
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=0|1|2 - Optimization level (default: 2 for release)
#   FLAG_STRIP=true|false - Strip symbols (default: true for release)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT="0"
        DEFAULT_STRIP="false"
        ;;
    release)
        # High optimization: -O2 with strip
        DEFAULT_OPT="2"
        DEFAULT_STRIP="true"
        ;;
    size)
        DEFAULT_OPT="2"
        DEFAULT_STRIP="true"
        ;;
    *)
        DEFAULT_OPT="2"
        DEFAULT_STRIP="true"
        ;;
esac

# Parse flags
OPT="${FLAG_OPT:-$DEFAULT_OPT}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"

FLAGS="-O$OPT"

# Find cabal package database if it exists
CABAL_STORE="/root/.cabal/store/ghc-$(ghc --numeric-version)"
if [ -d "$CABAL_STORE/package.db" ]; then
    FLAGS="$FLAGS -package-db $CABAL_STORE/package.db"
fi

# Static linking with pthread
ghc $FLAGS -static -optl-static -optl-pthread -o "$OUTPUT_PATH" "$SOURCE_PATH"

# Strip if requested
if [ "$STRIP" = "true" ]; then
    strip "$OUTPUT_PATH"
fi
