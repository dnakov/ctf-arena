#!/bin/bash
set -e

# Free Pascal compilation with static linking
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=1|2|3|4|s - Optimization level
#   FLAG_STRIP=true|false - Strip symbols (default: true for release)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT=""
        DEFAULT_STRIP="false"
        FLAGS="-g -gl"
        ;;
    release)
        # High optimization: -O3 with strip
        DEFAULT_OPT="3"
        DEFAULT_STRIP="true"
        FLAGS=""
        ;;
    size)
        DEFAULT_OPT="s"
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
if [ "$STRIP" = "true" ]; then
    FLAGS="$FLAGS -Xs"
fi

# -XX enables smart linking, -CX enables external linker
exec fpc $FLAGS -XX -CX -o"$OUTPUT_PATH" "$SOURCE_PATH"
