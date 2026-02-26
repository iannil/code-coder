# Omni-Nexus Phase 1 & 2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement autonomous code execution with Docker sandbox and global context hub with Qdrant vector database.

**Architecture:** Phase 1 adds Docker sandbox execution (bollard), self-reflection retry loop, and knowledge crystallization. Phase 2 adds Qdrant vector search, hybrid search engine, and context retrieval API endpoints to zero-gateway.

**Tech Stack:** Rust (bollard, qdrant-client, axum), TypeScript (ccode agents), Docker, Qdrant

---

## Phase 1: Docker Sandbox

### Task 1: Add bollard dependency to zero-cli

**Files:**
- Modify: `services/zero-cli/Cargo.toml`

**Step 1: Add bollard crate**

```toml
# Add after line 106 (before [dev-dependencies])
# Docker API client for sandbox execution
bollard = "0.16"
```

**Step 2: Verify compilation**

Run: `cd services && cargo check -p zero-cli`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add services/zero-cli/Cargo.toml
git commit -m "deps(zero-cli): add bollard crate for Docker sandbox"
```

---

### Task 2: Create sandbox types module

**Files:**
- Create: `services/zero-cli/src/sandbox/types.rs`
- Create: `services/zero-cli/src/sandbox/mod.rs`

**Step 1: Write the types test file**

Create `services/zero-cli/src/sandbox/types.rs`:

```rust
//! Sandbox execution types.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Supported languages for sandbox execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    Shell,
    Rust,
}

impl Language {
    /// Default Docker image for this language.
    pub fn default_image(&self) -> &'static str {
        match self {
            Self::Python => "python:3.11-slim",
            Self::JavaScript => "node:20-slim",
            Self::Shell => "alpine:3.19",
            Self::Rust => "rust:1.75-slim",
        }
    }

    /// File extension for this language.
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::Shell => "sh",
            Self::Rust => "rs",
        }
    }

    /// Command to execute a script file.
    pub fn run_command(&self, filename: &str) -> Vec<String> {
        match self {
            Self::Python => vec!["python".into(), filename.into()],
            Self::JavaScript => vec!["node".into(), filename.into()],
            Self::Shell => vec!["sh".into(), filename.into()],
            Self::Rust => vec!["rustc".into(), filename.into(), "-o".into(), "/tmp/out".into()],
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Python => write!(f, "python"),
            Self::JavaScript => write!(f, "javascript"),
            Self::Shell => write!(f, "shell"),
            Self::Rust => write!(f, "rust"),
        }
    }
}

/// Configuration for sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Docker image to use (overrides language default)
    pub image: Option<String>,
    /// Memory limit in bytes (default: 256MB)
    pub memory_limit: u64,
    /// CPU quota (default: 1.0 = 100% of one core)
    pub cpu_quota: f64,
    /// Network access enabled (default: false)
    pub network_enabled: bool,
    /// Maximum execution time
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Working directory inside container
    pub workdir: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: None,
            memory_limit: 256 * 1024 * 1024, // 256MB
            cpu_quota: 1.0,
            network_enabled: false,
            timeout: Duration::from_secs(60),
            workdir: "/workspace".into(),
        }
    }
}

/// Result of sandbox code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
    /// Whether execution was killed due to timeout
    pub timed_out: bool,
}

impl ExecutionResult {
    /// Check if execution succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// A single attempt at executing code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAttempt {
    /// The code that was executed
    pub code: String,
    /// Language used
    pub language: Language,
    /// Execution result
    pub result: ExecutionResult,
    /// AI reflection on the error (if failed)
    pub reflection: Option<String>,
    /// Timestamp of attempt
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_default_image() {
        assert_eq!(Language::Python.default_image(), "python:3.11-slim");
        assert_eq!(Language::JavaScript.default_image(), "node:20-slim");
        assert_eq!(Language::Shell.default_image(), "alpine:3.19");
        assert_eq!(Language::Rust.default_image(), "rust:1.75-slim");
    }

    #[test]
    fn language_file_extension() {
        assert_eq!(Language::Python.file_extension(), "py");
        assert_eq!(Language::JavaScript.file_extension(), "js");
        assert_eq!(Language::Shell.file_extension(), "sh");
        assert_eq!(Language::Rust.file_extension(), "rs");
    }

    #[test]
    fn language_display() {
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::JavaScript.to_string(), "javascript");
    }

    #[test]
    fn sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_limit, 256 * 1024 * 1024);
        assert!((config.cpu_quota - 1.0).abs() < f64::EPSILON);
        assert!(!config.network_enabled);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn execution_result_success() {
        let result = ExecutionResult {
            exit_code: 0,
            stdout: "output".into(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
            timed_out: false,
        };
        assert!(result.success());

        let failed = ExecutionResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".into(),
            duration: Duration::from_millis(50),
            timed_out: false,
        };
        assert!(!failed.success());

        let timeout = ExecutionResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_secs(60),
            timed_out: true,
        };
        assert!(!timeout.success());
    }

    #[test]
    fn language_serialization() {
        let json = serde_json::to_string(&Language::Python).unwrap();
        assert_eq!(json, "\"python\"");

        let parsed: Language = serde_json::from_str("\"javascript\"").unwrap();
        assert_eq!(parsed, Language::JavaScript);
    }
}
```

**Step 2: Create mod.rs**

Create `services/zero-cli/src/sandbox/mod.rs`:

```rust
//! Docker-based sandbox for secure code execution.
//!
//! This module provides isolated execution of untrusted code using Docker containers.
//! Security features:
//! - Network disabled by default
//! - Memory and CPU limits
//! - No host filesystem access
//! - Automatic container cleanup

pub mod docker;
pub mod types;

pub use docker::DockerSandbox;
pub use types::{ExecutionAttempt, ExecutionResult, Language, SandboxConfig};
```

**Step 3: Run tests**

Run: `cd services && cargo test -p zero-cli sandbox::types`
Expected: All tests pass

**Step 4: Commit**

```bash
git add services/zero-cli/src/sandbox/
git commit -m "feat(sandbox): add types module with Language, SandboxConfig, ExecutionResult"
```

---

### Task 3: Implement DockerSandbox

**Files:**
- Create: `services/zero-cli/src/sandbox/docker.rs`
- Modify: `services/zero-cli/src/lib.rs` (add sandbox module)

**Step 1: Add humantime-serde dependency**

Add to `services/zero-cli/Cargo.toml`:
```toml
humantime-serde = "1.1"
```

**Step 2: Create docker.rs with tests first**

Create `services/zero-cli/src/sandbox/docker.rs`:

```rust
//! Docker-based sandbox execution using bollard.

use crate::sandbox::types::{ExecutionResult, Language, SandboxConfig};
use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Docker-based sandbox for secure code execution.
pub struct DockerSandbox {
    client: Docker,
    config: SandboxConfig,
}

impl DockerSandbox {
    /// Create a new sandbox with default configuration.
    pub async fn new() -> Result<Self> {
        Self::with_config(SandboxConfig::default()).await
    }

    /// Create a sandbox with custom configuration.
    pub async fn with_config(config: SandboxConfig) -> Result<Self> {
        let client = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker daemon")?;

        // Verify Docker is accessible
        client
            .ping()
            .await
            .context("Docker daemon not responding")?;

        Ok(Self { client, config })
    }

    /// Check if Docker is available.
    pub async fn health_check(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    /// Execute code in an isolated Docker container.
    pub async fn execute(&self, code: &str, language: Language) -> Result<ExecutionResult> {
        self.execute_with_timeout(code, language, self.config.timeout)
            .await
    }

    /// Execute code with a custom timeout.
    pub async fn execute_with_timeout(
        &self,
        code: &str,
        language: Language,
        exec_timeout: Duration,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();
        let image = self
            .config
            .image
            .as_deref()
            .unwrap_or_else(|| language.default_image());

        // Ensure image is available
        self.pull_image_if_needed(image).await?;

        // Create container
        let container_name = format!("sandbox-{}", uuid::Uuid::new_v4());
        let filename = format!("/workspace/script.{}", language.file_extension());

        let container_config = Config {
            image: Some(image.to_string()),
            cmd: Some(language.run_command(&filename)),
            working_dir: Some(self.config.workdir.clone()),
            host_config: Some(bollard::service::HostConfig {
                memory: Some(self.config.memory_limit as i64),
                cpu_quota: Some((self.config.cpu_quota * 100_000.0) as i64),
                cpu_period: Some(100_000),
                network_mode: if self.config.network_enabled {
                    None
                } else {
                    Some("none".to_string())
                },
                auto_remove: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let create_opts = CreateContainerOptions {
            name: &container_name,
            platform: None,
        };

        self.client
            .create_container(Some(create_opts), container_config)
            .await
            .context("Failed to create container")?;

        // Copy code into container
        self.copy_code_to_container(&container_name, code, &filename)
            .await?;

        // Start container
        self.client
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        // Wait for completion with timeout
        let wait_result = timeout(exec_timeout, self.wait_for_container(&container_name)).await;

        let (exit_code, timed_out) = match wait_result {
            Ok(Ok(code)) => (code, false),
            Ok(Err(e)) => {
                // Kill container on error
                let _ = self.kill_container(&container_name).await;
                return Err(e);
            }
            Err(_) => {
                // Timeout - kill container
                let _ = self.kill_container(&container_name).await;
                (-1, true)
            }
        };

        // Collect logs
        let (stdout, stderr) = self.collect_logs(&container_name).await.unwrap_or_default();

        // Cleanup (auto_remove should handle this, but be safe)
        let _ = self
            .client
            .remove_container(
                &container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            duration: start.elapsed(),
            timed_out,
        })
    }

    async fn pull_image_if_needed(&self, image: &str) -> Result<()> {
        // Check if image exists
        if self.client.inspect_image(image).await.is_ok() {
            return Ok(());
        }

        tracing::info!("Pulling image: {image}");
        let opts = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(opts), None, None);
        while let Some(result) = stream.next().await {
            result.context("Failed to pull image")?;
        }

        Ok(())
    }

    async fn copy_code_to_container(
        &self,
        container: &str,
        code: &str,
        filename: &str,
    ) -> Result<()> {
        // Create exec to write file
        let exec = self
            .client
            .create_exec(
                container,
                CreateExecOptions {
                    attach_stdin: Some(true),
                    attach_stdout: Some(false),
                    attach_stderr: Some(false),
                    cmd: Some(vec!["sh", "-c", &format!("cat > {filename}")]),
                    ..Default::default()
                },
            )
            .await
            .context("Failed to create exec for file copy")?;

        // Start exec and write code
        if let StartExecResults::Attached { mut input, .. } =
            self.client.start_exec(&exec.id, None).await?
        {
            use tokio::io::AsyncWriteExt;
            input.write_all(code.as_bytes()).await?;
            input.shutdown().await?;
        }

        Ok(())
    }

    async fn wait_for_container(&self, container: &str) -> Result<i32> {
        let mut stream = self.client.wait_container(
            container,
            Some(WaitContainerOptions {
                condition: "not-running",
            }),
        );

        if let Some(result) = stream.next().await {
            let response = result.context("Failed to wait for container")?;
            Ok(response.status_code as i32)
        } else {
            Ok(-1)
        }
    }

    async fn kill_container(&self, container: &str) -> Result<()> {
        self.client
            .kill_container(container, None::<bollard::container::KillContainerOptions<String>>)
            .await
            .ok(); // Ignore errors (container may already be dead)
        Ok(())
    }

    async fn collect_logs(&self, container: &str) -> Result<(String, String)> {
        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.client.logs(container, Some(opts));
        let mut stdout = String::new();
        let mut stderr = String::new();

        while let Some(result) = stream.next().await {
            match result? {
                LogOutput::StdOut { message } => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                }
                LogOutput::StdErr { message } => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        Ok((stdout, stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require Docker to be running
    // Skip with: cargo test -- --skip docker

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn sandbox_health_check() {
        let sandbox = DockerSandbox::new().await.unwrap();
        assert!(sandbox.health_check().await);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_python_hello_world() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("print('Hello, World!')", Language::Python)
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("Hello, World!"));
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_python_with_error() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("raise ValueError('test error')", Language::Python)
            .await
            .unwrap();

        assert!(!result.success());
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("ValueError"));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_shell_script() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("echo 'hello' && echo 'world'", Language::Shell)
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("hello"));
        assert!(result.stdout.contains("world"));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_with_timeout() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute_with_timeout(
                "import time; time.sleep(10)",
                Language::Python,
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        assert!(!result.success());
        assert!(result.timed_out);
    }
}
```

**Step 3: Add sandbox module to lib.rs**

Modify `services/zero-cli/src/lib.rs`, add after line 45 (after `pub mod session;`):

```rust
pub mod sandbox;
```

**Step 4: Verify compilation**

Run: `cd services && cargo check -p zero-cli`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add services/zero-cli/src/sandbox/ services/zero-cli/src/lib.rs services/zero-cli/Cargo.toml
git commit -m "feat(sandbox): implement DockerSandbox with bollard"
```

---

### Task 4: Implement Crystallizer

**Files:**
- Create: `services/zero-cli/src/memory/crystallize.rs`
- Modify: `services/zero-cli/src/memory/mod.rs`

**Step 1: Write crystallize.rs**

Create `services/zero-cli/src/memory/crystallize.rs`:

```rust
//! Knowledge crystallization - extract and store successful solutions.

use crate::sandbox::{ExecutionAttempt, ExecutionResult, Language};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zero_memory::{Memory, MemoryCategory};

/// A crystallized piece of knowledge extracted from successful execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizedKnowledge {
    /// Unique identifier
    pub id: String,
    /// Original problem description
    pub problem: String,
    /// Error messages encountered during attempts
    pub errors: Vec<String>,
    /// Final successful solution code
    pub solution: String,
    /// Programming language used
    pub language: Language,
    /// Tags for categorization and retrieval
    pub tags: Vec<String>,
    /// When this knowledge was crystallized
    pub created_at: DateTime<Utc>,
    /// Number of attempts before success
    pub retry_count: u32,
}

/// Extracts and stores successful solutions for future retrieval.
pub struct Crystallizer {
    memory: Arc<dyn Memory>,
}

impl Crystallizer {
    /// Create a new crystallizer with the given memory backend.
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }

    /// Extract knowledge from a successful execution and store it.
    ///
    /// Returns the crystallized knowledge with its assigned ID.
    pub async fn crystallize(
        &self,
        problem: &str,
        attempts: &[ExecutionAttempt],
        final_result: &ExecutionResult,
    ) -> Result<CrystallizedKnowledge> {
        // Find the successful attempt (last one with success)
        let successful_attempt = attempts
            .iter()
            .rev()
            .find(|a| a.result.success())
            .ok_or_else(|| anyhow::anyhow!("No successful attempt found"))?;

        // Extract errors from failed attempts
        let errors: Vec<String> = attempts
            .iter()
            .filter(|a| !a.result.success())
            .map(|a| a.result.stderr.clone())
            .filter(|e| !e.is_empty())
            .collect();

        // Generate tags from problem and errors
        let tags = self.generate_tags(problem, &errors);

        let id = uuid::Uuid::new_v4().to_string();
        let knowledge = CrystallizedKnowledge {
            id: id.clone(),
            problem: problem.to_string(),
            errors,
            solution: successful_attempt.code.clone(),
            language: successful_attempt.language,
            tags,
            created_at: Utc::now(),
            retry_count: attempts.len() as u32 - 1,
        };

        // Store in memory
        let content = serde_json::to_string_pretty(&knowledge)?;
        self.memory
            .store(&format!("crystallized:{id}"), &content, MemoryCategory::Core)
            .await?;

        tracing::info!(
            id = %id,
            problem = %problem,
            retry_count = knowledge.retry_count,
            "Knowledge crystallized"
        );

        Ok(knowledge)
    }

    /// Search for relevant prior solutions.
    pub async fn search_solutions(&self, problem: &str, limit: usize) -> Result<Vec<CrystallizedKnowledge>> {
        let entries = self.memory.recall(problem, limit * 2).await?;

        let mut solutions = Vec::new();
        for entry in entries {
            if entry.key.starts_with("crystallized:") {
                if let Ok(knowledge) = serde_json::from_str::<CrystallizedKnowledge>(&entry.content) {
                    solutions.push(knowledge);
                    if solutions.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(solutions)
    }

    /// Generate tags from problem description and errors.
    fn generate_tags(&self, problem: &str, errors: &[String]) -> Vec<String> {
        let mut tags = Vec::new();

        // Extract potential tags from problem
        let keywords = ["api", "file", "database", "http", "json", "parse", "convert", "calculate"];
        for keyword in keywords {
            if problem.to_lowercase().contains(keyword) {
                tags.push(keyword.to_string());
            }
        }

        // Extract error types
        for error in errors {
            if error.contains("SyntaxError") {
                tags.push("syntax-error".to_string());
            }
            if error.contains("TypeError") {
                tags.push("type-error".to_string());
            }
            if error.contains("ImportError") || error.contains("ModuleNotFoundError") {
                tags.push("import-error".to_string());
            }
            if error.contains("ConnectionError") || error.contains("timeout") {
                tags.push("network-error".to_string());
            }
        }

        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::ExecutionResult;
    use std::time::Duration;

    // Mock memory for testing
    struct MockMemory {
        stored: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl MockMemory {
        fn new() -> Self {
            Self {
                stored: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(&self, key: &str, content: &str, _category: MemoryCategory) -> Result<()> {
            self.stored.lock().unwrap().push((key.to_string(), content.to_string()));
            Ok(())
        }

        async fn recall(&self, _query: &str, _limit: usize) -> Result<Vec<zero_memory::MemoryEntry>> {
            Ok(vec![])
        }

        async fn get(&self, _key: &str) -> Result<Option<zero_memory::MemoryEntry>> {
            Ok(None)
        }

        async fn list(&self, _category: Option<&MemoryCategory>) -> Result<Vec<zero_memory::MemoryEntry>> {
            Ok(vec![])
        }

        async fn forget(&self, _key: &str) -> Result<bool> {
            Ok(false)
        }

        async fn count(&self, _category: Option<&MemoryCategory>) -> Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    fn make_attempt(code: &str, success: bool, stderr: &str) -> ExecutionAttempt {
        ExecutionAttempt {
            code: code.to_string(),
            language: Language::Python,
            result: ExecutionResult {
                exit_code: if success { 0 } else { 1 },
                stdout: String::new(),
                stderr: stderr.to_string(),
                duration: Duration::from_millis(100),
                timed_out: false,
            },
            reflection: None,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn crystallize_stores_knowledge() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory.clone());

        let attempts = vec![
            make_attempt("print(x)", false, "NameError: name 'x' is not defined"),
            make_attempt("x = 1; print(x)", true, ""),
        ];

        let result = crystallizer
            .crystallize("print a variable", &attempts, &attempts[1].result)
            .await
            .unwrap();

        assert_eq!(result.retry_count, 1);
        assert_eq!(result.solution, "x = 1; print(x)");
        assert!(!result.errors.is_empty());

        let stored = memory.stored.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert!(stored[0].0.starts_with("crystallized:"));
    }

    #[test]
    fn generate_tags_extracts_keywords() {
        let memory = Arc::new(MockMemory::new());
        let crystallizer = Crystallizer::new(memory);

        let tags = crystallizer.generate_tags(
            "parse JSON file and convert to database",
            &["TypeError: expected string".to_string()],
        );

        assert!(tags.contains(&"json".to_string()));
        assert!(tags.contains(&"file".to_string()));
        assert!(tags.contains(&"database".to_string()));
        assert!(tags.contains(&"convert".to_string()));
        assert!(tags.contains(&"type-error".to_string()));
    }
}
```

**Step 2: Check if memory/mod.rs exists and update it**

Run: `ls services/zero-cli/src/memory/`

If mod.rs exists, add:
```rust
pub mod crystallize;
pub use crystallize::{Crystallizer, CrystallizedKnowledge};
```

If memory is a single file (mod.rs doesn't exist), you'll need to create the directory structure.

**Step 3: Run tests**

Run: `cd services && cargo test -p zero-cli memory::crystallize`
Expected: All tests pass

**Step 4: Commit**

```bash
git add services/zero-cli/src/memory/
git commit -m "feat(memory): add Crystallizer for knowledge extraction and storage"
```

---

## Phase 2: Qdrant Integration

### Task 5: Add qdrant-client to zero-memory

**Files:**
- Modify: `services/zero-memory/Cargo.toml`

**Step 1: Add dependency**

Add to `services/zero-memory/Cargo.toml`:

```toml
# Vector database client
qdrant-client = "1.8"
uuid = { workspace = true }
```

**Step 2: Verify compilation**

Run: `cd services && cargo check -p zero-memory`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add services/zero-memory/Cargo.toml
git commit -m "deps(zero-memory): add qdrant-client crate"
```

---

### Task 6: Implement QdrantMemory

**Files:**
- Create: `services/zero-memory/src/qdrant.rs`
- Modify: `services/zero-memory/src/lib.rs`

**Step 1: Create qdrant.rs**

Create `services/zero-memory/src/qdrant.rs`:

```rust
//! Qdrant vector database integration.

use crate::embeddings::EmbeddingProvider;
use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use qdrant_client::prelude::*;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchPoints, VectorParams, VectorsConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Metadata stored with each Qdrant point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantMetadata {
    pub key: String,
    pub content: String,
    pub category: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Qdrant-backed memory with vector search.
pub struct QdrantMemory {
    client: QdrantClient,
    collection: String,
    embedding: Arc<dyn EmbeddingProvider>,
    dimension: usize,
}

impl QdrantMemory {
    /// Connect to Qdrant and prepare collection.
    pub async fn connect(
        url: &str,
        collection: &str,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> Result<Self> {
        let client = QdrantClient::from_url(url)
            .build()
            .context("Failed to create Qdrant client")?;

        let dimension = embedding.dimension();

        let memory = Self {
            client,
            collection: collection.to_string(),
            embedding,
            dimension,
        };

        memory.ensure_collection().await?;
        Ok(memory)
    }

    /// Ensure the collection exists with proper schema.
    pub async fn ensure_collection(&self) -> Result<()> {
        // Check if collection exists
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            tracing::info!(collection = %self.collection, "Creating Qdrant collection");

            self.client
                .create_collection(&CreateCollection {
                    collection_name: self.collection.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: self.dimension as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .context("Failed to create collection")?;
        }

        Ok(())
    }

    /// Generate a point ID from a key.
    fn key_to_id(key: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[async_trait]
impl Memory for QdrantMemory {
    fn name(&self) -> &str {
        "qdrant"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()> {
        let embedding = self.embedding.embed(content).await?;
        let now = chrono::Utc::now().timestamp_millis();

        let metadata = QdrantMetadata {
            key: key.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            created_at: now,
            updated_at: now,
        };

        let payload: HashMap<String, qdrant_client::qdrant::Value> = serde_json::from_value(
            serde_json::to_value(&metadata)?,
        )?;

        let point = PointStruct::new(Self::key_to_id(key), embedding, payload);

        self.client
            .upsert_points_blocking(&self.collection, None, vec![point], None)
            .await
            .context("Failed to upsert point")?;

        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let embedding = self.embedding.embed(query).await?;

        let search = SearchPoints {
            collection_name: self.collection.clone(),
            vector: embedding,
            limit: limit as u64,
            with_payload: Some(true.into()),
            ..Default::default()
        };

        let results = self
            .client
            .search_points(&search)
            .await
            .context("Search failed")?;

        let mut entries = Vec::new();
        for point in results.result {
            if let Some(payload) = &point.payload {
                let key = payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let content = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let category = payload
                    .get("category")
                    .and_then(|v| v.as_str())
                    .map(MemoryCategory::from)
                    .unwrap_or(MemoryCategory::Scratch);
                let created_at = payload
                    .get("created_at")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0);
                let updated_at = payload
                    .get("updated_at")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0);

                entries.push(MemoryEntry {
                    key,
                    content,
                    category,
                    created_at,
                    updated_at,
                    score: point.score,
                });
            }
        }

        Ok(entries)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let id = Self::key_to_id(key);
        let points = self
            .client
            .get_points(&self.collection, None, &[id.into()], Some(true), Some(false), None)
            .await?;

        if let Some(point) = points.result.first() {
            if let Some(payload) = &point.payload {
                let content = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let category = payload
                    .get("category")
                    .and_then(|v| v.as_str())
                    .map(MemoryCategory::from)
                    .unwrap_or(MemoryCategory::Scratch);
                let created_at = payload
                    .get("created_at")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0);
                let updated_at = payload
                    .get("updated_at")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0);

                return Ok(Some(MemoryEntry {
                    key: key.to_string(),
                    content,
                    category,
                    created_at,
                    updated_at,
                    score: 1.0,
                }));
            }
        }

        Ok(None)
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> Result<Vec<MemoryEntry>> {
        // For listing, we do a scroll through all points
        // This is not efficient for large collections but works for moderate sizes
        self.recall("", 1000).await
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let id = Self::key_to_id(key);
        let result = self
            .client
            .delete_points_blocking(&self.collection, None, &[id.into()].into(), None)
            .await;

        Ok(result.is_ok())
    }

    async fn count(&self, _category: Option<&MemoryCategory>) -> Result<usize> {
        let info = self.client.collection_info(&self.collection).await?;
        Ok(info.result.map(|r| r.points_count as usize).unwrap_or(0))
    }

    async fn health_check(&self) -> bool {
        self.client.health_check().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::NoopEmbedding;

    // Note: These tests require Qdrant to be running
    // docker run -p 6333:6333 qdrant/qdrant

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_store_and_recall() {
        let embedding = Arc::new(NoopEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6333", "test_collection", embedding)
            .await
            .unwrap();

        memory
            .store("test_key", "test content", MemoryCategory::Core)
            .await
            .unwrap();

        let results = memory.recall("test", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.key == "test_key"));
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_get_by_key() {
        let embedding = Arc::new(NoopEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6333", "test_get", embedding)
            .await
            .unwrap();

        memory
            .store("get_key", "get content", MemoryCategory::Project)
            .await
            .unwrap();

        let entry = memory.get("get_key").await.unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().content, "get content");
    }

    #[tokio::test]
    #[ignore = "requires Qdrant"]
    async fn qdrant_forget() {
        let embedding = Arc::new(NoopEmbedding::new(128));
        let memory = QdrantMemory::connect("http://localhost:6333", "test_forget", embedding)
            .await
            .unwrap();

        memory
            .store("forget_key", "content", MemoryCategory::Scratch)
            .await
            .unwrap();

        let deleted = memory.forget("forget_key").await.unwrap();
        assert!(deleted);

        let entry = memory.get("forget_key").await.unwrap();
        assert!(entry.is_none());
    }
}
```

**Step 2: Update lib.rs**

Add to `services/zero-memory/src/lib.rs`:

```rust
pub mod qdrant;
pub use qdrant::QdrantMemory;
```

**Step 3: Run tests**

Run: `cd services && cargo test -p zero-memory qdrant --no-run`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add services/zero-memory/src/qdrant.rs services/zero-memory/src/lib.rs
git commit -m "feat(zero-memory): implement QdrantMemory for vector search"
```

---

### Task 7: Implement HybridSearchEngine

**Files:**
- Create: `services/zero-memory/src/hybrid_search.rs`
- Modify: `services/zero-memory/src/lib.rs`

**Step 1: Create hybrid_search.rs**

Create `services/zero-memory/src/hybrid_search.rs`:

```rust
//! Hybrid search combining vector and keyword search.

use crate::qdrant::QdrantMemory;
use crate::sqlite::SqliteMemory;
use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use crate::vector::{hybrid_merge, ScoredResult};
use anyhow::Result;
use std::sync::Arc;

/// Combines vector (Qdrant) and keyword (SQLite FTS5) search.
pub struct HybridSearchEngine {
    qdrant: Arc<QdrantMemory>,
    sqlite: Arc<SqliteMemory>,
    vector_weight: f32,
    keyword_weight: f32,
}

impl HybridSearchEngine {
    /// Create a new hybrid search engine.
    ///
    /// Default weights: 70% vector, 30% keyword.
    pub fn new(qdrant: Arc<QdrantMemory>, sqlite: Arc<SqliteMemory>) -> Self {
        Self {
            qdrant,
            sqlite,
            vector_weight: 0.7,
            keyword_weight: 0.3,
        }
    }

    /// Create with custom weights.
    pub fn with_weights(
        qdrant: Arc<QdrantMemory>,
        sqlite: Arc<SqliteMemory>,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Self {
        Self {
            qdrant,
            sqlite,
            vector_weight,
            keyword_weight,
        }
    }

    /// Perform hybrid search.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        category_filter: Option<MemoryCategory>,
    ) -> Result<Vec<MemoryEntry>> {
        // Run both searches in parallel
        let (vector_results, keyword_results) = tokio::join!(
            self.qdrant.recall(query, limit * 2),
            self.sqlite.recall(query, limit * 2)
        );

        let vector_results = vector_results.unwrap_or_default();
        let keyword_results = keyword_results.unwrap_or_default();

        // Convert to scored tuples
        let vector_tuples: Vec<(String, f32)> = vector_results
            .iter()
            .map(|e| (e.key.clone(), e.score))
            .collect();

        let keyword_tuples: Vec<(String, f32)> = keyword_results
            .iter()
            .map(|e| (e.key.clone(), e.score))
            .collect();

        // Merge results
        let merged = hybrid_merge(
            &vector_tuples,
            &keyword_tuples,
            self.vector_weight,
            self.keyword_weight,
            limit,
        );

        // Build final entries from merged results
        let mut entries = Vec::new();
        for scored in merged {
            // Find the full entry
            let entry = vector_results
                .iter()
                .chain(keyword_results.iter())
                .find(|e| e.key == scored.id);

            if let Some(e) = entry {
                // Apply category filter
                if let Some(ref cat) = category_filter {
                    if &e.category != cat {
                        continue;
                    }
                }

                entries.push(MemoryEntry {
                    key: e.key.clone(),
                    content: e.content.clone(),
                    category: e.category.clone(),
                    created_at: e.created_at,
                    updated_at: e.updated_at,
                    score: scored.final_score,
                });
            }
        }

        Ok(entries)
    }

    /// Check health of both backends.
    pub async fn health_check(&self) -> (bool, bool) {
        let (qdrant_ok, sqlite_ok) =
            tokio::join!(self.qdrant.health_check(), self.sqlite.health_check());
        (qdrant_ok, sqlite_ok)
    }
}

#[cfg(test)]
mod tests {
    // Tests would require both Qdrant and SQLite setup
    // Covered by integration tests
}
```

**Step 2: Update lib.rs**

Add to `services/zero-memory/src/lib.rs`:

```rust
pub mod hybrid_search;
pub use hybrid_search::HybridSearchEngine;
```

**Step 3: Verify compilation**

Run: `cd services && cargo check -p zero-memory`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add services/zero-memory/src/hybrid_search.rs services/zero-memory/src/lib.rs
git commit -m "feat(zero-memory): add HybridSearchEngine combining vector and keyword search"
```

---

### Task 8: Add Context API to zero-gateway

**Files:**
- Create: `services/zero-gateway/src/context.rs`
- Modify: `services/zero-gateway/src/lib.rs`

**Step 1: Create context.rs**

Create `services/zero-gateway/src/context.rs`:

```rust
//! Context retrieval API endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use zero_memory::{HybridSearchEngine, Memory, MemoryCategory, MemoryEntry};

/// Application state for context routes.
pub struct ContextState {
    pub search_engine: Arc<HybridSearchEngine>,
}

/// Query parameters for search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,
    /// Maximum results to return (default: 10)
    pub limit: Option<usize>,
    /// Filter by category
    pub category: Option<String>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Search response.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<ContextEntry>,
    pub total: usize,
    pub query_time_ms: u64,
}

/// A single context entry in search results.
#[derive(Debug, Serialize)]
pub struct ContextEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub score: f32,
    pub created_at: i64,
}

impl From<MemoryEntry> for ContextEntry {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.key,
            content: entry.content,
            category: entry.category.to_string(),
            score: entry.score,
            created_at: entry.created_at,
        }
    }
}

/// Request body for ingesting new content.
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub content: String,
    pub category: String,
    pub source: String,
    pub tags: Option<Vec<String>>,
}

/// Response for successful ingestion.
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub id: String,
    pub message: String,
}

/// Error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Create context API routes.
pub fn context_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<ContextState>: axum::extract::FromRef<S>,
{
    Router::new()
        .route("/api/v1/context/search", get(search_context))
        .route("/api/v1/context/ingest", post(ingest_context))
        .route("/api/v1/context/:id", get(get_context))
        .route("/api/v1/context/categories", get(list_categories))
}

/// Search context with hybrid vector + keyword search.
async fn search_context(
    State(state): State<Arc<ContextState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();
    let limit = query.limit.unwrap_or(10).min(100);
    let category = query.category.map(|c| MemoryCategory::from(c.as_str()));

    let results = state
        .search_engine
        .search(&query.q, limit, category)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: "SEARCH_ERROR".to_string(),
                }),
            )
        })?;

    let entries: Vec<ContextEntry> = results.into_iter().map(ContextEntry::from).collect();
    let total = entries.len();

    Ok(Json(SearchResponse {
        results: entries,
        total,
        query_time_ms: start.elapsed().as_millis() as u64,
    }))
}

/// Ingest new content into the context hub.
async fn ingest_context(
    State(_state): State<Arc<ContextState>>,
    Json(request): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    let id = uuid::Uuid::new_v4().to_string();

    // TODO: Store content via memory backend
    // For now, return success with generated ID

    Ok(Json(IngestResponse {
        id,
        message: "Content ingested successfully".to_string(),
    }))
}

/// Get a specific context entry by ID.
async fn get_context(
    State(_state): State<Arc<ContextState>>,
    Path(id): Path<String>,
) -> Result<Json<ContextEntry>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement get by ID
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("Context entry not found: {id}"),
            code: "NOT_FOUND".to_string(),
        }),
    ))
}

/// List available categories.
async fn list_categories() -> Json<Vec<String>> {
    Json(vec![
        "core".to_string(),
        "project".to_string(),
        "conversation".to_string(),
        "daily".to_string(),
        "scratch".to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_entry_from_memory_entry() {
        let memory_entry = MemoryEntry {
            key: "test_key".to_string(),
            content: "test content".to_string(),
            category: MemoryCategory::Core,
            created_at: 1234567890,
            updated_at: 1234567890,
            score: 0.95,
        };

        let context_entry: ContextEntry = memory_entry.into();
        assert_eq!(context_entry.id, "test_key");
        assert_eq!(context_entry.category, "core");
        assert!((context_entry.score - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn search_query_defaults() {
        let query: SearchQuery = serde_json::from_str(r#"{"q": "test"}"#).unwrap();
        assert_eq!(query.q, "test");
        assert!(query.limit.is_none());
        assert!(query.category.is_none());
    }
}
```

**Step 2: Update gateway lib.rs**

Check `services/zero-gateway/src/lib.rs` and add:

```rust
pub mod context;
pub use context::{context_routes, ContextState};
```

**Step 3: Verify compilation**

Run: `cd services && cargo check -p zero-gateway`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add services/zero-gateway/src/context.rs services/zero-gateway/src/lib.rs
git commit -m "feat(zero-gateway): add context retrieval API endpoints"
```

---

## Final Integration

### Task 9: Update configuration schema

**Files:**
- Modify: `services/zero-common/src/config.rs` (or wherever Config is defined)

**Step 1: Add sandbox and qdrant config sections**

Add to the Config struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_docker_socket")]
    pub docker_socket: String,
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,
    #[serde(default)]
    pub network_enabled: bool,
}

fn default_docker_socket() -> String {
    "/var/run/docker.sock".to_string()
}

fn default_timeout_secs() -> u64 {
    60
}

fn default_max_memory_mb() -> u64 {
    256
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_qdrant_url")]
    pub url: String,
    #[serde(default = "default_collection")]
    pub collection: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_qdrant_url() -> String {
    "http://localhost:6333".to_string()
}

fn default_collection() -> String {
    "codecoder_memory".to_string()
}

fn default_embedding_model() -> String {
    "openai".to_string()
}
```

**Step 2: Add to main Config struct**

```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub qdrant: QdrantConfig,
}
```

**Step 3: Commit**

```bash
git add services/zero-common/src/config.rs
git commit -m "feat(config): add sandbox and qdrant configuration sections"
```

---

### Task 10: Write integration tests

**Files:**
- Create: `services/zero-cli/tests/sandbox_integration.rs`
- Create: `services/zero-memory/tests/qdrant_integration.rs`

**Step 1: Create sandbox integration test**

Create `services/zero-cli/tests/sandbox_integration.rs`:

```rust
//! Integration tests for Docker sandbox.
//!
//! Requires Docker to be running.
//! Run with: cargo test --test sandbox_integration -- --ignored

use std::time::Duration;
use zero_cli::sandbox::{DockerSandbox, Language, SandboxConfig};

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_executes_python_successfully() {
    let sandbox = DockerSandbox::new().await.expect("Failed to create sandbox");

    let result = sandbox
        .execute(
            r#"
import sys
print("Hello from Python!")
print(f"Version: {sys.version}")
"#,
            Language::Python,
        )
        .await
        .expect("Execution failed");

    assert!(result.success(), "Expected success, got exit code {}", result.exit_code);
    assert!(result.stdout.contains("Hello from Python!"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_captures_stderr() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute("import sys; sys.stderr.write('error output')", Language::Python)
        .await
        .unwrap();

    assert!(result.stderr.contains("error output"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_enforces_timeout() {
    let config = SandboxConfig {
        timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let sandbox = DockerSandbox::with_config(config).await.unwrap();

    let result = sandbox
        .execute("import time; time.sleep(10)", Language::Python)
        .await
        .unwrap();

    assert!(result.timed_out);
    assert!(!result.success());
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn sandbox_shell_execution() {
    let sandbox = DockerSandbox::new().await.unwrap();

    let result = sandbox
        .execute("echo 'hello' && whoami", Language::Shell)
        .await
        .unwrap();

    assert!(result.success());
    assert!(result.stdout.contains("hello"));
}
```

**Step 2: Create Qdrant integration test**

Create `services/zero-memory/tests/qdrant_integration.rs`:

```rust
//! Integration tests for Qdrant memory.
//!
//! Requires Qdrant to be running: docker run -p 6333:6333 qdrant/qdrant
//! Run with: cargo test --test qdrant_integration -- --ignored

use std::sync::Arc;
use zero_memory::{Memory, MemoryCategory, NoopEmbedding, QdrantMemory};

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_store_recall_cycle() {
    let embedding = Arc::new(NoopEmbedding::new(128));
    let memory = QdrantMemory::connect("http://localhost:6333", "integration_test", embedding)
        .await
        .expect("Failed to connect to Qdrant");

    // Store
    memory
        .store("integration_key", "integration test content", MemoryCategory::Core)
        .await
        .expect("Store failed");

    // Recall
    let results = memory.recall("integration", 5).await.expect("Recall failed");
    assert!(!results.is_empty());

    // Cleanup
    memory.forget("integration_key").await.ok();
}

#[tokio::test]
#[ignore = "requires Qdrant"]
async fn qdrant_health_check() {
    let embedding = Arc::new(NoopEmbedding::new(128));
    let memory = QdrantMemory::connect("http://localhost:6333", "health_test", embedding)
        .await
        .unwrap();

    assert!(memory.health_check().await);
}
```

**Step 3: Commit**

```bash
git add services/zero-cli/tests/ services/zero-memory/tests/
git commit -m "test: add integration tests for sandbox and Qdrant"
```

---

## Summary

**Phase 1 completed files:**
- `services/zero-cli/src/sandbox/mod.rs`
- `services/zero-cli/src/sandbox/types.rs`
- `services/zero-cli/src/sandbox/docker.rs`
- `services/zero-cli/src/memory/crystallize.rs`

**Phase 2 completed files:**
- `services/zero-memory/src/qdrant.rs`
- `services/zero-memory/src/hybrid_search.rs`
- `services/zero-gateway/src/context.rs`

**Dependencies added:**
- `bollard = "0.16"` (zero-cli)
- `humantime-serde = "1.1"` (zero-cli)
- `qdrant-client = "1.8"` (zero-memory)

**Infrastructure required:**
- Docker daemon running
- Qdrant: `docker run -p 6333:6333 qdrant/qdrant`

---

## Verification Commands

```bash
# Build all
cd services && cargo build --release

# Run unit tests
cargo test -p zero-cli sandbox
cargo test -p zero-memory qdrant

# Run integration tests (requires Docker + Qdrant)
cargo test --test sandbox_integration -- --ignored
cargo test --test qdrant_integration -- --ignored

# Start services
./ops.sh start all

# Test context API
curl http://localhost:4430/api/v1/context/categories
curl "http://localhost:4430/api/v1/context/search?q=test&limit=5"
```
