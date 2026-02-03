#!/bin/bash
set -e

# Entry script that dispatches to language-specific compile scripts
# Environment variables:
#   LANGUAGE - the language to compile (c, rust, go, etc.)
#   OPTIMIZATION - debug, release, or size (default: release)
#   SOURCE_FILE - the source file name in /work
#   OUTPUT_FILE - the output binary name in /work (default: output)

LANGUAGE="${LANGUAGE:-}"
OPTIMIZATION="${OPTIMIZATION:-release}"
SOURCE_FILE="${SOURCE_FILE:-}"
OUTPUT_FILE="${OUTPUT_FILE:-output}"

if [ -z "$LANGUAGE" ]; then
    echo "ERROR: LANGUAGE environment variable is required" >&2
    exit 1
fi

if [ -z "$SOURCE_FILE" ]; then
    echo "ERROR: SOURCE_FILE environment variable is required" >&2
    exit 1
fi

SOURCE_PATH="/work/${SOURCE_FILE}"
OUTPUT_PATH="/work/${OUTPUT_FILE}"

if [ ! -f "$SOURCE_PATH" ]; then
    echo "ERROR: Source file not found: $SOURCE_PATH" >&2
    exit 1
fi

# Dispatch to language-specific script
SCRIPT_PATH="/compiler/scripts/compile-${LANGUAGE}.sh"

if [ ! -f "$SCRIPT_PATH" ]; then
    echo "ERROR: Unsupported language: $LANGUAGE" >&2
    exit 1
fi

# Export variables for child scripts
export SOURCE_PATH OUTPUT_PATH OPTIMIZATION

# Output files for metadata
VERSION_FILE="/work/compiler_version.txt"
FLAGS_FILE="/work/compile_flags.json"
get_version() {
    case "$LANGUAGE" in
        c|cpp)   gcc --version | head -1 ;;
        rust)    rustc --version ;;
        go)      go version ;;
        zig)     zig version ;;
        nim)     nim --version | head -1 ;;
        pascal)  fpc -version 2>&1 | head -1 ;;
        ocaml)   ocaml --version ;;
        swift)   swift --version 2>&1 | head -1 ;;
        haskell) ghc --version ;;
        csharp)  dotnet --version ;;
        java)    java --version 2>&1 | head -1 ;;
        kotlin)  kotlinc -version 2>&1 | head -1 ;;
        scala)   scala --version 2>&1 | head -1 || echo "scala3" ;;
        clojure) clojure --version 2>&1 | head -1 ;;
        python)  python3 --version 2>&1 && python3 -m nuitka --version 2>&1 | head -1 ;;
        javascript|typescript|bun) bun --version ;;
        deno)    deno --version | head -1 ;;
        node)    node --version ;;
        lua)     lua -v ;;
        perl)    perl --version | grep -o 'v[0-9.]*' | head -1 ;;
        php)     php --version | head -1 ;;
        tcl)     echo "jimtcl" ;;
        erlang)  erl -eval 'io:format("~s~n", [erlang:system_info(otp_release)]), halt().' -noshell ;;
        elixir)  elixir --version | head -2 ;;
        racket)  racket --version ;;
        asm)     as --version | head -1 ;;
        wasm)    wat2wasm --version ;;
        *)       echo "unknown" ;;
    esac
}

get_version > "$VERSION_FILE" 2>&1 || true

# Capture compile flags (all FLAG_* env vars plus OPTIMIZATION)
{
    echo "{"
    echo "  \"optimization\": \"$OPTIMIZATION\""
    # Add FLAG_* vars
    first=true
    for var in $(env | grep '^FLAG_' | cut -d= -f1); do
        value="${!var}"
        echo ",  \"${var}\": \"$value\""
    done
    echo "}"
} > "$FLAGS_FILE" 2>/dev/null || true

# Execute the language-specific script
"$SCRIPT_PATH"
