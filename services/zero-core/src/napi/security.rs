//! NAPI bindings for security - prompt injection scanner and risk assessment
//!
//! This module exposes the prompt injection scanner and risk assessment to Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

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
