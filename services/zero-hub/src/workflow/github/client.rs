//! GitHub API client for code review and issue operations.

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

    // ========================================================================
    // Issue Operations
    // ========================================================================

    /// Create a new issue in a repository.
    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<IssueResponse> {
        let url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);

        let request = CreateIssueRequest {
            title: title.to_string(),
            body: body.to_string(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to create issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse issue response")
    }

    /// Get an issue by number.
    pub async fn get_issue(&self, owner: &str, repo: &str, number: i64) -> Result<IssueResponse> {
        let url = format!("{}/repos/{}/{}/issues/{}", self.base_url, owner, repo, number);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse issue response")
    }

    /// Add labels to an issue.
    pub async fn add_labels(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        labels: &[&str],
    ) -> Result<Vec<Label>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/labels",
            self.base_url, owner, repo, number
        );

        let request = AddLabelsRequest {
            labels: labels.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to add labels")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse labels response")
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

// ============================================================================
// Issue Types
// ============================================================================

/// Request to create an issue.
#[derive(Debug, Clone, Serialize)]
pub struct CreateIssueRequest {
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: String,
    /// Labels to add
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

/// Issue response from API.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueResponse {
    /// Issue ID
    pub id: i64,
    /// Issue number
    pub number: i64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: Option<String>,
    /// HTML URL to view the issue
    pub html_url: String,
    /// Issue state (open, closed)
    pub state: String,
    /// Labels on the issue
    #[serde(default)]
    pub labels: Vec<Label>,
    /// User who created the issue
    pub user: Option<IssueUser>,
    /// Creation timestamp
    pub created_at: Option<String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
}

/// Label on an issue.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    /// Label ID
    pub id: i64,
    /// Label name
    pub name: String,
    /// Label color (hex without #)
    pub color: String,
    /// Label description
    #[serde(default)]
    pub description: Option<String>,
}

/// User info for issues.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueUser {
    /// User ID
    pub id: i64,
    /// Username
    pub login: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

/// Request to add labels to an issue.
#[derive(Debug, Clone, Serialize)]
struct AddLabelsRequest {
    labels: Vec<String>,
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

    #[test]
    fn test_create_issue_request_serialization() {
        let request = CreateIssueRequest {
            title: "Bug: App crashes on startup".into(),
            body: "Steps to reproduce...".into(),
            labels: vec!["bug".into(), "P1".into()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"title\":\"Bug: App crashes on startup\""));
        assert!(json.contains("\"labels\":[\"bug\",\"P1\"]"));
    }

    #[test]
    fn test_create_issue_request_empty_labels() {
        let request = CreateIssueRequest {
            title: "Feature request".into(),
            body: "Add dark mode".into(),
            labels: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"title\":\"Feature request\""));
        assert!(!json.contains("\"labels\"")); // Empty vec should be skipped
    }

    #[test]
    fn test_issue_response_deserialize() {
        let json = r#"{
            "id": 12345,
            "number": 42,
            "title": "Bug report",
            "body": "Description here",
            "html_url": "https://github.com/owner/repo/issues/42",
            "state": "open",
            "labels": [
                {"id": 1, "name": "bug", "color": "d73a4a", "description": "Something is broken"}
            ],
            "user": {"id": 789, "login": "reporter", "avatar_url": "https://example.com/avatar.png"},
            "created_at": "2024-01-15T10:00:00Z",
            "updated_at": "2024-01-15T10:00:00Z"
        }"#;

        let issue: IssueResponse = serde_json::from_str(json).unwrap();
        assert_eq!(issue.id, 12345);
        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "Bug report");
        assert_eq!(issue.state, "open");
        assert_eq!(issue.labels.len(), 1);
        assert_eq!(issue.labels[0].name, "bug");
        assert!(issue.user.is_some());
        assert_eq!(issue.user.unwrap().login, "reporter");
    }

    #[test]
    fn test_issue_response_minimal() {
        let json = r#"{
            "id": 12345,
            "number": 42,
            "title": "Bug report",
            "html_url": "https://github.com/owner/repo/issues/42",
            "state": "open"
        }"#;

        let issue: IssueResponse = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 42);
        assert!(issue.body.is_none());
        assert!(issue.labels.is_empty());
        assert!(issue.user.is_none());
    }

    #[test]
    fn test_label_serialization() {
        let label = Label {
            id: 1,
            name: "bug".into(),
            color: "d73a4a".into(),
            description: Some("Something is broken".into()),
        };

        let json = serde_json::to_string(&label).unwrap();
        assert!(json.contains("\"name\":\"bug\""));
        assert!(json.contains("\"color\":\"d73a4a\""));
    }
}
