//! Review bridge for Zero Workflow.
//!
//! Connects Git platform webhooks to CodeCoder's code-reviewer agent,
//! posts results back to the platform, and sends IM notifications.
//!
//! ## Human-in-the-Loop (HitL) Integration
//!
//! The review bridge supports HitL for merge approvals. When enabled, after a code
//! review is approved, the bridge can request human approval before proceeding with
//! the merge. This is useful for:
//! - Critical repositories requiring human oversight
//! - Compliance requirements for production deployments
//! - Additional review gates for security-sensitive changes

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::github::{self, GitHubClient};
use crate::gitlab::{self, GitLabClient};
use zero_common::hitl_client::{ApprovalType, CreateApprovalRequest, HitLClient};

// ============================================================================
// IM Notification Configuration
// ============================================================================

/// Configuration for IM notifications after code review.
#[derive(Debug, Clone, Deserialize)]
pub struct IMNotificationConfig {
    /// Enable IM notifications
    #[serde(default)]
    pub enabled: bool,
    /// Zero Channels endpoint (e.g., "http://localhost:4411")
    pub channels_endpoint: Option<String>,
    /// Default channel type for notifications (feishu, wecom, dingtalk)
    #[serde(default = "default_channel_type")]
    pub channel_type: String,
    /// Channel ID to send notifications to (group chat ID)
    pub channel_id: Option<String>,
    /// Only notify on certain verdicts
    #[serde(default = "default_notify_on")]
    pub notify_on: Vec<String>,
}

fn default_channel_type() -> String {
    "feishu".to_string()
}

fn default_notify_on() -> Vec<String> {
    vec!["request_changes".to_string(), "critical".to_string()]
}

impl Default for IMNotificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channels_endpoint: None,
            channel_type: default_channel_type(),
            channel_id: None,
            notify_on: default_notify_on(),
        }
    }
}

// ============================================================================
// HitL Configuration
// ============================================================================

/// Configuration for Human-in-the-Loop merge approvals.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HitLIntegrationConfig {
    /// Enable HitL for merge approvals
    #[serde(default)]
    pub enabled: bool,
    /// Require human approval before merge
    #[serde(default)]
    pub require_approval_for_merge: bool,
    /// Default IM channel type for approval requests
    #[serde(default = "default_hitl_channel")]
    pub default_channel: String,
    /// List of users who can approve merge requests
    #[serde(default)]
    pub approvers: Vec<String>,
    /// Mapping of user IDs to their IM channel IDs
    #[serde(default)]
    pub user_channel_map: HashMap<String, String>,
    /// TTL for approval requests in seconds (default: 1 hour)
    #[serde(default = "default_approval_ttl")]
    pub approval_ttl_seconds: u64,
}

fn default_hitl_channel() -> String {
    "telegram".to_string()
}

fn default_approval_ttl() -> u64 {
    3600 // 1 hour
}

impl Default for HitLIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_approval_for_merge: false,
            default_channel: default_hitl_channel(),
            approvers: Vec::new(),
            user_channel_map: HashMap::new(),
            approval_ttl_seconds: default_approval_ttl(),
        }
    }
}

// ============================================================================
// Review Bridge
// ============================================================================

/// Bridge between Git platforms and CodeCoder for automated code review.
pub struct ReviewBridge {
    /// CodeCoder API endpoint
    codecoder_endpoint: String,
    /// HTTP client
    client: reqwest::Client,
    /// GitHub client (if configured)
    github: Option<Arc<GitHubClient>>,
    /// GitLab client (if configured)
    gitlab: Option<Arc<GitLabClient>>,
    /// IM notification configuration
    im_config: IMNotificationConfig,
    /// HitL client for approval requests
    hitl_client: Option<HitLClient>,
    /// HitL integration configuration
    hitl_config: HitLIntegrationConfig,
    /// Pending approval requests (MR key -> approval ID)
    pending_approvals: std::sync::Mutex<HashMap<String, String>>,
}

impl ReviewBridge {
    /// Create a new review bridge.
    pub fn new(codecoder_endpoint: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 min timeout for LLM
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            codecoder_endpoint: codecoder_endpoint.into(),
            client,
            github: None,
            gitlab: None,
            im_config: IMNotificationConfig::default(),
            hitl_client: None,
            hitl_config: HitLIntegrationConfig::default(),
            pending_approvals: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Set GitHub client.
    pub fn with_github(mut self, client: Arc<GitHubClient>) -> Self {
        self.github = Some(client);
        self
    }

    /// Set GitLab client.
    pub fn with_gitlab(mut self, client: Arc<GitLabClient>) -> Self {
        self.gitlab = Some(client);
        self
    }

    /// Set IM notification configuration.
    pub fn with_im_config(mut self, config: IMNotificationConfig) -> Self {
        self.im_config = config;
        self
    }

    /// Set HitL client for approval requests.
    pub fn with_hitl_client(mut self, client: HitLClient) -> Self {
        self.hitl_client = Some(client);
        self
    }

    /// Set HitL integration configuration.
    pub fn with_hitl_config(mut self, config: HitLIntegrationConfig) -> Self {
        self.hitl_config = config;
        self
    }

    /// Process a GitHub pull request event.
    pub async fn process_github_pr(&self, event: &github::PullRequestEvent) -> Result<ReviewResult> {
        if !event.should_review() {
            return Ok(ReviewResult::Skipped {
                reason: "Event action does not require review".into(),
            });
        }

        let Some(ref github) = self.github else {
            return Err(anyhow::anyhow!("GitHub client not configured"));
        };

        // Parse owner/repo from full_name
        let parts: Vec<&str> = event.repo_full_name().split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid repository name: {}", event.repo_full_name()));
        }
        let (owner, repo) = (parts[0], parts[1]);
        let pr_number = event.pr_number();

        tracing::info!(
            owner = owner,
            repo = repo,
            pr = pr_number,
            "Processing GitHub PR for code review"
        );

        // Get diff
        let diff = github.get_pull_request_diff(owner, repo, pr_number).await?;
        let files = github.get_pull_request_files(owner, repo, pr_number).await?;

        // Prepare context for review
        let context = ReviewContext {
            platform: "github".into(),
            repo_full_name: event.repo_full_name().into(),
            pr_number,
            title: event.pull_request.title.clone(),
            description: event.pull_request.body.clone(),
            diff,
            files: files.iter().map(|f| f.filename.clone()).collect(),
        };

        // Call CodeCoder
        let review = self.call_code_reviewer(&context).await?;

        // Post review
        let comment_url = self
            .post_github_review(github, owner, repo, pr_number, &review)
            .await?;

        // Send IM notification
        if self.should_notify(&review) {
            let _ = self
                .send_im_notification(&context, &review, &comment_url)
                .await
                .map_err(|e| tracing::warn!(error = %e, "Failed to send IM notification"));
        }

        Ok(ReviewResult::Success {
            comment_url,
            summary: review.summary,
        })
    }

    /// Process a GitLab merge request event.
    pub async fn process_gitlab_mr(&self, event: &gitlab::MergeRequestEvent) -> Result<ReviewResult> {
        if !event.should_review() {
            return Ok(ReviewResult::Skipped {
                reason: "Event action does not require review".into(),
            });
        }

        let Some(ref gitlab) = self.gitlab else {
            return Err(anyhow::anyhow!("GitLab client not configured"));
        };

        let project_id = event.project_id();
        let mr_iid = event.mr_iid();

        tracing::info!(
            project = %event.project_path(),
            mr = mr_iid,
            "Processing GitLab MR for code review"
        );

        // Get diff
        let changes = gitlab.get_merge_request_changes(project_id, mr_iid).await?;

        // Build diff string from changes
        let diff = changes
            .changes
            .iter()
            .map(|c| format!("--- {}\n+++ {}\n{}", c.old_path, c.new_path, c.diff))
            .collect::<Vec<_>>()
            .join("\n");

        let files: Vec<String> = changes.changes.iter().map(|c| c.new_path.clone()).collect();

        // Prepare context for review
        let context = ReviewContext {
            platform: "gitlab".into(),
            repo_full_name: event.project_path().into(),
            pr_number: mr_iid,
            title: event.object_attributes.title.clone(),
            description: event.object_attributes.description.clone(),
            diff,
            files,
        };

        // Call CodeCoder
        let review = self.call_code_reviewer(&context).await?;

        // Post review
        let comment_url = self
            .post_gitlab_review(gitlab, project_id, mr_iid, &review)
            .await?;

        // Send IM notification
        if self.should_notify(&review) {
            let _ = self
                .send_im_notification(&context, &review, &comment_url)
                .await
                .map_err(|e| tracing::warn!(error = %e, "Failed to send IM notification"));
        }

        Ok(ReviewResult::Success {
            comment_url,
            summary: review.summary,
        })
    }

    /// Call CodeCoder code-reviewer agent.
    async fn call_code_reviewer(&self, context: &ReviewContext) -> Result<CodeReview> {
        let url = format!("{}/api/v1/chat", self.codecoder_endpoint);

        // Build the review prompt
        let prompt = format!(
            r#"Please review this pull request/merge request:

## {title}

{description}

### Changed Files
{files}

### Diff
```diff
{diff}
```

Provide a code review focusing on:
1. Potential bugs or logic errors
2. Security vulnerabilities
3. Code quality and best practices
4. Performance concerns

Format your response as:
- Start with a brief summary (2-3 sentences)
- List specific issues found with file names and line numbers when possible
- End with an overall assessment (APPROVE, REQUEST_CHANGES, or COMMENT)"#,
            title = context.title,
            description = context.description.as_deref().unwrap_or("No description provided."),
            files = context.files.join("\n"),
            diff = truncate_diff(&context.diff, 50000), // Limit diff size
        );

        let request = CodeCoderRequest {
            message: prompt,
            agent: Some("code-reviewer".into()),
            user_id: "zero-workflow".into(),
            channel: "github".into(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to call CodeCoder API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CodeCoder API error {}: {}", status, body);
        }

        let codecoder_response: CodeCoderResponse = response
            .json()
            .await
            .context("Failed to parse CodeCoder response")?;

        // Parse the response to extract review details
        let review = parse_review_response(&codecoder_response.message);

        Ok(review)
    }

    /// Post review to GitHub.
    async fn post_github_review(
        &self,
        client: &GitHubClient,
        owner: &str,
        repo: &str,
        pr_number: i64,
        review: &CodeReview,
    ) -> Result<String> {
        // Format review as markdown
        let body = format_review_markdown(review);

        // Post as a comment (simpler than full PR review for now)
        let response = client
            .create_issue_comment(owner, repo, pr_number, &body)
            .await?;

        Ok(response.html_url)
    }

    /// Post review to GitLab.
    async fn post_gitlab_review(
        &self,
        client: &GitLabClient,
        project_id: i64,
        mr_iid: i64,
        review: &CodeReview,
    ) -> Result<String> {
        // Format review as markdown
        let body = format_review_markdown(review);

        // Post as a note
        let note = client.create_note(project_id, mr_iid, &body).await?;

        // GitLab notes don't have direct URLs, construct one
        Ok(format!(
            "Note #{} created on MR !{}",
            note.id, mr_iid
        ))
    }

    /// Check if we should send IM notification for this review.
    fn should_notify(&self, review: &CodeReview) -> bool {
        if !self.im_config.enabled {
            return false;
        }

        if self.im_config.channel_id.is_none() {
            return false;
        }

        // Check if verdict matches notification criteria
        let verdict_str = match review.verdict {
            ReviewVerdict::Approve => "approve",
            ReviewVerdict::RequestChanges => "request_changes",
            ReviewVerdict::Comment => "comment",
        };

        // Check if any findings are critical
        let has_critical = review
            .findings
            .iter()
            .any(|f| matches!(f.severity, FindingSeverity::Critical));

        self.im_config.notify_on.iter().any(|c| {
            c == verdict_str || (c == "critical" && has_critical)
        })
    }

    /// Send IM notification for a code review.
    async fn send_im_notification(
        &self,
        context: &ReviewContext,
        review: &CodeReview,
        comment_url: &str,
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

        // Format notification message
        let message = self.format_im_notification(context, review, comment_url);

        // Build the request to Zero Channels
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
            platform = %context.platform,
            pr_number = context.pr_number,
            "Sent IM notification for code review"
        );

        Ok(())
    }

    /// Format the IM notification message.
    fn format_im_notification(
        &self,
        context: &ReviewContext,
        review: &CodeReview,
        comment_url: &str,
    ) -> String {
        let verdict_emoji = match review.verdict {
            ReviewVerdict::Approve => "✅",
            ReviewVerdict::RequestChanges => "🔴",
            ReviewVerdict::Comment => "💬",
        };

        let verdict_text = match review.verdict {
            ReviewVerdict::Approve => "Approved",
            ReviewVerdict::RequestChanges => "Changes Requested",
            ReviewVerdict::Comment => "Reviewed",
        };

        let critical_count = review
            .findings
            .iter()
            .filter(|f| matches!(f.severity, FindingSeverity::Critical))
            .count();

        let high_count = review
            .findings
            .iter()
            .filter(|f| matches!(f.severity, FindingSeverity::High))
            .count();

        let mut message = format!(
            "**{} Code Review: {}**\n\n",
            verdict_emoji, verdict_text
        );

        message.push_str(&format!(
            "📂 **{}** - {}\n",
            context.repo_full_name, context.title
        ));

        message.push_str(&format!(
            "🔗 [View Review]({})\n\n",
            comment_url
        ));

        message.push_str(&format!("**Summary:** {}\n", review.summary));

        if critical_count > 0 || high_count > 0 {
            message.push_str("\n**Issues Found:**\n");
            if critical_count > 0 {
                message.push_str(&format!("- 🔴 {} critical issue(s)\n", critical_count));
            }
            if high_count > 0 {
                message.push_str(&format!("- 🟠 {} high priority issue(s)\n", high_count));
            }
        }

        message.push_str("\n---\n*Automated review by CodeCoder*");

        message
    }

    // ========================================================================
    // HitL Methods
    // ========================================================================

    /// Request merge approval through HitL system.
    ///
    /// Creates an approval request in the HitL system and returns the request ID.
    /// The caller should track this ID to check approval status later.
    pub async fn request_merge_approval(&self, mr: &MergeRequestInfo) -> Result<String> {
        let client = self
            .hitl_client
            .as_ref()
            .ok_or_else(|| anyhow!("HitL client not configured"))?;

        if !self.hitl_config.enabled {
            return Err(anyhow!("HitL is not enabled"));
        }

        let channel_id = self.get_channel_id(&mr.author)?;

        let request = CreateApprovalRequest {
            approval_type: ApprovalType::MergeRequest {
                platform: mr.platform.clone(),
                repo: mr.repo.clone(),
                mr_id: mr.id,
            },
            requester: mr.author.clone(),
            approvers: self.hitl_config.approvers.clone(),
            title: format!("Merge Approval: {}", mr.title),
            description: Some(format!(
                "Review approved for MR !{} in {}\n\nURL: {}\nAuthor: {}",
                mr.id, mr.repo, mr.url, mr.author
            )),
            channel: format!("{}:{}", self.hitl_config.default_channel, channel_id),
            metadata: serde_json::json!({
                "title": mr.title,
                "url": mr.url,
                "review_status": "approved",
                "platform": mr.platform,
                "repo": mr.repo,
                "mr_id": mr.id,
            }),
            ttl_seconds: Some(self.hitl_config.approval_ttl_seconds),
        };

        let response = client
            .create_request(request)
            .await
            .map_err(|e| anyhow!("Failed to create HitL approval request: {}", e))?;

        let approval_id = response
            .approval
            .ok_or_else(|| anyhow!("HitL response missing approval data"))?
            .id;

        // Store the pending approval
        let mr_key = format!("{}:{}:{}", mr.platform, mr.repo, mr.id);
        if let Ok(mut pending) = self.pending_approvals.lock() {
            pending.insert(mr_key, approval_id.clone());
        }

        tracing::info!(
            approval_id = %approval_id,
            platform = %mr.platform,
            repo = %mr.repo,
            mr_id = mr.id,
            "Created HitL merge approval request"
        );

        Ok(approval_id)
    }

    /// Handle a review that was approved, potentially triggering HitL approval.
    ///
    /// If HitL is enabled and `require_approval_for_merge` is true, this creates
    /// an approval request instead of immediately merging.
    pub async fn handle_review_approved(&self, mr: &MergeRequestInfo) -> Result<ApprovalAction> {
        if self.hitl_config.enabled && self.hitl_config.require_approval_for_merge {
            let approval_id = self.request_merge_approval(mr).await?;
            tracing::info!(
                approval_id = %approval_id,
                mr_id = mr.id,
                "Merge approval requested, waiting for human decision"
            );
            Ok(ApprovalAction::PendingApproval { approval_id })
        } else {
            // No HitL required, proceed with merge
            Ok(ApprovalAction::ReadyToMerge)
        }
    }

    /// Get the channel ID for a given user.
    ///
    /// Looks up the user in the channel map, falling back to the IM config channel.
    fn get_channel_id(&self, user: &str) -> Result<String> {
        // Try user-specific channel first
        if let Some(channel_id) = self.hitl_config.user_channel_map.get(user) {
            return Ok(channel_id.clone());
        }

        // Fall back to IM config channel
        if let Some(ref channel_id) = self.im_config.channel_id {
            return Ok(channel_id.clone());
        }

        Err(anyhow!(
            "No channel ID configured for user '{}' and no default channel set",
            user
        ))
    }

    /// Check if a merge request has a pending HitL approval.
    pub fn get_pending_approval(&self, platform: &str, repo: &str, mr_id: i64) -> Option<String> {
        let mr_key = format!("{}:{}:{}", platform, repo, mr_id);
        self.pending_approvals
            .lock()
            .ok()
            .and_then(|pending| pending.get(&mr_key).cloned())
    }

    /// Clear a pending approval (e.g., after it's been processed).
    pub fn clear_pending_approval(&self, platform: &str, repo: &str, mr_id: i64) {
        let mr_key = format!("{}:{}:{}", platform, repo, mr_id);
        if let Ok(mut pending) = self.pending_approvals.lock() {
            pending.remove(&mr_key);
        }
    }
}

// ============================================================================
// HitL Types
// ============================================================================

/// Information about a merge request for HitL approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequestInfo {
    /// Git platform (e.g., "github", "gitlab")
    pub platform: String,
    /// Repository full name (e.g., "owner/repo")
    pub repo: String,
    /// Merge request ID or PR number
    pub id: i64,
    /// MR/PR title
    pub title: String,
    /// MR/PR URL
    pub url: String,
    /// Author username
    pub author: String,
}

/// Result of the approval check.
#[derive(Debug, Clone)]
pub enum ApprovalAction {
    /// Ready to merge (HitL not required or already approved)
    ReadyToMerge,
    /// Waiting for HitL approval
    PendingApproval { approval_id: String },
}

// ============================================================================
// Types
// ============================================================================

/// Context for a code review.
#[derive(Debug, Clone)]
struct ReviewContext {
    platform: String,
    repo_full_name: String,
    pr_number: i64,
    title: String,
    description: Option<String>,
    diff: String,
    files: Vec<String>,
}

/// Request to CodeCoder chat API.
#[derive(Debug, Serialize)]
struct CodeCoderRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    user_id: String,
    channel: String,
}

/// Response from CodeCoder chat API.
#[derive(Debug, Deserialize)]
struct CodeCoderResponse {
    message: String,
    #[serde(default)]
    agent: Option<String>,
}

/// Parsed code review.
#[derive(Debug, Clone)]
pub struct CodeReview {
    /// Review summary
    pub summary: String,
    /// Detailed findings
    pub findings: Vec<ReviewFinding>,
    /// Overall verdict
    pub verdict: ReviewVerdict,
}

/// A specific finding in the review.
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    /// Severity level
    pub severity: FindingSeverity,
    /// File (if applicable)
    pub file: Option<String>,
    /// Line number (if applicable)
    pub line: Option<i64>,
    /// Description
    pub description: String,
}

/// Finding severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Overall review verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}

/// Result of a review operation.
#[derive(Debug)]
pub enum ReviewResult {
    Success { comment_url: String, summary: String },
    Skipped { reason: String },
}

// ============================================================================
// Helpers
// ============================================================================

/// Truncate diff to a maximum length.
fn truncate_diff(diff: &str, max_len: usize) -> String {
    if diff.len() <= max_len {
        return diff.to_string();
    }

    let truncated = &diff[..max_len];
    format!("{}\n\n... [diff truncated, {} more characters]", truncated, diff.len() - max_len)
}

/// Parse review response from CodeCoder.
fn parse_review_response(response: &str) -> CodeReview {
    // Simple parsing - extract summary (first paragraph) and verdict
    let lines: Vec<&str> = response.lines().collect();

    // Find summary (first non-empty paragraph)
    let summary = lines
        .iter()
        .take_while(|l| !l.is_empty())
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");

    // Detect verdict from response
    let verdict = if response.to_lowercase().contains("approve")
        && !response.to_lowercase().contains("request changes")
    {
        ReviewVerdict::Approve
    } else if response.to_lowercase().contains("request changes")
        || response.to_lowercase().contains("must be fixed")
        || response.to_lowercase().contains("critical")
    {
        ReviewVerdict::RequestChanges
    } else {
        ReviewVerdict::Comment
    };

    // TODO: Parse specific findings with file/line info
    let findings = vec![];

    CodeReview {
        summary: if summary.is_empty() {
            "Review completed.".into()
        } else {
            summary
        },
        findings,
        verdict,
    }
}

/// Format review as markdown.
fn format_review_markdown(review: &CodeReview) -> String {
    let verdict_emoji = match review.verdict {
        ReviewVerdict::Approve => "✅",
        ReviewVerdict::RequestChanges => "🔴",
        ReviewVerdict::Comment => "💬",
    };

    let verdict_text = match review.verdict {
        ReviewVerdict::Approve => "Approved",
        ReviewVerdict::RequestChanges => "Changes Requested",
        ReviewVerdict::Comment => "Comments",
    };

    let mut md = format!(
        "## {} Code Review: {}\n\n{}\n",
        verdict_emoji, verdict_text, review.summary
    );

    if !review.findings.is_empty() {
        md.push_str("\n### Findings\n\n");
        for finding in &review.findings {
            let severity = match finding.severity {
                FindingSeverity::Critical => "🔴 CRITICAL",
                FindingSeverity::High => "🟠 HIGH",
                FindingSeverity::Medium => "🟡 MEDIUM",
                FindingSeverity::Low => "🟢 LOW",
                FindingSeverity::Info => "ℹ️ INFO",
            };

            let location = match (&finding.file, finding.line) {
                (Some(f), Some(l)) => format!(" (`{}:{}`)", f, l),
                (Some(f), None) => format!(" (`{}`)", f),
                _ => String::new(),
            };

            md.push_str(&format!("- **{}**{}: {}\n", severity, location, finding.description));
        }
    }

    md.push_str("\n---\n*Automated review by CodeCoder*\n");

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_diff() {
        let short = "short diff";
        assert_eq!(truncate_diff(short, 100), short);

        let long = "a".repeat(100);
        let truncated = truncate_diff(&long, 50);
        assert!(truncated.contains("... [diff truncated"));
    }

    #[test]
    fn test_parse_review_approve() {
        let response = "This PR looks great! The code is clean and well-tested. I approve this change.";
        let review = parse_review_response(response);
        assert_eq!(review.verdict, ReviewVerdict::Approve);
    }

    #[test]
    fn test_parse_review_request_changes() {
        let response = "There are several issues that must be fixed. I request changes.";
        let review = parse_review_response(response);
        assert_eq!(review.verdict, ReviewVerdict::RequestChanges);
    }

    #[test]
    fn test_format_review_markdown() {
        let review = CodeReview {
            summary: "Overall the code looks good.".into(),
            findings: vec![
                ReviewFinding {
                    severity: FindingSeverity::Medium,
                    file: Some("src/main.rs".into()),
                    line: Some(42),
                    description: "Consider using const here".into(),
                },
            ],
            verdict: ReviewVerdict::Comment,
        };

        let md = format_review_markdown(&review);
        assert!(md.contains("💬 Code Review"));
        assert!(md.contains("src/main.rs:42"));
        assert!(md.contains("MEDIUM"));
    }

    // ========================================================================
    // HitL Integration Tests
    // ========================================================================

    fn create_test_mr_info() -> MergeRequestInfo {
        MergeRequestInfo {
            platform: "github".to_string(),
            repo: "test-org/test-repo".to_string(),
            id: 123,
            title: "Add new feature".to_string(),
            url: "https://github.com/test-org/test-repo/pull/123".to_string(),
            author: "test-user".to_string(),
        }
    }

    #[test]
    fn test_hitl_config_defaults() {
        let config = HitLIntegrationConfig::default();
        assert!(!config.enabled);
        assert!(!config.require_approval_for_merge);
        assert_eq!(config.default_channel, "telegram");
        assert!(config.approvers.is_empty());
        assert!(config.user_channel_map.is_empty());
        assert_eq!(config.approval_ttl_seconds, 3600);
    }

    #[test]
    fn test_hitl_config_serialization() {
        let mut user_map = HashMap::new();
        user_map.insert("alice".to_string(), "channel-alice".to_string());
        user_map.insert("bob".to_string(), "channel-bob".to_string());

        let config = HitLIntegrationConfig {
            enabled: true,
            require_approval_for_merge: true,
            default_channel: "slack".to_string(),
            approvers: vec!["admin@example.com".to_string()],
            user_channel_map: user_map,
            approval_ttl_seconds: 7200,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"require_approval_for_merge\":true"));
        assert!(json.contains("\"default_channel\":\"slack\""));

        let deserialized: HitLIntegrationConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert!(deserialized.require_approval_for_merge);
        assert_eq!(deserialized.approvers, vec!["admin@example.com"]);
    }

    #[test]
    fn test_merge_request_info_serialization() {
        let mr = create_test_mr_info();

        let json = serde_json::to_string(&mr).unwrap();
        assert!(json.contains("\"platform\":\"github\""));
        assert!(json.contains("\"repo\":\"test-org/test-repo\""));
        assert!(json.contains("\"id\":123"));

        let deserialized: MergeRequestInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.platform, "github");
        assert_eq!(deserialized.id, 123);
    }

    #[test]
    fn test_review_bridge_with_hitl_config() {
        let config = HitLIntegrationConfig {
            enabled: true,
            require_approval_for_merge: true,
            default_channel: "telegram".to_string(),
            approvers: vec!["admin".to_string()],
            user_channel_map: HashMap::new(),
            approval_ttl_seconds: 3600,
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_config(config);

        assert!(bridge.hitl_config.enabled);
        assert!(bridge.hitl_config.require_approval_for_merge);
    }

    #[test]
    fn test_get_channel_id_from_user_map() {
        let mut user_map = HashMap::new();
        user_map.insert("alice".to_string(), "alice-channel-123".to_string());

        let config = HitLIntegrationConfig {
            enabled: true,
            require_approval_for_merge: true,
            default_channel: "telegram".to_string(),
            approvers: vec![],
            user_channel_map: user_map,
            approval_ttl_seconds: 3600,
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_config(config);

        let channel = bridge.get_channel_id("alice").unwrap();
        assert_eq!(channel, "alice-channel-123");
    }

    #[test]
    fn test_get_channel_id_fallback_to_im_config() {
        let config = HitLIntegrationConfig::default();
        let im_config = IMNotificationConfig {
            enabled: true,
            channels_endpoint: Some("http://localhost:4431".to_string()),
            channel_type: "telegram".to_string(),
            channel_id: Some("default-channel-456".to_string()),
            notify_on: vec![],
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_config(config)
            .with_im_config(im_config);

        let channel = bridge.get_channel_id("unknown-user").unwrap();
        assert_eq!(channel, "default-channel-456");
    }

    #[test]
    fn test_get_channel_id_error_when_no_channel() {
        let bridge = ReviewBridge::new("http://localhost:4400");
        let result = bridge.get_channel_id("unknown-user");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No channel ID configured"));
    }

    #[test]
    fn test_pending_approval_tracking() {
        let bridge = ReviewBridge::new("http://localhost:4400");

        // Initially no pending approval
        assert!(bridge.get_pending_approval("github", "org/repo", 42).is_none());

        // Simulate adding a pending approval (internal test)
        {
            let mut pending = bridge.pending_approvals.lock().unwrap();
            pending.insert("github:org/repo:42".to_string(), "approval-123".to_string());
        }

        // Now it should exist
        let approval_id = bridge.get_pending_approval("github", "org/repo", 42);
        assert_eq!(approval_id, Some("approval-123".to_string()));

        // Clear it
        bridge.clear_pending_approval("github", "org/repo", 42);
        assert!(bridge.get_pending_approval("github", "org/repo", 42).is_none());
    }

    #[tokio::test]
    async fn test_handle_review_approved_no_hitl() {
        let bridge = ReviewBridge::new("http://localhost:4400");
        let mr = create_test_mr_info();

        let result = bridge.handle_review_approved(&mr).await.unwrap();
        assert!(matches!(result, ApprovalAction::ReadyToMerge));
    }

    #[tokio::test]
    async fn test_handle_review_approved_hitl_disabled() {
        let config = HitLIntegrationConfig {
            enabled: false,
            require_approval_for_merge: true,
            ..Default::default()
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_config(config);

        let mr = create_test_mr_info();
        let result = bridge.handle_review_approved(&mr).await.unwrap();

        // Even with require_approval_for_merge, if HitL is disabled, skip it
        assert!(matches!(result, ApprovalAction::ReadyToMerge));
    }

    #[tokio::test]
    async fn test_request_merge_approval_no_client() {
        let config = HitLIntegrationConfig {
            enabled: true,
            require_approval_for_merge: true,
            ..Default::default()
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_config(config);

        let mr = create_test_mr_info();
        let result = bridge.request_merge_approval(&mr).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HitL client not configured"));
    }

    #[tokio::test]
    async fn test_request_merge_approval_hitl_disabled() {
        let client = HitLClient::new("http://localhost:4430");
        let config = HitLIntegrationConfig {
            enabled: false,
            ..Default::default()
        };

        let bridge = ReviewBridge::new("http://localhost:4400")
            .with_hitl_client(client)
            .with_hitl_config(config);

        let mr = create_test_mr_info();
        let result = bridge.request_merge_approval(&mr).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HitL is not enabled"));
    }

    #[test]
    fn test_approval_action_variants() {
        let ready = ApprovalAction::ReadyToMerge;
        assert!(matches!(ready, ApprovalAction::ReadyToMerge));

        let pending = ApprovalAction::PendingApproval {
            approval_id: "test-123".to_string(),
        };
        if let ApprovalAction::PendingApproval { approval_id } = pending {
            assert_eq!(approval_id, "test-123");
        } else {
            panic!("Expected PendingApproval variant");
        }
    }
}
