//! NAPI bindings for security - prompt injection scanner and risk assessment
//!
//! This module exposes the prompt injection scanner, risk assessment,
//! and auto-approve engine to Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::security::injection::{
    InjectionScanner as RustInjectionScanner,
    InjectionScanResult as RustInjectionScanResult,
    InjectionPattern as RustInjectionPattern,
    InjectionType as RustInjectionType,
    InjectionSeverity as RustInjectionSeverity,
    ScannerConfig as RustScannerConfig,
};

use crate::security::risk::{
    RiskLevel as RustRiskLevel,
    RiskAssessment as RustRiskAssessment,
    assess_bash_risk as rust_assess_bash_risk,
    assess_file_risk as rust_assess_file_risk,
    tool_base_risk as rust_tool_base_risk,
    risk_at_or_below_threshold as rust_risk_at_or_below_threshold,
};

use crate::security::auto_approve::{
    AutoApproveConfig as RustAutoApproveConfig,
    AutoApproveEngine as RustAutoApproveEngine,
    ApprovalDecision as RustApprovalDecision,
    ToolInput as RustToolInput,
    ExecutionContext as RustExecutionContext,
    AdaptiveRiskResult as RustAdaptiveRiskResult,
    AuditEntry as RustAuditEntry,
};

// ============================================================================
// Type conversions
// ============================================================================

/// Injection type
#[napi(string_enum)]
pub enum InjectionType {
    Jailbreak,
    RoleOverride,
    InstructionLeak,
    DelimiterAttack,
    EncodingBypass,
    ContextManipulation,
}

impl From<RustInjectionType> for InjectionType {
    fn from(t: RustInjectionType) -> Self {
        match t {
            RustInjectionType::Jailbreak => InjectionType::Jailbreak,
            RustInjectionType::RoleOverride => InjectionType::RoleOverride,
            RustInjectionType::InstructionLeak => InjectionType::InstructionLeak,
            RustInjectionType::DelimiterAttack => InjectionType::DelimiterAttack,
            RustInjectionType::EncodingBypass => InjectionType::EncodingBypass,
            RustInjectionType::ContextManipulation => InjectionType::ContextManipulation,
        }
    }
}

/// Injection severity
#[napi(string_enum)]
pub enum InjectionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl From<RustInjectionSeverity> for InjectionSeverity {
    fn from(s: RustInjectionSeverity) -> Self {
        match s {
            RustInjectionSeverity::Low => InjectionSeverity::Low,
            RustInjectionSeverity::Medium => InjectionSeverity::Medium,
            RustInjectionSeverity::High => InjectionSeverity::High,
            RustInjectionSeverity::Critical => InjectionSeverity::Critical,
        }
    }
}

/// Detected injection pattern
#[napi(object)]
pub struct InjectionPattern {
    /// Type of injection
    pub injection_type: String,
    /// Matched text
    pub matched: String,
    /// Position in input string
    pub position: u32,
    /// Severity level
    pub severity: String,
    /// Description
    pub description: String,
}

impl From<RustInjectionPattern> for InjectionPattern {
    fn from(p: RustInjectionPattern) -> Self {
        Self {
            injection_type: format!("{:?}", p.injection_type).to_lowercase(),
            matched: p.matched,
            position: p.position as u32,
            severity: format!("{:?}", p.severity).to_lowercase(),
            description: p.description,
        }
    }
}

/// Injection scan result
#[napi(object)]
pub struct InjectionScanResult {
    /// Whether injection was detected
    pub detected: bool,
    /// Confidence level (0.0-1.0)
    pub confidence: f64,
    /// Detected patterns
    pub patterns: Vec<InjectionPattern>,
    /// Sanitized input (if detected)
    pub sanitized: Option<String>,
    /// Scan duration in milliseconds
    pub duration_ms: f64,
}

impl From<RustInjectionScanResult> for InjectionScanResult {
    fn from(r: RustInjectionScanResult) -> Self {
        Self {
            detected: r.detected,
            confidence: r.confidence,
            patterns: r.patterns.into_iter().map(Into::into).collect(),
            sanitized: r.sanitized,
            duration_ms: r.duration_ms,
        }
    }
}

/// Scanner configuration
#[napi(object)]
pub struct InjectionScannerConfig {
    /// Enable strict mode
    pub strict: Option<bool>,
    /// Maximum input length
    pub max_input_length: Option<u32>,
    /// Check encoding bypass
    pub check_encoding_bypass: Option<bool>,
}

impl From<InjectionScannerConfig> for RustScannerConfig {
    fn from(c: InjectionScannerConfig) -> Self {
        let mut config = RustScannerConfig::default();
        if let Some(strict) = c.strict {
            config.strict = strict;
        }
        if let Some(max_len) = c.max_input_length {
            config.max_input_length = max_len as usize;
        }
        if let Some(check) = c.check_encoding_bypass {
            config.check_encoding_bypass = check;
        }
        config
    }
}

// ============================================================================
// NAPI functions
// ============================================================================

/// Scan input for prompt injection
#[napi]
pub fn scan_injection(input: String) -> InjectionScanResult {
    RustInjectionScanner::new().scan(&input).into()
}

/// Scan input with custom configuration
#[napi]
pub fn scan_injection_with_config(input: String, config: InjectionScannerConfig) -> InjectionScanResult {
    let rust_config: RustScannerConfig = config.into();
    RustInjectionScanner::with_config(rust_config).scan(&input).into()
}

/// Quick check if input might contain injection
#[napi]
pub fn quick_check_injection(input: String) -> bool {
    RustInjectionScanner::new().quick_check(&input)
}

/// Sanitize input by removing injection patterns
#[napi]
pub fn sanitize_injection_input(input: String) -> String {
    RustInjectionScanner::new().sanitize(&input)
}

/// Handle to an injection scanner instance (for reuse)
#[napi]
pub struct InjectionScannerHandle {
    inner: RustInjectionScanner,
}

/// Create a new injection scanner
#[napi]
pub fn create_injection_scanner() -> InjectionScannerHandle {
    InjectionScannerHandle {
        inner: RustInjectionScanner::new(),
    }
}

/// Create a new injection scanner with configuration
#[napi]
pub fn create_injection_scanner_with_config(config: InjectionScannerConfig) -> InjectionScannerHandle {
    InjectionScannerHandle {
        inner: RustInjectionScanner::with_config(config.into()),
    }
}

#[napi]
impl InjectionScannerHandle {
    /// Scan input for injection
    #[napi]
    pub fn scan(&self, input: String) -> InjectionScanResult {
        self.inner.scan(&input).into()
    }

    /// Quick check for injection
    #[napi]
    pub fn quick_check(&self, input: String) -> bool {
        self.inner.quick_check(&input)
    }

    /// Sanitize input
    #[napi]
    pub fn sanitize(&self, input: String) -> String {
        self.inner.sanitize(&input)
    }
}

// ============================================================================
// Risk Assessment Types
// ============================================================================

/// Risk level for tool operations
#[napi(string_enum)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl From<RustRiskLevel> for RiskLevel {
    fn from(r: RustRiskLevel) -> Self {
        match r {
            RustRiskLevel::Safe => RiskLevel::Safe,
            RustRiskLevel::Low => RiskLevel::Low,
            RustRiskLevel::Medium => RiskLevel::Medium,
            RustRiskLevel::High => RiskLevel::High,
            RustRiskLevel::Critical => RiskLevel::Critical,
        }
    }
}

impl From<RiskLevel> for RustRiskLevel {
    fn from(r: RiskLevel) -> Self {
        match r {
            RiskLevel::Safe => RustRiskLevel::Safe,
            RiskLevel::Low => RustRiskLevel::Low,
            RiskLevel::Medium => RustRiskLevel::Medium,
            RiskLevel::High => RustRiskLevel::High,
            RiskLevel::Critical => RustRiskLevel::Critical,
        }
    }
}

/// Risk assessment result
#[napi(object)]
pub struct RiskResult {
    /// Risk level as string
    pub risk: String,
    /// Human-readable reason
    pub reason: String,
    /// Whether this can be auto-approved (not critical)
    pub auto_approvable: bool,
}

impl From<RustRiskAssessment> for RiskResult {
    fn from(r: RustRiskAssessment) -> Self {
        Self {
            risk: r.risk.as_str().to_string(),
            reason: r.reason.to_string(),
            auto_approvable: r.auto_approvable(),
        }
    }
}

// ============================================================================
// Risk Assessment Functions
// ============================================================================

/// Assess risk level for a Bash command
///
/// Returns the highest matching risk level based on pattern analysis.
/// Unknown commands default to "high" risk.
///
/// @param command - The Bash command to assess
/// @returns Risk assessment result with level, reason, and auto-approvability
#[napi]
pub fn assess_bash_risk(command: String) -> RiskResult {
    rust_assess_bash_risk(&command).into()
}

/// Assess risk level for a file path operation
///
/// Evaluates file sensitivity based on:
/// - File extension (.env, .pem, .key, etc.)
/// - Directory location (/etc, /usr, ~/.ssh, etc.)
/// - File purpose (credentials, secrets, CI config)
///
/// @param path - The file path to assess
/// @returns Risk assessment result
#[napi]
pub fn assess_file_risk(path: String) -> RiskResult {
    rust_assess_file_risk(&path).into()
}

/// Get base risk level for a tool
///
/// Returns the default risk level for a tool type:
/// - Safe: Read, Glob, Grep, LS
/// - Low: WebFetch, WebSearch
/// - Medium: Write, Edit, TaskCreate
/// - High: Bash, Task, MCP tools
///
/// @param tool - The tool name
/// @returns Risk level as string
#[napi]
pub fn get_tool_base_risk(tool: String) -> String {
    rust_tool_base_risk(&tool).as_str().to_string()
}

/// Check if risk is at or below threshold
///
/// @param risk - The risk level to check
/// @param threshold - The maximum acceptable risk level
/// @returns true if risk <= threshold
#[napi]
pub fn check_risk_threshold(risk: String, threshold: String) -> bool {
    let Some(r) = RustRiskLevel::parse(&risk) else {
        return false;
    };
    let Some(t) = RustRiskLevel::parse(&threshold) else {
        return false;
    };
    rust_risk_at_or_below_threshold(r, t)
}

/// Parse risk level string
///
/// @param level - Risk level string (case-insensitive)
/// @returns Parsed risk level or "medium" if invalid
#[napi]
pub fn parse_risk_level(level: String) -> String {
    RustRiskLevel::parse(&level)
        .unwrap_or(RustRiskLevel::Medium)
        .as_str()
        .to_string()
}

// ============================================================================
// Auto-Approve Engine Types
// ============================================================================

/// Auto-approve configuration for NAPI
#[napi(object)]
pub struct AutoApproveConfig {
    /// Enable auto-approval
    pub enabled: bool,

    /// Tools allowed for auto-approval (whitelist)
    pub allowed_tools: Vec<String>,

    /// Maximum risk level for auto-approval ("safe", "low", "medium", "high")
    pub risk_threshold: String,

    /// Timeout in milliseconds before auto-approving
    pub timeout_ms: u32,

    /// Whether running in unattended mode
    pub unattended: bool,
}

impl From<AutoApproveConfig> for RustAutoApproveConfig {
    fn from(c: AutoApproveConfig) -> Self {
        Self {
            enabled: c.enabled,
            allowed_tools: c.allowed_tools,
            risk_threshold: RustRiskLevel::parse(&c.risk_threshold)
                .unwrap_or(RustRiskLevel::Low),
            timeout_ms: c.timeout_ms as u64,
            unattended: c.unattended,
        }
    }
}

impl From<&RustAutoApproveConfig> for AutoApproveConfig {
    fn from(c: &RustAutoApproveConfig) -> Self {
        Self {
            enabled: c.enabled,
            allowed_tools: c.allowed_tools.clone(),
            risk_threshold: c.risk_threshold.as_str().to_string(),
            timeout_ms: c.timeout_ms as u32,
            unattended: c.unattended,
        }
    }
}

/// Tool input for risk assessment
#[napi(object)]
pub struct ToolInput {
    /// Input type: "bash", "file", "json", or "none"
    pub input_type: String,

    /// Command string (for bash type)
    pub command: Option<String>,

    /// File path (for file type)
    pub path: Option<String>,

    /// JSON data (for json type)
    pub json: Option<String>,
}

impl From<ToolInput> for RustToolInput {
    fn from(i: ToolInput) -> Self {
        match i.input_type.as_str() {
            "bash" => RustToolInput::Bash {
                command: i.command.unwrap_or_default(),
            },
            "file" => RustToolInput::File {
                path: i.path.unwrap_or_default(),
            },
            "json" => {
                let json_str = i.json.unwrap_or_else(|| "{}".to_string());
                let value = serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);
                RustToolInput::Json(value)
            }
            _ => RustToolInput::None,
        }
    }
}

/// Approval decision result
#[napi(object)]
pub struct ApprovalDecision {
    /// Whether the operation is approved
    pub approved: bool,

    /// Risk level of the operation
    pub risk: String,

    /// Reason for the decision
    pub reason: String,

    /// Whether this was a timeout-based approval
    pub timeout_approved: bool,

    /// Whether the operation can potentially be auto-approved
    pub auto_approvable: bool,
}

impl From<RustApprovalDecision> for ApprovalDecision {
    fn from(d: RustApprovalDecision) -> Self {
        Self {
            approved: d.approved,
            risk: d.risk.as_str().to_string(),
            reason: d.reason,
            timeout_approved: d.timeout_approved,
            auto_approvable: d.auto_approvable,
        }
    }
}

/// Execution context for adaptive risk assessment
#[napi(object)]
pub struct ExecutionContext {
    /// Session ID
    pub session_id: String,

    /// Current iteration
    pub iteration: u32,

    /// Errors in this session
    pub errors: u32,

    /// Successful operations in session
    pub successes: u32,

    /// Project path
    pub project_path: Option<String>,

    /// Is production environment
    pub is_production: bool,
}

impl From<ExecutionContext> for RustExecutionContext {
    fn from(c: ExecutionContext) -> Self {
        Self {
            session_id: c.session_id,
            iteration: c.iteration,
            errors: c.errors,
            successes: c.successes,
            project_path: c.project_path,
            is_production: c.is_production,
        }
    }
}

/// Adaptive risk assessment result
#[napi(object)]
pub struct AdaptiveRiskResult {
    /// Base risk level
    pub base_risk: String,

    /// Adjusted risk level
    pub adjusted_risk: String,

    /// Adjustment applied
    pub adjustment: i32,

    /// Reason for adjustment
    pub adjustment_reason: String,
}

impl From<RustAdaptiveRiskResult> for AdaptiveRiskResult {
    fn from(r: RustAdaptiveRiskResult) -> Self {
        Self {
            base_risk: r.base_risk.as_str().to_string(),
            adjusted_risk: r.adjusted_risk.as_str().to_string(),
            adjustment: r.adjustment,
            adjustment_reason: r.adjustment_reason,
        }
    }
}

/// Audit entry for auto-approve decisions
#[napi(object)]
pub struct AuditEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,

    /// Permission request ID
    pub permission_id: Option<String>,

    /// Tool name
    pub tool: String,

    /// Associated patterns
    pub pattern: Option<Vec<String>>,

    /// Risk level
    pub risk: String,

    /// Decision made
    pub decision: String,

    /// Reason for decision
    pub reason: String,
}

impl From<RustAuditEntry> for AuditEntry {
    fn from(e: RustAuditEntry) -> Self {
        Self {
            timestamp: e.timestamp,
            permission_id: e.permission_id,
            tool: e.tool,
            pattern: e.pattern,
            risk: e.risk.as_str().to_string(),
            decision: e.decision,
            reason: e.reason,
        }
    }
}

// ============================================================================
// Auto-Approve Engine Handle
// ============================================================================

/// Handle to an auto-approve engine instance
#[napi]
pub struct AutoApproveEngineHandle {
    inner: Mutex<RustAutoApproveEngine>,
}

/// Create a new auto-approve engine with configuration
#[napi]
pub fn create_auto_approve_engine(config: AutoApproveConfig) -> AutoApproveEngineHandle {
    AutoApproveEngineHandle {
        inner: Mutex::new(RustAutoApproveEngine::new(config.into())),
    }
}

/// Create a safe-only auto-approve engine
#[napi]
pub fn create_safe_only_engine(unattended: bool) -> AutoApproveEngineHandle {
    AutoApproveEngineHandle {
        inner: Mutex::new(RustAutoApproveEngine::safe_only(unattended)),
    }
}

/// Create a permissive auto-approve engine
#[napi]
pub fn create_permissive_engine(unattended: bool) -> AutoApproveEngineHandle {
    AutoApproveEngineHandle {
        inner: Mutex::new(RustAutoApproveEngine::permissive(unattended)),
    }
}

#[napi]
impl AutoApproveEngineHandle {
    /// Get the current configuration
    #[napi]
    pub fn config(&self) -> napi::Result<AutoApproveConfig> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.config().into())
    }

    /// Update configuration
    #[napi]
    pub fn set_config(&self, config: AutoApproveConfig) -> napi::Result<()> {
        let mut guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.set_config(config.into());
        Ok(())
    }

    /// Evaluate a tool operation for auto-approval
    #[napi]
    pub fn evaluate(&self, tool: String, input: Option<ToolInput>) -> napi::Result<ApprovalDecision> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let rust_input = input.map(Into::into);
        Ok(guard.evaluate(&tool, rust_input).into())
    }

    /// Evaluate with adaptive risk assessment
    #[napi]
    pub fn evaluate_adaptive(
        &self,
        tool: String,
        input: Option<ToolInput>,
        ctx: ExecutionContext,
    ) -> napi::Result<ApprovalDecision> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let rust_input = input.map(Into::into);
        let rust_ctx: RustExecutionContext = ctx.into();
        Ok(guard.evaluate_adaptive(&tool, rust_input, &rust_ctx).into())
    }

    /// Quick check if a tool can be auto-approved
    #[napi]
    pub fn can_auto_approve(&self, tool: String) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.can_auto_approve(&tool))
    }

    /// Assess risk for a tool operation
    #[napi]
    pub fn assess_risk(&self, tool: String, input: Option<ToolInput>) -> napi::Result<RiskResult> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let rust_input = input.map(Into::into).unwrap_or(RustToolInput::None);
        Ok(guard.assess_risk(&tool, &rust_input).into())
    }

    /// Evaluate adaptive risk
    #[napi]
    pub fn evaluate_adaptive_risk(
        &self,
        tool: String,
        input: Option<ToolInput>,
        ctx: ExecutionContext,
    ) -> napi::Result<AdaptiveRiskResult> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let rust_input = input.map(Into::into).unwrap_or(RustToolInput::None);
        let rust_ctx: RustExecutionContext = ctx.into();
        Ok(guard.evaluate_adaptive_risk(&tool, &rust_input, &rust_ctx).into())
    }

    /// Get the audit log
    #[napi]
    pub fn audit_log(&self) -> napi::Result<Vec<AuditEntry>> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.audit_log().iter().cloned().map(Into::into).collect())
    }

    /// Clear the audit log
    #[napi]
    pub fn clear_audit_log(&self) -> napi::Result<()> {
        let mut guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.clear_audit_log();
        Ok(())
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Evaluate a tool operation for auto-approval (stateless)
///
/// Creates a temporary engine with the given config and evaluates the operation.
#[napi]
pub fn evaluate_auto_approve(
    config: AutoApproveConfig,
    tool: String,
    input: Option<ToolInput>,
) -> ApprovalDecision {
    let engine = RustAutoApproveEngine::new(config.into());
    let rust_input = input.map(Into::into);
    engine.evaluate(&tool, rust_input).into()
}

/// Evaluate with adaptive risk (stateless)
#[napi]
pub fn evaluate_adaptive_auto_approve(
    config: AutoApproveConfig,
    tool: String,
    input: Option<ToolInput>,
    ctx: ExecutionContext,
) -> ApprovalDecision {
    let engine = RustAutoApproveEngine::new(config.into());
    let rust_input = input.map(Into::into);
    let rust_ctx: RustExecutionContext = ctx.into();
    engine.evaluate_adaptive(&tool, rust_input, &rust_ctx).into()
}

/// Quick check if a tool can be auto-approved with safe-only config
#[napi]
pub fn can_safe_auto_approve(tool: String) -> bool {
    let engine = RustAutoApproveEngine::safe_only(false);
    engine.can_auto_approve(&tool)
}

// ============================================================================
// Remote Policy Bindings
// ============================================================================

use crate::security::remote_policy::{
    RemotePolicy as RustRemotePolicy,
    RemoteRiskLevel as RustRemoteRiskLevel,
    RemoteTaskContext as RustRemoteTaskContext,
};

/// Risk level for remote operations
#[napi(string_enum)]
pub enum RemoteRiskLevel {
    Safe,
    Moderate,
    Dangerous,
}

impl From<RustRemoteRiskLevel> for RemoteRiskLevel {
    fn from(r: RustRemoteRiskLevel) -> Self {
        match r {
            RustRemoteRiskLevel::Safe => RemoteRiskLevel::Safe,
            RustRemoteRiskLevel::Moderate => RemoteRiskLevel::Moderate,
            RustRemoteRiskLevel::Dangerous => RemoteRiskLevel::Dangerous,
        }
    }
}

/// Task context for remote policy evaluation
#[napi(object)]
pub struct RemoteTaskContext {
    /// Source of the request: "cli", "remote", "api"
    pub source: String,
    /// User identifier
    pub user_id: String,
    /// Optional session ID
    pub session_id: Option<String>,
}

impl From<RemoteTaskContext> for RustRemoteTaskContext {
    fn from(c: RemoteTaskContext) -> Self {
        Self {
            source: c.source,
            user_id: c.user_id,
            session_id: c.session_id,
        }
    }
}

/// Handle to a remote policy instance
#[napi]
pub struct RemotePolicyHandle {
    inner: Mutex<RustRemotePolicy>,
}

#[napi]
impl RemotePolicyHandle {
    /// Create a new remote policy
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RustRemotePolicy::new()),
        }
    }

    /// Check if an operation requires approval
    #[napi]
    pub fn should_require_approval(&self, tool: String, context: RemoteTaskContext) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let rust_ctx: RustRemoteTaskContext = context.into();
        Ok(guard.should_require_approval(&tool, &rust_ctx))
    }

    /// Get the risk level for an operation
    #[napi]
    pub fn risk_level(&self, tool: String) -> napi::Result<RemoteRiskLevel> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.risk_level(&tool).into())
    }

    /// Check if operation is dangerous
    #[napi]
    pub fn is_dangerous(&self, tool: String) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.is_dangerous(&tool))
    }

    /// Check if operation is safe
    #[napi]
    pub fn is_safe(&self, tool: String) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.is_safe(&tool))
    }

    /// Load allowlists from storage
    #[napi]
    pub fn load_allowlists(&self) -> napi::Result<()> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.load_allowlists().map_err(|e| {
            napi::Error::from_reason(format!("Failed to load allowlists: {}", e))
        })
    }

    /// Allow a tool for a user
    #[napi]
    pub fn allow_for_user(&self, user_id: String, tool: String) -> napi::Result<()> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.allow_for_user(&user_id, &tool).map_err(|e| {
            napi::Error::from_reason(format!("Failed to allow tool: {}", e))
        })
    }

    /// Revoke a tool for a user
    #[napi]
    pub fn revoke_for_user(&self, user_id: String, tool: String) -> napi::Result<()> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.revoke_for_user(&user_id, &tool).map_err(|e| {
            napi::Error::from_reason(format!("Failed to revoke tool: {}", e))
        })
    }

    /// Get a user's allowlist
    #[napi]
    pub fn get_user_allowlist(&self, user_id: String) -> napi::Result<Vec<String>> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(guard.get_user_allowlist(&user_id))
    }

    /// Clear a user's allowlist
    #[napi]
    pub fn clear_user_allowlist(&self, user_id: String) -> napi::Result<()> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        guard.clear_user_allowlist(&user_id).map_err(|e| {
            napi::Error::from_reason(format!("Failed to clear allowlist: {}", e))
        })
    }

    /// Describe why approval is needed
    #[napi]
    pub fn describe_approval_reason(&self, tool: String, args: Option<String>) -> napi::Result<String> {
        let guard = self.inner.lock().map_err(|e| {
            napi::Error::from_reason(format!("Failed to acquire lock: {}", e))
        })?;
        let args_json = args.and_then(|s| serde_json::from_str(&s).ok());
        Ok(guard.describe_approval_reason(&tool, args_json.as_ref()))
    }
}

// ============================================================================
// Remote Policy Convenience Functions
// ============================================================================

/// Get the remote risk level for a tool (stateless)
#[napi]
pub fn get_remote_risk_level(tool: String) -> RemoteRiskLevel {
    let policy = RustRemotePolicy::new();
    policy.risk_level(&tool).into()
}

/// Check if a tool is dangerous for remote access (stateless)
#[napi]
pub fn is_remote_dangerous(tool: String) -> bool {
    let policy = RustRemotePolicy::new();
    policy.is_dangerous(&tool)
}

/// Check if a tool is safe for remote access (stateless)
#[napi]
pub fn is_remote_safe(tool: String) -> bool {
    let policy = RustRemotePolicy::new();
    policy.is_safe(&tool)
}

/// Check if remote approval is required (stateless)
#[napi]
pub fn should_require_remote_approval(tool: String, source: String, user_id: String) -> bool {
    let policy = RustRemotePolicy::new();
    let context = RustRemoteTaskContext {
        source,
        user_id,
        session_id: None,
    };
    policy.should_require_approval(&tool, &context)
}
