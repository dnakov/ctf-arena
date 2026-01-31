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
    print(f"Memory peak: {result.memory_peak_kb} KB")
    print(f"Limit reached: {result.limit_reached}")
    if result.stdout:
        print(f"Stdout: {result.stdout.decode(errors='replace')}")
    if result.stderr:
        print(f"Stderr: {result.stderr.decode(errors='replace')}")
