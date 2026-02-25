# zero-browser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust browser automation service with CDP network monitoring to learn API patterns and support headless replay.

**Architecture:** Standalone service using chromiumoxide for CDP communication, axum for HTTP API, and zero-memory for pattern storage. Organized into browser/, network/, pattern/, and replay/ modules.

**Tech Stack:** Rust, chromiumoxide, axum 0.7, zero-memory, reqwest, tokio

---

## Task 1: Project Scaffolding

**Files:**
- Create: `services/zero-browser/Cargo.toml`
- Create: `services/zero-browser/src/lib.rs`
- Create: `services/zero-browser/src/main.rs`
- Modify: `services/Cargo.toml` (add to workspace members)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "zero-browser"
version = "0.1.0"
edition = "2021"
authors = ["theonlyhennygod"]
license = "MIT"
description = "Browser automation service with API learning capabilities"

[dependencies]
# CDP client
chromiumoxide = { version = "0.7", default-features = false, features = ["tokio-runtime"] }
futures = "0.3"

# Async runtime
tokio = { workspace = true }

# HTTP server
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }

# HTTP client (for replay)
reqwest = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
anyhow = { workspace = true }
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

# Time
chrono = { workspace = true }

# UUID
uuid = { workspace = true }

# Internal crates
zero-common = { workspace = true }
zero-memory = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = "0.4"
wiremock = "0.6"
```

**Step 2: Create minimal lib.rs**

```rust
//! zero-browser - Browser automation with API learning capabilities.

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod browser;
pub mod error;
pub mod network;
pub mod pattern;
pub mod replay;
pub mod routes;

pub use error::BrowserError;
pub use pattern::types::{ApiPattern, AuthPattern, HeaderPattern};
```

**Step 3: Create minimal main.rs**

```rust
//! zero-browser service entry point.

use anyhow::Result;
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    init_logging(&config.observability.log_level, &config.observability.log_format);

    tracing::info!("Zero Browser v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Service starting on port 4433");

    // TODO: Start HTTP server
    Ok(())
}
```

**Step 4: Update workspace Cargo.toml**

Add `"zero-browser"` to the members list in `services/Cargo.toml`.

**Step 5: Create directory structure**

```bash
mkdir -p services/zero-browser/src/{browser,network,pattern,replay}
mkdir -p services/zero-browser/tests
```

**Step 6: Verify it compiles**

Run: `cargo check -p zero-browser`
Expected: Compilation succeeds (with warnings about unused modules)

**Step 7: Commit**

```bash
git add services/zero-browser services/Cargo.toml
git commit -m "feat(zero-browser): scaffold project structure"
```

---

## Task 2: Data Types (pattern/types.rs)

**Files:**
- Create: `services/zero-browser/src/pattern/types.rs`
- Create: `services/zero-browser/src/pattern/mod.rs`

**Step 1: Write the test for ApiPattern serialization**

Create `services/zero-browser/src/pattern/types.rs`:

```rust
//! API pattern data types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Header value pattern.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HeaderPattern {
    /// Fixed header value.
    Fixed { value: String },
    /// Dynamic value extracted from context.
    Dynamic { source: String, key: String },
    /// Value comes from authentication.
    FromAuth,
}

/// Authentication pattern.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthPattern {
    /// Bearer token authentication.
    Bearer { token_source: String },
    /// Cookie-based authentication.
    Cookie { names: Vec<String> },
    /// API key in header.
    ApiKey { header: String, key_source: String },
}

/// Learned API pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPattern {
    /// Unique identifier: {host}:{method}:{path_pattern}
    pub id: String,
    /// Source host.
    pub host: String,
    /// HTTP method.
    pub method: String,
    /// Path pattern with placeholders (e.g., /users/{id}).
    pub path_pattern: String,
    /// Required headers.
    #[serde(default)]
    pub required_headers: HashMap<String, HeaderPattern>,
    /// Authentication pattern.
    pub auth: Option<AuthPattern>,
    /// Request body JSON schema.
    pub request_schema: Option<serde_json::Value>,
    /// Response body JSON schema.
    pub response_schema: Option<serde_json::Value>,
    /// When this pattern was learned.
    pub learned_at: DateTime<Utc>,
    /// Number of times this pattern was used.
    #[serde(default)]
    pub usage_count: u32,
    /// Last successful replay.
    pub last_success: Option<DateTime<Utc>>,
}

impl ApiPattern {
    /// Create a new API pattern.
    pub fn new(host: &str, method: &str, path_pattern: &str) -> Self {
        let id = format!("{}:{}:{}", host, method, path_pattern);
        Self {
            id,
            host: host.to_string(),
            method: method.to_string(),
            path_pattern: path_pattern.to_string(),
            required_headers: HashMap::new(),
            auth: None,
            request_schema: None,
            response_schema: None,
            learned_at: Utc::now(),
            usage_count: 0,
            last_success: None,
        }
    }

    /// Record a successful use of this pattern.
    pub fn record_success(&mut self) {
        self.usage_count += 1;
        self.last_success = Some(Utc::now());
    }
}

/// Raw network request snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSnapshot {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timestamp: DateTime<Utc>,
}

/// Raw network response snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSnapshot {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_pattern_new() {
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}");

        assert_eq!(pattern.id, "api.example.com:GET:/users/{id}");
        assert_eq!(pattern.host, "api.example.com");
        assert_eq!(pattern.method, "GET");
        assert_eq!(pattern.usage_count, 0);
    }

    #[test]
    fn test_api_pattern_record_success() {
        let mut pattern = ApiPattern::new("api.example.com", "GET", "/users");
        assert!(pattern.last_success.is_none());

        pattern.record_success();

        assert_eq!(pattern.usage_count, 1);
        assert!(pattern.last_success.is_some());
    }

    #[test]
    fn test_header_pattern_serialization() {
        let fixed = HeaderPattern::Fixed {
            value: "application/json".to_string(),
        };
        let json = serde_json::to_string(&fixed).unwrap();

        assert!(json.contains("fixed"));
        assert!(json.contains("application/json"));
    }

    #[test]
    fn test_auth_pattern_serialization() {
        let bearer = AuthPattern::Bearer {
            token_source: "env:API_TOKEN".to_string(),
        };
        let json = serde_json::to_string(&bearer).unwrap();

        assert!(json.contains("bearer"));
        assert!(json.contains("env:API_TOKEN"));
    }
}
```

**Step 2: Create pattern/mod.rs**

```rust
//! API pattern module.

pub mod types;

pub use types::{ApiPattern, AuthPattern, HeaderPattern, RequestSnapshot, ResponseSnapshot};
```

**Step 3: Run tests**

Run: `cargo test -p zero-browser pattern::types::tests`
Expected: All 4 tests pass

**Step 4: Commit**

```bash
git add services/zero-browser/src/pattern/
git commit -m "feat(zero-browser): add pattern data types with tests"
```

---

## Task 3: Error Types

**Files:**
- Create: `services/zero-browser/src/error.rs`

**Step 1: Create error types**

```rust
//! Error types for zero-browser.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Browser service errors.
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Browser launch failed: {0}")]
    LaunchFailed(String),

    #[error("Navigation timeout: {url}")]
    NavigationTimeout { url: String },

    #[error("Element not found: {selector}")]
    ElementNotFound { selector: String },

    #[error("Network interception failed: {0}")]
    NetworkError(String),

    #[error("Pattern not found: {0}")]
    PatternNotFound(String),

    #[error("Pattern extraction failed: {reason}")]
    PatternExtractionFailed { reason: String },

    #[error("Replay failed: {pattern_id} - {reason}")]
    ReplayFailed { pattern_id: String, reason: String },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// API error response.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl IntoResponse for BrowserError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            BrowserError::SessionNotFound(_) => (StatusCode::NOT_FOUND, "SESSION_NOT_FOUND"),
            BrowserError::PatternNotFound(_) => (StatusCode::NOT_FOUND, "PATTERN_NOT_FOUND"),
            BrowserError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST"),
            BrowserError::ElementNotFound { .. } => (StatusCode::NOT_FOUND, "ELEMENT_NOT_FOUND"),
            BrowserError::NavigationTimeout { .. } => (StatusCode::GATEWAY_TIMEOUT, "NAVIGATION_TIMEOUT"),
            BrowserError::LaunchFailed(_) => (StatusCode::SERVICE_UNAVAILABLE, "BROWSER_UNAVAILABLE"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = serde_json::json!({
            "success": false,
            "error": ApiError {
                code: code.to_string(),
                message: self.to_string(),
            }
        });

        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BrowserError::SessionNotFound("abc123".to_string());
        assert_eq!(err.to_string(), "Session not found: abc123");
    }

    #[test]
    fn test_error_into_response() {
        let err = BrowserError::InvalidRequest("missing url".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p zero-browser error::tests`
Expected: Both tests pass

**Step 3: Commit**

```bash
git add services/zero-browser/src/error.rs
git commit -m "feat(zero-browser): add error types with axum integration"
```

---

## Task 4: Browser Session Management

**Files:**
- Create: `services/zero-browser/src/browser/session.rs`
- Create: `services/zero-browser/src/browser/mod.rs`

**Step 1: Create session types and manager**

```rust
//! Browser session management.

use crate::error::BrowserError;
use crate::pattern::{RequestSnapshot, ResponseSnapshot};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_headless")]
    pub headless: bool,
    pub proxy: Option<String>,
    #[serde(default = "default_viewport")]
    pub viewport: Viewport,
}

fn default_headless() -> bool {
    true
}

fn default_viewport() -> Viewport {
    Viewport {
        width: 1280,
        height: 720,
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            headless: true,
            proxy: None,
            viewport: default_viewport(),
        }
    }
}

/// Viewport dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Learning,
    Expired,
    Closed,
}

/// Learning filter configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearnFilter {
    /// Only learn from these hosts.
    #[serde(default)]
    pub hosts: Vec<String>,
    /// Only learn paths starting with this prefix.
    pub path_prefix: Option<String>,
    /// Only learn these HTTP methods.
    #[serde(default)]
    pub methods: Vec<String>,
}

/// Browser session.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub config: SessionConfig,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub learn_filter: Option<LearnFilter>,
    pub captured_requests: Vec<(RequestSnapshot, Option<ResponseSnapshot>)>,
}

impl Session {
    /// Create a new session.
    pub fn new(config: SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            config,
            status: SessionStatus::Active,
            created_at: now,
            last_activity: now,
            learn_filter: None,
            captured_requests: Vec::new(),
        }
    }

    /// Check if session has expired.
    pub fn is_expired(&self, timeout_secs: i64) -> bool {
        let elapsed = Utc::now() - self.last_activity;
        elapsed > Duration::seconds(timeout_secs)
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Start learning mode.
    pub fn start_learning(&mut self, filter: LearnFilter) {
        self.status = SessionStatus::Learning;
        self.learn_filter = Some(filter);
        self.captured_requests.clear();
        self.touch();
    }

    /// Stop learning mode and return captured requests.
    pub fn stop_learning(&mut self) -> Vec<(RequestSnapshot, Option<ResponseSnapshot>)> {
        self.status = SessionStatus::Active;
        self.learn_filter = None;
        self.touch();
        std::mem::take(&mut self.captured_requests)
    }

    /// Check if a request should be captured based on filter.
    pub fn should_capture(&self, url: &str, method: &str) -> bool {
        let Some(ref filter) = self.learn_filter else {
            return false;
        };

        // Check host filter
        if !filter.hosts.is_empty() {
            let host_matches = filter.hosts.iter().any(|h| url.contains(h));
            if !host_matches {
                return false;
            }
        }

        // Check path prefix
        if let Some(ref prefix) = filter.path_prefix {
            if let Ok(parsed) = url::Url::parse(url) {
                if !parsed.path().starts_with(prefix) {
                    return false;
                }
            }
        }

        // Check method filter
        if !filter.methods.is_empty() && !filter.methods.contains(&method.to_uppercase()) {
            return false;
        }

        true
    }
}

/// Session manager.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    timeout_secs: i64,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(timeout_secs: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            timeout_secs,
        }
    }

    /// Create a new session.
    pub async fn create(&self, config: SessionConfig) -> String {
        let session = Session::new(config);
        let id = session.id.clone();
        self.sessions.write().await.insert(id.clone(), session);
        id
    }

    /// Get a session by ID.
    pub async fn get(&self, id: &str) -> Result<Session, BrowserError> {
        let sessions = self.sessions.read().await;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))
    }

    /// Update a session.
    pub async fn update<F>(&self, id: &str, f: F) -> Result<(), BrowserError>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))?;
        f(session);
        Ok(())
    }

    /// Close a session.
    pub async fn close(&self, id: &str) -> Result<(), BrowserError> {
        let mut sessions = self.sessions.write().await;
        sessions
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| BrowserError::SessionNotFound(id.to_string()))
    }

    /// Clean up expired sessions.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| !s.is_expired(self.timeout_secs));
        before - sessions.len()
    }
}

// Manual Clone for Session (needed because of captured_requests)
impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            config: self.config.clone(),
            status: self.status,
            created_at: self.created_at,
            last_activity: self.last_activity,
            learn_filter: self.learn_filter.clone(),
            captured_requests: self.captured_requests.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new(SessionConfig::default());
        assert!(!session.id.is_empty());
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn test_session_learning() {
        let mut session = Session::new(SessionConfig::default());

        session.start_learning(LearnFilter {
            hosts: vec!["api.example.com".to_string()],
            ..Default::default()
        });

        assert_eq!(session.status, SessionStatus::Learning);
        assert!(session.should_capture("https://api.example.com/users", "GET"));
        assert!(!session.should_capture("https://other.com/users", "GET"));
    }

    #[test]
    fn test_session_filter_methods() {
        let mut session = Session::new(SessionConfig::default());
        session.start_learning(LearnFilter {
            methods: vec!["POST".to_string(), "PUT".to_string()],
            ..Default::default()
        });

        assert!(session.should_capture("https://example.com/api", "POST"));
        assert!(!session.should_capture("https://example.com/api", "GET"));
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new(1800);

        let id = manager.create(SessionConfig::default()).await;
        assert!(!id.is_empty());

        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.id, id);

        manager.close(&id).await.unwrap();
        assert!(manager.get(&id).await.is_err());
    }
}
```

**Step 2: Create browser/mod.rs**

```rust
//! Browser control module.

pub mod session;

pub use session::{LearnFilter, Session, SessionConfig, SessionManager, SessionStatus, Viewport};
```

**Step 3: Add url dependency to Cargo.toml**

Add `url = { workspace = true }` to dependencies.

**Step 4: Run tests**

Run: `cargo test -p zero-browser browser::session::tests`
Expected: All 4 tests pass

**Step 5: Commit**

```bash
git add services/zero-browser/src/browser/ services/zero-browser/Cargo.toml
git commit -m "feat(zero-browser): add session management with learning filter"
```

---

## Task 5: Network Monitor Stubs

**Files:**
- Create: `services/zero-browser/src/network/monitor.rs`
- Create: `services/zero-browser/src/network/mod.rs`

**Step 1: Create network monitor with CDP integration**

```rust
//! Network monitoring via CDP.

use crate::browser::SessionManager;
use crate::error::BrowserError;
use crate::pattern::{RequestSnapshot, ResponseSnapshot};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network request entry being tracked.
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub request_id: String,
    pub snapshot: RequestSnapshot,
    pub start_time: std::time::Instant,
}

/// Network monitor for a browser session.
#[derive(Debug)]
pub struct NetworkMonitor {
    session_id: String,
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
    session_manager: SessionManager,
}

impl NetworkMonitor {
    /// Create a new network monitor.
    pub fn new(session_id: String, session_manager: SessionManager) -> Self {
        Self {
            session_id,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            session_manager,
        }
    }

    /// Handle a network request event from CDP.
    pub async fn on_request(
        &self,
        request_id: &str,
        url: &str,
        method: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<(), BrowserError> {
        // Check if we should capture this request
        let session = self.session_manager.get(&self.session_id).await?;
        if !session.should_capture(url, method) {
            return Ok(());
        }

        let snapshot = RequestSnapshot {
            url: url.to_string(),
            method: method.to_string(),
            headers,
            body,
            timestamp: Utc::now(),
        };

        let pending = PendingRequest {
            request_id: request_id.to_string(),
            snapshot,
            start_time: std::time::Instant::now(),
        };

        self.pending_requests
            .write()
            .await
            .insert(request_id.to_string(), pending);

        Ok(())
    }

    /// Handle a network response event from CDP.
    pub async fn on_response(
        &self,
        request_id: &str,
        status: u16,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<(), BrowserError> {
        let pending = {
            let mut pending_requests = self.pending_requests.write().await;
            pending_requests.remove(request_id)
        };

        let Some(pending) = pending else {
            // Request wasn't being tracked
            return Ok(());
        };

        let duration_ms = pending.start_time.elapsed().as_millis() as u64;

        let response = ResponseSnapshot {
            status,
            headers,
            body,
            duration_ms,
        };

        // Store in session
        self.session_manager
            .update(&self.session_id, |session| {
                session
                    .captured_requests
                    .push((pending.snapshot, Some(response)));
            })
            .await?;

        Ok(())
    }

    /// Get count of pending requests.
    pub async fn pending_count(&self) -> usize {
        self.pending_requests.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::{LearnFilter, SessionConfig};

    #[tokio::test]
    async fn test_network_monitor_captures_matching_requests() {
        let manager = SessionManager::new(1800);
        let session_id = manager.create(SessionConfig::default()).await;

        // Start learning
        manager
            .update(&session_id, |s| {
                s.start_learning(LearnFilter {
                    hosts: vec!["api.example.com".to_string()],
                    ..Default::default()
                });
            })
            .await
            .unwrap();

        let monitor = NetworkMonitor::new(session_id.clone(), manager.clone());

        // Send matching request
        monitor
            .on_request(
                "req1",
                "https://api.example.com/users",
                "GET",
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(monitor.pending_count().await, 1);

        // Send response
        monitor
            .on_response("req1", 200, HashMap::new(), None)
            .await
            .unwrap();

        assert_eq!(monitor.pending_count().await, 0);

        // Verify captured
        let session = manager.get(&session_id).await.unwrap();
        assert_eq!(session.captured_requests.len(), 1);
    }

    #[tokio::test]
    async fn test_network_monitor_ignores_non_matching() {
        let manager = SessionManager::new(1800);
        let session_id = manager.create(SessionConfig::default()).await;

        manager
            .update(&session_id, |s| {
                s.start_learning(LearnFilter {
                    hosts: vec!["api.example.com".to_string()],
                    ..Default::default()
                });
            })
            .await
            .unwrap();

        let monitor = NetworkMonitor::new(session_id.clone(), manager.clone());

        // Send non-matching request
        monitor
            .on_request(
                "req1",
                "https://other.com/users",
                "GET",
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        // Should not be tracked
        assert_eq!(monitor.pending_count().await, 0);
    }
}
```

**Step 2: Create network/mod.rs**

```rust
//! Network monitoring module.

pub mod monitor;

pub use monitor::NetworkMonitor;
```

**Step 3: Run tests**

Run: `cargo test -p zero-browser network::monitor::tests`
Expected: Both tests pass

**Step 4: Commit**

```bash
git add services/zero-browser/src/network/
git commit -m "feat(zero-browser): add network monitor for request capture"
```

---

## Task 6: Pattern Extractor

**Files:**
- Create: `services/zero-browser/src/pattern/extractor.rs`
- Modify: `services/zero-browser/src/pattern/mod.rs`

**Step 1: Create pattern extractor**

```rust
//! Extract API patterns from captured network traffic.

use crate::pattern::types::{ApiPattern, AuthPattern, HeaderPattern, RequestSnapshot, ResponseSnapshot};
use std::collections::HashMap;
use url::Url;

/// Extract patterns from captured requests.
pub fn extract_patterns(
    requests: &[(RequestSnapshot, Option<ResponseSnapshot>)],
) -> Vec<ApiPattern> {
    let mut patterns: HashMap<String, ApiPattern> = HashMap::new();

    for (request, response) in requests {
        let Some(pattern) = extract_single_pattern(request, response.as_ref()) else {
            continue;
        };

        // Merge with existing pattern or insert new
        patterns
            .entry(pattern.id.clone())
            .and_modify(|existing| {
                existing.usage_count += 1;
                // Could merge headers, schemas, etc.
            })
            .or_insert(pattern);
    }

    patterns.into_values().collect()
}

/// Extract a pattern from a single request/response pair.
fn extract_single_pattern(
    request: &RequestSnapshot,
    response: Option<&ResponseSnapshot>,
) -> Option<ApiPattern> {
    let url = Url::parse(&request.url).ok()?;
    let host = url.host_str()?;

    // Extract path pattern (replace numeric IDs with {id})
    let path_pattern = extract_path_pattern(url.path());

    let mut pattern = ApiPattern::new(host, &request.method, &path_pattern);

    // Extract authentication
    pattern.auth = extract_auth(&request.headers);

    // Extract required headers (skip common ones)
    pattern.required_headers = extract_required_headers(&request.headers);

    // Extract response schema if JSON
    if let Some(resp) = response {
        if let Some(ref body) = resp.body {
            if is_json_content_type(&resp.headers) {
                pattern.response_schema = extract_json_schema(body);
            }
        }
    }

    // Extract request schema if JSON
    if let Some(ref body) = request.body {
        if is_json_content_type(&request.headers) {
            pattern.request_schema = extract_json_schema(body);
        }
    }

    Some(pattern)
}

/// Convert path to pattern by replacing numeric segments with {id}.
fn extract_path_pattern(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.chars().all(|c| c.is_ascii_digit()) && !segment.is_empty() {
                "{id}".to_string()
            } else if is_uuid(segment) {
                "{id}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Check if a string looks like a UUID.
fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars()
            .enumerate()
            .all(|(i, c)| matches!((i, c), (8 | 13 | 18 | 23, '-') | (_, c) if c.is_ascii_hexdigit()))
}

/// Extract authentication pattern from headers.
fn extract_auth(headers: &HashMap<String, String>) -> Option<AuthPattern> {
    // Check for Bearer token
    if let Some(auth) = headers.get("authorization").or(headers.get("Authorization")) {
        if auth.to_lowercase().starts_with("bearer ") {
            return Some(AuthPattern::Bearer {
                token_source: "dynamic".to_string(),
            });
        }
    }

    // Check for API key headers
    for key in ["x-api-key", "api-key", "apikey"] {
        if headers.contains_key(key) {
            return Some(AuthPattern::ApiKey {
                header: key.to_string(),
                key_source: "dynamic".to_string(),
            });
        }
    }

    // Check for cookie auth
    if headers.contains_key("cookie") || headers.contains_key("Cookie") {
        return Some(AuthPattern::Cookie {
            names: vec!["session".to_string()],
        });
    }

    None
}

/// Extract required headers (excluding common ones).
fn extract_required_headers(headers: &HashMap<String, String>) -> HashMap<String, HeaderPattern> {
    let skip_headers = [
        "host",
        "user-agent",
        "accept",
        "accept-language",
        "accept-encoding",
        "connection",
        "cookie",
        "authorization",
        "content-length",
        "content-type",
        "origin",
        "referer",
        "sec-",
        "cache-control",
        "pragma",
    ];

    headers
        .iter()
        .filter(|(k, _)| {
            let lower = k.to_lowercase();
            !skip_headers.iter().any(|s| lower.starts_with(s))
        })
        .map(|(k, v)| {
            (
                k.clone(),
                HeaderPattern::Fixed {
                    value: v.clone(),
                },
            )
        })
        .collect()
}

/// Check if content type is JSON.
fn is_json_content_type(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .any(|(k, v)| k.to_lowercase() == "content-type" && v.contains("application/json"))
}

/// Extract a simple JSON schema from body bytes.
fn extract_json_schema(body: &[u8]) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    Some(infer_schema(&value))
}

/// Infer JSON schema from a value.
fn infer_schema(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({"type": "null"}),
        serde_json::Value::Bool(_) => serde_json::json!({"type": "boolean"}),
        serde_json::Value::Number(_) => serde_json::json!({"type": "number"}),
        serde_json::Value::String(_) => serde_json::json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            let items = arr.first().map(infer_schema).unwrap_or(serde_json::json!({}));
            serde_json::json!({"type": "array", "items": items})
        }
        serde_json::Value::Object(obj) => {
            let properties: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), infer_schema(v)))
                .collect();
            serde_json::json!({"type": "object", "properties": properties})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_path_pattern() {
        assert_eq!(extract_path_pattern("/users/123"), "/users/{id}");
        assert_eq!(extract_path_pattern("/api/v1/posts"), "/api/v1/posts");
        assert_eq!(
            extract_path_pattern("/users/123/posts/456"),
            "/users/{id}/posts/{id}"
        );
    }

    #[test]
    fn test_extract_auth_bearer() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer abc123".to_string());

        let auth = extract_auth(&headers);
        assert!(matches!(auth, Some(AuthPattern::Bearer { .. })));
    }

    #[test]
    fn test_extract_auth_api_key() {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "secret".to_string());

        let auth = extract_auth(&headers);
        assert!(matches!(auth, Some(AuthPattern::ApiKey { .. })));
    }

    #[test]
    fn test_extract_patterns() {
        let request = RequestSnapshot {
            url: "https://api.example.com/users/123".to_string(),
            method: "GET".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Authorization".to_string(), "Bearer token".to_string());
                h
            },
            body: None,
            timestamp: Utc::now(),
        };

        let response = ResponseSnapshot {
            status: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: Some(br#"{"id": 123, "name": "Alice"}"#.to_vec()),
            duration_ms: 50,
        };

        let patterns = extract_patterns(&[(request, Some(response))]);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].path_pattern, "/users/{id}");
        assert!(patterns[0].auth.is_some());
        assert!(patterns[0].response_schema.is_some());
    }

    #[test]
    fn test_infer_schema() {
        let value = serde_json::json!({"name": "Alice", "age": 30});
        let schema = infer_schema(&value);

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["properties"]["age"]["type"], "number");
    }
}
```

**Step 2: Update pattern/mod.rs**

```rust
//! API pattern module.

pub mod extractor;
pub mod types;

pub use extractor::extract_patterns;
pub use types::{ApiPattern, AuthPattern, HeaderPattern, RequestSnapshot, ResponseSnapshot};
```

**Step 3: Run tests**

Run: `cargo test -p zero-browser pattern::extractor::tests`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add services/zero-browser/src/pattern/
git commit -m "feat(zero-browser): add pattern extractor from network traffic"
```

---

## Task 7: API Replay Executor

**Files:**
- Create: `services/zero-browser/src/replay/executor.rs`
- Create: `services/zero-browser/src/replay/mod.rs`

**Step 1: Create replay executor**

```rust
//! API replay execution.

use crate::error::BrowserError;
use crate::pattern::{ApiPattern, AuthPattern, HeaderPattern};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Replay request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayParams {
    /// Pattern ID to replay.
    pub pattern_id: String,
    /// Path parameter substitutions.
    #[serde(default)]
    pub path_params: HashMap<String, String>,
    /// Query parameters.
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Request body.
    pub body: Option<serde_json::Value>,
    /// Authentication override.
    pub auth: Option<ReplayAuth>,
}

/// Authentication for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReplayAuth {
    Bearer { token: String },
    ApiKey { header: String, value: String },
    Cookie { value: String },
}

/// Replay response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
    pub duration_ms: u64,
}

/// API replay executor.
pub struct ReplayExecutor {
    client: Client,
}

impl ReplayExecutor {
    /// Create a new replay executor.
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Execute a replay request.
    pub async fn execute(
        &self,
        pattern: &ApiPattern,
        params: &ReplayParams,
    ) -> Result<ReplayResponse, BrowserError> {
        let start = std::time::Instant::now();

        // Build URL
        let url = self.build_url(pattern, params)?;

        // Build request
        let mut request = match pattern.method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            method => {
                return Err(BrowserError::ReplayFailed {
                    pattern_id: pattern.id.clone(),
                    reason: format!("Unsupported method: {}", method),
                })
            }
        };

        // Add headers from pattern
        for (name, header_pattern) in &pattern.required_headers {
            if let HeaderPattern::Fixed { value } = header_pattern {
                request = request.header(name, value);
            }
        }

        // Add authentication
        request = self.apply_auth(request, pattern, params)?;

        // Add body
        if let Some(ref body) = params.body {
            request = request.json(body);
        }

        // Execute
        let response = request.send().await.map_err(|e| BrowserError::ReplayFailed {
            pattern_id: pattern.id.clone(),
            reason: e.to_string(),
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();

        // Collect headers
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Parse body
        let body = response.json().await.unwrap_or(serde_json::Value::Null);

        Ok(ReplayResponse {
            status,
            headers,
            body,
            duration_ms,
        })
    }

    /// Build the full URL with path and query parameters.
    fn build_url(&self, pattern: &ApiPattern, params: &ReplayParams) -> Result<String, BrowserError> {
        // Substitute path parameters
        let mut path = pattern.path_pattern.clone();
        for (key, value) in &params.path_params {
            path = path.replace(&format!("{{{}}}", key), value);
        }

        // Check for unsubstituted placeholders
        if path.contains('{') {
            return Err(BrowserError::ReplayFailed {
                pattern_id: pattern.id.clone(),
                reason: format!("Unsubstituted path parameters in: {}", path),
            });
        }

        // Build full URL
        let mut url = format!("https://{}{}", pattern.host, path);

        // Add query parameters
        if !params.query_params.is_empty() {
            let query: String = params
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query);
        }

        Ok(url)
    }

    /// Apply authentication to the request.
    fn apply_auth(
        &self,
        mut request: reqwest::RequestBuilder,
        pattern: &ApiPattern,
        params: &ReplayParams,
    ) -> Result<reqwest::RequestBuilder, BrowserError> {
        // Use override auth if provided
        if let Some(ref auth) = params.auth {
            match auth {
                ReplayAuth::Bearer { token } => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                ReplayAuth::ApiKey { header, value } => {
                    request = request.header(header, value);
                }
                ReplayAuth::Cookie { value } => {
                    request = request.header("Cookie", value);
                }
            }
            return Ok(request);
        }

        // Use pattern auth (would need credential lookup in real implementation)
        if let Some(ref auth) = pattern.auth {
            match auth {
                AuthPattern::Bearer { token_source } => {
                    // In real implementation, resolve token_source
                    if token_source != "dynamic" {
                        return Err(BrowserError::ReplayFailed {
                            pattern_id: pattern.id.clone(),
                            reason: "Auth token required but not provided".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(request)
    }
}

impl Default for ReplayExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_simple() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            auth: None,
        };

        let url = executor.build_url(&pattern, &params).unwrap();
        assert_eq!(url, "https://api.example.com/users");
    }

    #[test]
    fn test_build_url_with_params() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}/posts/{post_id}");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: {
                let mut m = HashMap::new();
                m.insert("id".to_string(), "123".to_string());
                m.insert("post_id".to_string(), "456".to_string());
                m
            },
            query_params: {
                let mut m = HashMap::new();
                m.insert("page".to_string(), "1".to_string());
                m
            },
            body: None,
            auth: None,
        };

        let url = executor.build_url(&pattern, &params).unwrap();
        assert_eq!(url, "https://api.example.com/users/123/posts/456?page=1");
    }

    #[test]
    fn test_build_url_missing_param() {
        let executor = ReplayExecutor::new();
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}");
        let params = ReplayParams {
            pattern_id: pattern.id.clone(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            auth: None,
        };

        let result = executor.build_url(&pattern, &params);
        assert!(result.is_err());
    }
}
```

**Step 2: Create replay/mod.rs**

```rust
//! API replay module.

pub mod executor;

pub use executor::{ReplayAuth, ReplayExecutor, ReplayParams, ReplayResponse};
```

**Step 3: Run tests**

Run: `cargo test -p zero-browser replay::executor::tests`
Expected: All 3 tests pass

**Step 4: Commit**

```bash
git add services/zero-browser/src/replay/
git commit -m "feat(zero-browser): add API replay executor"
```

---

## Task 8: HTTP Routes

**Files:**
- Create: `services/zero-browser/src/routes.rs`

**Step 1: Create HTTP routes**

```rust
//! HTTP API routes.

use crate::browser::{LearnFilter, SessionConfig, SessionManager};
use crate::error::BrowserError;
use crate::pattern::{extract_patterns, ApiPattern};
use crate::replay::{ReplayExecutor, ReplayParams, ReplayResponse};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub session_manager: SessionManager,
    pub patterns: Arc<RwLock<HashMap<String, ApiPattern>>>,
    pub replay_executor: Arc<ReplayExecutor>,
}

impl AppState {
    pub fn new(session_timeout_secs: i64) -> Self {
        Self {
            session_manager: SessionManager::new(session_timeout_secs),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            replay_executor: Arc::new(ReplayExecutor::new()),
        }
    }
}

/// Build the application router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Session management
        .route("/browser/sessions", post(create_session))
        .route("/browser/sessions/:id", delete(close_session))
        // Learning
        .route("/browser/sessions/:id/learn/start", post(start_learning))
        .route("/browser/sessions/:id/learn/stop", post(stop_learning))
        // Pattern management
        .route("/patterns", get(list_patterns))
        .route("/patterns/:id", get(get_pattern))
        .route("/patterns/:id", delete(delete_pattern))
        // Replay
        .route("/replay", post(replay_api))
        .with_state(state)
}

// ============ Health Check ============

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "zero-browser",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// ============ Session Management ============

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    session_id: String,
}

async fn create_session(
    State(state): State<AppState>,
    Json(config): Json<SessionConfig>,
) -> impl IntoResponse {
    let session_id = state.session_manager.create(config).await;
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": CreateSessionResponse { session_id }
        })),
    )
}

async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    state.session_manager.close(&id).await?;
    Ok(Json(serde_json::json!({
        "success": true
    })))
}

// ============ Learning ============

#[derive(Debug, Deserialize)]
struct StartLearningRequest {
    filter: LearnFilter,
}

async fn start_learning(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<StartLearningRequest>,
) -> Result<impl IntoResponse, BrowserError> {
    state
        .session_manager
        .update(&id, |session| {
            session.start_learning(request.filter);
        })
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "status": "learning"
        }
    })))
}

#[derive(Debug, Serialize)]
struct StopLearningResponse {
    patterns: Vec<ApiPattern>,
    request_count: usize,
    unique_endpoints: usize,
}

async fn stop_learning(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    // Get captured requests
    let captured = {
        let mut captured = Vec::new();
        state
            .session_manager
            .update(&id, |session| {
                captured = session.stop_learning();
            })
            .await?;
        captured
    };

    let request_count = captured.len();

    // Extract patterns
    let patterns = extract_patterns(&captured);
    let unique_endpoints = patterns.len();

    // Store patterns
    {
        let mut stored = state.patterns.write().await;
        for pattern in &patterns {
            stored.insert(pattern.id.clone(), pattern.clone());
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": StopLearningResponse {
            patterns,
            request_count,
            unique_endpoints,
        }
    })))
}

// ============ Pattern Management ============

#[derive(Debug, Deserialize)]
struct ListPatternsQuery {
    host: Option<String>,
    method: Option<String>,
}

async fn list_patterns(
    State(state): State<AppState>,
    Query(query): Query<ListPatternsQuery>,
) -> impl IntoResponse {
    let patterns = state.patterns.read().await;

    let filtered: Vec<&ApiPattern> = patterns
        .values()
        .filter(|p| {
            query.host.as_ref().map_or(true, |h| &p.host == h)
                && query.method.as_ref().map_or(true, |m| &p.method == m)
        })
        .collect();

    Json(serde_json::json!({
        "success": true,
        "data": {
            "patterns": filtered,
            "count": filtered.len()
        }
    }))
}

async fn get_pattern(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    let patterns = state.patterns.read().await;
    let pattern = patterns
        .get(&id)
        .ok_or_else(|| BrowserError::PatternNotFound(id))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": pattern
    })))
}

async fn delete_pattern(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, BrowserError> {
    let mut patterns = state.patterns.write().await;
    patterns
        .remove(&id)
        .ok_or_else(|| BrowserError::PatternNotFound(id.clone()))?;

    Ok(Json(serde_json::json!({
        "success": true
    })))
}

// ============ Replay ============

async fn replay_api(
    State(state): State<AppState>,
    Json(params): Json<ReplayParams>,
) -> Result<impl IntoResponse, BrowserError> {
    let patterns = state.patterns.read().await;
    let pattern = patterns
        .get(&params.pattern_id)
        .ok_or_else(|| BrowserError::PatternNotFound(params.pattern_id.clone()))?
        .clone();
    drop(patterns);

    let response = state.replay_executor.execute(&pattern, &params).await?;

    // Update usage count
    {
        let mut patterns = state.patterns.write().await;
        if let Some(p) = patterns.get_mut(&params.pattern_id) {
            p.record_success();
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> Router {
        build_router(AppState::new(1800))
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = test_app();

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_session() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/browser/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"headless": true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_list_patterns_empty() {
        let app = test_app();

        let response = app
            .oneshot(Request::builder().uri("/patterns").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p zero-browser routes::tests`
Expected: All 3 tests pass

**Step 3: Commit**

```bash
git add services/zero-browser/src/routes.rs
git commit -m "feat(zero-browser): add HTTP API routes"
```

---

## Task 9: Complete Main Entry Point

**Files:**
- Modify: `services/zero-browser/src/main.rs`
- Modify: `services/zero-browser/src/lib.rs`

**Step 1: Update lib.rs**

```rust
//! zero-browser - Browser automation with API learning capabilities.

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod browser;
pub mod error;
pub mod network;
pub mod pattern;
pub mod replay;
pub mod routes;

pub use browser::{LearnFilter, Session, SessionConfig, SessionManager};
pub use error::BrowserError;
pub use network::NetworkMonitor;
pub use pattern::{extract_patterns, ApiPattern, AuthPattern, HeaderPattern};
pub use replay::{ReplayExecutor, ReplayParams, ReplayResponse};
pub use routes::{build_router, AppState};
```

**Step 2: Update main.rs**

```rust
//! zero-browser service entry point.

use anyhow::Result;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use zero_browser::{build_router, AppState};
use zero_common::config::Config;
use zero_common::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    let startup_start = std::time::Instant::now();

    let config = Config::load()?;
    init_logging(
        &config.observability.log_level,
        &config.observability.log_format,
    );

    tracing::info!("Zero Browser v{}", env!("CARGO_PKG_VERSION"));

    // Create application state
    let session_timeout = config
        .browser
        .as_ref()
        .map(|b| b.session_timeout_secs)
        .unwrap_or(1800);

    let state = AppState::new(session_timeout);

    // Build router with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = build_router(state).layer(cors);

    // Get bind address from config
    let port = config.browser.as_ref().map(|b| b.port).unwrap_or(4433);
    let host: std::net::IpAddr = config
        .browser
        .as_ref()
        .and_then(|b| b.host.parse().ok())
        .unwrap_or([127, 0, 0, 1].into());

    let addr = SocketAddr::from((host, port));

    let startup_duration = startup_start.elapsed();
    tracing::info!(
        duration_ms = startup_duration.as_millis() as u64,
        "Service initialized in {:?}",
        startup_duration
    );

    tracing::info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

**Step 3: Add browser config to zero-common**

Add to `services/zero-common/src/config.rs` (or create if needed):

```rust
/// Browser service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default = "default_browser_host")]
    pub host: String,
    #[serde(default = "default_browser_port")]
    pub port: u16,
    #[serde(default)]
    pub chrome_path: Option<String>,
    #[serde(default = "default_headless")]
    pub headless: bool,
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: i64,
    #[serde(default = "default_max_body_size")]
    pub max_body_size_mb: u64,
}

fn default_browser_host() -> String {
    "127.0.0.1".to_string()
}

fn default_browser_port() -> u16 {
    4433
}

fn default_headless() -> bool {
    true
}

fn default_session_timeout() -> i64 {
    1800
}

fn default_max_body_size() -> u64 {
    10
}
```

**Step 4: Verify it compiles**

Run: `cargo build -p zero-browser`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add services/zero-browser/src/ services/zero-common/src/
git commit -m "feat(zero-browser): complete main entry point with config"
```

---

## Task 10: Integration Tests

**Files:**
- Create: `services/zero-browser/tests/integration.rs`

**Step 1: Create integration test**

```rust
//! Integration tests for zero-browser.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use zero_browser::{build_router, AppState};

fn test_app() -> axum::Router {
    build_router(AppState::new(1800))
}

#[tokio::test]
async fn test_full_learning_flow() {
    let app = test_app();

    // 1. Create session
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/browser/sessions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"headless": true}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let session_id = json["data"]["session_id"].as_str().unwrap();

    // 2. Start learning
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/browser/sessions/{}/learn/start", session_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"filter": {"hosts": ["api.example.com"]}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 3. Stop learning (no requests captured in test)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/browser/sessions/{}/learn/stop", session_id))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["request_count"], 0);
    assert_eq!(json["data"]["unique_endpoints"], 0);

    // 4. Close session
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/browser/sessions/{}", session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_pattern_crud() {
    let state = AppState::new(1800);

    // Manually insert a pattern
    {
        let mut patterns = state.patterns.write().await;
        let pattern = zero_browser::ApiPattern::new("api.example.com", "GET", "/users");
        patterns.insert(pattern.id.clone(), pattern);
    }

    let app = build_router(state);

    // List patterns
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/patterns").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["count"], 1);

    // Get single pattern
    let pattern_id = "api.example.com:GET:/users";
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/patterns/{}", urlencoding::encode(pattern_id)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Delete pattern
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/patterns/{}", urlencoding::encode(pattern_id)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

**Step 2: Add urlencoding to dev-dependencies**

Add to Cargo.toml: `urlencoding = "2.1"`

**Step 3: Run integration tests**

Run: `cargo test -p zero-browser --test integration`
Expected: Both tests pass

**Step 4: Commit**

```bash
git add services/zero-browser/tests/ services/zero-browser/Cargo.toml
git commit -m "test(zero-browser): add integration tests"
```

---

## Task 11: Update Workspace and ops.sh

**Files:**
- Modify: `services/Cargo.toml`
- Modify: `ops.sh`

**Step 1: Ensure workspace member is added**

Verify `services/Cargo.toml` contains `"zero-browser"` in members.

**Step 2: Update ops.sh**

Add zero-browser to the service list and daemon startup.

**Step 3: Build and test all**

Run: `cargo test -p zero-browser`
Expected: All tests pass

Run: `cargo build -p zero-browser --release`
Expected: Build succeeds

**Step 4: Final commit**

```bash
git add services/Cargo.toml ops.sh
git commit -m "chore: integrate zero-browser into workspace and ops"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Project scaffolding | Cargo.toml, lib.rs, main.rs |
| 2 | Pattern data types | pattern/types.rs |
| 3 | Error types | error.rs |
| 4 | Session management | browser/session.rs |
| 5 | Network monitor | network/monitor.rs |
| 6 | Pattern extractor | pattern/extractor.rs |
| 7 | Replay executor | replay/executor.rs |
| 8 | HTTP routes | routes.rs |
| 9 | Main entry point | main.rs (complete) |
| 10 | Integration tests | tests/integration.rs |
| 11 | Workspace integration | Cargo.toml, ops.sh |

**Total: 11 tasks, ~372 lines of core code, ~200 lines of tests**
