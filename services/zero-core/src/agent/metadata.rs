//! Agent Metadata - Rich metadata for agent search and discovery
//!
//! This module provides extended metadata structures for agents, including:
//! - Capabilities: What an agent can do
//! - Triggers: When an agent should be automatically invoked
//! - Examples: Usage examples for documentation
//! - Search: Fuzzy search and recommendation functionality

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strsim::jaro_winkler;

// ============================================================================
// Core Types
// ============================================================================

/// Agent capability declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Unique capability identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this capability does
    pub description: String,
    /// Whether this is a primary capability
    #[serde(default)]
    pub primary: bool,
}

/// Trigger type for automatic agent invocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    Keyword,
    Pattern,
    Event,
    Context,
}

impl Default for TriggerType {
    fn default() -> Self {
        TriggerType::Keyword
    }
}

/// Trigger condition for automatic agent invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrigger {
    /// Trigger type
    #[serde(rename = "type", default)]
    pub trigger_type: TriggerType,
    /// Trigger value (keyword, regex pattern, event name, or context condition)
    pub value: String,
    /// Trigger priority (higher = more important)
    #[serde(default)]
    pub priority: i32,
    /// Optional description
    pub description: Option<String>,
}

/// Usage example for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExample {
    /// Example title
    pub title: String,
    /// User input
    pub input: String,
    /// Expected agent behavior or output summary
    pub output: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Agent category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentCategory {
    Engineering,
    Content,
    Analysis,
    Philosophy,
    System,
    #[default]
    Custom,
}

impl std::fmt::Display for AgentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentCategory::Engineering => write!(f, "engineering"),
            AgentCategory::Content => write!(f, "content"),
            AgentCategory::Analysis => write!(f, "analysis"),
            AgentCategory::Philosophy => write!(f, "philosophy"),
            AgentCategory::System => write!(f, "system"),
            AgentCategory::Custom => write!(f, "custom"),
        }
    }
}

/// Agent role within a mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Primary,
    Alternative,
    #[default]
    Capability,
    System,
    Hidden,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Primary => write!(f, "primary"),
            AgentRole::Alternative => write!(f, "alternative"),
            AgentRole::Capability => write!(f, "capability"),
            AgentRole::System => write!(f, "system"),
            AgentRole::Hidden => write!(f, "hidden"),
        }
    }
}

/// Extended agent metadata for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent name (must match AgentConfig.name)
    pub name: String,
    /// Display name for UI
    #[serde(default)]
    pub display_name: Option<String>,
    /// Short description (one line)
    #[serde(default)]
    pub short_description: Option<String>,
    /// Long description (markdown supported)
    #[serde(default)]
    pub long_description: Option<String>,
    /// Agent category
    #[serde(default)]
    pub category: AgentCategory,
    /// Mode this agent belongs to (build, writer, decision)
    #[serde(default)]
    pub mode: Option<String>,
    /// Role within the mode
    #[serde(default)]
    pub role: AgentRole,
    /// Agent capabilities
    #[serde(default)]
    pub capabilities: Vec<AgentCapability>,
    /// Auto-invocation triggers
    #[serde(default)]
    pub triggers: Vec<AgentTrigger>,
    /// Usage examples
    #[serde(default)]
    pub examples: Vec<AgentExample>,
    /// Tags for search
    #[serde(default)]
    pub tags: Vec<String>,
    /// Author information
    #[serde(default)]
    pub author: Option<String>,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Whether this is a built-in agent
    #[serde(default)]
    pub builtin: bool,
    /// Icon name or emoji
    #[serde(default)]
    pub icon: Option<String>,
    /// Recommended for first-time users
    #[serde(default)]
    pub recommended: bool,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl Default for AgentMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            display_name: None,
            short_description: None,
            long_description: None,
            category: AgentCategory::default(),
            mode: None,
            role: AgentRole::default(),
            capabilities: Vec::new(),
            triggers: Vec::new(),
            examples: Vec::new(),
            tags: Vec::new(),
            author: None,
            version: default_version(),
            builtin: false,
            icon: None,
            recommended: false,
        }
    }
}

// ============================================================================
// Search Types
// ============================================================================

/// Match information for a search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Field that matched
    pub key: String,
    /// Matched value
    pub value: String,
    /// Match indices (start, end) pairs
    pub indices: Vec<(usize, usize)>,
}

/// Search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matched agent metadata
    pub agent: AgentMetadata,
    /// Relevance score (0.0 = no match, 1.0 = perfect match)
    pub score: f64,
    /// Match details
    pub matches: Vec<SearchMatch>,
}

impl SearchResult {
    pub fn new(agent: AgentMetadata, score: f64) -> Self {
        Self {
            agent,
            score,
            matches: Vec::new(),
        }
    }

    pub fn with_match(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.matches.push(SearchMatch {
            key: key.into(),
            value: value.into(),
            indices: Vec::new(),
        });
        self
    }
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,
    /// Minimum score threshold (0.0 - 1.0)
    pub threshold: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            threshold: 0.3,
        }
    }
}

// ============================================================================
// Agent Metadata Index
// ============================================================================

/// Index for agent metadata with search capabilities
pub struct MetadataIndex {
    /// All agent metadata
    metadata: HashMap<String, AgentMetadata>,
    /// Field weights for scoring
    field_weights: FieldWeights,
}

/// Weights for different searchable fields
#[derive(Debug, Clone)]
pub struct FieldWeights {
    pub name: f64,
    pub display_name: f64,
    pub short_description: f64,
    pub long_description: f64,
    pub tags: f64,
    pub capabilities_name: f64,
    pub capabilities_description: f64,
    pub examples_title: f64,
    pub examples_input: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            name: 2.0,
            display_name: 2.0,
            short_description: 1.5,
            long_description: 1.0,
            tags: 1.5,
            capabilities_name: 1.0,
            capabilities_description: 0.8,
            examples_title: 0.8,
            examples_input: 0.6,
        }
    }
}

impl MetadataIndex {
    /// Create a new metadata index
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            field_weights: FieldWeights::default(),
        }
    }

    /// Create with custom field weights
    pub fn with_weights(weights: FieldWeights) -> Self {
        Self {
            metadata: HashMap::new(),
            field_weights: weights,
        }
    }

    /// Register agent metadata
    pub fn register(&mut self, meta: AgentMetadata) {
        self.metadata.insert(meta.name.clone(), meta);
    }

    /// Unregister agent metadata
    pub fn unregister(&mut self, name: &str) -> Option<AgentMetadata> {
        self.metadata.remove(name)
    }

    /// Get metadata by name
    pub fn get(&self, name: &str) -> Option<&AgentMetadata> {
        self.metadata.get(name)
    }

    /// List all metadata
    pub fn list(&self) -> Vec<&AgentMetadata> {
        self.metadata.values().collect()
    }

    /// List metadata by category
    pub fn list_by_category(&self, category: AgentCategory) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|m| m.category == category)
            .collect()
    }

    /// List metadata by mode
    pub fn list_by_mode(&self, mode: &str) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|m| m.mode.as_deref() == Some(mode))
            .collect()
    }

    /// List metadata by role
    pub fn list_by_role(&self, role: AgentRole) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|m| m.role == role)
            .collect()
    }

    /// List visible agents (excludes hidden)
    pub fn list_visible(&self) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|m| m.role != AgentRole::Hidden)
            .collect()
    }

    /// List recommended agents
    pub fn list_recommended(&self) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|m| m.recommended)
            .collect()
    }

    /// Get primary agent for a mode
    pub fn get_primary_for_mode(&self, mode: &str) -> Option<&AgentMetadata> {
        self.metadata
            .values()
            .find(|m| m.mode.as_deref() == Some(mode) && m.role == AgentRole::Primary)
    }

    // ========================================================================
    // Search Methods
    // ========================================================================

    /// Search agents by query using fuzzy matching
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Vec<SearchResult> {
        let opts = options.unwrap_or_default();
        let query_lower = query.to_lowercase();

        if query_lower.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = self
            .metadata
            .values()
            .filter_map(|meta| {
                let score = self.calculate_score(meta, &query_lower);
                if score >= opts.threshold {
                    Some(SearchResult::new(meta.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(opts.limit);

        results
    }

    /// Calculate relevance score for an agent
    fn calculate_score(&self, meta: &AgentMetadata, query: &str) -> f64 {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        // Score name
        let name_score = self.fuzzy_score(&meta.name.to_lowercase(), query);
        total_score += name_score * self.field_weights.name;
        total_weight += self.field_weights.name;

        // Score display name
        if let Some(ref display_name) = meta.display_name {
            let score = self.fuzzy_score(&display_name.to_lowercase(), query);
            total_score += score * self.field_weights.display_name;
            total_weight += self.field_weights.display_name;
        }

        // Score short description
        if let Some(ref desc) = meta.short_description {
            let score = self.fuzzy_score(&desc.to_lowercase(), query);
            total_score += score * self.field_weights.short_description;
            total_weight += self.field_weights.short_description;
        }

        // Score long description
        if let Some(ref desc) = meta.long_description {
            let score = self.fuzzy_score(&desc.to_lowercase(), query);
            total_score += score * self.field_weights.long_description;
            total_weight += self.field_weights.long_description;
        }

        // Score tags
        for tag in &meta.tags {
            let score = self.fuzzy_score(&tag.to_lowercase(), query);
            total_score += score * self.field_weights.tags;
            total_weight += self.field_weights.tags;
        }

        // Score capabilities
        for cap in &meta.capabilities {
            let name_score = self.fuzzy_score(&cap.name.to_lowercase(), query);
            total_score += name_score * self.field_weights.capabilities_name;
            total_weight += self.field_weights.capabilities_name;

            let desc_score = self.fuzzy_score(&cap.description.to_lowercase(), query);
            total_score += desc_score * self.field_weights.capabilities_description;
            total_weight += self.field_weights.capabilities_description;
        }

        // Score examples
        for example in &meta.examples {
            let title_score = self.fuzzy_score(&example.title.to_lowercase(), query);
            total_score += title_score * self.field_weights.examples_title;
            total_weight += self.field_weights.examples_title;

            let input_score = self.fuzzy_score(&example.input.to_lowercase(), query);
            total_score += input_score * self.field_weights.examples_input;
            total_weight += self.field_weights.examples_input;
        }

        if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        }
    }

    /// Calculate fuzzy match score using Jaro-Winkler
    fn fuzzy_score(&self, text: &str, query: &str) -> f64 {
        // Check for exact substring match first
        if text.contains(query) {
            return 1.0;
        }

        // Check word-level matches
        let text_words: Vec<&str> = text.split_whitespace().collect();
        let query_words: Vec<&str> = query.split_whitespace().collect();

        let mut max_score: f64 = 0.0;

        // Check each query word against text words
        for qword in &query_words {
            for tword in &text_words {
                let score = jaro_winkler(qword, tword);
                max_score = max_score.max(score);
            }
        }

        // Also check the full query against full text
        let full_score = jaro_winkler(query, text);
        max_score = max_score.max(full_score);

        max_score
    }

    // ========================================================================
    // Trigger Matching
    // ========================================================================

    /// Find agents by trigger match
    pub fn find_by_trigger(&self, input: &str) -> Vec<&AgentMetadata> {
        let input_lower = input.to_lowercase();
        let mut matches: Vec<(&AgentMetadata, i32)> = Vec::new();

        for meta in self.metadata.values() {
            for trigger in &meta.triggers {
                let matched = match trigger.trigger_type {
                    TriggerType::Keyword => input_lower.contains(&trigger.value.to_lowercase()),
                    TriggerType::Pattern => {
                        regex::Regex::new(&trigger.value)
                            .map(|re| re.is_match(input))
                            .unwrap_or(false)
                    }
                    TriggerType::Context => {
                        regex::Regex::new(&trigger.value)
                            .map(|re| re.is_match(input))
                            .unwrap_or(false)
                    }
                    TriggerType::Event => false, // Events handled separately
                };

                if matched {
                    matches.push((meta, trigger.priority));
                    break; // Only count first trigger match per agent
                }
            }
        }

        // Sort by priority descending
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        matches.into_iter().map(|(m, _)| m).collect()
    }

    /// Find agents by event trigger
    pub fn find_by_event(&self, event_name: &str) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|meta| {
                meta.triggers.iter().any(|t| {
                    t.trigger_type == TriggerType::Event && t.value == event_name
                })
            })
            .collect()
    }

    /// Find agents with a specific capability
    pub fn find_by_capability(&self, capability_id: &str) -> Vec<&AgentMetadata> {
        self.metadata
            .values()
            .filter(|meta| meta.capabilities.iter().any(|c| c.id == capability_id))
            .collect()
    }

    // ========================================================================
    // Recommendation
    // ========================================================================

    /// Recommend an agent based on user intent
    ///
    /// Priority:
    /// 1. Exact @mention match (if input starts with @)
    /// 2. Keyword trigger match
    /// 3. Fuzzy search match
    /// 4. Default recommended agent
    pub fn recommend(&self, intent: &str) -> Option<&AgentMetadata> {
        let trimmed = intent.trim();

        // First try @mention match
        if trimmed.starts_with('@') {
            let mention = &trimmed[1..].split_whitespace().next().unwrap_or("");
            if let Some(meta) = self.metadata.get(*mention) {
                return Some(meta);
            }
            // Also check display names and aliases
            for meta in self.metadata.values() {
                if let Some(ref display) = meta.display_name {
                    if display.to_lowercase() == mention.to_lowercase() {
                        return Some(meta);
                    }
                }
            }
        }

        // Try trigger matching
        let trigger_matches = self.find_by_trigger(intent);
        if let Some(first) = trigger_matches.first() {
            return Some(*first);
        }

        // For short inputs (≤3 chars), skip fuzzy search to avoid false matches
        if trimmed.len() <= 3 {
            return self.list_recommended().first().copied();
        }

        // Fall back to search
        let search_results = self.search(intent, Some(SearchOptions {
            limit: 1,
            threshold: 0.4,
        }));

        if let Some(first) = search_results.first() {
            return self.metadata.get(&first.agent.name);
        }

        // Fall back to default recommended
        self.list_recommended().first().copied()
    }
}

impl Default for MetadataIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in Metadata Definitions
// ============================================================================

/// Create default metadata for built-in agents
pub fn create_builtin_metadata() -> HashMap<String, AgentMetadata> {
    let mut metadata = HashMap::new();

    // Build agent
    metadata.insert(
        "build".to_string(),
        AgentMetadata {
            name: "build".to_string(),
            display_name: Some("Build".to_string()),
            short_description: Some("Primary development agent for building features and fixing bugs".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Primary,
            capabilities: vec![
                AgentCapability {
                    id: "code-write".to_string(),
                    name: "Code Writing".to_string(),
                    description: "Write and modify code".to_string(),
                    primary: true,
                },
                AgentCapability {
                    id: "file-edit".to_string(),
                    name: "File Editing".to_string(),
                    description: "Edit files in the codebase".to_string(),
                    primary: true,
                },
            ],
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "build".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "implement".to_string(),
                    priority: 8,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "create".to_string(),
                    priority: 7,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "fix".to_string(),
                    priority: 6,
                    description: None,
                },
            ],
            tags: vec!["development".to_string(), "coding".to_string(), "primary".to_string()],
            builtin: true,
            icon: Some("🔨".to_string()),
            ..Default::default()
        },
    );

    // Plan agent
    metadata.insert(
        "plan".to_string(),
        AgentMetadata {
            name: "plan".to_string(),
            display_name: Some("Plan".to_string()),
            short_description: Some("Creates detailed implementation plans before coding".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Alternative,
            capabilities: vec![
                AgentCapability {
                    id: "planning".to_string(),
                    name: "Planning".to_string(),
                    description: "Create step-by-step plans".to_string(),
                    primary: true,
                },
            ],
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "plan".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "design".to_string(),
                    priority: 8,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "architecture".to_string(),
                    priority: 7,
                    description: None,
                },
            ],
            tags: vec!["planning".to_string(), "design".to_string()],
            builtin: true,
            icon: Some("📋".to_string()),
            ..Default::default()
        },
    );

    // General agent
    metadata.insert(
        "general".to_string(),
        AgentMetadata {
            name: "general".to_string(),
            display_name: Some("General Assistant".to_string()),
            short_description: Some("General-purpose assistant for conversation and queries".to_string()),
            category: AgentCategory::Custom,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            capabilities: vec![
                AgentCapability {
                    id: "conversation".to_string(),
                    name: "Conversation".to_string(),
                    description: "Natural conversation".to_string(),
                    primary: true,
                },
            ],
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "help".to_string(),
                    priority: 5,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "帮助".to_string(),
                    priority: 5,
                    description: None,
                },
            ],
            tags: vec!["general".to_string(), "assistant".to_string(), "conversation".to_string()],
            builtin: true,
            icon: Some("💬".to_string()),
            recommended: true,
            ..Default::default()
        },
    );

    // Code reviewer
    metadata.insert(
        "code-reviewer".to_string(),
        AgentMetadata {
            name: "code-reviewer".to_string(),
            display_name: Some("Code Reviewer".to_string()),
            short_description: Some("Comprehensive code quality reviews with actionable feedback".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            capabilities: vec![
                AgentCapability {
                    id: "review".to_string(),
                    name: "Code Review".to_string(),
                    description: "Review code for quality issues".to_string(),
                    primary: true,
                },
            ],
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "review".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "code review".to_string(),
                    priority: 10,
                    description: None,
                },
            ],
            tags: vec!["review".to_string(), "quality".to_string(), "engineering".to_string()],
            builtin: true,
            icon: Some("🔍".to_string()),
            ..Default::default()
        },
    );

    // Security reviewer
    metadata.insert(
        "security-reviewer".to_string(),
        AgentMetadata {
            name: "security-reviewer".to_string(),
            display_name: Some("Security Reviewer".to_string()),
            short_description: Some("Analyzes code for security vulnerabilities".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            capabilities: vec![
                AgentCapability {
                    id: "security-audit".to_string(),
                    name: "Security Audit".to_string(),
                    description: "Identify security issues".to_string(),
                    primary: true,
                },
            ],
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "security".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "vulnerability".to_string(),
                    priority: 9,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Context,
                    value: "auth|payment|credential".to_string(),
                    priority: 8,
                    description: None,
                },
            ],
            tags: vec!["security".to_string(), "audit".to_string(), "engineering".to_string()],
            builtin: true,
            icon: Some("🔒".to_string()),
            ..Default::default()
        },
    );

    // TDD guide
    metadata.insert(
        "tdd-guide".to_string(),
        AgentMetadata {
            name: "tdd-guide".to_string(),
            display_name: Some("TDD Guide".to_string()),
            short_description: Some("Enforces test-driven development methodology".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "tdd".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "test first".to_string(),
                    priority: 9,
                    description: None,
                },
            ],
            tags: vec!["testing".to_string(), "tdd".to_string(), "engineering".to_string()],
            builtin: true,
            icon: Some("🧪".to_string()),
            ..Default::default()
        },
    );

    // Writer agent
    metadata.insert(
        "writer".to_string(),
        AgentMetadata {
            name: "writer".to_string(),
            display_name: Some("Writer".to_string()),
            short_description: Some("Long-form content writing (20k+ words)".to_string()),
            category: AgentCategory::Content,
            mode: Some("writer".to_string()),
            role: AgentRole::Primary,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "write book".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "write article".to_string(),
                    priority: 9,
                    description: None,
                },
            ],
            tags: vec!["writing".to_string(), "content".to_string(), "books".to_string()],
            builtin: true,
            icon: Some("✍️".to_string()),
            ..Default::default()
        },
    );

    // Decision agent (CLOSE framework)
    metadata.insert(
        "decision".to_string(),
        AgentMetadata {
            name: "decision".to_string(),
            display_name: Some("Decision (CLOSE)".to_string()),
            short_description: Some("Sustainable decision-making with CLOSE framework".to_string()),
            category: AgentCategory::Philosophy,
            mode: Some("decision".to_string()),
            role: AgentRole::Primary,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "decision".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "CLOSE".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "选择".to_string(),
                    priority: 8,
                    description: None,
                },
            ],
            tags: vec!["decision".to_string(), "close".to_string(), "philosophy".to_string()],
            builtin: true,
            icon: Some("🎯".to_string()),
            ..Default::default()
        },
    );

    // Observer agent (祝融说)
    metadata.insert(
        "observer".to_string(),
        AgentMetadata {
            name: "observer".to_string(),
            display_name: Some("Observer (祝融说)".to_string()),
            short_description: Some("Analysis through Zhu Rong philosophy".to_string()),
            category: AgentCategory::Philosophy,
            mode: Some("decision".to_string()),
            role: AgentRole::Alternative,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "祝融说".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "observer".to_string(),
                    priority: 9,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "可能性".to_string(),
                    priority: 8,
                    description: None,
                },
            ],
            tags: vec!["philosophy".to_string(), "zhurong".to_string(), "analysis".to_string()],
            builtin: true,
            icon: Some("👁️".to_string()),
            ..Default::default()
        },
    );

    // Explore agent
    metadata.insert(
        "explore".to_string(),
        AgentMetadata {
            name: "explore".to_string(),
            display_name: Some("Explorer".to_string()),
            short_description: Some("Fast codebase exploration and search".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "find".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "search".to_string(),
                    priority: 9,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "where".to_string(),
                    priority: 7,
                    description: None,
                },
            ],
            tags: vec!["search".to_string(), "exploration".to_string(), "engineering".to_string()],
            builtin: true,
            icon: Some("🔭".to_string()),
            ..Default::default()
        },
    );

    // Autonomous agent
    metadata.insert(
        "autonomous".to_string(),
        AgentMetadata {
            name: "autonomous".to_string(),
            display_name: Some("Autonomous".to_string()),
            short_description: Some("Fully autonomous execution with self-correction".to_string()),
            category: AgentCategory::System,
            mode: Some("build".to_string()),
            role: AgentRole::Alternative,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "autonomous".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "自主".to_string(),
                    priority: 9,
                    description: None,
                },
            ],
            tags: vec!["autonomous".to_string(), "system".to_string(), "self-directed".to_string()],
            builtin: true,
            icon: Some("🤖".to_string()),
            ..Default::default()
        },
    );

    // Architect agent
    metadata.insert(
        "architect".to_string(),
        AgentMetadata {
            name: "architect".to_string(),
            display_name: Some("Architect".to_string()),
            short_description: Some("Designs system architecture and establishes patterns".to_string()),
            category: AgentCategory::Engineering,
            mode: Some("build".to_string()),
            role: AgentRole::Capability,
            triggers: vec![
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "architect".to_string(),
                    priority: 10,
                    description: None,
                },
                AgentTrigger {
                    trigger_type: TriggerType::Keyword,
                    value: "design system".to_string(),
                    priority: 9,
                    description: None,
                },
            ],
            tags: vec!["architecture".to_string(), "design".to_string(), "engineering".to_string()],
            builtin: true,
            icon: Some("🏗️".to_string()),
            ..Default::default()
        },
    );

    metadata
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_index() -> MetadataIndex {
        let mut index = MetadataIndex::new();
        for (_, meta) in create_builtin_metadata() {
            index.register(meta);
        }
        index
    }

    #[test]
    fn test_search_basic() {
        let index = create_test_index();

        let results = index.search("build", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].agent.name, "build");
    }

    #[test]
    fn test_search_fuzzy() {
        let index = create_test_index();

        // Should find "build" even with typo
        let results = index.search("biuld", Some(SearchOptions {
            limit: 5,
            threshold: 0.6,
        }));
        assert!(!results.is_empty());
    }

    #[test]
    fn test_find_by_trigger() {
        let index = create_test_index();

        let matches = index.find_by_trigger("review the code");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.name == "code-reviewer"));
    }

    #[test]
    fn test_recommend() {
        let index = create_test_index();

        // Direct @mention
        let result = index.recommend("@build something");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "build");

        // Trigger keyword
        let result = index.recommend("review this code please");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "code-reviewer");
    }

    #[test]
    fn test_list_by_category() {
        let index = create_test_index();

        let engineering = index.list_by_category(AgentCategory::Engineering);
        assert!(engineering.len() > 3);
        assert!(engineering.iter().all(|m| m.category == AgentCategory::Engineering));
    }

    #[test]
    fn test_list_by_mode() {
        let index = create_test_index();

        let build_mode = index.list_by_mode("build");
        assert!(build_mode.len() > 3);
        assert!(build_mode.iter().all(|m| m.mode.as_deref() == Some("build")));
    }

    #[test]
    fn test_get_primary_for_mode() {
        let index = create_test_index();

        let build_primary = index.get_primary_for_mode("build");
        assert!(build_primary.is_some());
        assert_eq!(build_primary.unwrap().name, "build");

        let writer_primary = index.get_primary_for_mode("writer");
        assert!(writer_primary.is_some());
        assert_eq!(writer_primary.unwrap().name, "writer");
    }
}
