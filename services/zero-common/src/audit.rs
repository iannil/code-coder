//! Audit logging for compliance and regulatory requirements.
//!
//! Provides:
//! - Structured audit logging with tamper-evident hashes
//! - Configurable retention policies (5 years for financial)
//! - Compliance report generation
//! - Prompt/response archival with content hashing
//!
//! ## Compliance Standards
//!
//! - Financial: 5-year retention requirement
//! - Healthcare (HIPAA): 6-year retention
//! - General: Configurable retention period

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

/// Audit event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// User prompt submitted
    PromptSubmitted,
    /// AI response generated
    ResponseGenerated,
    /// Tool invoked
    ToolInvoked,
    /// Decision made
    DecisionMade,
    /// Configuration changed
    ConfigChanged,
    /// User authenticated
    UserAuthenticated,
    /// Session started
    SessionStarted,
    /// Session ended
    SessionEnded,
    /// Data exported
    DataExported,
    /// Error occurred
    ErrorOccurred,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PromptSubmitted => write!(f, "prompt_submitted"),
            Self::ResponseGenerated => write!(f, "response_generated"),
            Self::ToolInvoked => write!(f, "tool_invoked"),
            Self::DecisionMade => write!(f, "decision_made"),
            Self::ConfigChanged => write!(f, "config_changed"),
            Self::UserAuthenticated => write!(f, "user_authenticated"),
            Self::SessionStarted => write!(f, "session_started"),
            Self::SessionEnded => write!(f, "session_ended"),
            Self::DataExported => write!(f, "data_exported"),
            Self::ErrorOccurred => write!(f, "error_occurred"),
        }
    }
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Timestamp (RFC3339)
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// User ID (or "system")
    pub user_id: String,
    /// Session ID
    pub session_id: Option<String>,
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// Event description
    pub description: String,
    /// Content hash (SHA-256 of prompt/response)
    pub content_hash: Option<String>,
    /// Model used (if applicable)
    pub model: Option<String>,
    /// Token count (if applicable)
    pub tokens: Option<TokenInfo>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Hash of previous entry (for chain integrity)
    pub prev_hash: Option<String>,
    /// Hash of this entry
    pub entry_hash: String,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

/// Audit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Retention period in years
    #[serde(default = "default_retention_years")]
    pub retention_years: u32,
    /// Log directory path
    #[serde(default)]
    pub log_path: Option<String>,
    /// Enable content hashing
    #[serde(default = "default_true")]
    pub hash_content: bool,
    /// Enable chain integrity (hash linking)
    #[serde(default = "default_true")]
    pub chain_integrity: bool,
    /// Events to log (empty = all)
    #[serde(default)]
    pub include_events: Vec<AuditEventType>,
    /// Events to exclude
    #[serde(default)]
    pub exclude_events: Vec<AuditEventType>,
    /// Compliance standard (affects retention)
    #[serde(default)]
    pub compliance_standard: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_retention_years() -> u32 {
    5
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_years: 5,
            log_path: None,
            hash_content: true,
            chain_integrity: true,
            include_events: vec![],
            exclude_events: vec![],
            compliance_standard: Some("financial".to_string()),
        }
    }
}

/// Compliance report summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Report ID
    pub id: String,
    /// Report generation time
    pub generated_at: DateTime<Utc>,
    /// Reporting period start
    pub period_start: DateTime<Utc>,
    /// Reporting period end
    pub period_end: DateTime<Utc>,
    /// Total audit entries
    pub total_entries: usize,
    /// Entries by event type
    pub entries_by_type: HashMap<String, usize>,
    /// Entries by user
    pub entries_by_user: HashMap<String, usize>,
    /// Chain integrity status
    pub chain_integrity_valid: bool,
    /// Broken chain entries (if any)
    pub broken_chain_entries: Vec<String>,
    /// Compliance standard
    pub compliance_standard: String,
    /// Retention policy status
    pub retention_compliant: bool,
    /// Oldest entry date
    pub oldest_entry: Option<DateTime<Utc>>,
    /// Summary statistics
    pub statistics: AuditStatistics,
}

/// Audit statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_prompts: usize,
    pub total_responses: usize,
    pub total_tool_invocations: usize,
    pub total_tokens_input: u64,
    pub total_tokens_output: u64,
    pub unique_users: usize,
    pub unique_sessions: usize,
    pub error_count: usize,
}

// ============================================================================
// Audit Logger
// ============================================================================

/// Audit logger with file persistence and chain integrity.
pub struct AuditLogger {
    config: AuditConfig,
    log_dir: PathBuf,
    last_hash: Option<String>,
}

impl AuditLogger {
    /// Create a new audit logger.
    pub fn new(config: AuditConfig) -> Self {
        let log_dir = config
            .log_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::config::config_dir().join("audit"));

        // Ensure directory exists
        if let Err(e) = fs::create_dir_all(&log_dir) {
            tracing::warn!("Failed to create audit directory: {}", e);
        }

        Self {
            config,
            log_dir,
            last_hash: None,
        }
    }

    /// Log an audit event.
    pub fn log(&mut self, event: AuditEventBuilder) -> Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        // Check if event type should be logged
        if !self.config.include_events.is_empty()
            && !self.config.include_events.contains(&event.event_type)
        {
            return Ok(String::new());
        }
        if self.config.exclude_events.contains(&event.event_type) {
            return Ok(String::new());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        // Calculate content hash if needed
        let content_hash = if self.config.hash_content {
            event.content.as_ref().map(|c| self.hash_content(c))
        } else {
            None
        };

        // Create entry
        let mut entry = AuditEntry {
            id: id.clone(),
            timestamp,
            event_type: event.event_type,
            user_id: event.user_id,
            session_id: event.session_id,
            request_id: event.request_id,
            description: event.description,
            content_hash,
            model: event.model,
            tokens: event.tokens,
            metadata: event.metadata,
            prev_hash: if self.config.chain_integrity {
                self.last_hash.clone()
            } else {
                None
            },
            entry_hash: String::new(),
        };

        // Calculate entry hash
        entry.entry_hash = self.calculate_entry_hash(&entry);

        // Update last hash for chain
        if self.config.chain_integrity {
            self.last_hash = Some(entry.entry_hash.clone());
        }

        // Write to log file
        self.write_entry(&entry)?;

        tracing::debug!(
            event_type = %entry.event_type,
            user_id = %entry.user_id,
            "Audit event logged"
        );

        Ok(id)
    }

    /// Hash content using SHA-256.
    fn hash_content(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculate entry hash for chain integrity.
    fn calculate_entry_hash(&self, entry: &AuditEntry) -> String {
        let mut hasher = Sha256::new();
        hasher.update(entry.id.as_bytes());
        hasher.update(entry.timestamp.to_rfc3339().as_bytes());
        hasher.update(entry.event_type.to_string().as_bytes());
        hasher.update(entry.user_id.as_bytes());
        hasher.update(entry.description.as_bytes());
        if let Some(ref hash) = entry.content_hash {
            hasher.update(hash.as_bytes());
        }
        if let Some(ref prev) = entry.prev_hash {
            hasher.update(prev.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Write entry to daily log file.
    fn write_entry(&self, entry: &AuditEntry) -> Result<()> {
        let date = entry.timestamp.format("%Y-%m-%d").to_string();
        let filename = format!("audit-{}.jsonl", date);
        let filepath = self.log_dir.join(&filename);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filepath)
            .with_context(|| format!("Failed to open audit log: {}", filepath.display()))?;

        let line = serde_json::to_string(entry)?;
        writeln!(file, "{}", line)?;

        Ok(())
    }

    /// Load last hash for chain continuity.
    pub fn load_last_hash(&mut self) -> Result<()> {
        if !self.config.chain_integrity {
            return Ok(());
        }

        // Find most recent log file
        let entries = fs::read_dir(&self.log_dir)?;
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("audit-")
            })
            .collect();

        files.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

        if let Some(latest) = files.first() {
            let file = File::open(latest.path())?;
            let reader = BufReader::new(file);

            // Read last line
            if let Some(Ok(line)) = reader.lines().last() {
                if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                    self.last_hash = Some(entry.entry_hash);
                }
            }
        }

        Ok(())
    }

    /// Generate compliance report for a period.
    pub fn generate_report(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ComplianceReport> {
        let mut entries: Vec<AuditEntry> = Vec::new();

        // Load entries from date range
        let mut current = start;
        while current <= end {
            let date = current.format("%Y-%m-%d").to_string();
            let filename = format!("audit-{}.jsonl", date);
            let filepath = self.log_dir.join(&filename);

            if filepath.exists() {
                let file = File::open(&filepath)?;
                let reader = BufReader::new(file);

                for line in reader.lines().map_while(Result::ok) {
                    if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                        if entry.timestamp >= start && entry.timestamp <= end {
                            entries.push(entry);
                        }
                    }
                }
            }

            current += Duration::days(1);
        }

        // Calculate statistics
        let mut entries_by_type: HashMap<String, usize> = HashMap::new();
        let mut entries_by_user: HashMap<String, usize> = HashMap::new();
        let mut sessions: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut stats = AuditStatistics::default();

        let mut prev_hash: Option<String> = None;
        let mut broken_chain: Vec<String> = Vec::new();

        for entry in &entries {
            // Count by type
            *entries_by_type
                .entry(entry.event_type.to_string())
                .or_insert(0) += 1;

            // Count by user
            *entries_by_user.entry(entry.user_id.clone()).or_insert(0) += 1;

            // Track sessions
            if let Some(ref sid) = entry.session_id {
                sessions.insert(sid.clone());
            }

            // Update stats
            match entry.event_type {
                AuditEventType::PromptSubmitted => stats.total_prompts += 1,
                AuditEventType::ResponseGenerated => stats.total_responses += 1,
                AuditEventType::ToolInvoked => stats.total_tool_invocations += 1,
                AuditEventType::ErrorOccurred => stats.error_count += 1,
                _ => {}
            }

            if let Some(ref tokens) = entry.tokens {
                stats.total_tokens_input += tokens.input as u64;
                stats.total_tokens_output += tokens.output as u64;
            }

            // Verify chain integrity
            if self.config.chain_integrity {
                if entry.prev_hash != prev_hash {
                    broken_chain.push(entry.id.clone());
                }
                prev_hash = Some(entry.entry_hash.clone());
            }
        }

        stats.unique_users = entries_by_user.len();
        stats.unique_sessions = sessions.len();

        let oldest_entry = entries.first().map(|e| e.timestamp);
        let retention_compliant = oldest_entry
            .map(|oldest| {
                let retention_end = Utc::now() - Duration::days(365 * self.config.retention_years as i64);
                oldest >= retention_end
            })
            .unwrap_or(true);

        Ok(ComplianceReport {
            id: uuid::Uuid::new_v4().to_string(),
            generated_at: Utc::now(),
            period_start: start,
            period_end: end,
            total_entries: entries.len(),
            entries_by_type,
            entries_by_user,
            chain_integrity_valid: broken_chain.is_empty(),
            broken_chain_entries: broken_chain,
            compliance_standard: self
                .config
                .compliance_standard
                .clone()
                .unwrap_or_else(|| "general".to_string()),
            retention_compliant,
            oldest_entry,
            statistics: stats,
        })
    }

    /// Clean up old audit logs based on retention policy.
    pub fn cleanup_old_logs(&self) -> Result<usize> {
        let retention_cutoff =
            Utc::now() - Duration::days(365 * self.config.retention_years as i64);

        let mut removed = 0;

        for entry in fs::read_dir(&self.log_dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();

            if filename.starts_with("audit-") && filename.ends_with(".jsonl") {
                // Extract date from filename
                if let Some(date_str) = filename
                    .strip_prefix("audit-")
                    .and_then(|s| s.strip_suffix(".jsonl"))
                {
                    if let Ok(file_date) =
                        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    {
                        let file_datetime = file_date.and_hms_opt(0, 0, 0).unwrap();
                        let file_utc = DateTime::<Utc>::from_naive_utc_and_offset(file_datetime, Utc);

                        if file_utc < retention_cutoff {
                            // Archive before deletion (optional)
                            let archive_dir = self.log_dir.join("archive");
                            fs::create_dir_all(&archive_dir)?;
                            let archive_path = archive_dir.join(&filename);

                            // Move to archive instead of deleting
                            fs::rename(entry.path(), &archive_path)?;
                            removed += 1;

                            tracing::info!(
                                filename = %filename,
                                "Archived old audit log"
                            );
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

// ============================================================================
// Builder
// ============================================================================

/// Builder for audit events.
pub struct AuditEventBuilder {
    event_type: AuditEventType,
    user_id: String,
    description: String,
    session_id: Option<String>,
    request_id: Option<String>,
    content: Option<String>,
    model: Option<String>,
    tokens: Option<TokenInfo>,
    metadata: HashMap<String, serde_json::Value>,
}

impl AuditEventBuilder {
    /// Create a new audit event builder.
    pub fn new(event_type: AuditEventType, user_id: impl Into<String>) -> Self {
        Self {
            event_type,
            user_id: user_id.into(),
            description: String::new(),
            session_id: None,
            request_id: None,
            content: None,
            model: None,
            tokens: None,
            metadata: HashMap::new(),
        }
    }

    /// Set event description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set session ID.
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Set request ID.
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set content for hashing.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set model used.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set token usage.
    pub fn tokens(mut self, input: u32, output: u32) -> Self {
        self.tokens = Some(TokenInfo {
            input,
            output,
            total: input + output,
        });
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), v);
        }
        self
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Log a prompt submission.
pub fn log_prompt(
    logger: &mut AuditLogger,
    user_id: &str,
    session_id: &str,
    prompt: &str,
    model: Option<&str>,
) -> Result<String> {
    let mut builder = AuditEventBuilder::new(AuditEventType::PromptSubmitted, user_id)
        .description("User submitted prompt")
        .session_id(session_id)
        .content(prompt);

    if let Some(m) = model {
        builder = builder.model(m);
    }

    logger.log(builder)
}

/// Log a response generation.
pub fn log_response(
    logger: &mut AuditLogger,
    user_id: &str,
    session_id: &str,
    response: &str,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> Result<String> {
    logger.log(
        AuditEventBuilder::new(AuditEventType::ResponseGenerated, user_id)
            .description("AI response generated")
            .session_id(session_id)
            .content(response)
            .model(model)
            .tokens(input_tokens, output_tokens),
    )
}

/// Log a tool invocation.
pub fn log_tool(
    logger: &mut AuditLogger,
    user_id: &str,
    session_id: &str,
    tool_name: &str,
    parameters: &serde_json::Value,
) -> Result<String> {
    logger.log(
        AuditEventBuilder::new(AuditEventType::ToolInvoked, user_id)
            .description(format!("Tool invoked: {}", tool_name))
            .session_id(session_id)
            .metadata("tool_name", tool_name)
            .metadata("parameters", parameters),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_logger(tmp: &TempDir) -> AuditLogger {
        let config = AuditConfig {
            log_path: Some(tmp.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        AuditLogger::new(config)
    }

    #[test]
    fn test_log_event() {
        let tmp = TempDir::new().unwrap();
        let mut logger = test_logger(&tmp);

        let id = logger
            .log(
                AuditEventBuilder::new(AuditEventType::PromptSubmitted, "user1")
                    .description("Test prompt")
                    .content("Hello world"),
            )
            .unwrap();

        assert!(!id.is_empty());
    }

    #[test]
    fn test_chain_integrity() {
        let tmp = TempDir::new().unwrap();
        let mut logger = test_logger(&tmp);

        // Log multiple events
        logger
            .log(AuditEventBuilder::new(
                AuditEventType::SessionStarted,
                "user1",
            ))
            .unwrap();
        logger
            .log(AuditEventBuilder::new(
                AuditEventType::PromptSubmitted,
                "user1",
            ))
            .unwrap();
        logger
            .log(
                AuditEventBuilder::new(AuditEventType::SessionEnded, "user1")
                    .description("Session completed"),
            )
            .unwrap();

        // Last hash should be set
        assert!(logger.last_hash.is_some());
    }

    #[test]
    fn test_content_hashing() {
        let tmp = TempDir::new().unwrap();
        let logger = test_logger(&tmp);

        let hash1 = logger.hash_content("Hello world");
        let hash2 = logger.hash_content("Hello world");
        let hash3 = logger.hash_content("Different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_disabled_logging() {
        let tmp = TempDir::new().unwrap();
        let mut logger = AuditLogger::new(AuditConfig {
            enabled: false,
            log_path: Some(tmp.path().to_string_lossy().to_string()),
            ..Default::default()
        });

        let id = logger
            .log(AuditEventBuilder::new(
                AuditEventType::PromptSubmitted,
                "user1",
            ))
            .unwrap();

        assert!(id.is_empty());
    }
}
