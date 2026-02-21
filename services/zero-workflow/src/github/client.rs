//! GitHub API client for code review operations.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Client
// ============================================================================

/// GitHub API client.
pub struct GitHubClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub client.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        Self::with_base_url(token, "https://api.github.com")
    }

    /// Create a client with custom base URL (for GitHub Enterprise).
    pub fn with_base_url(token: impl Into<String>, base_url: impl Into<String>) -> Result<Self> {
        let token = token.into();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Zero-Workflow/1.0"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.into(),
            token,
        })
    }

    /// Get pull request details.
    pub async fn get_pull_request(&self, owner: &str, repo: &str, number: i64) -> Result<PullRequest> {
        let url = format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch pull request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse pull request response")
    }

    /// Get files changed in a pull request.
    pub async fn get_pull_request_files(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<Vec<PullRequestFile>> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/files",
            self.base_url, owner, repo, number
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch PR files")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse PR files response")
    }

    /// Get the diff for a pull request.
    pub async fn get_pull_request_diff(&self, owner: &str, repo: &str, number: i64) -> Result<String> {
        let url = format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number);

        let response = self
            .client
            .get(&url)
            .header(ACCEPT, "application/vnd.github.diff")
            .send()
            .await
            .context("Failed to fetch PR diff")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response.text().await.context("Failed to read diff")
    }

    /// Create a review on a pull request.
    pub async fn create_review(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        review: &CreateReviewRequest,
    ) -> Result<ReviewResponse> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/reviews",
            self.base_url, owner, repo, number
        );

        let response = self
            .client
            .post(&url)
            .json(review)
            .send()
            .await
            .context("Failed to create review")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse review response")
    }

    /// Add a comment to a pull request.
    pub async fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        body: &str,
    ) -> Result<CommentResponse> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, number
        );

        let request = CreateCommentRequest {
            body: body.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to create comment")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse comment response")
    }

    /// Get the token (for debugging).
    pub fn token(&self) -> &str {
        &self.token
    }
}

// ============================================================================
// API Types
// ============================================================================

/// Pull request details from API.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub head: PullRequestRef,
    pub base: PullRequestRef,
    pub html_url: String,
    pub diff_url: String,
    pub changed_files: i64,
    pub additions: i64,
    pub deletions: i64,
}

/// Pull request ref (head/base).
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestRef {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// File changed in a pull request.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestFile {
    pub sha: String,
    pub filename: String,
    pub status: String,
    pub additions: i64,
    pub deletions: i64,
    pub changes: i64,
    pub patch: Option<String>,
}

/// Request to create a review.
#[derive(Debug, Clone, Serialize)]
pub struct CreateReviewRequest {
    /// Review body/summary
    pub body: String,
    /// Event: APPROVE, REQUEST_CHANGES, COMMENT
    pub event: String,
    /// Line-specific comments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<ReviewComment>,
}

/// Line-specific review comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    /// File path
    pub path: String,
    /// Line number (for single line)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Start line (for multi-line)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<i64>,
    /// Side: LEFT or RIGHT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// Comment body
    pub body: String,
}

/// Request to create a comment.
#[derive(Debug, Serialize)]
struct CreateCommentRequest {
    body: String,
}

/// Review response from API.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewResponse {
    pub id: i64,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
}

/// Comment response from API.
#[derive(Debug, Clone, Deserialize)]
pub struct CommentResponse {
    pub id: i64,
    pub body: String,
    pub html_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_review_request_serialization() {
        let request = CreateReviewRequest {
            body: "LGTM!".into(),
            event: "APPROVE".into(),
            comments: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"body\":\"LGTM!\""));
        assert!(json.contains("\"event\":\"APPROVE\""));
        assert!(!json.contains("\"comments\"")); // Empty vec should be skipped
    }

    #[test]
    fn test_review_comment_serialization() {
        let comment = ReviewComment {
            path: "src/main.rs".into(),
            line: Some(42),
            start_line: None,
            side: Some("RIGHT".into()),
            body: "Consider using const here".into(),
        };

        let json = serde_json::to_string(&comment).unwrap();
        assert!(json.contains("\"path\":\"src/main.rs\""));
        assert!(json.contains("\"line\":42"));
        assert!(!json.contains("\"start_line\"")); // None should be skipped
    }

    #[test]
    fn test_pull_request_file_deserialize() {
        let json = r#"{
            "sha": "abc123",
            "filename": "src/lib.rs",
            "status": "modified",
            "additions": 10,
            "deletions": 5,
            "changes": 15,
            "patch": "@@ -1,5 +1,10 @@\n+new line"
        }"#;

        let file: PullRequestFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.filename, "src/lib.rs");
        assert_eq!(file.additions, 10);
        assert!(file.patch.is_some());
    }
}
