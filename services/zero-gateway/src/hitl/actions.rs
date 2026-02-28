//! Approval actions and callbacks.
//!
//! This module handles the execution of approved operations
//! and notification callbacks to IM channels.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{ApprovalRequest, ApprovalType, RiskLevel};

/// Result of executing an approved action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded.
    pub success: bool,
    /// Human-readable message describing the result.
    pub message: String,
    /// Additional data returned by the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ActionResult {
    /// Create a successful action result.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    /// Create a successful action result with data.
    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create a failed action result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

/// Trait for handling approved actions.
///
/// Implementations of this trait are responsible for executing
/// the actual operation after approval has been granted.
#[async_trait]
pub trait ApprovalAction: Send + Sync {
    /// Execute the approved action.
    ///
    /// This method is called after the approval request has been
    /// approved by an authorized user.
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult>;

    /// Rollback a failed action (best effort).
    ///
    /// This method is called if the action execution fails and
    /// a rollback is possible. Implementations should make a
    /// best-effort attempt to undo any partial changes.
    async fn rollback(&self, request: &ApprovalRequest) -> Result<()>;
}

/// Registry for action handlers.
///
/// Maps approval types to their corresponding action handlers.
pub struct ActionRegistry {
    handlers: HashMap<String, Arc<dyn ApprovalAction>>,
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRegistry {
    /// Create a new empty action registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for an approval type.
    pub fn register(&mut self, approval_type: &str, handler: Arc<dyn ApprovalAction>) {
        self.handlers.insert(approval_type.to_string(), handler);
    }

    /// Get the handler for an approval type.
    pub fn get(&self, approval_type: &str) -> Option<Arc<dyn ApprovalAction>> {
        self.handlers.get(approval_type).cloned()
    }

    /// Execute the appropriate action for an approval request.
    ///
    /// Looks up the handler based on the approval type and executes it.
    /// Returns an error if no handler is registered for the type.
    pub async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        let type_name = request.approval_type.type_name();

        let handler = self
            .get(type_name)
            .ok_or_else(|| anyhow!("No handler registered for approval type: {}", type_name))?;

        handler.execute(request).await
    }

    /// Create a registry with all default handlers registered.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register("merge_request", Arc::new(MergeRequestAction::new()));
        registry.register("trading_command", Arc::new(TradingCommandAction::new()));
        registry.register("config_change", Arc::new(ConfigChangeAction::new()));
        registry.register(
            "high_cost_operation",
            Arc::new(HighCostOperationAction::new()),
        );
        registry.register("risk_operation", Arc::new(RiskOperationAction::new()));
        registry.register("tool_execution", Arc::new(ToolExecutionAction::new()));

        registry
    }
}

/// Action handler for merge request approvals.
///
/// In a full implementation, this would integrate with GitHub/GitLab APIs.
pub struct MergeRequestAction {
    // In a real implementation, this would hold API clients
    // github_client: Option<GitHubClient>,
    // gitlab_client: Option<GitLabClient>,
}

impl MergeRequestAction {
    /// Create a new merge request action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MergeRequestAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for MergeRequestAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::MergeRequest {
            platform,
            repo,
            mr_id,
        } = &request.approval_type
        {
            info!(
                platform = %platform,
                repo = %repo,
                mr_id = %mr_id,
                "Executing merge request action"
            );

            // In a real implementation:
            // 1. Look up the appropriate API client based on platform
            // 2. Call the merge API endpoint
            // 3. Handle merge conflicts, CI failures, etc.

            Ok(ActionResult::success_with_data(
                format!("Would merge {} #{} on {}", repo, mr_id, platform),
                serde_json::json!({
                    "platform": platform,
                    "repo": repo,
                    "mr_id": mr_id
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for MergeRequestAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::MergeRequest {
            platform,
            repo,
            mr_id,
        } = &request.approval_type
        {
            info!(
                platform = %platform,
                repo = %repo,
                mr_id = %mr_id,
                "Rollback requested for merge request (not implemented)"
            );

            // In a real implementation:
            // 1. Revert the merge commit
            // 2. Re-open the merge request
        }

        Ok(())
    }
}

/// Action handler for trading command approvals.
pub struct TradingCommandAction {
    // In a real implementation:
    // exchange_clients: HashMap<String, ExchangeClient>,
}

impl TradingCommandAction {
    /// Create a new trading command action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TradingCommandAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for TradingCommandAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::TradingCommand {
            asset,
            action,
            amount,
        } = &request.approval_type
        {
            info!(
                asset = %asset,
                action = %action,
                amount = %amount,
                "Executing trading command action"
            );

            // In a real implementation:
            // 1. Validate the trading command
            // 2. Connect to the exchange
            // 3. Execute the trade
            // 4. Return order confirmation

            Ok(ActionResult::success_with_data(
                format!("Would {} {} {}", action, amount, asset),
                serde_json::json!({
                    "asset": asset,
                    "action": action,
                    "amount": amount,
                    "order_id": format!("simulated-{}", uuid::Uuid::new_v4())
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for TradingCommandAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::TradingCommand {
            asset,
            action,
            amount,
        } = &request.approval_type
        {
            info!(
                asset = %asset,
                action = %action,
                amount = %amount,
                "Rollback requested for trading command (would execute reverse trade)"
            );

            // In a real implementation:
            // 1. Execute the opposite trade (buy -> sell, sell -> buy)
            // 2. This may not fully recover the original position due to price changes
        }

        Ok(())
    }
}

/// Action handler for configuration change approvals.
pub struct ConfigChangeAction {
    // In a real implementation:
    // config_store: ConfigStore,
}

impl ConfigChangeAction {
    /// Create a new config change action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ConfigChangeAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for ConfigChangeAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::ConfigChange {
            key,
            old_value,
            new_value,
        } = &request.approval_type
        {
            info!(
                key = %key,
                old_value = %old_value,
                new_value = %new_value,
                "Executing config change action"
            );

            // In a real implementation:
            // 1. Validate the new value
            // 2. Update the configuration
            // 3. Notify dependent services
            // 4. Store the change in audit log

            Ok(ActionResult::success_with_data(
                format!("Would change {} from '{}' to '{}'", key, old_value, new_value),
                serde_json::json!({
                    "key": key,
                    "old_value": old_value,
                    "new_value": new_value
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for ConfigChangeAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::ConfigChange {
            key,
            old_value,
            new_value,
        } = &request.approval_type
        {
            info!(
                key = %key,
                old_value = %old_value,
                new_value = %new_value,
                "Rollback requested for config change (would revert to old value)"
            );

            // In a real implementation:
            // 1. Set the config key back to old_value
            // 2. Notify dependent services
        }

        Ok(())
    }
}

/// Action handler for high-cost operation approvals.
pub struct HighCostOperationAction {
    // In a real implementation:
    // cost_tracker: CostTracker,
}

impl HighCostOperationAction {
    /// Create a new high-cost operation action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for HighCostOperationAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for HighCostOperationAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::HighCostOperation {
            operation,
            estimated_cost,
        } = &request.approval_type
        {
            info!(
                operation = %operation,
                estimated_cost = %estimated_cost,
                "Executing high-cost operation action"
            );

            // In a real implementation:
            // 1. Verify budget availability
            // 2. Execute the operation
            // 3. Track actual costs
            // 4. Update billing

            Ok(ActionResult::success_with_data(
                format!(
                    "Would execute '{}' with estimated cost ${:.2}",
                    operation, estimated_cost
                ),
                serde_json::json!({
                    "operation": operation,
                    "estimated_cost": estimated_cost,
                    "execution_id": format!("exec-{}", uuid::Uuid::new_v4())
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for HighCostOperationAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::HighCostOperation {
            operation,
            estimated_cost,
        } = &request.approval_type
        {
            info!(
                operation = %operation,
                estimated_cost = %estimated_cost,
                "Rollback requested for high-cost operation (best effort)"
            );

            // In a real implementation:
            // 1. Attempt to cancel/undo the operation
            // 2. This may not be possible for all operations
        }

        Ok(())
    }
}

/// Action handler for risk operation approvals.
pub struct RiskOperationAction {
    // In a real implementation:
    // risk_assessor: RiskAssessor,
}

impl RiskOperationAction {
    /// Create a new risk operation action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RiskOperationAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for RiskOperationAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::RiskOperation {
            description,
            risk_level,
        } = &request.approval_type
        {
            info!(
                description = %description,
                risk_level = %risk_level.name(),
                "Executing risk operation action"
            );

            // In a real implementation:
            // 1. Create audit trail
            // 2. Execute with enhanced monitoring
            // 3. Alert security team if critical

            let warning = match risk_level {
                RiskLevel::Critical => " [CRITICAL - Full audit trail created]",
                RiskLevel::High => " [HIGH RISK - Enhanced monitoring active]",
                _ => "",
            };

            Ok(ActionResult::success_with_data(
                format!(
                    "Would execute risk operation: {}{}",
                    description, warning
                ),
                serde_json::json!({
                    "description": description,
                    "risk_level": risk_level.name(),
                    "risk_value": risk_level.value()
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for RiskOperationAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::RiskOperation {
            description,
            risk_level,
        } = &request.approval_type
        {
            info!(
                description = %description,
                risk_level = %risk_level.name(),
                "Rollback requested for risk operation"
            );

            // In a real implementation:
            // 1. Attempt to undo the operation
            // 2. Log the rollback attempt
            // 3. Alert if rollback fails
        }

        Ok(())
    }
}

/// Action handler for tool execution approvals (Hands autonomous execution).
///
/// This handler is a no-op as the actual tool execution is performed
/// by the Hands executor after approval is granted. The approval just
/// signals that the executor can proceed.
pub struct ToolExecutionAction {
    // No state needed - just signals approval
}

impl ToolExecutionAction {
    /// Create a new tool execution action handler.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ToolExecutionAction {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApprovalAction for ToolExecutionAction {
    async fn execute(&self, request: &ApprovalRequest) -> Result<ActionResult> {
        if let ApprovalType::ToolExecution {
            tool,
            args,
            risk_level,
            hand_id,
            execution_id,
        } = &request.approval_type
        {
            info!(
                tool = %tool,
                hand_id = %hand_id,
                execution_id = %execution_id,
                risk_level = %risk_level.name(),
                "Tool execution approved"
            );

            // The actual tool execution is performed by the Hands executor.
            // This handler just returns success to indicate approval was granted.
            // The executor polls for approval status and proceeds when approved.

            Ok(ActionResult::success_with_data(
                format!(
                    "Tool '{}' execution approved for hand '{}' (risk: {})",
                    tool, hand_id, risk_level.name()
                ),
                serde_json::json!({
                    "tool": tool,
                    "args": args,
                    "hand_id": hand_id,
                    "execution_id": execution_id,
                    "risk_level": risk_level.name(),
                    "approved": true
                }),
            ))
        } else {
            Err(anyhow!("Invalid approval type for ToolExecutionAction"))
        }
    }

    async fn rollback(&self, request: &ApprovalRequest) -> Result<()> {
        if let ApprovalType::ToolExecution {
            tool,
            hand_id,
            execution_id,
            ..
        } = &request.approval_type
        {
            info!(
                tool = %tool,
                hand_id = %hand_id,
                execution_id = %execution_id,
                "Rollback requested for tool execution (not supported)"
            );

            // Tool execution rollback is not directly supported.
            // The tool would need to implement its own undo logic.
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_merge_request() -> ApprovalRequest {
        ApprovalRequest {
            id: "mr-1".to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "org/repo".to_string(),
                mr_id: 42,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "developer".to_string(),
            approvers: vec!["reviewer".to_string()],
            title: "Add new feature".to_string(),
            description: Some("Implements the new feature".to_string()),
            channel: "slack".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    fn create_trading_command() -> ApprovalRequest {
        ApprovalRequest {
            id: "trade-1".to_string(),
            approval_type: ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 1.5,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "trader".to_string(),
            approvers: vec!["risk-manager".to_string()],
            title: "Buy 1.5 BTC".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    fn create_config_change() -> ApprovalRequest {
        ApprovalRequest {
            id: "config-1".to_string(),
            approval_type: ApprovalType::ConfigChange {
                key: "max_tokens".to_string(),
                old_value: "1000".to_string(),
                new_value: "2000".to_string(),
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "ops".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Increase max tokens".to_string(),
            description: None,
            channel: "slack".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    fn create_high_cost_operation() -> ApprovalRequest {
        ApprovalRequest {
            id: "cost-1".to_string(),
            approval_type: ApprovalType::HighCostOperation {
                operation: "deploy_cluster".to_string(),
                estimated_cost: 1500.0,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "devops".to_string(),
            approvers: vec!["finance".to_string()],
            title: "Deploy new cluster".to_string(),
            description: None,
            channel: "slack".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    fn create_risk_operation(risk_level: RiskLevel) -> ApprovalRequest {
        ApprovalRequest {
            id: "risk-1".to_string(),
            approval_type: ApprovalType::RiskOperation {
                description: "Delete production data".to_string(),
                risk_level,
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "admin".to_string(),
            approvers: vec!["security".to_string()],
            title: "Critical data operation".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    fn create_tool_execution(risk_level: RiskLevel) -> ApprovalRequest {
        ApprovalRequest {
            id: "tool-1".to_string(),
            approval_type: ApprovalType::ToolExecution {
                tool: "Bash".to_string(),
                args: serde_json::json!({"command": "rm -rf /tmp/test"}),
                risk_level,
                hand_id: "test-hand".to_string(),
                execution_id: "exec-123".to_string(),
            },
            status: super::super::ApprovalStatus::Pending,
            requester: "hands-executor".to_string(),
            approvers: vec!["admin".to_string()],
            title: "Execute Bash command".to_string(),
            description: Some("Executing Bash: rm -rf /tmp/test".to_string()),
            channel: "tui".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    // ========== ActionResult tests ==========

    #[test]
    fn test_action_result_success() {
        let result = ActionResult::success("Operation completed");
        assert!(result.success);
        assert_eq!(result.message, "Operation completed");
        assert!(result.data.is_none());
    }

    #[test]
    fn test_action_result_success_with_data() {
        let data = serde_json::json!({"key": "value"});
        let result = ActionResult::success_with_data("Operation completed", data.clone());
        assert!(result.success);
        assert_eq!(result.message, "Operation completed");
        assert_eq!(result.data, Some(data));
    }

    #[test]
    fn test_action_result_failure() {
        let result = ActionResult::failure("Operation failed");
        assert!(!result.success);
        assert_eq!(result.message, "Operation failed");
        assert!(result.data.is_none());
    }

    #[test]
    fn test_action_result_serialization() {
        let result = ActionResult::success_with_data(
            "Test",
            serde_json::json!({"id": 123}),
        );

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Test\""));
        assert!(json.contains("\"id\":123"));

        let deserialized: ActionResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.message, "Test");
    }

    // ========== ActionRegistry tests ==========

    #[test]
    fn test_registry_new() {
        let registry = ActionRegistry::new();
        assert!(registry.get("merge_request").is_none());
    }

    #[test]
    fn test_registry_default() {
        let registry = ActionRegistry::default();
        assert!(registry.get("merge_request").is_none());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ActionRegistry::new();
        let handler = Arc::new(MergeRequestAction::new());

        registry.register("merge_request", handler.clone());

        assert!(registry.get("merge_request").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_with_defaults() {
        let registry = ActionRegistry::with_defaults();

        assert!(registry.get("merge_request").is_some());
        assert!(registry.get("trading_command").is_some());
        assert!(registry.get("config_change").is_some());
        assert!(registry.get("high_cost_operation").is_some());
        assert!(registry.get("risk_operation").is_some());
        assert!(registry.get("unknown_type").is_none());
    }

    #[tokio::test]
    async fn test_registry_execute_success() {
        let registry = ActionRegistry::with_defaults();
        let request = create_merge_request();

        let result = registry.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("github"));
        assert!(result.message.contains("org/repo"));
    }

    #[tokio::test]
    async fn test_registry_execute_no_handler() {
        let registry = ActionRegistry::new(); // Empty registry
        let request = create_merge_request();

        let result = registry.execute(&request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No handler"));
    }

    // ========== MergeRequestAction tests ==========

    #[tokio::test]
    async fn test_merge_request_action_execute() {
        let action = MergeRequestAction::new();
        let request = create_merge_request();

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("org/repo"));
        assert!(result.message.contains("#42"));
        assert!(result.message.contains("github"));

        let data = result.data.unwrap();
        assert_eq!(data["platform"], "github");
        assert_eq!(data["repo"], "org/repo");
        assert_eq!(data["mr_id"], 42);
    }

    #[tokio::test]
    async fn test_merge_request_action_wrong_type() {
        let action = MergeRequestAction::new();
        let request = create_trading_command(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid approval type"));
    }

    #[tokio::test]
    async fn test_merge_request_action_rollback() {
        let action = MergeRequestAction::new();
        let request = create_merge_request();

        // Rollback should succeed (no-op for now)
        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== TradingCommandAction tests ==========

    #[tokio::test]
    async fn test_trading_command_action_execute() {
        let action = TradingCommandAction::new();
        let request = create_trading_command();

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("buy"));
        assert!(result.message.contains("1.5"));
        assert!(result.message.contains("BTC"));

        let data = result.data.unwrap();
        assert_eq!(data["asset"], "BTC");
        assert_eq!(data["action"], "buy");
        assert_eq!(data["amount"], 1.5);
        assert!(data["order_id"].as_str().unwrap().starts_with("simulated-"));
    }

    #[tokio::test]
    async fn test_trading_command_action_wrong_type() {
        let action = TradingCommandAction::new();
        let request = create_merge_request(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_trading_command_action_rollback() {
        let action = TradingCommandAction::new();
        let request = create_trading_command();

        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== ConfigChangeAction tests ==========

    #[tokio::test]
    async fn test_config_change_action_execute() {
        let action = ConfigChangeAction::new();
        let request = create_config_change();

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("max_tokens"));
        assert!(result.message.contains("1000"));
        assert!(result.message.contains("2000"));

        let data = result.data.unwrap();
        assert_eq!(data["key"], "max_tokens");
        assert_eq!(data["old_value"], "1000");
        assert_eq!(data["new_value"], "2000");
    }

    #[tokio::test]
    async fn test_config_change_action_wrong_type() {
        let action = ConfigChangeAction::new();
        let request = create_trading_command(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_change_action_rollback() {
        let action = ConfigChangeAction::new();
        let request = create_config_change();

        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== HighCostOperationAction tests ==========

    #[tokio::test]
    async fn test_high_cost_operation_action_execute() {
        let action = HighCostOperationAction::new();
        let request = create_high_cost_operation();

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("deploy_cluster"));
        assert!(result.message.contains("1500"));

        let data = result.data.unwrap();
        assert_eq!(data["operation"], "deploy_cluster");
        assert_eq!(data["estimated_cost"], 1500.0);
        assert!(data["execution_id"].as_str().unwrap().starts_with("exec-"));
    }

    #[tokio::test]
    async fn test_high_cost_operation_action_wrong_type() {
        let action = HighCostOperationAction::new();
        let request = create_merge_request(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_high_cost_operation_action_rollback() {
        let action = HighCostOperationAction::new();
        let request = create_high_cost_operation();

        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== RiskOperationAction tests ==========

    #[tokio::test]
    async fn test_risk_operation_action_execute_critical() {
        let action = RiskOperationAction::new();
        let request = create_risk_operation(RiskLevel::Critical);

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("Delete production data"));
        assert!(result.message.contains("CRITICAL"));

        let data = result.data.unwrap();
        assert_eq!(data["description"], "Delete production data");
        assert_eq!(data["risk_level"], "Critical");
        assert_eq!(data["risk_value"], 4);
    }

    #[tokio::test]
    async fn test_risk_operation_action_execute_high() {
        let action = RiskOperationAction::new();
        let request = create_risk_operation(RiskLevel::High);

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("HIGH RISK"));

        let data = result.data.unwrap();
        assert_eq!(data["risk_level"], "High");
        assert_eq!(data["risk_value"], 3);
    }

    #[tokio::test]
    async fn test_risk_operation_action_execute_low() {
        let action = RiskOperationAction::new();
        let request = create_risk_operation(RiskLevel::Low);

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        // Low risk should not have warnings
        assert!(!result.message.contains("CRITICAL"));
        assert!(!result.message.contains("HIGH RISK"));

        let data = result.data.unwrap();
        assert_eq!(data["risk_level"], "Low");
        assert_eq!(data["risk_value"], 1);
    }

    #[tokio::test]
    async fn test_risk_operation_action_wrong_type() {
        let action = RiskOperationAction::new();
        let request = create_merge_request(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_risk_operation_action_rollback() {
        let action = RiskOperationAction::new();
        let request = create_risk_operation(RiskLevel::Critical);

        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== ToolExecutionAction tests ==========

    #[tokio::test]
    async fn test_tool_execution_action_execute() {
        let action = ToolExecutionAction::new();
        let request = create_tool_execution(RiskLevel::High);

        let result = action.execute(&request).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("Bash"));
        assert!(result.message.contains("test-hand"));
        assert!(result.message.contains("High"));

        let data = result.data.unwrap();
        assert_eq!(data["tool"], "Bash");
        assert_eq!(data["hand_id"], "test-hand");
        assert_eq!(data["execution_id"], "exec-123");
        assert_eq!(data["risk_level"], "High");
        assert_eq!(data["approved"], true);
    }

    #[tokio::test]
    async fn test_tool_execution_action_wrong_type() {
        let action = ToolExecutionAction::new();
        let request = create_merge_request(); // Wrong type

        let result = action.execute(&request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid approval type"));
    }

    #[tokio::test]
    async fn test_tool_execution_action_rollback() {
        let action = ToolExecutionAction::new();
        let request = create_tool_execution(RiskLevel::Critical);

        // Rollback should succeed (no-op)
        let result = action.rollback(&request).await;
        assert!(result.is_ok());
    }

    // ========== Integration tests ==========

    #[tokio::test]
    async fn test_registry_execute_all_types() {
        let registry = ActionRegistry::with_defaults();

        // Test all approval types
        let requests = vec![
            create_merge_request(),
            create_trading_command(),
            create_config_change(),
            create_high_cost_operation(),
            create_risk_operation(RiskLevel::Medium),
            create_tool_execution(RiskLevel::High),
        ];

        for request in requests {
            let result = registry.execute(&request).await;
            assert!(result.is_ok(), "Failed for type: {}", request.approval_type.type_name());
            assert!(result.unwrap().success);
        }
    }
}
