//! Security module - vault, sandbox, permissions, secrets, injection detection, and risk assessment
//!
//! This module provides:
//! - **vault**: Encrypted credential storage
//! - **keyring**: System keyring integration (macOS Keychain, Linux Secret Service, Windows Credential Manager)
//! - **permission**: Permission management and RBAC
//! - **sandbox**: Sandboxed execution environment
//! - **secrets**: Secret detection and management
//! - **injection**: Prompt injection detection and sanitization
//! - **risk**: Risk assessment for tool operations (auto-approval support)
//! - **auto_approve**: Auto-approval engine for autonomous mode

pub mod permission;
pub mod vault;
pub mod keyring;
pub mod injection;
pub mod risk;
pub mod auto_approve;

// Sandbox will be added later
// pub mod sandbox;

// Re-export main types
pub use permission::{Permission, PermissionManager, PermissionRule};
pub use vault::{SecretEntry, Vault, VaultConfig};
pub use keyring::{
    Credential, CredentialManager, KeyringBackend, KeyringManager,
    McpAuthEntry, McpAuthStore,
};
pub use injection::{
    InjectionPattern, InjectionScanResult, InjectionScanner, InjectionSeverity, InjectionType,
    ScannerConfig, quick_check_injection, sanitize_input, scan_for_injection,
};
pub use risk::{
    RiskLevel, RiskAssessment, assess_bash_risk, assess_file_risk,
    risk_at_or_below_threshold, tool_base_risk,
};
pub use auto_approve::{
    AutoApproveConfig, AutoApproveEngine, ApprovalDecision, ToolInput,
    AuditEntry, ExecutionContext, ProjectSensitivity, TimeOfDay, AdaptiveRiskResult,
};
