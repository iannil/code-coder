//! GitLab integration for Zero Workflow.
//!
//! Provides webhook event parsing and API client for automated code review.

mod client;

pub use client::{GitLabClient, MergeRequest, MergeRequestDiff, Note};

use serde::Deserialize;

// ============================================================================
// Webhook Event Types
// ============================================================================

/// GitLab merge request webhook event.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequestEvent {
    /// Event type
    pub event_type: Option<String>,
    /// Object kind (should be "merge_request")
    pub object_kind: String,
    /// User who triggered the event
    pub user: User,
    /// Project info
    pub project: Project,
    /// Object attributes (MR details)
    pub object_attributes: MergeRequestAttributes,
    /// Changes info (what changed in this update)
    pub changes: Option<Changes>,
}

/// Merge request attributes from webhook.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequestAttributes {
    /// MR ID (internal)
    pub id: i64,
    /// MR IID (project-scoped)
    pub iid: i64,
    /// MR title
    pub title: String,
    /// MR description
    pub description: Option<String>,
    /// MR state (opened, closed, merged)
    pub state: String,
    /// Is it a draft/WIP MR?
    #[serde(default)]
    pub draft: bool,
    /// Also check work_in_progress for older GitLab versions
    #[serde(default)]
    pub work_in_progress: bool,
    /// Source branch
    pub source_branch: String,
    /// Target branch
    pub target_branch: String,
    /// Source project ID
    pub source_project_id: i64,
    /// Target project ID
    pub target_project_id: i64,
    /// Action (open, update, merge, close, etc.)
    pub action: Option<String>,
    /// Web URL
    pub url: String,
}

/// Project info.
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    /// Project ID
    pub id: i64,
    /// Project name
    pub name: String,
    /// Full path (namespace/project)
    pub path_with_namespace: String,
    /// Web URL
    pub web_url: String,
    /// Git HTTP URL
    pub git_http_url: String,
}

/// User info.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    /// User ID
    pub id: i64,
    /// Username
    pub username: String,
    /// Display name
    pub name: String,
}

/// Changes in the webhook event.
#[derive(Debug, Clone, Deserialize)]
pub struct Changes {
    /// Title changed
    pub title: Option<ChangeValue>,
    /// Description changed
    pub description: Option<ChangeValue>,
    /// State changed
    pub state: Option<ChangeValue>,
}

/// A change value (previous and current).
#[derive(Debug, Clone, Deserialize)]
pub struct ChangeValue {
    pub previous: Option<serde_json::Value>,
    pub current: Option<serde_json::Value>,
}

impl MergeRequestEvent {
    /// Check if this event should trigger a code review.
    pub fn should_review(&self) -> bool {
        let action = self
            .object_attributes
            .action
            .as_deref()
            .unwrap_or("");

        // Review on open or update (new commits pushed)
        let is_reviewable_action = matches!(action, "open" | "reopen" | "update");

        // Not a draft/WIP
        let is_not_draft = !self.object_attributes.draft && !self.object_attributes.work_in_progress;

        // State is opened
        let is_open = self.object_attributes.state == "opened";

        is_reviewable_action && is_not_draft && is_open
    }

    /// Get the project path.
    pub fn project_path(&self) -> &str {
        &self.project.path_with_namespace
    }

    /// Get the MR IID.
    pub fn mr_iid(&self) -> i64 {
        self.object_attributes.iid
    }

    /// Get the project ID.
    pub fn project_id(&self) -> i64 {
        self.project.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_request_event_deserialize() {
        let json = r#"{
            "object_kind": "merge_request",
            "user": {
                "id": 1,
                "username": "user",
                "name": "Test User"
            },
            "project": {
                "id": 123,
                "name": "project",
                "path_with_namespace": "group/project",
                "web_url": "https://gitlab.com/group/project",
                "git_http_url": "https://gitlab.com/group/project.git"
            },
            "object_attributes": {
                "id": 456,
                "iid": 1,
                "title": "Test MR",
                "description": "Test description",
                "state": "opened",
                "draft": false,
                "source_branch": "feature",
                "target_branch": "main",
                "source_project_id": 123,
                "target_project_id": 123,
                "action": "open",
                "url": "https://gitlab.com/group/project/-/merge_requests/1"
            }
        }"#;

        let event: MergeRequestEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.object_kind, "merge_request");
        assert_eq!(event.object_attributes.iid, 1);
        assert!(event.should_review());
    }

    #[test]
    fn test_draft_mr_should_not_review() {
        let json = r#"{
            "object_kind": "merge_request",
            "user": { "id": 1, "username": "user", "name": "User" },
            "project": {
                "id": 123,
                "name": "project",
                "path_with_namespace": "group/project",
                "web_url": "https://gitlab.com/group/project",
                "git_http_url": "https://gitlab.com/group/project.git"
            },
            "object_attributes": {
                "id": 456,
                "iid": 1,
                "title": "Draft: Test MR",
                "state": "opened",
                "draft": true,
                "source_branch": "feature",
                "target_branch": "main",
                "source_project_id": 123,
                "target_project_id": 123,
                "action": "open",
                "url": "https://gitlab.com/group/project/-/merge_requests/1"
            }
        }"#;

        let event: MergeRequestEvent = serde_json::from_str(json).unwrap();
        assert!(!event.should_review());
    }
}
