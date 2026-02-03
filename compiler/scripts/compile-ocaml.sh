#!/bin/bash
set -e

# OCaml native compilation
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=2|3 - Optimization level
#   FLAG_STRIP=true|false - Strip symbols (default: true for release)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT=""
        DEFAULT_STRIP="false"
        FLAGS="-g"
        ;;
    release)
        # High optimization: -O3 with strip
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        FLAGS=""
        ;;
    size)
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        FLAGS=""
        ;;
    *)
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        FLAGS=""
        ;;
esac

# Parse flags
OPT="${FLAG_OPT:-$DEFAULT_OPT}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"

if [ -n "$OPT" ]; then
    FLAGS="$FLAGS -O$OPT"
fi

# ocamlopt produces native code
# Link with Unix module for socket operations
ocamlopt $FLAGS unix.cmxa -o "$OUTPUT_PATH" "$SOURCE_PATH"

# Strip if requested
if [ "$STRIP" = "true" ]; then
    strip "$OUTPUT_PATH"
fi
