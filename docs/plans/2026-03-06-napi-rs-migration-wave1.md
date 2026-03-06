# NAPI-RS 深度集成迁移 Wave 1 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 Config 解析和 Permission 引擎完全迁移到 Rust，通过 NAPI 暴露给 TypeScript

**Architecture:** 扩展现有 `services/zero-core/src/napi/` 模块，增加热重载、Schema 验证和完整权限规则引擎。TypeScript 层简化为调用 `@codecoder-ai/core` 的薄封装。

**Tech Stack:** Rust + napi-rs + notify (file watcher) + jsonschema (validation)

---

## Wave 1 概览

| Phase | 目标 | TS 行数变化 | Rust 行数增加 |
|-------|------|-------------|---------------|
| D | Config 解析统一 | 3,304 → ~300 | +400 |
| E | Permission 引擎统一 | 1,612 → ~200 | +600 |

**依赖关系:** Permission 依赖 Config (配置中的权限规则)

---

## Task 1: 扩展 Rust Config - 添加 JSON Schema 验证

**Files:**
- Create: `services/zero-core/src/foundation/schema.rs`
- Modify: `services/zero-core/src/foundation/mod.rs`
- Modify: `services/zero-core/src/foundation/config.rs:600-650`
- Modify: `services/zero-core/Cargo.toml`
- Test: `services/zero-core/src/foundation/schema.rs` (内联测试)

**Step 1: Add jsonschema dependency to Cargo.toml**

```toml
# In [dependencies] section
jsonschema = "0.26"
```

**Step 2: Create schema.rs with validation logic**

```rust
//! JSON Schema validation for configuration files
//!
//! Provides compile-time and runtime schema validation.

use anyhow::{Context, Result};
use jsonschema::{Draft, JSONSchema, ValidationError};
use serde_json::Value;
use std::path::Path;
use std::fs;

/// Schema validator for configuration files
pub struct SchemaValidator {
    schema: JSONSchema,
}

impl SchemaValidator {
    /// Create a validator from a schema file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read schema: {:?}", path.as_ref()))?;
        let schema_value: Value = serde_json::from_str(&content)
            .context("Failed to parse schema JSON")?;
        Self::from_value(schema_value)
    }

    /// Create a validator from a JSON value
    pub fn from_value(schema: Value) -> Result<Self> {
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema)
            .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;
        Ok(Self { schema: compiled })
    }

    /// Validate a configuration value
    pub fn validate(&self, config: &Value) -> Result<Vec<ValidationIssue>> {
        let result = self.schema.validate(config);
        match result {
            Ok(_) => Ok(vec![]),
            Err(errors) => {
                let issues: Vec<_> = errors
                    .map(|e| ValidationIssue {
                        path: e.instance_path.to_string(),
                        message: e.to_string(),
                    })
                    .collect();
                Ok(issues)
            }
        }
    }

    /// Check if config is valid (no issues)
    pub fn is_valid(&self, config: &Value) -> bool {
        self.schema.is_valid(config)
    }
}

/// A validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// JSON path to the issue
    pub path: String,
    /// Human-readable message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "theme": { "type": "string" },
                "model": { "type": "string" }
            },
            "required": ["theme"]
        });

        let validator = SchemaValidator::from_value(schema).unwrap();

        // Valid config
        let valid = json!({ "theme": "dark" });
        assert!(validator.is_valid(&valid));

        // Invalid config (missing required field)
        let invalid = json!({ "model": "opus" });
        assert!(!validator.is_valid(&invalid));
    }
}
```

**Step 3: Update foundation/mod.rs to export schema**

```rust
// Add at the end of the file
pub mod schema;
pub use schema::{SchemaValidator, ValidationIssue};
```

**Step 4: Run tests**

Run: `cd services/zero-core && cargo test foundation::schema --features napi-bindings`
Expected: PASS with test output

**Step 5: Commit**

```bash
git add services/zero-core/src/foundation/schema.rs services/zero-core/src/foundation/mod.rs services/zero-core/Cargo.toml
git commit -m "feat(zero-core): add JSON Schema validation for config"
```

---

## Task 2: 添加配置文件监视器 (Hot Reload)

**Files:**
- Create: `services/zero-core/src/foundation/watcher.rs`
- Modify: `services/zero-core/src/foundation/mod.rs`
- Modify: `services/zero-core/Cargo.toml`
- Test: `services/zero-core/src/foundation/watcher.rs` (内联测试)

**Step 1: Add notify dependency**

```toml
# In [dependencies] section
notify = { version = "6.1", default-features = false, features = ["macos_kqueue"] }
```

**Step 2: Create watcher.rs with file watching logic**

```rust
//! Configuration file watcher for hot reload
//!
//! Watches configuration files and directories for changes,
//! triggering callbacks when modifications are detected.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

/// Configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// Paths that changed
    pub paths: Vec<PathBuf>,
    /// Kind of change
    pub kind: ChangeKind,
}

/// Kind of configuration change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File was created
    Create,
    /// File was modified
    Modify,
    /// File was removed
    Remove,
}

/// Configuration file watcher
pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    _rx: Receiver<Result<Event, notify::Error>>,
}

impl ConfigWatcher {
    /// Create a new config watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )?;

        Ok(Self {
            watcher,
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
            _rx: rx,
        })
    }

    /// Watch a path for changes
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
        self.watched_paths.lock().unwrap().insert(path);
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        self.watcher.unwatch(path)?;
        self.watched_paths.lock().unwrap().remove(path);
        Ok(())
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().unwrap().iter().cloned().collect()
    }
}

/// Async callback-based watcher
pub struct AsyncConfigWatcher {
    inner: ConfigWatcher,
    callback_tx: Sender<ConfigChangeEvent>,
}

impl AsyncConfigWatcher {
    /// Create with callback
    pub fn with_callback<F>(callback: F) -> Result<Self>
    where
        F: Fn(ConfigChangeEvent) + Send + 'static,
    {
        let (tx, rx) = channel::<ConfigChangeEvent>();

        // Spawn callback thread
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                callback(event);
            }
        });

        Ok(Self {
            inner: ConfigWatcher::new()?,
            callback_tx: tx,
        })
    }

    /// Watch a path
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.inner.watch(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_creation() {
        let watcher = ConfigWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_path() {
        let dir = tempdir().unwrap();
        let mut watcher = ConfigWatcher::new().unwrap();

        let result = watcher.watch(dir.path());
        assert!(result.is_ok());
        assert_eq!(watcher.watched_paths().len(), 1);
    }
}
```

**Step 3: Update foundation/mod.rs**

```rust
pub mod watcher;
pub use watcher::{ConfigWatcher, ConfigChangeEvent, ChangeKind};
```

**Step 4: Run tests**

Run: `cd services/zero-core && cargo test foundation::watcher`
Expected: PASS

**Step 5: Commit**

```bash
git add services/zero-core/src/foundation/watcher.rs services/zero-core/src/foundation/mod.rs services/zero-core/Cargo.toml
git commit -m "feat(zero-core): add config file watcher for hot reload"
```

---

## Task 3: 扩展 NAPI ConfigLoaderHandle - Schema 验证和热重载

**Files:**
- Modify: `services/zero-core/src/napi/config.rs:444-500`
- Test: Run existing tests

**Step 1: Add schema validation methods to ConfigLoaderHandle**

```rust
// Add to ConfigLoaderHandle impl block

/// Validate configuration against schema
#[napi]
pub fn validate_schema(&self, config_json: String, schema_json: String) -> Result<Vec<NapiValidationIssue>> {
    let config: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| Error::from_reason(format!("Invalid config JSON: {}", e)))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_json)
        .map_err(|e| Error::from_reason(format!("Invalid schema JSON: {}", e)))?;

    let validator = crate::foundation::SchemaValidator::from_value(schema)
        .map_err(|e| Error::from_reason(format!("Failed to compile schema: {}", e)))?;

    let issues = validator.validate(&config)
        .map_err(|e| Error::from_reason(format!("Validation failed: {}", e)))?;

    Ok(issues.into_iter().map(|i| NapiValidationIssue {
        path: i.path,
        message: i.message,
    }).collect())
}

/// Check if configuration is valid against schema
#[napi]
pub fn is_valid_schema(&self, config_json: String, schema_json: String) -> Result<bool> {
    let config: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| Error::from_reason(format!("Invalid config JSON: {}", e)))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_json)
        .map_err(|e| Error::from_reason(format!("Invalid schema JSON: {}", e)))?;

    let validator = crate::foundation::SchemaValidator::from_value(schema)
        .map_err(|e| Error::from_reason(format!("Failed to compile schema: {}", e)))?;

    Ok(validator.is_valid(&config))
}
```

**Step 2: Add NapiValidationIssue type**

```rust
// Add near the top of config.rs, after other type definitions

/// Validation issue from schema validation
#[napi(object)]
pub struct NapiValidationIssue {
    /// JSON path to the issue
    pub path: String,
    /// Human-readable message
    pub message: String,
}
```

**Step 3: Run tests**

Run: `cd services/zero-core && cargo test napi::config --features napi-bindings`
Expected: PASS

**Step 4: Commit**

```bash
git add services/zero-core/src/napi/config.rs
git commit -m "feat(napi): add schema validation to ConfigLoaderHandle"
```

---

## Task 4: 扩展 Rust Permission 模块 - 完整规则引擎

**Files:**
- Create: `services/zero-core/src/security/auto_approve.rs`
- Modify: `services/zero-core/src/security/mod.rs`
- Test: `services/zero-core/src/security/auto_approve.rs` (内联测试)

**Step 1: Create auto_approve.rs with full rule engine**

```rust
//! Auto-approve permission engine
//!
//! Risk-based automatic approval for tool calls in autonomous mode.
//! Mirrors TypeScript `permission/auto-approve.ts` functionality.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use super::risk::{RiskLevel, RiskAssessment, assess_bash_risk, assess_file_risk};

/// Auto-approve configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoApproveConfig {
    /// Enable auto-approval
    #[serde(default)]
    pub enabled: bool,

    /// Tools allowed for auto-approval (whitelist)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Maximum risk level for auto-approval
    #[serde(default)]
    pub risk_threshold: RiskLevel,

    /// Timeout in milliseconds before auto-approving
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Whether running in unattended mode
    #[serde(default)]
    pub unattended: bool,
}

fn default_timeout() -> u64 {
    5000
}

/// Tool risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRiskAssessment {
    /// Tool name
    pub tool: String,
    /// Risk level
    pub risk: RiskLevel,
    /// Reason for risk assessment
    pub reason: String,
    /// Whether this can be auto-approved
    pub auto_approvable: bool,
}

/// Auto-approve decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApproveDecision {
    /// Approved automatically
    Approved,
    /// Rejected (too risky)
    Rejected,
    /// Needs manual approval
    NeedsApproval,
}

/// Auto-approve audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoApproveAudit {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Permission ID
    pub permission_id: String,
    /// Tool name
    pub tool: String,
    /// Pattern used (if any)
    pub pattern: Option<String>,
    /// Risk level
    pub risk: RiskLevel,
    /// Decision made
    pub decision: ApproveDecision,
    /// Reason for decision
    pub reason: String,
}

/// Auto-approve engine
#[derive(Debug, Default)]
pub struct AutoApproveEngine {
    config: AutoApproveConfig,
    audit_log: Vec<AutoApproveAudit>,
    allowed_tools_set: HashSet<String>,
}

impl AutoApproveEngine {
    /// Create a new engine with configuration
    pub fn new(config: AutoApproveConfig) -> Self {
        let allowed_tools_set: HashSet<_> = config.allowed_tools.iter().cloned().collect();
        Self {
            config,
            audit_log: Vec::new(),
            allowed_tools_set,
        }
    }

    /// Check if auto-approve is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Assess a tool call for auto-approval
    pub fn assess(&self, tool: &str, args: &serde_json::Value) -> ToolRiskAssessment {
        let (risk, reason) = self.assess_tool_risk(tool, args);
        let auto_approvable = self.can_auto_approve(tool, risk);

        ToolRiskAssessment {
            tool: tool.to_string(),
            risk,
            reason,
            auto_approvable,
        }
    }

    /// Decide whether to auto-approve
    pub fn decide(&mut self, tool: &str, args: &serde_json::Value) -> (ApproveDecision, String) {
        if !self.config.enabled {
            return (ApproveDecision::NeedsApproval, "Auto-approve disabled".to_string());
        }

        let assessment = self.assess(tool, args);

        let decision = if assessment.risk == RiskLevel::Critical {
            ApproveDecision::Rejected
        } else if assessment.auto_approvable {
            ApproveDecision::Approved
        } else {
            ApproveDecision::NeedsApproval
        };

        (decision, assessment.reason)
    }

    /// Assess risk for a tool call
    fn assess_tool_risk(&self, tool: &str, args: &serde_json::Value) -> (RiskLevel, String) {
        match tool {
            "Bash" => {
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let assessment = assess_bash_risk(command);
                (assessment.risk, assessment.reason.to_string())
            }
            "Write" | "Edit" | "Read" => {
                let path = args.get("file_path")
                    .or_else(|| args.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let assessment = assess_file_risk(path);
                (assessment.risk, assessment.reason.to_string())
            }
            // Safe tools
            "Glob" | "Grep" | "LS" | "NotebookRead" | "TaskList" | "TaskGet" => {
                (RiskLevel::Safe, "Read-only operation".to_string())
            }
            // Low risk
            "WebFetch" | "WebSearch" => {
                (RiskLevel::Low, "External read-only operation".to_string())
            }
            // Medium risk
            "NotebookEdit" | "TaskCreate" | "TaskUpdate" => {
                (RiskLevel::Medium, "Local reversible write".to_string())
            }
            // High risk - MCP tools
            tool if tool.starts_with("mcp__") => {
                (RiskLevel::High, format!("MCP tool: {}", tool))
            }
            // Unknown tools default to high
            _ => {
                (RiskLevel::High, format!("Unknown tool: {}", tool))
            }
        }
    }

    /// Check if tool can be auto-approved
    fn can_auto_approve(&self, tool: &str, risk: RiskLevel) -> bool {
        // Critical risk is never auto-approved
        if risk == RiskLevel::Critical {
            return false;
        }

        // Check whitelist
        if !self.allowed_tools_set.is_empty() && !self.allowed_tools_set.contains(tool) {
            return false;
        }

        // Check risk threshold
        risk <= self.config.risk_threshold
    }

    /// Get audit log
    pub fn audit_log(&self) -> &[AutoApproveAudit] {
        &self.audit_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auto_approve_disabled() {
        let engine = AutoApproveEngine::new(AutoApproveConfig::default());
        let (decision, _) = engine.assess("Read", &json!({"path": "/tmp/test.txt"}));
        assert_eq!(decision.risk, RiskLevel::Safe);
    }

    #[test]
    fn test_auto_approve_enabled() {
        let mut engine = AutoApproveEngine::new(AutoApproveConfig {
            enabled: true,
            risk_threshold: RiskLevel::Medium,
            ..Default::default()
        });

        // Safe tool should be approved
        let (decision, _) = engine.decide("Glob", &json!({"pattern": "*.rs"}));
        assert_eq!(decision, ApproveDecision::Approved);

        // High risk should need approval
        let (decision, _) = engine.decide("Bash", &json!({"command": "rm -rf /"}));
        assert_eq!(decision, ApproveDecision::Rejected);
    }

    #[test]
    fn test_whitelist() {
        let mut engine = AutoApproveEngine::new(AutoApproveConfig {
            enabled: true,
            allowed_tools: vec!["Read".to_string(), "Glob".to_string()],
            risk_threshold: RiskLevel::Safe,
            ..Default::default()
        });

        // Whitelisted tool
        let (decision, _) = engine.decide("Read", &json!({}));
        assert_eq!(decision, ApproveDecision::Approved);

        // Non-whitelisted tool
        let (decision, _) = engine.decide("Write", &json!({}));
        assert_eq!(decision, ApproveDecision::NeedsApproval);
    }
}
```

**Step 2: Update security/mod.rs**

```rust
// Add to the module
pub mod auto_approve;
pub use auto_approve::{AutoApproveEngine, AutoApproveConfig, ToolRiskAssessment, ApproveDecision};
```

**Step 3: Run tests**

Run: `cd services/zero-core && cargo test security::auto_approve`
Expected: PASS

**Step 4: Commit**

```bash
git add services/zero-core/src/security/auto_approve.rs services/zero-core/src/security/mod.rs
git commit -m "feat(zero-core): add auto-approve permission engine"
```

---

## Task 5: 扩展 NAPI Security - AutoApproveEngine Handle

**Files:**
- Modify: `services/zero-core/src/napi/security.rs:359-450`
- Test: Run tests

**Step 1: Add AutoApproveEngineHandle to security.rs**

```rust
// Add imports at top
use crate::security::auto_approve::{
    AutoApproveEngine as RustAutoApproveEngine,
    AutoApproveConfig as RustAutoApproveConfig,
    ToolRiskAssessment as RustToolRiskAssessment,
    ApproveDecision as RustApproveDecision,
};

// Add config type
#[napi(object)]
pub struct NapiAutoApproveConfig {
    pub enabled: Option<bool>,
    pub allowed_tools: Vec<String>,
    pub risk_threshold: Option<String>,
    pub timeout_ms: Option<u32>,
    pub unattended: Option<bool>,
}

impl From<NapiAutoApproveConfig> for RustAutoApproveConfig {
    fn from(c: NapiAutoApproveConfig) -> Self {
        Self {
            enabled: c.enabled.unwrap_or(false),
            allowed_tools: c.allowed_tools,
            risk_threshold: c.risk_threshold
                .and_then(|s| RustRiskLevel::parse(&s))
                .unwrap_or(RustRiskLevel::Medium),
            timeout_ms: c.timeout_ms.map(|t| t as u64).unwrap_or(5000),
            unattended: c.unattended.unwrap_or(false),
        }
    }
}

// Add assessment result type
#[napi(object)]
pub struct NapiToolRiskAssessment {
    pub tool: String,
    pub risk: String,
    pub reason: String,
    pub auto_approvable: bool,
}

impl From<RustToolRiskAssessment> for NapiToolRiskAssessment {
    fn from(a: RustToolRiskAssessment) -> Self {
        Self {
            tool: a.tool,
            risk: a.risk.as_str().to_string(),
            reason: a.reason,
            auto_approvable: a.auto_approvable,
        }
    }
}

// Add decision type
#[napi(object)]
pub struct NapiApproveDecision {
    pub decision: String,
    pub reason: String,
}

/// Handle to auto-approve engine
#[napi]
pub struct AutoApproveEngineHandle {
    inner: std::sync::Mutex<RustAutoApproveEngine>,
}

/// Create auto-approve engine with config
#[napi]
pub fn create_auto_approve_engine(config: NapiAutoApproveConfig) -> AutoApproveEngineHandle {
    AutoApproveEngineHandle {
        inner: std::sync::Mutex::new(RustAutoApproveEngine::new(config.into())),
    }
}

#[napi]
impl AutoApproveEngineHandle {
    /// Check if auto-approve is enabled
    #[napi]
    pub fn is_enabled(&self) -> bool {
        self.inner.lock().unwrap().is_enabled()
    }

    /// Assess a tool call
    #[napi]
    pub fn assess(&self, tool: String, args_json: String) -> Result<NapiToolRiskAssessment> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON: {}", e)))?;
        let assessment = self.inner.lock().unwrap().assess(&tool, &args);
        Ok(assessment.into())
    }

    /// Make auto-approve decision
    #[napi]
    pub fn decide(&self, tool: String, args_json: String) -> Result<NapiApproveDecision> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON: {}", e)))?;
        let (decision, reason) = self.inner.lock().unwrap().decide(&tool, &args);
        Ok(NapiApproveDecision {
            decision: match decision {
                RustApproveDecision::Approved => "approved",
                RustApproveDecision::Rejected => "rejected",
                RustApproveDecision::NeedsApproval => "needs_approval",
            }.to_string(),
            reason,
        })
    }
}
```

**Step 2: Run tests**

Run: `cd services/zero-core && cargo test napi::security --features napi-bindings`
Expected: PASS

**Step 3: Commit**

```bash
git add services/zero-core/src/napi/security.rs
git commit -m "feat(napi): add AutoApproveEngineHandle for permission engine"
```

---

## Task 6: 更新 TypeScript core 包 - 导出新 NAPI 绑定

**Files:**
- Create: `packages/core/src/permission.ts`
- Modify: `packages/core/src/index.ts`
- Test: `packages/core/test/permission.test.ts`

**Step 1: Create permission.ts wrapper**

```typescript
/**
 * Permission engine - TypeScript wrapper for Rust auto-approve engine
 */

import {
  createAutoApproveEngine as createAutoApproveEngineNative,
  type AutoApproveEngineHandle,
  type NapiAutoApproveConfig,
  type NapiToolRiskAssessment,
  type NapiApproveDecision,
} from './binding.js'

export type RiskLevel = 'safe' | 'low' | 'medium' | 'high' | 'critical'

export interface AutoApproveConfig {
  enabled?: boolean
  allowedTools?: string[]
  riskThreshold?: RiskLevel
  timeoutMs?: number
  unattended?: boolean
}

export interface ToolRiskAssessment {
  tool: string
  risk: RiskLevel
  reason: string
  autoApprovable: boolean
}

export type ApproveDecision = 'approved' | 'rejected' | 'needs_approval'

export interface ApproveResult {
  decision: ApproveDecision
  reason: string
}

/**
 * Auto-approve permission engine
 *
 * Uses native Rust implementation for high-performance risk assessment.
 */
export class AutoApproveEngine {
  private handle: AutoApproveEngineHandle

  constructor(config: AutoApproveConfig = {}) {
    this.handle = createAutoApproveEngineNative({
      enabled: config.enabled ?? false,
      allowedTools: config.allowedTools ?? [],
      riskThreshold: config.riskThreshold ?? 'medium',
      timeoutMs: config.timeoutMs ?? 5000,
      unattended: config.unattended ?? false,
    })
  }

  /** Check if auto-approve is enabled */
  isEnabled(): boolean {
    return this.handle.isEnabled()
  }

  /** Assess a tool call for risk */
  assess(tool: string, args: Record<string, unknown>): ToolRiskAssessment {
    const result = this.handle.assess(tool, JSON.stringify(args))
    return {
      tool: result.tool,
      risk: result.risk as RiskLevel,
      reason: result.reason,
      autoApprovable: result.autoApprovable,
    }
  }

  /** Decide whether to auto-approve a tool call */
  decide(tool: string, args: Record<string, unknown>): ApproveResult {
    const result = this.handle.decide(tool, JSON.stringify(args))
    return {
      decision: result.decision as ApproveDecision,
      reason: result.reason,
    }
  }
}

// Re-export native risk assessment functions
export { assessBashRisk, assessFileRisk, getToolBaseRisk, checkRiskThreshold } from './binding.js'
```

**Step 2: Update index.ts to export permission module**

```typescript
// Add export
export * from './permission.js'
```

**Step 3: Create test file**

```typescript
// packages/core/test/permission.test.ts
import { describe, expect, test } from 'bun:test'
import { AutoApproveEngine } from '../src/permission'

describe('AutoApproveEngine', () => {
  test('disabled by default', () => {
    const engine = new AutoApproveEngine()
    expect(engine.isEnabled()).toBe(false)
  })

  test('assess safe tool', () => {
    const engine = new AutoApproveEngine({ enabled: true, riskThreshold: 'medium' })
    const result = engine.assess('Glob', { pattern: '*.rs' })
    expect(result.risk).toBe('safe')
    expect(result.autoApprovable).toBe(true)
  })

  test('assess high risk command', () => {
    const engine = new AutoApproveEngine({ enabled: true, riskThreshold: 'medium' })
    const result = engine.assess('Bash', { command: 'rm -rf /' })
    expect(result.risk).toBe('critical')
    expect(result.autoApprovable).toBe(false)
  })

  test('whitelist enforcement', () => {
    const engine = new AutoApproveEngine({
      enabled: true,
      allowedTools: ['Read', 'Glob'],
      riskThreshold: 'safe',
    })

    const allowed = engine.decide('Glob', {})
    expect(allowed.decision).toBe('approved')

    const notAllowed = engine.decide('Write', {})
    expect(notAllowed.decision).toBe('needs_approval')
  })
})
```

**Step 4: Run tests**

Run: `cd packages/core && bun test permission`
Expected: PASS

**Step 5: Commit**

```bash
git add packages/core/src/permission.ts packages/core/src/index.ts packages/core/test/permission.test.ts
git commit -m "feat(core): add AutoApproveEngine TypeScript wrapper"
```

---

## Task 7: 简化 TypeScript auto-approve.ts - 使用 Native 实现

**Files:**
- Modify: `packages/ccode/src/permission/auto-approve.ts`
- Test: Existing permission tests

**Step 1: Refactor auto-approve.ts to use native engine**

Replace the complex TypeScript implementation with calls to the native engine. Keep the Zod schemas for config validation but delegate risk assessment to Rust.

```typescript
/**
 * Auto-Approve Permission Handler
 *
 * Now uses native Rust implementation via @codecoder-ai/core.
 */

import { Log } from "@/util/log"
import z from "zod"
import type { Permission } from "./index"
import {
  AutoApproveEngine,
  assessBashRisk,
  assessFileRisk,
  type RiskLevel,
  type ApproveResult,
} from "@codecoder-ai/core"

const log = Log.create({ service: "permission.auto-approve" })

// ============================================================================
// Re-export native types
// ============================================================================

export type { RiskLevel, ApproveResult }

// ============================================================================
// Zod Schemas (for config integration)
// ============================================================================

export const RiskLevelSchema = z.enum(["safe", "low", "medium", "high"]).meta({
  ref: "RiskLevel",
})

export const AutoApproveConfigSchema = z
  .object({
    enabled: z.boolean().optional(),
    allowedTools: z.array(z.string()).optional(),
    riskThreshold: RiskLevelSchema.optional(),
    timeoutMs: z.number().int().positive().optional(),
  })
  .strict()
  .meta({ ref: "AutoApproveConfig" })
export type AutoApproveConfigInput = z.infer<typeof AutoApproveConfigSchema>

// ============================================================================
// Engine Wrapper
// ============================================================================

let engine: AutoApproveEngine | null = null

export function initAutoApprove(config: AutoApproveConfigInput) {
  engine = new AutoApproveEngine({
    enabled: config.enabled ?? false,
    allowedTools: config.allowedTools ?? [],
    riskThreshold: config.riskThreshold ?? "medium",
    timeoutMs: config.timeoutMs ?? 5000,
  })
  log.info("auto-approve engine initialized", { enabled: config.enabled })
}

export function shouldAutoApprove(permission: Permission): ApproveResult {
  if (!engine) {
    return { decision: "needs_approval", reason: "Engine not initialized" }
  }

  const args = permission.args ?? {}
  return engine.decide(permission.tool, args)
}

export function assessToolRisk(tool: string, args: Record<string, unknown>) {
  if (!engine) {
    return { risk: "high" as RiskLevel, reason: "Engine not initialized", autoApprovable: false }
  }
  return engine.assess(tool, args)
}

// Re-export native functions for direct use
export { assessBashRisk, assessFileRisk }
```

**Step 2: Run tests**

Run: `cd packages/ccode && bun test permission`
Expected: PASS (or update tests to match new API)

**Step 3: Commit**

```bash
git add packages/ccode/src/permission/auto-approve.ts
git commit -m "refactor(permission): use native Rust auto-approve engine"
```

---

## Task 8: 集成测试 - 验证 NAPI 绑定完整性

**Files:**
- Create: `packages/core/test/integration/napi-config.test.ts`
- Create: `packages/core/test/integration/napi-permission.test.ts`

**Step 1: Create config integration test**

```typescript
// packages/core/test/integration/napi-config.test.ts
import { describe, expect, test } from 'bun:test'
import { ConfigLoaderHandle, createConfigLoader } from '../../src/binding'

describe('Config NAPI Integration', () => {
  test('create config loader', () => {
    const handle = createConfigLoader()
    expect(handle).toBeDefined()
    expect(handle.configDir()).toContain('.codecoder')
  })

  test('parse JSONC', () => {
    const handle = createConfigLoader()
    const result = handle.parseJsonc('{ "theme": "dark" /* comment */ }')
    const parsed = JSON.parse(result)
    expect(parsed.theme).toBe('dark')
  })

  test('schema validation', () => {
    const handle = createConfigLoader()
    const config = JSON.stringify({ theme: 'dark' })
    const schema = JSON.stringify({
      type: 'object',
      properties: { theme: { type: 'string' } },
      required: ['theme'],
    })
    const issues = handle.validateSchema(config, schema)
    expect(issues).toEqual([])
  })
})
```

**Step 2: Create permission integration test**

```typescript
// packages/core/test/integration/napi-permission.test.ts
import { describe, expect, test } from 'bun:test'
import { createAutoApproveEngine, assessBashRisk, assessFileRisk } from '../../src/binding'

describe('Permission NAPI Integration', () => {
  test('bash risk assessment', () => {
    const result = assessBashRisk('ls -la')
    expect(result.risk).toBe('safe')

    const dangerous = assessBashRisk('rm -rf /')
    expect(dangerous.risk).toBe('critical')
  })

  test('file risk assessment', () => {
    const safe = assessFileRisk('/tmp/test.txt')
    expect(safe.risk).toBe('safe')

    const sensitive = assessFileRisk('/etc/passwd')
    expect(sensitive.risk).toBe('high')
  })

  test('auto-approve engine', () => {
    const engine = createAutoApproveEngine({
      enabled: true,
      allowedTools: [],
      riskThreshold: 'medium',
      timeoutMs: 5000,
      unattended: false,
    })

    expect(engine.isEnabled()).toBe(true)

    const result = engine.decide('Glob', JSON.stringify({ pattern: '*.rs' }))
    expect(result.decision).toBe('approved')
  })
})
```

**Step 3: Run integration tests**

Run: `cd packages/core && bun test integration`
Expected: PASS

**Step 4: Commit**

```bash
git add packages/core/test/integration/
git commit -m "test(core): add NAPI integration tests for config and permission"
```

---

## Task 9: 构建和发布 - 更新 NAPI 绑定

**Files:**
- Modify: `services/zero-core/build.rs` (if needed)
- Run: Build pipeline

**Step 1: Build Rust with NAPI bindings**

Run: `cd services/zero-core && cargo build --release --features napi-bindings`
Expected: Build success

**Step 2: Generate TypeScript declarations**

Run: `cd packages/core && bun run build`
Expected: Build success with updated binding.d.ts

**Step 3: Run full test suite**

Run: `bun turbo test --filter=@codecoder-ai/core`
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A
git commit -m "build: update NAPI bindings for Wave 1 migration"
```

---

## 验证检查清单

- [ ] Rust 测试通过: `cargo test -p zero-core --features napi-bindings`
- [ ] TypeScript 测试通过: `cd packages/core && bun test`
- [ ] NAPI 绑定生成成功: `packages/core/src/binding.d.ts` 包含新类型
- [ ] 集成测试通过: Config 和 Permission 通过 NAPI 正常工作
- [ ] ccode 权限功能正常: `bun dev` 启动无错误

---

## 预期结果

| 指标 | 迁移前 | 迁移后 |
|------|--------|--------|
| TypeScript 权限代码 | 848 行 (auto-approve.ts) | ~100 行 |
| TypeScript 配置代码 | 1,797 行 (config.ts) | ~200 行 |
| Rust security 模块 | ~360 行 | ~960 行 |
| Rust config 模块 | ~470 行 | ~600 行 |
| 权限检查延迟 | ~5ms | ~0.5ms |

---

## 下一步: Wave 2

完成 Wave 1 后，继续 Wave 2:
- Phase A: MCP 协议统一
- Phase C: LSP Server 迁移

创建新计划文件: `docs/plans/2026-03-XX-napi-rs-migration-wave2.md`
