//! Agent Registry
//!
//! Central registry for agent configurations with caching and lookup.
//!
//! The registry provides:
//! - Thread-safe access to agent configurations
//! - Hot-reload support for configuration changes
//! - Mode-based filtering (primary, subagent, all)
//! - Default agent selection
//!
//! ## Example
//!
//! ```rust,no_run
//! use zero_core::agent::registry::AgentRegistry;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create and initialize registry
//!     let registry = AgentRegistry::new();
//!     registry.reload().await?;
//!
//!     // Get an agent by name
//!     if let Some(agent) = registry.get("build").await {
//!         println!("Found agent: {}", agent.name);
//!     }
//!
//!     // List all primary agents
//!     let primary = registry.list_primary().await;
//!     for agent in primary {
//!         println!("Primary agent: {}", agent.name);
//!     }
//!
//!     Ok(())
//! }
//! ```

use super::loader::{AgentConfig, AgentLoader, AgentMode, LoaderError, LoaderPaths};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Registry Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Loader error: {0}")]
    Loader(#[from] LoaderError),

    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("No default agent available")]
    NoDefault,

    #[error("Invalid default agent '{0}': {1}")]
    InvalidDefault(String, String),
}

// ============================================================================
// Agent Registry
// ============================================================================

/// Thread-safe agent registry with caching
pub struct AgentRegistry {
    /// Cached agent configurations
    agents: RwLock<HashMap<String, Arc<AgentConfig>>>,
    /// Agent loader
    loader: AgentLoader,
    /// Default agent name (if configured)
    default_agent: RwLock<Option<String>>,
}

impl AgentRegistry {
    /// Create a new registry with default paths
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            loader: AgentLoader::new(),
            default_agent: RwLock::new(None),
        }
    }

    /// Create a new registry with custom paths
    pub fn with_paths(paths: LoaderPaths) -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            loader: AgentLoader::with_paths(paths),
            default_agent: RwLock::new(None),
        }
    }

    /// Create a registry with a custom base directory
    pub fn with_base(base: impl AsRef<Path>) -> Self {
        Self::with_paths(LoaderPaths::with_base(base))
    }

    /// Reload all agents from disk
    pub async fn reload(&self) -> Result<usize, RegistryError> {
        let configs = self.loader.load_all().await?;
        let count = configs.len();

        let mut agents = self.agents.write().await;
        agents.clear();

        for config in configs {
            agents.insert(config.name.clone(), Arc::new(config));
        }

        tracing::info!("Loaded {} agent configurations", count);
        Ok(count)
    }

    /// Register a built-in (native) agent configuration
    pub async fn register_native(&self, config: AgentConfig) {
        let mut agents = self.agents.write().await;
        agents.insert(config.name.clone(), Arc::new(config));
    }

    /// Register multiple native agents
    pub async fn register_natives(&self, configs: Vec<AgentConfig>) {
        let mut agents = self.agents.write().await;
        for config in configs {
            agents.insert(config.name.clone(), Arc::new(config));
        }
    }

    /// Get an agent by name
    pub async fn get(&self, name: &str) -> Option<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        agents.get(name).cloned()
    }

    /// Check if an agent exists
    pub async fn exists(&self, name: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(name)
    }

    /// List all agents
    pub async fn list(&self) -> Vec<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        let mut list: Vec<_> = agents.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// List visible agents (not hidden)
    pub async fn list_visible(&self) -> Vec<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        let mut list: Vec<_> = agents
            .values()
            .filter(|a| !a.hidden)
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// List primary agents (can be invoked by users)
    pub async fn list_primary(&self) -> Vec<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        let mut list: Vec<_> = agents
            .values()
            .filter(|a| matches!(a.mode, AgentMode::Primary | AgentMode::All))
            .filter(|a| !a.hidden)
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// List subagents (can be invoked by other agents)
    pub async fn list_subagents(&self) -> Vec<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        let mut list: Vec<_> = agents
            .values()
            .filter(|a| matches!(a.mode, AgentMode::Subagent | AgentMode::All))
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// List agents with observer capability
    pub async fn list_observers(&self) -> Vec<Arc<AgentConfig>> {
        let agents = self.agents.read().await;
        let mut list: Vec<_> = agents
            .values()
            .filter(|a| !a.observer.can_watch.is_empty())
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Get all agent names
    pub async fn names(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        let mut names: Vec<_> = agents.keys().cloned().collect();
        names.sort();
        names
    }

    /// Set the default agent
    pub async fn set_default(&self, name: Option<String>) {
        let mut default = self.default_agent.write().await;
        *default = name;
    }

    /// Get the default agent
    pub async fn default(&self) -> Result<Arc<AgentConfig>, RegistryError> {
        // Check configured default
        let default_name = self.default_agent.read().await;
        if let Some(ref name) = *default_name {
            if let Some(agent) = self.get(name).await {
                // Validate it's usable as default
                if agent.hidden {
                    return Err(RegistryError::InvalidDefault(
                        name.clone(),
                        "agent is hidden".to_string(),
                    ));
                }
                if matches!(agent.mode, AgentMode::Subagent) {
                    return Err(RegistryError::InvalidDefault(
                        name.clone(),
                        "agent is a subagent".to_string(),
                    ));
                }
                return Ok(agent);
            }
        }

        // Fall back to first primary visible agent
        let primaries = self.list_primary().await;
        primaries.into_iter().next().ok_or(RegistryError::NoDefault)
    }

    /// Save an agent configuration to disk
    pub async fn save(&self, config: &AgentConfig) -> Result<(), RegistryError> {
        self.loader.save(config).await?;

        // Update cache
        let mut agents = self.agents.write().await;
        agents.insert(config.name.clone(), Arc::new(config.clone()));

        Ok(())
    }

    /// Remove an agent by name
    pub async fn remove(&self, name: &str) -> Option<Arc<AgentConfig>> {
        let mut agents = self.agents.write().await;
        agents.remove(name)
    }

    /// Get the number of registered agents
    pub async fn len(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if the registry is empty
    pub async fn is_empty(&self) -> bool {
        let agents = self.agents.read().await;
        agents.is_empty()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Registry
// ============================================================================

use once_cell::sync::OnceCell;

static GLOBAL_REGISTRY: OnceCell<AgentRegistry> = OnceCell::new();

/// Initialize the global agent registry
pub fn init_global_registry() -> &'static AgentRegistry {
    GLOBAL_REGISTRY.get_or_init(AgentRegistry::new)
}

/// Get the global agent registry
pub fn get_global_registry() -> Option<&'static AgentRegistry> {
    GLOBAL_REGISTRY.get()
}

/// Initialize and load the global registry
pub async fn init_and_load() -> Result<&'static AgentRegistry, RegistryError> {
    let registry = init_global_registry();
    registry.reload().await?;
    Ok(registry)
}

// ============================================================================
// Built-in Agent Definitions
// ============================================================================

/// Create default built-in agent configurations
pub fn create_builtin_agents() -> Vec<AgentConfig> {
    use super::builtin_prompts::get_builtin_prompt;
    use super::loader::{
        AutoApproveConfig, ObserverCapability, PermissionAction, PermissionConfig,
        PermissionValue, RiskThreshold, ThinkingMode, WatcherType,
    };

    // Helper to get embedded prompt content
    let prompt_for = |name: &str| -> Option<String> {
        get_builtin_prompt(name).map(|s| s.to_string())
    };

    let default_permission = || {
        let mut rules = HashMap::new();
        rules.insert("default".to_string(), PermissionValue::Simple(PermissionAction::Allow));
        rules.insert("doom_loop".to_string(), PermissionValue::Simple(PermissionAction::Ask));
        rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Deny));
        rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Deny));
        rules.insert("plan_exit".to_string(), PermissionValue::Simple(PermissionAction::Deny));
        PermissionConfig { rules }
    };

    vec![
        // Build agent (primary)
        AgentConfig {
            name: "build".to_string(),
            description: Some("Main development agent for software engineering tasks".to_string()),
            mode: AgentMode::Primary,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            prompt: Some("build.md".to_string()),
            prompt_content: prompt_for("build"),
            ..Default::default()
        },
        // Plan agent (primary)
        AgentConfig {
            name: "plan".to_string(),
            description: Some("Planning agent for implementation strategies".to_string()),
            mode: AgentMode::Primary,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_exit".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            prompt: Some("plan.md".to_string()),
            prompt_content: prompt_for("plan"),
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(128_000));
                m
            },
            ..Default::default()
        },
        // Explore agent (subagent)
        AgentConfig {
            name: "explore".to_string(),
            description: Some("Fast agent for codebase exploration and search".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            permission: {
                let mut rules = HashMap::new();
                rules.insert("default".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                rules.insert("grep".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                rules.insert("glob".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                rules.insert("list".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                rules.insert("read".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                rules.insert("websearch".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                rules.insert("webfetch".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                PermissionConfig { rules }
            },
            auto_approve: AutoApproveConfig {
                enabled: true,
                allowed_tools: vec!["Read".to_string(), "Glob".to_string(), "Grep".to_string(), "LS".to_string()],
                risk_threshold: RiskThreshold::Low,
                max_approvals: None,
            },
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("explore.md".to_string()),
            prompt_content: prompt_for("explore"),
            ..Default::default()
        },
        // General agent (subagent)
        AgentConfig {
            name: "general".to_string(),
            description: Some("General-purpose agent for multi-step tasks".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("todoread".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                p.rules.insert("todowrite".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                p
            },
            auto_approve: AutoApproveConfig {
                enabled: true,
                allowed_tools: vec!["Read".to_string(), "Glob".to_string(), "Grep".to_string(), "LS".to_string()],
                risk_threshold: RiskThreshold::Safe,
                max_approvals: None,
            },
            prompt: Some("general.md".to_string()),
            prompt_content: prompt_for("general"),
            ..Default::default()
        },
        // Code reviewer (subagent)
        AgentConfig {
            name: "code-reviewer".to_string(),
            description: Some("Performs comprehensive code quality reviews".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("code-reviewer.md".to_string()),
            prompt_content: prompt_for("code-reviewer"),
            ..Default::default()
        },
        // Security reviewer (subagent)
        AgentConfig {
            name: "security-reviewer".to_string(),
            description: Some("Analyzes code for security vulnerabilities".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("security-reviewer.md".to_string()),
            prompt_content: prompt_for("security-reviewer"),
            ..Default::default()
        },
        // TDD guide (subagent)
        AgentConfig {
            name: "tdd-guide".to_string(),
            description: Some("Enforces test-driven development methodology".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("tdd-guide.md".to_string()),
            prompt_content: prompt_for("tdd-guide"),
            ..Default::default()
        },
        // Architect (subagent)
        AgentConfig {
            name: "architect".to_string(),
            description: Some("Designs system architecture and patterns".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("architect.md".to_string()),
            prompt_content: prompt_for("architect"),
            ..Default::default()
        },
        // Writer (primary)
        AgentConfig {
            name: "writer".to_string(),
            description: Some("Specialized agent for writing long-form content".to_string()),
            mode: AgentMode::Primary,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            thinking: ThinkingMode::Disabled,
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(128_000));
                m
            },
            prompt: Some("writer.md".to_string()),
            prompt_content: prompt_for("writer"),
            color: Some("blue".to_string()),
            ..Default::default()
        },
        // Observer (subagent) - Zhurong philosophy
        AgentConfig {
            name: "observer".to_string(),
            description: Some("Zhurong philosophy observer theory analyst".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Meta],
                contribute_to_consensus: true,
                report_to_meta: false, // MetaWatch doesn't report to itself
            },
            prompt: Some("observer.md".to_string()),
            prompt_content: prompt_for("observer"),
            ..Default::default()
        },
        // Decision (subagent) - CLOSE framework
        AgentConfig {
            name: "decision".to_string(),
            description: Some("Decision advisor using CLOSE evaluation framework".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("decision.md".to_string()),
            prompt_content: prompt_for("decision"),
            ..Default::default()
        },
        // Macro (subagent) - Macroeconomic analysis
        AgentConfig {
            name: "macro".to_string(),
            description: Some("Macroeconomic analyst for GDP, policy data".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("macro.md".to_string()),
            prompt_content: prompt_for("macro"),
            ..Default::default()
        },
        // Trader (subagent)
        AgentConfig {
            name: "trader".to_string(),
            description: Some("Short-term trading technical analysis guide".to_string()),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("trader.md".to_string()),
            prompt_content: prompt_for("trader"),
            ..Default::default()
        },
        // Autonomous (primary)
        AgentConfig {
            name: "autonomous".to_string(),
            description: Some("Fully autonomous execution agent with CLOSE decision framework".to_string()),
            mode: AgentMode::Primary,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_exit".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("doom_loop".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                p.rules.insert("websearch".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("webfetch".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            thinking: ThinkingMode::Disabled,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(128_000));
                m
            },
            prompt: Some("autonomous.md".to_string()),
            prompt_content: prompt_for("autonomous"),
            color: Some("magenta".to_string()),
            ..Default::default()
        },
        // Hidden system agents
        AgentConfig {
            name: "compaction".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            permission: {
                let mut rules = HashMap::new();
                rules.insert("default".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                PermissionConfig { rules }
            },
            prompt: Some("compaction.md".to_string()),
            prompt_content: prompt_for("compaction"),
            ..Default::default()
        },
        AgentConfig {
            name: "title".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            permission: {
                let mut rules = HashMap::new();
                rules.insert("default".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                PermissionConfig { rules }
            },
            prompt: Some("title.md".to_string()),
            prompt_content: prompt_for("title"),
            ..Default::default()
        },
        AgentConfig {
            name: "summary".to_string(),
            description: None,
            mode: AgentMode::Primary,
            native: true,
            hidden: true,
            permission: {
                let mut rules = HashMap::new();
                rules.insert("default".to_string(), PermissionValue::Simple(PermissionAction::Deny));
                PermissionConfig { rules }
            },
            prompt: Some("summary.md".to_string()),
            prompt_content: prompt_for("summary"),
            ..Default::default()
        },
        // Expander (subagent) - unified content expansion
        AgentConfig {
            name: "expander".to_string(),
            description: Some(
                "Unified content expansion specialist for fiction, non-fiction, and general content. \
                Transforms ideas into comprehensive books through systematic framework building."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            thinking: ThinkingMode::Disabled,
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(128_000));
                m
            },
            prompt: Some("expander.md".to_string()),
            prompt_content: prompt_for("expander"),
            color: Some("blue".to_string()),
            ..Default::default()
        },
        // Proofreader (subagent)
        AgentConfig {
            name: "proofreader".to_string(),
            description: Some(
                "Proofreading specialist for long-form text. Checks grammar, spelling, \
                punctuation, style, terminology, flow, readability using the PROOF framework."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            thinking: ThinkingMode::Disabled,
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(128_000));
                m
            },
            prompt: Some("proofreader.md".to_string()),
            prompt_content: prompt_for("proofreader"),
            ..Default::default()
        },
        // Code-reverse (subagent) - website reverse engineering
        AgentConfig {
            name: "code-reverse".to_string(),
            description: Some(
                "Website reverse engineering agent for pixel-perfect recreation planning. \
                Analyzes websites, identifies technology stacks, extracts design systems."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_exit".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            prompt: Some("code-reverse.md".to_string()),
            prompt_content: prompt_for("code-reverse"),
            color: Some("cyan".to_string()),
            ..Default::default()
        },
        // JAR-code-reverse (subagent) - Java JAR reverse engineering
        AgentConfig {
            name: "jar-code-reverse".to_string(),
            description: Some(
                "JAR reverse engineering agent for Java source code reconstruction. \
                Analyzes JAR files, identifies frameworks and libraries, extracts class structure."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            permission: {
                let mut p = default_permission();
                p.rules.insert("question".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_enter".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p.rules.insert("plan_exit".to_string(), PermissionValue::Simple(PermissionAction::Allow));
                p
            },
            prompt: Some("jar-code-reverse.md".to_string()),
            prompt_content: prompt_for("jar-code-reverse"),
            color: Some("magenta".to_string()),
            ..Default::default()
        },
        // Picker (subagent) - product selection expert
        AgentConfig {
            name: "picker".to_string(),
            description: Some(
                "Product selection expert using '爆品之眼' methodology. \
                Identifies market opportunities using seven deadly sins selection method."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("picker.md".to_string()),
            prompt_content: prompt_for("picker"),
            ..Default::default()
        },
        // Miniproduct (subagent) - indie product coach
        AgentConfig {
            name: "miniproduct".to_string(),
            description: Some(
                "Indie product coach for building profitable software products from 0 to 1. \
                Covers requirement validation, AI-assisted development, monetization strategies."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            prompt: Some("miniproduct.md".to_string()),
            prompt_content: prompt_for("miniproduct"),
            ..Default::default()
        },
        // Synton-assistant (subagent) - SYNTON-DB helper
        AgentConfig {
            name: "synton-assistant".to_string(),
            description: Some(
                "SYNTON-DB assistant for understanding and using the LLM-designed memory database. \
                Includes tensor graph storage, PaQL queries, and Graph-RAG retrieval."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            prompt: Some("synton-assistant.md".to_string()),
            prompt_content: prompt_for("synton-assistant"),
            ..Default::default()
        },
        // AI-engineer (subagent) - AI engineering mentor
        AgentConfig {
            name: "ai-engineer".to_string(),
            description: Some(
                "AI engineer mentor covering Python basics to LLM application development, \
                RAG system construction, fine-tuning, and performance optimization."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            prompt: Some("ai-engineer.md".to_string()),
            prompt_content: prompt_for("ai-engineer"),
            ..Default::default()
        },
        // Value-analyst (subagent) - value analysis expert
        AgentConfig {
            name: "value-analyst".to_string(),
            description: Some(
                "Value analyst using observer construction framework from '价值逻辑'. \
                Analyzes national consensus, business evaluation rights, and financial realities."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::World],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("value-analyst.md".to_string()),
            prompt_content: prompt_for("value-analyst"),
            ..Default::default()
        },
        // Verifier (subagent) - comprehensive verification
        AgentConfig {
            name: "verifier".to_string(),
            description: Some(
                "Verification agent for comprehensive validation. Performs build check, type check, \
                lint, test suite, console.log audit, git status, formal methods, and coverage analysis."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Self_],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("verifier.md".to_string()),
            prompt_content: prompt_for("verifier"),
            ..Default::default()
        },
        // PRD-generator (subagent) - product requirement document generator
        AgentConfig {
            name: "prd-generator".to_string(),
            description: Some(
                "PRD generator that transforms meeting notes or requirement discussions \
                into structured documents with user analysis, functional requirements, \
                interaction design, technical solutions, and development plans."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            options: {
                let mut m = HashMap::new();
                m.insert("maxOutputTokens".to_string(), serde_json::json!(64_000));
                m
            },
            prompt: Some("prd-generator.md".to_string()),
            prompt_content: prompt_for("prd-generator"),
            color: Some("blue".to_string()),
            ..Default::default()
        },
        // Feasibility-assess (subagent) - technical feasibility assessment
        AgentConfig {
            name: "feasibility-assess".to_string(),
            description: Some(
                "Technical feasibility assessment expert. Analyzes requirement complexity, \
                existing capabilities, change lists, dependencies, and risks based on \
                codebase semantic graph. Outputs structured JSON assessment reports."
                    .to_string(),
            ),
            mode: AgentMode::Subagent,
            native: true,
            observer: ObserverCapability {
                can_watch: vec![WatcherType::Code],
                contribute_to_consensus: true,
                report_to_meta: true,
            },
            prompt: Some("feasibility-assess.md".to_string()),
            prompt_content: prompt_for("feasibility-assess"),
            color: Some("yellow".to_string()),
            ..Default::default()
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_basic() {
        let registry = AgentRegistry::new();

        // Register native agents
        let natives = create_builtin_agents();
        registry.register_natives(natives).await;

        // Check count
        assert!(registry.len().await > 0);

        // Get specific agent
        let build = registry.get("build").await;
        assert!(build.is_some());
        assert_eq!(build.unwrap().name, "build");

        // Check exists
        assert!(registry.exists("build").await);
        assert!(!registry.exists("nonexistent").await);
    }

    #[tokio::test]
    async fn test_list_by_mode() {
        let registry = AgentRegistry::new();
        registry.register_natives(create_builtin_agents()).await;

        let primary = registry.list_primary().await;
        assert!(primary.iter().all(|a| matches!(a.mode, AgentMode::Primary | AgentMode::All)));

        let subagents = registry.list_subagents().await;
        assert!(subagents.iter().all(|a| matches!(a.mode, AgentMode::Subagent | AgentMode::All)));
    }

    #[tokio::test]
    async fn test_list_visible() {
        let registry = AgentRegistry::new();
        registry.register_natives(create_builtin_agents()).await;

        let visible = registry.list_visible().await;
        let all = registry.list().await;

        // Should have fewer visible than total (hidden agents filtered)
        assert!(visible.len() < all.len());
        assert!(visible.iter().all(|a| !a.hidden));
    }

    #[tokio::test]
    async fn test_list_observers() {
        let registry = AgentRegistry::new();
        registry.register_natives(create_builtin_agents()).await;

        let observers = registry.list_observers().await;
        assert!(!observers.is_empty());
        assert!(observers.iter().all(|a| !a.observer.can_watch.is_empty()));
    }

    #[tokio::test]
    async fn test_default_agent() {
        let registry = AgentRegistry::new();
        registry.register_natives(create_builtin_agents()).await;

        // Default should return first primary visible agent
        let default = registry.default().await;
        assert!(default.is_ok());

        // Set explicit default
        registry.set_default(Some("plan".to_string())).await;
        let default = registry.default().await.unwrap();
        assert_eq!(default.name, "plan");

        // Setting hidden agent as default should fail
        registry.set_default(Some("compaction".to_string())).await;
        let result = registry.default().await;
        assert!(result.is_err());
    }
}
