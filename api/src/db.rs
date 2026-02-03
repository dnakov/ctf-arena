use crate::error::ApiError;
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

// ============ User Types ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum UserType {
    Human,
    Clanker,
}

impl Default for UserType {
    fn default() -> Self {
        UserType::Human
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub github_id: Option<i64>,
    pub github_login: Option<String>,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub twitter_handle: Option<String>,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_method: Option<String>,
    pub user_type: String, // 'human' or 'clanker'
    pub clanker_twitter: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub twitter_handle: Option<String>,
    pub is_verified: bool,
    pub user_type: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for PublicUser {
    fn from(u: User) -> Self {
        PublicUser {
            id: u.id,
            username: u.username,
            avatar_url: u.avatar_url,
            display_name: u.display_name,
            bio: u.bio,
            twitter_handle: u.twitter_handle,
            is_verified: u.is_verified,
            user_type: u.user_type,
            created_at: u.created_at,
        }
    }
}

// ============ Session Types ============

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ============ Challenge Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub stdin: String,
    pub expected_stdout: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VerifyMode {
    Exact,
    Trimmed,
    Sorted,
}

impl Default for VerifyMode {
    fn default() -> Self {
        VerifyMode::Exact
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Challenge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub difficulty: String,
    pub input_spec: Option<String>,
    pub output_spec: String,
    pub test_cases: serde_json::Value, // Vec<TestCase> as JSON
    pub verify_mode: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    // Challenge execution options
    pub network_enabled: bool,
    pub env_vars: Option<serde_json::Value>, // HashMap<String, String> as JSON
    // Baseline solutions per language
    pub baselines: Option<serde_json::Value>, // Vec<ChallengeBaseline> as JSON
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeBaseline {
    pub language: String,
    pub name: String,
    pub tier: String,
    pub source_code: String,
    pub reference_instructions: Option<i64>,
}

// ============ Leaderboard Types ============

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub challenge_id: String,
    pub language: String,
    pub instructions: i64,
    pub run_id: Uuid,
    pub source_code: String,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntryWithUser {
    pub rank: i64,
    pub user: PublicUser,
    pub instructions: i64,
    pub language: String,
    pub submitted_at: DateTime<Utc>,
}

// ============ Challenge Submission Types ============

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChallengeSubmission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub challenge_id: String,
    pub language: String,
    pub source_code: String,
    pub binary_id: Option<String>,
    pub status: String, // 'pending', 'compiling', 'running', 'passed', 'failed'
    pub test_results: Option<serde_json::Value>,
    pub instructions: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============ Verification Types ============

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VerificationCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    pub twitter_handle: String,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_pool(database_url: &str) -> Result<PgPool, ApiError> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to connect to database: {}", e)))
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), ApiError> {
    // Create tables if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(50) UNIQUE NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create users table: {}", e)))?;

    // Add new columns to users table for OAuth and verification
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS github_id BIGINT UNIQUE"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS github_login VARCHAR(100)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_url VARCHAR(500)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name VARCHAR(100)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS bio TEXT"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS twitter_handle VARCHAR(100)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS is_verified BOOLEAN DEFAULT FALSE"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS verified_at TIMESTAMPTZ"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS verification_method VARCHAR(50)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS user_type VARCHAR(20) DEFAULT 'human'"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE users ADD COLUMN IF NOT EXISTS clanker_twitter VARCHAR(100)"#)
        .execute(pool).await.ok();

    // Create sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token_hash VARCHAR(64) NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create sessions table: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at)"#)
        .execute(pool).await.ok();

    // Create challenges table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS challenges (
            id VARCHAR(100) PRIMARY KEY,
            name VARCHAR(200) NOT NULL,
            description TEXT NOT NULL,
            category VARCHAR(50) NOT NULL,
            difficulty VARCHAR(20) DEFAULT 'medium',
            input_spec TEXT,
            output_spec TEXT NOT NULL,
            test_cases JSONB NOT NULL,
            verify_mode VARCHAR(20) DEFAULT 'exact',
            is_active BOOLEAN DEFAULT TRUE,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create challenges table: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_challenges_category ON challenges(category)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_challenges_is_active ON challenges(is_active)"#)
        .execute(pool).await.ok();

    // Add network_enabled, env_vars, and baselines columns (migration)
    sqlx::query(r#"ALTER TABLE challenges ADD COLUMN IF NOT EXISTS network_enabled BOOLEAN DEFAULT FALSE"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE challenges ADD COLUMN IF NOT EXISTS env_vars JSONB"#)
        .execute(pool).await.ok();
    sqlx::query(r#"ALTER TABLE challenges ADD COLUMN IF NOT EXISTS baselines JSONB"#)
        .execute(pool).await.ok();

    // Create leaderboard_entries table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS leaderboard_entries (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            challenge_id VARCHAR(100) NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
            language VARCHAR(50) NOT NULL,
            instructions BIGINT NOT NULL,
            run_id UUID NOT NULL REFERENCES runs(id),
            source_code TEXT NOT NULL,
            is_verified BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE (user_id, challenge_id, language)
        )
        "#,
    )
    .execute(pool)
    .await
    .ok(); // May fail if runs table doesn't exist yet, we'll try again after

    sqlx::query(
        r#"CREATE INDEX IF NOT EXISTS idx_leaderboard_ranking ON leaderboard_entries(challenge_id, language, instructions)"#,
    )
    .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_leaderboard_user ON leaderboard_entries(user_id)"#)
        .execute(pool).await.ok();

    // Create challenge_submissions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS challenge_submissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            challenge_id VARCHAR(100) NOT NULL REFERENCES challenges(id),
            language VARCHAR(50) NOT NULL,
            source_code TEXT NOT NULL,
            binary_id VARCHAR(100),
            status VARCHAR(20) DEFAULT 'pending',
            test_results JSONB,
            instructions BIGINT,
            error_message TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create challenge_submissions table: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_challenge_submissions_user ON challenge_submissions(user_id)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_challenge_submissions_challenge ON challenge_submissions(challenge_id)"#)
        .execute(pool).await.ok();
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_challenge_submissions_status ON challenge_submissions(status)"#)
        .execute(pool).await.ok();

    // Create verification_codes table (for clanker Twitter verification)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS verification_codes (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            code VARCHAR(50) NOT NULL UNIQUE,
            twitter_handle VARCHAR(100) NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL,
            verified BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create verification_codes table: {}", e)))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS submissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id),
            job_id UUID NOT NULL,
            challenge_id UUID,
            instructions BIGINT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create submissions table: {}", e)))?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rate_limits (
            user_id UUID REFERENCES users(id),
            window_start TIMESTAMPTZ NOT NULL,
            count INTEGER DEFAULT 1,
            PRIMARY KEY (user_id, window_start)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create rate_limits table: {}", e)))?;

    // Create index on submissions for efficient lookups
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_submissions_user_id ON submissions(user_id)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create index: {}", e)))?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_submissions_job_id ON submissions(job_id)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create index: {}", e)))?;

    // Create binaries table for compiled binary storage
    create_binaries_table(pool).await?;

    // Create runs table for permanent run storage
    create_runs_table(pool).await?;

    Ok(())
}

pub async fn check_rate_limit(
    pool: &PgPool,
    user_id: &Uuid,
    limit_per_minute: u32,
) -> Result<(), ApiError> {
    let window_start = Utc::now()
        .duration_trunc(TimeDelta::minutes(1))
        .expect("Failed to truncate time");

    // Upsert the rate limit counter and check in one query
    let result: Option<(i32,)> = sqlx::query_as(
        r#"
        INSERT INTO rate_limits (user_id, window_start, count)
        VALUES ($1, $2, 1)
        ON CONFLICT (user_id, window_start)
        DO UPDATE SET count = rate_limits.count + 1
        RETURNING count
        "#,
    )
    .bind(user_id)
    .bind(window_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to check rate limit: {}", e)))?;

    if let Some((count,)) = result {
        if count > limit_per_minute as i32 {
            return Err(ApiError::RateLimited);
        }
    }

    Ok(())
}

pub async fn record_submission(
    pool: &PgPool,
    user_id: Option<&Uuid>,
    job_id: &Uuid,
    challenge_id: Option<&Uuid>,
) -> Result<Uuid, ApiError> {
    let result: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO submissions (user_id, job_id, challenge_id)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(job_id)
    .bind(challenge_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to record submission: {}", e)))?;

    Ok(result.0)
}

pub async fn update_submission_instructions(
    pool: &PgPool,
    job_id: &Uuid,
    instructions: i64,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE submissions
        SET instructions = $1
        WHERE job_id = $2
        "#,
    )
    .bind(instructions)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to update submission: {}", e)))?;

    Ok(())
}

#[derive(Debug)]
pub struct Submission {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub job_id: Uuid,
    pub challenge_id: Option<Uuid>,
    pub instructions: Option<i64>,
    pub created_at: DateTime<Utc>,
}

pub async fn get_submission_by_job_id(
    pool: &PgPool,
    job_id: &Uuid,
) -> Result<Option<Submission>, ApiError> {
    let result: Option<(Uuid, Option<Uuid>, Uuid, Option<Uuid>, Option<i64>, DateTime<Utc>)> =
        sqlx::query_as(
            r#"
            SELECT id, user_id, job_id, challenge_id, instructions, created_at
            FROM submissions
            WHERE job_id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to get submission: {}", e)))?;

    Ok(result.map(|(id, user_id, job_id, challenge_id, instructions, created_at)| Submission {
        id,
        user_id,
        job_id,
        challenge_id,
        instructions,
        created_at,
    }))
}

pub async fn get_or_create_anonymous_user(pool: &PgPool) -> Result<Uuid, ApiError> {
    // Try to get or create an anonymous user for unauthenticated submissions
    let result: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO users (username)
        VALUES ('anonymous')
        ON CONFLICT (username) DO UPDATE SET username = EXCLUDED.username
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get anonymous user: {}", e)))?;

    Ok(result.0)
}

pub async fn cleanup_old_rate_limits(pool: &PgPool) -> Result<u64, ApiError> {
    let cutoff = Utc::now() - TimeDelta::minutes(5);

    let result = sqlx::query(
        r#"
        DELETE FROM rate_limits
        WHERE window_start < $1
        "#,
    )
    .bind(cutoff)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to cleanup rate limits: {}", e)))?;

    Ok(result.rows_affected())
}

// Binary storage functions

pub async fn create_binaries_table(pool: &PgPool) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS binaries (
            id VARCHAR(100) PRIMARY KEY,
            data BYTEA NOT NULL,
            size BIGINT NOT NULL,
            language VARCHAR(50),
            optimization VARCHAR(20),
            compiler_version VARCHAR(200),
            compile_flags JSONB,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create binaries table: {}", e)))?;

    // Add metadata columns if they don't exist (migration for existing tables)
    sqlx::query(r#"ALTER TABLE binaries ADD COLUMN IF NOT EXISTS language VARCHAR(50)"#)
        .execute(pool)
        .await
        .ok(); // Ignore errors (column may already exist)

    sqlx::query(r#"ALTER TABLE binaries ADD COLUMN IF NOT EXISTS optimization VARCHAR(20)"#)
        .execute(pool)
        .await
        .ok();

    sqlx::query(r#"ALTER TABLE binaries ADD COLUMN IF NOT EXISTS compiler_version VARCHAR(200)"#)
        .execute(pool)
        .await
        .ok();

    sqlx::query(r#"ALTER TABLE binaries ADD COLUMN IF NOT EXISTS compile_flags JSONB"#)
        .execute(pool)
        .await
        .ok();

    // Create index for cleanup
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_binaries_created_at ON binaries(created_at)
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create binaries index: {}", e)))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BinaryMetadata {
    pub language: Option<String>,
    pub optimization: Option<String>,
    pub compiler_version: Option<String>,
    pub compile_flags: Option<serde_json::Value>,
}

pub async fn store_binary(
    pool: &PgPool,
    id: &str,
    data: &[u8],
    metadata: Option<&BinaryMetadata>,
) -> Result<(), ApiError> {
    let size = data.len() as i64;
    let (language, optimization, compiler_version, compile_flags) = metadata
        .map(|m| {
            (
                m.language.as_deref(),
                m.optimization.as_deref(),
                m.compiler_version.as_deref(),
                m.compile_flags.as_ref(),
            )
        })
        .unwrap_or((None, None, None, None));

    sqlx::query(
        r#"
        INSERT INTO binaries (id, data, size, language, optimization, compiler_version, compile_flags)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (id) DO UPDATE SET
            language = COALESCE(EXCLUDED.language, binaries.language),
            optimization = COALESCE(EXCLUDED.optimization, binaries.optimization),
            compiler_version = COALESCE(EXCLUDED.compiler_version, binaries.compiler_version),
            compile_flags = COALESCE(EXCLUDED.compile_flags, binaries.compile_flags)
        "#,
    )
    .bind(id)
    .bind(data)
    .bind(size)
    .bind(language)
    .bind(optimization)
    .bind(compiler_version)
    .bind(compile_flags)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to store binary: {}", e)))?;

    Ok(())
}

pub async fn get_binary(pool: &PgPool, id: &str) -> Result<Option<Vec<u8>>, ApiError> {
    let result: Option<(Vec<u8>,)> = sqlx::query_as(
        r#"
        SELECT data FROM binaries WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get binary: {}", e)))?;

    Ok(result.map(|(data,)| data))
}

pub async fn get_binary_metadata(
    pool: &PgPool,
    id: &str,
) -> Result<Option<BinaryMetadata>, ApiError> {
    let result: Option<(Option<String>, Option<String>, Option<String>, Option<serde_json::Value>)> = sqlx::query_as(
        r#"
        SELECT language, optimization, compiler_version, compile_flags FROM binaries WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get binary metadata: {}", e)))?;

    Ok(result.map(|(language, optimization, compiler_version, compile_flags)| BinaryMetadata {
        language,
        optimization,
        compiler_version,
        compile_flags,
    }))
}

pub async fn cleanup_old_binaries(pool: &PgPool, max_age_hours: i64) -> Result<u64, ApiError> {
    let cutoff = Utc::now() - TimeDelta::hours(max_age_hours);

    let result = sqlx::query(
        r#"
        DELETE FROM binaries
        WHERE created_at < $1
        "#,
    )
    .bind(cutoff)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to cleanup binaries: {}", e)))?;

    Ok(result.rows_affected())
}

// Runs table functions for permanent run storage

pub async fn create_runs_table(pool: &PgPool) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS runs (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            job_id UUID NOT NULL UNIQUE,

            -- Binary and source info
            binary_id VARCHAR(100) NOT NULL,
            binary_size BIGINT,
            source_code TEXT,
            language VARCHAR(50),
            optimization VARCHAR(20),
            compiler_version VARCHAR(200),
            compile_time_ms BIGINT,
            compile_cached BOOLEAN,

            -- Execution stats
            instructions BIGINT NOT NULL,
            memory_peak_kb BIGINT,
            memory_rss_kb BIGINT,
            memory_hwm_kb BIGINT,
            memory_data_kb BIGINT,
            memory_stack_kb BIGINT,
            io_read_bytes BIGINT,
            io_write_bytes BIGINT,
            -- Guest memory (actual binary allocations)
            guest_mmap_bytes BIGINT,
            guest_mmap_peak BIGINT,
            guest_heap_bytes BIGINT,
            limit_reached BOOLEAN DEFAULT FALSE,
            exit_code INTEGER,
            execution_time_ms BIGINT,
            instruction_limit BIGINT,

            -- Syscalls (JSONB for flexibility)
            syscalls BIGINT,
            syscall_breakdown JSONB,

            -- Output
            stdout TEXT,
            stderr TEXT,

            -- Benchmark tracking
            benchmark_id VARCHAR(100),

            -- Timestamps
            created_at TIMESTAMPTZ DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs table: {}", e)))?;

    // Create indexes
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_runs_job_id ON runs(job_id)"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs index: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_runs_binary_id ON runs(binary_id)"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs index: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_runs_created_at ON runs(created_at DESC)"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs index: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_runs_instructions ON runs(instructions)"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs index: {}", e)))?;

    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_runs_benchmark ON runs(benchmark_id, language)"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to create runs index: {}", e)))?;

    // Add compiler_version column if it doesn't exist (migration)
    sqlx::query(r#"ALTER TABLE runs ADD COLUMN IF NOT EXISTS compiler_version VARCHAR(200)"#)
        .execute(pool)
        .await
        .ok();

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Run {
    pub id: Uuid,
    pub job_id: Uuid,
    pub binary_id: String,
    pub binary_size: Option<i64>,
    pub source_code: Option<String>,
    pub language: Option<String>,
    pub optimization: Option<String>,
    pub compiler_version: Option<String>,
    pub compile_time_ms: Option<i64>,
    pub compile_cached: Option<bool>,
    pub instructions: i64,
    pub memory_peak_kb: Option<i64>,
    pub memory_rss_kb: Option<i64>,
    pub memory_hwm_kb: Option<i64>,
    pub memory_data_kb: Option<i64>,
    pub memory_stack_kb: Option<i64>,
    pub io_read_bytes: Option<i64>,
    pub io_write_bytes: Option<i64>,
    // Guest memory (actual binary allocations)
    pub guest_mmap_bytes: Option<i64>,
    pub guest_mmap_peak: Option<i64>,
    pub guest_heap_bytes: Option<i64>,
    pub limit_reached: bool,
    pub exit_code: Option<i32>,
    pub execution_time_ms: Option<i64>,
    pub instruction_limit: Option<i64>,
    pub syscalls: Option<i64>,
    pub syscall_breakdown: Option<serde_json::Value>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub benchmark_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveRunRequest {
    pub job_id: Uuid,
    pub binary_id: String,
    pub binary_size: Option<i64>,
    pub source_code: Option<String>,
    pub language: Option<String>,
    pub optimization: Option<String>,
    pub compiler_version: Option<String>,
    pub compile_time_ms: Option<i64>,
    pub compile_cached: Option<bool>,
    pub instructions: i64,
    pub memory_peak_kb: Option<i64>,
    pub memory_rss_kb: Option<i64>,
    pub memory_hwm_kb: Option<i64>,
    pub memory_data_kb: Option<i64>,
    pub memory_stack_kb: Option<i64>,
    pub io_read_bytes: Option<i64>,
    pub io_write_bytes: Option<i64>,
    // Guest memory (actual binary allocations)
    pub guest_mmap_bytes: Option<i64>,
    pub guest_mmap_peak: Option<i64>,
    pub guest_heap_bytes: Option<i64>,
    pub limit_reached: bool,
    pub exit_code: Option<i32>,
    pub execution_time_ms: Option<i64>,
    pub instruction_limit: Option<i64>,
    pub syscalls: Option<i64>,
    pub syscall_breakdown: Option<serde_json::Value>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub benchmark_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub async fn save_run(pool: &PgPool, req: &SaveRunRequest) -> Result<Uuid, ApiError> {
    let result: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO runs (
            job_id, binary_id, binary_size, source_code, language, optimization, compiler_version,
            compile_time_ms, compile_cached, instructions, memory_peak_kb,
            memory_rss_kb, memory_hwm_kb, memory_data_kb, memory_stack_kb,
            io_read_bytes, io_write_bytes, guest_mmap_bytes, guest_mmap_peak,
            guest_heap_bytes, limit_reached, exit_code,
            execution_time_ms, instruction_limit, syscalls, syscall_breakdown,
            stdout, stderr, benchmark_id, started_at, completed_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31)
        ON CONFLICT (job_id) DO UPDATE SET
            instructions = EXCLUDED.instructions,
            memory_peak_kb = EXCLUDED.memory_peak_kb,
            memory_rss_kb = EXCLUDED.memory_rss_kb,
            memory_hwm_kb = EXCLUDED.memory_hwm_kb,
            memory_data_kb = EXCLUDED.memory_data_kb,
            memory_stack_kb = EXCLUDED.memory_stack_kb,
            io_read_bytes = EXCLUDED.io_read_bytes,
            io_write_bytes = EXCLUDED.io_write_bytes,
            guest_mmap_bytes = EXCLUDED.guest_mmap_bytes,
            guest_mmap_peak = EXCLUDED.guest_mmap_peak,
            guest_heap_bytes = EXCLUDED.guest_heap_bytes,
            limit_reached = EXCLUDED.limit_reached,
            exit_code = EXCLUDED.exit_code,
            execution_time_ms = EXCLUDED.execution_time_ms,
            syscalls = EXCLUDED.syscalls,
            syscall_breakdown = EXCLUDED.syscall_breakdown,
            stdout = EXCLUDED.stdout,
            stderr = EXCLUDED.stderr,
            completed_at = EXCLUDED.completed_at
        RETURNING id
        "#,
    )
    .bind(&req.job_id)
    .bind(&req.binary_id)
    .bind(req.binary_size)
    .bind(&req.source_code)
    .bind(&req.language)
    .bind(&req.optimization)
    .bind(&req.compiler_version)
    .bind(req.compile_time_ms)
    .bind(req.compile_cached)
    .bind(req.instructions)
    .bind(req.memory_peak_kb)
    .bind(req.memory_rss_kb)
    .bind(req.memory_hwm_kb)
    .bind(req.memory_data_kb)
    .bind(req.memory_stack_kb)
    .bind(req.io_read_bytes)
    .bind(req.io_write_bytes)
    .bind(req.guest_mmap_bytes)
    .bind(req.guest_mmap_peak)
    .bind(req.guest_heap_bytes)
    .bind(req.limit_reached)
    .bind(req.exit_code)
    .bind(req.execution_time_ms)
    .bind(req.instruction_limit)
    .bind(req.syscalls)
    .bind(&req.syscall_breakdown)
    .bind(&req.stdout)
    .bind(&req.stderr)
    .bind(&req.benchmark_id)
    .bind(req.started_at)
    .bind(req.completed_at)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to save run: {}", e)))?;

    Ok(result.0)
}

pub async fn get_run(pool: &PgPool, run_id: &Uuid) -> Result<Option<Run>, ApiError> {
    let result: Option<Run> = sqlx::query_as(
        r#"
        SELECT id, job_id, binary_id, binary_size, source_code, language, optimization, compiler_version,
               compile_time_ms, compile_cached, instructions, memory_peak_kb,
               memory_rss_kb, memory_hwm_kb, memory_data_kb, memory_stack_kb,
               io_read_bytes, io_write_bytes, guest_mmap_bytes, guest_mmap_peak,
               guest_heap_bytes, limit_reached, exit_code,
               execution_time_ms, instruction_limit, syscalls, syscall_breakdown,
               stdout, stderr, benchmark_id, created_at, started_at, completed_at
        FROM runs
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get run: {}", e)))?;

    Ok(result)
}

pub async fn get_run_by_job_id(pool: &PgPool, job_id: &Uuid) -> Result<Option<Run>, ApiError> {
    let result: Option<Run> = sqlx::query_as(
        r#"
        SELECT id, job_id, binary_id, binary_size, source_code, language, optimization, compiler_version,
               compile_time_ms, compile_cached, instructions, memory_peak_kb,
               memory_rss_kb, memory_hwm_kb, memory_data_kb, memory_stack_kb,
               io_read_bytes, io_write_bytes, guest_mmap_bytes, guest_mmap_peak,
               guest_heap_bytes, limit_reached, exit_code,
               execution_time_ms, instruction_limit, syscalls, syscall_breakdown,
               stdout, stderr, benchmark_id, created_at, started_at, completed_at
        FROM runs
        WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get run by job_id: {}", e)))?;

    Ok(result)
}

pub async fn list_runs(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Run>, ApiError> {
    let results: Vec<Run> = sqlx::query_as(
        r#"
        SELECT id, job_id, binary_id, binary_size, source_code, language, optimization, compiler_version,
               compile_time_ms, compile_cached, instructions, memory_peak_kb,
               memory_rss_kb, memory_hwm_kb, memory_data_kb, memory_stack_kb,
               io_read_bytes, io_write_bytes, guest_mmap_bytes, guest_mmap_peak,
               guest_heap_bytes, limit_reached, exit_code,
               execution_time_ms, instruction_limit, syscalls, syscall_breakdown,
               stdout, stderr, benchmark_id, created_at, started_at, completed_at
        FROM runs
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to list runs: {}", e)))?;

    Ok(results)
}

pub async fn get_min_instructions(
    pool: &PgPool,
    benchmark_id: &str,
) -> Result<HashMap<String, i64>, ApiError> {
    let results: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT language, MIN(instructions) as min_instructions
        FROM runs
        WHERE benchmark_id = $1 AND language IS NOT NULL AND limit_reached = FALSE
        GROUP BY language
        "#,
    )
    .bind(benchmark_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get min instructions: {}", e)))?;

    Ok(results.into_iter().collect())
}

// ============ User Functions ============

pub async fn get_user_by_id(pool: &PgPool, user_id: &Uuid) -> Result<Option<User>, ApiError> {
    let result: Option<User> = sqlx::query_as(
        r#"
        SELECT id, username, github_id, github_login, avatar_url, display_name, bio,
               twitter_handle, COALESCE(is_verified, FALSE) as is_verified, verified_at, verification_method,
               COALESCE(user_type, 'human') as user_type, clanker_twitter, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get user: {}", e)))?;

    Ok(result)
}

pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, ApiError> {
    let result: Option<User> = sqlx::query_as(
        r#"
        SELECT id, username, github_id, github_login, avatar_url, display_name, bio,
               twitter_handle, COALESCE(is_verified, FALSE) as is_verified, verified_at, verification_method,
               COALESCE(user_type, 'human') as user_type, clanker_twitter, created_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get user by username: {}", e)))?;

    Ok(result)
}

pub async fn get_user_by_github_id(pool: &PgPool, github_id: i64) -> Result<Option<User>, ApiError> {
    let result: Option<User> = sqlx::query_as(
        r#"
        SELECT id, username, github_id, github_login, avatar_url, display_name, bio,
               twitter_handle, COALESCE(is_verified, FALSE) as is_verified, verified_at, verification_method,
               COALESCE(user_type, 'human') as user_type, clanker_twitter, created_at
        FROM users
        WHERE github_id = $1
        "#,
    )
    .bind(github_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get user by github_id: {}", e)))?;

    Ok(result)
}

#[derive(Debug)]
pub struct CreateUserFromGitHub {
    pub github_id: i64,
    pub github_login: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
}

pub async fn create_or_update_user_from_github(
    pool: &PgPool,
    data: &CreateUserFromGitHub,
) -> Result<User, ApiError> {
    let result: User = sqlx::query_as(
        r#"
        INSERT INTO users (username, github_id, github_login, avatar_url, display_name)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (github_id) DO UPDATE SET
            github_login = EXCLUDED.github_login,
            avatar_url = COALESCE(EXCLUDED.avatar_url, users.avatar_url),
            display_name = COALESCE(EXCLUDED.display_name, users.display_name)
        RETURNING id, username, github_id, github_login, avatar_url, display_name, bio,
                  twitter_handle, COALESCE(is_verified, FALSE) as is_verified, verified_at, verification_method,
                  COALESCE(user_type, 'human') as user_type, clanker_twitter, created_at
        "#,
    )
    .bind(&data.github_login) // username = github_login initially
    .bind(data.github_id)
    .bind(&data.github_login)
    .bind(&data.avatar_url)
    .bind(&data.display_name)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create/update user: {}", e)))?;

    Ok(result)
}

pub async fn verify_user(
    pool: &PgPool,
    user_id: &Uuid,
    method: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE users
        SET is_verified = TRUE, verified_at = NOW(), verification_method = $2
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(method)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to verify user: {}", e)))?;

    Ok(())
}

pub async fn set_user_type(
    pool: &PgPool,
    user_id: &Uuid,
    user_type: &str,
    clanker_twitter: Option<&str>,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE users
        SET user_type = $2, clanker_twitter = $3
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(user_type)
    .bind(clanker_twitter)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to set user type: {}", e)))?;

    Ok(())
}

pub async fn update_user_profile(
    pool: &PgPool,
    user_id: &Uuid,
    display_name: Option<&str>,
    bio: Option<&str>,
    twitter_handle: Option<&str>,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE users
        SET display_name = COALESCE($2, display_name),
            bio = COALESCE($3, bio),
            twitter_handle = COALESCE($4, twitter_handle)
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(display_name)
    .bind(bio)
    .bind(twitter_handle)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to update user profile: {}", e)))?;

    Ok(())
}

// ============ Session Functions ============

pub async fn create_session(
    pool: &PgPool,
    user_id: &Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<Session, ApiError> {
    let result: Session = sqlx::query_as(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, token_hash, expires_at, created_at
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create session: {}", e)))?;

    Ok(result)
}

pub async fn get_session_by_token_hash(pool: &PgPool, token_hash: &str) -> Result<Option<Session>, ApiError> {
    let result: Option<Session> = sqlx::query_as(
        r#"
        SELECT id, user_id, token_hash, expires_at, created_at
        FROM sessions
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get session: {}", e)))?;

    Ok(result)
}

pub async fn delete_session(pool: &PgPool, session_id: &Uuid) -> Result<(), ApiError> {
    sqlx::query(r#"DELETE FROM sessions WHERE id = $1"#)
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to delete session: {}", e)))?;

    Ok(())
}

pub async fn delete_user_sessions(pool: &PgPool, user_id: &Uuid) -> Result<u64, ApiError> {
    let result = sqlx::query(r#"DELETE FROM sessions WHERE user_id = $1"#)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to delete user sessions: {}", e)))?;

    Ok(result.rows_affected())
}

pub async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64, ApiError> {
    let result = sqlx::query(r#"DELETE FROM sessions WHERE expires_at < NOW()"#)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to cleanup sessions: {}", e)))?;

    Ok(result.rows_affected())
}

// ============ Challenge Functions ============

pub async fn get_challenge(pool: &PgPool, challenge_id: &str) -> Result<Option<Challenge>, ApiError> {
    let result: Option<Challenge> = sqlx::query_as(
        r#"
        SELECT id, name, description, category, difficulty, input_spec, output_spec,
               test_cases, verify_mode, is_active, created_at,
               COALESCE(network_enabled, FALSE) as network_enabled, env_vars, baselines
        FROM challenges
        WHERE id = $1
        "#,
    )
    .bind(challenge_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get challenge: {}", e)))?;

    Ok(result)
}

pub async fn list_challenges(pool: &PgPool, active_only: bool) -> Result<Vec<Challenge>, ApiError> {
    let results: Vec<Challenge> = if active_only {
        sqlx::query_as(
            r#"
            SELECT id, name, description, category, difficulty, input_spec, output_spec,
                   test_cases, verify_mode, is_active, created_at,
                   COALESCE(network_enabled, FALSE) as network_enabled, env_vars, baselines
            FROM challenges
            WHERE is_active = TRUE
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(
            r#"
            SELECT id, name, description, category, difficulty, input_spec, output_spec,
                   test_cases, verify_mode, is_active, created_at,
                   COALESCE(network_enabled, FALSE) as network_enabled, env_vars, baselines
            FROM challenges
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ApiError::DatabaseError(format!("Failed to list challenges: {}", e)))?;

    Ok(results)
}

pub async fn create_challenge(
    pool: &PgPool,
    id: &str,
    name: &str,
    description: &str,
    category: &str,
    difficulty: &str,
    input_spec: Option<&str>,
    output_spec: &str,
    test_cases: &serde_json::Value,
    verify_mode: &str,
    network_enabled: bool,
    env_vars: Option<&serde_json::Value>,
    baselines: Option<&serde_json::Value>,
) -> Result<Challenge, ApiError> {
    let result: Challenge = sqlx::query_as(
        r#"
        INSERT INTO challenges (id, name, description, category, difficulty, input_spec, output_spec, test_cases, verify_mode, network_enabled, env_vars, baselines)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            category = EXCLUDED.category,
            difficulty = EXCLUDED.difficulty,
            input_spec = EXCLUDED.input_spec,
            output_spec = EXCLUDED.output_spec,
            test_cases = EXCLUDED.test_cases,
            verify_mode = EXCLUDED.verify_mode,
            network_enabled = EXCLUDED.network_enabled,
            env_vars = EXCLUDED.env_vars,
            baselines = EXCLUDED.baselines
        RETURNING id, name, description, category, difficulty, input_spec, output_spec,
                  test_cases, verify_mode, is_active, created_at,
                  COALESCE(network_enabled, FALSE) as network_enabled, env_vars, baselines
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(category)
    .bind(difficulty)
    .bind(input_spec)
    .bind(output_spec)
    .bind(test_cases)
    .bind(verify_mode)
    .bind(network_enabled)
    .bind(env_vars)
    .bind(baselines)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create challenge: {}", e)))?;

    Ok(result)
}

// ============ Challenge Submission Functions ============

pub async fn create_challenge_submission(
    pool: &PgPool,
    user_id: &Uuid,
    challenge_id: &str,
    language: &str,
    source_code: &str,
) -> Result<ChallengeSubmission, ApiError> {
    let result: ChallengeSubmission = sqlx::query_as(
        r#"
        INSERT INTO challenge_submissions (user_id, challenge_id, language, source_code)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, challenge_id, language, source_code, binary_id, status,
                  test_results, instructions, error_message, created_at, completed_at
        "#,
    )
    .bind(user_id)
    .bind(challenge_id)
    .bind(language)
    .bind(source_code)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create challenge submission: {}", e)))?;

    Ok(result)
}

pub async fn get_challenge_submission(pool: &PgPool, submission_id: &Uuid) -> Result<Option<ChallengeSubmission>, ApiError> {
    let result: Option<ChallengeSubmission> = sqlx::query_as(
        r#"
        SELECT id, user_id, challenge_id, language, source_code, binary_id, status,
               test_results, instructions, error_message, created_at, completed_at
        FROM challenge_submissions
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get challenge submission: {}", e)))?;

    Ok(result)
}

pub async fn update_challenge_submission_status(
    pool: &PgPool,
    submission_id: &Uuid,
    status: &str,
    binary_id: Option<&str>,
    test_results: Option<&serde_json::Value>,
    instructions: Option<i64>,
    error_message: Option<&str>,
) -> Result<(), ApiError> {
    let completed_at = if status == "passed" || status == "failed" {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE challenge_submissions
        SET status = $2,
            binary_id = COALESCE($3, binary_id),
            test_results = COALESCE($4, test_results),
            instructions = COALESCE($5, instructions),
            error_message = COALESCE($6, error_message),
            completed_at = COALESCE($7, completed_at)
        WHERE id = $1
        "#,
    )
    .bind(submission_id)
    .bind(status)
    .bind(binary_id)
    .bind(test_results)
    .bind(instructions)
    .bind(error_message)
    .bind(completed_at)
    .execute(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to update challenge submission: {}", e)))?;

    Ok(())
}

// ============ Leaderboard Functions ============

pub async fn update_leaderboard_entry(
    pool: &PgPool,
    user_id: &Uuid,
    challenge_id: &str,
    language: &str,
    instructions: i64,
    run_id: &Uuid,
    source_code: &str,
    is_verified: bool,
) -> Result<LeaderboardEntry, ApiError> {
    // Only update if this is a better score (lower instructions)
    let result: LeaderboardEntry = sqlx::query_as(
        r#"
        INSERT INTO leaderboard_entries (user_id, challenge_id, language, instructions, run_id, source_code, is_verified)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id, challenge_id, language) DO UPDATE SET
            instructions = CASE WHEN EXCLUDED.instructions < leaderboard_entries.instructions
                               THEN EXCLUDED.instructions
                               ELSE leaderboard_entries.instructions END,
            run_id = CASE WHEN EXCLUDED.instructions < leaderboard_entries.instructions
                         THEN EXCLUDED.run_id
                         ELSE leaderboard_entries.run_id END,
            source_code = CASE WHEN EXCLUDED.instructions < leaderboard_entries.instructions
                              THEN EXCLUDED.source_code
                              ELSE leaderboard_entries.source_code END,
            is_verified = CASE WHEN EXCLUDED.instructions < leaderboard_entries.instructions
                              THEN EXCLUDED.is_verified
                              ELSE leaderboard_entries.is_verified END,
            created_at = CASE WHEN EXCLUDED.instructions < leaderboard_entries.instructions
                             THEN NOW()
                             ELSE leaderboard_entries.created_at END
        RETURNING id, user_id, challenge_id, language, instructions, run_id, source_code, is_verified, created_at
        "#,
    )
    .bind(user_id)
    .bind(challenge_id)
    .bind(language)
    .bind(instructions)
    .bind(run_id)
    .bind(source_code)
    .bind(is_verified)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to update leaderboard entry: {}", e)))?;

    Ok(result)
}

pub async fn get_challenge_leaderboard(
    pool: &PgPool,
    challenge_id: &str,
    language: Option<&str>,
    user_type: Option<&str>,
    limit: i64,
) -> Result<Vec<LeaderboardEntryWithUser>, ApiError> {
    let results: Vec<(i64, Uuid, String, Option<String>, Option<String>, Option<String>, bool, String, DateTime<Utc>, i64, String, DateTime<Utc>)> =
        if let Some(lang) = language {
            if let Some(utype) = user_type {
                sqlx::query_as(
                    r#"
                    SELECT
                        ROW_NUMBER() OVER (ORDER BY le.instructions ASC) as rank,
                        u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                        COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                        le.instructions, le.language, le.created_at as submitted_at
                    FROM leaderboard_entries le
                    JOIN users u ON le.user_id = u.id
                    WHERE le.challenge_id = $1 AND le.language = $2 AND COALESCE(u.user_type, 'human') = $4
                    ORDER BY le.instructions ASC
                    LIMIT $3
                    "#,
                )
                .bind(challenge_id)
                .bind(lang)
                .bind(limit)
                .bind(utype)
                .fetch_all(pool)
                .await
            } else {
                sqlx::query_as(
                    r#"
                    SELECT
                        ROW_NUMBER() OVER (ORDER BY le.instructions ASC) as rank,
                        u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                        COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                        le.instructions, le.language, le.created_at as submitted_at
                    FROM leaderboard_entries le
                    JOIN users u ON le.user_id = u.id
                    WHERE le.challenge_id = $1 AND le.language = $2
                    ORDER BY le.instructions ASC
                    LIMIT $3
                    "#,
                )
                .bind(challenge_id)
                .bind(lang)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        } else if let Some(utype) = user_type {
            sqlx::query_as(
                r#"
                SELECT
                    ROW_NUMBER() OVER (PARTITION BY le.language ORDER BY le.instructions ASC) as rank,
                    u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                    COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                    le.instructions, le.language, le.created_at as submitted_at
                FROM leaderboard_entries le
                JOIN users u ON le.user_id = u.id
                WHERE le.challenge_id = $1 AND COALESCE(u.user_type, 'human') = $3
                ORDER BY le.language, le.instructions ASC
                LIMIT $2
                "#,
            )
            .bind(challenge_id)
            .bind(limit)
            .bind(utype)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT
                    ROW_NUMBER() OVER (PARTITION BY le.language ORDER BY le.instructions ASC) as rank,
                    u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                    COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                    le.instructions, le.language, le.created_at as submitted_at
                FROM leaderboard_entries le
                JOIN users u ON le.user_id = u.id
                WHERE le.challenge_id = $1
                ORDER BY le.language, le.instructions ASC
                LIMIT $2
                "#,
            )
            .bind(challenge_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        .map_err(|e| ApiError::DatabaseError(format!("Failed to get leaderboard: {}", e)))?;

    Ok(results
        .into_iter()
        .map(|(rank, id, username, avatar_url, display_name, twitter_handle, is_verified, user_type, created_at, instructions, language, submitted_at)| {
            LeaderboardEntryWithUser {
                rank,
                user: PublicUser {
                    id,
                    username,
                    avatar_url,
                    display_name,
                    bio: None,
                    twitter_handle,
                    is_verified,
                    user_type,
                    created_at,
                },
                instructions,
                language,
                submitted_at,
            }
        })
        .collect())
}

pub async fn get_user_challenge_stats(
    pool: &PgPool,
    user_id: &Uuid,
) -> Result<Vec<LeaderboardEntry>, ApiError> {
    let results: Vec<LeaderboardEntry> = sqlx::query_as(
        r#"
        SELECT id, user_id, challenge_id, language, instructions, run_id, source_code, is_verified, created_at
        FROM leaderboard_entries
        WHERE user_id = $1
        ORDER BY challenge_id, language
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get user challenge stats: {}", e)))?;

    Ok(results)
}

// ============ Verification Code Functions ============

pub async fn create_verification_code(
    pool: &PgPool,
    user_id: &Uuid,
    code: &str,
    twitter_handle: &str,
    expires_at: DateTime<Utc>,
) -> Result<VerificationCode, ApiError> {
    let result: VerificationCode = sqlx::query_as(
        r#"
        INSERT INTO verification_codes (user_id, code, twitter_handle, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, code, twitter_handle, expires_at, verified, created_at
        "#,
    )
    .bind(user_id)
    .bind(code)
    .bind(twitter_handle)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to create verification code: {}", e)))?;

    Ok(result)
}

pub async fn get_verification_code(pool: &PgPool, user_id: &Uuid) -> Result<Option<VerificationCode>, ApiError> {
    let result: Option<VerificationCode> = sqlx::query_as(
        r#"
        SELECT id, user_id, code, twitter_handle, expires_at, verified, created_at
        FROM verification_codes
        WHERE user_id = $1 AND expires_at > NOW() AND verified = FALSE
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::DatabaseError(format!("Failed to get verification code: {}", e)))?;

    Ok(result)
}

pub async fn mark_verification_code_used(pool: &PgPool, code_id: &Uuid) -> Result<(), ApiError> {
    sqlx::query(r#"UPDATE verification_codes SET verified = TRUE WHERE id = $1"#)
        .bind(code_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(format!("Failed to mark verification code used: {}", e)))?;

    Ok(())
}

// ============ Global Leaderboard ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalLeaderboardEntry {
    pub rank: i64,
    pub user: PublicUser,
    pub total_score: i64,
    pub challenges_completed: i64,
    pub first_places: i64,
}

pub async fn get_global_leaderboard(
    pool: &PgPool,
    user_type: Option<&str>,
    limit: i64,
) -> Result<Vec<GlobalLeaderboardEntry>, ApiError> {
    // Score = sum of (best_in_language / user_instructions * 1000) for each entry
    // Plus bonus for #1 positions
    let results: Vec<(i64, Uuid, String, Option<String>, Option<String>, Option<String>, bool, String, DateTime<Utc>, i64, i64, i64)> =
        if let Some(utype) = user_type {
            sqlx::query_as(
                r#"
                WITH user_scores AS (
                    SELECT
                        le.user_id,
                        COUNT(DISTINCT le.challenge_id) as challenges_completed,
                        SUM(
                            CASE
                                WHEN le.instructions = (
                                    SELECT MIN(le2.instructions)
                                    FROM leaderboard_entries le2
                                    WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                                ) THEN 1000
                                ELSE (
                                    SELECT MIN(le2.instructions)::float / le.instructions::float * 1000
                                    FROM leaderboard_entries le2
                                    WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                                )::bigint
                            END
                        ) as total_score,
                        SUM(
                            CASE WHEN le.instructions = (
                                SELECT MIN(le2.instructions)
                                FROM leaderboard_entries le2
                                WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                            ) THEN 1 ELSE 0 END
                        ) as first_places
                    FROM leaderboard_entries le
                    JOIN users u ON le.user_id = u.id
                    WHERE COALESCE(u.user_type, 'human') = $2
                    GROUP BY le.user_id
                )
                SELECT
                    ROW_NUMBER() OVER (ORDER BY us.total_score DESC) as rank,
                    u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                    COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                    us.total_score, us.challenges_completed, us.first_places
                FROM user_scores us
                JOIN users u ON us.user_id = u.id
                ORDER BY us.total_score DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .bind(utype)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                WITH user_scores AS (
                    SELECT
                        le.user_id,
                        COUNT(DISTINCT le.challenge_id) as challenges_completed,
                        SUM(
                            CASE
                                WHEN le.instructions = (
                                    SELECT MIN(le2.instructions)
                                    FROM leaderboard_entries le2
                                    WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                                ) THEN 1000
                                ELSE (
                                    SELECT MIN(le2.instructions)::float / le.instructions::float * 1000
                                    FROM leaderboard_entries le2
                                    WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                                )::bigint
                            END
                        ) as total_score,
                        SUM(
                            CASE WHEN le.instructions = (
                                SELECT MIN(le2.instructions)
                                FROM leaderboard_entries le2
                                WHERE le2.challenge_id = le.challenge_id AND le2.language = le.language
                            ) THEN 1 ELSE 0 END
                        ) as first_places
                    FROM leaderboard_entries le
                    GROUP BY le.user_id
                )
                SELECT
                    ROW_NUMBER() OVER (ORDER BY us.total_score DESC) as rank,
                    u.id, u.username, u.avatar_url, u.display_name, u.twitter_handle,
                    COALESCE(u.is_verified, FALSE) as is_verified, COALESCE(u.user_type, 'human') as user_type, u.created_at,
                    us.total_score, us.challenges_completed, us.first_places
                FROM user_scores us
                JOIN users u ON us.user_id = u.id
                ORDER BY us.total_score DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        .map_err(|e| ApiError::DatabaseError(format!("Failed to get global leaderboard: {}", e)))?;

    Ok(results
        .into_iter()
        .map(|(rank, id, username, avatar_url, display_name, twitter_handle, is_verified, user_type, created_at, total_score, challenges_completed, first_places)| {
            GlobalLeaderboardEntry {
                rank,
                user: PublicUser {
                    id,
                    username,
                    avatar_url,
                    display_name,
                    bio: None,
                    twitter_handle,
                    is_verified,
                    user_type,
                    created_at,
                },
                total_score,
                challenges_completed,
                first_places,
            }
        })
        .collect())
}
