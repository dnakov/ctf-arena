use async_nats::jetstream::{self, consumer::PullConsumer, kv::Store};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

const COMPILES_STREAM: &str = "COMPILES";
const COMPILES_KV: &str = "compiles";
const COMPILE_CACHE_KV: &str = "compile_cache";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    // Tier 1: Native compilation
    C,
    Cpp,
    Rust,
    Go,
    Zig,
    Asm,
    Nim,
    Pascal,
    Ocaml,
    Swift,
    Haskell,
    Csharp,
    // Tier 2: JVM -> Native (GraalVM)
    Java,
    Kotlin,
    Scala,
    Clojure,
    // Tier 3: Scripting -> Bundle
    Python,
    Javascript,
    Typescript,
    Bun,
    Deno,
    Node,
    Lua,
    Perl,
    Php,
    Tcl,
    // Tier 4: Special runtimes
    Erlang,
    Elixir,
    Racket,
    Wasm,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Zig => "zig",
            Language::Asm => "asm",
            Language::Nim => "nim",
            Language::Pascal => "pascal",
            Language::Ocaml => "ocaml",
            Language::Swift => "swift",
            Language::Haskell => "haskell",
            Language::Csharp => "csharp",
            Language::Java => "java",
            Language::Kotlin => "kotlin",
            Language::Scala => "scala",
            Language::Clojure => "clojure",
            Language::Python => "python",
            Language::Javascript => "javascript",
            Language::Typescript => "typescript",
            Language::Bun => "bun",
            Language::Deno => "deno",
            Language::Node => "node",
            Language::Lua => "lua",
            Language::Perl => "perl",
            Language::Php => "php",
            Language::Tcl => "tcl",
            Language::Erlang => "erlang",
            Language::Elixir => "elixir",
            Language::Racket => "racket",
            Language::Wasm => "wasm",
        }
    }

    fn source_extension(&self) -> &'static str {
        match self {
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Rust => "rs",
            Language::Go => "go",
            Language::Zig => "zig",
            Language::Asm => "S",
            Language::Nim => "nim",
            Language::Pascal => "pas",
            Language::Ocaml => "ml",
            Language::Swift => "swift",
            Language::Haskell => "hs",
            Language::Csharp => "cs",
            Language::Java => "java",
            Language::Kotlin => "kt",
            Language::Scala => "scala",
            Language::Clojure => "clj",
            Language::Python => "py",
            Language::Javascript => "js",
            Language::Typescript => "ts",
            Language::Bun => "ts",
            Language::Deno => "ts",
            Language::Node => "js",
            Language::Lua => "lua",
            Language::Perl => "pl",
            Language::Php => "php",
            Language::Tcl => "tcl",
            Language::Erlang => "erl",
            Language::Elixir => "ex",
            Language::Racket => "rkt",
            Language::Wasm => "wat",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Optimization {
    Debug,
    #[default]
    Release,
    Size,
}

impl Optimization {
    fn as_str(&self) -> &'static str {
        match self {
            Optimization::Debug => "debug",
            Optimization::Release => "release",
            Optimization::Size => "size",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileJob {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub source_code: String,
    pub language: Language,
    pub optimization: Optimization,
    #[serde(default)]
    pub flags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompileStatus {
    Pending,
    Compiling,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileMetadata {
    pub status: CompileStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub position: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub binary_id: String,
    pub binary_size: usize,
    pub compile_time_ms: u64,
    pub cached: bool,
}

struct Config {
    nats_url: String,
    api_url: String,
    compiler_image: String,
    memory_limit_mb: u32,
    timeout_sec: u64,
    job_ttl_seconds: u64,
    binary_ttl_seconds: u64,
}

impl Config {
    fn from_env() -> Self {
        Self {
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            api_url: env::var("API_URL").unwrap_or_else(|_| "http://ctf-api:3000".to_string()),
            compiler_image: env::var("COMPILER_IMAGE").unwrap_or_else(|_| "compiler".to_string()),
            memory_limit_mb: env::var("COMPILE_MEMORY_LIMIT_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4096),
            timeout_sec: env::var("COMPILE_TIMEOUT_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
            job_ttl_seconds: env::var("JOB_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            binary_ttl_seconds: env::var("BINARY_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400),
        }
    }
}

fn compute_cache_key(source: &str, language: Language, optimization: Optimization, flags: &HashMap<String, String>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(language.as_str().as_bytes());
    hasher.update(optimization.as_str().as_bytes());
    // Sort flags for consistent hashing
    let mut flag_pairs: Vec<_> = flags.iter().collect();
    flag_pairs.sort_by_key(|(k, _)| *k);
    for (k, v) in flag_pairs {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b";");
    }
    hex::encode(hasher.finalize())
}

fn compute_binary_id(binary: &[u8]) -> String {
    let hash = Sha256::digest(binary);
    format!("sha256-{}", hex::encode(hash))
}

struct CompileOutput {
    binary: Vec<u8>,
    compiler_version: Option<String>,
    compile_flags: Option<serde_json::Value>,
}

async fn compile_source(job: &CompileJob, config: &Config) -> Result<CompileOutput, String> {
    // Create temp directory for compilation
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let work_dir = temp_dir.path();

    // Write source file
    let source_filename = format!("source.{}", job.language.source_extension());
    let source_path = work_dir.join(&source_filename);

    let mut file = tokio::fs::File::create(&source_path)
        .await
        .map_err(|e| format!("Failed to create source file: {}", e))?;
    file.write_all(job.source_code.as_bytes())
        .await
        .map_err(|e| format!("Failed to write source: {}", e))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to sync source: {}", e))?;
    drop(file);

    // Build docker command
    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        &format!("--memory={}m", config.memory_limit_mb),
        &format!("--memory-swap={}m", config.memory_limit_mb),
        // Network access needed for package managers (NuGet, Maven, Hackage, etc.)
        // Execution still runs sandboxed with --network=none
        "--tmpfs=/tmp:rw,exec,nosuid,size=512m",
        "-v",
        &format!("{}:/work:rw", work_dir.display()),
        "-e",
        &format!("LANGUAGE={}", job.language.as_str()),
        "-e",
        &format!("OPTIMIZATION={}", job.optimization.as_str()),
        "-e",
        &format!("SOURCE_FILE={}", source_filename),
        "-e",
        "OUTPUT_FILE=output",
    ]);

    // Pass flags as environment variables (FLAG_<name>=<value>)
    for (key, value) in &job.flags {
        // Sanitize key: only alphanumeric and underscore
        let safe_key: String = key.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !safe_key.is_empty() {
            cmd.args(["-e", &format!("FLAG_{}={}", safe_key.to_uppercase(), value)]);
        }
    }

    // Also pass flags as JSON for complex parsing
    if !job.flags.is_empty() {
        let flags_json = serde_json::to_string(&job.flags).unwrap_or_default();
        cmd.args(["-e", &format!("FLAGS_JSON={}", flags_json)]);
    }

    cmd.arg(&config.compiler_image);

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn docker: {}", e))?;

    // Wait with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(config.timeout_sec),
        child.wait_with_output(),
    )
    .await;

    let output = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(format!("Compilation failed: {}", e)),
        Err(_) => return Err(format!("Compilation timed out after {} seconds", config.timeout_sec)),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Compilation failed (exit {})\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        ));
    }

    // Read compiled binary
    let output_path = work_dir.join("output");
    let binary = tokio::fs::read(&output_path)
        .await
        .map_err(|e| format!("Failed to read compiled binary: {}", e))?;

    if binary.is_empty() {
        return Err("Compilation produced empty binary".to_string());
    }

    // Read compiler version
    let version_path = work_dir.join("compiler_version.txt");
    let compiler_version = tokio::fs::read_to_string(&version_path)
        .await
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Read compile flags
    let flags_path = work_dir.join("compile_flags.json");
    let compile_flags = tokio::fs::read_to_string(&flags_path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    Ok(CompileOutput {
        binary,
        compiler_version,
        compile_flags,
    })
}

async fn update_compile_status(
    compiles_kv: &Store,
    job_id: &Uuid,
    status: CompileStatus,
    error: Option<String>,
) -> Result<(), String> {
    let key = job_id.to_string();

    // Get current metadata
    let entry = compiles_kv
        .get(&key)
        .await
        .map_err(|e| format!("Failed to get compile metadata: {}", e))?
        .ok_or_else(|| format!("Compile job {} not found", job_id))?;

    let mut metadata: CompileMetadata =
        serde_json::from_slice(&entry).map_err(|e| format!("Failed to parse metadata: {}", e))?;

    metadata.status = status;
    match status {
        CompileStatus::Compiling => {
            metadata.started_at = Some(Utc::now());
        }
        CompileStatus::Completed | CompileStatus::Failed => {
            metadata.completed_at = Some(Utc::now());
            metadata.error = error;
        }
        _ => {}
    }

    compiles_kv
        .put(
            &key,
            serde_json::to_vec(&metadata)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?
                .into(),
        )
        .await
        .map_err(|e| format!("Failed to update compile status: {}", e))?;

    Ok(())
}

async fn store_compile_result(
    http_client: &reqwest::Client,
    api_url: &str,
    compile_cache_kv: &Store,
    cache_key: &str,
    binary: &[u8],
    compile_time_ms: u64,
    language: Language,
    optimization: Optimization,
    compiler_version: Option<&str>,
    compile_flags: Option<&serde_json::Value>,
) -> Result<CompileResult, String> {
    let binary_id = compute_binary_id(binary);
    let binary_size = binary.len();

    // Build URL with metadata query parameters
    let mut url = format!(
        "{}/binaries/{}?language={}&optimization={}",
        api_url, binary_id, language.as_str(), optimization.as_str()
    );
    if let Some(version) = compiler_version {
        url.push_str(&format!(
            "&compiler_version={}",
            urlencoding::encode(version)
        ));
    }
    if let Some(flags) = compile_flags {
        if let Ok(flags_json) = serde_json::to_string(flags) {
            url.push_str(&format!(
                "&compile_flags={}",
                urlencoding::encode(&flags_json)
            ));
        }
    }

    // Store binary via HTTP API (PostgreSQL backend, more reliable than NATS KV for large files)
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 3;
    loop {
        attempts += 1;
        let result = http_client
            .put(&url)
            .body(binary.to_vec())
            .timeout(Duration::from_secs(120))
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => break,
            Ok(resp) if attempts < MAX_ATTEMPTS => {
                warn!("Binary store attempt {} failed: HTTP {}, retrying...", attempts, resp.status());
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(resp) => return Err(format!("Failed to store binary after {} attempts: HTTP {}", attempts, resp.status())),
            Err(e) if attempts < MAX_ATTEMPTS => {
                warn!("Binary store attempt {} failed: {}, retrying...", attempts, e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => return Err(format!("Failed to store binary after {} attempts: {}", attempts, e)),
        }
    }

    // Store cache mapping in NATS (small data, no timeout issues)
    let result = CompileResult {
        binary_id: binary_id.clone(),
        binary_size,
        compile_time_ms,
        cached: false,
    };

    compile_cache_kv
        .put(
            cache_key,
            serde_json::to_vec(&result)
                .map_err(|e| format!("Failed to serialize cache entry: {}", e))?
                .into(),
        )
        .await
        .map_err(|e| format!("Failed to store cache entry: {}", e))?;

    Ok(result)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("compile_worker=info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();

    info!(
        "Starting Compile Worker (NATS: {}, compiler: {})",
        config.nats_url, config.compiler_image
    );

    // Connect to NATS with longer request timeout for large binary uploads
    let client = loop {
        let nats_options = async_nats::ConnectOptions::new()
            .request_timeout(Some(Duration::from_secs(120))); // 120s timeout for KV puts

        match nats_options.connect(&config.nats_url).await {
            Ok(c) => {
                info!("Connected to NATS at {}", config.nats_url);
                break c;
            }
            Err(e) => {
                error!("Failed to connect to NATS: {}. Retrying in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    let jetstream = jetstream::new(client);

    // Get or create stream
    let stream = jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: COMPILES_STREAM.to_string(),
            subjects: vec!["compiles.submit".to_string()],
            retention: jetstream::stream::RetentionPolicy::WorkQueue,
            max_age: Duration::from_secs(config.job_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create/get COMPILES stream");

    // Get or create KV buckets
    let compiles_kv = jetstream
        .create_key_value(jetstream::kv::Config {
            bucket: COMPILES_KV.to_string(),
            max_age: Duration::from_secs(config.job_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create compiles KV");

    // HTTP client for binary storage via API (PostgreSQL backend)
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    let compile_cache_kv = jetstream
        .create_key_value(jetstream::kv::Config {
            bucket: COMPILE_CACHE_KV.to_string(),
            max_age: Duration::from_secs(config.binary_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create compile_cache KV");

    // Create durable consumer
    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            "compile-worker",
            jetstream::consumer::pull::Config {
                durable_name: Some("compile-worker".to_string()),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                max_deliver: 3,
                ack_wait: Duration::from_secs(config.timeout_sec + 60),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create consumer");

    info!("Compile Worker ready, waiting for jobs...");

    // Process messages
    loop {
        let mut messages = match consumer.fetch().max_messages(1).messages().await {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to fetch messages: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        while let Some(msg_result) = messages.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to receive message: {}", e);
                    continue;
                }
            };

            let job: CompileJob = match serde_json::from_slice(&msg.payload) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to parse compile job: {}", e);
                    let _ = msg.ack().await;
                    continue;
                }
            };

            info!(
                job_id = %job.id,
                language = ?job.language,
                optimization = ?job.optimization,
                source_size = job.source_code.len(),
                "Processing compile job"
            );

            let start = Instant::now();
            let cache_key = compute_cache_key(&job.source_code, job.language, job.optimization, &job.flags);

            // Check cache first
            if let Ok(Some(cached_entry)) = compile_cache_kv.get(&cache_key).await {
                if let Ok(mut cached_result) = serde_json::from_slice::<CompileResult>(&cached_entry)
                {
                    info!(
                        job_id = %job.id,
                        binary_id = %cached_result.binary_id,
                        "Cache hit"
                    );

                    cached_result.cached = true;

                    // Store result for this job
                    let result_key = format!("{}_result", job.id);
                    if let Err(e) = compiles_kv
                        .put(
                            &result_key,
                            serde_json::to_vec(&cached_result).unwrap().into(),
                        )
                        .await
                    {
                        error!("Failed to store cached result: {}", e);
                    }

                    // Update status to completed
                    if let Err(e) =
                        update_compile_status(&compiles_kv, &job.id, CompileStatus::Completed, None)
                            .await
                    {
                        error!("Failed to update compile status: {}", e);
                    }

                    let _ = msg.ack().await;
                    continue;
                }
            }

            // Update status to compiling
            if let Err(e) =
                update_compile_status(&compiles_kv, &job.id, CompileStatus::Compiling, None).await
            {
                error!("Failed to update compile status: {}", e);
            }

            // Compile the source
            match compile_source(&job, &config).await {
                Ok(output) => {
                    let compile_time_ms = start.elapsed().as_millis() as u64;

                    info!(
                        job_id = %job.id,
                        binary_size = output.binary.len(),
                        compiler_version = ?output.compiler_version,
                        time_ms = compile_time_ms,
                        "Compilation succeeded"
                    );

                    // Store binary and cache entry
                    match store_compile_result(
                        &http_client,
                        &config.api_url,
                        &compile_cache_kv,
                        &cache_key,
                        &output.binary,
                        compile_time_ms,
                        job.language,
                        job.optimization,
                        output.compiler_version.as_deref(),
                        output.compile_flags.as_ref(),
                    )
                    .await
                    {
                        Ok(result) => {
                            // Store result for this job
                            let result_key = format!("{}_result", job.id);
                            if let Err(e) = compiles_kv
                                .put(&result_key, serde_json::to_vec(&result).unwrap().into())
                                .await
                            {
                                error!("Failed to store result: {}", e);
                            }

                            // Update status to completed
                            if let Err(e) = update_compile_status(
                                &compiles_kv,
                                &job.id,
                                CompileStatus::Completed,
                                None,
                            )
                            .await
                            {
                                error!("Failed to update compile status: {}", e);
                            }
                        }
                        Err(e) => {
                            error!(job_id = %job.id, error = %e, "Failed to store compile result");
                            if let Err(e2) = update_compile_status(
                                &compiles_kv,
                                &job.id,
                                CompileStatus::Failed,
                                Some(e),
                            )
                            .await
                            {
                                error!("Failed to update compile status: {}", e2);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(job_id = %job.id, error = %e, "Compilation failed");

                    // Update status to failed
                    if let Err(e2) = update_compile_status(
                        &compiles_kv,
                        &job.id,
                        CompileStatus::Failed,
                        Some(e),
                    )
                    .await
                    {
                        error!("Failed to update compile status: {}", e2);
                    }
                }
            }

            // Acknowledge the message
            if let Err(e) = msg.ack().await {
                error!("Failed to ack message: {}", e);
            }
        }

        // Small delay before next fetch
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
