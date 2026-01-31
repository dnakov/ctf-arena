# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a sandbox for executing arbitrary x86_64 binaries with deterministic instruction counting, hard instruction limits, and memory tracking. It uses QEMU user-mode emulation with a custom TCG plugin to count instructions.

**Goal**: Run untrusted binaries in isolation while measuring exactly how many CPU instructions they execute. Used for CTF-style code golf challenges where the winner is determined by instruction count.

## Build Commands

```bash
# Build the sandbox Docker image
docker build --platform linux/amd64 -t sandbox .

# Run a binary in the sandbox
python3 sandbox.py <binary> [instruction_limit]

# Example: run with 10M instruction limit (default)
python3 sandbox.py tests/hello

# Example: run with custom limit
python3 sandbox.py tests/infinite 1000000
```

## Compiling Test Binaries

Test binaries must be x86_64 Linux executables. Static linking recommended for portability.

```bash
# C with musl (minimal overhead)
docker run --rm --platform linux/amd64 -v $(pwd)/tests:/tests alpine:latest \
  sh -c "apk add gcc musl-dev && gcc -static -O2 -o /tests/binary /tests/source.c"

# Zig (very minimal runtime)
docker run --rm --platform linux/amd64 -v $(pwd)/tests:/tests alpine:latest \
  sh -c "apk add zig && zig build-exe /tests/source.zig -target x86_64-linux-musl -OReleaseFast -fstrip -femit-bin=/tests/binary"

# Pure assembly
docker run --rm --entrypoint "" -v $(pwd)/tests:/tests sandbox:latest \
  sh -c "x86_64-linux-gnu-as -o /tests/prog.o /tests/prog.S && x86_64-linux-gnu-ld -o /tests/prog /tests/prog.o"
```

## Architecture

```
sandbox.py          # Python wrapper - handles Docker, parses plugin output
Dockerfile          # Multi-stage: builds QEMU 9.2.0 with plugin support
plugin/
  sandbox.c         # QEMU TCG plugin - counts instructions, enforces limits
  Makefile          # Builds plugin as shared library
tests/              # Test binaries in various languages
```

### How It Works

1. `sandbox.py` writes the binary to a temp file, mounts it into Docker
2. Docker runs with `--network=none`, `--read-only`, memory limits
3. QEMU x86_64-linux-user executes the binary with the TCG plugin loaded
4. Plugin counts instructions per translation block, exits with code 137 if limit exceeded
5. On exit, plugin outputs JSON stats to stderr: `{"instructions": N, "memory_peak_kb": M, "limit_reached": bool}`
6. `sandbox.py` parses this and returns a `Result` dataclass

### Plugin Features

- Counts instructions at translation block granularity (fast)
- Can start counting from `main()` instead of `_start` if binary has symbols
- Handles PIE binaries by detecting runtime base address
- Supports Go binaries (looks for `main.main` symbol)
- Reports VmPeak from `/proc/self/status`

## Notes

- Sandbox uses `--tmpfs=/tmp` and `--tmpfs=/var` to support runtimes that need temp extraction (PyInstaller, etc.)
- On native Linux x86_64, all major runtimes work (Bun, Deno, Node, Python, etc.)
- Previous Mac/ARM issues with QEMU are resolved on native Linux

## Instruction Count Reference (Port Scanner - 3 ports)

| Language | Instructions |
|----------|-------------:|
| Assembly (hand-optimized) | 51 |
| Zig | 594 |
| C (raw syscalls) | 1,525 |
| C (musl) | 2,008 |
| Nim | 23,580 |
| Rust (musl) | 24,530 |
| Pascal | 30,375 |
| Swift | 45,503 |
| OCaml | 295,270 |
| Lua 5.4 | 410,182 |
| LuaJIT | 433,335 |
| C++ | 535,110 |
| Haskell | 558,511 |
| Scala (GraalVM) | 922,764 |
| Go | 1,120,285 |
| Java (GraalVM) | 1,134,302 |
| Clojure (GraalVM) | 1,147,912 |
| Kotlin (GraalVM) | 1,219,305 |
| C# (.NET AOT) | 3,203,528 |
| Bun | 17,653,512 |
| Deno | 130,547,222 |
| Node | 176,107,839 |
| Python (PyInstaller) | 376,920,746 |
| Racket | 1,390,379,233 |

## TinyGo vs Go

TinyGo produces much smaller and more efficient binaries:
- TinyGo hello: 85,651 instructions
- Standard Go hello: 747,617 instructions (9x more)

Note: TinyGo's network stack crashes in QEMU user-mode.

## WASM Runtimes

Testing WASM requires bundling a runtime (wasmtime, wasmer):
- Wasmtime hello: ~25M instructions
- Wasmer hello: ~31M instructions (22% slower)

WASI has limited socket support, so port scanner not feasible.
