#!/usr/bin/env python3
import json
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Result:
    instructions: int
    memory_peak_kb: int
    stdout: bytes
    stderr: bytes
    exit_code: int
    limit_reached: bool
    syscalls: int = 0
    syscall_cost: int = 0
    syscall_breakdown: dict = None
    # QEMU process memory (for reference)
    memory_rss_kb: int = 0
    memory_hwm_kb: int = 0
    memory_data_kb: int = 0
    memory_stack_kb: int = 0
    io_read_bytes: int = 0
    io_write_bytes: int = 0
    # Guest memory (actual binary allocations)
    guest_mmap_bytes: int = 0
    guest_mmap_peak: int = 0
    guest_heap_bytes: int = 0


def run(
    binary: bytes,
    *,
    instruction_limit: int = 10_000_000,
    memory_limit_mb: int = 256,
    timeout_sec: float = 30,
    stdin: bytes = b"",
) -> Result:
    with tempfile.NamedTemporaryFile(delete=False) as f:
        f.write(binary)
        binary_path = f.name

    try:
        Path(binary_path).chmod(0o755)
        proc = subprocess.run(
            [
                "docker", "run", "--rm", "-i",
                f"--memory={memory_limit_mb}m",
                f"--memory-swap={memory_limit_mb}m",
                "--network=none",
                "--read-only",
                "--tmpfs=/tmp:rw,exec,nosuid,size=64m",
                "--tmpfs=/var:rw,nosuid,size=16m",
                "-e", f"LIMIT={instruction_limit}",
                "-v", f"{binary_path}:/work/binary:ro",
                "sandbox",
            ],
            input=stdin,
            capture_output=True,
            timeout=timeout_sec,
        )
    finally:
        Path(binary_path).unlink()

    stderr = proc.stderr
    stats = {"instructions": 0, "memory_peak_kb": 0, "limit_reached": False}

    match = re.search(rb'\n(\{[^\n]+\})\n?$', stderr)
    if match:
        stats = json.loads(match.group(1))
        stderr = stderr[:match.start()]

    return Result(
        instructions=stats["instructions"],
        memory_peak_kb=stats["memory_peak_kb"],
        stdout=proc.stdout,
        stderr=stderr,
        exit_code=proc.returncode,
        limit_reached=stats["limit_reached"],
        syscalls=stats.get("syscalls", 0),
        syscall_cost=stats.get("syscall_cost", 0),
        syscall_breakdown=stats.get("syscall_breakdown", {}),
        memory_rss_kb=stats.get("memory_rss_kb", 0),
        memory_hwm_kb=stats.get("memory_hwm_kb", 0),
        memory_data_kb=stats.get("memory_data_kb", 0),
        memory_stack_kb=stats.get("memory_stack_kb", 0),
        io_read_bytes=stats.get("io_read_bytes", 0),
        io_write_bytes=stats.get("io_write_bytes", 0),
        guest_mmap_bytes=stats.get("guest_mmap_bytes", 0),
        guest_mmap_peak=stats.get("guest_mmap_peak", 0),
        guest_heap_bytes=stats.get("guest_heap_bytes", 0),
    )


if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("Usage: sandbox.py <binary> [instruction_limit]", file=sys.stderr)
        sys.exit(1)
    binary_data = Path(sys.argv[1]).read_bytes()
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 10_000_000
    result = run(binary_data, instruction_limit=limit)
    print(f"Exit code: {result.exit_code}")
    print(f"Instructions: {result.instructions}")
    print(f"Memory peak (QEMU): {result.memory_peak_kb} KB")
    print(f"Guest memory:")
    print(f"  mmap current: {result.guest_mmap_bytes} bytes")
    print(f"  mmap peak: {result.guest_mmap_peak} bytes")
    print(f"  heap (brk): {result.guest_heap_bytes} bytes")
    print(f"Limit reached: {result.limit_reached}")
    print(f"Syscalls: {result.syscalls}")
    if result.syscall_breakdown:
        print(f"Syscall breakdown:")
        for name, count in sorted(result.syscall_breakdown.items(), key=lambda x: -x[1]):
            print(f"  {name}: {count}")
    if result.stdout:
        print(f"Stdout: {result.stdout.decode(errors='replace')}")
    if result.stderr:
        print(f"Stderr: {result.stderr.decode(errors='replace')}")
