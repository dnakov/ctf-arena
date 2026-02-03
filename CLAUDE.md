# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CTF Arena is a competitive code golf platform where users compete for the lowest instruction count per challenge. It features:

1. **Sandbox** (`sandbox/`) - QEMU-based execution with deterministic instruction counting and syscall tracking
2. **API** (`api/`) - Rust HTTP service with GitHub OAuth, challenges, and leaderboards
3. **Compile Worker** (`compile-worker/`) - Compiles source code to static binaries
4. **Compiler** (`compiler/`) - Docker image with 26+ language compilers
5. **Worker** (`worker/`) - Executes binaries in sandboxed QEMU environment
6. **Web Frontend** (`web/`) - Svelte 5 + SvelteKit UI with Monaco editor
7. **Kubernetes** (`k8s/`) - Deployment manifests for local (kind) and production

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Svelte 5 Frontend (web/)                                               │
│  ┌─────────────────┐  ┌─────────────┐  ┌───────────────────────────────┐│
│  │ Monaco Editor   │  │ Challenge   │  │ Results Panel                 ││
│  │ (code input)    │  │ Selector    │  │ - Instructions: 2,008         ││
│  │                 │  │             │  │ - Syscalls: 5                 ││
│  │                 │  │             │  │ - Memory: 256 KB              ││
│  └─────────────────┘  └─────────────┘  └───────────────────────────────┘│
└───────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────────────┐
│ API (Rust)      │────▶│ NATS JetStream  │────▶│ Compile Workers         │
│ /compile        │     │ COMPILES stream │     │ + Docker compiler image │
│ /submit         │◀────│ binaries KV     │◀────│                         │
│ /challenges     │     │                 │     └─────────────────────────┘
│ /auth/github    │     └─────────────────┘
└─────────────────┘            │
        │                      ▼
        │               ┌─────────────────────────┐
        │               │ Execute Workers         │
        │               │ + QEMU Sandbox          │
        ▼               └─────────────────────────┘
┌─────────────────┐
│ PostgreSQL      │
│ - users         │
│ - sessions      │
│ - challenges    │
│ - leaderboard   │
└─────────────────┘
```

## Quick Start

### Prerequisites
- Docker with buildx support
- kubectl + kind (for local k8s)
- Rust toolchain (for local API development)
- Node.js 20+ (for frontend development)

### Local Development with Kubernetes

```bash
# 1. Create kind cluster with local registry (first time only)
./scripts/kind-with-registry.sh

# 2. Build and push all images
docker build -t localhost:5001/ctf-api:latest ./api && docker push localhost:5001/ctf-api:latest
docker build -t localhost:5001/ctf-worker:latest ./worker && docker push localhost:5001/ctf-worker:latest
docker build -t localhost:5001/ctf-web:latest ./web && docker push localhost:5001/ctf-web:latest
docker build -t localhost:5001/compile-worker:latest ./compile-worker && docker push localhost:5001/compile-worker:latest
docker build --platform linux/amd64 -t localhost:5001/sandbox:latest ./sandbox && docker push localhost:5001/sandbox:latest

# 3. Build compiler image (large, ~15-20GB)
docker build --platform linux/amd64 -t compiler:latest ./compiler

# 4. Apply k8s manifests
kubectl apply -k k8s/overlays/local

# 5. Wait for pods to be ready
kubectl wait --for=condition=ready pod -l app=ctf-api -n ctf-arena --timeout=120s
kubectl wait --for=condition=ready pod -l app=ctf-worker -n ctf-arena --timeout=120s

# 6. Load images into worker DinD sidecars
WORKER_POD=$(kubectl get pods -n ctf-arena -l app=ctf-worker -o jsonpath='{.items[0].metadata.name}')
docker save localhost:5001/sandbox:latest | kubectl exec -n ctf-arena -i "$WORKER_POD" -c dind -- docker load

COMPILE_POD=$(kubectl get pods -n ctf-arena -l app=compile-worker -o jsonpath='{.items[0].metadata.name}')
docker save compiler:latest | kubectl exec -n ctf-arena -i "$COMPILE_POD" -c dind -- docker load

# 7. Start port forwards
kubectl port-forward -n ctf-arena svc/ctf-api 3000:3000 &
kubectl port-forward -n ctf-arena svc/ctf-web 8080:80 &

# 8. Verify
curl http://localhost:3000/health
```

**Access:**
- Web UI: http://localhost:8080
- API: http://localhost:3000

## Deploy Changes

After making code changes:

```bash
# Build and push changed images
docker build -t localhost:5001/ctf-api:latest ./api && docker push localhost:5001/ctf-api:latest
docker build -t localhost:5001/ctf-worker:latest ./worker && docker push localhost:5001/ctf-worker:latest
docker build -t localhost:5001/ctf-web:latest ./web && docker push localhost:5001/ctf-web:latest

# Restart deployments
kubectl rollout restart deployment/ctf-api deployment/ctf-worker deployment/ctf-web -n ctf-arena

# Wait for rollout
kubectl rollout status deployment/ctf-api deployment/ctf-worker deployment/ctf-web -n ctf-arena --timeout=120s

# Reload sandbox image into worker DinD
WORKER_POD=$(kubectl get pods -n ctf-arena -l app=ctf-worker --field-selector=status.phase=Running -o jsonpath='{.items[0].metadata.name}')
docker save localhost:5001/sandbox:latest | kubectl exec -n ctf-arena -i "$WORKER_POD" -c dind -- docker load
```

## API Endpoints

### Authentication
```bash
# GitHub OAuth login (redirects to GitHub)
GET /auth/github

# OAuth callback (handled automatically)
GET /auth/github/callback

# Get current user
GET /auth/me

# Logout
POST /auth/logout
```

### Challenges
```bash
# List all challenges
curl http://localhost:3000/challenges

# Get challenge details
curl http://localhost:3000/challenges/{id}

# Get leaderboard for a challenge
curl http://localhost:3000/challenges/{id}/leaderboard
```

### Compilation
```bash
# Submit source code for compilation
curl -X POST http://localhost:3000/compile \
  -F "source_code=@main.c" \
  -F "language=c" \
  -F "optimization=release"

# Check compile status
curl http://localhost:3000/compile/status/{compile_job_id}

# Get compile result (includes binary_id)
curl http://localhost:3000/compile/result/{compile_job_id}
```

### Execution
```bash
# Execute a compiled binary
curl -X POST http://localhost:3000/submit \
  -F "binary_id=sha256-abc123..." \
  -F "instruction_limit=1000000000" \
  -F "stdin=input data" \
  -F 'env_vars={"FLAG":"CTF{test}"}'

# Check execution status
curl http://localhost:3000/status/{job_id}

# Get execution result (includes instructions, syscalls, memory)
curl http://localhost:3000/result/{job_id}
```

### Benchmarks
```bash
# List available benchmarks
curl http://localhost:3000/benchmarks

# Get benchmark source for a language
curl http://localhost:3000/benchmarks/{id}/source/{filename}
```

## Supported Languages (26+)

### Tier 1: Native Compilation
| Language | Extension | Compiler | Notes |
|----------|-----------|----------|-------|
| C | .c | gcc + musl | Static linking |
| C++ | .cpp | g++ + musl | Static linking |
| Rust | .rs | rustc + musl | Static linking |
| Go | .go | go build | Static by default |
| Zig | .zig | zig build-exe | Native musl support |
| Assembly | .S | as + ld | Raw x86_64 |
| Nim | .nim | nim c | Static with musl |
| Pascal | .pas | fpc | Static linking |
| OCaml | .ml | ocamlopt | Native compilation |
| Swift | .swift | swiftc | Static stdlib |
| Haskell | .hs | ghc -static | Requires network package |
| C# | .cs | dotnet publish AOT | ImplicitUsings enabled |

### Tier 2: JVM → Native (GraalVM)
| Language | Extension | Compiler | Notes |
|----------|-----------|----------|-------|
| Java | .java | javac + native-image | GraalVM |
| Kotlin | .kt | kotlinc + native-image | GraalVM |
| Scala | .scala | scalac + native-image | GraalVM |
| Clojure | .clj | clojure + native-image | GraalVM |

### Tier 3: Scripting → Bundle
| Language | Extension | Bundler | Notes |
|----------|-----------|---------|-------|
| Python | .py | Nuitka | Large binaries |
| TypeScript | .ts | Bun.build | Single-file executable |
| JavaScript | .js | Bun.build | Single-file executable |
| Lua | .lua | luac + runtime | Bytecode bundle |
| Perl | .pl | PAR::Packer | Bundles interpreter |

### Tier 4: Special Runtimes
| Language | Extension | Runtime | Notes |
|----------|-----------|---------|-------|
| Erlang | .erl | escript | BEAM compile |
| Elixir | .ex | escript | BEAM compile |
| Racket | .rkt | raco exe | Large binaries |
| Node.js | .js | pkg | V8 bundled |
| Deno | .ts | deno compile | V8 bundled |

## Current Benchmarks

| Benchmark | Description | Test Input |
|-----------|-------------|------------|
| hello-world | Print "Hello, World!\n" | None |
| env-leak | Read FLAG environment variable | FLAG=CTF{env_leak_test} |
| base64-decode | Decode base64 from stdin | SGVsbG8sIFdvcmxkIQ== |
| portscan | Scan ports 22, 80, 443 on localhost | Network enabled |

## Execution Results

The sandbox reports detailed metrics:

```json
{
  "instructions": 1311,
  "syscalls": 5,
  "syscall_breakdown": {
    "arch_prctl": 1,
    "exit_group": 1,
    "writev": 1,
    "set_tid_address": 1,
    "ioctl": 1
  },
  "memory_peak_kb": 222432,
  "guest_mmap_bytes": 0,
  "guest_heap_bytes": 0,
  "io_read_bytes": 17291,
  "io_write_bytes": 18,
  "limit_reached": false,
  "exit_code": 0,
  "stdout": "Q1RGe3Rlc3RfZW52X3Zhcn0K",
  "stderr": ""
}
```

## Project Structure

```
ctf-arena/
├── api/                      # Rust API server (Axum)
│   ├── src/
│   │   ├── main.rs          # Routes, endpoints, benchmarks
│   │   ├── auth.rs          # GitHub OAuth
│   │   ├── challenges.rs    # Challenge management
│   │   ├── db.rs            # PostgreSQL + SQLx
│   │   ├── queue.rs         # NATS JetStream
│   │   ├── config.rs        # Environment config
│   │   └── error.rs         # Error handling
│   ├── tests/               # Benchmark source files
│   └── Cargo.toml
├── worker/                   # Execute worker
│   └── src/main.rs          # QEMU sandbox execution
├── compile-worker/           # Compile worker
│   └── src/main.rs          # Docker compilation
├── compiler/                 # Multi-language compiler image
│   ├── Dockerfile           # ~15-20GB image
│   ├── compile.sh           # Entry dispatcher
│   └── scripts/             # Per-language compile scripts
├── sandbox/                  # QEMU sandbox
│   ├── Dockerfile
│   ├── entrypoint.sh        # Env var forwarding to QEMU
│   ├── sandbox.py           # Python wrapper
│   └── plugin/
│       └── sandbox.c        # TCG plugin (instruction + syscall counting)
├── web/                      # Svelte 5 frontend
│   ├── src/
│   │   ├── routes/          # SvelteKit routes
│   │   └── lib/
│   │       ├── components/  # UI components
│   │       ├── api/         # API client
│   │       └── benchmarks.ts
│   └── package.json
├── k8s/
│   ├── base/                # Base manifests
│   └── overlays/
│       ├── local/           # Kind cluster
│       └── production/      # Production
└── tests/                   # Integration tests
```

## Environment Variables

### API
| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Listen port |
| `NATS_URL` | `nats://localhost:4222` | NATS server |
| `DATABASE_URL` | (required) | PostgreSQL connection |
| `GITHUB_CLIENT_ID` | | OAuth client ID |
| `GITHUB_CLIENT_SECRET` | | OAuth client secret |
| `SESSION_SECRET` | | Cookie signing secret |
| `FRONTEND_URL` | `http://localhost:8080` | For OAuth redirect |

### Workers
| Variable | Default | Description |
|----------|---------|-------------|
| `NATS_URL` | `nats://localhost:4222` | NATS server |
| `DOCKER_HOST` | | Docker daemon (DinD) |
| `SANDBOX_IMAGE` | `sandbox:latest` | Execution sandbox |
| `COMPILER_IMAGE` | `compiler:latest` | Compiler image |

## Instruction Count Reference

| Language | Hello World | Port Scanner |
|----------|------------:|-------------:|
| Assembly | ~50 | 51 |
| Zig | ~500 | 594 |
| C (musl) | ~650 | 2,008 |
| Nim | ~20,000 | 23,580 |
| Rust | ~20,000 | 24,530 |
| Pascal | ~25,000 | 30,375 |
| Swift | ~40,000 | 45,503 |
| OCaml | ~250,000 | 295,270 |
| Haskell | ~500,000 | 558,511 |
| Scala | ~900,000 | 922,764 |
| Go | ~1,000,000 | 1,120,285 |
| Java | ~1,100,000 | 1,134,302 |
| C# | ~3,000,000 | 3,203,528 |
| Bun | ~17,000,000 | 17,653,512 |
| Deno | ~130,000,000 | 130,547,222 |
| Node | ~176,000,000 | 176,107,839 |

## Troubleshooting

### Port forwards not working
```bash
pkill -f "port-forward.*ctf-arena"
kubectl port-forward -n ctf-arena svc/ctf-api 3000:3000 &
kubectl port-forward -n ctf-arena svc/ctf-web 8080:80 &
```

### Sandbox image not loaded
```bash
WORKER_POD=$(kubectl get pods -n ctf-arena -l app=ctf-worker -o jsonpath='{.items[0].metadata.name}')
docker save localhost:5001/sandbox:latest | kubectl exec -n ctf-arena -i "$WORKER_POD" -c dind -- docker load
```

### Compiler image not loaded
```bash
COMPILE_POD=$(kubectl get pods -n ctf-arena -l app=compile-worker -o jsonpath='{.items[0].metadata.name}')
docker save compiler:latest | kubectl exec -n ctf-arena -i "$COMPILE_POD" -c dind -- docker load
```

### Database connection issues
```bash
kubectl logs -n ctf-arena deployment/ctf-api | tail -20
# Check DATABASE_URL in configmap
kubectl get configmap ctf-api-config -n ctf-arena -o yaml
```

### GitHub OAuth not working
1. Ensure `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET` are set in configmap or api/.env
2. Verify callback URL matches GitHub OAuth app settings
3. Check API logs: `kubectl logs -n ctf-arena deployment/ctf-api`
