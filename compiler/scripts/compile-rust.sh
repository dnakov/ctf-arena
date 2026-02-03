#!/bin/bash
set -e

# Rust compilation with musl target for static binary
#
# Supported flags (via FLAG_* environment variables):
#   FLAG_NOSTD=true      - Compile as no_std (requires manual _start)
#   FLAG_LTO=thin|fat    - LTO mode (default: true/fat for release)
#   FLAG_PANIC=abort|unwind - Panic strategy (default: abort)
#   FLAG_OPT=0|1|2|3|s|z - Optimization level
#   FLAG_TARGET=<target> - Custom target (default: x86_64-unknown-linux-musl)
#   FLAG_CODEGEN_UNITS=N - Codegen units (default: 1)
#   FLAG_STRIP=true|false - Strip symbols (default: true for release)

case "$OPTIMIZATION" in
    debug)
        PROFILE="dev"
        DEFAULT_OPT="0"
        DEFAULT_LTO="false"
        DEFAULT_STRIP="false"
        ;;
    release)
        PROFILE="release"
        DEFAULT_OPT="3"
        DEFAULT_LTO="true"
        DEFAULT_STRIP="true"
        ;;
    size)
        PROFILE="release"
        DEFAULT_OPT="z"
        DEFAULT_LTO="true"
        DEFAULT_STRIP="true"
        ;;
    *)
        PROFILE="release"
        DEFAULT_OPT="3"
        DEFAULT_LTO="true"
        DEFAULT_STRIP="true"
        ;;
esac

# Parse flags
OPT_LEVEL="${FLAG_OPT:-$DEFAULT_OPT}"
LTO="${FLAG_LTO:-$DEFAULT_LTO}"
PANIC="${FLAG_PANIC:-abort}"
TARGET="${FLAG_TARGET:-x86_64-unknown-linux-musl}"
CODEGEN_UNITS="${FLAG_CODEGEN_UNITS:-1}"
STRIP="${FLAG_STRIP:-$DEFAULT_STRIP}"
NOSTD="${FLAG_NOSTD:-false}"

# Create a temporary cargo project
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

mkdir -p "$TEMP_DIR/src"

# For no_std, we need to add the attribute
if [ "$NOSTD" = "true" ]; then
    echo '#![no_std]' > "$TEMP_DIR/src/main.rs"
    echo '#![no_main]' >> "$TEMP_DIR/src/main.rs"
    cat "$SOURCE_PATH" >> "$TEMP_DIR/src/main.rs"
else
    cp "$SOURCE_PATH" "$TEMP_DIR/src/main.rs"
fi

# Generate Cargo.toml with flags
cat > "$TEMP_DIR/Cargo.toml" << EOF
[package]
name = "solution"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = $OPT_LEVEL
lto = $LTO
codegen-units = $CODEGEN_UNITS
panic = "$PANIC"
strip = $STRIP

[profile.dev]
opt-level = $OPT_LEVEL
panic = "$PANIC"
EOF

cd "$TEMP_DIR"

# Build with appropriate flags
RUSTFLAGS="-C target-feature=+crt-static"

# Add linker for no_std builds
if [ "$NOSTD" = "true" ]; then
    RUSTFLAGS="$RUSTFLAGS -C link-arg=-nostartfiles"
fi

export RUSTFLAGS

if [ "$PROFILE" = "release" ]; then
    cargo build --release --target "$TARGET" 2>&1
    cp "$TEMP_DIR/target/$TARGET/release/solution" "$OUTPUT_PATH"
else
    cargo build --target "$TARGET" 2>&1
    cp "$TEMP_DIR/target/$TARGET/debug/solution" "$OUTPUT_PATH"
fi
