#!/bin/bash
set -e

# Lua compilation - compile to bytecode and create self-contained executable
# Uses luac for bytecode and embeds it with a C wrapper

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cp "$SOURCE_PATH" "$TEMP_DIR/main.lua"
cd "$TEMP_DIR"

# Compile to bytecode
luac5.4 -o main.luac main.lua

# Create C wrapper that embeds the bytecode
xxd -i main.luac > bytecode.c

cat > wrapper.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <lua.h>
#include <lauxlib.h>
#include <lualib.h>
#include "bytecode.c"

int main(int argc, char *argv[]) {
    lua_State *L = luaL_newstate();
    if (!L) {
        fprintf(stderr, "Failed to create Lua state\n");
        return 1;
    }
    luaL_openlibs(L);

    if (luaL_loadbuffer(L, (const char*)main_luac, main_luac_len, "main") != LUA_OK) {
        fprintf(stderr, "Load error: %s\n", lua_tostring(L, -1));
        lua_close(L);
        return 1;
    }

    if (lua_pcall(L, 0, LUA_MULTRET, 0) != LUA_OK) {
        fprintf(stderr, "Runtime error: %s\n", lua_tostring(L, -1));
        lua_close(L);
        return 1;
    }

    lua_close(L);
    return 0;
}
EOF

# Compile with static Lua library
gcc -static -O2 -I/usr/include/lua5.4 wrapper.c -llua5.4 -lm -o output

cp "$TEMP_DIR/output" "$OUTPUT_PATH"
