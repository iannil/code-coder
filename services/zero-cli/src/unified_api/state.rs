//! Unified API State
//!
//! This module provides shared state for the unified API server.
//! It manages connections to core services and caches for performance.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::gear::GearState;
use crate::observer::network::ObserverNetworkState;
use crate::observer::WatcherManager;
use crate::session::store::SessionStore;
use crate::tools::ToolRegistry;

// Re-export streaming types from zero-core for agent execution
pub use zero_core::agent::{
    AnthropicProvider, ContentPart, Message, Role, StreamEvent, StreamRequest, StreamingProvider,
    ToolDef,
};

/// Cached agent metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMetadata {
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub temperature: Option<f64>,
    pub color: Option<String>,
    pub hidden: bool,
    /// Last modification time of the prompt file
    pub prompt_modified_at: Option<DateTime<Utc>>,
    /// Cached prompt content
    pub prompt: Option<String>,
}

/// Unified API state shared across all handlers
#[derive(Clone)]
pub struct UnifiedApiState {
    /// Session store for conversation persistence
    pub sessions: Arc<SessionStore>,

    /// Tool registry (native + MCP tools)
    pub tools: Arc<RwLock<ToolRegistry>>,

    /// Agent metadata cache (name -> metadata)
    pub agents: Arc<RwLock<HashMap<String, AgentMetadata>>>,

    /// Custom agent definitions (runtime overrides and user-defined agents)
    pub custom_agents: Arc<RwLock<HashMap<String, super::definitions::AgentDefinition>>>,

    /// Gear control state (P/N/D/S/M presets + three dials)
    pub gear: GearState,

    /// Path to prompt files directory (TS source)
    pub prompts_dir: PathBuf,

    /// Workspace directory
    pub workspace_dir: PathBuf,

    /// Server start time
    pub started_at: DateTime<Utc>,

    /// API version
    pub version: &'static str,

    /// LLM streaming provider for agent execution
    pub llm_provider: Option<Arc<dyn StreamingProvider>>,

    /// Observer network state
    pub observer: Option<ObserverNetworkState>,

    /// Watcher manager for controlling individual watchers
    pub watcher_manager: Option<Arc<RwLock<WatcherManager>>>,
}

impl UnifiedApiState {
    /// Create new unified API state
    pub fn new(
        sessions: Arc<SessionStore>,
        tools: Arc<RwLock<ToolRegistry>>,
        prompts_dir: PathBuf,
        workspace_dir: PathBuf,
    ) -> Self {
        Self {
            sessions,
            tools,
            agents: Arc::new(RwLock::new(HashMap::new())),
            custom_agents: Arc::new(RwLock::new(HashMap::new())),
            gear: GearState::new(),
            prompts_dir,
            workspace_dir,
            started_at: Utc::now(),
            version: super::VERSION,
            llm_provider: None,
            observer: None,
            watcher_manager: None,
        }
    }

    /// Create new unified API state with LLM provider
    pub fn with_provider(
        sessions: Arc<SessionStore>,
        tools: Arc<RwLock<ToolRegistry>>,
        prompts_dir: PathBuf,
        workspace_dir: PathBuf,
        provider: Arc<dyn StreamingProvider>,
    ) -> Self {
        Self {
            sessions,
            tools,
            agents: Arc::new(RwLock::new(HashMap::new())),
            custom_agents: Arc::new(RwLock::new(HashMap::new())),
            gear: GearState::new(),
            prompts_dir,
            workspace_dir,
            started_at: Utc::now(),
            version: super::VERSION,
            llm_provider: Some(provider),
            observer: None,
            watcher_manager: None,
        }
    }

    /// Set the LLM provider (for lazy initialization)
    pub fn set_provider(&mut self, provider: Arc<dyn StreamingProvider>) {
        self.llm_provider = Some(provider);
    }

    /// Set the observer network state (for lazy initialization)
    pub fn set_observer(&mut self, observer: ObserverNetworkState) {
        self.observer = Some(observer);
    }

    /// Set the watcher manager (for lazy initialization)
    pub fn set_watcher_manager(&mut self, manager: WatcherManager) {
        self.watcher_manager = Some(Arc::new(RwLock::new(manager)));
    }

    /// Load agent metadata from prompt files
    pub async fn load_agents(&self) -> anyhow::Result<()> {
        use std::fs;
        use tracing::info;

        let prompt_dir = &self.prompts_dir;
        if !prompt_dir.exists() {
            tracing::warn!("Prompt directory not found: {}", prompt_dir.display());
            return Ok(());
        }

        let mut agents = self.agents.write().await;
        agents.clear();

        // Read all .txt files in the prompt directory
        for entry in fs::read_dir(prompt_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "txt") {
                let file_name = path.file_stem().unwrap_or_default().to_string_lossy();
                let name = file_name.to_string();

                // Read prompt content
                let content = fs::read_to_string(&path)?;

                // Parse metadata from first few lines (YAML-like frontmatter)
                let metadata = parse_prompt_metadata(&name, &content, &path);

                info!("Loaded agent: {}", name);
                agents.insert(name, metadata);
            }
        }

        info!("Loaded {} agents", agents.len());
        Ok(())
    }

    /// Reload a single agent's prompt
    pub async fn reload_agent(&self, name: &str) -> anyhow::Result<()> {
        let path = self.prompts_dir.join(format!("{}.txt", name));
        if !path.exists() {
            anyhow::bail!("Prompt file not found: {}", path.display());
        }

        let content = std::fs::read_to_string(&path)?;
        let metadata = parse_prompt_metadata(name, &content, &path);

        let mut agents = self.agents.write().await;
        agents.insert(name.to_string(), metadata);

        Ok(())
    }

    /// Get agent metadata by name
    pub async fn get_agent(&self, name: &str) -> Option<AgentMetadata> {
        let agents = self.agents.read().await;
        agents.get(name).cloned()
    }

    /// List all agents
    pub async fn list_agents(&self) -> Vec<AgentMetadata> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}

/// Parse metadata from prompt file content
fn parse_prompt_metadata(name: &str, content: &str, path: &std::path::Path) -> AgentMetadata {
    let modified_at = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| DateTime::<Utc>::from(t));

    // Try to extract metadata from the first comment block
    // Format: <!-- key: value --> or # key: value at the start
    let mut description = None;
    let mut mode = "subagent".to_string();
    let mut temperature = None;
    let mut color = None;
    let mut hidden = false;

    // Simple parsing: look for metadata patterns
    for line in content.lines().take(20) {
        let line = line.trim();

        if line.starts_with("<!-- description:") {
            description = line
                .strip_prefix("<!-- description:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim().to_string());
        } else if line.starts_with("<!-- mode:") {
            if let Some(m) = line
                .strip_prefix("<!-- mode:")
                .and_then(|s| s.strip_suffix("-->"))
            {
                mode = m.trim().to_string();
            }
        } else if line.starts_with("<!-- temperature:") {
            temperature = line
                .strip_prefix("<!-- temperature:")
                .and_then(|s| s.strip_suffix("-->"))
                .and_then(|s| s.trim().parse().ok());
        } else if line.starts_with("<!-- color:") {
            color = line
                .strip_prefix("<!-- color:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim().to_string());
        } else if line.starts_with("<!-- hidden:") {
            hidden = line
                .strip_prefix("<!-- hidden:")
                .and_then(|s| s.strip_suffix("-->"))
                .map(|s| s.trim() == "true")
                .unwrap_or(false);
        }
    }

    AgentMetadata {
        name: name.to_string(),
        description,
        mode,
        temperature,
        color,
        hidden,
        prompt_modified_at: modified_at,
        prompt: Some(content.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_prompt_metadata() {
        let content = r#"<!-- description: A test agent -->
<!-- mode: primary -->
<!-- temperature: 0.7 -->
<!-- color: blue -->
<!-- hidden: false -->

You are a helpful assistant.
"#;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, content).unwrap();

        let metadata = parse_prompt_metadata("test", content, &path);

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.description, Some("A test agent".to_string()));
        assert_eq!(metadata.mode, "primary");
        assert_eq!(metadata.temperature, Some(0.7));
        assert_eq!(metadata.color, Some("blue".to_string()));
        assert!(!metadata.hidden);
    }

    #[test]
    fn test_parse_prompt_metadata_minimal() {
        let content = "You are a helpful assistant.";
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("minimal.txt");
        std::fs::write(&path, content).unwrap();

        let metadata = parse_prompt_metadata("minimal", content, &path);

        assert_eq!(metadata.name, "minimal");
        assert!(metadata.description.is_none());
        assert_eq!(metadata.mode, "subagent");
        assert!(metadata.temperature.is_none());
    }
}
