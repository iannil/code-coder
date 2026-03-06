//! NAPI bindings for history module

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::memory::history::{
    AdrStatus as RustAdrStatus, Alternative as RustAlternative,
    ArchitectureDecisionRecord as RustAdr, DecisionRecord as RustDecisionRecord,
    DecisionType as RustDecisionType, EditRecord as RustEditRecord,
    EditSession as RustEditSession, EditStats as RustEditStats,
    FileEdit as RustFileEdit, FileEditType as RustFileEditType, HistoryStore as RustHistoryStore,
};

// ============================================================================
// File Edit Types
// ============================================================================

/// Type of file edit
#[napi(string_enum)]
pub enum FileEditType {
    Create,
    Update,
    Delete,
    Move,
}

impl From<RustFileEditType> for FileEditType {
    fn from(t: RustFileEditType) -> Self {
        match t {
            RustFileEditType::Create => FileEditType::Create,
            RustFileEditType::Update => FileEditType::Update,
            RustFileEditType::Delete => FileEditType::Delete,
            RustFileEditType::Move => FileEditType::Move,
        }
    }
}

impl From<FileEditType> for RustFileEditType {
    fn from(t: FileEditType) -> Self {
        match t {
            FileEditType::Create => RustFileEditType::Create,
            FileEditType::Update => RustFileEditType::Update,
            FileEditType::Delete => RustFileEditType::Delete,
            FileEditType::Move => RustFileEditType::Move,
        }
    }
}

/// A single file edit
#[napi(object)]
pub struct NapiFileEdit {
    pub path: String,
    #[napi(js_name = "type")]
    pub edit_type: String,
    pub additions: u32,
    pub deletions: u32,
    pub pre_hash: Option<String>,
    pub post_hash: Option<String>,
}

impl From<RustFileEdit> for NapiFileEdit {
    fn from(e: RustFileEdit) -> Self {
        Self {
            path: e.path,
            edit_type: e.edit_type.to_string(),
            additions: e.additions as u32,
            deletions: e.deletions as u32,
            pre_hash: e.pre_hash,
            post_hash: e.post_hash,
        }
    }
}

impl TryFrom<NapiFileEdit> for RustFileEdit {
    type Error = Error;

    fn try_from(e: NapiFileEdit) -> Result<Self> {
        Ok(Self {
            path: e.path,
            edit_type: e
                .edit_type
                .parse()
                .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?,
            additions: e.additions as usize,
            deletions: e.deletions as usize,
            pre_hash: e.pre_hash,
            post_hash: e.post_hash,
        })
    }
}

// ============================================================================
// Edit Record Types
// ============================================================================

/// An edit record
#[napi(object)]
pub struct NapiEditRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub timestamp: i64,
    pub description: Option<String>,
    pub edits: Vec<NapiFileEdit>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
    pub duration: Option<u32>,
}

impl From<RustEditRecord> for NapiEditRecord {
    fn from(r: RustEditRecord) -> Self {
        Self {
            id: r.id,
            session_id: r.session_id,
            timestamp: r.timestamp,
            description: r.description,
            edits: r.edits.into_iter().map(Into::into).collect(),
            agent: r.agent,
            model: r.model,
            tokens_used: r.tokens_used.map(|t| t as u32),
            duration: r.duration.map(|d| d as u32),
        }
    }
}

impl TryFrom<NapiEditRecord> for RustEditRecord {
    type Error = Error;

    fn try_from(r: NapiEditRecord) -> Result<Self> {
        Ok(Self {
            id: r.id,
            session_id: r.session_id,
            timestamp: r.timestamp,
            description: r.description,
            edits: r
                .edits
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>>>()?,
            agent: r.agent,
            model: r.model,
            tokens_used: r.tokens_used.map(|t| t as usize),
            duration: r.duration.map(|d| d as u64),
        })
    }
}

/// Input for creating an edit record
#[napi(object)]
pub struct CreateEditRecordInput {
    pub edits: Vec<NapiFileEdit>,
    pub session_id: Option<String>,
    pub description: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
    pub duration: Option<u32>,
}

// ============================================================================
// Edit Session Types
// ============================================================================

/// An edit session
#[napi(object)]
pub struct NapiEditSession {
    pub id: String,
    pub project_id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub edits: Vec<String>,
    pub total_tokens: u32,
    pub total_duration: u32,
    pub description: Option<String>,
}

impl From<RustEditSession> for NapiEditSession {
    fn from(s: RustEditSession) -> Self {
        Self {
            id: s.id,
            project_id: s.project_id,
            start_time: s.start_time,
            end_time: s.end_time,
            edits: s.edits,
            total_tokens: s.total_tokens as u32,
            total_duration: s.total_duration as u32,
            description: s.description,
        }
    }
}

// ============================================================================
// Decision Types
// ============================================================================

/// Type of decision
#[napi(string_enum)]
pub enum DecisionType {
    Architecture,
    Implementation,
    Refactor,
    Bugfix,
    Feature,
    Other,
}

impl From<RustDecisionType> for DecisionType {
    fn from(t: RustDecisionType) -> Self {
        match t {
            RustDecisionType::Architecture => DecisionType::Architecture,
            RustDecisionType::Implementation => DecisionType::Implementation,
            RustDecisionType::Refactor => DecisionType::Refactor,
            RustDecisionType::Bugfix => DecisionType::Bugfix,
            RustDecisionType::Feature => DecisionType::Feature,
            RustDecisionType::Other => DecisionType::Other,
        }
    }
}

impl From<DecisionType> for RustDecisionType {
    fn from(t: DecisionType) -> Self {
        match t {
            DecisionType::Architecture => RustDecisionType::Architecture,
            DecisionType::Implementation => RustDecisionType::Implementation,
            DecisionType::Refactor => RustDecisionType::Refactor,
            DecisionType::Bugfix => RustDecisionType::Bugfix,
            DecisionType::Feature => RustDecisionType::Feature,
            DecisionType::Other => RustDecisionType::Other,
        }
    }
}

/// A decision record
#[napi(object)]
pub struct NapiDecisionRecord {
    pub id: String,
    #[napi(js_name = "type")]
    pub decision_type: String,
    pub title: String,
    pub description: String,
    pub rationale: Option<String>,
    pub alternatives: Option<Vec<String>>,
    pub outcome: Option<String>,
    pub session_id: Option<String>,
    pub files: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub timestamp: i64,
}

impl From<RustDecisionRecord> for NapiDecisionRecord {
    fn from(d: RustDecisionRecord) -> Self {
        Self {
            id: d.id,
            decision_type: d.decision_type.to_string(),
            title: d.title,
            description: d.description,
            rationale: d.rationale,
            alternatives: d.alternatives,
            outcome: d.outcome,
            session_id: d.session_id,
            files: d.files,
            tags: d.tags,
            timestamp: d.timestamp,
        }
    }
}

impl TryFrom<NapiDecisionRecord> for RustDecisionRecord {
    type Error = Error;

    fn try_from(d: NapiDecisionRecord) -> Result<Self> {
        Ok(Self {
            id: d.id,
            decision_type: d
                .decision_type
                .parse()
                .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?,
            title: d.title,
            description: d.description,
            rationale: d.rationale,
            alternatives: d.alternatives,
            outcome: d.outcome,
            session_id: d.session_id,
            files: d.files,
            tags: d.tags,
            timestamp: d.timestamp,
        })
    }
}

/// Input for creating a decision
#[napi(object)]
pub struct CreateDecisionInput {
    #[napi(js_name = "type")]
    pub decision_type: String,
    pub title: String,
    pub description: String,
    pub rationale: Option<String>,
    pub alternatives: Option<Vec<String>>,
    pub outcome: Option<String>,
    pub session_id: Option<String>,
    pub files: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

// ============================================================================
// ADR Types
// ============================================================================

/// ADR status
#[napi(string_enum)]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
    Rejected,
}

impl From<RustAdrStatus> for AdrStatus {
    fn from(s: RustAdrStatus) -> Self {
        match s {
            RustAdrStatus::Proposed => AdrStatus::Proposed,
            RustAdrStatus::Accepted => AdrStatus::Accepted,
            RustAdrStatus::Deprecated => AdrStatus::Deprecated,
            RustAdrStatus::Superseded => AdrStatus::Superseded,
            RustAdrStatus::Rejected => AdrStatus::Rejected,
        }
    }
}

impl From<AdrStatus> for RustAdrStatus {
    fn from(s: AdrStatus) -> Self {
        match s {
            AdrStatus::Proposed => RustAdrStatus::Proposed,
            AdrStatus::Accepted => RustAdrStatus::Accepted,
            AdrStatus::Deprecated => RustAdrStatus::Deprecated,
            AdrStatus::Superseded => RustAdrStatus::Superseded,
            AdrStatus::Rejected => RustAdrStatus::Rejected,
        }
    }
}

/// An alternative considered for an ADR
#[napi(object)]
pub struct NapiAlternative {
    pub description: String,
    pub rejected: bool,
    pub reason: Option<String>,
}

impl From<RustAlternative> for NapiAlternative {
    fn from(a: RustAlternative) -> Self {
        Self {
            description: a.description,
            rejected: a.rejected,
            reason: a.reason,
        }
    }
}

impl From<NapiAlternative> for RustAlternative {
    fn from(a: NapiAlternative) -> Self {
        Self {
            description: a.description,
            rejected: a.rejected,
            reason: a.reason,
        }
    }
}

/// An Architecture Decision Record
#[napi(object)]
pub struct NapiAdr {
    pub id: String,
    pub title: String,
    pub status: String,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub alternatives: Option<Vec<NapiAlternative>>,
    pub superseded_by: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub tags: Option<Vec<String>>,
}

impl From<RustAdr> for NapiAdr {
    fn from(a: RustAdr) -> Self {
        Self {
            id: a.id,
            title: a.title,
            status: a.status.to_string(),
            context: a.context,
            decision: a.decision,
            consequences: a.consequences,
            alternatives: a.alternatives.map(|alts| alts.into_iter().map(Into::into).collect()),
            superseded_by: a.superseded_by,
            created: a.created,
            updated: a.updated,
            tags: a.tags,
        }
    }
}

/// Input for creating an ADR
#[napi(object)]
pub struct CreateAdrInput {
    pub title: String,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub status: Option<String>,
    pub alternatives: Option<Vec<NapiAlternative>>,
    pub tags: Option<Vec<String>>,
}

// ============================================================================
// Stats Types
// ============================================================================

/// Agent statistics
#[napi(object)]
pub struct NapiAgentStats {
    pub name: String,
    pub edit_count: u32,
    pub token_count: u32,
}

/// File edit count
#[napi(object)]
pub struct NapiFileEditCount {
    pub path: String,
    pub count: u32,
}

/// Edit statistics
#[napi(object)]
pub struct NapiEditStats {
    pub total_edits: u32,
    pub total_additions: u32,
    pub total_deletions: u32,
    pub total_files: u32,
    pub top_files: Vec<NapiFileEditCount>,
    pub agent_stats: Vec<NapiAgentStats>,
}

impl From<RustEditStats> for NapiEditStats {
    fn from(s: RustEditStats) -> Self {
        Self {
            total_edits: s.total_edits as u32,
            total_additions: s.total_additions as u32,
            total_deletions: s.total_deletions as u32,
            total_files: s.total_files as u32,
            top_files: s
                .top_files
                .into_iter()
                .map(|(path, count)| NapiFileEditCount {
                    path,
                    count: count as u32,
                })
                .collect(),
            agent_stats: s
                .agent_stats
                .into_iter()
                .map(|(name, stats)| NapiAgentStats {
                    name,
                    edit_count: stats.edit_count as u32,
                    token_count: stats.token_count as u32,
                })
                .collect(),
        }
    }
}

// ============================================================================
// History Store Handle
// ============================================================================

/// Handle to a HistoryStore
#[napi]
pub struct HistoryStoreHandle {
    inner: Arc<Mutex<RustHistoryStore>>,
}

/// Open or create a history store
#[napi]
pub fn open_history_store(path: String) -> Result<HistoryStoreHandle> {
    let store = RustHistoryStore::open(Path::new(&path))
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(HistoryStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

/// Create an in-memory history store (for testing)
#[napi]
pub fn create_memory_history_store() -> Result<HistoryStoreHandle> {
    let store = RustHistoryStore::in_memory().map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(HistoryStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

#[napi]
impl HistoryStoreHandle {
    // ========================================================================
    // Edit Records
    // ========================================================================

    /// Create and save an edit record
    #[napi]
    pub fn create_edit_record(
        &self,
        project_id: String,
        input: CreateEditRecordInput,
    ) -> Result<NapiEditRecord> {
        let edits: Vec<RustFileEdit> = input
            .edits
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        let mut record = RustEditRecord::new(edits);

        if let Some(sid) = input.session_id {
            record = record.with_session(sid);
        }
        if let Some(desc) = input.description {
            record = record.with_description(desc);
        }
        if let Some(agent) = input.agent {
            record = record.with_agent(agent);
        }
        if let Some(model) = input.model {
            record = record.with_model(model);
        }
        if let Some(tokens) = input.tokens_used {
            record = record.with_tokens(tokens as usize);
        }
        if let Some(duration) = input.duration {
            record = record.with_duration(duration as u64);
        }

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .save_edit_record(&project_id, &record)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(record.into())
    }

    /// Get an edit record by ID
    #[napi]
    pub fn get_edit_record(
        &self,
        project_id: String,
        id: String,
    ) -> Result<Option<NapiEditRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let record = store
            .get_edit_record(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(record.map(Into::into))
    }

    /// Get recent edit records
    #[napi]
    pub fn get_recent_edits(
        &self,
        project_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<NapiEditRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let records = store
            .get_recent_edits(&project_id, limit.unwrap_or(20) as usize)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(records.into_iter().map(Into::into).collect())
    }

    /// Get edit records by session
    #[napi]
    pub fn get_edits_by_session(
        &self,
        project_id: String,
        session_id: String,
    ) -> Result<Vec<NapiEditRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let records = store
            .get_edits_by_session(&project_id, &session_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(records.into_iter().map(Into::into).collect())
    }

    /// Get edit records by file
    #[napi]
    pub fn get_edits_by_file(
        &self,
        project_id: String,
        file_path: String,
    ) -> Result<Vec<NapiEditRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let records = store
            .get_edits_by_file(&project_id, &file_path)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(records.into_iter().map(Into::into).collect())
    }

    // ========================================================================
    // Edit Sessions
    // ========================================================================

    /// Start a new edit session
    #[napi]
    pub fn start_edit_session(
        &self,
        project_id: String,
        description: Option<String>,
    ) -> Result<NapiEditSession> {
        let mut session = RustEditSession::new(&project_id);
        if let Some(desc) = description {
            session = session.with_description(desc);
        }

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .save_edit_session(&session)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(session.into())
    }

    /// Get an edit session by ID
    #[napi]
    pub fn get_edit_session(
        &self,
        project_id: String,
        id: String,
    ) -> Result<Option<NapiEditSession>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let session = store
            .get_edit_session(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(session.map(Into::into))
    }

    /// End an edit session
    #[napi]
    pub fn end_edit_session(
        &self,
        project_id: String,
        id: String,
    ) -> Result<Option<NapiEditSession>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;

        let session = store
            .get_edit_session(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let Some(mut session) = session else {
            return Ok(None);
        };

        session.end();
        store
            .save_edit_session(&session)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Some(session.into()))
    }

    /// Get all edit sessions
    #[napi]
    pub fn get_all_sessions(&self, project_id: String) -> Result<Vec<NapiEditSession>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let sessions = store
            .get_all_sessions(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(sessions.into_iter().map(Into::into).collect())
    }

    /// Get active (not ended) sessions
    #[napi]
    pub fn get_active_sessions(&self, project_id: String) -> Result<Vec<NapiEditSession>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let sessions = store
            .get_active_sessions(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(sessions.into_iter().map(Into::into).collect())
    }

    // ========================================================================
    // Decision Records
    // ========================================================================

    /// Create and save a decision record
    #[napi]
    pub fn create_decision(
        &self,
        project_id: String,
        input: CreateDecisionInput,
    ) -> Result<NapiDecisionRecord> {
        let decision_type: RustDecisionType = input
            .decision_type
            .parse()
            .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?;

        let mut decision =
            RustDecisionRecord::new(decision_type, &input.title, &input.description);

        if let Some(rationale) = input.rationale {
            decision = decision.with_rationale(rationale);
        }
        if let Some(alts) = input.alternatives {
            decision = decision.with_alternatives(alts);
        }
        if let Some(outcome) = input.outcome {
            decision = decision.with_outcome(outcome);
        }
        if let Some(sid) = input.session_id {
            decision = decision.with_session(sid);
        }
        if let Some(files) = input.files {
            decision = decision.with_files(files);
        }
        if let Some(tags) = input.tags {
            decision = decision.with_tags(tags);
        }

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .save_decision(&project_id, &decision)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(decision.into())
    }

    /// Get a decision record by ID
    #[napi]
    pub fn get_decision(
        &self,
        project_id: String,
        id: String,
    ) -> Result<Option<NapiDecisionRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let decision = store
            .get_decision(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(decision.map(Into::into))
    }

    /// Get recent decisions
    #[napi]
    pub fn get_recent_decisions(
        &self,
        project_id: String,
        limit: Option<u32>,
    ) -> Result<Vec<NapiDecisionRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let decisions = store
            .get_recent_decisions(&project_id, limit.unwrap_or(10) as usize)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(decisions.into_iter().map(Into::into).collect())
    }

    /// Get decisions by type
    #[napi]
    pub fn get_decisions_by_type(
        &self,
        project_id: String,
        decision_type: String,
    ) -> Result<Vec<NapiDecisionRecord>> {
        let dt: RustDecisionType = decision_type
            .parse()
            .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?;

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let decisions = store
            .get_decisions_by_type(&project_id, dt)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(decisions.into_iter().map(Into::into).collect())
    }

    /// Search decisions
    #[napi]
    pub fn search_decisions(
        &self,
        project_id: String,
        query: String,
    ) -> Result<Vec<NapiDecisionRecord>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let decisions = store
            .search_decisions(&project_id, &query)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(decisions.into_iter().map(Into::into).collect())
    }

    /// Delete a decision
    #[napi]
    pub fn delete_decision(&self, project_id: String, id: String) -> Result<bool> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .delete_decision(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ========================================================================
    // ADRs
    // ========================================================================

    /// Create and save an ADR
    #[napi]
    pub fn create_adr(&self, project_id: String, input: CreateAdrInput) -> Result<NapiAdr> {
        let mut adr =
            RustAdr::new(&input.title, &input.context, &input.decision, input.consequences);

        if let Some(status) = input.status {
            let s: RustAdrStatus = status
                .parse()
                .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?;
            adr = adr.with_status(s);
        }
        if let Some(alts) = input.alternatives {
            adr = adr.with_alternatives(alts.into_iter().map(Into::into).collect());
        }
        if let Some(tags) = input.tags {
            adr = adr.with_tags(tags);
        }

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .save_adr(&project_id, &adr)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(adr.into())
    }

    /// Get an ADR by ID
    #[napi]
    pub fn get_adr(&self, project_id: String, id: String) -> Result<Option<NapiAdr>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let adr = store
            .get_adr(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(adr.map(Into::into))
    }

    /// Get all ADRs
    #[napi]
    pub fn get_all_adrs(&self, project_id: String) -> Result<Vec<NapiAdr>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let adrs = store
            .get_all_adrs(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(adrs.into_iter().map(Into::into).collect())
    }

    /// Format an ADR as markdown
    #[napi]
    pub fn format_adr_markdown(&self, project_id: String, id: String) -> Result<Option<String>> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let adr = store
            .get_adr(&project_id, &id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(adr.map(|a| a.to_markdown()))
    }

    // ========================================================================
    // Statistics & Maintenance
    // ========================================================================

    /// Get edit statistics
    #[napi]
    pub fn get_edit_stats(&self, project_id: String) -> Result<NapiEditStats> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let stats = store
            .get_edit_stats(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(stats.into())
    }

    /// Clean up old records
    #[napi]
    pub fn cleanup(&self, project_id: String, before_timestamp: i64) -> Result<u32> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let removed = store
            .cleanup(&project_id, before_timestamp)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(removed as u32)
    }

    /// Invalidate all history for a project
    #[napi]
    pub fn invalidate(&self, project_id: String) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store
            .invalidate(&project_id)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}
