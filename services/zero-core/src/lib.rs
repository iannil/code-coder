//! zero-core: Unified core library for CodeCoder
//!
//! This crate provides high-performance implementations of core functionality:
//! - **tools**: File operations (grep, glob, read, write, edit), shell execution
//! - **session**: Message storage, compaction, prompt templates
//! - **protocol**: MCP, LSP, JSON-RPC implementations
//! - **security**: Vault, sandbox, permissions, secrets management
//! - **foundation**: Configuration, file utilities, scheduler, memory
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
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

pub mod audit;
pub mod autonomous;
pub mod context;
pub mod foundation;
pub mod git;
pub mod graph;
pub mod java;
pub mod markdown;
pub mod memory;
pub mod protocol;
pub mod provider;
pub mod security;
pub mod session;
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
    create_embedding_provider, EmbeddingProvider, EmbeddingConfig, NoopEmbedding, OpenAiEmbedding,
    // History types (FileEdit renamed to HistoryFileEdit to avoid conflict with multiedit::FileEdit)
    EditRecord, EditSession, FileEdit as HistoryFileEdit, FileEditType, EditStats, AgentStats,
    DecisionRecord, DecisionType, ArchitectureDecisionRecord, AdrStatus, Alternative,
    HistoryStore,
    // Tokenizer types
    estimate_tokens, estimate_tokens_batch, fits_token_budget, truncate_to_tokens,
    BatchCountResult, TokenCounter, TokenCounterConfig, TokenizerModel,
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

pub use web::{
    WebCategory, WebConfidence, WebDetection, WebFingerprint, WebFingerprintEngine,
    WebFingerprintInput, WebPatternType, WEB_FINGERPRINT_ENGINE,
};

pub use markdown::{
    parse_markdown, extract_headings, extract_code_blocks, extract_links, extract_images,
    render_to_html, extract_frontmatter, strip_frontmatter,
    CodeBlock, Heading, Image, Link, MarkdownNode,
};

pub use git::{
    CloneOptions, CommitInfo, CommitResult, DiffFile, DiffResult,
    FileStatus, FileStatusType, GitError, GitOpsHandle, GitResult,
    GitStatus, InitOptions, OperationResult,
};

pub use provider::{
    apply_caching, get_sdk_key, get_temperature, get_top_k, get_top_p,
    normalize_messages, remap_provider_options,
    CacheResult, ModelInfo, NormalizeResult,
    ProviderMessage, ProviderMessageContent,
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
