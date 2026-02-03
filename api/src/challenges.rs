use crate::auth::AuthenticatedUser;
use crate::db::{self, Challenge, TestCase, VerifyMode};
use crate::error::ApiError;
use crate::queue::{CompileJob, CompileStatus, Job, JobStatus, Language, Optimization, QueueClient};
use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

// ============ Response Types ============

#[derive(Debug, Serialize)]
pub struct ChallengeListResponse {
    pub challenges: Vec<ChallengeInfo>,
}

#[derive(Debug, Serialize)]
pub struct ChallengeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub difficulty: String,
    pub is_active: bool,
}

impl From<Challenge> for ChallengeInfo {
    fn from(c: Challenge) -> Self {
        ChallengeInfo {
            id: c.id,
            name: c.name,
            description: c.description,
            category: c.category,
            difficulty: c.difficulty,
            is_active: c.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChallengeDetailResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub difficulty: String,
    pub input_spec: Option<String>,
    pub output_spec: String,
    pub test_cases: Vec<PublicTestCase>,
    pub verify_mode: String,
    pub baselines: Option<Vec<ChallengeBaseline>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeBaseline {
    pub language: String,
    pub name: String,
    pub tier: String,
    pub source_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_instructions: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PublicTestCase {
    pub description: Option<String>,
    pub stdin: String,
    // expected_stdout is hidden to prevent cheating
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub submission_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SubmissionStatusResponse {
    pub submission_id: Uuid,
    pub status: String,
    pub test_results: Option<Vec<TestResult>>,
    pub instructions: Option<i64>,
    pub error_message: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_index: usize,
    pub passed: bool,
    pub expected_preview: Option<String>, // First 50 chars of expected output
    pub actual_preview: Option<String>,   // First 50 chars of actual output
    pub error: Option<String>,
}

// ============ Query Types ============

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub language: Option<String>,
    pub user_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

// ============ Handlers ============

pub async fn list_challenges(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Json<ChallengeListResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let challenges = db::list_challenges(pool, true).await?;

    Ok(Json(ChallengeListResponse {
        challenges: challenges.into_iter().map(|c| c.into()).collect(),
    }))
}

pub async fn get_challenge(
    State(state): State<Arc<crate::AppState>>,
    Path(challenge_id): Path<String>,
) -> Result<Json<ChallengeDetailResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let challenge = db::get_challenge(pool, &challenge_id)
        .await?
        .ok_or_else(|| ApiError::ChallengeNotFound(challenge_id))?;

    // Parse test cases but hide expected output
    let test_cases: Vec<TestCase> = serde_json::from_value(challenge.test_cases.clone())
        .map_err(|e| ApiError::Internal(format!("Invalid test cases: {}", e)))?;

    let public_test_cases: Vec<PublicTestCase> = test_cases
        .into_iter()
        .map(|tc| PublicTestCase {
            description: tc.description,
            stdin: tc.stdin,
        })
        .collect();

    // Parse baselines
    let baselines: Option<Vec<ChallengeBaseline>> = challenge.baselines
        .and_then(|v| serde_json::from_value(v).ok());

    Ok(Json(ChallengeDetailResponse {
        id: challenge.id,
        name: challenge.name,
        description: challenge.description,
        category: challenge.category,
        difficulty: challenge.difficulty,
        input_spec: challenge.input_spec,
        output_spec: challenge.output_spec,
        test_cases: public_test_cases,
        verify_mode: challenge.verify_mode,
        baselines,
    }))
}

pub async fn submit_challenge(
    State(state): State<Arc<crate::AppState>>,
    Path(challenge_id): Path<String>,
    AuthenticatedUser(user): AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Json<SubmitResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| ApiError::QueueError("Queue not available".to_string()))?;

    // Verify challenge exists
    let challenge = db::get_challenge(pool, &challenge_id)
        .await?
        .ok_or_else(|| ApiError::ChallengeNotFound(challenge_id.clone()))?;

    // Parse multipart form
    let mut source_code: Option<String> = None;
    let mut language: Option<String> = None;
    let mut optimization: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "source_code" => {
                source_code = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?,
                );
            }
            "language" => {
                language = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?,
                );
            }
            "optimization" => {
                optimization = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::Internal(e.to_string()))?,
                );
            }
            _ => {}
        }
    }

    let source_code = source_code.ok_or(ApiError::MissingField("source_code"))?;
    let language = language.ok_or(ApiError::MissingField("language"))?;

    // Create challenge submission
    let submission = db::create_challenge_submission(
        pool,
        &user.id,
        &challenge_id,
        &language,
        &source_code,
    )
    .await?;

    info!(
        submission_id = %submission.id,
        user_id = %user.id,
        challenge_id = %challenge_id,
        language = %language,
        "Challenge submission created"
    );

    // Process synchronously (queue operations are async but we need to wait)
    // In production, this could be moved to a background worker
    let submission_id = submission.id;

    if let Err(e) = process_challenge_submission(
        pool,
        queue,
        submission_id,
        &challenge,
        &user,
        &source_code,
        &language,
        optimization.as_deref(),
    )
    .await
    {
        warn!(
            submission_id = %submission_id,
            error = %e,
            "Challenge submission processing failed"
        );
        // Update status to failed
        let _ = db::update_challenge_submission_status(
            pool,
            &submission_id,
            "failed",
            None,
            None,
            None,
            Some(&e.to_string()),
        )
        .await;
    }

    Ok(Json(SubmitResponse {
        submission_id,
        status: "pending".to_string(),
    }))
}

async fn process_challenge_submission(
    pool: &PgPool,
    queue: &QueueClient,
    submission_id: Uuid,
    challenge: &Challenge,
    user: &db::User,
    source_code: &str,
    language_str: &str,
    optimization_str: Option<&str>,
) -> Result<(), ApiError> {
    // Update status to compiling
    db::update_challenge_submission_status(pool, &submission_id, "compiling", None, None, None, None).await?;

    // Parse language
    let language = Language::from_str(language_str)
        .ok_or_else(|| ApiError::InvalidLanguage(language_str.to_string()))?;

    let optimization = optimization_str
        .and_then(|s| Optimization::from_str(s))
        .unwrap_or(Optimization::Release);

    // Submit compile job
    let compile_job = CompileJob {
        id: Uuid::new_v4(),
        user_id: Some(user.id),
        source_code: source_code.to_string(),
        language,
        optimization,
        flags: HashMap::new(),
        created_at: Utc::now(),
    };

    let compile_job_id = compile_job.id;
    queue.submit_compile_job(compile_job).await?;

    // Wait for compilation
    let compile_result = wait_for_compile(&queue, compile_job_id, Duration::from_secs(120)).await?;

    let binary_id = compile_result.binary_id;
    db::update_challenge_submission_status(pool, &submission_id, "running", Some(&binary_id), None, None, None).await?;

    // Parse test cases
    let test_cases: Vec<TestCase> = serde_json::from_value(challenge.test_cases.clone())
        .map_err(|e| ApiError::Internal(format!("Invalid test cases: {}", e)))?;

    let verify_mode = match challenge.verify_mode.as_str() {
        "exact" => VerifyMode::Exact,
        "trimmed" => VerifyMode::Trimmed,
        "sorted" => VerifyMode::Sorted,
        _ => VerifyMode::Exact,
    };

    // Run each test case and collect results
    let mut test_results = Vec::new();
    let mut all_passed = true;
    let mut total_instructions: i64 = 0;
    let mut max_instructions: i64 = 0;
    let mut final_run_id: Option<Uuid> = None;

    // Parse challenge env_vars if present
    let challenge_env_vars: std::collections::HashMap<String, String> = challenge.env_vars
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    for (i, test_case) in test_cases.iter().enumerate() {
        // Submit execute job
        let job = Job {
            id: Uuid::new_v4(),
            user_id: Some(user.id),
            binary_id: binary_id.clone(),
            instruction_limit: 1_000_000_000, // 1B instruction limit for challenges
            stdin: test_case.stdin.as_bytes().to_vec(),
            created_at: Utc::now(),
            benchmark_id: Some(challenge.id.clone()),
            network_enabled: challenge.network_enabled,
            env_vars: challenge_env_vars.clone(),
        };

        let job_id = job.id;
        queue.submit_job(job).await?;

        // Wait for execution
        let exec_result = match wait_for_execution(&queue, job_id, Duration::from_secs(30)).await {
            Ok(result) => result,
            Err(e) => {
                test_results.push(TestResult {
                    test_index: i,
                    passed: false,
                    expected_preview: Some(truncate_preview(&test_case.expected_stdout, 50)),
                    actual_preview: None,
                    error: Some(format!("Execution failed: {}", e)),
                });
                all_passed = false;
                continue;
            }
        };

        // Get the run from database (saved by worker)
        if let Ok(Some(run)) = db::get_run_by_job_id(pool, &job_id).await {
            final_run_id = Some(run.id);
        }

        // Check output
        let actual_output = exec_result.stdout.clone();
        let passed = verify_output(&actual_output, &test_case.expected_stdout, &verify_mode);

        if !passed {
            all_passed = false;
        }

        total_instructions += exec_result.instructions as i64;
        if exec_result.instructions as i64 > max_instructions {
            max_instructions = exec_result.instructions as i64;
        }

        test_results.push(TestResult {
            test_index: i,
            passed,
            expected_preview: Some(truncate_preview(&test_case.expected_stdout, 50)),
            actual_preview: Some(truncate_preview(&actual_output, 50)),
            error: if exec_result.exit_code != 0 {
                Some(format!("Exit code: {}", exec_result.exit_code))
            } else {
                None
            },
        });
    }

    // Update submission with results
    let status = if all_passed { "passed" } else { "failed" };
    let test_results_json = serde_json::to_value(&test_results)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize test results: {}", e)))?;

    db::update_challenge_submission_status(
        pool,
        &submission_id,
        status,
        None,
        Some(&test_results_json),
        Some(max_instructions),
        None,
    )
    .await?;

    // If all tests passed, update leaderboard
    if all_passed {
        if let Some(run_id) = final_run_id {
            db::update_leaderboard_entry(
                pool,
                &user.id,
                &challenge.id,
                language_str,
                max_instructions,
                &run_id,
                source_code,
                user.is_verified,
            )
            .await?;

            info!(
                user_id = %user.id,
                challenge_id = %challenge.id,
                language = %language_str,
                instructions = max_instructions,
                "Leaderboard entry updated"
            );
        }
    }

    Ok(())
}

fn verify_output(actual: &str, expected: &str, mode: &VerifyMode) -> bool {
    match mode {
        VerifyMode::Exact => actual == expected,
        VerifyMode::Trimmed => {
            let actual_lines: Vec<&str> = actual.lines().map(|l| l.trim()).collect();
            let expected_lines: Vec<&str> = expected.lines().map(|l| l.trim()).collect();
            actual_lines == expected_lines
        }
        VerifyMode::Sorted => {
            let mut actual_lines: Vec<&str> = actual.lines().map(|l| l.trim()).collect();
            let mut expected_lines: Vec<&str> = expected.lines().map(|l| l.trim()).collect();
            actual_lines.sort();
            expected_lines.sort();
            actual_lines == expected_lines
        }
    }
}

fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

async fn wait_for_compile(
    queue: &QueueClient,
    job_id: Uuid,
    timeout: Duration,
) -> Result<crate::queue::CompileResult, ApiError> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(ApiError::Timeout(timeout.as_secs()));
        }

        if let Some(metadata) = queue.get_compile_status(&job_id).await? {
            match metadata.status {
                CompileStatus::Completed => {
                    if let Some(result) = queue.get_compile_result(&job_id).await? {
                        return Ok(result);
                    }
                }
                CompileStatus::Failed => {
                    return Err(ApiError::CompileError(
                        metadata.error.unwrap_or_else(|| "Compilation failed".to_string()),
                    ));
                }
                _ => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn wait_for_execution(
    queue: &QueueClient,
    job_id: Uuid,
    timeout: Duration,
) -> Result<crate::sandbox::ExecutionResult, ApiError> {
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            return Err(ApiError::Timeout(timeout.as_secs()));
        }

        if let Some(metadata) = queue.get_job_status(&job_id).await? {
            match metadata.status {
                JobStatus::Completed => {
                    if let Some(result) = queue.get_job_result(&job_id).await? {
                        return Ok(result);
                    }
                }
                JobStatus::Failed => {
                    return Err(ApiError::Internal(
                        metadata.error.unwrap_or_else(|| "Execution failed".to_string()),
                    ));
                }
                _ => {}
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub async fn get_submission_status(
    State(state): State<Arc<crate::AppState>>,
    Path((challenge_id, submission_id)): Path<(String, Uuid)>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<Json<SubmissionStatusResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let submission = db::get_challenge_submission(pool, &submission_id)
        .await?
        .ok_or_else(|| ApiError::SubmissionNotFound(submission_id.to_string()))?;

    // Verify user owns this submission
    if submission.user_id != user.id {
        return Err(ApiError::Forbidden("You don't own this submission".to_string()));
    }

    // Verify submission is for this challenge
    if submission.challenge_id != challenge_id {
        return Err(ApiError::SubmissionNotFound(submission_id.to_string()));
    }

    let test_results: Option<Vec<TestResult>> = submission
        .test_results
        .and_then(|v| serde_json::from_value(v).ok());

    Ok(Json(SubmissionStatusResponse {
        submission_id: submission.id,
        status: submission.status,
        test_results,
        instructions: submission.instructions,
        error_message: submission.error_message,
        completed_at: submission.completed_at.map(|t| t.to_rfc3339()),
    }))
}

pub async fn get_challenge_leaderboard(
    State(state): State<Arc<crate::AppState>>,
    Path(challenge_id): Path<String>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Vec<db::LeaderboardEntryWithUser>>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Verify challenge exists
    db::get_challenge(pool, &challenge_id)
        .await?
        .ok_or_else(|| ApiError::ChallengeNotFound(challenge_id.clone()))?;

    let leaderboard = db::get_challenge_leaderboard(
        pool,
        &challenge_id,
        query.language.as_deref(),
        query.user_type.as_deref(),
        query.limit.min(500),
    )
    .await?;

    Ok(Json(leaderboard))
}

// ============ Global Leaderboard ============

#[derive(Debug, Deserialize)]
pub struct GlobalLeaderboardQuery {
    pub user_type: Option<String>,
    #[serde(default = "default_global_limit")]
    pub limit: i64,
}

fn default_global_limit() -> i64 {
    100
}

pub async fn get_global_leaderboard(
    State(state): State<Arc<crate::AppState>>,
    Query(query): Query<GlobalLeaderboardQuery>,
) -> Result<Json<Vec<db::GlobalLeaderboardEntry>>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let leaderboard = db::get_global_leaderboard(
        pool,
        query.user_type.as_deref(),
        query.limit.min(500),
    )
    .await?;

    Ok(Json(leaderboard))
}

// ============ Challenge Seeding ============

pub async fn seed_challenges(pool: &PgPool) -> Result<(), ApiError> {
    // Hello World challenge (simplest baseline)
    let hello_tests = serde_json::json!([
        {
            "stdin": "",
            "expected_stdout": "Hello, World!\n",
            "description": "Print greeting"
        }
    ]);

    let hello_baselines = serde_json::json!([
        {
            "language": "asm",
            "name": "Assembly (x86_64)",
            "tier": "native",
            "source_code": r#".global _start
.section .data
msg: .ascii "Hello, World!\n"
.section .text
_start:
    mov $1, %rax
    mov $1, %rdi
    lea msg(%rip), %rsi
    mov $14, %rdx
    syscall
    mov $60, %rax
    xor %rdi, %rdi
    syscall"#
        },
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": "#include <stdio.h>\nint main() { printf(\"Hello, World!\\n\"); return 0; }"
        },
        {
            "language": "rust",
            "name": "Rust",
            "tier": "native",
            "source_code": "fn main() { println!(\"Hello, World!\"); }"
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"Hello, World!\") }"
        },
        {
            "language": "zig",
            "name": "Zig",
            "tier": "native",
            "source_code": "const std = @import(\"std\");\npub fn main() !void {\n    const stdout = std.io.getStdOut().writer();\n    try stdout.print(\"Hello, World!\\n\", .{});\n}"
        },
        {
            "language": "nim",
            "name": "Nim",
            "tier": "native",
            "source_code": "echo \"Hello, World!\""
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": "print(\"Hello, World!\")"
        }
    ]);

    db::create_challenge(
        pool,
        "hello-world",
        "Hello World",
        "Print \"Hello, World!\" followed by a newline. The simplest challenge - establish your baseline instruction count.",
        "intro",
        "easy",
        None,
        "Print exactly: Hello, World!",
        &hello_tests,
        "exact",
        false,
        None,
        Some(&hello_baselines),
    )
    .await?;

    // Port Scanner challenge (needs network)
    let portscan_tests = serde_json::json!([
        {
            "stdin": "",
            "expected_stdout": "22 open\n80 open\n443 open\n",
            "description": "All ports open"
        }
    ]);

    let portscan_baselines = serde_json::json!([
        {
            "language": "asm",
            "name": "Assembly (x86_64)",
            "tier": "native",
            "source_code": r#".global _start
.section .data
ports: .word 22, 80, 443
msg_open: .ascii " open\n"
.section .bss
buf: .skip 16
.section .text
_start:
    xor %r12d, %r12d
.loop:
    cmp $3, %r12d
    jge .exit
    mov $41, %rax
    mov $2, %rdi
    mov $1, %rsi
    xor %rdx, %rdx
    syscall
    mov %rax, %r13
    sub $16, %rsp
    movw $2, (%rsp)
    movzwl ports(,%r12,2), %eax
    xchg %al, %ah
    movw %ax, 2(%rsp)
    movl $0x0100007f, 4(%rsp)
    mov $42, %rax
    mov %r13, %rdi
    mov %rsp, %rsi
    mov $16, %rdx
    syscall
    add $16, %rsp
    test %rax, %rax
    jnz .close
    movzwl ports(,%r12,2), %eax
    lea buf(%rip), %rdi
    call itoa
    mov $1, %rax
    mov $1, %rdi
    lea buf(%rip), %rsi
    syscall
    mov $1, %rax
    mov $1, %rdi
    lea msg_open(%rip), %rsi
    mov $6, %rdx
    syscall
.close:
    mov $3, %rax
    mov %r13, %rdi
    syscall
    inc %r12d
    jmp .loop
.exit:
    mov $60, %rax
    xor %rdi, %rdi
    syscall
itoa:
    mov %eax, %ecx
    xor %edx, %edx
    mov $10, %r8d
.itoa_loop:
    xor %edx, %edx
    div %r8d
    add $'0', %dl
    movb %dl, (%rdi)
    inc %rdi
    test %eax, %eax
    jnz .itoa_loop
    mov %rdi, %rax
    sub $buf, %rax
    mov %rax, %rdx
    ret"#
        },
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": "#include <stdio.h>\n#include <sys/socket.h>\n#include <netinet/in.h>\n#include <unistd.h>\nint main() {\n    int ports[] = {22, 80, 443};\n    for (int i = 0; i < 3; i++) {\n        int s = socket(AF_INET, SOCK_STREAM, 0);\n        struct sockaddr_in a = {.sin_family = AF_INET, .sin_port = htons(ports[i]), .sin_addr.s_addr = htonl(0x7f000001)};\n        if (connect(s, (void*)&a, sizeof(a)) == 0) printf(\"%d open\\n\", ports[i]);\n        close(s);\n    }\n}"
        },
        {
            "language": "rust",
            "name": "Rust",
            "tier": "native",
            "source_code": "use std::net::TcpStream;\nfn main() {\n    for port in [22, 80, 443] {\n        if TcpStream::connect((\"127.0.0.1\", port)).is_ok() {\n            println!(\"{} open\", port);\n        }\n    }\n}"
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": "package main\nimport (\"fmt\"; \"net\")\nfunc main() {\n    for _, port := range []int{22, 80, 443} {\n        if conn, err := net.Dial(\"tcp\", fmt.Sprintf(\"127.0.0.1:%d\", port)); err == nil {\n            fmt.Printf(\"%d open\\n\", port)\n            conn.Close()\n        }\n    }\n}"
        },
        {
            "language": "zig",
            "name": "Zig",
            "tier": "native",
            "source_code": "const std = @import(\"std\");\npub fn main() !void {\n    const stdout = std.io.getStdOut().writer();\n    const ports = [_]u16{ 22, 80, 443 };\n    for (ports) |port| {\n        const addr = std.net.Address.initIp4(.{ 127, 0, 0, 1 }, port);\n        if (std.posix.socket(std.posix.AF.INET, std.posix.SOCK.STREAM, 0)) |sock| {\n            defer std.posix.close(sock);\n            std.posix.connect(sock, &addr.any, addr.getLen()) catch continue;\n            try stdout.print(\"{d} open\\n\", .{port});\n        } else |_| {}\n    }\n}"
        }
    ]);

    db::create_challenge(
        pool,
        "portscan",
        "Port Scanner",
        "Scan localhost (127.0.0.1) on ports 22, 80, 443. Print \"<port> open\" for each open port, one per line. If a port is closed, print nothing for it.",
        "networking",
        "medium",
        None,
        "Print \"<port> open\" for each open port (22, 80, 443), one per line. Order doesn't matter.",
        &portscan_tests,
        "sorted",
        true,  // Network enabled for port scanning
        None,
        Some(&portscan_baselines),
    )
    .await?;

    // Env Leak challenge (needs env vars)
    let env_vars = serde_json::json!({
        "FLAG": "ctf{env_leak_flag_42}"
    });
    let env_tests = serde_json::json!([
        {
            "stdin": "",
            "expected_stdout": "ctf{env_leak_flag_42}\n",
            "description": "Find the FLAG environment variable"
        }
    ]);

    let env_baselines = serde_json::json!([
        {
            "language": "asm",
            "name": "Assembly (x86_64)",
            "tier": "native",
            "source_code": r#".global _start
.section .text
_start:
    mov 8(%rsp), %rdi
    lea 16(%rsp), %rsi
    lea 8(%rsi,%rdi,8), %r12
find_flag:
    mov (%r12), %rdi
    test %rdi, %rdi
    jz exit
    cmpb $'F', (%rdi)
    jne next
    cmpb $'L', 1(%rdi)
    jne next
    cmpb $'A', 2(%rdi)
    jne next
    cmpb $'G', 3(%rdi)
    jne next
    cmpb $'=', 4(%rdi)
    jne next
    add $5, %rdi
    mov %rdi, %rsi
    xor %rdx, %rdx
strlen:
    cmpb $0, (%rsi,%rdx)
    je print
    inc %rdx
    jmp strlen
print:
    mov $1, %rax
    mov $1, %rdi
    syscall
    push $10
    mov $1, %rax
    mov $1, %rdi
    mov %rsp, %rsi
    mov $1, %rdx
    syscall
    pop %rax
    jmp exit
next:
    add $8, %r12
    jmp find_flag
exit:
    mov $60, %rax
    xor %rdi, %rdi
    syscall"#
        },
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": "#include <stdio.h>\n#include <stdlib.h>\nint main() {\n    char *flag = getenv(\"FLAG\");\n    if (flag) printf(\"%s\\n\", flag);\n    return 0;\n}"
        },
        {
            "language": "rust",
            "name": "Rust",
            "tier": "native",
            "source_code": "use std::env;\nfn main() {\n    if let Ok(flag) = env::var(\"FLAG\") {\n        println!(\"{}\", flag);\n    }\n}"
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": "package main\nimport (\"fmt\"; \"os\")\nfunc main() {\n    if flag := os.Getenv(\"FLAG\"); flag != \"\" {\n        fmt.Println(flag)\n    }\n}"
        },
        {
            "language": "zig",
            "name": "Zig",
            "tier": "native",
            "source_code": "const std = @import(\"std\");\npub fn main() !void {\n    const stdout = std.io.getStdOut().writer();\n    if (std.posix.getenv(\"FLAG\")) |flag| {\n        try stdout.print(\"{s}\\n\", .{flag});\n    }\n}"
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": "import os\nflag = os.environ.get(\"FLAG\")\nif flag:\n    print(flag)"
        }
    ]);

    db::create_challenge(
        pool,
        "env-leak",
        "Env Leak",
        "A flag is hidden in an environment variable called FLAG. Find and print it.",
        "system",
        "easy",
        None,
        "Print the value of the FLAG environment variable.",
        &env_tests,
        "exact",
        false,
        Some(&env_vars),  // Set FLAG env var
        Some(&env_baselines),
    )
    .await?;

    // Base64 Decode challenge
    let b64_tests = serde_json::json!([
        {
            "stdin": "SGVsbG8gV29ybGQh",
            "expected_stdout": "Hello World!",
            "description": "Decode 'Hello World!'"
        },
        {
            "stdin": "VGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZw==",
            "expected_stdout": "The quick brown fox jumps over the lazy dog",
            "description": "Decode pangram"
        },
        {
            "stdin": "Y3Rme2Jhc2U2NF9tYXN0ZXJ9",
            "expected_stdout": "ctf{base64_master}",
            "description": "Decode flag"
        }
    ]);

    let b64_baselines = serde_json::json!([
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": r#"#include <stdio.h>
#include <string.h>
static const char b64[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
int idx(char c) { char *p = strchr(b64, c); return p ? p - b64 : 0; }
int main() {
    char buf[65536];
    int n = fread(buf, 1, sizeof(buf), stdin);
    for (int i = 0; i < n; i += 4) {
        int a = idx(buf[i]), b = idx(buf[i+1]);
        int c = idx(buf[i+2]), d = idx(buf[i+3]);
        putchar((a << 2) | (b >> 4));
        if (buf[i+2] != '=') putchar(((b & 0xf) << 4) | (c >> 2));
        if (buf[i+3] != '=') putchar(((c & 0x3) << 6) | d);
    }
    return 0;
}"#
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": "package main\nimport (\"encoding/base64\"; \"fmt\"; \"io\"; \"os\")\nfunc main() {\n    data, _ := io.ReadAll(os.Stdin)\n    decoded, _ := base64.StdEncoding.DecodeString(string(data))\n    fmt.Print(string(decoded))\n}"
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": "import sys, base64\nprint(base64.b64decode(sys.stdin.read().strip()).decode(), end=\"\")"
        },
        {
            "language": "rust",
            "name": "Rust",
            "tier": "native",
            "source_code": r#"use std::io::{self, Read};
fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let decoded = base64_decode(input.trim());
    print!("{}", String::from_utf8_lossy(&decoded));
}
fn base64_decode(s: &str) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let bytes: Vec<u8> = s.bytes().collect();
    for chunk in bytes.chunks(4) {
        let a = alphabet.iter().position(|&c| c == chunk[0]).unwrap_or(0);
        let b = alphabet.iter().position(|&c| c == chunk[1]).unwrap_or(0);
        out.push(((a << 2) | (b >> 4)) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            let c = alphabet.iter().position(|&x| x == chunk[2]).unwrap_or(0);
            out.push((((b & 0xf) << 4) | (c >> 2)) as u8);
            if chunk.len() > 3 && chunk[3] != b'=' {
                let d = alphabet.iter().position(|&x| x == chunk[3]).unwrap_or(0);
                out.push((((c & 0x3) << 6) | d) as u8);
            }
        }
    }
    out
}"#
        }
    ]);

    db::create_challenge(
        pool,
        "base64-decode",
        "Base64 Decode",
        "Decode a base64-encoded string from stdin and print the decoded output.",
        "crypto",
        "easy",
        Some("Base64-encoded string"),
        "Decoded plaintext",
        &b64_tests,
        "exact",
        false,
        None,
        Some(&b64_baselines),
    )
    .await?;

    // XOR Decode challenge
    let xor_tests = serde_json::json!([
        {
            "stdin": "0x42 213b22193d2d3f2316",
            "expected_stdout": "ctf{xor!}",
            "description": "XOR with key 0x42"
        },
        {
            "stdin": "0xff 9c8b99bc8c9e8c86bc9f9c9a8e9c9c",
            "expected_stdout": "ctf{caesar_shift}",
            "description": "XOR with key 0xff"
        }
    ]);

    let xor_baselines = serde_json::json!([
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int hex2int(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return 0;
}
int main() {
    char buf[65536];
    fgets(buf, sizeof(buf), stdin);
    int key = (hex2int(buf[2]) << 4) | hex2int(buf[3]);
    char *hex = buf + 5;
    int len = strlen(hex);
    if (hex[len-1] == '\n') hex[--len] = 0;
    for (int i = 0; i < len; i += 2) {
        int byte = (hex2int(hex[i]) << 4) | hex2int(hex[i+1]);
        putchar(byte ^ key);
    }
    return 0;
}"#
        },
        {
            "language": "rust",
            "name": "Rust",
            "tier": "native",
            "source_code": r#"use std::io::{self, BufRead};
fn main() {
    let stdin = io::stdin();
    let line = stdin.lock().lines().next().unwrap().unwrap();
    let parts: Vec<&str> = line.split_whitespace().collect();
    let key = u8::from_str_radix(&parts[0][2..], 16).unwrap();
    let hex = parts[1];
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16).unwrap() ^ key)
        .collect();
    print!("{}", String::from_utf8_lossy(&bytes));
}"#
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": r#"package main
import ("bufio"; "fmt"; "os"; "strconv"; "strings")
func main() {
    reader := bufio.NewReader(os.Stdin)
    line, _ := reader.ReadString('\n')
    parts := strings.Fields(line)
    key, _ := strconv.ParseUint(parts[0][2:], 16, 8)
    hex := parts[1]
    for i := 0; i < len(hex); i += 2 {
        b, _ := strconv.ParseUint(hex[i:i+2], 16, 8)
        fmt.Print(string(rune(byte(b) ^ byte(key))))
    }
}"#
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": "import sys\nline = sys.stdin.read().strip()\nparts = line.split()\nkey = int(parts[0], 16)\nhex_data = parts[1]\nresult = bytes(int(hex_data[i:i+2], 16) ^ key for i in range(0, len(hex_data), 2))\nprint(result.decode(), end=\"\")"
        }
    ]);

    db::create_challenge(
        pool,
        "xor-decode",
        "XOR Decode",
        "Decode a XOR-encrypted message. Input format: \"0xKEY hex_data\" where KEY is a single-byte hex key and hex_data is the encrypted message in hex (no spaces).",
        "crypto",
        "medium",
        Some("XOR key and encrypted hex data"),
        "Decrypted plaintext",
        &xor_tests,
        "exact",
        false,
        None,
        Some(&xor_baselines),
    )
    .await?;

    // Crypto Chain challenge (multi-layer: base64 → reverse → xor → rot13)
    // Generate test data: "ctf{crypto_chain}" → ROT13 → XOR(key=0x42) → reverse → base64
    let crypto_chain_tests = serde_json::json!([
        {
            "stdin": "PXF0Mz4wTj5zNCY8SXNPMQ==",
            "expected_stdout": "ctf{crypto_chain}",
            "description": "Decode multi-layer encryption"
        }
    ]);

    let crypto_chain_baselines = serde_json::json!([
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": r#"#include <stdio.h>
#include <string.h>
char b64[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
int b64idx(char c) { for(int i=0;i<64;i++) if(b64[i]==c) return i; return 0; }
int main() {
    char buf[4096]; int n = fread(buf, 1, sizeof(buf), stdin);
    // 1. Base64 decode
    char dec[4096]; int dn = 0;
    for (int i = 0; i < n; i += 4) {
        int a = b64idx(buf[i]), b = b64idx(buf[i+1]);
        int c = b64idx(buf[i+2]), d = b64idx(buf[i+3]);
        dec[dn++] = (a << 2) | (b >> 4);
        if (buf[i+2] != '=') dec[dn++] = (b << 4) | (c >> 2);
        if (buf[i+3] != '=') dec[dn++] = (c << 6) | d;
    }
    // 2. Reverse bytes
    for (int i = 0; i < dn/2; i++) {
        char t = dec[i]; dec[i] = dec[dn-1-i]; dec[dn-1-i] = t;
    }
    // 3. XOR with first 4 bytes as key
    for (int i = 4; i < dn; i++) dec[i] ^= dec[i % 4];
    // 4. ROT13
    for (int i = 4; i < dn; i++) {
        if (dec[i] >= 'a' && dec[i] <= 'z') dec[i] = (dec[i] - 'a' + 13) % 26 + 'a';
        else if (dec[i] >= 'A' && dec[i] <= 'Z') dec[i] = (dec[i] - 'A' + 13) % 26 + 'A';
    }
    fwrite(dec + 4, 1, dn - 4, stdout);
    return 0;
}"#
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": r#"import sys, base64
data = base64.b64decode(sys.stdin.read().strip())
data = data[::-1]  # reverse
key = data[:4]
data = bytes(b ^ key[i % 4] for i, b in enumerate(data[4:]))
result = ''.join(chr((ord(c) - ord('a') + 13) % 26 + ord('a')) if 'a' <= c <= 'z'
                 else chr((ord(c) - ord('A') + 13) % 26 + ord('A')) if 'A' <= c <= 'Z'
                 else c for c in data.decode())
print(result, end="")"#
        }
    ]);

    db::create_challenge(
        pool,
        "crypto-chain",
        "Crypto Chain",
        "Decode a message encrypted with multiple layers: Base64 → Reverse bytes → XOR (key from first 4 bytes) → ROT13. Apply them in order to reveal the flag.",
        "crypto",
        "hard",
        Some("Multi-layer encrypted blob"),
        "Decrypted flag",
        &crypto_chain_tests,
        "exact",
        false,
        None,
        Some(&crypto_chain_baselines),
    )
    .await?;

    // HTTP GET challenge (needs network)
    let http_tests = serde_json::json!([
        {
            "stdin": "",
            "expected_stdout": "ctf{http_fetcher}\n",
            "description": "Fetch flag from local HTTP server"
        }
    ]);

    let http_baselines = serde_json::json!([
        {
            "language": "c",
            "name": "C (musl)",
            "tier": "native",
            "source_code": r#"#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>
int main() {
    int s = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in a = {.sin_family = AF_INET, .sin_port = htons(8080), .sin_addr.s_addr = htonl(0x7f000001)};
    connect(s, (void*)&a, sizeof(a));
    write(s, "GET /flag HTTP/1.0\r\nHost: localhost\r\n\r\n", 39);
    char buf[4096]; int n = read(s, buf, sizeof(buf));
    close(s);
    char *body = strstr(buf, "\r\n\r\n");
    if (body) printf("%s", body + 4);
    return 0;
}"#
        },
        {
            "language": "go",
            "name": "Go",
            "tier": "native",
            "source_code": r#"package main
import ("fmt"; "io"; "net/http")
func main() {
    resp, _ := http.Get("http://127.0.0.1:8080/flag")
    body, _ := io.ReadAll(resp.Body)
    fmt.Print(string(body))
}"#
        },
        {
            "language": "python",
            "name": "Python (Nuitka)",
            "tier": "scripting",
            "source_code": "import urllib.request\nprint(urllib.request.urlopen('http://127.0.0.1:8080/flag').read().decode(), end='')"
        }
    ]);

    db::create_challenge(
        pool,
        "http-get",
        "HTTP GET",
        "Perform an HTTP GET request to http://127.0.0.1:8080/flag and print the response body. Implement HTTP/1.1 using raw TCP sockets.",
        "networking",
        "hard",
        None,
        "HTTP response body",
        &http_tests,
        "trimmed",
        true,  // Network enabled for HTTP
        None,
        Some(&http_baselines),
    )
    .await?;

    info!("Seeded 7 initial challenges");
    Ok(())
}
