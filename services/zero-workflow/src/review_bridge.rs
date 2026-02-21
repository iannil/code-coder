//! Review bridge for Zero Workflow.
//!
//! Connects Git platform webhooks to CodeCoder's code-reviewer agent,
//! and posts results back to the platform.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::github::{self, GitHubClient};
use crate::gitlab::{self, GitLabClient};

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
}
