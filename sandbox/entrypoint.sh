#!/bin/sh
# Forward environment variables to QEMU guest binary
# QEMU uses -E KEY=VALUE to pass env vars to the emulated process

# Build arguments file
ARGS_FILE=$(mktemp)

# Base command
echo "-cpu" >> "$ARGS_FILE"
echo "max" >> "$ARGS_FILE"

# Pass selected environment variables to QEMU
# Only pass FLAG and other user-defined vars, not system vars
for var in FLAG SECRET_KEY; do
    val=$(printenv "$var" 2>/dev/null)
    if [ -n "$val" ]; then
        echo "-E" >> "$ARGS_FILE"
        echo "$var=$val" >> "$ARGS_FILE"
    fi
done

# Plugin and binary
echo "-plugin" >> "$ARGS_FILE"
echo "/plugin/sandbox.so,limit=$LIMIT,binary=/work/binary,from_start=on" >> "$ARGS_FILE"
echo "/work/binary" >> "$ARGS_FILE"

# Execute QEMU with arguments from file
xargs -a "$ARGS_FILE" qemu-x86_64
EXIT_CODE=$?

rm -f "$ARGS_FILE"
exit $EXIT_CODE
