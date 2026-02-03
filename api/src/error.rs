use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid field value: {0}")]
    InvalidField(String),

    #[error("Binary too large: {size} bytes (max {max})")]
    BinaryTooLarge { size: usize, max: usize },

    #[error("Instruction limit too high: {limit} (max {max})")]
    InstructionLimitTooHigh { limit: u64, max: u64 },

    #[error("Docker execution failed: {0}")]
    DockerError(String),

    #[error("Execution timed out after {0} seconds")]
    Timeout(u64),

    #[error("Server too busy, try again later")]
    TooManyRequests,

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Job result not ready yet")]
    JobNotReady,

    #[error("Rate limit exceeded, try again later")]
    RateLimited,

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Compilation failed: {0}")]
    CompileError(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Compile job not found: {0}")]
    CompileJobNotFound(String),

    #[error("Compile job result not ready yet")]
    CompileJobNotReady,

    #[error("Source code too large: {size} bytes (max {max})")]
    SourceTooLarge { size: usize, max: usize },

    #[error("Invalid language: {0}")]
    InvalidLanguage(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),

    #[error("Submission not found: {0}")]
    SubmissionNotFound(String),

    #[error("Challenge verification failed: {0}")]
    VerificationFailed(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::MissingField(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InvalidField(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::BinaryTooLarge { .. } => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            ApiError::InstructionLimitTooHigh { .. } => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            ApiError::DockerError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, self.to_string()),
            ApiError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            ApiError::JobNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::JobNotReady => (StatusCode::ACCEPTED, self.to_string()),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            ApiError::QueueError(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ApiError::CompileError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::BinaryNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::CompileJobNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::CompileJobNotReady => (StatusCode::ACCEPTED, self.to_string()),
            ApiError::SourceTooLarge { .. } => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            ApiError::InvalidLanguage(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            ApiError::ChallengeNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::SubmissionNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::VerificationFailed(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
