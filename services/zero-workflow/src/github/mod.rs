//! GitHub integration for Zero Workflow.
//!
//! Provides webhook event parsing and API client for automated code review.

mod client;

pub use client::{GitHubClient, PullRequest, PullRequestFile, ReviewComment};

use serde::{Deserialize, Serialize};

// ============================================================================
// Webhook Event Types
// ============================================================================

/// GitHub pull request webhook event.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestEvent {
    /// Action: opened, synchronize, reopened, closed, etc.
    pub action: String,
    /// Pull request number
    pub number: i64,
    /// Pull request details
    pub pull_request: PullRequestPayload,
    /// Repository info
    pub repository: Repository,
    /// Sender info
    pub sender: User,
}

/// Pull request payload from webhook.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequestPayload {
    /// PR ID
    pub id: i64,
    /// PR number
    pub number: i64,
    /// PR title
    pub title: String,
    /// PR body/description
    pub body: Option<String>,
    /// PR state (open, closed)
    pub state: String,
    /// Is it a draft PR?
    #[serde(default)]
    pub draft: bool,
    /// Head branch info
    pub head: BranchRef,
    /// Base branch info
    pub base: BranchRef,
    /// HTML URL
    pub html_url: String,
    /// Diff URL
    pub diff_url: String,
    /// Number of changed files
    pub changed_files: Option<i64>,
    /// Number of additions
    pub additions: Option<i64>,
    /// Number of deletions
    pub deletions: Option<i64>,
}

/// Branch reference.
#[derive(Debug, Clone, Deserialize)]
pub struct BranchRef {
    /// Branch name
    #[serde(rename = "ref")]
    pub ref_name: String,
    /// SHA
    pub sha: String,
    /// Repository
    pub repo: Option<Repository>,
}

/// Repository info.
#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    /// Repository ID
    pub id: i64,
    /// Repository name
    pub name: String,
    /// Full name (owner/repo)
    pub full_name: String,
    /// Clone URL
    pub clone_url: String,
    /// Default branch
    pub default_branch: Option<String>,
}

/// User info.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    /// User ID
    pub id: i64,
    /// Login/username
    pub login: String,
}

// ============================================================================
// Review Types
// ============================================================================

/// Code review result ready for posting.
#[derive(Debug, Clone, Serialize)]
pub struct CodeReviewResult {
    /// Overall summary
    pub summary: String,
    /// Review status: APPROVE, REQUEST_CHANGES, COMMENT
    pub event: ReviewEvent,
    /// Line-specific comments
    pub comments: Vec<ReviewComment>,
}

/// Review event type.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    Approve,
    RequestChanges,
    Comment,
}

impl PullRequestEvent {
    /// Check if this event should trigger a code review.
    pub fn should_review(&self) -> bool {
        // Review on opened or synchronized (new commits pushed)
        matches!(self.action.as_str(), "opened" | "synchronize" | "reopened")
            && !self.pull_request.draft
    }

    /// Get the repository full name.
    pub fn repo_full_name(&self) -> &str {
        &self.repository.full_name
    }

    /// Get the PR number.
    pub fn pr_number(&self) -> i64 {
        self.number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_request_event_deserialize() {
        let json = r#"{
            "action": "opened",
            "number": 1,
            "pull_request": {
                "id": 123,
                "number": 1,
                "title": "Test PR",
                "body": "Test description",
                "state": "open",
                "draft": false,
                "head": {
                    "ref": "feature-branch",
                    "sha": "abc123"
                },
                "base": {
                    "ref": "main",
                    "sha": "def456"
                },
                "html_url": "https://github.com/owner/repo/pull/1",
                "diff_url": "https://github.com/owner/repo/pull/1.diff"
            },
            "repository": {
                "id": 456,
                "name": "repo",
                "full_name": "owner/repo",
                "clone_url": "https://github.com/owner/repo.git"
            },
            "sender": {
                "id": 789,
                "login": "user"
            }
        }"#;

        let event: PullRequestEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.action, "opened");
        assert_eq!(event.number, 1);
        assert!(event.should_review());
    }

    #[test]
    fn test_draft_pr_should_not_review() {
        let json = r#"{
            "action": "opened",
            "number": 1,
            "pull_request": {
                "id": 123,
                "number": 1,
                "title": "Draft PR",
                "state": "open",
                "draft": true,
                "head": { "ref": "feature", "sha": "abc" },
                "base": { "ref": "main", "sha": "def" },
                "html_url": "https://github.com/owner/repo/pull/1",
                "diff_url": "https://github.com/owner/repo/pull/1.diff"
            },
            "repository": {
                "id": 456,
                "name": "repo",
                "full_name": "owner/repo",
                "clone_url": "https://github.com/owner/repo.git"
            },
            "sender": { "id": 789, "login": "user" }
        }"#;

        let event: PullRequestEvent = serde_json::from_str(json).unwrap();
        assert!(!event.should_review());
    }
}
