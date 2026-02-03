use crate::config::Config;
use crate::error::ApiError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::sync::LazyLock;
use std::time::Instant;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

static STATS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n(\{[^\n]+\})\n?$").unwrap());

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
pub struct ExecutionResult {
    pub instructions: u64,
    pub memory_peak_kb: u64,
    #[serde(default)]
    pub memory_rss_kb: u64,
    #[serde(default)]
    pub memory_hwm_kb: u64,
    #[serde(default)]
    pub memory_data_kb: u64,
    #[serde(default)]
    pub memory_stack_kb: u64,
    #[serde(default)]
    pub io_read_bytes: u64,
    #[serde(default)]
    pub io_write_bytes: u64,
    // Guest memory (actual binary allocations)
    #[serde(default)]
    pub guest_mmap_bytes: u64,
    #[serde(default)]
    pub guest_mmap_peak: u64,
    #[serde(default)]
    pub guest_heap_bytes: u64,
    pub limit_reached: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    #[serde(default)]
    pub syscalls: u64,
    #[serde(default)]
    pub syscall_breakdown: std::collections::HashMap<String, u64>,
}

pub async fn execute(
    binary: Vec<u8>,
    instruction_limit: u64,
    stdin: Vec<u8>,
    config: &Config,
) -> Result<ExecutionResult, ApiError> {
    // Write binary to temp file
    let temp_file = NamedTempFile::new().map_err(|e| ApiError::Internal(e.to_string()))?;
    let binary_path = temp_file.path().to_path_buf();

    // Write binary data
    let mut file = tokio::fs::File::create(&binary_path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    file.write_all(&binary)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    file.sync_all()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    drop(file);

    // Make executable
    let mut perms = tokio::fs::metadata(&binary_path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .permissions();
    perms.set_mode(0o755);
    tokio::fs::set_permissions(&binary_path, perms)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let start = Instant::now();

    // Build docker command
    let mut cmd = Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        "-i",
        &format!("--memory={}m", config.memory_limit_mb),
        &format!("--memory-swap={}m", config.memory_limit_mb),
        "--network=none",
        "--read-only",
        "--tmpfs=/tmp:rw,exec,nosuid,size=64m",
        "--tmpfs=/var:rw,nosuid,size=16m",
        "-e",
        &format!("LIMIT={}", instruction_limit),
        "-v",
        &format!("{}:/work/binary:ro", binary_path.display()),
        &config.sandbox_image,
    ]);

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| ApiError::DockerError(e.to_string()))?;

    // Write stdin if provided
    if !stdin.is_empty() {
        if let Some(mut child_stdin) = child.stdin.take() {
            let _ = child_stdin.write_all(&stdin).await;
        }
    } else {
        // Close stdin
        drop(child.stdin.take());
    }

    // Wait with timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(config.timeout_sec),
        child.wait_with_output(),
    )
    .await;

    let execution_time_ms = start.elapsed().as_millis() as u64;

    let output = match result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(ApiError::DockerError(e.to_string())),
        Err(_) => return Err(ApiError::Timeout(config.timeout_sec)),
    };

    // Parse plugin stats from stderr
    let mut stderr = output.stderr;
    let stats = if let Some(captures) = STATS_REGEX.captures(&stderr) {
        let json_match = captures.get(1).unwrap();
        let stats: PluginStats = serde_json::from_slice(json_match.as_bytes())
            .unwrap_or(PluginStats {
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
        stderr.truncate(json_match.start() - 1); // -1 for the leading \n
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

pub async fn check_docker() -> bool {
    Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
