use async_nats::jetstream::{self, consumer::PullConsumer, kv::Store};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info};
use uuid::Uuid;

const JOBS_STREAM: &str = "JOBS";
const JOBS_KV: &str = "jobs";
const RESULTS_KV: &str = "results";

static STATS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n(\{[^\n]+\})\n?$").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    id: Uuid,
    user_id: Option<Uuid>,
    binary_id: String,  // Reference to binary stored in PostgreSQL
    instruction_limit: u64,
    stdin: Vec<u8>,
    created_at: DateTime<Utc>,
    #[serde(default)]
    benchmark_id: Option<String>,
    // Challenge-specific execution options
    #[serde(default)]
    network_enabled: bool,
    #[serde(default)]
    env_vars: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobMetadata {
    status: JobStatus,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PluginStats {
    instructions: u64,
    memory_peak_kb: u64,
    #[serde(default)]
    memory_rss_kb: u64,
    #[serde(default)]
    memory_hwm_kb: u64,
    #[serde(default)]
    memory_data_kb: u64,
    #[serde(default)]
    memory_stack_kb: u64,
    #[serde(default)]
    io_read_bytes: u64,
    #[serde(default)]
    io_write_bytes: u64,
    // Guest memory (actual binary allocations)
    #[serde(default)]
    guest_mmap_bytes: u64,
    #[serde(default)]
    guest_mmap_peak: u64,
    #[serde(default)]
    guest_heap_bytes: u64,
    limit_reached: bool,
    #[serde(default)]
    syscalls: u64,
    #[serde(default)]
    syscall_cost: u64,
    #[serde(default)]
    syscall_breakdown: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionResult {
    instructions: u64,
    memory_peak_kb: u64,
    #[serde(default)]
    memory_rss_kb: u64,
    #[serde(default)]
    memory_hwm_kb: u64,
    #[serde(default)]
    memory_data_kb: u64,
    #[serde(default)]
    memory_stack_kb: u64,
    #[serde(default)]
    io_read_bytes: u64,
    #[serde(default)]
    io_write_bytes: u64,
    // Guest memory (actual binary allocations)
    #[serde(default)]
    guest_mmap_bytes: u64,
    #[serde(default)]
    guest_mmap_peak: u64,
    #[serde(default)]
    guest_heap_bytes: u64,
    limit_reached: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
    execution_time_ms: u64,
    #[serde(default)]
    syscalls: u64,
    #[serde(default)]
    syscall_breakdown: std::collections::HashMap<String, u64>,
}

struct Config {
    nats_url: String,
    api_url: String,
    sandbox_image: String,
    memory_limit_mb: u32,
    timeout_sec: u64,
    job_ttl_seconds: u64,
}

impl Config {
    fn from_env() -> Self {
        Self {
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            api_url: env::var("API_URL").unwrap_or_else(|_| "http://ctf-api:3000".to_string()),
            sandbox_image: env::var("SANDBOX_IMAGE").unwrap_or_else(|_| "sandbox".to_string()),
            memory_limit_mb: env::var("MEMORY_LIMIT_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(256),
            timeout_sec: env::var("TIMEOUT_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            job_ttl_seconds: env::var("JOB_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
        }
    }
}

async fn execute_sandbox(job: &Job, binary: &[u8], config: &Config) -> Result<ExecutionResult, String> {
    // Write binary to temp file
    let temp_file = NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
    let binary_path = temp_file.path().to_path_buf();

    // Write binary data
    let mut file = tokio::fs::File::create(&binary_path)
        .await
        .map_err(|e| format!("Failed to create binary file: {}", e))?;
    file.write_all(binary)
        .await
        .map_err(|e| format!("Failed to write binary: {}", e))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to sync binary: {}", e))?;
    drop(file);

    // Make executable
    let mut perms = tokio::fs::metadata(&binary_path)
        .await
        .map_err(|e| format!("Failed to get metadata: {}", e))?
        .permissions();
    perms.set_mode(0o755);
    tokio::fs::set_permissions(&binary_path, perms)
        .await
        .map_err(|e| format!("Failed to set permissions: {}", e))?;

    let start = Instant::now();

    // Build docker command
    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        "-i",
        &format!("--memory={}m", config.memory_limit_mb),
        &format!("--memory-swap={}m", config.memory_limit_mb),
    ]);

    // Only disable network if not explicitly enabled
    if !job.network_enabled {
        cmd.arg("--network=none");
    }

    cmd.args([
        "--read-only",
        "--tmpfs=/tmp:rw,exec,nosuid,size=64m",
        "--tmpfs=/var:rw,nosuid,size=16m",
        "-e",
        &format!("LIMIT={}", job.instruction_limit),
    ]);

    // Pass environment variables from challenge
    for (key, value) in &job.env_vars {
        cmd.arg("-e");
        cmd.arg(format!("{}={}", key, value));
    }

    cmd.args([
        "-v",
        &format!("{}:/work/binary:ro", binary_path.display()),
        &config.sandbox_image,
    ]);

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn docker: {}", e))?;

    // Write stdin if provided
    if !job.stdin.is_empty() {
        if let Some(mut child_stdin) = child.stdin.take() {
            let _ = child_stdin.write_all(&job.stdin).await;
        }
    } else {
        drop(child.stdin.take());
    }

    // Wait with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(config.timeout_sec),
        child.wait_with_output(),
    )
    .await;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    let output = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(format!("Docker execution failed: {}", e)),
        Err(_) => return Err(format!("Execution timed out after {} seconds", config.timeout_sec)),
    };

    // Parse plugin stats from stderr
    let mut stderr = output.stderr;
    let stats = if let Some(captures) = STATS_REGEX.captures(&stderr) {
        let json_match = captures.get(1).unwrap();
        let stats: PluginStats = serde_json::from_slice(json_match.as_bytes()).unwrap_or(PluginStats {
            instructions: 0,
            memory_peak_kb: 0,
            memory_rss_kb: 0,
            memory_hwm_kb: 0,
            memory_data_kb: 0,
            memory_stack_kb: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            guest_mmap_bytes: 0,
            guest_mmap_peak: 0,
            guest_heap_bytes: 0,
            limit_reached: false,
            syscalls: 0,
            syscall_cost: 0,
            syscall_breakdown: std::collections::HashMap::new(),
        });
        // Remove stats JSON from stderr
        stderr.truncate(json_match.start() - 1);
        stats
    } else {
        PluginStats {
            instructions: 0,
            memory_peak_kb: 0,
            memory_rss_kb: 0,
            memory_hwm_kb: 0,
            memory_data_kb: 0,
            memory_stack_kb: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            guest_mmap_bytes: 0,
            guest_mmap_peak: 0,
            guest_heap_bytes: 0,
            limit_reached: false,
            syscalls: 0,
            syscall_cost: 0,
            syscall_breakdown: std::collections::HashMap::new(),
        }
    };

    Ok(ExecutionResult {
        instructions: stats.instructions,
        memory_peak_kb: stats.memory_peak_kb,
        memory_rss_kb: stats.memory_rss_kb,
        memory_hwm_kb: stats.memory_hwm_kb,
        memory_data_kb: stats.memory_data_kb,
        memory_stack_kb: stats.memory_stack_kb,
        io_read_bytes: stats.io_read_bytes,
        io_write_bytes: stats.io_write_bytes,
        guest_mmap_bytes: stats.guest_mmap_bytes,
        guest_mmap_peak: stats.guest_mmap_peak,
        guest_heap_bytes: stats.guest_heap_bytes,
        limit_reached: stats.limit_reached,
        exit_code: output.status.code().unwrap_or(-1),
        stdout: BASE64.encode(&output.stdout),
        stderr: BASE64.encode(&stderr),
        execution_time_ms,
        syscalls: stats.syscalls,
        syscall_breakdown: stats.syscall_breakdown,
    })
}

async fn update_job_status(
    jobs_kv: &Store,
    job_id: &Uuid,
    status: JobStatus,
    error: Option<String>,
) -> Result<(), String> {
    let key = job_id.to_string();

    // Get current metadata
    let entry = jobs_kv
        .get(&key)
        .await
        .map_err(|e| format!("Failed to get job metadata: {}", e))?
        .ok_or_else(|| format!("Job {} not found", job_id))?;

    let mut metadata: JobMetadata =
        serde_json::from_slice(&entry).map_err(|e| format!("Failed to parse metadata: {}", e))?;

    metadata.status = status;
    match status {
        JobStatus::Running => {
            metadata.started_at = Some(Utc::now());
        }
        JobStatus::Completed | JobStatus::Failed => {
            metadata.completed_at = Some(Utc::now());
            metadata.error = error;
        }
        _ => {}
    }

    jobs_kv
        .put(
            &key,
            serde_json::to_vec(&metadata)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?
                .into(),
        )
        .await
        .map_err(|e| format!("Failed to update job status: {}", e))?;

    Ok(())
}

async fn store_job_result(
    results_kv: &Store,
    job_id: &Uuid,
    result: &ExecutionResult,
) -> Result<(), String> {
    let key = job_id.to_string();

    results_kv
        .put(
            &key,
            serde_json::to_vec(result)
                .map_err(|e| format!("Failed to serialize result: {}", e))?
                .into(),
        )
        .await
        .map_err(|e| format!("Failed to store result: {}", e))?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct SaveRunRequest {
    job_id: Uuid,
    benchmark_id: Option<String>,
    binary_id: String,
    binary_size: Option<i64>,
    language: Option<String>,
    optimization: Option<String>,
    compiler_version: Option<String>,
    instructions: i64,
    memory_peak_kb: Option<i64>,
    memory_rss_kb: Option<i64>,
    memory_hwm_kb: Option<i64>,
    memory_data_kb: Option<i64>,
    memory_stack_kb: Option<i64>,
    io_read_bytes: Option<i64>,
    io_write_bytes: Option<i64>,
    // Guest memory (actual binary allocations)
    guest_mmap_bytes: Option<i64>,
    guest_mmap_peak: Option<i64>,
    guest_heap_bytes: Option<i64>,
    limit_reached: bool,
    exit_code: Option<i32>,
    execution_time_ms: Option<i64>,
    instruction_limit: Option<i64>,
    syscalls: Option<i64>,
    syscall_breakdown: Option<serde_json::Value>,
    stdout: Option<String>,
    stderr: Option<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct BinaryMetadata {
    language: Option<String>,
    optimization: Option<String>,
    compiler_version: Option<String>,
}

async fn persist_run(
    http_client: &reqwest::Client,
    api_url: &str,
    job: &Job,
    binary_size: usize,
    metadata: Option<&BinaryMetadata>,
    result: &ExecutionResult,
) -> Result<(), String> {
    let req = SaveRunRequest {
        job_id: job.id,
        benchmark_id: job.benchmark_id.clone(),
        binary_id: job.binary_id.clone(),
        binary_size: Some(binary_size as i64),
        language: metadata.and_then(|m| m.language.clone()),
        optimization: metadata.and_then(|m| m.optimization.clone()),
        compiler_version: metadata.and_then(|m| m.compiler_version.clone()),
        instructions: result.instructions as i64,
        memory_peak_kb: Some(result.memory_peak_kb as i64),
        memory_rss_kb: Some(result.memory_rss_kb as i64),
        memory_hwm_kb: Some(result.memory_hwm_kb as i64),
        memory_data_kb: Some(result.memory_data_kb as i64),
        memory_stack_kb: Some(result.memory_stack_kb as i64),
        io_read_bytes: Some(result.io_read_bytes as i64),
        io_write_bytes: Some(result.io_write_bytes as i64),
        guest_mmap_bytes: Some(result.guest_mmap_bytes as i64),
        guest_mmap_peak: Some(result.guest_mmap_peak as i64),
        guest_heap_bytes: Some(result.guest_heap_bytes as i64),
        limit_reached: result.limit_reached,
        exit_code: Some(result.exit_code),
        execution_time_ms: Some(result.execution_time_ms as i64),
        instruction_limit: Some(job.instruction_limit as i64),
        syscalls: Some(result.syscalls as i64),
        syscall_breakdown: Some(serde_json::to_value(&result.syscall_breakdown).unwrap_or_default()),
        stdout: Some(result.stdout.clone()),
        stderr: Some(result.stderr.clone()),
        started_at: None, // Could track this if needed
        completed_at: Some(Utc::now()),
    };

    let response = http_client
        .post(&format!("{}/runs", api_url))
        .json(&req)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to send persist request: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to persist run: HTTP {} - {}", status, body));
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ctf_worker=info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();

    info!("Starting CTF Worker (NATS: {}, sandbox: {})", config.nats_url, config.sandbox_image);

    // Connect to NATS with longer request timeout for large binary operations
    let client = loop {
        let nats_options = async_nats::ConnectOptions::new()
            .request_timeout(Some(Duration::from_secs(120)));

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
            name: JOBS_STREAM.to_string(),
            subjects: vec!["jobs.submit".to_string()],
            retention: jetstream::stream::RetentionPolicy::WorkQueue,
            max_age: Duration::from_secs(config.job_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create/get JOBS stream");

    // Get or create KV buckets
    let jobs_kv = jetstream
        .create_key_value(jetstream::kv::Config {
            bucket: JOBS_KV.to_string(),
            max_age: Duration::from_secs(config.job_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create jobs KV");

    let results_kv = jetstream
        .create_key_value(jetstream::kv::Config {
            bucket: RESULTS_KV.to_string(),
            max_age: Duration::from_secs(config.job_ttl_seconds),
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .expect("Failed to create results KV");

    // HTTP client for fetching binaries from API
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");

    // Create durable consumer
    let consumer: PullConsumer = stream
        .get_or_create_consumer(
            "worker",
            jetstream::consumer::pull::Config {
                durable_name: Some("worker".to_string()),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                max_deliver: 3,
                ack_wait: Duration::from_secs(config.timeout_sec + 30),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create consumer");

    info!("Worker ready, waiting for jobs...");

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

            let job: Job = match serde_json::from_slice(&msg.payload) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to parse job: {}", e);
                    let _ = msg.ack().await;
                    continue;
                }
            };

            info!(job_id = %job.id, instruction_limit = job.instruction_limit, binary_id = %job.binary_id, "Processing job");

            // Fetch binary from API
            let binary = match http_client
                .get(&format!("{}/binaries/{}", config.api_url, job.binary_id))
                .timeout(Duration::from_secs(60))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    match resp.bytes().await {
                        Ok(b) => b.to_vec(),
                        Err(e) => {
                            error!("Failed to read binary response: {}", e);
                            let _ = update_job_status(&jobs_kv, &job.id, JobStatus::Failed, Some(format!("Failed to fetch binary: {}", e))).await;
                            let _ = msg.ack().await;
                            continue;
                        }
                    }
                }
                Ok(resp) => {
                    error!("Binary not found: HTTP {}", resp.status());
                    let _ = update_job_status(&jobs_kv, &job.id, JobStatus::Failed, Some(format!("Binary not found: {}", job.binary_id))).await;
                    let _ = msg.ack().await;
                    continue;
                }
                Err(e) => {
                    error!("Failed to fetch binary: {}", e);
                    let _ = update_job_status(&jobs_kv, &job.id, JobStatus::Failed, Some(format!("Failed to fetch binary: {}", e))).await;
                    let _ = msg.ack().await;
                    continue;
                }
            };

            info!(job_id = %job.id, binary_size = binary.len(), "Binary fetched");

            // Fetch binary metadata
            let metadata: Option<BinaryMetadata> = match http_client
                .get(&format!("{}/binaries/{}/metadata", config.api_url, job.binary_id))
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    resp.json().await.ok()
                }
                _ => None
            };

            if let Some(ref m) = metadata {
                info!(job_id = %job.id, language = ?m.language, optimization = ?m.optimization, "Binary metadata fetched");
            }

            // Update status to running
            if let Err(e) = update_job_status(&jobs_kv, &job.id, JobStatus::Running, None).await {
                error!("Failed to update job status: {}", e);
            }

            // Execute the sandbox
            match execute_sandbox(&job, &binary, &config).await {
                Ok(result) => {
                    info!(
                        job_id = %job.id,
                        instructions = result.instructions,
                        exit_code = result.exit_code,
                        time_ms = result.execution_time_ms,
                        "Job completed"
                    );

                    // Store result in NATS KV (for fast access)
                    if let Err(e) = store_job_result(&results_kv, &job.id, &result).await {
                        error!("Failed to store result: {}", e);
                    }

                    // Persist run to PostgreSQL (permanent storage)
                    if let Err(e) = persist_run(&http_client, &config.api_url, &job, binary.len(), metadata.as_ref(), &result).await {
                        error!("Failed to persist run to database: {}", e);
                        // Don't fail the job - NATS KV still has the result
                    }

                    // Update status to completed
                    if let Err(e) = update_job_status(&jobs_kv, &job.id, JobStatus::Completed, None).await {
                        error!("Failed to update job status: {}", e);
                    }
                }
                Err(e) => {
                    error!(job_id = %job.id, error = %e, "Job failed");

                    // Update status to failed
                    if let Err(e2) = update_job_status(&jobs_kv, &job.id, JobStatus::Failed, Some(e)).await {
                        error!("Failed to update job status: {}", e2);
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
