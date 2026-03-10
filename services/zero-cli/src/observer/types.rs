//! Observer Network Core Types
//!
//! Type definitions for the Observer Network Architecture.
//! Mirrors the TypeScript types from `packages/ccode/src/observer/types.ts`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// Enums
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatcherType {
    Code,
    World,
    #[serde(rename = "self")]
    Self_,
    Meta,
}

impl std::fmt::Display for WatcherType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code => write!(f, "code"),
            Self::World => write!(f, "world"),
            Self::Self_ => write!(f, "self"),
            Self::Meta => write!(f, "meta"),
        }
    }
}

/// Severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Medium
    }
}

/// Code observation types (from CodeWatch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeObservationType {
    GitChange,
    BuildStatus,
    TestCoverage,
    TechDebt,
    FileChange,
    DependencyUpdate,
    LintIssue,
    TypeError,
}

/// World observation types (from WorldWatch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldObservationType {
    MarketData,
    News,
    ApiChange,
    Competitor,
    DependencyRelease,
    SecurityAdvisory,
    Regulatory,
    Trend,
}

/// Self observation types (from SelfWatch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfObservationType {
    AgentBehavior,
    DecisionLog,
    ResourceUsage,
    ErrorPattern,
    ToolInvocation,
    QualityMetric,
    Latency,
    Cost,
}

/// Meta observation types (from MetaWatch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetaObservationType {
    ObservationQuality,
    SystemHealth,
    BlindSpot,
    ConsensusDrift,
    WatcherStatus,
    CoverageGap,
    Calibration,
}

/// Pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    Trend,
    Anomaly,
    Correlation,
    Cycle,
    Threshold,
    Sequence,
}

/// Anomaly types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    Outlier,
    SuddenChange,
    MissingExpected,
    UnexpectedPresence,
    Timing,
    Frequency,
}

/// Anomaly status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnomalyStatus {
    Suspected,
    Confirmed,
    Dismissed,
}

impl Default for AnomalyStatus {
    fn default() -> Self {
        Self::Suspected
    }
}

/// Opportunity types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpportunityType {
    Optimization,
    Automation,
    Learning,
    Improvement,
    Market,
    Timing,
}

/// Impact levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
}

impl Default for ImpactLevel {
    fn default() -> Self {
        Self::Medium
    }
}

/// Urgency levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
}

impl Default for UrgencyLevel {
    fn default() -> Self {
        Self::Medium
    }
}

/// Sentiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

impl Default for Sentiment {
    fn default() -> Self {
        Self::Neutral
    }
}

/// Build status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    Passing,
    Failing,
    Unknown,
}

impl Default for BuildStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Tech debt level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TechDebtLevel {
    Low,
    Medium,
    High,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Failing,
    Stopped,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Session health
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionHealth {
    Healthy,
    Degraded,
    Critical,
}

impl Default for SessionHealth {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Market sentiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketSentiment {
    Bullish,
    Bearish,
    Neutral,
}

// ══════════════════════════════════════════════════════════════════════════════
// Observation Types
// ══════════════════════════════════════════════════════════════════════════════

/// Impact information for code changes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Impact {
    /// Scope of impact
    pub scope: String,
    /// Severity of impact
    pub severity: Severity,
    /// Affected files
    #[serde(default)]
    pub affected_files: Vec<String>,
}

/// Change information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Change {
    /// Action type
    pub action: String,
    /// Value before change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<serde_json::Value>,
    /// Value after change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<serde_json::Value>,
    /// Diff representation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// Quality metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    /// CLOSE score (0-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_score: Option<f32>,
    /// Accuracy (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f32>,
    /// Efficiency (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efficiency: Option<f32>,
}

/// Health assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthAssessment {
    /// Health status
    pub health: HealthStatus,
    /// Coverage percentage (0-1)
    pub coverage: f32,
    /// Accuracy percentage (0-1)
    pub accuracy: f32,
    /// Average latency in ms
    pub latency: f64,
}

/// Observer issue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserverIssue {
    /// Issue type
    #[serde(rename = "type")]
    pub issue_type: String,
    /// Severity
    pub severity: Severity,
    /// Description
    pub description: String,
}

/// Base observation fields shared by all observation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseObservation {
    /// Unique observation ID
    pub id: String,
    /// Observation timestamp
    pub timestamp: DateTime<Utc>,
    /// Watcher ID
    pub watcher_id: String,
    /// Watcher type
    pub watcher_type: WatcherType,
    /// Confidence score 0-1
    pub confidence: f32,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl BaseObservation {
    /// Create a new base observation
    pub fn new(watcher_id: impl Into<String>, watcher_type: WatcherType) -> Self {
        Self {
            id: generate_observation_id(&watcher_type.to_string()),
            timestamp: Utc::now(),
            watcher_id: watcher_id.into(),
            watcher_type,
            confidence: 1.0,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Code observation (from CodeWatch)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeObservation {
    /// Base observation fields
    #[serde(flatten)]
    pub base: BaseObservation,
    /// Observation type
    #[serde(rename = "type")]
    pub observation_type: CodeObservationType,
    /// Source (file path, repo, or module)
    pub source: String,
    /// Change information
    pub change: Change,
    /// Impact information
    pub impact: Impact,
}

impl CodeObservation {
    /// Create a new code observation
    pub fn new(
        watcher_id: impl Into<String>,
        observation_type: CodeObservationType,
        source: impl Into<String>,
    ) -> Self {
        Self {
            base: BaseObservation::new(watcher_id, WatcherType::Code),
            observation_type,
            source: source.into(),
            change: Change::default(),
            impact: Impact::default(),
        }
    }
}

/// World observation data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldData {
    /// Title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Content
    pub content: serde_json::Value,
    /// Source URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Published timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
}

/// World observation (from WorldWatch)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldObservation {
    /// Base observation fields
    #[serde(flatten)]
    pub base: BaseObservation,
    /// Observation type
    #[serde(rename = "type")]
    pub observation_type: WorldObservationType,
    /// Source (URL, API name, or data source)
    pub source: String,
    /// Data
    pub data: WorldData,
    /// Relevance score (0-1)
    pub relevance: f32,
    /// Sentiment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment: Option<Sentiment>,
}

impl WorldObservation {
    /// Create a new world observation
    pub fn new(
        watcher_id: impl Into<String>,
        observation_type: WorldObservationType,
        source: impl Into<String>,
    ) -> Self {
        Self {
            base: BaseObservation::new(watcher_id, WatcherType::World),
            observation_type,
            source: source.into(),
            data: WorldData::default(),
            relevance: 0.5,
            sentiment: None,
        }
    }
}

/// Self observation details
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelfObservationDetails {
    /// Action taken
    pub action: String,
    /// Input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Duration in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Success flag
    pub success: bool,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Self observation (from SelfWatch)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfObservation {
    /// Base observation fields
    #[serde(flatten)]
    pub base: BaseObservation,
    /// Observation type
    #[serde(rename = "type")]
    pub observation_type: SelfObservationType,
    /// Agent ID
    pub agent_id: String,
    /// Observation details
    pub observation: SelfObservationDetails,
    /// Quality metrics
    pub quality: QualityMetrics,
}

impl SelfObservation {
    /// Create a new self observation
    pub fn new(
        watcher_id: impl Into<String>,
        observation_type: SelfObservationType,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            base: BaseObservation::new(watcher_id, WatcherType::Self_),
            observation_type,
            agent_id: agent_id.into(),
            observation: SelfObservationDetails::default(),
            quality: QualityMetrics::default(),
        }
    }
}

/// Meta observation (from MetaWatch)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaObservation {
    /// Base observation fields
    #[serde(flatten)]
    pub base: BaseObservation,
    /// Observation type
    #[serde(rename = "type")]
    pub observation_type: MetaObservationType,
    /// Target watcher ID being observed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_watcher_id: Option<String>,
    /// Health assessment
    pub assessment: HealthAssessment,
    /// Recommendations
    #[serde(default)]
    pub recommendations: Vec<String>,
    /// Issues found
    #[serde(default)]
    pub issues: Vec<ObserverIssue>,
}

impl MetaObservation {
    /// Create a new meta observation
    pub fn new(
        watcher_id: impl Into<String>,
        observation_type: MetaObservationType,
    ) -> Self {
        Self {
            base: BaseObservation::new(watcher_id, WatcherType::Meta),
            observation_type,
            target_watcher_id: None,
            assessment: HealthAssessment::default(),
            recommendations: Vec::new(),
            issues: Vec::new(),
        }
    }
}

/// Union of all observation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "watcherType", rename_all = "lowercase")]
pub enum Observation {
    Code(CodeObservation),
    World(WorldObservation),
    #[serde(rename = "self")]
    Self_(SelfObservation),
    Meta(MetaObservation),
}

impl Observation {
    /// Get the base observation fields
    pub fn base(&self) -> &BaseObservation {
        match self {
            Self::Code(o) => &o.base,
            Self::World(o) => &o.base,
            Self::Self_(o) => &o.base,
            Self::Meta(o) => &o.base,
        }
    }

    /// Get the observation ID
    pub fn id(&self) -> &str {
        &self.base().id
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.base().timestamp
    }

    /// Get the watcher type
    pub fn watcher_type(&self) -> WatcherType {
        self.base().watcher_type
    }

    /// Get the confidence
    pub fn confidence(&self) -> f32 {
        self.base().confidence
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// World Model
// ══════════════════════════════════════════════════════════════════════════════

/// Code world state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeWorldState {
    /// Last commit hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit: Option<String>,
    /// Build status
    pub build_status: BuildStatus,
    /// Test coverage percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_coverage: Option<f32>,
    /// Tech debt level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tech_debt_level: Option<TechDebtLevel>,
    /// Recent change count
    pub recent_changes: usize,
}

/// External world state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalWorldState {
    /// Market sentiment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_sentiment: Option<MarketSentiment>,
    /// Relevant news
    #[serde(default)]
    pub relevant_news: Vec<String>,
    /// External risks
    #[serde(default)]
    pub external_risks: Vec<String>,
    /// Opportunities
    #[serde(default)]
    pub opportunities: Vec<String>,
}

/// Resource usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Tokens used
    pub tokens: u64,
    /// Cost incurred
    pub cost: f64,
    /// Duration in ms
    pub duration: f64,
}

/// Self state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfState {
    /// Current agent ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_agent: Option<String>,
    /// Session health
    pub session_health: SessionHealth,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Recent error count
    pub recent_errors: usize,
    /// Decision quality (CLOSE score)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_quality: Option<f32>,
}

/// Meta state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaState {
    /// Observer health
    pub observer_health: HealthStatus,
    /// Coverage gaps
    #[serde(default)]
    pub coverage_gaps: Vec<String>,
    /// Consensus strength (0-1)
    pub consensus_strength: f32,
}

impl Default for MetaState {
    fn default() -> Self {
        Self {
            observer_health: HealthStatus::Healthy,
            coverage_gaps: Vec::new(),
            consensus_strength: 0.5,
        }
    }
}

/// World model snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldModel {
    /// Snapshot ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Contributing observation IDs
    pub observation_ids: Vec<String>,
    /// Code state
    pub code: CodeWorldState,
    /// World state
    pub world: ExternalWorldState,
    /// Self state
    #[serde(rename = "self")]
    pub self_: SelfState,
    /// Meta state
    pub meta: MetaState,
    /// Overall confidence (0-1)
    pub confidence: f32,
}

impl WorldModel {
    /// Create a new empty world model
    pub fn new() -> Self {
        Self {
            id: generate_world_model_id(),
            timestamp: Utc::now(),
            observation_ids: Vec::new(),
            code: CodeWorldState::default(),
            world: ExternalWorldState::default(),
            self_: SelfState::default(),
            meta: MetaState::default(),
            confidence: 0.0,
        }
    }
}

impl Default for WorldModel {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Pattern Types
// ══════════════════════════════════════════════════════════════════════════════

/// Attention weights for prioritizing observations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionWeights {
    /// Per-watcher type weights
    pub by_watcher: WatcherWeights,
    /// Per-observation type weights
    #[serde(default)]
    pub by_type: HashMap<String, f32>,
    /// Time decay factor (0-1)
    pub time_decay: f32,
    /// Recency bias (0-1)
    pub recency_bias: f32,
}

/// Weights per watcher type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherWeights {
    pub code: f32,
    pub world: f32,
    #[serde(rename = "self")]
    pub self_: f32,
    pub meta: f32,
}

impl Default for WatcherWeights {
    fn default() -> Self {
        Self {
            code: 0.3,
            world: 0.2,
            self_: 0.3,
            meta: 0.2,
        }
    }
}

impl Default for AttentionWeights {
    fn default() -> Self {
        Self {
            by_watcher: WatcherWeights::default(),
            by_type: HashMap::new(),
            time_decay: 0.1,
            recency_bias: 0.7,
        }
    }
}

/// Emergent pattern detected from observations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergentPattern {
    /// Pattern ID
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Description
    pub description: String,
    /// Pattern type
    #[serde(rename = "type")]
    pub pattern_type: PatternType,
    /// Contributing observation IDs
    pub observation_ids: Vec<String>,
    /// Confidence (0-1)
    pub confidence: f32,
    /// First detected timestamp
    pub detected_at: DateTime<Utc>,
    /// Last seen timestamp
    pub last_seen_at: DateTime<Utc>,
    /// Pattern strength (0-1)
    pub strength: f32,
    /// Suggested actions
    #[serde(default)]
    pub suggested_actions: Vec<String>,
    /// Pattern metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EmergentPattern {
    /// Create a new pattern
    pub fn new(name: impl Into<String>, pattern_type: PatternType) -> Self {
        let now = Utc::now();
        Self {
            id: generate_pattern_id(),
            name: name.into(),
            description: String::new(),
            pattern_type,
            observation_ids: Vec::new(),
            confidence: 0.5,
            detected_at: now,
            last_seen_at: now,
            strength: 0.5,
            suggested_actions: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Anomaly detected in observations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anomaly {
    /// Anomaly ID
    pub id: String,
    /// Anomaly type
    #[serde(rename = "type")]
    pub anomaly_type: AnomalyType,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
    /// Related observation IDs
    pub observation_ids: Vec<String>,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Status
    pub status: AnomalyStatus,
    /// Confidence (0-1)
    pub confidence: f32,
}

impl Anomaly {
    /// Create a new anomaly
    pub fn new(anomaly_type: AnomalyType, description: impl Into<String>) -> Self {
        Self {
            id: generate_anomaly_id(),
            anomaly_type,
            description: description.into(),
            severity: Severity::Medium,
            observation_ids: Vec::new(),
            detected_at: Utc::now(),
            status: AnomalyStatus::Suspected,
            confidence: 0.5,
        }
    }
}

/// Opportunity identified from observations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opportunity {
    /// Opportunity ID
    pub id: String,
    /// Opportunity type
    #[serde(rename = "type")]
    pub opportunity_type: OpportunityType,
    /// Description
    pub description: String,
    /// Potential impact
    pub impact: ImpactLevel,
    /// Time sensitivity
    pub urgency: UrgencyLevel,
    /// Related observation IDs
    pub observation_ids: Vec<String>,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Confidence (0-1)
    pub confidence: f32,
    /// Suggested actions
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

impl Opportunity {
    /// Create a new opportunity
    pub fn new(opportunity_type: OpportunityType, description: impl Into<String>) -> Self {
        Self {
            id: generate_opportunity_id(),
            opportunity_type,
            description: description.into(),
            impact: ImpactLevel::Medium,
            urgency: UrgencyLevel::Medium,
            observation_ids: Vec::new(),
            detected_at: Utc::now(),
            confidence: 0.5,
            suggested_actions: Vec::new(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ID Generation
// ══════════════════════════════════════════════════════════════════════════════

use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_id(prefix: &str) -> String {
    let count = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = Utc::now().timestamp_millis();
    format!("{}_{timestamp}_{count}", prefix)
}

/// Generate observation ID
pub fn generate_observation_id(watcher_type: &str) -> String {
    generate_id(&format!("obs_{watcher_type}"))
}

/// Generate pattern ID
pub fn generate_pattern_id() -> String {
    generate_id("pat")
}

/// Generate anomaly ID
pub fn generate_anomaly_id() -> String {
    generate_id("anom")
}

/// Generate opportunity ID
pub fn generate_opportunity_id() -> String {
    generate_id("opp")
}

/// Generate world model ID
pub fn generate_world_model_id() -> String {
    generate_id("wm")
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_observation() {
        let obs = BaseObservation::new("test-watcher", WatcherType::Code);
        assert!(obs.id.starts_with("obs_code_"));
        assert_eq!(obs.watcher_type, WatcherType::Code);
        assert_eq!(obs.confidence, 1.0);
    }

    #[test]
    fn test_code_observation() {
        let obs = CodeObservation::new(
            "code-watcher",
            CodeObservationType::GitChange,
            "src/main.rs",
        );
        assert_eq!(obs.observation_type, CodeObservationType::GitChange);
        assert_eq!(obs.source, "src/main.rs");
    }

    #[test]
    fn test_observation_enum() {
        let code_obs = CodeObservation::new(
            "watcher",
            CodeObservationType::BuildStatus,
            "project",
        );
        let obs = Observation::Code(code_obs);

        assert_eq!(obs.watcher_type(), WatcherType::Code);
        assert_eq!(obs.confidence(), 1.0);
    }

    #[test]
    fn test_world_model() {
        let model = WorldModel::new();
        assert!(model.id.starts_with("wm_"));
        assert_eq!(model.confidence, 0.0);
    }

    #[test]
    fn test_emergent_pattern() {
        let pattern = EmergentPattern::new("test pattern", PatternType::Trend);
        assert!(pattern.id.starts_with("pat_"));
        assert_eq!(pattern.pattern_type, PatternType::Trend);
    }

    #[test]
    fn test_serialization() {
        let obs = CodeObservation::new(
            "watcher",
            CodeObservationType::GitChange,
            "file.rs",
        );
        let json = serde_json::to_string(&obs).unwrap();
        assert!(json.contains("git_change"));
        assert!(json.contains("file.rs"));

        let parsed: CodeObservation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.observation_type, CodeObservationType::GitChange);
    }
}
