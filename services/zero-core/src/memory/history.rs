//! History module - edit and decision history tracking
//!
//! Provides persistent storage for:
//! - **EditRecord**: File edit history with additions/deletions
//! - **EditSession**: Groups of related edits with token/duration stats
//! - **DecisionRecord**: Implementation decisions with rationale
//! - **ArchitectureDecisionRecord (ADR)**: Formal architecture decisions

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// ============================================================================
// Edit History Types
// ============================================================================

/// Type of file edit operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileEditType {
    Create,
    Update,
    Delete,
    Move,
}

impl std::fmt::Display for FileEditType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
            Self::Move => write!(f, "move"),
        }
    }
}

impl std::str::FromStr for FileEditType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "create" => Ok(Self::Create),
            "update" => Ok(Self::Update),
            "delete" => Ok(Self::Delete),
            "move" => Ok(Self::Move),
            _ => Err(anyhow::anyhow!("Invalid file edit type: {}", s)),
        }
    }
}

/// A single file edit within an edit record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEdit {
    pub path: String,
    #[serde(rename = "type")]
    pub edit_type: FileEditType,
    pub additions: usize,
    pub deletions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_hash: Option<String>,
}

/// A record of one or more file edits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRecord {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub edits: Vec<FileEdit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

impl EditRecord {
    /// Create a new edit record with generated ID and timestamp
    pub fn new(edits: Vec<FileEdit>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: format!("edit_{}_{}", now, random_suffix()),
            session_id: None,
            timestamp: now,
            description: None,
            edits,
            agent: None,
            model: None,
            tokens_used: None,
            duration: None,
        }
    }

    /// Set the session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the agent name
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    /// Set the model name
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set tokens used
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    /// Set duration in milliseconds
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self
    }
}

/// A group of related edits with aggregate stats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSession {
    pub id: String,
    pub project_id: String,
    pub start_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    pub edits: Vec<String>,
    pub total_tokens: usize,
    pub total_duration: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl EditSession {
    /// Create a new edit session
    pub fn new(project_id: impl Into<String>) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: format!("session_{}_{}", now, random_suffix()),
            project_id: project_id.into(),
            start_time: now,
            end_time: None,
            edits: Vec::new(),
            total_tokens: 0,
            total_duration: 0,
            description: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add an edit to the session
    pub fn add_edit(&mut self, edit_id: String, tokens: Option<usize>, duration: Option<u64>) {
        self.edits.push(edit_id);
        if let Some(t) = tokens {
            self.total_tokens += t;
        }
        if let Some(d) = duration {
            self.total_duration += d;
        }
    }

    /// End the session
    pub fn end(&mut self) {
        self.end_time = Some(Utc::now().timestamp_millis());
    }
}

// ============================================================================
// Decision History Types
// ============================================================================

/// Status of an architecture decision record
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
    Rejected,
}

impl std::fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proposed => write!(f, "proposed"),
            Self::Accepted => write!(f, "accepted"),
            Self::Deprecated => write!(f, "deprecated"),
            Self::Superseded => write!(f, "superseded"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for AdrStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "deprecated" => Ok(Self::Deprecated),
            "superseded" => Ok(Self::Superseded),
            "rejected" => Ok(Self::Rejected),
            _ => Err(anyhow::anyhow!("Invalid ADR status: {}", s)),
        }
    }
}

/// Type of decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionType {
    Architecture,
    Implementation,
    Refactor,
    Bugfix,
    Feature,
    Other,
}

impl std::fmt::Display for DecisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Architecture => write!(f, "architecture"),
            Self::Implementation => write!(f, "implementation"),
            Self::Refactor => write!(f, "refactor"),
            Self::Bugfix => write!(f, "bugfix"),
            Self::Feature => write!(f, "feature"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for DecisionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "architecture" => Ok(Self::Architecture),
            "implementation" => Ok(Self::Implementation),
            "refactor" => Ok(Self::Refactor),
            "bugfix" => Ok(Self::Bugfix),
            "feature" => Ok(Self::Feature),
            "other" => Ok(Self::Other),
            _ => Err(anyhow::anyhow!("Invalid decision type: {}", s)),
        }
    }
}

/// An alternative considered for a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub description: String,
    pub rejected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A record of a decision made during development
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub decision_type: DecisionType,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub timestamp: i64,
}

impl DecisionRecord {
    /// Create a new decision record
    pub fn new(
        decision_type: DecisionType,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: format!("decision_{}_{}", now, random_suffix()),
            decision_type,
            title: title.into(),
            description: description.into(),
            rationale: None,
            alternatives: None,
            outcome: None,
            session_id: None,
            files: None,
            tags: None,
            timestamp: now,
        }
    }

    /// Set the rationale
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Set the alternatives
    pub fn with_alternatives(mut self, alternatives: Vec<String>) -> Self {
        self.alternatives = Some(alternatives);
        self
    }

    /// Set the outcome
    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.outcome = Some(outcome.into());
        self
    }

    /// Set the session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the affected files
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = Some(files);
        self
    }

    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
}

/// A formal Architecture Decision Record (ADR)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchitectureDecisionRecord {
    pub id: String,
    pub title: String,
    pub status: AdrStatus,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<Alternative>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub created: i64,
    pub updated: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl ArchitectureDecisionRecord {
    /// Create a new ADR
    pub fn new(
        title: impl Into<String>,
        context: impl Into<String>,
        decision: impl Into<String>,
        consequences: Vec<String>,
    ) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: format!("adr_{}_{}", now, random_suffix()),
            title: title.into(),
            status: AdrStatus::Proposed,
            context: context.into(),
            decision: decision.into(),
            consequences,
            alternatives: None,
            superseded_by: None,
            created: now,
            updated: now,
            tags: None,
        }
    }

    /// Set the status
    pub fn with_status(mut self, status: AdrStatus) -> Self {
        self.status = status;
        self
    }

    /// Set alternatives
    pub fn with_alternatives(mut self, alternatives: Vec<Alternative>) -> Self {
        self.alternatives = Some(alternatives);
        self
    }

    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Format as markdown
    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            format!("# {}", self.title),
            String::new(),
            format!("**Status:** {}", self.status),
            format!(
                "**Date:** {}",
                DateTime::from_timestamp_millis(self.created)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()
            ),
        ];

        if let Some(tags) = &self.tags {
            if !tags.is_empty() {
                lines.push(format!("**Tags:** {}", tags.join(", ")));
            }
        }

        lines.push(String::new());
        lines.push("## Context".to_string());
        lines.push(self.context.clone());
        lines.push(String::new());

        lines.push("## Decision".to_string());
        lines.push(self.decision.clone());
        lines.push(String::new());

        if !self.consequences.is_empty() {
            lines.push("## Consequences".to_string());
            for consequence in &self.consequences {
                lines.push(format!("- {}", consequence));
            }
            lines.push(String::new());
        }

        if let Some(alternatives) = &self.alternatives {
            if !alternatives.is_empty() {
                lines.push("## Alternatives Considered".to_string());
                for alt in alternatives {
                    let suffix = if alt.rejected { " (rejected)" } else { "" };
                    lines.push(format!("- {}{}", alt.description, suffix));
                    if let Some(reason) = &alt.reason {
                        lines.push(format!("  - Reason: {}", reason));
                    }
                }
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }
}

// ============================================================================
// History Store
// ============================================================================

/// Storage for edit and decision history
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// Open or create a history store
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open history store: {}", path.display()))?;

        // Initialize schema
        conn.execute_batch(
            r#"
            -- Edit records
            CREATE TABLE IF NOT EXISTS edit_records (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                session_id TEXT,
                timestamp INTEGER NOT NULL,
                description TEXT,
                edits TEXT NOT NULL,
                agent TEXT,
                model TEXT,
                tokens_used INTEGER,
                duration INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_edit_records_project ON edit_records(project_id);
            CREATE INDEX IF NOT EXISTS idx_edit_records_session ON edit_records(session_id);
            CREATE INDEX IF NOT EXISTS idx_edit_records_timestamp ON edit_records(timestamp DESC);

            -- Edit sessions
            CREATE TABLE IF NOT EXISTS edit_sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                edits TEXT NOT NULL,
                total_tokens INTEGER NOT NULL,
                total_duration INTEGER NOT NULL,
                description TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_edit_sessions_project ON edit_sessions(project_id);

            -- Decision records
            CREATE TABLE IF NOT EXISTS decision_records (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                decision_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                rationale TEXT,
                alternatives TEXT,
                outcome TEXT,
                session_id TEXT,
                files TEXT,
                tags TEXT,
                timestamp INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_decision_records_project ON decision_records(project_id);
            CREATE INDEX IF NOT EXISTS idx_decision_records_type ON decision_records(decision_type);
            CREATE INDEX IF NOT EXISTS idx_decision_records_timestamp ON decision_records(timestamp DESC);

            -- Architecture Decision Records
            CREATE TABLE IF NOT EXISTS adrs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                context TEXT NOT NULL,
                decision TEXT NOT NULL,
                consequences TEXT NOT NULL,
                alternatives TEXT,
                superseded_by TEXT,
                created INTEGER NOT NULL,
                updated INTEGER NOT NULL,
                tags TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_adrs_project ON adrs(project_id);
            CREATE INDEX IF NOT EXISTS idx_adrs_status ON adrs(status);

            -- Summary statistics
            CREATE TABLE IF NOT EXISTS history_summary (
                project_id TEXT PRIMARY KEY,
                total_edits INTEGER DEFAULT 0,
                total_sessions INTEGER DEFAULT 0,
                total_decisions INTEGER DEFAULT 0,
                total_adrs INTEGER DEFAULT 0,
                updated INTEGER NOT NULL
            );
            "#,
        )
        .with_context(|| "Failed to initialize history store schema")?;

        Ok(Self { conn })
    }

    /// Open an in-memory history store (for testing)
    pub fn in_memory() -> Result<Self> {
        Self::open(Path::new(":memory:"))
    }

    // ========================================================================
    // Edit Records
    // ========================================================================

    /// Save an edit record
    pub fn save_edit_record(&self, project_id: &str, record: &EditRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO edit_records
            (id, project_id, session_id, timestamp, description, edits, agent, model, tokens_used, duration)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                &record.id,
                project_id,
                &record.session_id,
                record.timestamp,
                &record.description,
                serde_json::to_string(&record.edits)?,
                &record.agent,
                &record.model,
                record.tokens_used.map(|t| t as i64),
                record.duration.map(|d| d as i64),
            ],
        )?;

        // Update summary
        self.conn.execute(
            r#"
            INSERT INTO history_summary (project_id, total_edits, updated)
            VALUES (?1, 1, ?2)
            ON CONFLICT(project_id) DO UPDATE SET
                total_edits = total_edits + 1,
                updated = ?2
            "#,
            params![project_id, Utc::now().timestamp_millis()],
        )?;

        Ok(())
    }

    /// Get an edit record by ID
    pub fn get_edit_record(&self, project_id: &str, id: &str) -> Result<Option<EditRecord>> {
        self.conn
            .query_row(
                "SELECT id, session_id, timestamp, description, edits, agent, model, tokens_used, duration
                 FROM edit_records WHERE project_id = ?1 AND id = ?2",
                params![project_id, id],
                |row| {
                    let edits_json: String = row.get(4)?;
                    let edits: Vec<FileEdit> = serde_json::from_str(&edits_json).unwrap_or_default();

                    Ok(EditRecord {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        timestamp: row.get(2)?,
                        description: row.get(3)?,
                        edits,
                        agent: row.get(5)?,
                        model: row.get(6)?,
                        tokens_used: row.get::<_, Option<i64>>(7)?.map(|t| t as usize),
                        duration: row.get::<_, Option<i64>>(8)?.map(|d| d as u64),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get recent edit records
    pub fn get_recent_edits(&self, project_id: &str, limit: usize) -> Result<Vec<EditRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, timestamp, description, edits, agent, model, tokens_used, duration
             FROM edit_records WHERE project_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;

        let records = stmt.query_map(params![project_id, limit as i64], |row| {
            let edits_json: String = row.get(4)?;
            let edits: Vec<FileEdit> = serde_json::from_str(&edits_json).unwrap_or_default();

            Ok(EditRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                description: row.get(3)?,
                edits,
                agent: row.get(5)?,
                model: row.get(6)?,
                tokens_used: row.get::<_, Option<i64>>(7)?.map(|t| t as usize),
                duration: row.get::<_, Option<i64>>(8)?.map(|d| d as u64),
            })
        })?;

        records.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get edit records by session ID
    pub fn get_edits_by_session(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<Vec<EditRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, timestamp, description, edits, agent, model, tokens_used, duration
             FROM edit_records WHERE project_id = ?1 AND session_id = ?2 ORDER BY timestamp ASC",
        )?;

        let records = stmt.query_map(params![project_id, session_id], |row| {
            let edits_json: String = row.get(4)?;
            let edits: Vec<FileEdit> = serde_json::from_str(&edits_json).unwrap_or_default();

            Ok(EditRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                description: row.get(3)?,
                edits,
                agent: row.get(5)?,
                model: row.get(6)?,
                tokens_used: row.get::<_, Option<i64>>(7)?.map(|t| t as usize),
                duration: row.get::<_, Option<i64>>(8)?.map(|d| d as u64),
            })
        })?;

        records.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get edit records affecting a specific file
    pub fn get_edits_by_file(&self, project_id: &str, file_path: &str) -> Result<Vec<EditRecord>> {
        // We need to search within the JSON edits array
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, timestamp, description, edits, agent, model, tokens_used, duration
             FROM edit_records WHERE project_id = ?1 AND edits LIKE ?2 ORDER BY timestamp DESC",
        )?;

        let pattern = format!("%\"path\":\"{}%", file_path.replace('\\', "/"));

        let records = stmt.query_map(params![project_id, pattern], |row| {
            let edits_json: String = row.get(4)?;
            let edits: Vec<FileEdit> = serde_json::from_str(&edits_json).unwrap_or_default();

            Ok(EditRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                description: row.get(3)?,
                edits,
                agent: row.get(5)?,
                model: row.get(6)?,
                tokens_used: row.get::<_, Option<i64>>(7)?.map(|t| t as usize),
                duration: row.get::<_, Option<i64>>(8)?.map(|d| d as u64),
            })
        })?;

        // Filter to only records that actually contain the file
        let records: Vec<EditRecord> = records
            .filter_map(|r| r.ok())
            .filter(|r| {
                r.edits
                    .iter()
                    .any(|e| e.path == file_path || e.path.ends_with(file_path))
            })
            .collect();

        Ok(records)
    }

    // ========================================================================
    // Edit Sessions
    // ========================================================================

    /// Save an edit session
    pub fn save_edit_session(&self, session: &EditSession) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO edit_sessions
            (id, project_id, start_time, end_time, edits, total_tokens, total_duration, description)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                &session.id,
                &session.project_id,
                session.start_time,
                session.end_time,
                serde_json::to_string(&session.edits)?,
                session.total_tokens as i64,
                session.total_duration as i64,
                &session.description,
            ],
        )?;
        Ok(())
    }

    /// Get an edit session by ID
    pub fn get_edit_session(&self, project_id: &str, id: &str) -> Result<Option<EditSession>> {
        self.conn
            .query_row(
                "SELECT id, project_id, start_time, end_time, edits, total_tokens, total_duration, description
                 FROM edit_sessions WHERE project_id = ?1 AND id = ?2",
                params![project_id, id],
                |row| {
                    let edits_json: String = row.get(4)?;
                    let edits: Vec<String> = serde_json::from_str(&edits_json).unwrap_or_default();

                    Ok(EditSession {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        start_time: row.get(2)?,
                        end_time: row.get(3)?,
                        edits,
                        total_tokens: row.get::<_, i64>(5)? as usize,
                        total_duration: row.get::<_, i64>(6)? as u64,
                        description: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get all edit sessions for a project
    pub fn get_all_sessions(&self, project_id: &str) -> Result<Vec<EditSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, start_time, end_time, edits, total_tokens, total_duration, description
             FROM edit_sessions WHERE project_id = ?1 ORDER BY start_time DESC",
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            let edits_json: String = row.get(4)?;
            let edits: Vec<String> = serde_json::from_str(&edits_json).unwrap_or_default();

            Ok(EditSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                edits,
                total_tokens: row.get::<_, i64>(5)? as usize,
                total_duration: row.get::<_, i64>(6)? as u64,
                description: row.get(7)?,
            })
        })?;

        sessions.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get active (not ended) sessions
    pub fn get_active_sessions(&self, project_id: &str) -> Result<Vec<EditSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, start_time, end_time, edits, total_tokens, total_duration, description
             FROM edit_sessions WHERE project_id = ?1 AND end_time IS NULL ORDER BY start_time DESC",
        )?;

        let sessions = stmt.query_map(params![project_id], |row| {
            let edits_json: String = row.get(4)?;
            let edits: Vec<String> = serde_json::from_str(&edits_json).unwrap_or_default();

            Ok(EditSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                edits,
                total_tokens: row.get::<_, i64>(5)? as usize,
                total_duration: row.get::<_, i64>(6)? as u64,
                description: row.get(7)?,
            })
        })?;

        sessions.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // ========================================================================
    // Decision Records
    // ========================================================================

    /// Save a decision record
    pub fn save_decision(&self, project_id: &str, decision: &DecisionRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO decision_records
            (id, project_id, decision_type, title, description, rationale, alternatives, outcome, session_id, files, tags, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                &decision.id,
                project_id,
                decision.decision_type.to_string(),
                &decision.title,
                &decision.description,
                &decision.rationale,
                decision.alternatives.as_ref().map(|a| serde_json::to_string(a).ok()).flatten(),
                &decision.outcome,
                &decision.session_id,
                decision.files.as_ref().map(|f| serde_json::to_string(f).ok()).flatten(),
                decision.tags.as_ref().map(|t| serde_json::to_string(t).ok()).flatten(),
                decision.timestamp,
            ],
        )?;

        // Update summary
        self.conn.execute(
            r#"
            INSERT INTO history_summary (project_id, total_decisions, updated)
            VALUES (?1, 1, ?2)
            ON CONFLICT(project_id) DO UPDATE SET
                total_decisions = total_decisions + 1,
                updated = ?2
            "#,
            params![project_id, Utc::now().timestamp_millis()],
        )?;

        Ok(())
    }

    /// Get a decision record by ID
    pub fn get_decision(&self, project_id: &str, id: &str) -> Result<Option<DecisionRecord>> {
        self.conn
            .query_row(
                "SELECT id, decision_type, title, description, rationale, alternatives, outcome, session_id, files, tags, timestamp
                 FROM decision_records WHERE project_id = ?1 AND id = ?2",
                params![project_id, id],
                |row| {
                    let decision_type: String = row.get(1)?;
                    let alternatives_json: Option<String> = row.get(5)?;
                    let files_json: Option<String> = row.get(8)?;
                    let tags_json: Option<String> = row.get(9)?;

                    Ok(DecisionRecord {
                        id: row.get(0)?,
                        decision_type: decision_type.parse().unwrap_or(DecisionType::Other),
                        title: row.get(2)?,
                        description: row.get(3)?,
                        rationale: row.get(4)?,
                        alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                        outcome: row.get(6)?,
                        session_id: row.get(7)?,
                        files: files_json.and_then(|j| serde_json::from_str(&j).ok()),
                        tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
                        timestamp: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get recent decisions
    pub fn get_recent_decisions(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<DecisionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, decision_type, title, description, rationale, alternatives, outcome, session_id, files, tags, timestamp
             FROM decision_records WHERE project_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;

        let records = stmt.query_map(params![project_id, limit as i64], |row| {
            let decision_type: String = row.get(1)?;
            let alternatives_json: Option<String> = row.get(5)?;
            let files_json: Option<String> = row.get(8)?;
            let tags_json: Option<String> = row.get(9)?;

            Ok(DecisionRecord {
                id: row.get(0)?,
                decision_type: decision_type.parse().unwrap_or(DecisionType::Other),
                title: row.get(2)?,
                description: row.get(3)?,
                rationale: row.get(4)?,
                alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                outcome: row.get(6)?,
                session_id: row.get(7)?,
                files: files_json.and_then(|j| serde_json::from_str(&j).ok()),
                tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
                timestamp: row.get(10)?,
            })
        })?;

        records.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get decisions by type
    pub fn get_decisions_by_type(
        &self,
        project_id: &str,
        decision_type: DecisionType,
    ) -> Result<Vec<DecisionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, decision_type, title, description, rationale, alternatives, outcome, session_id, files, tags, timestamp
             FROM decision_records WHERE project_id = ?1 AND decision_type = ?2 ORDER BY timestamp DESC",
        )?;

        let records = stmt.query_map(params![project_id, decision_type.to_string()], |row| {
            let decision_type: String = row.get(1)?;
            let alternatives_json: Option<String> = row.get(5)?;
            let files_json: Option<String> = row.get(8)?;
            let tags_json: Option<String> = row.get(9)?;

            Ok(DecisionRecord {
                id: row.get(0)?,
                decision_type: decision_type.parse().unwrap_or(DecisionType::Other),
                title: row.get(2)?,
                description: row.get(3)?,
                rationale: row.get(4)?,
                alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                outcome: row.get(6)?,
                session_id: row.get(7)?,
                files: files_json.and_then(|j| serde_json::from_str(&j).ok()),
                tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
                timestamp: row.get(10)?,
            })
        })?;

        records.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Search decisions by text
    pub fn search_decisions(&self, project_id: &str, query: &str) -> Result<Vec<DecisionRecord>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, decision_type, title, description, rationale, alternatives, outcome, session_id, files, tags, timestamp
             FROM decision_records WHERE project_id = ?1 AND (title LIKE ?2 OR description LIKE ?2 OR rationale LIKE ?2)
             ORDER BY timestamp DESC",
        )?;

        let records = stmt.query_map(params![project_id, pattern], |row| {
            let decision_type: String = row.get(1)?;
            let alternatives_json: Option<String> = row.get(5)?;
            let files_json: Option<String> = row.get(8)?;
            let tags_json: Option<String> = row.get(9)?;

            Ok(DecisionRecord {
                id: row.get(0)?,
                decision_type: decision_type.parse().unwrap_or(DecisionType::Other),
                title: row.get(2)?,
                description: row.get(3)?,
                rationale: row.get(4)?,
                alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                outcome: row.get(6)?,
                session_id: row.get(7)?,
                files: files_json.and_then(|j| serde_json::from_str(&j).ok()),
                tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
                timestamp: row.get(10)?,
            })
        })?;

        records.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Delete a decision record
    pub fn delete_decision(&self, project_id: &str, id: &str) -> Result<bool> {
        let deleted = self.conn.execute(
            "DELETE FROM decision_records WHERE project_id = ?1 AND id = ?2",
            params![project_id, id],
        )?;
        Ok(deleted > 0)
    }

    // ========================================================================
    // Architecture Decision Records (ADRs)
    // ========================================================================

    /// Save an ADR
    pub fn save_adr(&self, project_id: &str, adr: &ArchitectureDecisionRecord) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO adrs
            (id, project_id, title, status, context, decision, consequences, alternatives, superseded_by, created, updated, tags)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                &adr.id,
                project_id,
                &adr.title,
                adr.status.to_string(),
                &adr.context,
                &adr.decision,
                serde_json::to_string(&adr.consequences)?,
                adr.alternatives.as_ref().map(|a| serde_json::to_string(a).ok()).flatten(),
                &adr.superseded_by,
                adr.created,
                adr.updated,
                adr.tags.as_ref().map(|t| serde_json::to_string(t).ok()).flatten(),
            ],
        )?;

        // Update summary
        self.conn.execute(
            r#"
            INSERT INTO history_summary (project_id, total_adrs, updated)
            VALUES (?1, 1, ?2)
            ON CONFLICT(project_id) DO UPDATE SET
                total_adrs = total_adrs + 1,
                updated = ?2
            "#,
            params![project_id, Utc::now().timestamp_millis()],
        )?;

        Ok(())
    }

    /// Get an ADR by ID
    pub fn get_adr(&self, project_id: &str, id: &str) -> Result<Option<ArchitectureDecisionRecord>> {
        self.conn
            .query_row(
                "SELECT id, title, status, context, decision, consequences, alternatives, superseded_by, created, updated, tags
                 FROM adrs WHERE project_id = ?1 AND id = ?2",
                params![project_id, id],
                |row| {
                    let status: String = row.get(2)?;
                    let consequences_json: String = row.get(5)?;
                    let alternatives_json: Option<String> = row.get(6)?;
                    let tags_json: Option<String> = row.get(10)?;

                    Ok(ArchitectureDecisionRecord {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        status: status.parse().unwrap_or(AdrStatus::Proposed),
                        context: row.get(3)?,
                        decision: row.get(4)?,
                        consequences: serde_json::from_str(&consequences_json).unwrap_or_default(),
                        alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                        superseded_by: row.get(7)?,
                        created: row.get(8)?,
                        updated: row.get(9)?,
                        tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get all ADRs for a project
    pub fn get_all_adrs(&self, project_id: &str) -> Result<Vec<ArchitectureDecisionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, context, decision, consequences, alternatives, superseded_by, created, updated, tags
             FROM adrs WHERE project_id = ?1 ORDER BY created DESC",
        )?;

        let adrs = stmt.query_map(params![project_id], |row| {
            let status: String = row.get(2)?;
            let consequences_json: String = row.get(5)?;
            let alternatives_json: Option<String> = row.get(6)?;
            let tags_json: Option<String> = row.get(10)?;

            Ok(ArchitectureDecisionRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                status: status.parse().unwrap_or(AdrStatus::Proposed),
                context: row.get(3)?,
                decision: row.get(4)?,
                consequences: serde_json::from_str(&consequences_json).unwrap_or_default(),
                alternatives: alternatives_json.and_then(|j| serde_json::from_str(&j).ok()),
                superseded_by: row.get(7)?,
                created: row.get(8)?,
                updated: row.get(9)?,
                tags: tags_json.and_then(|j| serde_json::from_str(&j).ok()),
            })
        })?;

        adrs.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get edit statistics for a project
    pub fn get_edit_stats(
        &self,
        project_id: &str,
    ) -> Result<EditStats> {
        let mut stmt = self.conn.prepare(
            "SELECT edits, agent, tokens_used FROM edit_records WHERE project_id = ?1",
        )?;

        let mut total_additions = 0usize;
        let mut total_deletions = 0usize;
        let mut file_edit_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut agent_stats: std::collections::HashMap<String, AgentStats> =
            std::collections::HashMap::new();

        let rows = stmt.query_map(params![project_id], |row| {
            let edits_json: String = row.get(0)?;
            let agent: Option<String> = row.get(1)?;
            let tokens: Option<i64> = row.get(2)?;
            Ok((edits_json, agent, tokens))
        })?;

        for row in rows {
            let (edits_json, agent, tokens) = row?;
            let edits: Vec<FileEdit> = serde_json::from_str(&edits_json).unwrap_or_default();

            for edit in &edits {
                total_additions += edit.additions;
                total_deletions += edit.deletions;
                *file_edit_counts.entry(edit.path.clone()).or_default() += 1;
            }

            if let Some(agent_name) = agent {
                let stats = agent_stats.entry(agent_name).or_insert(AgentStats {
                    edit_count: 0,
                    token_count: 0,
                });
                stats.edit_count += 1;
                if let Some(t) = tokens {
                    stats.token_count += t as usize;
                }
            }
        }

        let mut top_files: Vec<(String, usize)> = file_edit_counts.into_iter().collect();
        top_files.sort_by(|a, b| b.1.cmp(&a.1));
        top_files.truncate(10);

        Ok(EditStats {
            total_edits: top_files.iter().map(|(_, c)| c).sum(),
            total_additions,
            total_deletions,
            total_files: top_files.len(),
            top_files,
            agent_stats: agent_stats.into_iter().collect(),
        })
    }

    /// Clean up old history records
    pub fn cleanup(&self, project_id: &str, before_timestamp: i64) -> Result<usize> {
        let mut removed = 0usize;

        removed += self.conn.execute(
            "DELETE FROM edit_records WHERE project_id = ?1 AND timestamp < ?2",
            params![project_id, before_timestamp],
        )?;

        removed += self.conn.execute(
            "DELETE FROM edit_sessions WHERE project_id = ?1 AND start_time < ?2",
            params![project_id, before_timestamp],
        )?;

        removed += self.conn.execute(
            "DELETE FROM decision_records WHERE project_id = ?1 AND timestamp < ?2",
            params![project_id, before_timestamp],
        )?;

        Ok(removed)
    }

    /// Invalidate all history for a project
    pub fn invalidate(&self, project_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM edit_records WHERE project_id = ?1", params![project_id])?;
        self.conn
            .execute("DELETE FROM edit_sessions WHERE project_id = ?1", params![project_id])?;
        self.conn
            .execute("DELETE FROM decision_records WHERE project_id = ?1", params![project_id])?;
        self.conn
            .execute("DELETE FROM adrs WHERE project_id = ?1", params![project_id])?;
        self.conn
            .execute("DELETE FROM history_summary WHERE project_id = ?1", params![project_id])?;
        Ok(())
    }
}

/// Statistics about edits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditStats {
    pub total_edits: usize,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub total_files: usize,
    pub top_files: Vec<(String, usize)>,
    pub agent_stats: Vec<(String, AgentStats)>,
}

/// Statistics about an agent's edits
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStats {
    pub edit_count: usize,
    pub token_count: usize,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Generate a random suffix for IDs
fn random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos & 0xFFFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_record() {
        let store = HistoryStore::in_memory().unwrap();
        let project_id = "test-project";

        let record = EditRecord::new(vec![FileEdit {
            path: "src/main.rs".to_string(),
            edit_type: FileEditType::Update,
            additions: 10,
            deletions: 5,
            pre_hash: None,
            post_hash: None,
        }])
        .with_description("Fixed bug")
        .with_agent("code-reviewer");

        store.save_edit_record(project_id, &record).unwrap();

        let loaded = store.get_edit_record(project_id, &record.id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, record.id);
        assert_eq!(loaded.edits.len(), 1);
        assert_eq!(loaded.edits[0].additions, 10);
    }

    #[test]
    fn test_decision_record() {
        let store = HistoryStore::in_memory().unwrap();
        let project_id = "test-project";

        let decision = DecisionRecord::new(
            DecisionType::Architecture,
            "Use Rust for core",
            "Decided to use Rust for performance-critical code",
        )
        .with_rationale("Better memory safety and performance");

        store.save_decision(project_id, &decision).unwrap();

        let loaded = store.get_decision(project_id, &decision.id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.title, "Use Rust for core");
        assert_eq!(loaded.decision_type, DecisionType::Architecture);
    }

    #[test]
    fn test_adr() {
        let store = HistoryStore::in_memory().unwrap();
        let project_id = "test-project";

        let adr = ArchitectureDecisionRecord::new(
            "Use SQLite for storage",
            "Need embedded database for cross-platform support",
            "Use SQLite with rusqlite bindings",
            vec!["Portable".to_string(), "No external dependencies".to_string()],
        )
        .with_status(AdrStatus::Accepted);

        store.save_adr(project_id, &adr).unwrap();

        let loaded = store.get_adr(project_id, &adr.id).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.title, "Use SQLite for storage");
        assert_eq!(loaded.status, AdrStatus::Accepted);

        // Test markdown generation
        let md = loaded.to_markdown();
        assert!(md.contains("# Use SQLite for storage"));
        assert!(md.contains("**Status:** accepted"));
    }

    #[test]
    fn test_edit_stats() {
        let store = HistoryStore::in_memory().unwrap();
        let project_id = "test-project";

        // Add some records
        for i in 0..3 {
            let record = EditRecord::new(vec![FileEdit {
                path: format!("src/file{}.rs", i % 2),
                edit_type: FileEditType::Update,
                additions: 10,
                deletions: 5,
                pre_hash: None,
                post_hash: None,
            }])
            .with_agent("test-agent")
            .with_tokens(100);

            store.save_edit_record(project_id, &record).unwrap();
        }

        let stats = store.get_edit_stats(project_id).unwrap();
        assert_eq!(stats.total_edits, 3);
        assert_eq!(stats.total_additions, 30);
        assert_eq!(stats.total_deletions, 15);
    }
}
