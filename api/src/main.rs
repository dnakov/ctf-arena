mod auth;
mod challenges;
mod config;
mod db;
mod error;
mod queue;
mod sandbox;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, Method},
    routing::{get, post, put},
    Json, Router,
};
use db::{BinaryMetadata, Run, SaveRunRequest};
use chrono::Utc;
use config::Config;
use error::ApiError;
use queue::{CompileJob, CompileStatus, Job, JobStatus, Language, Optimization, QueueClient};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};
use sha2::Digest;
use tracing::{info, warn};
use uuid::Uuid;

pub struct AppState {
    pub config: Config,
    pub semaphore: Semaphore,
    pub queue: Option<QueueClient>,
    pub db: Option<PgPool>,
    pub auth_config: Option<auth::AuthConfig>,
}

// ============ Benchmark Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkDef {
    id: String,
    name: String,
    description: String,
    implementations: Vec<BenchmarkImpl>,
    #[serde(default)]
    env_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    stdin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkImpl {
    language: String,
    name: String,
    file: String,
    tier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_instructions: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkImplWithSource {
    language: String,
    name: String,
    file: String,
    tier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_instructions: Option<u64>,
    source_code: String,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkWithSource {
    id: String,
    name: String,
    description: String,
    implementations: Vec<BenchmarkImplWithSource>,
    env_vars: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdin: Option<String>,
}

#[derive(Deserialize)]
struct BenchmarkQuery {
    #[serde(default)]
    include_source: bool,
}

fn get_benchmarks_config() -> Vec<BenchmarkDef> {
    vec![
        // Hello World benchmark
        BenchmarkDef {
            id: "hello-world".to_string(),
            name: "Hello World".to_string(),
            description: "Print \"Hello, World!\" followed by a newline. The simplest benchmark.".to_string(),
            env_vars: std::collections::HashMap::new(),
            stdin: None,
            implementations: vec![
                BenchmarkImpl { language: "asm".to_string(), name: "Assembly".to_string(), file: "hello.S".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "c".to_string(), name: "C (musl)".to_string(), file: "hello.c".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "cpp".to_string(), name: "C++".to_string(), file: "hello.cpp".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "rust".to_string(), name: "Rust".to_string(), file: "hello.rs".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "go".to_string(), name: "Go".to_string(), file: "hello.go".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "zig".to_string(), name: "Zig".to_string(), file: "hello.zig".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "nim".to_string(), name: "Nim".to_string(), file: "hello.nim".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "pascal".to_string(), name: "Pascal".to_string(), file: "hello.pas".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "swift".to_string(), name: "Swift".to_string(), file: "hello.swift".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "ocaml".to_string(), name: "OCaml".to_string(), file: "hello.ml".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "haskell".to_string(), name: "Haskell".to_string(), file: "hello.hs".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "java".to_string(), name: "Java (GraalVM)".to_string(), file: "hello.java".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "kotlin".to_string(), name: "Kotlin (GraalVM)".to_string(), file: "hello.kt".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "scala".to_string(), name: "Scala (GraalVM)".to_string(), file: "hello.scala".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "clojure".to_string(), name: "Clojure (GraalVM)".to_string(), file: "hello.clj".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "csharp".to_string(), name: "C# (.NET AOT)".to_string(), file: "hello.cs".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "python".to_string(), name: "Python".to_string(), file: "hello.py".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "bun".to_string(), name: "JavaScript (Bun)".to_string(), file: "hello.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "typescript".to_string(), name: "TypeScript (Bun)".to_string(), file: "hello.ts".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "node".to_string(), name: "JavaScript (Node)".to_string(), file: "hello.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "deno".to_string(), name: "TypeScript (Deno)".to_string(), file: "hello.ts".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "lua".to_string(), name: "Lua".to_string(), file: "hello.lua".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "perl".to_string(), name: "Perl".to_string(), file: "hello.pl".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "racket".to_string(), name: "Racket".to_string(), file: "hello.rkt".to_string(), tier: "special".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "erlang".to_string(), name: "Erlang".to_string(), file: "hello.erl".to_string(), tier: "special".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "elixir".to_string(), name: "Elixir".to_string(), file: "hello.exs".to_string(), tier: "special".to_string(), reference_instructions: None },
            ],
        },
        // Env Leak benchmark
        BenchmarkDef {
            id: "env-leak".to_string(),
            name: "Environment Variable Leak".to_string(),
            description: "Read the FLAG environment variable and print its value.".to_string(),
            env_vars: [("FLAG".to_string(), "CTF{env_leak_test}".to_string())].into_iter().collect(),
            stdin: None,
            implementations: vec![
                BenchmarkImpl { language: "asm".to_string(), name: "Assembly".to_string(), file: "envleak.S".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "c".to_string(), name: "C (musl)".to_string(), file: "envleak.c".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "cpp".to_string(), name: "C++".to_string(), file: "envleak.cpp".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "rust".to_string(), name: "Rust".to_string(), file: "envleak.rs".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "go".to_string(), name: "Go".to_string(), file: "envleak.go".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "zig".to_string(), name: "Zig".to_string(), file: "envleak.zig".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "nim".to_string(), name: "Nim".to_string(), file: "envleak.nim".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "pascal".to_string(), name: "Pascal".to_string(), file: "envleak.pas".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "swift".to_string(), name: "Swift".to_string(), file: "envleak.swift".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "ocaml".to_string(), name: "OCaml".to_string(), file: "envleak.ml".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "haskell".to_string(), name: "Haskell".to_string(), file: "envleak.hs".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "java".to_string(), name: "Java (GraalVM)".to_string(), file: "envleak.java".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "kotlin".to_string(), name: "Kotlin (GraalVM)".to_string(), file: "envleak.kt".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "scala".to_string(), name: "Scala (GraalVM)".to_string(), file: "envleak.scala".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "clojure".to_string(), name: "Clojure (GraalVM)".to_string(), file: "envleak.clj".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "csharp".to_string(), name: "C# (.NET AOT)".to_string(), file: "envleak.cs".to_string(), tier: "managed".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "python".to_string(), name: "Python".to_string(), file: "envleak.py".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "bun".to_string(), name: "JavaScript (Bun)".to_string(), file: "envleak.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "typescript".to_string(), name: "TypeScript (Bun)".to_string(), file: "envleak.ts".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "node".to_string(), name: "JavaScript (Node)".to_string(), file: "envleak.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "deno".to_string(), name: "TypeScript (Deno)".to_string(), file: "envleak_deno.ts".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "lua".to_string(), name: "Lua".to_string(), file: "envleak.lua".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "perl".to_string(), name: "Perl".to_string(), file: "envleak.pl".to_string(), tier: "scripting".to_string(), reference_instructions: None },
            ],
        },
        // Base64 Decode benchmark
        BenchmarkDef {
            id: "base64-decode".to_string(),
            name: "Base64 Decode".to_string(),
            description: "Decode a base64-encoded string from stdin and print the decoded output.".to_string(),
            env_vars: std::collections::HashMap::new(),
            stdin: Some("SGVsbG8sIFdvcmxkIQ==".to_string()), // "Hello, World!" in base64
            implementations: vec![
                BenchmarkImpl { language: "c".to_string(), name: "C (musl)".to_string(), file: "base64.c".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "rust".to_string(), name: "Rust".to_string(), file: "base64.rs".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "go".to_string(), name: "Go".to_string(), file: "base64.go".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "zig".to_string(), name: "Zig".to_string(), file: "base64.zig".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "nim".to_string(), name: "Nim".to_string(), file: "base64.nim".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "python".to_string(), name: "Python".to_string(), file: "base64.py".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "bun".to_string(), name: "JavaScript (Bun)".to_string(), file: "base64.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "node".to_string(), name: "JavaScript (Node)".to_string(), file: "base64.js".to_string(), tier: "scripting".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "lua".to_string(), name: "Lua".to_string(), file: "base64.lua".to_string(), tier: "scripting".to_string(), reference_instructions: None },
            ],
        },
        // Port Scanner benchmark
        BenchmarkDef {
            id: "portscan".to_string(),
            name: "Port Scanner".to_string(),
            description: "Scan localhost (127.0.0.1) on ports 22, 80, 443. Print \"<port> open\" for each open port.".to_string(),
            env_vars: std::collections::HashMap::new(),
            stdin: None,
            implementations: vec![
                BenchmarkImpl { language: "asm".to_string(), name: "Assembly (minimal)".to_string(), file: "portscan_min.S".to_string(), tier: "native".to_string(), reference_instructions: Some(51) },
                BenchmarkImpl { language: "asm".to_string(), name: "Assembly (optimized)".to_string(), file: "portscan_opt.S".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "asm".to_string(), name: "Assembly (with function)".to_string(), file: "portscan_asm.S".to_string(), tier: "native".to_string(), reference_instructions: None },
                BenchmarkImpl { language: "c".to_string(), name: "C (musl)".to_string(), file: "portscan.c".to_string(), tier: "native".to_string(), reference_instructions: Some(2008) },
                BenchmarkImpl { language: "c".to_string(), name: "C (raw syscalls)".to_string(), file: "portscan_raw.c".to_string(), tier: "native".to_string(), reference_instructions: Some(1525) },
                BenchmarkImpl { language: "cpp".to_string(), name: "C++".to_string(), file: "portscan.cpp".to_string(), tier: "native".to_string(), reference_instructions: Some(535110) },
                BenchmarkImpl { language: "rust".to_string(), name: "Rust".to_string(), file: "portscan.rs".to_string(), tier: "native".to_string(), reference_instructions: Some(24530) },
                BenchmarkImpl { language: "go".to_string(), name: "Go".to_string(), file: "portscan.go".to_string(), tier: "native".to_string(), reference_instructions: Some(1120285) },
                BenchmarkImpl { language: "zig".to_string(), name: "Zig".to_string(), file: "portscan.zig".to_string(), tier: "native".to_string(), reference_instructions: Some(594) },
                BenchmarkImpl { language: "nim".to_string(), name: "Nim".to_string(), file: "portscan.nim".to_string(), tier: "native".to_string(), reference_instructions: Some(23580) },
                BenchmarkImpl { language: "pascal".to_string(), name: "Pascal".to_string(), file: "portscan.pas".to_string(), tier: "native".to_string(), reference_instructions: Some(30375) },
                BenchmarkImpl { language: "swift".to_string(), name: "Swift".to_string(), file: "portscan.swift".to_string(), tier: "native".to_string(), reference_instructions: Some(45503) },
                BenchmarkImpl { language: "ocaml".to_string(), name: "OCaml".to_string(), file: "portscan.ml".to_string(), tier: "native".to_string(), reference_instructions: Some(295270) },
                BenchmarkImpl { language: "haskell".to_string(), name: "Haskell".to_string(), file: "portscan.hs".to_string(), tier: "native".to_string(), reference_instructions: Some(558511) },
                BenchmarkImpl { language: "java".to_string(), name: "Java (GraalVM)".to_string(), file: "portscan.java".to_string(), tier: "managed".to_string(), reference_instructions: Some(1134302) },
                BenchmarkImpl { language: "kotlin".to_string(), name: "Kotlin (GraalVM)".to_string(), file: "portscan.kt".to_string(), tier: "managed".to_string(), reference_instructions: Some(1219305) },
                BenchmarkImpl { language: "scala".to_string(), name: "Scala (GraalVM)".to_string(), file: "portscan.scala".to_string(), tier: "managed".to_string(), reference_instructions: Some(922764) },
                BenchmarkImpl { language: "clojure".to_string(), name: "Clojure (GraalVM)".to_string(), file: "portscan.clj".to_string(), tier: "managed".to_string(), reference_instructions: Some(1147912) },
                BenchmarkImpl { language: "csharp".to_string(), name: "C# (.NET AOT)".to_string(), file: "portscan.cs".to_string(), tier: "managed".to_string(), reference_instructions: Some(3203528) },
                BenchmarkImpl { language: "python".to_string(), name: "Python".to_string(), file: "portscan.py".to_string(), tier: "scripting".to_string(), reference_instructions: Some(376920746) },
                BenchmarkImpl { language: "bun".to_string(), name: "JavaScript (Bun)".to_string(), file: "portscan.js".to_string(), tier: "scripting".to_string(), reference_instructions: Some(17653512) },
                BenchmarkImpl { language: "typescript".to_string(), name: "TypeScript (Bun)".to_string(), file: "portscan.ts".to_string(), tier: "scripting".to_string(), reference_instructions: Some(17653512) },
                BenchmarkImpl { language: "node".to_string(), name: "JavaScript (Node)".to_string(), file: "portscan.js".to_string(), tier: "scripting".to_string(), reference_instructions: Some(176107839) },
                BenchmarkImpl { language: "deno".to_string(), name: "TypeScript (Deno)".to_string(), file: "portscan_deno.ts".to_string(), tier: "scripting".to_string(), reference_instructions: Some(130547222) },
                BenchmarkImpl { language: "lua".to_string(), name: "Lua".to_string(), file: "portscan.lua".to_string(), tier: "scripting".to_string(), reference_instructions: Some(410182) },
            ],
        },
    ]
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    docker_available: bool,
    nats_connected: bool,
    db_connected: bool,
}

#[derive(Serialize)]
struct SubmitResponse {
    job_id: Uuid,
    status: &'static str,
    position: Option<u64>,
}

#[derive(Serialize)]
struct StatusResponse {
    job_id: Uuid,
    status: String,
    position: Option<u64>,
    created_at: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct QueueStatsResponse {
    queue_length: u64,
    active_jobs: u64,
    workers_online: u64,
}

// ============ Compile Response Types ============

#[derive(Serialize)]
struct CompileSubmitResponse {
    compile_job_id: Uuid,
    status: &'static str,
    position: Option<u64>,
}

#[derive(Serialize)]
struct CompileStatusResponse {
    compile_job_id: Uuid,
    status: String,
    position: Option<u64>,
    created_at: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct CompileResultResponse {
    binary_id: String,
    binary_size: usize,
    compile_time_ms: u64,
    cached: bool,
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let docker_available = sandbox::check_docker().await;
    let nats_connected = state.queue.is_some();
    let db_connected = if let Some(ref pool) = state.db {
        sqlx::query("SELECT 1")
            .execute(pool)
            .await
            .is_ok()
    } else {
        false
    };

    let status = if docker_available || nats_connected {
        "ok"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status,
        docker_available,
        nats_connected,
        db_connected,
    })
}

async fn submit(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<SubmitResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    let mut binary: Option<Vec<u8>> = None;
    let mut binary_id: Option<String> = None;
    let mut instruction_limit: Option<u64> = None;
    let mut stdin: Vec<u8> = Vec::new();
    let mut benchmark_id: Option<String> = None;
    let mut env_vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "binary" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                if data.len() > state.config.max_binary_size {
                    return Err(ApiError::BinaryTooLarge {
                        size: data.len(),
                        max: state.config.max_binary_size,
                    });
                }
                binary = Some(data.to_vec());
            }
            "binary_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                binary_id = Some(text);
            }
            "instruction_limit" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                let limit: u64 = text
                    .parse()
                    .map_err(|_| ApiError::InvalidField("instruction_limit must be a number".into()))?;
                if limit > state.config.max_instruction_limit {
                    return Err(ApiError::InstructionLimitTooHigh {
                        limit,
                        max: state.config.max_instruction_limit,
                    });
                }
                instruction_limit = Some(limit);
            }
            "stdin" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                stdin = data.to_vec();
            }
            "benchmark_id" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                benchmark_id = Some(text);
            }
            "env_vars" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                env_vars = serde_json::from_str(&text)
                    .map_err(|e| ApiError::InvalidField(format!("env_vars: {}", e)))?;
            }
            _ => {
                warn!("Unknown field: {}", name);
            }
        }
    }

    // Resolve binary_id (store binary in PostgreSQL if uploaded directly)
    let binary_id_str = if let Some(bid) = binary_id {
        // Verify the binary exists
        if let Some(ref pool) = state.db {
            if db::get_binary(pool, &bid).await?.is_none() {
                return Err(ApiError::BinaryNotFound(bid));
            }
        }
        bid
    } else if let Some(bin) = binary {
        // Store the binary and get its ID
        let pool = state.db.as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;
        let bid = format!("sha256-{}", hex::encode(sha2::Sha256::digest(&bin)));
        db::store_binary(pool, &bid, &bin, None).await?;
        bid
    } else {
        return Err(ApiError::MissingField("binary or binary_id"));
    };
    let instruction_limit = instruction_limit.unwrap_or(state.config.default_instruction_limit);

    // Create job with binary_id reference (not the full binary data)
    let job = Job {
        id: Uuid::new_v4(),
        user_id: None, // TODO: extract from auth
        binary_id: binary_id_str,
        instruction_limit,
        stdin,
        created_at: Utc::now(),
        benchmark_id,
        network_enabled: false,
        env_vars,
    };

    let job_id = job.id;

    // Record submission in database
    if let Some(ref pool) = state.db {
        let _ = db::record_submission(pool, None, &job_id, None).await;
    }

    // Submit to queue
    queue.submit_job(job).await?;

    // Get queue position
    let position = queue.get_queue_depth().await.ok();

    info!(job_id = %job_id, "Job submitted to queue");

    Ok(Json(SubmitResponse {
        job_id,
        status: "queued",
        position,
    }))
}

async fn status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<StatusResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    let metadata = queue
        .get_job_status(&job_id)
        .await?
        .ok_or_else(|| ApiError::JobNotFound(job_id.to_string()))?;

    // Get approximate position for pending jobs
    let position = if metadata.status == JobStatus::Pending {
        queue.get_queue_depth().await.ok()
    } else {
        None
    };

    Ok(Json(StatusResponse {
        job_id,
        status: format!("{:?}", metadata.status).to_lowercase(),
        position,
        created_at: Some(metadata.created_at.to_rfc3339()),
        started_at: metadata.started_at.map(|t| t.to_rfc3339()),
        completed_at: metadata.completed_at.map(|t| t.to_rfc3339()),
        error: metadata.error,
    }))
}

async fn result(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<sandbox::ExecutionResult>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    // Check job status first
    let metadata = queue
        .get_job_status(&job_id)
        .await?
        .ok_or_else(|| ApiError::JobNotFound(job_id.to_string()))?;

    match metadata.status {
        JobStatus::Completed => {
            let result = queue
                .get_job_result(&job_id)
                .await?
                .ok_or(ApiError::JobNotReady)?;
            Ok(Json(result))
        }
        JobStatus::Failed => Err(ApiError::Internal(
            metadata.error.unwrap_or_else(|| "Job failed".to_string()),
        )),
        _ => Err(ApiError::JobNotReady),
    }
}

async fn queue_stats(State(state): State<Arc<AppState>>) -> Result<Json<QueueStatsResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    let queue_length = queue.get_queue_depth().await.unwrap_or(0);

    // TODO: track active jobs and workers in the queue client
    Ok(Json(QueueStatsResponse {
        queue_length,
        active_jobs: 0,
        workers_online: 0,
    }))
}

// ============ Compile Endpoints ============

async fn compile(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<CompileSubmitResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    let mut source_code: Option<String> = None;
    let mut language: Option<Language> = None;
    let mut optimization: Optimization = Optimization::Release;
    let mut flags: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "source_code" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                if text.len() > state.config.max_source_size {
                    return Err(ApiError::SourceTooLarge {
                        size: text.len(),
                        max: state.config.max_source_size,
                    });
                }
                source_code = Some(text);
            }
            "language" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                language = Some(
                    Language::from_str(&text)
                        .ok_or_else(|| ApiError::InvalidLanguage(text.clone()))?,
                );
            }
            "optimization" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                optimization = Optimization::from_str(&text).unwrap_or(Optimization::Release);
            }
            "flags" => {
                // Accept flags as JSON object: {"nostd": "true", "lto": "thin"}
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                flags = serde_json::from_str(&text)
                    .map_err(|e| ApiError::InvalidField(format!("flags must be valid JSON: {}", e)))?;
            }
            _ if name.starts_with("flag_") => {
                // Also accept individual flag fields: flag_nostd=true, flag_lto=thin
                let flag_name = name.strip_prefix("flag_").unwrap().to_string();
                let value = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                flags.insert(flag_name, value);
            }
            _ => {
                warn!("Unknown field: {}", name);
            }
        }
    }

    let source_code = source_code.ok_or(ApiError::MissingField("source_code"))?;
    let language = language.ok_or(ApiError::MissingField("language"))?;

    // Check compile cache first
    if let Ok(Some(cached_result)) = queue
        .check_compile_cache(&source_code, language, optimization, &flags)
        .await
    {
        info!(
            binary_id = %cached_result.binary_id,
            "Compile cache hit"
        );
        // For cache hits, we could return immediately but the client expects a job_id
        // So we still create a job but it will complete instantly via cache
    }

    // Create compile job
    let job = CompileJob {
        id: Uuid::new_v4(),
        user_id: None, // TODO: extract from auth
        source_code,
        language,
        optimization,
        flags,
        created_at: Utc::now(),
    };

    let job_id = job.id;

    // Submit to queue
    queue.submit_compile_job(job).await?;

    // Get queue position
    let position = queue.get_compile_queue_depth().await.ok();

    info!(
        compile_job_id = %job_id,
        language = ?language,
        optimization = ?optimization,
        "Compile job submitted"
    );

    Ok(Json(CompileSubmitResponse {
        compile_job_id: job_id,
        status: "queued",
        position,
    }))
}

async fn compile_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<CompileStatusResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    let metadata = queue
        .get_compile_status(&job_id)
        .await?
        .ok_or_else(|| ApiError::CompileJobNotFound(job_id.to_string()))?;

    // Get approximate position for pending jobs
    let position = if metadata.status == CompileStatus::Pending {
        queue.get_compile_queue_depth().await.ok()
    } else {
        None
    };

    Ok(Json(CompileStatusResponse {
        compile_job_id: job_id,
        status: format!("{:?}", metadata.status).to_lowercase(),
        position,
        created_at: Some(metadata.created_at.to_rfc3339()),
        started_at: metadata.started_at.map(|t| t.to_rfc3339()),
        completed_at: metadata.completed_at.map(|t| t.to_rfc3339()),
        error: metadata.error,
    }))
}

async fn compile_result(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<CompileResultResponse>, ApiError> {
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    // Check job status first
    let metadata = queue
        .get_compile_status(&job_id)
        .await?
        .ok_or_else(|| ApiError::CompileJobNotFound(job_id.to_string()))?;

    match metadata.status {
        CompileStatus::Completed => {
            let result = queue
                .get_compile_result(&job_id)
                .await?
                .ok_or(ApiError::CompileJobNotReady)?;
            Ok(Json(CompileResultResponse {
                binary_id: result.binary_id,
                binary_size: result.binary_size,
                compile_time_ms: result.compile_time_ms,
                cached: result.cached,
            }))
        }
        CompileStatus::Failed => Err(ApiError::CompileError(
            metadata.error.unwrap_or_else(|| "Compilation failed".to_string()),
        )),
        _ => Err(ApiError::CompileJobNotReady),
    }
}

// ============ Benchmark Endpoints ============

async fn list_benchmarks() -> Json<Vec<BenchmarkDef>> {
    Json(get_benchmarks_config())
}

async fn get_benchmark(
    Path(id): Path<String>,
    Query(query): Query<BenchmarkQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let benchmarks = get_benchmarks_config();
    let benchmark = benchmarks
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Benchmark '{}' not found", id)))?;

    if query.include_source {
        let tests_dir = std::path::Path::new("/app/tests");
        let fallback_dir = std::path::Path::new("../sandbox/tests");

        let base_dir = if tests_dir.exists() { tests_dir } else { fallback_dir };

        let implementations: Vec<BenchmarkImplWithSource> = benchmark
            .implementations
            .into_iter()
            .filter_map(|impl_| {
                let file_path = base_dir.join(&impl_.file);
                let source_code = std::fs::read_to_string(&file_path).ok()?;
                Some(BenchmarkImplWithSource {
                    language: impl_.language,
                    name: impl_.name,
                    file: impl_.file,
                    tier: impl_.tier,
                    reference_instructions: impl_.reference_instructions,
                    source_code,
                })
            })
            .collect();

        let result = BenchmarkWithSource {
            id: benchmark.id,
            name: benchmark.name,
            description: benchmark.description,
            implementations,
            env_vars: benchmark.env_vars,
            stdin: benchmark.stdin,
        };
        Ok(Json(serde_json::to_value(result).unwrap()))
    } else {
        Ok(Json(serde_json::to_value(benchmark).unwrap()))
    }
}

async fn get_benchmark_source(
    Path((id, file)): Path<(String, String)>,
) -> Result<String, ApiError> {
    let benchmarks = get_benchmarks_config();
    let benchmark = benchmarks
        .iter()
        .find(|b| b.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Benchmark '{}' not found", id)))?;

    // Verify the file belongs to this benchmark
    if !benchmark.implementations.iter().any(|i| i.file == file) {
        return Err(ApiError::NotFound(format!("File '{}' not in benchmark '{}'", file, id)));
    }

    let tests_dir = std::path::Path::new("/app/tests");
    let fallback_dir = std::path::Path::new("../sandbox/tests");
    let base_dir = if tests_dir.exists() { tests_dir } else { fallback_dir };

    let file_path = base_dir.join(&file);
    std::fs::read_to_string(&file_path)
        .map_err(|_| ApiError::NotFound(format!("File '{}' not found", file)))
}

// ============ Benchmark Stats Endpoint ============

#[derive(Serialize)]
struct BenchmarkStatsResponse {
    min_instructions: std::collections::HashMap<String, i64>,
}

async fn get_benchmark_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<BenchmarkStatsResponse>, ApiError> {
    // Verify benchmark exists
    let benchmarks = get_benchmarks_config();
    if !benchmarks.iter().any(|b| b.id == id) {
        return Err(ApiError::NotFound(format!("Benchmark '{}' not found", id)));
    }

    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let min_instructions = db::get_min_instructions(pool, &id).await?;

    Ok(Json(BenchmarkStatsResponse { min_instructions }))
}

// ============ Runs Endpoints ============

#[derive(Serialize)]
struct SaveRunResponse {
    id: Uuid,
}

#[derive(Deserialize)]
struct ListRunsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

async fn save_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveRunRequest>,
) -> Result<Json<SaveRunResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let id = db::save_run(pool, &req).await?;

    info!(run_id = %id, job_id = %req.job_id, "Run saved");

    Ok(Json(SaveRunResponse { id }))
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<Run>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let run = db::get_run(pool, &run_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run '{}' not found", run_id)))?;

    Ok(Json(run))
}

async fn get_run_by_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<Run>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let run = db::get_run_by_job_id(pool, &job_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Run for job '{}' not found", job_id)))?;

    Ok(Json(run))
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListRunsQuery>,
) -> Result<Json<Vec<Run>>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let limit = query.limit.min(100).max(1); // Cap at 100, minimum 1
    let runs = db::list_runs(pool, limit, query.offset).await?;

    Ok(Json(runs))
}

// Binary storage endpoints (for compile-worker and execute-worker)

#[derive(Serialize)]
struct StoreBinaryResponse {
    success: bool,
}

#[derive(Debug, Deserialize)]
struct StoreBinaryQuery {
    language: Option<String>,
    optimization: Option<String>,
    compiler_version: Option<String>,
    compile_flags: Option<String>, // JSON string
}

async fn store_binary(
    State(state): State<Arc<AppState>>,
    Path(binary_id): Path<String>,
    Query(query): Query<StoreBinaryQuery>,
    body: axum::body::Bytes,
) -> Result<Json<StoreBinaryResponse>, ApiError> {
    info!(
        binary_id = %binary_id,
        body_size = body.len(),
        language = ?query.language,
        optimization = ?query.optimization,
        "Storing binary"
    );

    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let compile_flags = query.compile_flags
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());

    let metadata = BinaryMetadata {
        language: query.language,
        optimization: query.optimization,
        compiler_version: query.compiler_version,
        compile_flags,
    };

    if let Err(e) = db::store_binary(pool, &binary_id, &body, Some(&metadata)).await {
        tracing::error!(binary_id = %binary_id, body_size = body.len(), error = %e, "Failed to store binary");
        return Err(e);
    }

    info!(binary_id = %binary_id, "Binary stored successfully");
    Ok(Json(StoreBinaryResponse { success: true }))
}

async fn get_binary(
    State(state): State<Arc<AppState>>,
    Path(binary_id): Path<String>,
) -> Result<axum::body::Bytes, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let data = db::get_binary(pool, &binary_id)
        .await?
        .ok_or_else(|| ApiError::BinaryNotFound(binary_id))?;

    Ok(axum::body::Bytes::from(data))
}

async fn get_binary_metadata(
    State(state): State<Arc<AppState>>,
    Path(binary_id): Path<String>,
) -> Result<Json<BinaryMetadata>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let metadata = db::get_binary_metadata(pool, &binary_id)
        .await?
        .ok_or_else(|| ApiError::BinaryNotFound(binary_id))?;

    Ok(Json(metadata))
}

/// Deprecated: synchronous execute endpoint for backward compatibility
/// Internally submits job and polls for result
async fn execute(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<sandbox::ExecutionResult>, ApiError> {
    // If queue is available, use async path
    if state.queue.is_some() {
        let mut binary: Option<Vec<u8>> = None;
        let mut instruction_limit: Option<u64> = None;
        let mut stdin: Vec<u8> = Vec::new();

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            let name = field.name().unwrap_or("").to_string();
            match name.as_str() {
                "binary" => {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?;
                    if data.len() > state.config.max_binary_size {
                        return Err(ApiError::BinaryTooLarge {
                            size: data.len(),
                            max: state.config.max_binary_size,
                        });
                    }
                    binary = Some(data.to_vec());
                }
                "instruction_limit" => {
                    let text = field
                        .text()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?;
                    let limit: u64 = text
                        .parse()
                        .map_err(|_| ApiError::InvalidField("instruction_limit must be a number".into()))?;
                    if limit > state.config.max_instruction_limit {
                        return Err(ApiError::InstructionLimitTooHigh {
                            limit,
                            max: state.config.max_instruction_limit,
                        });
                    }
                    instruction_limit = Some(limit);
                }
                "stdin" => {
                    let data = field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?;
                    stdin = data.to_vec();
                }
                _ => {}
            }
        }

        let binary = binary.ok_or(ApiError::MissingField("binary"))?;
        let instruction_limit = instruction_limit.unwrap_or(state.config.default_instruction_limit);

        // Store binary in PostgreSQL
        let pool = state.db.as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;
        let binary_id = format!("sha256-{}", hex::encode(sha2::Sha256::digest(&binary)));
        db::store_binary(pool, &binary_id, &binary, None).await?;

        let queue = state.queue.as_ref().unwrap();

        // Create and submit job with binary_id reference
        let job = Job {
            id: Uuid::new_v4(),
            user_id: None,
            binary_id,
            instruction_limit,
            stdin,
            created_at: Utc::now(),
            benchmark_id: None,
            network_enabled: false,
            env_vars: std::collections::HashMap::new(),
        };
        let job_id = job.id;
        queue.submit_job(job).await?;

        // Poll for result with timeout
        let timeout = Duration::from_secs(state.config.timeout_sec);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(ApiError::Timeout(state.config.timeout_sec));
            }

            if let Some(metadata) = queue.get_job_status(&job_id).await? {
                match metadata.status {
                    JobStatus::Completed => {
                        if let Some(result) = queue.get_job_result(&job_id).await? {
                            return Ok(Json(result));
                        }
                    }
                    JobStatus::Failed => {
                        return Err(ApiError::Internal(
                            metadata.error.unwrap_or_else(|| "Job failed".to_string()),
                        ));
                    }
                    _ => {}
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Fallback: direct execution (original behavior)
    let mut binary: Option<Vec<u8>> = None;
    let mut instruction_limit: Option<u64> = None;
    let mut stdin: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "binary" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                if data.len() > state.config.max_binary_size {
                    return Err(ApiError::BinaryTooLarge {
                        size: data.len(),
                        max: state.config.max_binary_size,
                    });
                }
                binary = Some(data.to_vec());
            }
            "instruction_limit" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                let limit: u64 = text
                    .parse()
                    .map_err(|_| ApiError::InvalidField("instruction_limit must be a number".into()))?;
                if limit > state.config.max_instruction_limit {
                    return Err(ApiError::InstructionLimitTooHigh {
                        limit,
                        max: state.config.max_instruction_limit,
                    });
                }
                instruction_limit = Some(limit);
            }
            "stdin" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                stdin = data.to_vec();
            }
            _ => {
                warn!("Unknown field: {}", name);
            }
        }
    }

    let binary = binary.ok_or(ApiError::MissingField("binary"))?;
    let instruction_limit = instruction_limit.unwrap_or(state.config.default_instruction_limit);

    // Acquire semaphore permit for concurrency control
    let _permit = state
        .semaphore
        .try_acquire()
        .map_err(|_| ApiError::TooManyRequests)?;

    info!(
        binary_size = binary.len(),
        instruction_limit,
        "Executing binary"
    );

    let result = sandbox::execute(binary, instruction_limit, stdin, &state.config).await?;

    info!(
        instructions = result.instructions,
        memory_kb = result.memory_peak_kb,
        exit_code = result.exit_code,
        time_ms = result.execution_time_ms,
        "Execution complete"
    );

    Ok(Json(result))
}

#[tokio::main]
async fn main() {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ctf_sandbox_api=info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();
    let addr = format!("{}:{}", config.host, config.port);

    info!(
        "Starting CTF Sandbox API on {} (max_concurrent: {}, max_binary: {}MB)",
        addr,
        config.max_concurrent,
        config.max_binary_size / 1024 / 1024
    );

    // Try to connect to NATS (optional - fallback to direct execution)
    let queue = match QueueClient::connect(&config.nats_url, config.job_ttl_seconds, config.binary_ttl_seconds).await {
        Ok(q) => {
            info!("Connected to NATS at {}", config.nats_url);
            Some(q)
        }
        Err(e) => {
            warn!("Failed to connect to NATS: {}. Running in direct mode.", e);
            None
        }
    };

    // Try to connect to PostgreSQL (optional)
    let db = match db::create_pool(&config.database_url).await {
        Ok(pool) => {
            if let Err(e) = db::run_migrations(&pool).await {
                warn!("Failed to run migrations: {}", e);
            } else {
                info!("Connected to PostgreSQL and ran migrations");
            }
            // Try to seed challenges
            if let Err(e) = challenges::seed_challenges(&pool).await {
                warn!("Failed to seed challenges: {}", e);
            }
            Some(pool)
        }
        Err(e) => {
            warn!("Failed to connect to PostgreSQL: {}. Submissions won't be persisted.", e);
            None
        }
    };

    // Initialize auth config (optional - requires GitHub OAuth credentials)
    let auth_config = auth::AuthConfig::from_env();
    if auth_config.is_some() {
        info!("GitHub OAuth configured");
    } else {
        warn!("GitHub OAuth not configured. Set GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET to enable.");
    }

    let state = Arc::new(AppState {
        semaphore: Semaphore::new(config.max_concurrent),
        config,
        queue,
        db,
        auth_config,
    });

    // Configure CORS - when using credentials, we can't use wildcards
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let allowed_origins: Vec<_> = frontend_url
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(allowed_origins)
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
            header::COOKIE,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", get(health))
        .route("/execute", post(execute))
        .route("/submit", post(submit))
        .route("/status/:job_id", get(status))
        .route("/result/:job_id", get(result))
        .route("/queue/stats", get(queue_stats))
        // Compile endpoints
        .route("/compile", post(compile))
        .route("/compile/status/:job_id", get(compile_status))
        .route("/compile/result/:job_id", get(compile_result))
        // Binary storage endpoints (for workers)
        .route("/binaries/:binary_id", put(store_binary).get(get_binary))
        .route("/binaries/:binary_id/metadata", get(get_binary_metadata))
        // Benchmark endpoints
        .route("/benchmarks", get(list_benchmarks))
        .route("/benchmarks/:id", get(get_benchmark))
        .route("/benchmarks/:id/source/:file", get(get_benchmark_source))
        .route("/benchmarks/:id/stats", get(get_benchmark_stats))
        // Runs endpoints (permanent storage)
        .route("/runs", post(save_run).get(list_runs))
        .route("/runs/:id", get(get_run))
        .route("/runs/job/:job_id", get(get_run_by_job))
        // Auth endpoints
        .route("/auth/github", get(auth::github_login))
        .route("/auth/github/callback", get(auth::github_callback))
        .route("/auth/me", get(auth::auth_me))
        .route("/auth/logout", post(auth::logout))
        // Clanker verification endpoints
        .route("/verification/clanker", post(auth::init_clanker_verification))
        .route("/verification/clanker/check", post(auth::check_clanker_verification))
        // User profile endpoint
        .route("/users/:username", get(auth::get_user_profile))
        // Challenge endpoints
        .route("/challenges", get(challenges::list_challenges))
        .route("/challenges/:id", get(challenges::get_challenge))
        .route("/challenges/:id/submit", post(challenges::submit_challenge))
        .route("/challenges/:id/submission/:submission_id", get(challenges::get_submission_status))
        .route("/challenges/:id/leaderboard", get(challenges::get_challenge_leaderboard))
        // Global leaderboard
        .route("/leaderboard", get(challenges::get_global_leaderboard))
        .layer(cors)
        .layer(DefaultBodyLimit::max(state.config.max_binary_size + 1024 * 1024))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
