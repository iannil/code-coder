# Omni-Nexus Phase 1 & 2 Design Document

**Date:** 2026-02-24
**Status:** Approved
**Author:** Claude + Human

---

## 1. Overview

This document details the technical design for implementing Phase 1 (自主求解闭环) and Phase 2 (全局上下文枢纽) of the Omni-Nexus architecture, as defined in `docs/standards/goals.md`.

### 1.1 Goals

- **Phase 1:** Implement autonomous solving loop with Docker sandbox execution, self-reflection, and knowledge crystallization
- **Phase 2:** Implement global context hub with Qdrant vector database integration and hybrid search API

### 1.2 Non-Goals

- JetBrains/VSCode plugin implementation (deferred)
- Phase 3-6 implementation (separate design docs)

---

## 2. Phase 1: 自主求解闭环 (Autonomous Solving Loop)

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Autonomous Execution Engine                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐   │
│  │ Code Gen    │────▶│ Docker      │────▶│ Result          │   │
│  │ (CodeCoder) │     │ Sandbox     │     │ Analyzer        │   │
│  └─────────────┘     │ (bollard)   │     └────────┬────────┘   │
│        ▲             └─────────────┘              │            │
│        │                                          │            │
│        │         ┌────────────────────────────────┘            │
│        │         │                                              │
│        │    Exit Code != 0?                                     │
│        │         │                                              │
│        │    YES  ▼                                              │
│        └─────── Self-Reflection ──────────────────────────────▶│
│                 (analyze stderr)               Exit Code = 0   │
│                                                      │         │
│                                                      ▼         │
│                                             ┌───────────────┐  │
│                                             │ Crystallize   │  │
│                                             │ (save to KB)  │  │
│                                             └───────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Components

#### 2.2.1 Docker Sandbox (`services/zero-cli/src/sandbox/`)

**Files:**
- `mod.rs` - Module entry point
- `docker.rs` - bollard-based Docker execution
- `types.rs` - Type definitions

**Interface:**

```rust
// types.rs
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Docker image to use for execution
    pub image: String,
    /// Memory limit in bytes (default: 256MB)
    pub memory_limit: u64,
    /// CPU quota (default: 1.0 = 100%)
    pub cpu_quota: f64,
    /// Network access enabled (default: false for security)
    pub network_enabled: bool,
    /// Maximum execution time
    pub timeout: Duration,
    /// Working directory inside container
    pub workdir: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: "python:3.11-slim".into(),
            memory_limit: 256 * 1024 * 1024, // 256MB
            cpu_quota: 1.0,
            network_enabled: false,
            timeout: Duration::from_secs(60),
            workdir: "/workspace".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Python,
    JavaScript,
    Shell,
    Rust,
}

impl Language {
    pub fn default_image(&self) -> &'static str {
        match self {
            Language::Python => "python:3.11-slim",
            Language::JavaScript => "node:20-slim",
            Language::Shell => "alpine:3.19",
            Language::Rust => "rust:1.75-slim",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::Shell => "sh",
            Language::Rust => "rs",
        }
    }
}
```

```rust
// docker.rs
use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};

pub struct DockerSandbox {
    client: Docker,
    config: SandboxConfig,
}

impl DockerSandbox {
    /// Create a new Docker sandbox with default config
    pub async fn new() -> Result<Self>;

    /// Create with custom config
    pub async fn with_config(config: SandboxConfig) -> Result<Self>;

    /// Execute code in isolated container
    pub async fn execute(&self, code: &str, language: Language) -> Result<ExecutionResult>;

    /// Execute with custom timeout
    pub async fn execute_with_timeout(
        &self,
        code: &str,
        language: Language,
        timeout: Duration
    ) -> Result<ExecutionResult>;

    /// Check if Docker is available
    pub async fn health_check(&self) -> bool;
}
```

**Security Considerations:**
- Network disabled by default
- Memory and CPU limits enforced
- No volume mounts to host filesystem
- Container runs as non-root user
- Containers are removed after execution

#### 2.2.2 Knowledge Crystallization (`services/zero-cli/src/memory/crystallize.rs`)

**Purpose:** Extract successful solutions and store them for future retrieval.

```rust
pub struct Crystallizer {
    memory: Arc<dyn Memory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedKnowledge {
    /// Unique identifier
    pub id: String,
    /// Original problem description
    pub problem: String,
    /// Error messages encountered
    pub errors: Vec<String>,
    /// Final successful solution
    pub solution: String,
    /// Language used
    pub language: Language,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Number of retry attempts
    pub retry_count: u32,
}

impl Crystallizer {
    /// Create crystallizer with memory backend
    pub fn new(memory: Arc<dyn Memory>) -> Self;

    /// Extract and store knowledge from successful execution
    pub async fn crystallize(
        &self,
        problem: &str,
        attempts: &[ExecutionAttempt],
        final_result: &ExecutionResult,
    ) -> Result<CrystallizedKnowledge>;

    /// Search for relevant prior solutions
    pub async fn search_solutions(&self, problem: &str, limit: usize) -> Result<Vec<CrystallizedKnowledge>>;
}
```

#### 2.2.3 Autonomous Loop (`packages/ccode/src/agent/autonomous-loop.ts`)

**Purpose:** Orchestrate the 4-step evolution loop in TypeScript.

```typescript
interface AutonomousLoopConfig {
  maxRetries: number       // Default: 10
  timeout: Duration        // Default: 5 minutes
  sandbox: SandboxConfig
}

interface ExecutionAttempt {
  code: string
  language: Language
  result: ExecutionResult
  reflection?: string
  timestamp: Date
}

class AutonomousLoop {
  constructor(config: AutonomousLoopConfig)

  /**
   * Execute the 4-step evolution loop:
   * 1. Generate code for the task
   * 2. Execute in Docker sandbox
   * 3. If failed, reflect on stderr and retry
   * 4. If succeeded, crystallize knowledge
   */
  async execute(task: string): Promise<AutonomousResult>

  /**
   * Analyze stderr and generate reflection prompt
   */
  private analyzeError(stderr: string): string

  /**
   * Check if error is recoverable
   */
  private isRecoverableError(result: ExecutionResult): boolean
}

interface AutonomousResult {
  success: boolean
  attempts: ExecutionAttempt[]
  finalCode?: string
  crystallizedId?: string
  totalDuration: Duration
}
```

### 2.3 Data Flow

```
User Request
    │
    ▼
┌─────────────────┐
│ CodeCoder Agent │ ─── Generate code based on task
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Docker Sandbox  │ ─── Execute code in isolation
└────────┬────────┘
         │
    ┌────┴────┐
    │ Exit=0? │
    └────┬────┘
         │
    NO   │   YES
    ▼    │    ▼
┌────────┴────────┐    ┌─────────────────┐
│ Self-Reflection │    │ Crystallize     │
│ (analyze stderr)│    │ (save to memory)│
└────────┬────────┘    └────────┬────────┘
         │                      │
         │                      ▼
    Retry ◀───────────── Return Success
    (max 10)
```

### 2.4 Error Handling

| Error Type | Handling Strategy |
|------------|-------------------|
| Docker unavailable | Fall back to local subprocess with security warnings |
| Timeout exceeded | Kill container, report timeout in result |
| Memory limit | Kill container, suggest memory-efficient alternatives |
| Syntax error | Extract error line, focus reflection on syntax |
| Runtime error | Analyze stack trace, suggest fixes |
| Max retries | Return failure with all attempts for human review |

### 2.5 Testing Strategy

- **Unit Tests:** Mock Docker client, test code generation logic
- **Integration Tests:** Real Docker execution with simple scripts
- **E2E Tests:** Full loop from task to crystallization
- **Coverage Target:** 80%

---

## 3. Phase 2: 全局上下文枢纽 (Global Context Hub)

### 3.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Global Context Hub                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐                     ┌───────────────────────┐ │
│  │ MD Memory    │◀───── Sync ────────▶│ Qdrant Vector DB     │ │
│  │ (daily/*.md) │                     │ (semantic search)    │ │
│  │ (MEMORY.md)  │                     └───────────────────────┘ │
│  └──────────────┘                                │              │
│         │                                        │              │
│         │                                        │              │
│         ▼                                        ▼              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Hybrid Search Engine                         │  │
│  │  ┌─────────────┐    ┌─────────────┐    ┌──────────────┐  │  │
│  │  │ Keyword     │    │ Vector      │    │ Hybrid       │  │  │
│  │  │ (FTS5/BM25) │    │ (Cosine)    │    │ Merge        │  │  │
│  │  └─────────────┘    └─────────────┘    └──────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │            Context Retrieval API (zero-gateway)          │  │
│  │                                                          │  │
│  │  GET  /api/v1/context/search?q=...&limit=...&category=.. │  │
│  │  POST /api/v1/context/ingest                             │  │
│  │  GET  /api/v1/context/:id                                │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Components

#### 3.2.1 Qdrant Integration (`services/zero-memory/src/qdrant.rs`)

**Dependencies:** `qdrant-client` crate

```rust
use qdrant_client::prelude::*;
use qdrant_client::qdrant::{vectors_config, VectorParams, Distance};

pub struct QdrantMemory {
    client: QdrantClient,
    collection: String,
    embedding_provider: Box<dyn EmbeddingProvider>,
    dimension: usize,
}

impl QdrantMemory {
    /// Connect to Qdrant server
    pub async fn connect(url: &str, collection: &str, embedding: Box<dyn EmbeddingProvider>) -> Result<Self>;

    /// Ensure collection exists with proper schema
    pub async fn ensure_collection(&self) -> Result<()>;

    /// Store content with automatic embedding generation
    pub async fn store(&self, content: &str, metadata: MemoryMetadata) -> Result<String>;

    /// Semantic vector search
    pub async fn search(&self, query: &str, limit: usize, filter: Option<Filter>) -> Result<Vec<MemoryEntry>>;

    /// Delete entry by ID
    pub async fn delete(&self, id: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub category: MemoryCategory,
    pub source: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
}
```

#### 3.2.2 Hybrid Search (`services/zero-memory/src/hybrid_search.rs`)

**Purpose:** Combine keyword (BM25) and vector (cosine) search results.

```rust
pub struct HybridSearchEngine {
    qdrant: Arc<QdrantMemory>,
    sqlite: Arc<SqliteMemory>,
    vector_weight: f32,
    keyword_weight: f32,
}

impl HybridSearchEngine {
    pub fn new(
        qdrant: Arc<QdrantMemory>,
        sqlite: Arc<SqliteMemory>,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Self;

    /// Perform hybrid search combining vector and keyword results
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        category_filter: Option<MemoryCategory>,
    ) -> Result<Vec<ScoredResult>>;
}
```

#### 3.2.3 Context API (`services/zero-gateway/src/context.rs`)

**Endpoints:**

```rust
use axum::{Router, routing::{get, post}, extract::{Query, Path, State, Json}};

pub fn context_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/context/search", get(search_context))
        .route("/api/v1/context/ingest", post(ingest_context))
        .route("/api/v1/context/:id", get(get_context))
        .route("/api/v1/context/categories", get(list_categories))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<ContextEntry>,
    pub total: usize,
    pub query_time_ms: u64,
}

#[derive(Deserialize)]
pub struct IngestRequest {
    pub content: String,
    pub category: MemoryCategory,
    pub source: String,
    pub tags: Option<Vec<String>>,
}

/// Search context with RBAC filtering
async fn search_context(
    State(state): State<AppState>,
    claims: AuthClaims,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError>;

/// Ingest new content
async fn ingest_context(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(request): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, ApiError>;
```

### 3.3 Data Flow

```
Ingest Flow:

  Content ──▶ Chunk ──▶ Embed ──▶ Store in Qdrant
                  │                    │
                  └──▶ Store in SQLite ◀┘
                       (FTS5 index)

Search Flow:

  Query ──┬──▶ Embed ──▶ Qdrant Vector Search ──┐
          │                                      │
          └──▶ SQLite FTS5 Keyword Search ──────┼──▶ Hybrid Merge ──▶ Results
                                                │
                                    RBAC Filter ◀┘
```

### 3.4 Memory Synchronization

**Bidirectional Sync Strategy:**

1. **MD → Qdrant:** On file change (via fsnotify), parse MD, chunk, embed, upsert
2. **Qdrant → MD:** On API ingest, also write to appropriate MD file for human readability
3. **Conflict Resolution:** Qdrant is source of truth for search; MD files for human editing

```rust
pub struct MemorySynchronizer {
    qdrant: Arc<QdrantMemory>,
    markdown: Arc<MarkdownMemory>,
    watcher: notify::Watcher,
}

impl MemorySynchronizer {
    pub async fn start_watching(&self) -> Result<()>;
    pub async fn sync_file(&self, path: &Path) -> Result<()>;
    pub async fn full_sync(&self) -> Result<SyncReport>;
}
```

### 3.5 Permission Model

| Category | Required Role | Description |
|----------|---------------|-------------|
| `conversation` | `user` | General conversation history |
| `daily` | `user` | Daily notes and logs |
| `project` | `developer` | Project-specific context |
| `finance` | `analyst` | Financial data and analysis |
| `security` | `admin` | Security-related information |
| `system` | `admin` | System configuration |

### 3.6 Error Handling

| Error Type | Handling Strategy |
|------------|-------------------|
| Qdrant unavailable | Fall back to SQLite-only search |
| Embedding API failure | Queue for retry, return cached if available |
| Permission denied | Return 403 with reason |
| Invalid category | Return 400 with valid categories list |

### 3.7 Testing Strategy

- **Unit Tests:** Mock Qdrant client, test search logic
- **Integration Tests:** Real Qdrant container, test ingestion and search
- **E2E Tests:** Full API flow with authentication
- **Coverage Target:** 80%

---

## 4. Implementation Plan

### 4.1 Phase 1 Tasks (Priority: P0)

| Task | Estimated Effort | Dependencies |
|------|------------------|--------------|
| Create sandbox module structure | 0.5 day | None |
| Implement DockerSandbox with bollard | 2 days | Docker installed |
| Add sandbox config to daemon | 0.5 day | DockerSandbox |
| Implement Crystallizer | 1 day | Memory trait |
| Implement AutonomousLoop (TS) | 1.5 days | DockerSandbox API |
| Write unit tests | 1 day | All components |
| Write integration tests | 1 day | All components |

**Total: ~7.5 days**

### 4.2 Phase 2 Tasks (Priority: P0)

| Task | Estimated Effort | Dependencies |
|------|------------------|--------------|
| Add qdrant-client dependency | 0.5 day | None |
| Implement QdrantMemory | 2 days | Qdrant running |
| Implement HybridSearchEngine | 1 day | QdrantMemory |
| Add context API routes | 1.5 days | HybridSearchEngine |
| Implement MemorySynchronizer | 1.5 days | Both memories |
| Write unit tests | 1 day | All components |
| Write integration tests | 1 day | All components |

**Total: ~8.5 days**

---

## 5. File Structure

```
services/
├── zero-cli/
│   └── src/
│       ├── sandbox/
│       │   ├── mod.rs           # [NEW] Module entry
│       │   ├── docker.rs        # [NEW] bollard-based execution
│       │   └── types.rs         # [NEW] Type definitions
│       └── memory/
│           └── crystallize.rs   # [NEW] Knowledge extraction
├── zero-memory/
│   └── src/
│       ├── qdrant.rs            # [NEW] Qdrant client
│       └── hybrid_search.rs     # [NEW] Combined search
├── zero-gateway/
│   └── src/
│       └── context.rs           # [NEW] Context API
└── zero-common/
    └── src/
        └── context_types.rs     # [NEW] Shared types

packages/
└── ccode/
    └── src/
        └── agent/
            └── autonomous-loop.ts # [NEW] 4-step loop
```

---

## 6. Dependencies

### 6.1 New Rust Crates

```toml
# services/zero-cli/Cargo.toml
[dependencies]
bollard = "0.16"

# services/zero-memory/Cargo.toml
[dependencies]
qdrant-client = "1.8"
```

### 6.2 Infrastructure Requirements

- **Docker:** Required for sandbox execution
- **Qdrant:** Vector database (can run in Docker or cloud)
  - Suggested: `docker run -p 6333:6333 qdrant/qdrant`

---

## 7. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Docker not available on host | High | Implement subprocess fallback with security warnings |
| Qdrant connection failures | Medium | SQLite-only fallback for search |
| Embedding API rate limits | Medium | Local embedding model option (e.g., sentence-transformers) |
| Container escape vulnerabilities | High | Use gVisor/Kata containers for production |

---

## 8. Success Criteria

### Phase 1
- [ ] Docker sandbox executes Python/JS code successfully
- [ ] Self-reflection loop retries up to 10 times
- [ ] Crystallized knowledge retrievable in future sessions
- [ ] 80% test coverage

### Phase 2
- [ ] Qdrant stores and retrieves embeddings
- [ ] Hybrid search returns relevant results
- [ ] Context API responds within 100ms for typical queries
- [ ] RBAC filtering works correctly
- [ ] 80% test coverage

---

## Appendix A: Configuration Schema

```json
{
  "sandbox": {
    "enabled": true,
    "docker_socket": "/var/run/docker.sock",
    "default_timeout_secs": 60,
    "max_memory_mb": 256,
    "network_enabled": false
  },
  "qdrant": {
    "enabled": true,
    "url": "http://localhost:6333",
    "collection": "codecoder_memory",
    "embedding_model": "openai"
  }
}
```
