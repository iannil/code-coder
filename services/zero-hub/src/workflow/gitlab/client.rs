//! GitLab API client for code review operations.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Client
// ============================================================================

/// GitLab API client.
pub struct GitLabClient {
    client: reqwest::Client,
    base_url: String,
}

impl GitLabClient {
    /// Create a new GitLab client.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        Self::with_base_url(token, "https://gitlab.com/api/v4")
    }

    /// Create a client with custom base URL (for self-hosted GitLab).
    pub fn with_base_url(token: impl Into<String>, base_url: impl Into<String>) -> Result<Self> {
        let token = token.into();
        let mut headers = HeaderMap::new();
        headers.insert(
            "PRIVATE-TOKEN",
            HeaderValue::from_str(&token)?,
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// Get merge request details.
    pub async fn get_merge_request(&self, project_id: i64, mr_iid: i64) -> Result<MergeRequest> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}",
            self.base_url, project_id, mr_iid
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch merge request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse merge request response")
    }

    /// Get changes (diff) for a merge request.
    pub async fn get_merge_request_changes(
        &self,
        project_id: i64,
        mr_iid: i64,
    ) -> Result<MergeRequestChanges> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/changes",
            self.base_url, project_id, mr_iid
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch MR changes")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse MR changes response")
    }

    /// Get diffs for a merge request.
    pub async fn get_merge_request_diffs(
        &self,
        project_id: i64,
        mr_iid: i64,
    ) -> Result<Vec<MergeRequestDiff>> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/diffs",
            self.base_url, project_id, mr_iid
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch MR diffs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse MR diffs response")
    }

    /// Create a note (comment) on a merge request.
    pub async fn create_note(
        &self,
        project_id: i64,
        mr_iid: i64,
        body: &str,
    ) -> Result<Note> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/notes",
            self.base_url, project_id, mr_iid
        );

        let request = CreateNoteRequest {
            body: body.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to create note")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse note response")
    }

    /// Create a discussion (thread) on a specific line.
    pub async fn create_discussion(
        &self,
        project_id: i64,
        mr_iid: i64,
        request: &CreateDiscussionRequest,
    ) -> Result<Discussion> {
        let url = format!(
            "{}/projects/{}/merge_requests/{}/discussions",
            self.base_url, project_id, mr_iid
        );

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to create discussion")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab API error {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse discussion response")
    }
}

// ============================================================================
// API Types
// ============================================================================

/// Merge request details from API.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequest {
    pub id: i64,
    pub iid: i64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub source_branch: String,
    pub target_branch: String,
    pub web_url: String,
    pub diff_refs: Option<DiffRefs>,
}

/// Diff refs (head, base, start SHAs).
#[derive(Debug, Clone, Deserialize)]
pub struct DiffRefs {
    pub base_sha: String,
    pub head_sha: String,
    pub start_sha: String,
}

/// Merge request with changes.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequestChanges {
    pub id: i64,
    pub iid: i64,
    pub changes: Vec<MergeRequestDiff>,
}

/// Diff for a single file.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequestDiff {
    pub old_path: String,
    pub new_path: String,
    pub a_mode: Option<String>,
    pub b_mode: Option<String>,
    pub new_file: bool,
    pub renamed_file: bool,
    pub deleted_file: bool,
    pub diff: String,
}

/// Note (comment) on a merge request.
#[derive(Debug, Clone, Deserialize)]
pub struct Note {
    pub id: i64,
    pub body: String,
    pub author: NoteAuthor,
}

/// Note author.
#[derive(Debug, Clone, Deserialize)]
pub struct NoteAuthor {
    pub id: i64,
    pub username: String,
    pub name: String,
}

/// Request to create a note.
#[derive(Debug, Serialize)]
struct CreateNoteRequest {
    body: String,
}

/// Request to create a discussion.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDiscussionRequest {
    /// Discussion body
    pub body: String,
    /// Position for line comments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<DiscussionPosition>,
}

/// Position for a line comment.
#[derive(Debug, Clone, Serialize)]
pub struct DiscussionPosition {
    pub base_sha: String,
    pub start_sha: String,
    pub head_sha: String,
    pub position_type: String,
    pub old_path: Option<String>,
    pub new_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line: Option<i64>,
}

/// Discussion response.
#[derive(Debug, Clone, Deserialize)]
pub struct Discussion {
    pub id: String,
    pub notes: Vec<Note>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_note_serialization() {
        let request = CreateNoteRequest {
            body: "LGTM! Great work.".into(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"body\":\"LGTM! Great work.\""));
    }

    #[test]
    fn test_merge_request_diff_deserialize() {
        let json = r#"{
            "old_path": "src/old.rs",
            "new_path": "src/new.rs",
            "new_file": false,
            "renamed_file": true,
            "deleted_file": false,
            "diff": "@@ -1,5 +1,10 @@\n+new line"
        }"#;

        let diff: MergeRequestDiff = serde_json::from_str(json).unwrap();
        assert_eq!(diff.old_path, "src/old.rs");
        assert_eq!(diff.new_path, "src/new.rs");
        assert!(diff.renamed_file);
    }

    #[test]
    fn test_discussion_position_serialization() {
        let position = DiscussionPosition {
            base_sha: "abc123".into(),
            start_sha: "def456".into(),
            head_sha: "ghi789".into(),
            position_type: "text".into(),
            old_path: None,
            new_path: "src/main.rs".into(),
            old_line: None,
            new_line: Some(42),
        };

        let json = serde_json::to_string(&position).unwrap();
        assert!(json.contains("\"new_line\":42"));
        assert!(!json.contains("\"old_line\"")); // None should be skipped
    }
}
