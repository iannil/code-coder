//! Notification bridge for Hand execution results.
//!
//! Sends Hand execution notifications to IM channels via zero-channels service.

use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::hands::state::ExecutionStatus;
use zero_common::config::HandNotificationConfig;

use super::{HandExecution, HandManifest};

/// Request body for zero-channels send API.
#[derive(Debug, Serialize)]
struct ChannelsSendRequest {
    channel_type: String,
    channel_id: String,
    content: MessageContent,
}

/// Message content for IM notification.
#[derive(Debug, Serialize)]
struct MessageContent {
    #[serde(rename = "type")]
    msg_type: String,
    text: String,
}

/// Response from zero-channels send API.
#[derive(Debug, Deserialize)]
struct ChannelsSendResponse {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

/// Bridge for sending Hand execution notifications.
#[derive(Clone)]
pub struct NotificationBridge {
    /// HTTP client
    client: Client,
    /// Zero channels endpoint
    channels_endpoint: String,
}

impl NotificationBridge {
    /// Create a new notification bridge.
    pub fn new(channels_endpoint: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            channels_endpoint,
        }
    }

    /// Check if notification should be sent based on configuration and status.
    pub fn should_send(
        &self,
        config: &HandNotificationConfig,
        status: &ExecutionStatus,
    ) -> bool {
        match config.send_when.as_str() {
            "always" => true,
            "on_success" => *status == ExecutionStatus::Success,
            "on_failure" => matches!(
                status,
                ExecutionStatus::Failed | ExecutionStatus::Cancelled
            ),
            _ => true, // Default to always for unknown values
        }
    }

    /// Send notification for Hand execution result.
    pub async fn send_notification(
        &self,
        hand: &HandManifest,
        execution: &HandExecution,
        config: &HandNotificationConfig,
    ) -> Result<()> {
        // Check if we should send based on configuration
        if !self.should_send(config, &execution.status) {
            tracing::debug!(
                hand_id = %hand.config.id,
                status = ?execution.status,
                send_when = %config.send_when,
                "Skipping notification (condition not met)"
            );
            return Ok(());
        }

        // Format message based on template
        let message = self.format_message(hand, execution, &config.template)?;

        // Build request
        let endpoint = format!("{}/api/v1/send", self.channels_endpoint.trim_end_matches('/'));
        let request = ChannelsSendRequest {
            channel_type: config.channel_type.clone(),
            channel_id: config.channel_id.clone(),
            content: MessageContent {
                msg_type: "markdown".to_string(),
                text: message,
            },
        };

        tracing::info!(
            hand_id = %hand.config.id,
            channel_type = %config.channel_type,
            channel_id = %config.channel_id,
            template = %config.template,
            "Sending Hand execution notification"
        );

        // Send request
        let response = self
            .client
            .post(&endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send notification request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("IM notification failed {}: {}", status, body);
        }

        let response_json: ChannelsSendResponse = response
            .json()
            .await
            .context("Failed to parse notification response")?;

        if !response_json.success {
            if let Some(error) = response_json.error {
                anyhow::bail!("IM notification error: {}", error);
            }
            anyhow::bail!("IM notification failed with unknown error");
        }

        tracing::info!(
            hand_id = %hand.config.id,
            execution_id = %execution.id,
            "Hand execution notification sent successfully"
        );

        Ok(())
    }

    /// Format notification message based on template.
    fn format_message(
        &self,
        hand: &HandManifest,
        execution: &HandExecution,
        template: &str,
    ) -> Result<String> {
        match template {
            "detailed" => self.format_detailed(hand, execution),
            _ => self.format_brief(hand, execution), // Default to brief
        }
    }

    /// Format brief notification message.
    fn format_brief(&self, hand: &HandManifest, execution: &HandExecution) -> Result<String> {
        let status_icon = match execution.status {
            ExecutionStatus::Success => "✅",
            ExecutionStatus::Failed => "❌",
            ExecutionStatus::Cancelled => "🚫",
            _ => "⏳",
        };

        let status_text = match execution.status {
            ExecutionStatus::Success => "执行成功",
            ExecutionStatus::Failed => "执行失败",
            ExecutionStatus::Cancelled => "已取消",
            ExecutionStatus::Running => "执行中",
            ExecutionStatus::Scheduled => "已调度",
            ExecutionStatus::WaitingApproval => "等待审批",
        };

        let duration_ms = execution
            .ended_at
            .unwrap_or(Utc::now())
            .timestamp_millis()
            - execution.started_at.timestamp_millis();
        let duration_sec = duration_ms as f64 / 1000.0;

        let mut message = format!(
            "{} **Hand {}**\n\n**{}** ({})\n",
            status_icon, status_text, hand.config.name, hand.config.id
        );

        // Add agent info
        if !hand.config.agent.is_empty() {
            message.push_str(&format!("Agent: {}", hand.config.agent));
        } else if let Some(ref agents) = hand.config.agents {
            message.push_str(&format!("Agents: {}", agents.join(", ")));
        }
        message.push_str(&format!(" | 耗时: {:.1}s\n\n", duration_sec));

        // Add output preview (first 500 chars)
        if let Some(ref output) = execution.output {
            let preview = if output.len() > 500 {
                format!("{}...", &output[..500])
            } else {
                output.clone()
            };
            message.push_str(&format!("{}\n", preview));
        }

        // Add error if present
        if let Some(ref error) = execution.error {
            message.push_str(&format!("\n❗ **错误**: {}\n", error));
        }

        message.push_str(&format!(
            "\n---\nGenerated at {}\n",
            execution.started_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(message)
    }

    /// Format detailed notification message.
    fn format_detailed(&self, hand: &HandManifest, execution: &HandExecution) -> Result<String> {
        let status_icon = match execution.status {
            ExecutionStatus::Success => "✅",
            ExecutionStatus::Failed => "❌",
            ExecutionStatus::Cancelled => "🚫",
            _ => "⏳",
        };

        let status_text = match execution.status {
            ExecutionStatus::Success => "Success",
            ExecutionStatus::Failed => "Failed",
            ExecutionStatus::Cancelled => "Cancelled",
            ExecutionStatus::Running => "Running",
            ExecutionStatus::Scheduled => "Scheduled",
            ExecutionStatus::WaitingApproval => "Waiting Approval",
        };

        let mut message = format!("# Hand 执行报告\n\n");

        // Basic info section
        message.push_str("## 基本信息\n\n");
        message.push_str(&format!("- **Hand**: {} ({})\n", hand.config.name, hand.config.id));

        // Agent info
        if !hand.config.agent.is_empty() {
            message.push_str(&format!("- **Agent**: {}\n", hand.config.agent));
        } else if let Some(ref agents) = hand.config.agents {
            message.push_str(&format!("- **Agents**: {}\n", agents.join(", ")));
        }

        message.push_str(&format!("- **状态**: {} {}\n", status_icon, status_text));

        // Timestamps
        message.push_str(&format!(
            "- **开始时间**: {}\n",
            execution.started_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(ended) = execution.ended_at {
            message.push_str(&format!(
                "- **结束时间**: {}\n",
                ended.format("%Y-%m-%d %H:%M:%S UTC")
            ));

            let duration_ms = ended.timestamp_millis() - execution.started_at.timestamp_millis();
            let duration_sec = duration_ms as f64 / 1000.0;
            message.push_str(&format!("- **耗时**: {:.2}s\n", duration_sec));
        }

        message.push('\n');

        // CLOSE scores if available
        if let Some(metadata) = execution.metadata.as_object() {
            if let Some(close_scores) = metadata.get("close_scores").and_then(|v| v.as_array()) {
                if let Some(first_score) = close_scores.first().as_ref() {
                    message.push_str("## CLOSE 评估\n\n");
                    if let Some(total) = first_score.get("total").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- **总分**: {:.1}/10\n", total));
                    }
                    if let Some(convergence) = first_score.get("convergence").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- Convergence: {:.1}\n", convergence));
                    }
                    if let Some(leverage) = first_score.get("leverage").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- Leverage: {:.1}\n", leverage));
                    }
                    if let Some(optionality) = first_score.get("optionality").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- Optionality: {:.1}\n", optionality));
                    }
                    if let Some(surplus) = first_score.get("surplus").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- Surplus: {:.1}\n", surplus));
                    }
                    if let Some(evolution) = first_score.get("evolution").and_then(|v| v.as_f64()) {
                        message.push_str(&format!("- Evolution: {:.1}\n", evolution));
                    }
                    message.push('\n');
                }
            }
        }

        // Execution result
        message.push_str("## 执行结果\n\n");

        if let Some(ref output) = execution.output {
            // Limit output to 3000 chars for IM
            let truncated = if output.len() > 3000 {
                format!("{}...\n\n*(Output truncated)*", &output[..3000])
            } else {
                output.clone()
            };
            message.push_str(&truncated);
        }

        // Error section
        if let Some(ref error) = execution.error {
            message.push_str(&format!("\n\n## 错误\n\n```\n{}\n```\n", error));
        }

        message.push_str(&format!(
            "\n---\n*Generated by Hands system at {}*",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_hand() -> HandManifest {
        HandManifest {
            config: crate::hands::manifest::HandConfig {
                id: "test-hand".to_string(),
                name: "Test Hand".to_string(),
                version: "1.0.0".to_string(),
                schedule: "0 * * * * *".to_string(),
                agent: "macro".to_string(),
                agents: None,
                pipeline: None,
                enabled: true,
                memory_path: None,
                params: serde_json::json!({}),
                autonomy: None,
                decision: None,
                resources: None,
                notification: None,
                description: String::new(),
            },
            content: "Test content".to_string(),
            path: std::path::PathBuf::from("/test/HAND.md"),
            frontmatter: String::new(),
        }
    }

    fn create_test_execution(status: ExecutionStatus) -> HandExecution {
        HandExecution {
            id: "exec-123".to_string(),
            hand_id: "test-hand".to_string(),
            status,
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            output: Some("Test output content here".to_string()),
            error: None,
            memory_path: None,
            previous_execution_id: None,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_should_send_on_success() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "brief".to_string(),
            send_when: "on_success".to_string(),
        };

        assert!(bridge.should_send(&config, &ExecutionStatus::Success));
        assert!(!bridge.should_send(&config, &ExecutionStatus::Failed));
    }

    #[test]
    fn test_should_send_on_failure() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "brief".to_string(),
            send_when: "on_failure".to_string(),
        };

        assert!(!bridge.should_send(&config, &ExecutionStatus::Success));
        assert!(bridge.should_send(&config, &ExecutionStatus::Failed));
        assert!(bridge.should_send(&config, &ExecutionStatus::Cancelled));
    }

    #[test]
    fn test_should_send_always() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "brief".to_string(),
            send_when: "always".to_string(),
        };

        assert!(bridge.should_send(&config, &ExecutionStatus::Success));
        assert!(bridge.should_send(&config, &ExecutionStatus::Failed));
    }

    #[test]
    fn test_format_brief_message() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());
        let hand = create_test_hand();
        let execution = create_test_execution(ExecutionStatus::Success);

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "brief".to_string(),
            send_when: "always".to_string(),
        };

        let message = bridge.format_message(&hand, &execution, &config.template).unwrap();

        assert!(message.contains("Test Hand"));
        assert!(message.contains("test-hand"));
        assert!(message.contains("✅"));
        assert!(message.contains("Agent: macro"));
        assert!(message.contains("Test output content"));
    }

    #[test]
    fn test_format_detailed_message() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());
        let hand = create_test_hand();
        let execution = create_test_execution(ExecutionStatus::Success);

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "detailed".to_string(),
            send_when: "always".to_string(),
        };

        let message = bridge
            .format_message(&hand, &execution, &config.template)
            .unwrap();

        assert!(message.contains("# Hand 执行报告"));
        assert!(message.contains("## 基本信息"));
        assert!(message.contains("## 执行结果"));
        assert!(message.contains("Test Hand"));
        assert!(message.contains("macro"));
    }

    #[test]
    fn test_format_brief_with_error() {
        let bridge = NotificationBridge::new("http://localhost:4431".to_string());
        let hand = create_test_hand();
        let mut execution = create_test_execution(ExecutionStatus::Failed);
        execution.error = Some("Test error message".to_string());

        let config = HandNotificationConfig {
            channel_type: "telegram".to_string(),
            channel_id: "test-channel".to_string(),
            template: "brief".to_string(),
            send_when: "always".to_string(),
        };

        let message = bridge.format_message(&hand, &execution, &config.template).unwrap();

        assert!(message.contains("❌"));
        assert!(message.contains("Test error message"));
    }
}
