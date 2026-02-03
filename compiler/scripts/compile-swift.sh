#!/bin/bash
set -e

# Swift compilation
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_OPT=none|O|Osize|Ounchecked - Optimization level
#   FLAG_STRIP=true|false - Strip symbols (default: true for release)
#   FLAG_WMO=true|false - Whole module optimization (default: true for release)

case "$OPTIMIZATION" in
    debug)
        DEFAULT_OPT="-Onone"
        DEFAULT_STRIP="false"
        DEFAULT_WMO="false"
        FLAGS="-g"
        ;;
    release)
        # High optimization: -O with WMO and strip
        DEFAULT_OPT="-O"
        DEFAULT_STRIP="true"
        DEFAULT_WMO="true"
        FLAGS=""
        ;;
    size)
        DEFAULT_OPT="-Osize"
        DEFAULT_STRIP="true"
        DEFAULT_WMO="true"
        FLAGS=""
        ;;
    *)
        DEFAULT_OPT="-O"
        DEFAULT_STRIP="true"
        DEFAULT_WMO="true"
        FLAGS=""
        ;;
esac

# Parse flags
OPT="${FLAG_OPT:+"-$FLAG_OPT"}"
OPT="${OPT:-$DEFAULT_OPT}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
WMO="${FLAG_WMO:-$DEFAULT_WMO}"

# Build flags
FLAGS="$FLAGS $OPT"
if [ "$WMO" = "true" ]; then
    FLAGS="$FLAGS -whole-module-optimization"
fi

# Note: -static-stdlib bundles Swift runtime
swiftc $FLAGS -static-stdlib -o "$OUTPUT_PATH" "$SOURCE_PATH"

# Strip if requested
if [ "$STRIP" = "true" ]; then
    strip "$OUTPUT_PATH"
fi
