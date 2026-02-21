//! Security sandbox for Zero Gateway.
//!
//! Provides request/response filtering, sensitive data detection, and audit logging.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        }
    }
}

/// Security sandbox for request/response filtering.
pub struct Sandbox {
    config: SandboxConfig,
    sensitive_patterns: Vec<SensitivePattern>,
    blocked_patterns: Vec<Regex>,
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
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
            // API Keys
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
            // AWS
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
            // Generic secrets
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
            // JWT (don't redact entirely, just the signature)
            SensitivePattern::new(
                "jwt_token",
                r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]+",
                "[REDACTED_JWT]",
            )
            .unwrap(),
            // Private keys
            SensitivePattern::new(
                "private_key",
                r"-----BEGIN[A-Z ]*PRIVATE KEY-----",
                "[REDACTED_PRIVATE_KEY]",
            )
            .unwrap(),
            // Credit card numbers (basic pattern)
            SensitivePattern::new(
                "credit_card",
                r"\b(?:\d{4}[- ]?){3}\d{4}\b",
                "[REDACTED_CC]",
            )
            .unwrap(),
            // Email addresses (for PII protection in logs)
            // Note: Only redact in sensitive contexts, not all emails
        ];

        let blocked_patterns = config
            .blocked_path_patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self {
            config,
            sensitive_patterns,
            blocked_patterns,
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
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

        let mut log = self
            .audit_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Trim old entries if needed
        while log.len() >= self.config.max_audit_entries {
            log.remove(0);
        }

        // Log to tracing
        tracing::info!(
            user_id = ?entry.user_id,
            action = ?entry.action,
            request_id = ?entry.request_id,
            "Audit log entry"
        );

        log.push(entry);
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
        let log = self
            .audit_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        log.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get audit entries for a specific user.
    pub fn get_user_audit_log(&self, user_id: &str, limit: usize) -> Vec<AuditEntry> {
        let log = self
            .audit_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        log.iter()
            .rev()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clear audit log (for testing).
    #[cfg(test)]
    pub fn clear_audit_log(&self) {
        let mut log = self
            .audit_log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        log.clear();
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
