//! Guardrails module for Human-In-The-Loop (HITL) confirmation.
//!
//! Inspired by OpenFang's approval gates, this module provides:
//! - Action classification by risk level
//! - Approval workflow for sensitive operations
//! - Audit logging for all decisions
//!
//! # Risk Levels
//!
//! | Level | Description | Actions |
//! |-------|-------------|---------|
//! | Safe | No confirmation needed | Read-only, analysis |
//! | Low | Log only | Code review, suggestions |
//! | Medium | Notify user | Minor code changes, file edits |
//! | High | Require confirmation | External API calls, purchases |
//! | Critical | Block and escalate | Destructive operations, large transactions |
//!
//! # Features
//!
//! This module requires the `guardrails` feature for notification functionality.
//! Without the feature, notifications are disabled but all other functionality works.
//!
//! # Example
//!
//! ```no_run
//! use zero_common::guardrails::{Guardrails, Action, RiskLevel};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let guardrails = Guardrails::new()
//!     .with_autonomy_level("crazy");
//!
//! let action = Action::new("browser", "purchase")
//!     .with_amount(99.99)
//!     .with_description("Buy domain example.com");
//!
//! let decision = guardrails.evaluate(&action).await?;
//!
//! match decision {
//!     Decision::Approved => println!("Proceed"),
//!     Decision::Pending { approval_id, .. } => println!("Waiting for approval: {}", approval_id),
//!     Decision::Denied { reason } => println!("Blocked: {}", reason),
//! }
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
#[cfg(feature = "guardrails")]
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Risk Level Classification
// ============================================================================

/// Risk level for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// No confirmation needed (read-only, analysis)
    Safe,
    /// Log only (code review, suggestions)
    Low,
    /// Notify user (minor code changes, file edits)
    Medium,
    /// Require confirmation (external API calls, purchases)
    High,
    /// Block and escalate (destructive operations, large transactions)
    Critical,
}

impl RiskLevel {
    /// Get the numeric value (0-4) for comparison.
    pub fn value(&self) -> u8 {
        match self {
            RiskLevel::Safe => 0,
            RiskLevel::Low => 1,
            RiskLevel::Medium => 2,
            RiskLevel::High => 3,
            RiskLevel::Critical => 4,
        }
    }

    /// Check if this level requires user confirmation.
    pub fn requires_confirmation(&self, autonomy_threshold: u8) -> bool {
        self.value() > autonomy_threshold
    }
}

impl Default for RiskLevel {
    fn default() -> Self {
        RiskLevel::Medium
    }
}

// ============================================================================
// Action Types
// ============================================================================

/// Category of action being performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCategory {
    /// Browser automation actions
    Browser,
    /// File system operations
    FileSystem,
    /// External API calls
    ExternalApi,
    /// Financial transactions
    Financial,
    /// Code execution
    CodeExecution,
    /// Data deletion
    DataDeletion,
    /// Configuration changes
    ConfigChange,
    /// Other/custom
    Custom(String),
}

impl ActionCategory {
    /// Get the base risk level for this category.
    pub fn base_risk(&self) -> RiskLevel {
        match self {
            ActionCategory::Browser => RiskLevel::Medium,
            ActionCategory::FileSystem => RiskLevel::Medium,
            ActionCategory::ExternalApi => RiskLevel::High,
            ActionCategory::Financial => RiskLevel::Critical,
            ActionCategory::CodeExecution => RiskLevel::High,
            ActionCategory::DataDeletion => RiskLevel::Critical,
            ActionCategory::ConfigChange => RiskLevel::Medium,
            ActionCategory::Custom(_) => RiskLevel::Medium,
        }
    }
}

/// An action to be evaluated by guardrails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action ID
    pub id: String,

    /// Action category
    pub category: ActionCategory,

    /// Specific action type (e.g., "purchase", "delete", "execute")
    pub action_type: String,

    /// Human-readable description
    pub description: String,

    /// Source hand/agent ID
    pub source_id: String,

    /// Optional monetary amount (USD)
    pub amount: Option<f64>,

    /// Optional target (URL, file path, etc.)
    pub target: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Timestamp
    pub created_at: DateTime<Utc>,
}

impl Action {
    /// Create a new action.
    pub fn new(category: impl Into<String>, action_type: impl Into<String>) -> Self {
        let category_str = category.into();
        let category = match category_str.to_lowercase().as_str() {
            "browser" => ActionCategory::Browser,
            "filesystem" | "file_system" => ActionCategory::FileSystem,
            "external_api" | "api" => ActionCategory::ExternalApi,
            "financial" | "finance" => ActionCategory::Financial,
            "code_execution" | "code" => ActionCategory::CodeExecution,
            "data_deletion" | "delete" => ActionCategory::DataDeletion,
            "config_change" | "config" => ActionCategory::ConfigChange,
            other => ActionCategory::Custom(other.to_string()),
        };

        Self {
            id: Uuid::new_v4().to_string(),
            category,
            action_type: action_type.into(),
            description: String::new(),
            source_id: String::new(),
            amount: None,
            target: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the source ID.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_id = source.into();
        self
    }

    /// Set the monetary amount.
    pub fn with_amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the target.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Calculate the risk level for this action.
    pub fn risk_level(&self) -> RiskLevel {
        let base = self.category.base_risk();

        // Elevate risk based on amount
        if let Some(amount) = self.amount {
            if amount >= 1000.0 {
                return RiskLevel::Critical;
            } else if amount >= 100.0 && base.value() < RiskLevel::High.value() {
                return RiskLevel::High;
            }
        }

        // Elevate risk based on action type
        let elevated = match self.action_type.to_lowercase().as_str() {
            "delete" | "remove" | "destroy" => true,
            "purchase" | "buy" | "pay" => true,
            "execute" | "run" | "eval" => true,
            "publish" | "deploy" | "release" => true,
            _ => false,
        };

        if elevated && base.value() < RiskLevel::High.value() {
            return RiskLevel::High;
        }

        base
    }
}

// ============================================================================
// Decision Types
// ============================================================================

/// Decision from guardrails evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Decision {
    /// Action approved, proceed immediately
    Approved,

    /// Action pending approval, wait for user response
    Pending {
        approval_id: String,
        expires_at: DateTime<Utc>,
    },

    /// Action denied
    Denied { reason: String },
}

impl Decision {
    /// Check if the action can proceed.
    pub fn can_proceed(&self) -> bool {
        matches!(self, Decision::Approved)
    }

    /// Check if the action is blocked.
    pub fn is_blocked(&self) -> bool {
        matches!(self, Decision::Denied { .. })
    }

    /// Check if the action is pending approval.
    pub fn is_pending(&self) -> bool {
        matches!(self, Decision::Pending { .. })
    }
}

// ============================================================================
// Approval Request
// ============================================================================

/// An approval request waiting for user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique approval ID
    pub id: String,

    /// The action being approved
    pub action: Action,

    /// Risk level of the action
    pub risk_level: RiskLevel,

    /// When the request was created
    pub created_at: DateTime<Utc>,

    /// When the request expires
    pub expires_at: DateTime<Utc>,

    /// Current status
    pub status: ApprovalStatus,

    /// User who approved/denied (if resolved)
    pub resolved_by: Option<String>,

    /// Resolution timestamp
    pub resolved_at: Option<DateTime<Utc>>,

    /// Optional notes from resolver
    pub notes: Option<String>,
}

/// Status of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

// ============================================================================
// Guardrails Configuration
// ============================================================================

/// Configuration for guardrails behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    /// Autonomy level threshold (0-4)
    /// Actions with risk > threshold require confirmation
    #[serde(default = "default_autonomy_threshold")]
    pub autonomy_threshold: u8,

    /// Approval timeout in seconds
    #[serde(default = "default_approval_timeout")]
    pub approval_timeout_secs: u64,

    /// Whether to notify users via IM channels
    #[serde(default)]
    pub notify_via_channels: bool,

    /// Channels endpoint for notifications
    #[serde(default)]
    pub channels_endpoint: Option<String>,

    /// User ID to notify for approvals
    #[serde(default)]
    pub notify_user_id: Option<String>,

    /// Channel type for notifications (telegram, discord, etc.)
    #[serde(default = "default_channel_type")]
    pub notify_channel_type: String,
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self {
            autonomy_threshold: default_autonomy_threshold(),
            approval_timeout_secs: default_approval_timeout(),
            notify_via_channels: false,
            channels_endpoint: None,
            notify_user_id: None,
            notify_channel_type: default_channel_type(),
        }
    }
}

fn default_autonomy_threshold() -> u8 {
    2 // Medium - matches "crazy" autonomy level
}

fn default_approval_timeout() -> u64 {
    300 // 5 minutes
}

fn default_channel_type() -> String {
    "telegram".to_string()
}

// ============================================================================
// Guardrails Service
// ============================================================================

/// Guardrails service for evaluating actions and managing approvals.
pub struct Guardrails {
    config: GuardrailsConfig,
    pending: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    #[cfg(feature = "guardrails")]
    client: reqwest::Client,
}

impl Guardrails {
    /// Create a new guardrails service with default configuration.
    pub fn new() -> Self {
        Self {
            config: GuardrailsConfig::default(),
            pending: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "guardrails")]
            client: reqwest::Client::new(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: GuardrailsConfig) -> Self {
        Self {
            config,
            pending: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "guardrails")]
            client: reqwest::Client::new(),
        }
    }

    /// Set the autonomy level by name.
    pub fn with_autonomy_level(mut self, level: &str) -> Self {
        self.config.autonomy_threshold = match level.to_lowercase().as_str() {
            "lunatic" => 4, // Allow everything except critical
            "insane" => 3,  // Require confirmation for critical
            "crazy" => 2,   // Require confirmation for high+
            "wild" => 2,
            "bold" => 1,   // Require confirmation for medium+
            "timid" => 0,  // Require confirmation for everything
            _ => 2,
        };
        self
    }

    /// Set the channels endpoint for notifications.
    #[cfg(feature = "guardrails")]
    pub fn with_channels_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.channels_endpoint = Some(endpoint.into());
        self.config.notify_via_channels = true;
        self
    }

    /// Set the channels endpoint for notifications.
    /// (No-op without the `guardrails` feature)
    #[cfg(not(feature = "guardrails"))]
    pub fn with_channels_endpoint(self, _endpoint: impl Into<String>) -> Self {
        tracing::warn!("Notification disabled: compile with `guardrails` feature");
        self
    }

    /// Evaluate an action and return a decision.
    pub async fn evaluate(&self, action: &Action) -> Result<Decision> {
        let risk = action.risk_level();

        tracing::debug!(
            action_id = %action.id,
            action_type = %action.action_type,
            risk_level = ?risk,
            threshold = self.config.autonomy_threshold,
            "Evaluating action"
        );

        // Critical actions are ALWAYS blocked for review (regardless of autonomy level)
        if risk == RiskLevel::Critical {
            tracing::warn!(
                action_id = %action.id,
                "Critical action blocked for review"
            );
            return Ok(Decision::Denied {
                reason: "Critical action requires manual review".to_string(),
            });
        }

        // Check if confirmation is required for non-critical actions
        if !risk.requires_confirmation(self.config.autonomy_threshold) {
            tracing::info!(
                action_id = %action.id,
                "Action approved automatically (risk {} <= threshold {})",
                risk.value(),
                self.config.autonomy_threshold
            );
            return Ok(Decision::Approved);
        }

        // Create approval request
        let expires_at = Utc::now() + chrono::Duration::seconds(self.config.approval_timeout_secs as i64);

        let approval = ApprovalRequest {
            id: Uuid::new_v4().to_string(),
            action: action.clone(),
            risk_level: risk,
            created_at: Utc::now(),
            expires_at,
            status: ApprovalStatus::Pending,
            resolved_by: None,
            resolved_at: None,
            notes: None,
        };

        let approval_id = approval.id.clone();

        // Store pending approval
        {
            let mut pending = self.pending.write().await;
            pending.insert(approval_id.clone(), approval.clone());
        }

        // Send notification if configured (requires `guardrails` feature)
        #[cfg(feature = "guardrails")]
        if self.config.notify_via_channels {
            if let Err(e) = self.send_notification(&approval).await {
                tracing::warn!(error = %e, "Failed to send approval notification");
            }
        }

        tracing::info!(
            action_id = %action.id,
            approval_id = %approval_id,
            expires_at = %expires_at,
            "Action pending approval"
        );

        Ok(Decision::Pending {
            approval_id,
            expires_at,
        })
    }

    /// Approve a pending action.
    pub async fn approve(&self, approval_id: &str, user: &str, notes: Option<String>) -> Result<bool> {
        let mut pending = self.pending.write().await;

        if let Some(approval) = pending.get_mut(approval_id) {
            if approval.status != ApprovalStatus::Pending {
                return Ok(false);
            }

            if Utc::now() > approval.expires_at {
                approval.status = ApprovalStatus::Expired;
                return Ok(false);
            }

            approval.status = ApprovalStatus::Approved;
            approval.resolved_by = Some(user.to_string());
            approval.resolved_at = Some(Utc::now());
            approval.notes = notes;

            tracing::info!(
                approval_id = %approval_id,
                user = %user,
                "Approval granted"
            );

            return Ok(true);
        }

        Ok(false)
    }

    /// Deny a pending action.
    pub async fn deny(&self, approval_id: &str, user: &str, reason: String) -> Result<bool> {
        let mut pending = self.pending.write().await;

        if let Some(approval) = pending.get_mut(approval_id) {
            if approval.status != ApprovalStatus::Pending {
                return Ok(false);
            }

            approval.status = ApprovalStatus::Denied;
            approval.resolved_by = Some(user.to_string());
            approval.resolved_at = Some(Utc::now());
            approval.notes = Some(reason);

            tracing::info!(
                approval_id = %approval_id,
                user = %user,
                "Approval denied"
            );

            return Ok(true);
        }

        Ok(false)
    }

    /// Check the status of an approval.
    pub async fn check_status(&self, approval_id: &str) -> Option<ApprovalStatus> {
        let pending = self.pending.read().await;
        pending.get(approval_id).map(|a| {
            if a.status == ApprovalStatus::Pending && Utc::now() > a.expires_at {
                ApprovalStatus::Expired
            } else {
                a.status
            }
        })
    }

    /// List all pending approvals.
    pub async fn list_pending(&self) -> Vec<ApprovalRequest> {
        let pending = self.pending.read().await;
        pending
            .values()
            .filter(|a| a.status == ApprovalStatus::Pending && Utc::now() <= a.expires_at)
            .cloned()
            .collect()
    }

    /// Send notification via channels service.
    #[cfg(feature = "guardrails")]
    async fn send_notification(&self, approval: &ApprovalRequest) -> Result<()> {
        let endpoint = self.config.channels_endpoint.as_ref()
            .context("Channels endpoint not configured")?;

        let user_id = self.config.notify_user_id.as_ref()
            .context("Notify user ID not configured")?;

        let message = format!(
            "🔔 **Approval Required**\n\n\
            **Action:** {} - {}\n\
            **Risk Level:** {:?}\n\
            **Description:** {}\n\
            **Source:** {}\n\
            {}\n\
            **Expires:** {}\n\n\
            Reply with `/approve {}` or `/deny {}`",
            match &approval.action.category {
                ActionCategory::Browser => "Browser",
                ActionCategory::FileSystem => "File System",
                ActionCategory::ExternalApi => "External API",
                ActionCategory::Financial => "Financial",
                ActionCategory::CodeExecution => "Code Execution",
                ActionCategory::DataDeletion => "Data Deletion",
                ActionCategory::ConfigChange => "Config Change",
                ActionCategory::Custom(s) => s,
            },
            approval.action.action_type,
            approval.risk_level,
            approval.action.description,
            approval.action.source_id,
            approval.action.amount.map(|a| format!("**Amount:** ${:.2}", a)).unwrap_or_default(),
            approval.expires_at.format("%Y-%m-%d %H:%M:%S UTC"),
            approval.id,
            approval.id,
        );

        let url = format!("{}/api/v1/send", endpoint.trim_end_matches('/'));

        let payload = serde_json::json!({
            "channel": self.config.notify_channel_type,
            "user_id": user_id,
            "message": message,
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send notification")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Channels API returned {}: {}", status, body);
        }

        tracing::debug!(
            approval_id = %approval.id,
            "Notification sent"
        );

        Ok(())
    }
}

impl Default for Guardrails {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe.value() < RiskLevel::Low.value());
        assert!(RiskLevel::Low.value() < RiskLevel::Medium.value());
        assert!(RiskLevel::Medium.value() < RiskLevel::High.value());
        assert!(RiskLevel::High.value() < RiskLevel::Critical.value());
    }

    #[test]
    fn test_action_risk_calculation() {
        // Browser action is medium risk
        let action = Action::new("browser", "click");
        assert_eq!(action.risk_level(), RiskLevel::Medium);

        // Purchase action elevates to high
        let action = Action::new("browser", "purchase");
        assert_eq!(action.risk_level(), RiskLevel::High);

        // Large amount elevates to critical
        let action = Action::new("browser", "purchase").with_amount(1500.0);
        assert_eq!(action.risk_level(), RiskLevel::Critical);

        // Financial actions are always critical
        let action = Action::new("financial", "transfer");
        assert_eq!(action.risk_level(), RiskLevel::Critical);
    }

    #[test]
    fn test_autonomy_thresholds() {
        // Safe actions never require confirmation
        assert!(!RiskLevel::Safe.requires_confirmation(0));
        assert!(!RiskLevel::Safe.requires_confirmation(4));

        // Timid (threshold 0) requires confirmation for everything except safe
        assert!(RiskLevel::Low.requires_confirmation(0));
        assert!(RiskLevel::Medium.requires_confirmation(0));

        // Crazy (threshold 2) allows low and medium
        assert!(!RiskLevel::Low.requires_confirmation(2));
        assert!(!RiskLevel::Medium.requires_confirmation(2));
        assert!(RiskLevel::High.requires_confirmation(2));
        assert!(RiskLevel::Critical.requires_confirmation(2));

        // Lunatic (threshold 4) - High is allowed
        assert!(!RiskLevel::High.requires_confirmation(4));
        // Note: Critical at threshold 4 returns false from requires_confirmation,
        // but the evaluate() function handles Critical separately by always blocking
    }

    #[tokio::test]
    async fn test_guardrails_auto_approve() {
        let guardrails = Guardrails::new().with_autonomy_level("crazy");

        // Safe action should auto-approve
        let action = Action::new("browser", "scroll")
            .with_description("Scroll page down");

        let decision = guardrails.evaluate(&action).await.unwrap();
        assert!(decision.can_proceed());
    }

    #[tokio::test]
    async fn test_guardrails_block_critical() {
        let guardrails = Guardrails::new().with_autonomy_level("lunatic");

        // Critical action should be blocked even for lunatic level
        let action = Action::new("financial", "transfer")
            .with_amount(10000.0)
            .with_description("Transfer funds");

        let decision = guardrails.evaluate(&action).await.unwrap();
        assert!(decision.is_blocked());
    }

    #[tokio::test]
    async fn test_guardrails_pending() {
        let guardrails = Guardrails::new().with_autonomy_level("timid");

        // Medium risk action should be pending for timid level
        let action = Action::new("browser", "click")
            .with_description("Click button");

        let decision = guardrails.evaluate(&action).await.unwrap();
        assert!(decision.is_pending());
    }
}
