//! NAPI bindings for Agent Registry with fuzzy search
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Agent metadata registration and lookup
//! - Fuzzy search across agents
//! - Trigger-based agent matching
//! - Agent recommendation based on user intent

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::metadata::{
    AgentCapability as RustAgentCapability,
    AgentCategory as RustAgentCategory,
    AgentExample as RustAgentExample,
    AgentMetadata as RustAgentMetadata,
    AgentRole as RustAgentRole,
    AgentTrigger as RustAgentTrigger,
    MetadataIndex,
    SearchOptions as RustSearchOptions,
    SearchResult as RustSearchResult,
    TriggerType as RustTriggerType,
    create_builtin_metadata,
};

// ============================================================================
// NAPI Type Conversions
// ============================================================================

/// Agent category for NAPI
#[napi(string_enum)]
pub enum NapiAgentCategory {
    Engineering,
    Content,
    Analysis,
    Philosophy,
    System,
    Custom,
}

impl From<RustAgentCategory> for NapiAgentCategory {
    fn from(c: RustAgentCategory) -> Self {
        match c {
            RustAgentCategory::Engineering => NapiAgentCategory::Engineering,
            RustAgentCategory::Content => NapiAgentCategory::Content,
            RustAgentCategory::Analysis => NapiAgentCategory::Analysis,
            RustAgentCategory::Philosophy => NapiAgentCategory::Philosophy,
            RustAgentCategory::System => NapiAgentCategory::System,
            RustAgentCategory::Custom => NapiAgentCategory::Custom,
        }
    }
}

impl From<NapiAgentCategory> for RustAgentCategory {
    fn from(c: NapiAgentCategory) -> Self {
        match c {
            NapiAgentCategory::Engineering => RustAgentCategory::Engineering,
            NapiAgentCategory::Content => RustAgentCategory::Content,
            NapiAgentCategory::Analysis => RustAgentCategory::Analysis,
            NapiAgentCategory::Philosophy => RustAgentCategory::Philosophy,
            NapiAgentCategory::System => RustAgentCategory::System,
            NapiAgentCategory::Custom => RustAgentCategory::Custom,
        }
    }
}

/// Agent role for NAPI
#[napi(string_enum)]
pub enum NapiAgentRole {
    Primary,
    Alternative,
    Capability,
    System,
    Hidden,
}

impl From<RustAgentRole> for NapiAgentRole {
    fn from(r: RustAgentRole) -> Self {
        match r {
            RustAgentRole::Primary => NapiAgentRole::Primary,
            RustAgentRole::Alternative => NapiAgentRole::Alternative,
            RustAgentRole::Capability => NapiAgentRole::Capability,
            RustAgentRole::System => NapiAgentRole::System,
            RustAgentRole::Hidden => NapiAgentRole::Hidden,
        }
    }
}

impl From<NapiAgentRole> for RustAgentRole {
    fn from(r: NapiAgentRole) -> Self {
        match r {
            NapiAgentRole::Primary => RustAgentRole::Primary,
            NapiAgentRole::Alternative => RustAgentRole::Alternative,
            NapiAgentRole::Capability => RustAgentRole::Capability,
            NapiAgentRole::System => RustAgentRole::System,
            NapiAgentRole::Hidden => RustAgentRole::Hidden,
        }
    }
}

/// Trigger type for NAPI
#[napi(string_enum)]
pub enum NapiTriggerType {
    Keyword,
    Pattern,
    Event,
    Context,
}

impl From<RustTriggerType> for NapiTriggerType {
    fn from(t: RustTriggerType) -> Self {
        match t {
            RustTriggerType::Keyword => NapiTriggerType::Keyword,
            RustTriggerType::Pattern => NapiTriggerType::Pattern,
            RustTriggerType::Event => NapiTriggerType::Event,
            RustTriggerType::Context => NapiTriggerType::Context,
        }
    }
}

impl From<NapiTriggerType> for RustTriggerType {
    fn from(t: NapiTriggerType) -> Self {
        match t {
            NapiTriggerType::Keyword => RustTriggerType::Keyword,
            NapiTriggerType::Pattern => RustTriggerType::Pattern,
            NapiTriggerType::Event => RustTriggerType::Event,
            NapiTriggerType::Context => RustTriggerType::Context,
        }
    }
}

/// Agent capability for NAPI
#[napi(object)]
pub struct NapiAgentCapability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub primary: bool,
}

impl From<RustAgentCapability> for NapiAgentCapability {
    fn from(c: RustAgentCapability) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            primary: c.primary,
        }
    }
}

impl From<NapiAgentCapability> for RustAgentCapability {
    fn from(c: NapiAgentCapability) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            primary: c.primary,
        }
    }
}

/// Agent trigger for NAPI
#[napi(object)]
pub struct NapiAgentTrigger {
    #[napi(js_name = "type")]
    pub trigger_type: String,
    pub value: String,
    pub priority: i32,
    pub description: Option<String>,
}

impl From<RustAgentTrigger> for NapiAgentTrigger {
    fn from(t: RustAgentTrigger) -> Self {
        let type_str = match t.trigger_type {
            RustTriggerType::Keyword => "keyword",
            RustTriggerType::Pattern => "pattern",
            RustTriggerType::Event => "event",
            RustTriggerType::Context => "context",
        };
        Self {
            trigger_type: type_str.to_string(),
            value: t.value,
            priority: t.priority,
            description: t.description,
        }
    }
}

impl From<NapiAgentTrigger> for RustAgentTrigger {
    fn from(t: NapiAgentTrigger) -> Self {
        let trigger_type = match t.trigger_type.as_str() {
            "keyword" => RustTriggerType::Keyword,
            "pattern" => RustTriggerType::Pattern,
            "event" => RustTriggerType::Event,
            "context" => RustTriggerType::Context,
            _ => RustTriggerType::Keyword,
        };
        Self {
            trigger_type,
            value: t.value,
            priority: t.priority,
            description: t.description,
        }
    }
}

/// Agent example for NAPI
#[napi(object)]
pub struct NapiAgentExample {
    pub title: String,
    pub input: String,
    pub output: String,
    pub tags: Vec<String>,
}

impl From<RustAgentExample> for NapiAgentExample {
    fn from(e: RustAgentExample) -> Self {
        Self {
            title: e.title,
            input: e.input,
            output: e.output,
            tags: e.tags,
        }
    }
}

impl From<NapiAgentExample> for RustAgentExample {
    fn from(e: NapiAgentExample) -> Self {
        Self {
            title: e.title,
            input: e.input,
            output: e.output,
            tags: e.tags,
        }
    }
}

/// Agent metadata for NAPI
#[napi(object)]
pub struct NapiAgentMetadata {
    pub name: String,
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub category: String,
    pub mode: Option<String>,
    pub role: String,
    pub capabilities: Vec<NapiAgentCapability>,
    pub triggers: Vec<NapiAgentTrigger>,
    pub examples: Vec<NapiAgentExample>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub version: String,
    pub builtin: bool,
    pub icon: Option<String>,
    pub recommended: bool,
}

impl From<RustAgentMetadata> for NapiAgentMetadata {
    fn from(m: RustAgentMetadata) -> Self {
        Self {
            name: m.name,
            display_name: m.display_name,
            short_description: m.short_description,
            long_description: m.long_description,
            category: m.category.to_string(),
            mode: m.mode,
            role: m.role.to_string(),
            capabilities: m.capabilities.into_iter().map(Into::into).collect(),
            triggers: m.triggers.into_iter().map(Into::into).collect(),
            examples: m.examples.into_iter().map(Into::into).collect(),
            tags: m.tags,
            author: m.author,
            version: m.version,
            builtin: m.builtin,
            icon: m.icon,
            recommended: m.recommended,
        }
    }
}

impl From<NapiAgentMetadata> for RustAgentMetadata {
    fn from(m: NapiAgentMetadata) -> Self {
        let category = match m.category.as_str() {
            "engineering" => RustAgentCategory::Engineering,
            "content" => RustAgentCategory::Content,
            "analysis" => RustAgentCategory::Analysis,
            "philosophy" => RustAgentCategory::Philosophy,
            "system" => RustAgentCategory::System,
            _ => RustAgentCategory::Custom,
        };
        let role = match m.role.as_str() {
            "primary" => RustAgentRole::Primary,
            "alternative" => RustAgentRole::Alternative,
            "capability" => RustAgentRole::Capability,
            "system" => RustAgentRole::System,
            "hidden" => RustAgentRole::Hidden,
            _ => RustAgentRole::Capability,
        };
        Self {
            name: m.name,
            display_name: m.display_name,
            short_description: m.short_description,
            long_description: m.long_description,
            category,
            mode: m.mode,
            role,
            capabilities: m.capabilities.into_iter().map(Into::into).collect(),
            triggers: m.triggers.into_iter().map(Into::into).collect(),
            examples: m.examples.into_iter().map(Into::into).collect(),
            tags: m.tags,
            author: m.author,
            version: m.version,
            builtin: m.builtin,
            icon: m.icon,
            recommended: m.recommended,
        }
    }
}

/// Search match for NAPI
#[napi(object)]
pub struct NapiSearchMatch {
    pub key: String,
    pub value: String,
    pub indices: Vec<Vec<u32>>,
}

/// Search result for NAPI
#[napi(object)]
pub struct NapiSearchResult {
    pub agent: NapiAgentMetadata,
    pub score: f64,
    pub matches: Vec<NapiSearchMatch>,
}

impl From<RustSearchResult> for NapiSearchResult {
    fn from(r: RustSearchResult) -> Self {
        Self {
            agent: r.agent.into(),
            score: r.score,
            matches: r.matches.into_iter().map(|m| NapiSearchMatch {
                key: m.key,
                value: m.value,
                indices: m.indices.into_iter().map(|(s, e)| vec![s as u32, e as u32]).collect(),
            }).collect(),
        }
    }
}

/// Search options for NAPI
#[napi(object)]
pub struct NapiSearchOptions {
    pub limit: Option<u32>,
    pub threshold: Option<f64>,
}

// ============================================================================
// Agent Registry Handle
// ============================================================================

/// Handle to the agent metadata index with search capabilities
#[napi]
pub struct AgentMetadataIndexHandle {
    inner: Arc<RwLock<MetadataIndex>>,
}

#[napi]
impl AgentMetadataIndexHandle {
    /// Create a new agent metadata index
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MetadataIndex::new())),
        }
    }

    /// Create and initialize with built-in agents
    #[napi(factory)]
    pub fn with_builtins() -> Self {
        let mut index = MetadataIndex::new();
        for (_, meta) in create_builtin_metadata() {
            index.register(meta);
        }
        Self {
            inner: Arc::new(RwLock::new(index)),
        }
    }

    /// Register agent metadata
    #[napi]
    pub async fn register(&self, metadata: NapiAgentMetadata) {
        let mut index = self.inner.write().await;
        index.register(metadata.into());
    }

    /// Unregister agent metadata
    #[napi]
    pub async fn unregister(&self, name: String) -> Option<NapiAgentMetadata> {
        let mut index = self.inner.write().await;
        index.unregister(&name).map(Into::into)
    }

    /// Get metadata by name
    #[napi]
    pub async fn get(&self, name: String) -> Option<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.get(&name).cloned().map(Into::into)
    }

    /// List all metadata
    #[napi]
    pub async fn list(&self) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list().into_iter().cloned().map(Into::into).collect()
    }

    /// List metadata by category
    #[napi]
    pub async fn list_by_category(&self, category: NapiAgentCategory) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list_by_category(category.into())
            .into_iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// List metadata by mode
    #[napi]
    pub async fn list_by_mode(&self, mode: String) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list_by_mode(&mode)
            .into_iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// List metadata by role
    #[napi]
    pub async fn list_by_role(&self, role: NapiAgentRole) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list_by_role(role.into())
            .into_iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    /// List visible agents (excludes hidden)
    #[napi]
    pub async fn list_visible(&self) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list_visible().into_iter().cloned().map(Into::into).collect()
    }

    /// List recommended agents
    #[napi]
    pub async fn list_recommended(&self) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.list_recommended().into_iter().cloned().map(Into::into).collect()
    }

    /// Get primary agent for a mode
    #[napi]
    pub async fn get_primary_for_mode(&self, mode: String) -> Option<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.get_primary_for_mode(&mode).cloned().map(Into::into)
    }

    // ========================================================================
    // Search Methods
    // ========================================================================

    /// Search agents by query using fuzzy matching
    #[napi]
    pub async fn search(&self, query: String, options: Option<NapiSearchOptions>) -> Vec<NapiSearchResult> {
        let index = self.inner.read().await;
        let opts = options.map(|o| RustSearchOptions {
            limit: o.limit.unwrap_or(10) as usize,
            threshold: o.threshold.unwrap_or(0.3),
        });
        index.search(&query, opts).into_iter().map(Into::into).collect()
    }

    /// Find agents by trigger match
    #[napi]
    pub async fn find_by_trigger(&self, input: String) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.find_by_trigger(&input).into_iter().cloned().map(Into::into).collect()
    }

    /// Find agents by event trigger
    #[napi]
    pub async fn find_by_event(&self, event_name: String) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.find_by_event(&event_name).into_iter().cloned().map(Into::into).collect()
    }

    /// Find agents with a specific capability
    #[napi]
    pub async fn find_by_capability(&self, capability_id: String) -> Vec<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.find_by_capability(&capability_id).into_iter().cloned().map(Into::into).collect()
    }

    /// Recommend an agent based on user intent
    #[napi]
    pub async fn recommend(&self, intent: String) -> Option<NapiAgentMetadata> {
        let index = self.inner.read().await;
        index.recommend(&intent).cloned().map(Into::into)
    }

    /// Get agent count
    #[napi]
    pub async fn count(&self) -> u32 {
        let index = self.inner.read().await;
        index.list().len() as u32
    }
}

/// Create a new agent metadata index
#[napi]
pub fn create_agent_metadata_index() -> AgentMetadataIndexHandle {
    AgentMetadataIndexHandle::new()
}

/// Create agent metadata index with built-in agents
#[napi]
pub fn create_agent_metadata_index_with_builtins() -> AgentMetadataIndexHandle {
    AgentMetadataIndexHandle::with_builtins()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_index_handle() {
        let handle = AgentMetadataIndexHandle::with_builtins();

        let count = handle.count().await;
        assert!(count > 5);

        let build = handle.get("build".to_string()).await;
        assert!(build.is_some());
        assert_eq!(build.unwrap().name, "build");
    }

    #[tokio::test]
    async fn test_search() {
        let handle = AgentMetadataIndexHandle::with_builtins();

        let results = handle.search("code review".to_string(), None).await;
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_recommend() {
        let handle = AgentMetadataIndexHandle::with_builtins();

        let result = handle.recommend("review this code".to_string()).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "code-reviewer");
    }
}
