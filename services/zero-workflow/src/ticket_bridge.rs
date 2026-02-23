//! Ticket bridge for Zero Workflow.
//!
//! Connects IM channels to ticket systems (GitHub Issues) with LLM-powered
//! classification. User feedback is automatically classified and routed:
//! - FAQ/common questions → Direct LLM response
//! - Bug reports → Create GitHub Issue → Notify dev team
//! - Feature requests → Create GitHub Issue (lower priority)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::github::GitHubClient;

// ============================================================================
// Feedback Types
// ============================================================================

/// User feedback to be processed.
#[derive(Debug, Clone)]
pub struct Feedback {
    /// Unique feedback ID (maps to message ID)
    pub id: String,
    /// User who submitted the feedback
    pub user_id: String,
    /// User display name (if available)
    pub user_name: Option<String>,
    /// Feedback content
    pub content: String,
    /// Source channel (feishu, wecom, telegram, etc.)
    pub channel_type: String,
    /// Source channel/group ID
    pub channel_id: String,
    /// Original message timestamp
    pub timestamp: i64,
}

/// Category of feedback determined by LLM classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackCategory {
    /// Frequently asked question - can be answered directly
    Faq,
    /// Bug report - requires GitHub issue
    Bug,
    /// Technical issue - requires GitHub issue
    TechnicalIssue,
    /// Feature request - lower priority GitHub issue
    FeatureRequest,
    /// General feedback - no action needed
    General,
}

impl FeedbackCategory {
    /// Check if this category requires creating a GitHub issue.
    pub fn requires_issue(&self) -> bool {
        matches!(
            self,
            FeedbackCategory::Bug | FeedbackCategory::TechnicalIssue | FeedbackCategory::FeatureRequest
        )
    }

    /// Get the priority level (lower is more urgent).
    pub fn priority(&self) -> u8 {
        match self {
            FeedbackCategory::Bug => 1,
            FeedbackCategory::TechnicalIssue => 2,
            FeedbackCategory::FeatureRequest => 3,
            FeedbackCategory::Faq => 4,
            FeedbackCategory::General => 5,
        }
    }
}

/// Result of LLM feedback classification.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedbackClassification {
    /// Detected category
    pub category: FeedbackCategory,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Suggested issue title (for Bug/Feature)
    pub suggested_title: Option<String>,
    /// Summary of the issue
    pub summary: String,
    /// Extracted key details
    pub details: Vec<String>,
    /// Suggested priority (P0, P1, P2, P3)
    pub priority: Option<String>,
}

/// Result of processing feedback.
#[derive(Debug, Clone)]
pub enum TicketResult {
    /// FAQ was answered directly
    DirectAnswer {
        answer: String,
    },
    /// GitHub issue was created
    IssueCreated {
        issue: CreatedIssue,
    },
    /// Feedback acknowledged but no action taken
    Acknowledged {
        reason: String,
    },
    /// Processing failed
    Failed {
        error: String,
    },
}

/// Information about a created GitHub issue.
#[derive(Debug, Clone)]
pub struct CreatedIssue {
    /// Issue number
    pub number: i64,
    /// Issue URL
    pub html_url: String,
    /// Issue title
    pub title: String,
    /// Labels applied
    pub labels: Vec<String>,
}

// ============================================================================
// IM Notification Configuration
// ============================================================================

/// Configuration for IM notifications.
#[derive(Debug, Clone, Default)]
pub struct TicketIMConfig {
    /// Enable IM notifications
    pub enabled: bool,
    /// Zero Channels endpoint
    pub channels_endpoint: Option<String>,
    /// Channel type (feishu, wecom, dingtalk)
    pub channel_type: String,
    /// Channel ID to notify
    pub channel_id: Option<String>,
}

// ============================================================================
// Ticket Bridge
// ============================================================================

/// Bridge between user feedback and ticket systems.
pub struct TicketBridge {
    /// CodeCoder API endpoint for LLM classification
    codecoder_endpoint: String,
    /// HTTP client
    client: reqwest::Client,
    /// GitHub client (if configured)
    github: Option<Arc<GitHubClient>>,
    /// Default repository (owner/repo)
    default_repo: Option<String>,
    /// Bug labels
    bug_labels: Vec<String>,
    /// Feature labels
    feature_labels: Vec<String>,
    /// IM notification configuration
    im_config: TicketIMConfig,
}

impl TicketBridge {
    /// Create a new ticket bridge.
    pub fn new(codecoder_endpoint: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            codecoder_endpoint: codecoder_endpoint.into(),
            client,
            github: None,
            default_repo: None,
            bug_labels: vec!["bug".into(), "triage".into()],
            feature_labels: vec!["enhancement".into()],
            im_config: TicketIMConfig::default(),
        }
    }

    /// Set the GitHub client.
    pub fn with_github(mut self, client: Arc<GitHubClient>) -> Self {
        self.github = Some(client);
        self
    }

    /// Set the default repository.
    pub fn with_default_repo(mut self, repo: impl Into<String>) -> Self {
        self.default_repo = Some(repo.into());
        self
    }

    /// Set bug labels.
    pub fn with_bug_labels(mut self, labels: Vec<String>) -> Self {
        self.bug_labels = labels;
        self
    }

    /// Set feature labels.
    pub fn with_feature_labels(mut self, labels: Vec<String>) -> Self {
        self.feature_labels = labels;
        self
    }

    /// Set IM notification configuration.
    pub fn with_im_config(mut self, config: TicketIMConfig) -> Self {
        self.im_config = config;
        self
    }

    /// Process user feedback and route appropriately.
    pub async fn process_feedback(&self, feedback: &Feedback) -> Result<TicketResult> {
        tracing::info!(
            feedback_id = %feedback.id,
            user_id = %feedback.user_id,
            channel = %feedback.channel_type,
            "Processing user feedback"
        );

        // Step 1: Classify the feedback using LLM
        let classification = match self.classify_feedback(&feedback.content).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "Failed to classify feedback");
                return Ok(TicketResult::Failed {
                    error: format!("Classification failed: {}", e),
                });
            }
        };

        tracing::info!(
            category = ?classification.category,
            confidence = classification.confidence,
            "Feedback classified"
        );

        // Step 2: Route based on category
        match classification.category {
            FeedbackCategory::Faq => {
                // Search knowledge base and return direct answer
                let answer = self.search_knowledge(&feedback.content).await?;
                Ok(TicketResult::DirectAnswer { answer })
            }
            FeedbackCategory::Bug | FeedbackCategory::TechnicalIssue => {
                // Create GitHub issue
                let issue = self.create_github_issue(&classification, feedback).await?;

                // Send IM notification
                if self.im_config.enabled {
                    if let Err(e) = self.notify_team(&issue, &classification).await {
                        tracing::warn!(error = %e, "Failed to send IM notification");
                    }
                }

                Ok(TicketResult::IssueCreated { issue })
            }
            FeedbackCategory::FeatureRequest => {
                // Create GitHub issue with feature labels
                let issue = self.create_github_issue(&classification, feedback).await?;
                Ok(TicketResult::IssueCreated { issue })
            }
            FeedbackCategory::General => Ok(TicketResult::Acknowledged {
                reason: "General feedback acknowledged".into(),
            }),
        }
    }

    /// Classify feedback using LLM.
    async fn classify_feedback(&self, content: &str) -> Result<FeedbackClassification> {
        let url = format!("{}/api/v1/chat", self.codecoder_endpoint);

        let prompt = format!(
            r#"你是一个用户反馈分类助手。请分析以下用户反馈，并以JSON格式返回分类结果。

用户反馈:
"""
{}
"""

请返回以下格式的JSON（不要包含其他内容）:
{{
  "category": "faq" | "bug" | "technical_issue" | "feature_request" | "general",
  "confidence": 0.0-1.0,
  "suggested_title": "建议的Issue标题（如果是bug或feature）",
  "summary": "问题摘要",
  "details": ["关键细节1", "关键细节2"],
  "priority": "P0" | "P1" | "P2" | "P3"
}}

分类规则:
- faq: 常见问题，可以直接回答
- bug: 明确的软件错误、崩溃、白屏、功能异常
- technical_issue: 技术性问题，需要调查
- feature_request: 功能建议或改进请求
- general: 一般性反馈、感谢或无需处理的内容"#,
            content
        );

        let request = ClassifyRequest {
            message: prompt,
            agent: Some("general".into()),
            user_id: "ticket-bridge".into(),
            channel: "internal".into(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to call classification API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Classification API error {}: {}", status, body);
        }

        let chat_response: ClassifyResponse = response
            .json()
            .await
            .context("Failed to parse classification response")?;

        // Parse JSON from the LLM response
        parse_classification_response(&chat_response.message)
    }

    /// Search knowledge base for FAQ answer.
    async fn search_knowledge(&self, query: &str) -> Result<String> {
        let url = format!("{}/api/v1/knowledge/search", self.codecoder_endpoint);

        let request = serde_json::json!({
            "query": query,
            "limit": 3,
            "min_score": 0.5
        });

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to search knowledge base")?;

        if !response.status().is_success() {
            // If knowledge search fails, return a generic message
            return Ok("抱歉，我暂时无法找到相关信息。我们的团队会尽快处理您的问题。".into());
        }

        let search_response: KnowledgeSearchResponse = response
            .json()
            .await
            .context("Failed to parse knowledge response")?;

        if let Some(data) = search_response.data {
            if !data.results.is_empty() {
                // Format the best result as an answer
                let best = &data.results[0];
                return Ok(format!(
                    "📚 **来自知识库**\n\n{}\n\n_来源: {}_",
                    best.content, best.filename
                ));
            }
        }

        Ok("抱歉，我暂时无法找到相关信息。您的问题已记录，我们的团队会尽快处理。".into())
    }

    /// Create a GitHub issue from classified feedback.
    async fn create_github_issue(
        &self,
        classification: &FeedbackClassification,
        feedback: &Feedback,
    ) -> Result<CreatedIssue> {
        let Some(ref github) = self.github else {
            anyhow::bail!("GitHub client not configured");
        };

        let Some(ref repo) = self.default_repo else {
            anyhow::bail!("Default repository not configured");
        };

        // Parse owner/repo
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid repository format: {}", repo);
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        // Build issue title
        let title = classification
            .suggested_title
            .clone()
            .unwrap_or_else(|| format!("用户反馈: {}", truncate_string(&feedback.content, 50)));

        // Build issue body
        let body = format!(
            r#"## 问题摘要

{}

## 详细信息

{}

## 用户信息

- **用户ID**: {}
- **用户名**: {}
- **渠道**: {}
- **时间**: {}

## 原始反馈

> {}

---
*由 ZeroBot 自动创建*"#,
            classification.summary,
            classification
                .details
                .iter()
                .map(|d| format!("- {}", d))
                .collect::<Vec<_>>()
                .join("\n"),
            feedback.user_id,
            feedback.user_name.as_deref().unwrap_or("Unknown"),
            feedback.channel_type,
            chrono::DateTime::from_timestamp_millis(feedback.timestamp)
                .map_or_else(|| "Unknown".to_string(), |dt| dt.to_string()),
            feedback.content,
        );

        // Determine labels
        let labels: Vec<&str> = match classification.category {
            FeedbackCategory::Bug | FeedbackCategory::TechnicalIssue => {
                self.bug_labels.iter().map(|s| s.as_str()).collect()
            }
            FeedbackCategory::FeatureRequest => {
                self.feature_labels.iter().map(|s| s.as_str()).collect()
            }
            _ => vec![],
        };

        // Create the issue
        let issue_response = github
            .create_issue(owner, repo_name, &title, &body, &labels)
            .await
            .context("Failed to create GitHub issue")?;

        tracing::info!(
            issue_number = issue_response.number,
            url = %issue_response.html_url,
            "Created GitHub issue"
        );

        Ok(CreatedIssue {
            number: issue_response.number,
            html_url: issue_response.html_url,
            title,
            labels: labels.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Send IM notification to the team.
    async fn notify_team(
        &self,
        issue: &CreatedIssue,
        classification: &FeedbackClassification,
    ) -> Result<()> {
        let endpoint = self
            .im_config
            .channels_endpoint
            .as_ref()
            .context("Channels endpoint not configured")?;

        let channel_id = self
            .im_config
            .channel_id
            .as_ref()
            .context("Channel ID not configured")?;

        let priority_emoji = match classification.priority.as_deref() {
            Some("P0") => "🔴",
            Some("P1") => "🟠",
            Some("P2") => "🟡",
            Some("P3") => "🟢",
            _ => "⚪",
        };

        let category_label = match classification.category {
            FeedbackCategory::Bug => "Bug",
            FeedbackCategory::TechnicalIssue => "技术问题",
            FeedbackCategory::FeatureRequest => "功能请求",
            _ => "反馈",
        };

        let message = format!(
            r#"🎫 **新工单创建**

{} **{}** - {}

**标题**: {}
**摘要**: {}

🔗 [查看 Issue]({})"#,
            priority_emoji,
            classification.priority.as_deref().unwrap_or("P2"),
            category_label,
            issue.title,
            classification.summary,
            issue.html_url,
        );

        let url = format!("{}/api/v1/send", endpoint);
        let body = serde_json::json!({
            "channel_type": self.im_config.channel_type,
            "channel_id": channel_id,
            "content": {
                "type": "markdown",
                "text": message
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send IM notification")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("IM notification failed {}: {}", status, body);
        }

        tracing::info!(
            channel_type = %self.im_config.channel_type,
            channel_id = %channel_id,
            "Sent IM notification for new issue"
        );

        Ok(())
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct ClassifyRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    user_id: String,
    channel: String,
}

#[derive(Debug, Deserialize)]
struct ClassifyResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSearchResponse {
    success: bool,
    data: Option<KnowledgeData>,
}

#[derive(Debug, Deserialize)]
struct KnowledgeData {
    results: Vec<KnowledgeResult>,
}

#[derive(Debug, Deserialize)]
struct KnowledgeResult {
    content: String,
    filename: String,
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse LLM response into FeedbackClassification.
fn parse_classification_response(response: &str) -> Result<FeedbackClassification> {
    // Try to find JSON in the response
    let json_start = response.find('{');
    let json_end = response.rfind('}');

    match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => {
            let json_str = &response[start..=end];
            serde_json::from_str(json_str).context("Failed to parse classification JSON")
        }
        _ => {
            // Fallback: return a general classification
            Ok(FeedbackClassification {
                category: FeedbackCategory::General,
                confidence: 0.5,
                suggested_title: None,
                summary: "Unable to classify feedback".into(),
                details: vec![],
                priority: None,
            })
        }
    }
}

/// Truncate a string to a maximum length.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }

    let truncated: String = s.chars().take(max_len).collect();
    format!("{}...", truncated)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_category_requires_issue() {
        assert!(FeedbackCategory::Bug.requires_issue());
        assert!(FeedbackCategory::TechnicalIssue.requires_issue());
        assert!(FeedbackCategory::FeatureRequest.requires_issue());
        assert!(!FeedbackCategory::Faq.requires_issue());
        assert!(!FeedbackCategory::General.requires_issue());
    }

    #[test]
    fn test_feedback_category_priority() {
        assert!(FeedbackCategory::Bug.priority() < FeedbackCategory::FeatureRequest.priority());
        assert!(FeedbackCategory::TechnicalIssue.priority() < FeedbackCategory::Faq.priority());
    }

    #[test]
    fn test_parse_classification_response() {
        let response = r#"Based on the feedback, here is the classification:
{
  "category": "bug",
  "confidence": 0.85,
  "suggested_title": "App crashes on startup",
  "summary": "User reports app crash when opening",
  "details": ["Crash on launch", "iOS 17"],
  "priority": "P1"
}
"#;

        let classification = parse_classification_response(response).unwrap();
        assert_eq!(classification.category, FeedbackCategory::Bug);
        assert_eq!(classification.confidence, 0.85);
        assert_eq!(
            classification.suggested_title,
            Some("App crashes on startup".into())
        );
        assert_eq!(classification.priority, Some("P1".into()));
    }

    #[test]
    fn test_parse_classification_response_no_json() {
        let response = "I couldn't understand the feedback";
        let classification = parse_classification_response(response).unwrap();
        assert_eq!(classification.category, FeedbackCategory::General);
        assert_eq!(classification.confidence, 0.5);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 5), "hello...");
        assert_eq!(truncate_string("你好世界", 2), "你好...");
    }

    #[test]
    fn test_ticket_bridge_creation() {
        let bridge = TicketBridge::new("http://localhost:4400")
            .with_default_repo("company/product")
            .with_bug_labels(vec!["bug".into(), "urgent".into()]);

        assert_eq!(bridge.default_repo, Some("company/product".into()));
        assert_eq!(bridge.bug_labels, vec!["bug", "urgent"]);
    }

    #[test]
    fn test_feedback_category_serialization() {
        let category = FeedbackCategory::Bug;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"bug\"");

        let parsed: FeedbackCategory = serde_json::from_str("\"feature_request\"").unwrap();
        assert_eq!(parsed, FeedbackCategory::FeatureRequest);
    }
}
