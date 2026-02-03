use crate::error::ApiError;
use crate::sandbox::ExecutionResult;
use async_nats::jetstream::{self, kv::Store, stream::Stream};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

const JOBS_STREAM: &str = "JOBS";
const JOBS_KV: &str = "jobs";
const RESULTS_KV: &str = "results";
const COMPILES_STREAM: &str = "COMPILES";
const COMPILES_KV: &str = "compiles";
const BINARIES_KV: &str = "binaries";
const COMPILE_CACHE_KV: &str = "compile_cache";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub binary_id: String,        // Reference to binary in PostgreSQL
    pub instruction_limit: u64,
    pub stdin: Vec<u8>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub benchmark_id: Option<String>,
    // Challenge-specific execution options
    #[serde(default)]
    pub network_enabled: bool,
    #[serde(default)]
    pub env_vars: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

// ============ Compile Types ============

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
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "c" => Some(Language::C),
            "cpp" | "c++" => Some(Language::Cpp),
            "rust" => Some(Language::Rust),
            "go" | "golang" => Some(Language::Go),
            "zig" => Some(Language::Zig),
            "asm" | "assembly" => Some(Language::Asm),
            "nim" => Some(Language::Nim),
            "pascal" => Some(Language::Pascal),
            "ocaml" => Some(Language::Ocaml),
            "swift" => Some(Language::Swift),
            "haskell" => Some(Language::Haskell),
            "csharp" | "c#" => Some(Language::Csharp),
            "java" => Some(Language::Java),
            "kotlin" => Some(Language::Kotlin),
            "scala" => Some(Language::Scala),
            "clojure" => Some(Language::Clojure),
            "python" => Some(Language::Python),
            "javascript" | "js" => Some(Language::Javascript),
            "typescript" | "ts" => Some(Language::Typescript),
            "bun" => Some(Language::Bun),
            "deno" => Some(Language::Deno),
            "node" | "nodejs" => Some(Language::Node),
            "lua" => Some(Language::Lua),
            "perl" => Some(Language::Perl),
            "php" => Some(Language::Php),
            "tcl" => Some(Language::Tcl),
            "erlang" => Some(Language::Erlang),
            "elixir" => Some(Language::Elixir),
            "racket" => Some(Language::Racket),
            "wasm" | "wat" => Some(Language::Wasm),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Optimization::Debug),
            "release" => Some(Optimization::Release),
            "size" => Some(Optimization::Size),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Optimization::Debug => "debug",
            Optimization::Release => "release",
            Optimization::Size => "size",
        }
    }
}

use std::collections::HashMap;

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

pub struct QueueClient {
    jetstream: jetstream::Context,
    jobs_stream: Arc<RwLock<Stream>>,
    jobs_kv: Store,
    results_kv: Store,
    compiles_stream: Arc<RwLock<Stream>>,
    compiles_kv: Store,
    binaries_kv: Store,
    compile_cache_kv: Store,
}

impl QueueClient {
    pub async fn connect(nats_url: &str, job_ttl_seconds: u64, binary_ttl_seconds: u64) -> Result<Self, ApiError> {
        // Use longer request timeout for large binary operations
        let nats_options = async_nats::ConnectOptions::new()
            .request_timeout(Some(std::time::Duration::from_secs(120)));

        let client = nats_options.connect(nats_url)
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to connect to NATS: {}", e)))?;

        let jetstream = jetstream::new(client);

        // Create or get the JOBS stream (work queue pattern)
        let jobs_stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: JOBS_STREAM.to_string(),
                subjects: vec!["jobs.submit".to_string()],
                retention: jetstream::stream::RetentionPolicy::WorkQueue,
                max_age: Duration::from_secs(job_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create JOBS stream: {}", e)))?;

        // Create or get the jobs KV bucket for status tracking
        let jobs_kv = jetstream
            .create_key_value(jetstream::kv::Config {
                bucket: JOBS_KV.to_string(),
                max_age: Duration::from_secs(job_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create jobs KV: {}", e)))?;

        // Create or get the results KV bucket
        let results_kv = jetstream
            .create_key_value(jetstream::kv::Config {
                bucket: RESULTS_KV.to_string(),
                max_age: Duration::from_secs(job_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create results KV: {}", e)))?;

        // Create or get the COMPILES stream (work queue pattern)
        let compiles_stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: COMPILES_STREAM.to_string(),
                subjects: vec!["compiles.submit".to_string()],
                retention: jetstream::stream::RetentionPolicy::WorkQueue,
                max_age: Duration::from_secs(job_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create COMPILES stream: {}", e)))?;

        // Create or get the compiles KV bucket for status tracking
        let compiles_kv = jetstream
            .create_key_value(jetstream::kv::Config {
                bucket: COMPILES_KV.to_string(),
                max_age: Duration::from_secs(job_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create compiles KV: {}", e)))?;

        // Create or get the binaries KV bucket for compiled binary storage
        // max_value_size allows storing large binaries (up to 100MB)
        // Use memory storage for faster writes (binaries are ephemeral anyway)
        let binaries_kv = jetstream
            .create_key_value(jetstream::kv::Config {
                bucket: BINARIES_KV.to_string(),
                max_age: Duration::from_secs(binary_ttl_seconds),
                max_value_size: 100 * 1024 * 1024, // 100MB max binary size
                storage: jetstream::stream::StorageType::Memory,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create binaries KV: {}", e)))?;

        // Create or get the compile_cache KV bucket for source hash -> binary_id mapping
        let compile_cache_kv = jetstream
            .create_key_value(jetstream::kv::Config {
                bucket: COMPILE_CACHE_KV.to_string(),
                max_age: Duration::from_secs(binary_ttl_seconds),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to create compile_cache KV: {}", e)))?;

        Ok(Self {
            jetstream,
            jobs_stream: Arc::new(RwLock::new(jobs_stream)),
            jobs_kv,
            results_kv,
            compiles_stream: Arc::new(RwLock::new(compiles_stream)),
            compiles_kv,
            binaries_kv,
            compile_cache_kv,
        })
    }

    pub async fn submit_job(&self, job: Job) -> Result<(), ApiError> {
        let job_id = job.id.to_string();

        // Store initial job metadata
        let metadata = JobMetadata {
            status: JobStatus::Pending,
            created_at: job.created_at,
            started_at: None,
            completed_at: None,
            error: None,
        };

        self.jobs_kv
            .put(
                &job_id,
                serde_json::to_vec(&metadata)
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .into(),
            )
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to store job metadata: {}", e)))?;

        // Publish job to the work queue
        let payload = serde_json::to_vec(&job).map_err(|e| ApiError::Internal(e.to_string()))?;

        self.jetstream
            .publish("jobs.submit", payload.into())
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to publish job: {}", e)))?
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to confirm job publish: {}", e)))?;

        Ok(())
    }

    pub async fn get_job_status(&self, job_id: &Uuid) -> Result<Option<JobMetadata>, ApiError> {
        let key = job_id.to_string();

        match self.jobs_kv.get(&key).await {
            Ok(Some(entry)) => {
                let metadata: JobMetadata = serde_json::from_slice(&entry)
                    .map_err(|e| ApiError::Internal(format!("Failed to parse job metadata: {}", e)))?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to get job status: {}",
                e
            ))),
        }
    }

    pub async fn get_job_result(&self, job_id: &Uuid) -> Result<Option<ExecutionResult>, ApiError> {
        let key = job_id.to_string();

        match self.results_kv.get(&key).await {
            Ok(Some(entry)) => {
                let result: ExecutionResult = serde_json::from_slice(&entry)
                    .map_err(|e| ApiError::Internal(format!("Failed to parse job result: {}", e)))?;
                Ok(Some(result))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to get job result: {}",
                e
            ))),
        }
    }

    pub async fn get_queue_depth(&self) -> Result<u64, ApiError> {
        let mut stream = self.jobs_stream.write().await;
        let info = stream
            .info()
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to get stream info: {}", e)))?;

        Ok(info.state.messages)
    }

    pub async fn update_job_status(
        &self,
        job_id: &Uuid,
        status: JobStatus,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let key = job_id.to_string();

        let mut metadata = self
            .get_job_status(job_id)
            .await?
            .ok_or_else(|| ApiError::JobNotFound(job_id.to_string()))?;

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

        self.jobs_kv
            .put(
                &key,
                serde_json::to_vec(&metadata)
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .into(),
            )
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to update job status: {}", e)))?;

        Ok(())
    }

    pub async fn store_job_result(
        &self,
        job_id: &Uuid,
        result: &ExecutionResult,
    ) -> Result<(), ApiError> {
        let key = job_id.to_string();

        self.results_kv
            .put(
                &key,
                serde_json::to_vec(result)
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .into(),
            )
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to store job result: {}", e)))?;

        Ok(())
    }

    // ============ Compile Methods ============

    pub async fn submit_compile_job(&self, job: CompileJob) -> Result<(), ApiError> {
        let job_id = job.id.to_string();

        // Store initial compile metadata
        let metadata = CompileMetadata {
            status: CompileStatus::Pending,
            created_at: job.created_at,
            started_at: None,
            completed_at: None,
            error: None,
            position: None,
        };

        self.compiles_kv
            .put(
                &job_id,
                serde_json::to_vec(&metadata)
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .into(),
            )
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to store compile metadata: {}", e)))?;

        // Publish job to the work queue
        let payload = serde_json::to_vec(&job).map_err(|e| ApiError::Internal(e.to_string()))?;

        self.jetstream
            .publish("compiles.submit", payload.into())
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to publish compile job: {}", e)))?
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to confirm compile job publish: {}", e)))?;

        Ok(())
    }

    pub async fn get_compile_status(&self, job_id: &Uuid) -> Result<Option<CompileMetadata>, ApiError> {
        let key = job_id.to_string();

        match self.compiles_kv.get(&key).await {
            Ok(Some(entry)) => {
                let metadata: CompileMetadata = serde_json::from_slice(&entry)
                    .map_err(|e| ApiError::Internal(format!("Failed to parse compile metadata: {}", e)))?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to get compile status: {}",
                e
            ))),
        }
    }

    pub async fn get_compile_result(&self, job_id: &Uuid) -> Result<Option<CompileResult>, ApiError> {
        let result_key = format!("{}_result", job_id);

        match self.compiles_kv.get(&result_key).await {
            Ok(Some(entry)) => {
                let result: CompileResult = serde_json::from_slice(&entry)
                    .map_err(|e| ApiError::Internal(format!("Failed to parse compile result: {}", e)))?;
                Ok(Some(result))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to get compile result: {}",
                e
            ))),
        }
    }

    pub async fn get_compile_queue_depth(&self) -> Result<u64, ApiError> {
        let mut stream = self.compiles_stream.write().await;
        let info = stream
            .info()
            .await
            .map_err(|e| ApiError::QueueError(format!("Failed to get compile stream info: {}", e)))?;

        Ok(info.state.messages)
    }

    pub async fn get_binary(&self, binary_id: &str) -> Result<Option<Vec<u8>>, ApiError> {
        match self.binaries_kv.get(binary_id).await {
            Ok(Some(entry)) => Ok(Some(entry.to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to get binary: {}",
                e
            ))),
        }
    }

    pub async fn check_compile_cache(
        &self,
        source: &str,
        language: Language,
        optimization: Optimization,
        flags: &HashMap<String, String>,
    ) -> Result<Option<CompileResult>, ApiError> {
        let cache_key = compute_cache_key(source, language, optimization, flags);

        match self.compile_cache_kv.get(&cache_key).await {
            Ok(Some(entry)) => {
                let result: CompileResult = serde_json::from_slice(&entry)
                    .map_err(|e| ApiError::Internal(format!("Failed to parse cache entry: {}", e)))?;

                // Verify the binary still exists
                match self.binaries_kv.get(&result.binary_id).await {
                    Ok(Some(_)) => Ok(Some(result)),
                    _ => Ok(None), // Binary expired, cache miss
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::QueueError(format!(
                "Failed to check compile cache: {}",
                e
            ))),
        }
    }
}
