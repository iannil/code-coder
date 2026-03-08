//! Action executor for approved HitL requests.
//!
//! This module executes operations after they have been approved through
//! the Human-in-the-Loop system.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::{ApprovalRequest, ApprovalType};

/// Executor for approved HitL operations.
///
/// After a user approves or rejects a request, the ActionExecutor
/// carries out the corresponding action or cleanup.
pub struct ActionExecutor {
    /// HTTP client for making requests
    client: reqwest::Client,
    /// Base URL for the zero-channels service (for notifications)
    #[allow(dead_code)]
    channels_endpoint: String,
    /// Base URL for the zero-trading service
    trading_endpoint: String,
    /// Base URL for the hands/workflow service
    workflow_endpoint: String,
}

impl ActionExecutor {
    /// Create a new ActionExecutor.
    ///
    /// # Arguments
    /// * `channels_endpoint` - Base URL for channels service (for notifications)
    /// * `trading_endpoint` - Base URL for trading service (for trade execution)
    /// * `workflow_endpoint` - Base URL for workflow/hands service (for tool execution)
    pub fn new(
        channels_endpoint: impl Into<String>,
        trading_endpoint: impl Into<String>,
        workflow_endpoint: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            channels_endpoint: channels_endpoint.into(),
            trading_endpoint: trading_endpoint.into(),
            workflow_endpoint: workflow_endpoint.into(),
        }
    }

    /// Create with a custom HTTP client.
    pub fn with_client(
        client: reqwest::Client,
        channels_endpoint: impl Into<String>,
        trading_endpoint: impl Into<String>,
        workflow_endpoint: impl Into<String>,
    ) -> Self {
        Self {
            client,
            channels_endpoint: channels_endpoint.into(),
            trading_endpoint: trading_endpoint.into(),
            workflow_endpoint: workflow_endpoint.into(),
        }
    }

    /// Execute the action based on approval decision.
    ///
    /// # Arguments
    /// * `request` - The approval request that was decided
    /// * `approved` - Whether the request was approved (true) or rejected (false)
    /// * `decided_by` - User who made the decision
    ///
    /// # Returns
    /// * `Ok(())` on successful execution
    /// * `Err` if execution fails
    pub async fn execute(
        &self,
        request: &ApprovalRequest,
        approved: bool,
        decided_by: &str,
    ) -> Result<()> {
        info!(
            request_id = %request.id,
            approval_type = %request.approval_type.type_name(),
            approved = approved,
            decided_by = decided_by,
            "Executing HitL action"
        );

        if !approved {
            // For rejected requests, we only need to notify and cleanup
            return self.handle_rejection(request, decided_by).await;
        }

        // Dispatch to specific handler based on approval type
        match &request.approval_type {
            ApprovalType::MergeRequest { platform, repo, mr_id } => {
                self.execute_merge_request(platform, repo, *mr_id).await
            }
            ApprovalType::TradingCommand { asset, action, amount } => {
                self.execute_trading_command(asset, action, *amount).await
            }
            ApprovalType::ConfigChange { key, old_value: _, new_value } => {
                self.execute_config_change(key, new_value).await
            }
            ApprovalType::HighCostOperation { operation, estimated_cost } => {
                self.execute_high_cost_operation(operation, *estimated_cost).await
            }
            ApprovalType::RiskOperation { description, risk_level } => {
                self.execute_risk_operation(description, *risk_level).await
            }
            ApprovalType::ToolExecution {
                tool,
                args,
                risk_level: _,
                hand_id,
                execution_id,
            } => {
                self.execute_tool(tool, args, hand_id, execution_id).await
            }
        }
    }

    /// Handle a rejected request by notifying relevant parties.
    async fn handle_rejection(&self, request: &ApprovalRequest, decided_by: &str) -> Result<()> {
        info!(
            request_id = %request.id,
            decided_by = decided_by,
            "Request rejected, sending notification"
        );

        // For tool executions, notify the Hand that its request was rejected
        if let ApprovalType::ToolExecution { hand_id, execution_id, .. } = &request.approval_type {
            let url = format!("{}/hands/{}/executions/{}/reject", self.workflow_endpoint, hand_id, execution_id);
            let payload = serde_json::json!({
                "rejected_by": decided_by,
                "request_id": request.id
            });

            let response = self.client.post(&url).json(&payload).send().await;
            if let Err(e) = response {
                warn!(
                    hand_id = %hand_id,
                    execution_id = %execution_id,
                    error = %e,
                    "Failed to notify Hand of rejection"
                );
            }
        }

        Ok(())
    }

    /// Execute a merge request approval (approve/merge on Git platform).
    async fn execute_merge_request(
        &self,
        platform: &str,
        repo: &str,
        mr_id: i64,
    ) -> Result<()> {
        info!(
            platform = %platform,
            repo = %repo,
            mr_id = mr_id,
            "Executing merge request approval"
        );

        let url = format!("{}/git/merge", self.workflow_endpoint);
        let payload = MergeRequestPayload {
            platform: platform.to_string(),
            repo: repo.to_string(),
            mr_id,
            action: "approve".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute merge request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Merge request execution failed ({}): {}",
                status,
                body
            ));
        }

        info!(
            platform = %platform,
            repo = %repo,
            mr_id = mr_id,
            "Merge request approved successfully"
        );
        Ok(())
    }

    /// Execute a trading command.
    async fn execute_trading_command(
        &self,
        asset: &str,
        action: &str,
        amount: f64,
    ) -> Result<()> {
        info!(
            asset = %asset,
            action = %action,
            amount = %amount,
            "Executing trading command"
        );

        let url = format!("{}/trade/execute", self.trading_endpoint);
        let payload = TradeCommandPayload {
            asset: asset.to_string(),
            action: action.to_string(),
            amount,
            approved: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute trading command: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Trading command execution failed ({}): {}",
                status,
                body
            ));
        }

        let result: TradeResult = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse trade result: {}", e))?;

        info!(
            asset = %asset,
            action = %action,
            amount = %amount,
            order_id = %result.order_id.unwrap_or_default(),
            "Trading command executed successfully"
        );
        Ok(())
    }

    /// Execute a configuration change.
    async fn execute_config_change(&self, key: &str, new_value: &str) -> Result<()> {
        info!(
            key = %key,
            new_value = %new_value,
            "Executing configuration change"
        );

        let url = format!("{}/config/apply", self.workflow_endpoint);
        let payload = ConfigChangePayload {
            key: key.to_string(),
            value: new_value.to_string(),
            approved: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to apply config change: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Config change execution failed ({}): {}",
                status,
                body
            ));
        }

        info!(key = %key, "Configuration change applied successfully");
        Ok(())
    }

    /// Execute a high-cost operation.
    async fn execute_high_cost_operation(
        &self,
        operation: &str,
        estimated_cost: f64,
    ) -> Result<()> {
        info!(
            operation = %operation,
            estimated_cost = %estimated_cost,
            "Executing high-cost operation"
        );

        let url = format!("{}/operations/execute", self.workflow_endpoint);
        let payload = HighCostOperationPayload {
            operation: operation.to_string(),
            estimated_cost,
            approved: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute high-cost operation: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "High-cost operation execution failed ({}): {}",
                status,
                body
            ));
        }

        info!(operation = %operation, "High-cost operation executed successfully");
        Ok(())
    }

    /// Execute a risk operation.
    async fn execute_risk_operation(
        &self,
        description: &str,
        risk_level: super::RiskLevel,
    ) -> Result<()> {
        info!(
            description = %description,
            risk_level = ?risk_level,
            "Executing risk operation"
        );

        let url = format!("{}/operations/risk-execute", self.workflow_endpoint);
        let payload = RiskOperationPayload {
            description: description.to_string(),
            risk_level: risk_level.name().to_string(),
            approved: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute risk operation: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Risk operation execution failed ({}): {}",
                status,
                body
            ));
        }

        info!(description = %description, "Risk operation executed successfully");
        Ok(())
    }

    /// Execute a tool for Hands autonomous execution.
    async fn execute_tool(
        &self,
        tool: &str,
        args: &serde_json::Value,
        hand_id: &str,
        execution_id: &str,
    ) -> Result<()> {
        info!(
            tool = %tool,
            hand_id = %hand_id,
            execution_id = %execution_id,
            "Executing approved tool for Hand"
        );

        let url = format!(
            "{}/hands/{}/executions/{}/execute",
            self.workflow_endpoint, hand_id, execution_id
        );
        let payload = ToolExecutionPayload {
            tool: tool.to_string(),
            args: args.clone(),
            approved: true,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to execute tool: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Tool execution failed ({}): {}",
                status,
                body
            ));
        }

        info!(
            tool = %tool,
            hand_id = %hand_id,
            execution_id = %execution_id,
            "Tool executed successfully"
        );
        Ok(())
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct MergeRequestPayload {
    platform: String,
    repo: String,
    mr_id: i64,
    action: String,
}

#[derive(Debug, Serialize)]
struct TradeCommandPayload {
    asset: String,
    action: String,
    amount: f64,
    approved: bool,
}

#[derive(Debug, Deserialize)]
struct TradeResult {
    order_id: Option<String>,
    #[allow(dead_code)]
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfigChangePayload {
    key: String,
    value: String,
    approved: bool,
}

#[derive(Debug, Serialize)]
struct HighCostOperationPayload {
    operation: String,
    estimated_cost: f64,
    approved: bool,
}

#[derive(Debug, Serialize)]
struct RiskOperationPayload {
    description: String,
    risk_level: String,
    approved: bool,
}

#[derive(Debug, Serialize)]
struct ToolExecutionPayload {
    tool: String,
    args: serde_json::Value,
    approved: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_executor_construction() {
        let executor = ActionExecutor::new(
            "http://localhost:4431",
            "http://localhost:4432",
            "http://localhost:4433",
        );

        assert_eq!(executor.channels_endpoint, "http://localhost:4431");
        assert_eq!(executor.trading_endpoint, "http://localhost:4432");
        assert_eq!(executor.workflow_endpoint, "http://localhost:4433");
    }

    #[test]
    fn test_merge_request_payload_serialization() {
        let payload = MergeRequestPayload {
            platform: "github".to_string(),
            repo: "org/repo".to_string(),
            mr_id: 42,
            action: "approve".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"platform\":\"github\""));
        assert!(json.contains("\"repo\":\"org/repo\""));
        assert!(json.contains("\"mr_id\":42"));
        assert!(json.contains("\"action\":\"approve\""));
    }

    #[test]
    fn test_trade_command_payload_serialization() {
        let payload = TradeCommandPayload {
            asset: "BTC".to_string(),
            action: "buy".to_string(),
            amount: 0.5,
            approved: true,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"asset\":\"BTC\""));
        assert!(json.contains("\"action\":\"buy\""));
        assert!(json.contains("\"amount\":0.5"));
        assert!(json.contains("\"approved\":true"));
    }

    #[test]
    fn test_config_change_payload_serialization() {
        let payload = ConfigChangePayload {
            key: "max_tokens".to_string(),
            value: "2000".to_string(),
            approved: true,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"key\":\"max_tokens\""));
        assert!(json.contains("\"value\":\"2000\""));
    }

    #[test]
    fn test_tool_execution_payload_serialization() {
        let payload = ToolExecutionPayload {
            tool: "bash".to_string(),
            args: serde_json::json!({"command": "ls -la"}),
            approved: true,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"tool\":\"bash\""));
        assert!(json.contains("\"command\":\"ls -la\""));
        assert!(json.contains("\"approved\":true"));
    }
}
