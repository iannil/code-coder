//! Security sandbox for Zero Gateway.
//!
//! Provides request/response filtering, sensitive data detection, and audit logging.
//! Supports both in-memory and SQLite-based audit log persistence.

use chrono::{DateTime, Utc};
use regex::Regex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Audit action types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// User login attempt
    Login { username: String, success: bool },
    /// User logout
    Logout { user_id: String },
    /// API request
    Request {
        method: String,
        path: String,
        status: u16,
    },
    /// Blocked request
    Blocked { reason: String, details: String },
    /// Sensitive data detected
    SensitiveDataDetected { pattern_name: String, redacted: bool },
    /// User management action
    UserAction {
        action: String,
        target_user_id: String,
    },
    /// Configuration change
    ConfigChange { key: String },
}

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Source IP address
    pub ip_address: Option<String>,
    /// Action performed
    pub action: AuditAction,
    /// Request ID for correlation
    pub request_id: Option<String>,
}

/// Daily count for audit summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCount {
    /// Date in YYYY-MM-DD format
    pub date: String,
    /// Number of entries on this day
    pub count: u64,
}

/// Audit summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    /// Total number of audit entries
    pub total_entries: u64,
    /// Count by action type
    pub by_action_type: HashMap<String, u64>,
    /// Count by day (last 30 days)
    pub by_day: Vec<DayCount>,
    /// Recent blocked entries
    pub recent_blocked: Vec<AuditEntry>,
}

/// Pattern for detecting sensitive data.
#[derive(Debug, Clone)]
pub struct SensitivePattern {
    /// Pattern name
    pub name: String,
    /// Regex pattern
    pub pattern: Regex,
    /// Replacement string for redaction
    pub replacement: String,
}

impl SensitivePattern {
    /// Create a new sensitive pattern.
    pub fn new(name: &str, pattern: &str, replacement: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            name: name.to_string(),
            pattern: Regex::new(pattern)?,
            replacement: replacement.to_string(),
        })
    }
}

/// Security sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable request filtering
    pub filter_requests: bool,
    /// Enable response sanitization
    pub sanitize_responses: bool,
    /// Enable audit logging
    pub audit_logging: bool,
    /// Blocked endpoints (exact match)
    pub blocked_endpoints: Vec<String>,
    /// Blocked path patterns (regex)
    pub blocked_path_patterns: Vec<String>,
    /// Maximum request body size in bytes
    pub max_request_body_size: usize,
    /// Maximum audit log entries to keep in memory
    pub max_audit_entries: usize,
    /// Path to SQLite database for audit log persistence (optional)
    #[serde(default)]
    pub audit_db_path: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            filter_requests: true,
            sanitize_responses: true,
            audit_logging: true,
            blocked_endpoints: vec![
                "/admin/debug".to_string(),
                "/internal/metrics".to_string(),
            ],
            blocked_path_patterns: vec![
                r"\.\.".to_string(), // Path traversal
                r"<script".to_string(), // XSS attempt
            ],
            max_request_body_size: 10 * 1024 * 1024, // 10MB
            max_audit_entries: 10000,
            audit_db_path: None,
        }
    }
}

/// Audit log storage backend
enum AuditStorage {
    /// In-memory only
    Memory(Vec<AuditEntry>),
    /// SQLite-backed with in-memory cache
    Sqlite {
        conn: Connection,
        cache: Vec<AuditEntry>,
    },
}

/// Security sandbox for request/response filtering.
pub struct Sandbox {
    config: SandboxConfig,
    sensitive_patterns: Vec<SensitivePattern>,
    blocked_patterns: Vec<Regex>,
    audit_storage: Arc<Mutex<AuditStorage>>,
}

/// Result of request filtering.
#[derive(Debug)]
pub enum FilterResult {
    /// Request is allowed
    Allowed,
    /// Request is blocked with reason
    Blocked { reason: String },
}

/// Result of response sanitization.
#[derive(Debug)]
pub struct SanitizeResult {
    /// Sanitized content
    pub content: String,
    /// Patterns that were detected and redacted
    pub redacted_patterns: Vec<String>,
}

impl Sandbox {
    /// Create a new sandbox with default configuration and patterns.
    pub fn new(config: SandboxConfig) -> Self {
        let sensitive_patterns = vec![
            // ── API Keys ──────────────────────────────────────────────
            SensitivePattern::new(
                "api_key_sk",
                r"sk[-_][a-zA-Z0-9]{20,}",
                "[REDACTED_API_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "api_key_pk",
                r"pk[-_][a-zA-Z0-9]{20,}",
                "[REDACTED_API_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "openai_key",
                r"sk-[a-zA-Z0-9]{48}",
                "[REDACTED_OPENAI_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "anthropic_key",
                r"sk-ant-[a-zA-Z0-9\-]{95,}",
                "[REDACTED_ANTHROPIC_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "google_api_key",
                r"AIza[0-9A-Za-z\-_]{35}",
                "[REDACTED_GOOGLE_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "github_token",
                r"ghp_[a-zA-Z0-9]{36}",
                "[REDACTED_GITHUB_TOKEN]",
            )
            .unwrap(),
            SensitivePattern::new(
                "github_oauth",
                r"gho_[a-zA-Z0-9]{36}",
                "[REDACTED_GITHUB_OAUTH]",
            )
            .unwrap(),
            SensitivePattern::new(
                "stripe_key",
                r"(sk|pk)_(live|test)_[a-zA-Z0-9]{24,}",
                "[REDACTED_STRIPE_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "slack_token",
                r"xox[baprs]-[a-zA-Z0-9\-]{10,}",
                "[REDACTED_SLACK_TOKEN]",
            )
            .unwrap(),
            // ── AWS ────────────────────────────────────────────────────
            SensitivePattern::new(
                "aws_access_key",
                r"AKIA[0-9A-Z]{16}",
                "[REDACTED_AWS_KEY]",
            )
            .unwrap(),
            SensitivePattern::new(
                "aws_secret",
                r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key["']?\s*[:=]\s*["']?([A-Za-z0-9/+=]{40})"#,
                "[REDACTED_AWS_SECRET]",
            )
            .unwrap(),
            // ── Generic Credentials ────────────────────────────────────
            SensitivePattern::new(
                "bearer_token",
                r"Bearer\s+[a-zA-Z0-9\-_.~+/]+=*",
                "Bearer [REDACTED]",
            )
            .unwrap(),
            SensitivePattern::new(
                "password_field",
                r#"(?i)"password"\s*:\s*"[^"]+""#,
                r#""password":"[REDACTED]""#,
            )
            .unwrap(),
            SensitivePattern::new(
                "api_key_field",
                r#"(?i)"api[_-]?key"\s*:\s*"[^"]+""#,
                r#""api_key":"[REDACTED]""#,
            )
            .unwrap(),
            SensitivePattern::new(
                "secret_field",
                r#"(?i)"(secret|token|auth)[_-]?(key|token)?"\s*:\s*"[^"]+""#,
                r#""secret":"[REDACTED]""#,
            )
            .unwrap(),
            SensitivePattern::new(
                "jwt_token",
                r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]+",
                "[REDACTED_JWT]",
            )
            .unwrap(),
            SensitivePattern::new(
                "private_key",
                r"-----BEGIN[A-Z ]*PRIVATE KEY-----",
                "[REDACTED_PRIVATE_KEY]",
            )
            .unwrap(),
            // ── PII (Personally Identifiable Information) ──────────────
            SensitivePattern::new(
                "credit_card",
                r"\b(?:\d{4}[- ]?){3}\d{4}\b",
                "[REDACTED_CC]",
            )
            .unwrap(),
            SensitivePattern::new(
                "ssn_us",
                r"\b\d{3}-\d{2}-\d{4}\b",
                "[REDACTED_SSN]",
            )
            .unwrap(),
            SensitivePattern::new(
                "phone_us",
                r"\b(?:\+1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
                "[REDACTED_PHONE]",
            )
            .unwrap(),
            SensitivePattern::new(
                "phone_cn",
                r"\b(?:\+86[-.\s]?)?1[3-9]\d{9}\b",
                "[REDACTED_PHONE_CN]",
            )
            .unwrap(),
            SensitivePattern::new(
                "id_cn",
                r"\b\d{6}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]\b",
                "[REDACTED_ID_CN]",
            )
            .unwrap(),
            SensitivePattern::new(
                "bank_account_json",
                r#"(?i)"(?:account|bank|iban)[_-]?(?:number|no|num)?"\s*:\s*"?\d{10,18}"?"#,
                r#""account":"[REDACTED_BANK]""#,
            )
            .unwrap(),
            SensitivePattern::new(
                "iban",
                r"\b[A-Z]{2}\d{2}[A-Z0-9]{4}\d{7}([A-Z0-9]?){0,16}\b",
                "[REDACTED_IBAN]",
            )
            .unwrap(),
            // ── Database Connection Strings ────────────────────────────
            SensitivePattern::new(
                "postgres_url",
                r"postgres(?:ql)?://[^:]+:[^@]+@[^/]+",
                "[REDACTED_DB_URL]",
            )
            .unwrap(),
            SensitivePattern::new(
                "mysql_url",
                r"mysql://[^:]+:[^@]+@[^/]+",
                "[REDACTED_DB_URL]",
            )
            .unwrap(),
            SensitivePattern::new(
                "redis_url",
                r"redis://(?:[^:]+:)?[^@]+@[^/]+",
                "[REDACTED_REDIS_URL]",
            )
            .unwrap(),
        ];

        let blocked_patterns = config
            .blocked_path_patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        // Initialize audit storage
        let audit_storage = if let Some(ref db_path) = config.audit_db_path {
            match Self::init_sqlite_storage(db_path) {
                Ok(conn) => AuditStorage::Sqlite {
                    conn,
                    cache: Vec::new(),
                },
                Err(e) => {
                    tracing::warn!("Failed to initialize audit database at {}: {}. Using in-memory.", db_path, e);
                    AuditStorage::Memory(Vec::new())
                }
            }
        } else {
            AuditStorage::Memory(Vec::new())
        };

        Self {
            config,
            sensitive_patterns,
            blocked_patterns,
            audit_storage: Arc::new(Mutex::new(audit_storage)),
        }
    }

    /// Initialize SQLite storage for audit logs
    fn init_sqlite_storage(db_path: &str) -> Result<Connection, rusqlite::Error> {
        let path = PathBuf::from(db_path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&path)?;

        // Create audit_log table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                user_id TEXT,
                ip_address TEXT,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                request_id TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // Create indexes for common queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_user_id ON audit_log(user_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_action_type ON audit_log(action_type)",
            [],
        )?;

        tracing::info!("Audit log database initialized at {}", db_path);
        Ok(conn)
    }

    /// Filter an incoming request.
    pub fn filter_request(
        &self,
        _method: &str,
        path: &str,
        body: Option<&str>,
        _headers: &HashMap<String, String>,
    ) -> FilterResult {
        if !self.config.filter_requests {
            return FilterResult::Allowed;
        }

        // Check blocked endpoints
        if self.config.blocked_endpoints.iter().any(|e| path == e) {
            return FilterResult::Blocked {
                reason: format!("Endpoint {} is blocked", path),
            };
        }

        // Check blocked path patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(path) {
                return FilterResult::Blocked {
                    reason: format!(
                        "Path matches blocked pattern: {}",
                        pattern.as_str()
                    ),
                };
            }
        }

        // Check request body size
        if let Some(body) = body {
            if body.len() > self.config.max_request_body_size {
                return FilterResult::Blocked {
                    reason: format!(
                        "Request body too large: {} bytes (max: {})",
                        body.len(),
                        self.config.max_request_body_size
                    ),
                };
            }
        }

        // Check for suspicious patterns in body
        if let Some(body) = body {
            // SQL injection patterns
            let sql_patterns = [
                r"(?i)\b(union|select|insert|update|delete|drop|truncate)\b.*\b(from|into|table)\b",
                r"(?i)--\s*$",
                r";\s*--",
            ];
            for pattern in &sql_patterns {
                if let Ok(re) = Regex::new(pattern) {
                    if re.is_match(body) {
                        return FilterResult::Blocked {
                            reason: "Potential SQL injection detected".to_string(),
                        };
                    }
                }
            }
        }

        // Check for XSS in path
        if path.contains('<') || path.contains('>') || path.contains("javascript:") {
            return FilterResult::Blocked {
                reason: "Potential XSS in path".to_string(),
            };
        }

        FilterResult::Allowed
    }

    /// Sanitize a response by redacting sensitive data.
    pub fn sanitize_response(&self, content: &str) -> SanitizeResult {
        if !self.config.sanitize_responses {
            return SanitizeResult {
                content: content.to_string(),
                redacted_patterns: vec![],
            };
        }

        let mut result = content.to_string();
        let mut redacted = vec![];

        for pattern in &self.sensitive_patterns {
            if pattern.pattern.is_match(&result) {
                result = pattern
                    .pattern
                    .replace_all(&result, &pattern.replacement)
                    .to_string();
                redacted.push(pattern.name.clone());
            }
        }

        SanitizeResult {
            content: result,
            redacted_patterns: redacted,
        }
    }

    /// Check if content contains sensitive data.
    pub fn contains_sensitive_data(&self, content: &str) -> Vec<String> {
        self.sensitive_patterns
            .iter()
            .filter(|p| p.pattern.is_match(content))
            .map(|p| p.name.clone())
            .collect()
    }

    /// Log an audit entry.
    pub fn audit(&self, entry: AuditEntry) {
        if !self.config.audit_logging {
            return;
        }

        let mut storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Log to tracing
        tracing::info!(
            user_id = ?entry.user_id,
            action = ?entry.action,
            request_id = ?entry.request_id,
            "Audit log entry"
        );

        match &mut *storage {
            AuditStorage::Memory(log) => {
                // Trim old entries if needed
                while log.len() >= self.config.max_audit_entries {
                    log.remove(0);
                }
                log.push(entry);
            }
            AuditStorage::Sqlite { conn, cache } => {
                // Persist to SQLite
                if let Err(e) = Self::persist_audit_entry(conn, &entry) {
                    tracing::error!("Failed to persist audit entry: {}", e);
                }

                // Also keep in cache for fast access
                while cache.len() >= self.config.max_audit_entries {
                    cache.remove(0);
                }
                cache.push(entry);
            }
        }
    }

    /// Persist an audit entry to SQLite
    fn persist_audit_entry(conn: &Connection, entry: &AuditEntry) -> Result<(), rusqlite::Error> {
        let action_type = match &entry.action {
            AuditAction::Login { .. } => "login",
            AuditAction::Logout { .. } => "logout",
            AuditAction::Request { .. } => "request",
            AuditAction::Blocked { .. } => "blocked",
            AuditAction::SensitiveDataDetected { .. } => "sensitive_data",
            AuditAction::UserAction { .. } => "user_action",
            AuditAction::ConfigChange { .. } => "config_change",
        };

        let action_data = serde_json::to_string(&entry.action).unwrap_or_default();

        conn.execute(
            r#"
            INSERT INTO audit_log (id, timestamp, user_id, ip_address, action_type, action_data, request_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                entry.id,
                entry.timestamp.to_rfc3339(),
                entry.user_id,
                entry.ip_address,
                action_type,
                action_data,
                entry.request_id,
            ],
        )?;

        Ok(())
    }

    /// Create an audit entry with current timestamp.
    pub fn create_audit_entry(
        &self,
        user_id: Option<String>,
        ip_address: Option<String>,
        action: AuditAction,
        request_id: Option<String>,
    ) -> AuditEntry {
        AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            user_id,
            ip_address,
            action,
            request_id,
        }
    }

    /// Get recent audit entries.
    pub fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => {
                log.iter().rev().take(limit).cloned().collect()
            }
            AuditStorage::Sqlite { conn, cache } => {
                // Try to query from SQLite first for persistent data
                match Self::query_audit_log(conn, limit, None) {
                    Ok(entries) => entries,
                    Err(e) => {
                        tracing::warn!("Failed to query audit log from SQLite: {}, using cache", e);
                        cache.iter().rev().take(limit).cloned().collect()
                    }
                }
            }
        }
    }

    /// Get audit entries for a specific user.
    pub fn get_user_audit_log(&self, user_id: &str, limit: usize) -> Vec<AuditEntry> {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => {
                log.iter()
                    .rev()
                    .filter(|e| e.user_id.as_deref() == Some(user_id))
                    .take(limit)
                    .cloned()
                    .collect()
            }
            AuditStorage::Sqlite { conn, cache } => {
                // Try to query from SQLite
                match Self::query_audit_log(conn, limit, Some(user_id)) {
                    Ok(entries) => entries,
                    Err(e) => {
                        tracing::warn!("Failed to query user audit log from SQLite: {}, using cache", e);
                        cache.iter()
                            .rev()
                            .filter(|e| e.user_id.as_deref() == Some(user_id))
                            .take(limit)
                            .cloned()
                            .collect()
                    }
                }
            }
        }
    }

    /// Query audit entries from SQLite
    fn query_audit_log(
        conn: &Connection,
        limit: usize,
        user_id: Option<&str>,
    ) -> Result<Vec<AuditEntry>, rusqlite::Error> {
        let mut entries = Vec::new();

        let query = if user_id.is_some() {
            "SELECT id, timestamp, user_id, ip_address, action_data, request_id FROM audit_log WHERE user_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
        } else {
            "SELECT id, timestamp, user_id, ip_address, action_data, request_id FROM audit_log ORDER BY timestamp DESC LIMIT ?1"
        };

        let mut stmt = conn.prepare(query)?;

        let rows = if let Some(uid) = user_id {
            stmt.query_map(params![uid, limit], Self::row_to_entry)?
        } else {
            stmt.query_map(params![limit], Self::row_to_entry)?
        };

        for row in rows {
            if let Ok(Some(entry)) = row {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Convert a SQLite row to an AuditEntry
    fn row_to_entry(row: &rusqlite::Row) -> Result<Option<AuditEntry>, rusqlite::Error> {
        let id: String = row.get(0)?;
        let timestamp_str: String = row.get(1)?;
        let user_id: Option<String> = row.get(2)?;
        let ip_address: Option<String> = row.get(3)?;
        let action_data: String = row.get(4)?;
        let request_id: Option<String> = row.get(5)?;

        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let action: AuditAction = serde_json::from_str(&action_data).unwrap_or(AuditAction::Request {
            method: "UNKNOWN".to_string(),
            path: "".to_string(),
            status: 0,
        });

        Ok(Some(AuditEntry {
            id,
            timestamp,
            user_id,
            ip_address,
            action,
            request_id,
        }))
    }

    /// Get total count of audit entries (for pagination)
    pub fn get_audit_count(&self) -> usize {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => log.len(),
            AuditStorage::Sqlite { conn, cache } => {
                conn.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
                    .unwrap_or(cache.len())
            }
        }
    }

    /// Query audit entries by action type
    pub fn get_audit_by_action_type(&self, action_type: &str, limit: usize) -> Vec<AuditEntry> {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => {
                log.iter()
                    .rev()
                    .filter(|e| {
                        let entry_type = match &e.action {
                            AuditAction::Login { .. } => "login",
                            AuditAction::Logout { .. } => "logout",
                            AuditAction::Request { .. } => "request",
                            AuditAction::Blocked { .. } => "blocked",
                            AuditAction::SensitiveDataDetected { .. } => "sensitive_data",
                            AuditAction::UserAction { .. } => "user_action",
                            AuditAction::ConfigChange { .. } => "config_change",
                        };
                        entry_type == action_type
                    })
                    .take(limit)
                    .cloned()
                    .collect()
            }
            AuditStorage::Sqlite { conn, cache } => {
                let query = "SELECT id, timestamp, user_id, ip_address, action_data, request_id FROM audit_log WHERE action_type = ?1 ORDER BY timestamp DESC LIMIT ?2";

                match conn.prepare(query) {
                    Ok(mut stmt) => {
                        let rows = stmt.query_map(params![action_type, limit], Self::row_to_entry);
                        match rows {
                            Ok(rows) => rows.filter_map(|r| r.ok().flatten()).collect(),
                            Err(_) => cache.iter().rev().take(limit).cloned().collect(),
                        }
                    }
                    Err(_) => cache.iter().rev().take(limit).cloned().collect(),
                }
            }
        }
    }

    /// Get a single audit entry by ID.
    pub fn get_audit_by_id(&self, id: &str) -> Option<AuditEntry> {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => log.iter().find(|e| e.id == id).cloned(),
            AuditStorage::Sqlite { conn, cache } => {
                // Try SQLite first
                let query = "SELECT id, timestamp, user_id, ip_address, action_data, request_id FROM audit_log WHERE id = ?1";
                match conn.prepare(query) {
                    Ok(mut stmt) => {
                        match stmt.query_row(params![id], Self::row_to_entry) {
                            Ok(entry) => entry,
                            Err(_) => cache.iter().find(|e| e.id == id).cloned(),
                        }
                    }
                    Err(_) => cache.iter().find(|e| e.id == id).cloned(),
                }
            }
        }
    }

    /// Get paginated audit entries with optional filters.
    pub fn get_audit_paginated(
        &self,
        limit: usize,
        offset: usize,
        action_type: Option<&str>,
        user_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> (Vec<AuditEntry>, u64) {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => {
                // Apply filters in memory
                let filtered: Vec<_> = log
                    .iter()
                    .rev()
                    .filter(|e| {
                        // Filter by action type
                        if let Some(at) = action_type {
                            let entry_type = match &e.action {
                                AuditAction::Login { .. } => "login",
                                AuditAction::Logout { .. } => "logout",
                                AuditAction::Request { .. } => "request",
                                AuditAction::Blocked { .. } => "blocked",
                                AuditAction::SensitiveDataDetected { .. } => "sensitive_data",
                                AuditAction::UserAction { .. } => "user_action",
                                AuditAction::ConfigChange { .. } => "config_change",
                            };
                            if entry_type != at {
                                return false;
                            }
                        }
                        // Filter by user
                        if let Some(uid) = user_id {
                            if e.user_id.as_deref() != Some(uid) {
                                return false;
                            }
                        }
                        // Filter by time range
                        if let Some(start) = start_time {
                            if e.timestamp < start {
                                return false;
                            }
                        }
                        if let Some(end) = end_time {
                            if e.timestamp > end {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                let total = filtered.len() as u64;
                let entries = filtered
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .cloned()
                    .collect();

                (entries, total)
            }
            AuditStorage::Sqlite { conn, cache } => {
                match Self::query_audit_paginated(conn, limit, offset, action_type, user_id, start_time, end_time) {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::warn!("Failed to query paginated audit log from SQLite: {}, using cache", e);
                        // Fallback to cache with filtering
                        let filtered: Vec<_> = cache
                            .iter()
                            .rev()
                            .filter(|e| {
                                if let Some(at) = action_type {
                                    let entry_type = match &e.action {
                                        AuditAction::Login { .. } => "login",
                                        AuditAction::Logout { .. } => "logout",
                                        AuditAction::Request { .. } => "request",
                                        AuditAction::Blocked { .. } => "blocked",
                                        AuditAction::SensitiveDataDetected { .. } => "sensitive_data",
                                        AuditAction::UserAction { .. } => "user_action",
                                        AuditAction::ConfigChange { .. } => "config_change",
                                    };
                                    if entry_type != at {
                                        return false;
                                    }
                                }
                                if let Some(uid) = user_id {
                                    if e.user_id.as_deref() != Some(uid) {
                                        return false;
                                    }
                                }
                                if let Some(start) = start_time {
                                    if e.timestamp < start {
                                        return false;
                                    }
                                }
                                if let Some(end) = end_time {
                                    if e.timestamp > end {
                                        return false;
                                    }
                                }
                                true
                            })
                            .collect();

                        let total = filtered.len() as u64;
                        let entries = filtered.into_iter().skip(offset).take(limit).cloned().collect();
                        (entries, total)
                    }
                }
            }
        }
    }

    /// Query paginated audit entries from SQLite with filters.
    fn query_audit_paginated(
        conn: &Connection,
        limit: usize,
        offset: usize,
        action_type: Option<&str>,
        user_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(Vec<AuditEntry>, u64), rusqlite::Error> {
        // Build dynamic WHERE clause
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(at) = action_type {
            conditions.push(format!("action_type = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(at.to_string()));
        }
        if let Some(uid) = user_id {
            conditions.push(format!("user_id = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(uid.to_string()));
        }
        if let Some(start) = start_time {
            conditions.push(format!("timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(start.to_rfc3339()));
        }
        if let Some(end) = end_time {
            conditions.push(format!("timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(end.to_rfc3339()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM audit_log {}", where_clause);
        let total: u64 = {
            let mut stmt = conn.prepare(&count_query)?;
            let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
            stmt.query_row(params_refs.as_slice(), |row| row.get(0))?
        };

        // Get paginated entries
        let select_query = format!(
            "SELECT id, timestamp, user_id, ip_address, action_data, request_id \
             FROM audit_log {} ORDER BY timestamp DESC LIMIT ?{} OFFSET ?{}",
            where_clause,
            params_vec.len() + 1,
            params_vec.len() + 2
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let mut entries = Vec::new();
        let mut stmt = conn.prepare(&select_query)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), Self::row_to_entry)?;

        for row in rows {
            if let Ok(Some(entry)) = row {
                entries.push(entry);
            }
        }

        Ok((entries, total))
    }

    /// Get audit summary statistics.
    pub fn get_audit_summary(&self) -> AuditSummary {
        let storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &*storage {
            AuditStorage::Memory(log) => Self::compute_summary_from_entries(log),
            AuditStorage::Sqlite { conn, cache } => {
                match Self::query_audit_summary(conn) {
                    Ok(summary) => summary,
                    Err(e) => {
                        tracing::warn!("Failed to query audit summary from SQLite: {}, using cache", e);
                        Self::compute_summary_from_entries(cache)
                    }
                }
            }
        }
    }

    /// Compute summary from a list of entries.
    fn compute_summary_from_entries(entries: &[AuditEntry]) -> AuditSummary {
        let mut by_action_type = HashMap::new();
        let mut by_day = HashMap::new();
        let mut recent_blocked = Vec::new();

        for entry in entries {
            // Count by action type
            let action_type = match &entry.action {
                AuditAction::Login { .. } => "login",
                AuditAction::Logout { .. } => "logout",
                AuditAction::Request { .. } => "request",
                AuditAction::Blocked { .. } => "blocked",
                AuditAction::SensitiveDataDetected { .. } => "sensitive_data",
                AuditAction::UserAction { .. } => "user_action",
                AuditAction::ConfigChange { .. } => "config_change",
            };
            *by_action_type.entry(action_type.to_string()).or_insert(0u64) += 1;

            // Count by day
            let day = entry.timestamp.format("%Y-%m-%d").to_string();
            *by_day.entry(day).or_insert(0u64) += 1;

            // Collect blocked entries
            if matches!(&entry.action, AuditAction::Blocked { .. }) {
                recent_blocked.push(entry.clone());
            }
        }

        // Sort by_day and convert to DayCount vec
        let mut by_day_vec: Vec<DayCount> = by_day
            .into_iter()
            .map(|(date, count)| DayCount { date, count })
            .collect();
        by_day_vec.sort_by(|a, b| b.date.cmp(&a.date));
        by_day_vec.truncate(30); // Last 30 days

        // Sort blocked entries by timestamp desc, take recent 10
        recent_blocked.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recent_blocked.truncate(10);

        AuditSummary {
            total_entries: entries.len() as u64,
            by_action_type,
            by_day: by_day_vec,
            recent_blocked,
        }
    }

    /// Query audit summary from SQLite.
    fn query_audit_summary(conn: &Connection) -> Result<AuditSummary, rusqlite::Error> {
        // Total count
        let total_entries: u64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log",
            [],
            |row| row.get(0),
        )?;

        // Count by action type
        let mut by_action_type = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT action_type, COUNT(*) FROM audit_log GROUP BY action_type"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?;
        for row in rows {
            if let Ok((action_type, count)) = row {
                by_action_type.insert(action_type, count);
            }
        }

        // Count by day (last 30 days)
        let mut by_day = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT date(timestamp) as day, COUNT(*) FROM audit_log \
             GROUP BY day ORDER BY day DESC LIMIT 30"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DayCount {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        for row in rows {
            if let Ok(day_count) = row {
                by_day.push(day_count);
            }
        }

        // Recent blocked entries
        let mut recent_blocked = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, user_id, ip_address, action_data, request_id \
             FROM audit_log WHERE action_type = 'blocked' \
             ORDER BY timestamp DESC LIMIT 10"
        )?;
        let rows = stmt.query_map([], Self::row_to_entry)?;
        for row in rows {
            if let Ok(Some(entry)) = row {
                recent_blocked.push(entry);
            }
        }

        Ok(AuditSummary {
            total_entries,
            by_action_type,
            by_day,
            recent_blocked,
        })
    }

    /// Clear audit log (for testing).
    #[cfg(test)]
    pub fn clear_audit_log(&self) {
        let mut storage = self
            .audit_storage
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match &mut *storage {
            AuditStorage::Memory(log) => log.clear(),
            AuditStorage::Sqlite { conn, cache } => {
                cache.clear();
                conn.execute("DELETE FROM audit_log", []).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sandbox() -> Sandbox {
        Sandbox::new(SandboxConfig::default())
    }

    // ── Request Filtering ──────────────────────────────────────

    #[test]
    fn test_allows_normal_request() {
        let sandbox = create_test_sandbox();
        let result = sandbox.filter_request("GET", "/api/v1/sessions", None, &HashMap::new());
        assert!(matches!(result, FilterResult::Allowed));
    }

    #[test]
    fn test_blocks_path_traversal() {
        let sandbox = create_test_sandbox();
        let result =
            sandbox.filter_request("GET", "/api/v1/../../../etc/passwd", None, &HashMap::new());
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }

    #[test]
    fn test_blocks_blocked_endpoint() {
        let sandbox = create_test_sandbox();
        let result = sandbox.filter_request("GET", "/admin/debug", None, &HashMap::new());
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }

    #[test]
    fn test_blocks_large_body() {
        let sandbox = Sandbox::new(SandboxConfig {
            max_request_body_size: 100,
            ..Default::default()
        });
        let large_body = "x".repeat(200);
        let result =
            sandbox.filter_request("POST", "/api/v1/data", Some(&large_body), &HashMap::new());
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }

    #[test]
    fn test_blocks_sql_injection() {
        let sandbox = create_test_sandbox();
        let body = r#"{"query": "SELECT * FROM users; DROP TABLE users;--"}"#;
        let result = sandbox.filter_request("POST", "/api/v1/query", Some(body), &HashMap::new());
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }

    #[test]
    fn test_blocks_xss_in_path() {
        let sandbox = create_test_sandbox();
        let result = sandbox.filter_request(
            "GET",
            "/api/v1/search?q=<script>alert(1)</script>",
            None,
            &HashMap::new(),
        );
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }

    // ── Response Sanitization ──────────────────────────────────

    #[test]
    fn test_redacts_api_keys() {
        let sandbox = create_test_sandbox();
        // Pattern is sk[-_][a-zA-Z0-9]{20,} - needs 20+ alphanumeric after sk-
        let content = r#"{"api_key": "sk-1234567890abcdefghijABCDEF"}"#;
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("sk-1234"), "API key should be redacted");
        assert!(result.content.contains("[REDACTED"), "Should contain redaction marker");
        assert!(!result.redacted_patterns.is_empty(), "Should have redacted patterns");
    }

    #[test]
    fn test_redacts_openai_key() {
        let sandbox = create_test_sandbox();
        // OpenAI keys are exactly 51 chars: sk- + 48 alphanumeric
        let content = "Using API key: sk-abcdefghijklmnopqrstuvwxyz123456789012345678";
        let result = sandbox.sanitize_response(content);
        // Should match api_key_sk pattern since it starts with sk-
        assert!(
            !result.content.contains("sk-abc"),
            "Content should not contain the key"
        );
    }

    #[test]
    fn test_redacts_aws_access_key() {
        let sandbox = create_test_sandbox();
        let content = "AWS Key: AKIAIOSFODNN7EXAMPLE";
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(result.content.contains("[REDACTED_AWS_KEY]"));
    }

    #[test]
    fn test_redacts_bearer_token() {
        let sandbox = create_test_sandbox();
        let content = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("eyJhbGci"));
        assert!(result.content.contains("[REDACTED]"));
    }

    #[test]
    fn test_redacts_password_field() {
        let sandbox = create_test_sandbox();
        let content = r#"{"username": "admin", "password": "secretpass123"}"#;
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("secretpass123"));
        assert!(result.content.contains("[REDACTED]"));
    }

    #[test]
    fn test_redacts_private_key() {
        let sandbox = create_test_sandbox();
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpA...";
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("BEGIN RSA PRIVATE KEY"));
        assert!(result.content.contains("[REDACTED_PRIVATE_KEY]"));
    }

    #[test]
    fn test_redacts_jwt() {
        let sandbox = create_test_sandbox();
        let content =
            "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let result = sandbox.sanitize_response(content);
        assert!(!result.content.contains("eyJhbGciOi"));
        assert!(result.content.contains("[REDACTED_JWT]"));
    }

    #[test]
    fn test_preserves_safe_content() {
        let sandbox = create_test_sandbox();
        let content = r#"{"message": "Hello, world!", "count": 42}"#;
        let result = sandbox.sanitize_response(content);
        assert_eq!(result.content, content);
        assert!(result.redacted_patterns.is_empty());
    }

    // ── Sensitive Data Detection ────────────────────────────────

    #[test]
    fn test_detects_sensitive_data() {
        let sandbox = create_test_sandbox();
        // Use a key that matches the api_key_sk pattern (sk- followed by 20+ alphanumeric)
        let content = "API Key: sk-1234567890abcdefghijABCD";
        let detected = sandbox.contains_sensitive_data(content);
        assert!(
            !detected.is_empty(),
            "Should detect sensitive data in: {}",
            content
        );
    }

    #[test]
    fn test_no_false_positives() {
        let sandbox = create_test_sandbox();
        let content = "This is a normal message without any secrets.";
        let detected = sandbox.contains_sensitive_data(content);
        assert!(detected.is_empty());
    }

    // ── Audit Logging ──────────────────────────────────────────

    #[test]
    fn test_audit_logging() {
        let sandbox = create_test_sandbox();

        let entry = sandbox.create_audit_entry(
            Some("user123".to_string()),
            Some("192.168.1.1".to_string()),
            AuditAction::Login {
                username: "testuser".to_string(),
                success: true,
            },
            None,
        );

        sandbox.audit(entry);

        let log = sandbox.get_audit_log(10);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].user_id, Some("user123".to_string()));
    }

    #[test]
    fn test_audit_log_limit() {
        let sandbox = Sandbox::new(SandboxConfig {
            max_audit_entries: 3,
            ..Default::default()
        });

        for i in 0..5 {
            let entry = sandbox.create_audit_entry(
                Some(format!("user{}", i)),
                None,
                AuditAction::Request {
                    method: "GET".to_string(),
                    path: "/test".to_string(),
                    status: 200,
                },
                None,
            );
            sandbox.audit(entry);
        }

        let log = sandbox.get_audit_log(10);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_user_audit_log() {
        let sandbox = create_test_sandbox();

        // Add entries for different users
        for user in ["user1", "user2", "user1", "user3", "user1"] {
            let entry = sandbox.create_audit_entry(
                Some(user.to_string()),
                None,
                AuditAction::Request {
                    method: "GET".to_string(),
                    path: "/test".to_string(),
                    status: 200,
                },
                None,
            );
            sandbox.audit(entry);
        }

        let user1_log = sandbox.get_user_audit_log("user1", 10);
        assert_eq!(user1_log.len(), 3);
    }

    // ── Configuration ──────────────────────────────────────────

    #[test]
    fn test_disabled_filtering() {
        let sandbox = Sandbox::new(SandboxConfig {
            filter_requests: false,
            ..Default::default()
        });

        let result =
            sandbox.filter_request("GET", "/api/v1/../../../etc/passwd", None, &HashMap::new());
        assert!(matches!(result, FilterResult::Allowed));
    }

    #[test]
    fn test_disabled_sanitization() {
        let sandbox = Sandbox::new(SandboxConfig {
            sanitize_responses: false,
            ..Default::default()
        });

        let content = "API Key: sk-test-1234567890abcdefghijklmnop";
        let result = sandbox.sanitize_response(content);
        assert_eq!(result.content, content);
        assert!(result.redacted_patterns.is_empty());
    }

    #[test]
    fn test_custom_blocked_endpoints() {
        let sandbox = Sandbox::new(SandboxConfig {
            blocked_endpoints: vec!["/custom/blocked".to_string()],
            ..Default::default()
        });

        let result = sandbox.filter_request("GET", "/custom/blocked", None, &HashMap::new());
        assert!(matches!(result, FilterResult::Blocked { .. }));
    }
}
