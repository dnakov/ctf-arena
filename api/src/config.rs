use std::env;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub default_instruction_limit: u64,
    pub max_instruction_limit: u64,
    pub memory_limit_mb: u32,
    pub timeout_sec: u64,
    pub max_binary_size: usize,
    pub max_concurrent: usize,
    pub sandbox_image: String,
    pub nats_url: String,
    pub database_url: String,
    pub job_ttl_seconds: u64,
    pub rate_limit_per_minute: u32,
    pub compile_timeout_sec: u64,
    pub max_source_size: usize,
    pub binary_ttl_seconds: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
            default_instruction_limit: env::var("DEFAULT_INSTRUCTION_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000_000),
            max_instruction_limit: env::var("MAX_INSTRUCTION_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1_000_000_000_000),
            memory_limit_mb: env::var("MEMORY_LIMIT_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(256),
            timeout_sec: env::var("TIMEOUT_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_binary_size: env::var("MAX_BINARY_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100 * 1024 * 1024), // 100MB
            max_concurrent: env::var("MAX_CONCURRENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            sandbox_image: env::var("SANDBOX_IMAGE").unwrap_or_else(|_| "sandbox".to_string()),
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://ctf:ctf@localhost:5432/ctf".to_string()),
            job_ttl_seconds: env::var("JOB_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            rate_limit_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            compile_timeout_sec: env::var("COMPILE_TIMEOUT_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(120),
            max_source_size: env::var("MAX_SOURCE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1024 * 1024), // 1MB
            binary_ttl_seconds: env::var("BINARY_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400), // 24 hours
        }
    }
}
