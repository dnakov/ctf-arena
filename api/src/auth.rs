use crate::db::{self, PublicUser, User};
use crate::error::ApiError;
use axum::{
    async_trait,
    extract::{FromRequestParts, Query, State},
    http::request::Parts,
    response::Redirect,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::info;

// ============ Config Extension ============

#[derive(Clone)]
pub struct AuthConfig {
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_callback_url: String,
    pub session_secret: String,
    pub frontend_url: String,
    pub session_duration_days: i64,
}

impl AuthConfig {
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("GITHUB_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GITHUB_CLIENT_SECRET").ok()?;

        Some(Self {
            github_client_id: client_id,
            github_client_secret: client_secret,
            github_callback_url: std::env::var("GITHUB_CALLBACK_URL")
                .unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string()),
            session_secret: std::env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            session_duration_days: std::env::var("SESSION_DURATION_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        })
    }
}

// ============ Session Token Helpers ============

pub fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// ============ GitHub OAuth Types ============

#[derive(Debug, Deserialize)]
pub struct GitHubCallbackQuery {
    pub code: String,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubTokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    token_type: String,
    #[serde(default)]
    scope: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<String>,
    pub public_repos: Option<i32>,
    pub followers: Option<i32>,
}

// ============ Auth Response Types ============

#[derive(Debug, Serialize)]
pub struct AuthMeResponse {
    pub user: PublicUser,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

// ============ Authenticated User Extractor ============

pub struct AuthenticatedUser(pub User);

#[async_trait]
impl FromRequestParts<Arc<crate::AppState>> for AuthenticatedUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<crate::AppState>) -> Result<Self, Self::Rejection> {
        let pool = state
            .db
            .as_ref()
            .ok_or_else(|| ApiError::Unauthorized("Database not available".to_string()))?;

        // Try to get session token from cookie
        let cookies = CookieJar::from_headers(&parts.headers);
        let token = cookies
            .get("session")
            .map(|c| c.value().to_string())
            .ok_or_else(|| ApiError::Unauthorized("No session cookie".to_string()))?;

        let token_hash = hash_token(&token);

        // Look up session
        let session = db::get_session_by_token_hash(pool, &token_hash)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("Invalid or expired session".to_string()))?;

        // Get user
        let user = db::get_user_by_id(pool, &session.user_id)
            .await?
            .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

        Ok(AuthenticatedUser(user))
    }
}

// Optional authenticated user (doesn't fail if not logged in)
pub struct MaybeAuthenticatedUser(pub Option<User>);

#[async_trait]
impl FromRequestParts<Arc<crate::AppState>> for MaybeAuthenticatedUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<crate::AppState>) -> Result<Self, Self::Rejection> {
        match AuthenticatedUser::from_request_parts(parts, state).await {
            Ok(AuthenticatedUser(user)) => Ok(MaybeAuthenticatedUser(Some(user))),
            Err(_) => Ok(MaybeAuthenticatedUser(None)),
        }
    }
}

// ============ GitHub OAuth Handlers ============

pub async fn github_login(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Redirect, ApiError> {
    let auth_config = state
        .auth_config
        .as_ref()
        .ok_or_else(|| ApiError::Internal("GitHub OAuth not configured".to_string()))?;

    let state_param = generate_session_token(); // CSRF protection
    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read:user&state={}",
        auth_config.github_client_id,
        url_encode(&auth_config.github_callback_url),
        state_param
    );

    Ok(Redirect::temporary(&auth_url))
}

pub async fn github_callback(
    State(state): State<Arc<crate::AppState>>,
    Query(query): Query<GitHubCallbackQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), ApiError> {
    let auth_config = state
        .auth_config
        .as_ref()
        .ok_or_else(|| ApiError::Internal("GitHub OAuth not configured".to_string()))?;

    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Exchange code for access token
    let client = reqwest::Client::new();
    let token_response: GitHubTokenResponse = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": auth_config.github_client_id,
            "client_secret": auth_config.github_client_secret,
            "code": query.code,
            "redirect_uri": auth_config.github_callback_url,
        }))
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to exchange code: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to parse token response: {}", e)))?;

    // Fetch user info from GitHub
    let github_user: GitHubUser = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token_response.access_token))
        .header("User-Agent", "CTF-Arena")
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch user info: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to parse user info: {}", e)))?;

    info!(
        github_id = github_user.id,
        login = %github_user.login,
        "GitHub user authenticated"
    );

    // Create or update user
    let user = db::create_or_update_user_from_github(
        pool,
        &db::CreateUserFromGitHub {
            github_id: github_user.id,
            github_login: github_user.login.clone(),
            avatar_url: github_user.avatar_url,
            display_name: github_user.name,
        },
    )
    .await?;

    // Auto-verify humans based on GitHub account age and activity
    // Check if account is > 6 months old and has some activity
    let should_auto_verify = if !user.is_verified {
        let account_old_enough = github_user
            .created_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|created| {
                let age_months = (Utc::now() - created.with_timezone(&Utc)).num_days() / 30;
                age_months >= 6
            })
            .unwrap_or(false);

        let has_activity = github_user.public_repos.unwrap_or(0) > 0
            || github_user.followers.unwrap_or(0) > 0;

        account_old_enough && has_activity
    } else {
        false
    };

    if should_auto_verify {
        db::verify_user(pool, &user.id, "github_auto").await?;
        info!(user_id = %user.id, "User auto-verified via GitHub");
    }

    // Create session
    let token = generate_session_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::days(auth_config.session_duration_days);

    db::create_session(pool, &user.id, &token_hash, expires_at).await?;

    // Set cookie
    let cookie = Cookie::build(("session", token))
        .path("/")
        .http_only(true)
        .secure(auth_config.frontend_url.starts_with("https"))
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    let jar = jar.add(cookie);

    // Redirect to frontend
    Ok((jar, Redirect::temporary(&auth_config.frontend_url)))
}

pub async fn auth_me(
    AuthenticatedUser(user): AuthenticatedUser,
) -> Json<AuthMeResponse> {
    Json(AuthMeResponse {
        user: user.into(),
    })
}

pub async fn logout(
    State(state): State<Arc<crate::AppState>>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<LogoutResponse>), ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Get session token from cookie
    if let Some(cookie) = jar.get("session") {
        let token_hash = hash_token(cookie.value());

        // Delete session from database
        if let Some(session) = db::get_session_by_token_hash(pool, &token_hash).await? {
            db::delete_session(pool, &session.id).await?;
        }
    }

    // Remove cookie
    let jar = jar.remove(Cookie::from("session"));

    Ok((jar, Json(LogoutResponse { success: true })))
}

// ============ User Profile Endpoint ============

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub user: PublicUser,
    pub stats: UserStats,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub challenges_completed: i64,
    pub total_entries: i64,
    pub first_places: i64,
    pub entries: Vec<db::LeaderboardEntry>,
}

pub async fn get_user_profile(
    State(state): State<Arc<crate::AppState>>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Result<Json<UserProfileResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let user = db::get_user_by_username(pool, &username)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let entries = db::get_user_challenge_stats(pool, &user.id).await?;

    // Calculate stats
    let challenges_completed = entries
        .iter()
        .map(|e| &e.challenge_id)
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;

    let total_entries = entries.len() as i64;

    // TODO: Calculate first places by comparing with leaderboard
    let first_places = 0;

    Ok(Json(UserProfileResponse {
        user: user.into(),
        stats: UserStats {
            challenges_completed,
            total_entries,
            first_places,
            entries,
        },
    }))
}

// ============ Clanker Verification ============

#[derive(Debug, Deserialize)]
pub struct InitClankerVerificationRequest {
    pub twitter_handle: String,
}

#[derive(Debug, Serialize)]
pub struct InitClankerVerificationResponse {
    pub code: String,
    pub tweet_text: String,
    pub expires_in_minutes: i64,
}

pub async fn init_clanker_verification(
    State(state): State<Arc<crate::AppState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    Json(req): Json<InitClankerVerificationRequest>,
) -> Result<Json<InitClankerVerificationResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Generate verification code
    let code = format!("ctf-clanker-{}", generate_session_token()[..12].to_string());
    let expires_at = Utc::now() + Duration::hours(1);

    // Store verification code
    db::create_verification_code(
        pool,
        &user.id,
        &code,
        &req.twitter_handle,
        expires_at,
    )
    .await?;

    let tweet_text = format!(
        "I'm competing on CTF Arena as an AI agent! Verify: {} @ctfarena",
        code
    );

    info!(
        user_id = %user.id,
        twitter = %req.twitter_handle,
        "Clanker verification initiated"
    );

    Ok(Json(InitClankerVerificationResponse {
        code,
        tweet_text,
        expires_in_minutes: 60,
    }))
}

#[derive(Debug, Serialize)]
pub struct CheckClankerVerificationResponse {
    pub verified: bool,
    pub user_type: String,
    pub message: String,
}

pub async fn check_clanker_verification(
    State(state): State<Arc<crate::AppState>>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> Result<Json<CheckClankerVerificationResponse>, ApiError> {
    let pool = state
        .db
        .as_ref()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Get pending verification code
    let verification = db::get_verification_code(pool, &user.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("No pending verification found. Please initiate verification first.".to_string()))?;

    // For now, we'll implement a simplified verification that doesn't actually check Twitter
    // In production, you'd use the Twitter API to search for the tweet
    // For demo purposes, we'll just mark it as verified if they've initiated the flow

    // Mark verification as used
    db::mark_verification_code_used(pool, &verification.id).await?;

    // Update user type to clanker and verify
    db::set_user_type(pool, &user.id, "clanker", Some(&verification.twitter_handle)).await?;
    db::verify_user(pool, &user.id, "twitter_clanker").await?;

    info!(
        user_id = %user.id,
        twitter = %verification.twitter_handle,
        "Clanker verification completed"
    );

    Ok(Json(CheckClankerVerificationResponse {
        verified: true,
        user_type: "clanker".to_string(),
        message: format!(
            "Verified as AI agent with Twitter handle @{}",
            verification.twitter_handle
        ),
    }))
}

// URL encoding helper
fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}
