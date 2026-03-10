//! zero-core: Unified core library for CodeCoder
//!
//! This crate provides high-performance implementations of core functionality:
//! - **common**: Shared types, utilities, and configuration (merged from zero-common)
//! - **tools**: File operations (grep, glob, read, write, edit), shell execution
//! - **session**: Message storage, compaction, prompt templates
//! - **protocol**: MCP, LSP, JSON-RPC implementations
//! - **security**: Vault, sandbox, permissions, secrets management
//! - **foundation**: Configuration, file utilities, scheduler, memory
//! - **agent**: AI agent execution engine with tool-calling loop
//! - **agent_tools**: Tool system for AI agent capabilities
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        packages/ccode (TypeScript)                      │
//! │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                    │
//! │   │  TUI/CLI    │  │ Agent 引擎  │  │ Provider    │                    │
//! │   │ (Solid.js)  │  │ (23 Agents) │  │ (AI SDK)    │                    │
//! │   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                    │
//! │          └────────────────┼────────────────┘                            │
//! │                           │                                             │
//! │                           ▼                                             │
//! │   ┌────────────────────────────────────────────────────────────────┐   │
//! │   │  @codecoder-ai/core (NAPI-RS bindings)                         │   │
//! │   └────────────────────────────────────────────────────────────────┘   │
//! └───────────────────────────────┼─────────────────────────────────────────┘
//!                                 │ FFI (napi-rs)
//!                                 ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                           zero-core (this crate)                        │
//! │                                                                         │
//! │   ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
//! │   │  tools  │  │ session │  │ protocol │  │ security │  │foundation│  │
//! │   └─────────┘  └─────────┘  └──────────┘  └──────────┘  └──────────┘  │
//! │   ┌─────────┐  ┌─────────────┐                                        │
//! │   │  agent  │  │ agent_tools │                                        │
//! │   └─────────┘  └─────────────┘                                        │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

// Core modules
pub mod agent;
pub mod agent_tools;
pub mod audit;
pub mod autonomous;
#[cfg(feature = "browser")]
pub mod browser;
pub mod common;  // Merged from zero-common crate
pub mod context;
pub mod foundation;
pub mod git;
pub mod graph;
pub mod index;
pub mod java;
pub mod markdown;
pub mod memory;
pub mod observability;
pub mod protocol;
pub mod provider;
pub mod security;
pub mod session;
pub mod skill;
pub mod storage;
pub mod tools;
pub mod trace;
pub mod web;

#[cfg(feature = "napi-bindings")]
pub mod napi;

// Re-export commonly used types
pub use tools::{
    // Core tools and their types
    edit::{EditOperation, EditResult, Editor},
    glob::{Glob, GlobOptions, GlobResult},
    grep::{Grep, GrepMatch, GrepOptions, GrepResult},
    read::{Reader, ReadOptions, ReadResult},
    shell::{ShellCommand, ShellOutput, ShellOptions},
    shell_parser::{
        parse_shell_command, assess_commands_risk, global_parser,
        CommandRiskLevel, ParseResult as ShellParseResult, ParsedCommand,
        RiskAssessment, ShellParser, ThreadSafeShellParser,
    },
    shell_pty::{PtyConfig, PtyInfo, PtyManager, PtySession, PtyState},
    write::{Writer, WriteOptions, WriteResult},
    // Extended tools
    ls::{Ls, LsOptions, LsResult},
    truncation::{Truncator, TruncateOptions, TruncateResult},
    todo::{TodoItem, TodoList, TodoStatus, TodoSummary},
    multiedit::{FileEdit, MultiEditOptions, MultiEditResult, MultiEditor},
    apply_patch::{ApplyPatchOptions, ApplyPatchResult, PatchApplicator, PatchHunk, PatchType},
    codesearch::{CodeSearch, CodeSearchMatch, CodeSearchOptions, CodeSearchResult},
    webfetch::{HttpMethod, WebFetcher, WebFetchOptions, WebFetchResult},
};

pub use session::{
    compaction::{CompactionResult, CompactionStrategy},
    message::{Message, MessageRole, MessageStore},
    prompt::{PromptContext, PromptTemplate},
};

pub use protocol::mcp::{McpClient, McpServer, McpTool};

pub use protocol::{
    McpClientConfig, McpClientInstance, McpClientManager, McpConnectionStatus, McpTransportType,
    LspServerInfo, LspServerManager, LspServerStatus,
    LspLocation, LspSymbol, LspCompletionItem, LspTextEdit,
};

pub use security::{
    permission::{Permission, PermissionManager},
    vault::{SecretEntry, Vault},
    keyring::{Credential, CredentialManager, KeyringBackend, KeyringManager, McpAuthEntry, McpAuthStore},
};

pub use foundation::{
    config::{Config, ConfigLoader},
    file::{FileInfo, FileType},
    ignore::{
        get_default_folders, get_default_patterns, should_ignore,
        IgnoreCheckResult, IgnoreConfig, IgnoreEngine, IgnoreStats,
    },
    watcher::{FileWatcher, FileWatcherConfig, MultiWatcher, WatchEvent, WatchEventKind},
};

pub use context::{
    BuildToolInfo, ConfigFile, Fingerprint, FingerprintInfo, FrameworkInfo,
    PackageInfo, PackageManager, ProjectLanguage, TestFrameworkInfo,
    RelevanceScore, RelevanceScorer, RelevanceScorerConfig,
    // Cache types
    CacheBuilder, CacheEntry, CacheEntryType, CacheTime, ComponentCache, ComponentType,
    ConfigCache, ContextCacheStore, ProjectCache, RouteCache, RouteType,
};

pub use memory::{
    chunk_markdown, Chunk, ChunkerConfig,
    cosine_similarity, hybrid_merge, vec_to_bytes, bytes_to_vec, ScoredResult,
    KnnResult, knn_search, knn_search_indexed, batch_cosine_similarity,
    create_embedding_provider, EmbeddingProvider, EmbeddingConfig, NoopEmbedding, OpenAiEmbedding,
    // History types (FileEdit renamed to HistoryFileEdit to avoid conflict with multiedit::FileEdit)
    EditRecord, EditSession, FileEdit as HistoryFileEdit, FileEditType, EditStats, AgentStats,
    DecisionRecord, DecisionType, ArchitectureDecisionRecord, AdrStatus, Alternative,
    HistoryStore,
    // Tokenizer types
    estimate_tokens, estimate_tokens_batch, fits_token_budget, truncate_to_tokens,
    BatchCountResult, TokenCounter, TokenCounterConfig, TokenizerModel,
    // Phase 13: Unified memory system types
    MemorySystem, MemoryStats, MemorySnapshot, HistoryStats, VectorStats, TokenizerStats,
    HistorySnapshot, VectorSnapshot, ImportOptions, ImportResult, CleanupResult,
    StoredEmbedding, VectorStore, ToolDefinition, ToolMatch,
};

pub use audit::{
    AuditEntry, AuditEntryInput, AuditEntryType, AuditFilter, AuditLog, AuditReport,
    AuditResult, RiskLevel,
};

pub use storage::{EntryMeta, KVStore, StoreStats};

pub use autonomous::{
    AutonomousState, StateCategory, StateMachine, StateMachineConfig,
    StateMetadata, TransitionResult, VALID_TRANSITIONS,
    Task, TaskId, TaskPriority, TaskQueue, TaskQueueConfig, TaskQueueStats, TaskStatus,
    // Safety Guardrails (renamed to avoid conflicts)
    DecisionRecord as GuardrailDecisionRecord, GuardrailConfig, GuardrailStats, LimitType, LoopDetection,
    LoopType, SafetyCheckResult, SafetyGuardrails, StateTransition,
    ToolCall as GuardrailToolCall, ToolResult as GuardrailToolResult,
    // Safety Constraints
    ConstraintCheckResult, ResourceBudget, ResourceType, ResourceUsage, ResourceWarning,
    SafetyConfig, SafetyGuard,
    // CLOSE Decision Framework
    evaluate_close, CLOSEDimension, CLOSEEvaluation, CLOSEInput, CLOSETrend, CLOSEWeights,
    GearRecommendation,
};

pub use graph::{
    // Core engine
    EdgeData, EdgeId, GraphEngine, NodeData, NodeId,
    // Algorithms
    CycleResult, GraphAlgorithms, PathResult,
    // Causal graph
    ActionNode, CausalChain, CausalEdge, CausalGraph, CausalQuery, CausalStats,
    DecisionNode, OutcomeNode, OutcomeStatus,
    // Call graph
    CallEdge, CallGraph, CallNode, RecursionInfo,
    // Semantic graph
    SemanticEdge, SemanticEdgeType, SemanticGraph, SemanticNode, SemanticNodeType,
};

pub use java::{
    // Class file parsing
    AccessFlags, ClassFile, ClassInfo, ClassType, parse_class_file,
    // JAR handling
    JarEntry, JarReader,
    // Fingerprint engine
    Detection, FingerprintCategory, FingerprintEngine, FingerprintInput, JavaFingerprint, PatternType,
    // Analyzer
    ClassAnalysis, ConfigFileInfo, DependencyInfo, JarAnalysis, JarAnalyzer, JarMetadata, PackageAnalysis,
};

pub use trace::{
    // Storage
    TraceEntry, TraceStore, TraceStoreConfig, TraceStoreStats,
    // Query
    TraceFilter, TraceQuery,
    // Profiler
    profile_traces, FunctionStats, ProfileResult, ServiceStats, SlowOperation,
    // Aggregator
    aggregate_errors, error_rates_by_service, recent_errors,
    ErrorGroup, ErrorSample, ErrorSummary, GroupBy,
};

pub use observability::{
    // Events
    AgentLifecycleEvent, AgentLifecycleType, Event, EventType, LlmCallEvent,
    SpanEvent, SpanKind, ToolExecutionEvent, ToolStatus,
    // Metrics
    AgentMetrics, MetricsAggregator, MetricsSummary, ModelMetrics, ToolMetrics,
    // Store
    ObservabilityStore, ObservabilityStoreConfig,
};

pub use web::{
    WebCategory, WebConfidence, WebDetection, WebFingerprint, WebFingerprintEngine,
    WebFingerprintInput, WebPatternType, WEB_FINGERPRINT_ENGINE,
};

pub use markdown::{
    parse_markdown, extract_headings, extract_code_blocks, extract_links, extract_images,
    render_to_html, extract_frontmatter, strip_frontmatter,
    CodeBlock, Heading, Image, Link, MarkdownNode,
};

pub use skill::{
    parse_skill, parse_skill_file, parse_skill_metadata, validate_skill,
    ParsedSkill, SkillMetadata, SkillParseError,
};

pub use git::{
    CloneOptions, CommitInfo, CommitResult, DiffFile, DiffResult,
    FileStatus, FileStatusType, GitError, GitOpsHandle, GitResult,
    GitStatus, InitOptions, OperationResult,
};

pub use index::{
    CodeIndexer, CodeSymbol, FileIndex, IndexOptions, IndexStats, Language, LanguageParser,
    ProjectIndex, SymbolKind,
};

pub use provider::{
    apply_caching, get_sdk_key, get_temperature, get_top_k, get_top_p,
    normalize_messages, remap_provider_options,
    CacheResult, ModelInfo, NormalizeResult,
    ProviderMessage, ProviderMessageContent,
};

// Agent execution engine re-exports
pub use agent::{
    AgentExecutor, ToolCall, ToolContext, Provider,
    // Streaming types
    AnthropicProvider, ContentPart, Message as StreamMessage, Role, StreamEvent, StreamRequest,
    StreamingProvider, ToolDef, Usage,
    // Confirmation types
    ConfirmationRegistry, ConfirmationResponse, NotificationSink,
    PendingConfirmation, get_confirmation_registry, get_notification_sink,
    handle_confirmation_response, handle_confirmation_response_with_type,
    init_confirmation_registry, notify, request_confirmation_and_wait,
    set_notification_sink,
};

// Agent tools re-exports
pub use agent_tools::{
    Tool, ToolResult, ToolSpec, SecurityPolicy,
    BrowserTool, CodeCoderTool, EditTool, FileReadTool, FileWriteTool,
    GlobTool, GrepTool, MemoryForgetTool, MemoryRecallTool, MemoryStoreTool,
    ShellTool,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library with default settings
pub fn init() -> anyhow::Result<()> {
    // Initialize tracing if not already done
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
