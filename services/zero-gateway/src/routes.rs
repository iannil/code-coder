//! Route definitions for Zero Gateway.
//!
//! Provides HTTP endpoints for authentication, user management, proxy, quota, and health checks.

use crate::auth::{auth_middleware, AuthState, AuthUser};
use crate::metering::{MeteringState, UsageReport};
use crate::parallel::{ParallelState, parallel_routes};
use crate::provider::create_registry;
use crate::proxy::{proxy_request, ProxyState};
use crate::quota::QuotaLimits;
use crate::sandbox::{AuditAction, AuditEntry, DayCount, Sandbox, SandboxConfig};
use crate::user::{CreateUserRequest, UpdateUserRequest, User, UserStore};
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{any, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use zero_common::config::Config;
use zero_common::security::rbac::{check_permission, Permission};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub auth: AuthState,
    pub user_store: Arc<UserStore>,
    pub sandbox: Arc<Sandbox>,
    pub metering: MeteringState,
}

/// Login request body.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// Error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Health check response.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

/// User info response (for /auth/me).
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub user_id: String,
    pub roles: Vec<String>,
}

/// User response (sanitized user data).
#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            roles: user.roles,
            enabled: user.enabled,
            email: user.email,
            display_name: user.display_name,
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
            last_login_at: user.last_login_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// List users response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserResponse>,
    pub total: u64,
}

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Get the database path for user storage.
fn get_db_path() -> PathBuf {
    let config_dir = zero_common::config::config_dir();
    config_dir.join("gateway.db")
}

/// Build the complete router with all routes.
pub fn build_all_routes(config: &Config) -> Router {
    build_all_routes_with_db(config, None)
}

/// Build the complete router with all routes, using a custom database path.
/// This is useful for testing with isolated databases.
pub fn build_all_routes_with_db(config: &Config, db_path: Option<PathBuf>) -> Router {
    // Auth settings are now in config.auth
    let jwt_secret = config
        .auth
        .jwt_secret
        .clone()
        .or_else(|| std::env::var("JWT_SECRET").ok())
        .unwrap_or_else(|| "zero-gateway-default-secret-change-me!".to_string());

    let auth_state = AuthState::new(&jwt_secret, config.auth.token_expiry_secs);

    // Initialize user store
    let db_path = db_path.unwrap_or_else(get_db_path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let user_store = Arc::new(
        UserStore::new(&db_path).expect("Failed to initialize user store"),
    );

    // Initialize sandbox
    let sandbox = Arc::new(Sandbox::new(SandboxConfig::default()));

    // Initialize metering
    let metering = MeteringState::new().expect("Failed to initialize metering");

    let app_state = AppState {
        auth: auth_state.clone(),
        user_store,
        sandbox,
        metering: metering.clone(),
    };

    let proxy_state = ProxyState::new(&config.codecoder_endpoint());

    // Build proxy routes with metering middleware
    let proxy_routes = Router::new()
        .route("/api/v1/proxy/*path", any(proxy_request))
        .layer(middleware::from_fn_with_state(
            metering.clone(),
            crate::metering::metering_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ))
        .with_state(proxy_state);

    // Build app routes (they need AppState)
    let app_routes = Router::new()
        // Auth routes (public)
        .route("/api/v1/auth/login", post(login_handler))
        // Auth routes (authenticated)
        .route(
            "/api/v1/auth/refresh",
            post(refresh_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/auth/me",
            get(me_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        // User management routes (authenticated)
        .route(
            "/api/v1/users",
            get(list_users_handler)
                .post(create_user_handler)
                .layer(middleware::from_fn_with_state(
                    auth_state.clone(),
                    auth_middleware,
                )),
        )
        .route(
            "/api/v1/users/:id",
            get(get_user_handler)
                .put(update_user_handler)
                .delete(delete_user_handler)
                .layer(middleware::from_fn_with_state(
                    auth_state.clone(),
                    auth_middleware,
                )),
        )
        // Quota routes (authenticated)
        .route(
            "/api/v1/quota",
            get(get_my_quota_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/quota/:user_id",
            get(get_user_quota_handler)
                .put(set_user_quota_handler)
                .layer(middleware::from_fn_with_state(
                    auth_state.clone(),
                    auth_middleware,
                )),
        )
        // Audit routes (authenticated, requires AuditRead permission)
        .route(
            "/api/v1/audit",
            get(list_audit_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/audit/summary",
            get(get_audit_summary_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/audit/:id",
            get(get_audit_entry_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/audit/user/:user_id",
            get(get_user_audit_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .with_state(app_state);

    // Merge all routes
    // Create provider registry for parallel inference
    // Use config.get_api_key() which reads from secrets.llm with env fallback
    let anthropic_key = config.get_api_key("anthropic");
    let openai_key = config.get_api_key("openai");

    let registry = create_registry(anthropic_key.as_deref(), openai_key.as_deref());
    let parallel_state = ParallelState::new(registry);

    // Build parallel routes with authentication
    let parallel_router = parallel_routes(parallel_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(app_routes)
        .merge(proxy_routes)
        .merge(parallel_router)
        .merge(health_routes())
}

/// Build authentication routes (legacy, for backward compatibility).
pub fn auth_routes() -> Router {
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "zero-gateway-default-secret-change-me!".to_string());
    let auth_state = AuthState::new(jwt_secret, 86400);

    // Initialize user store
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let user_store = Arc::new(
        UserStore::new(&db_path).expect("Failed to initialize user store"),
    );

    let sandbox = Arc::new(Sandbox::new(SandboxConfig::default()));
    let metering = MeteringState::new().expect("Failed to initialize metering");

    let app_state = AppState {
        auth: auth_state.clone(),
        user_store,
        sandbox,
        metering,
    };

    Router::new()
        .route("/api/v1/auth/login", post(login_handler))
        .route(
            "/api/v1/auth/refresh",
            post(refresh_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .route(
            "/api/v1/auth/me",
            get(me_handler).layer(middleware::from_fn_with_state(
                auth_state.clone(),
                auth_middleware,
            )),
        )
        .with_state(app_state)
}

/// Build proxy routes (legacy, for backward compatibility).
pub fn proxy_routes(config: &Config) -> Router {
    let proxy_state = ProxyState::new(&config.codecoder_endpoint());
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "zero-gateway-default-secret-change-me!".to_string());
    let auth_state = AuthState::new(jwt_secret, config.auth.token_expiry_secs);

    Router::new()
        .route("/api/v1/proxy/*path", any(proxy_request))
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .with_state(proxy_state)
}

/// Build health check routes.
pub fn health_routes() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Login handler with proper password verification.
async fn login_handler(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate input
    if request.username.is_empty() || request.password.is_empty() {
        // Audit failed login attempt
        let entry = state.sandbox.create_audit_entry(
            None,
            None,
            AuditAction::Login {
                username: request.username.clone(),
                success: false,
            },
            None,
        );
        state.sandbox.audit(entry);

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".into(),
                code: "AUTH_INVALID_CREDENTIALS".into(),
            }),
        ));
    }

    // Verify credentials
    let user = state
        .user_store
        .verify_password(&request.username, &request.password)
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Authentication error".into(),
                    code: "AUTH_ERROR".into(),
                }),
            )
        })?;

    let Some(user) = user else {
        // Audit failed login
        let entry = state.sandbox.create_audit_entry(
            None,
            None,
            AuditAction::Login {
                username: request.username.clone(),
                success: false,
            },
            None,
        );
        state.sandbox.audit(entry);

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid credentials".into(),
                code: "AUTH_INVALID_CREDENTIALS".into(),
            }),
        ));
    };

    // Generate token
    let token = state
        .auth
        .generate_token(&user.id, user.roles.clone())
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to generate token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate token".into(),
                    code: "AUTH_TOKEN_ERROR".into(),
                }),
            )
        })?;

    // Audit successful login
    let entry = state.sandbox.create_audit_entry(
        Some(user.id.clone()),
        None,
        AuditAction::Login {
            username: user.username.clone(),
            success: true,
        },
        None,
    );
    state.sandbox.audit(entry);

    Ok(Json(LoginResponse {
        token,
        expires_in: state.auth.token_expiry_secs,
        user: user.into(),
    }))
}

/// Token refresh handler.
async fn refresh_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get fresh user data
    let user = state
        .user_store
        .get(&auth_user.user_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to refresh token".into(),
                    code: "AUTH_TOKEN_ERROR".into(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "User not found".into(),
                    code: "AUTH_USER_NOT_FOUND".into(),
                }),
            )
        })?;

    if !user.enabled {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "User is disabled".into(),
                code: "AUTH_USER_DISABLED".into(),
            }),
        ));
    }

    let token = state
        .auth
        .generate_token(&user.id, user.roles.clone())
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to refresh token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to refresh token".into(),
                    code: "AUTH_TOKEN_ERROR".into(),
                }),
            )
        })?;

    Ok(Json(LoginResponse {
        token,
        expires_in: state.auth.token_expiry_secs,
        user: user.into(),
    }))
}

/// Get current user info.
async fn me_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = state
        .user_store
        .get(&auth_user.user_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get user".into(),
                    code: "USER_ERROR".into(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User not found".into(),
                    code: "USER_NOT_FOUND".into(),
                }),
            )
        })?;

    Ok(Json(user.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// User Management Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// List all users (requires UserRead permission).
async fn list_users_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<ListUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::UserRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let users = state
        .user_store
        .list(pagination.limit, pagination.offset)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list users");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list users".into(),
                    code: "USER_ERROR".into(),
                }),
            )
        })?;

    let total = state.user_store.count().unwrap_or(0);

    Ok(Json(ListUsersResponse {
        users: users.into_iter().map(Into::into).collect(),
        total,
    }))
}

/// Create a new user (requires UserWrite permission).
async fn create_user_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::UserWrite) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let user = state.user_store.create(&request).map_err(|e| {
        let error_msg = e.to_string();
        tracing::error!(error = %e, "Failed to create user");

        // Check for duplicate username
        if error_msg.contains("UNIQUE constraint") {
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "Username already exists".into(),
                    code: "USER_EXISTS".into(),
                }),
            );
        }

        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: error_msg,
                code: "USER_CREATE_ERROR".into(),
            }),
        )
    })?;

    // Audit user creation
    let entry = state.sandbox.create_audit_entry(
        Some(auth_user.user_id.clone()),
        None,
        AuditAction::UserAction {
            action: "create".into(),
            target_user_id: user.id.clone(),
        },
        None,
    );
    state.sandbox.audit(entry);

    Ok((StatusCode::CREATED, Json(user.into())))
}

/// Get a user by ID (requires UserRead permission, or own user).
async fn get_user_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Allow users to read their own profile
    let is_own_profile = auth_user.user_id == id;

    if !is_own_profile && !check_permission(&auth_user.roles, Permission::UserRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let user = state
        .user_store
        .get(&id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get user".into(),
                    code: "USER_ERROR".into(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User not found".into(),
                    code: "USER_NOT_FOUND".into(),
                }),
            )
        })?;

    Ok(Json(user.into()))
}

/// Update a user (requires UserWrite permission, or own profile for limited updates).
async fn update_user_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let is_own_profile = auth_user.user_id == id;
    let has_write_permission = check_permission(&auth_user.roles, Permission::UserWrite);

    // Check permission
    if !is_own_profile && !has_write_permission {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    // Only admins can change roles or enabled status
    if !has_write_permission && (request.roles.is_some() || request.enabled.is_some()) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cannot modify roles or enabled status".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let user = state
        .user_store
        .update(&id, &request)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update user");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "USER_UPDATE_ERROR".into(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User not found".into(),
                    code: "USER_NOT_FOUND".into(),
                }),
            )
        })?;

    // Audit user update
    let entry = state.sandbox.create_audit_entry(
        Some(auth_user.user_id.clone()),
        None,
        AuditAction::UserAction {
            action: "update".into(),
            target_user_id: user.id.clone(),
        },
        None,
    );
    state.sandbox.audit(entry);

    Ok(Json(user.into()))
}

/// Delete a user (requires UserAdmin permission).
async fn delete_user_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check permission (need admin to delete)
    if !check_permission(&auth_user.roles, Permission::UserAdmin) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    // Prevent self-deletion
    if auth_user.user_id == id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Cannot delete your own account".into(),
                code: "SELF_DELETE".into(),
            }),
        ));
    }

    let deleted = state.user_store.delete(&id).map_err(|e| {
        tracing::error!(error = %e, "Failed to delete user");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to delete user".into(),
                code: "USER_DELETE_ERROR".into(),
            }),
        )
    })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".into(),
                code: "USER_NOT_FOUND".into(),
            }),
        ));
    }

    // Audit user deletion
    let entry = state.sandbox.create_audit_entry(
        Some(auth_user.user_id.clone()),
        None,
        AuditAction::UserAction {
            action: "delete".into(),
            target_user_id: id,
        },
        None,
    );
    state.sandbox.audit(entry);

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Quota Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Quota response for API.
#[derive(Debug, Serialize)]
pub struct QuotaResponse {
    pub user_id: String,
    pub usage: UsageReport,
    pub limits: QuotaLimitsResponse,
}

#[derive(Debug, Serialize)]
pub struct QuotaLimitsResponse {
    pub daily_input_tokens: i64,
    pub daily_output_tokens: i64,
    pub daily_requests: i64,
    pub monthly_input_tokens: i64,
    pub monthly_output_tokens: i64,
}

impl From<QuotaLimits> for QuotaLimitsResponse {
    fn from(limits: QuotaLimits) -> Self {
        Self {
            daily_input_tokens: limits.daily_input_tokens,
            daily_output_tokens: limits.daily_output_tokens,
            daily_requests: limits.daily_requests,
            monthly_input_tokens: limits.monthly_input_tokens,
            monthly_output_tokens: limits.monthly_output_tokens,
        }
    }
}

/// Get current user's quota usage.
async fn get_my_quota_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<QuotaResponse>, (StatusCode, Json<ErrorResponse>)> {
    let usage = state
        .metering
        .get_usage_report(&auth_user.user_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get usage report");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get quota".into(),
                    code: "QUOTA_ERROR".into(),
                }),
            )
        })?;

    let limits = state
        .metering
        .quota_manager
        .get_limits(&auth_user.user_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get quota limits");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get quota limits".into(),
                    code: "QUOTA_ERROR".into(),
                }),
            )
        })?;

    Ok(Json(QuotaResponse {
        user_id: auth_user.user_id,
        usage,
        limits: limits.into(),
    }))
}

/// Get a specific user's quota usage (requires admin permission).
async fn get_user_quota_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
) -> Result<Json<QuotaResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Allow users to check their own quota, or admins to check any user
    let is_own_quota = auth_user.user_id == user_id;
    if !is_own_quota && !check_permission(&auth_user.roles, Permission::UserRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let usage = state.metering.get_usage_report(&user_id).map_err(|e| {
        tracing::error!(error = %e, "Failed to get usage report");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to get quota".into(),
                code: "QUOTA_ERROR".into(),
            }),
        )
    })?;

    let limits = state
        .metering
        .quota_manager
        .get_limits(&user_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get quota limits");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get quota limits".into(),
                    code: "QUOTA_ERROR".into(),
                }),
            )
        })?;

    Ok(Json(QuotaResponse {
        user_id,
        usage,
        limits: limits.into(),
    }))
}

/// Set a user's quota limits (requires admin permission).
#[derive(Debug, Deserialize)]
pub struct SetQuotaRequest {
    pub daily_input_tokens: Option<i64>,
    pub daily_output_tokens: Option<i64>,
    pub daily_requests: Option<i64>,
    pub monthly_input_tokens: Option<i64>,
    pub monthly_output_tokens: Option<i64>,
}

async fn set_user_quota_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
    Json(request): Json<SetQuotaRequest>,
) -> Result<Json<QuotaLimitsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only admins can set quota limits
    if !check_permission(&auth_user.roles, Permission::UserAdmin) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    // Get current limits and merge with request
    let current = state
        .metering
        .quota_manager
        .get_limits(&user_id)
        .unwrap_or_default();

    let new_limits = QuotaLimits {
        daily_input_tokens: request.daily_input_tokens.unwrap_or(current.daily_input_tokens),
        daily_output_tokens: request.daily_output_tokens.unwrap_or(current.daily_output_tokens),
        daily_requests: request.daily_requests.unwrap_or(current.daily_requests),
        monthly_input_tokens: request.monthly_input_tokens.unwrap_or(current.monthly_input_tokens),
        monthly_output_tokens: request
            .monthly_output_tokens
            .unwrap_or(current.monthly_output_tokens),
    };

    state
        .metering
        .quota_manager
        .set_limits(&user_id, &new_limits)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to set quota limits");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to set quota limits".into(),
                    code: "QUOTA_ERROR".into(),
                }),
            )
        })?;

    tracing::info!(
        user_id = %user_id,
        admin = %auth_user.user_id,
        "Quota limits updated"
    );

    Ok(Json(new_limits.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Audit Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Audit query parameters.
#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_audit_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub action_type: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
}

fn default_audit_limit() -> u32 {
    50
}

/// Audit list response.
#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub entries: Vec<AuditEntry>,
    pub total: u64,
    pub has_more: bool,
}

/// Audit summary response.
#[derive(Debug, Serialize)]
pub struct AuditSummaryResponse {
    pub total_entries: u64,
    pub by_action_type: std::collections::HashMap<String, u64>,
    pub by_day: Vec<DayCount>,
    pub recent_blocked: Vec<AuditEntry>,
}

/// List audit entries with pagination and filters.
async fn list_audit_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AuditListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::AuditRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    // Parse time filters
    let start_time = query.start_time.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let end_time = query.end_time.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let (entries, total) = state.sandbox.get_audit_paginated(
        query.limit as usize,
        query.offset as usize,
        query.action_type.as_deref(),
        query.user_id.as_deref(),
        start_time,
        end_time,
    );

    let has_more = (query.offset as u64 + entries.len() as u64) < total;

    Ok(Json(AuditListResponse {
        entries,
        total,
        has_more,
    }))
}

/// Get a single audit entry by ID.
async fn get_audit_entry_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<AuditEntry>, (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::AuditRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    state
        .sandbox
        .get_audit_by_id(&id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Audit entry not found".into(),
                    code: "AUDIT_NOT_FOUND".into(),
                }),
            )
        })
        .map(Json)
}

/// Get audit entries for a specific user.
async fn get_user_audit_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<AuditListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::AuditRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let limit = pagination.limit.unwrap_or(50) as usize;
    let offset = pagination.offset.unwrap_or(0) as usize;

    let (entries, total) = state.sandbox.get_audit_paginated(
        limit,
        offset,
        None,
        Some(&user_id),
        None,
        None,
    );

    let has_more = (offset as u64 + entries.len() as u64) < total;

    Ok(Json(AuditListResponse {
        entries,
        total,
        has_more,
    }))
}

/// Get audit summary statistics.
async fn get_audit_summary_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<AuditSummaryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check permission
    if !check_permission(&auth_user.roles, Permission::AuditRead) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Insufficient permissions".into(),
                code: "FORBIDDEN".into(),
            }),
        ));
    }

    let summary = state.sandbox.get_audit_summary();

    Ok(Json(AuditSummaryResponse {
        total_entries: summary.total_entries,
        by_action_type: summary.by_action_type,
        by_day: summary.by_day,
        recent_blocked: summary.recent_blocked,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Health Handler
// ─────────────────────────────────────────────────────────────────────────────

/// Health check handler.
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        service: "zero-gateway".into(),
    })
}
