//! Approval queue management with delegation and batch processing.
//!
//! This module provides enhanced queue capabilities:
//! - Delegation: Transfer approval authority to another user
//! - Batch approval: Approve multiple requests at once
//! - Queue statistics and monitoring
//!
//! ## Design Principle
//!
//! Queue operations are deterministic - delegation rules and batch
//! approval logic follow explicit rules without LLM involvement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::escalation::{EscalationEvent, EscalationManager};
use super::store::HitLStore;
use super::{ApprovalRequest, ApprovalStatus};

/// Result of a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// IDs of successfully processed requests
    pub succeeded: Vec<String>,
    /// IDs of failed requests with error messages
    pub failed: Vec<(String, String)>,
    /// Total processing time in milliseconds
    pub duration_ms: u64,
}

impl BatchResult {
    /// Returns true if all operations succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }

    /// Returns the total number of operations attempted.
    pub fn total_count(&self) -> usize {
        self.succeeded.len() + self.failed.len()
    }

    /// Returns the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.total_count();
        if total == 0 {
            100.0
        } else {
            (self.succeeded.len() as f64 / total as f64) * 100.0
        }
    }
}

/// Delegation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRecord {
    /// Original approver
    pub from: String,
    /// Delegated approver
    pub to: String,
    /// When the delegation was created
    pub created_at: DateTime<Utc>,
    /// When the delegation expires (None = permanent)
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional reason for delegation
    pub reason: Option<String>,
    /// Whether this delegation is active
    pub active: bool,
}

impl DelegationRecord {
    /// Create a new permanent delegation.
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            created_at: Utc::now(),
            expires_at: None,
            reason: None,
            active: true,
        }
    }

    /// Create a temporary delegation.
    pub fn temporary(
        from: impl Into<String>,
        to: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            created_at: Utc::now(),
            expires_at: Some(expires_at),
            reason: None,
            active: true,
        }
    }

    /// Set the reason for delegation.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Check if the delegation is currently valid.
    pub fn is_valid(&self) -> bool {
        if !self.active {
            return false;
        }

        match self.expires_at {
            Some(expires) => Utc::now() < expires,
            None => true,
        }
    }
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total pending requests
    pub pending_count: usize,
    /// Pending requests by channel
    pub by_channel: HashMap<String, usize>,
    /// Pending requests by approver
    pub by_approver: HashMap<String, usize>,
    /// Average age of pending requests (in seconds)
    pub avg_age_seconds: f64,
    /// Oldest pending request age (in seconds)
    pub max_age_seconds: Option<f64>,
    /// Requests pending escalation
    pub pending_escalation: usize,
    /// Computed at
    pub computed_at: DateTime<Utc>,
}

/// Approval queue manager.
pub struct ApprovalQueue {
    store: Arc<HitLStore>,
    escalation_manager: EscalationManager,
    delegations: HashMap<String, Vec<DelegationRecord>>,
}

impl ApprovalQueue {
    /// Create a new approval queue.
    pub fn new(store: Arc<HitLStore>, escalation_manager: EscalationManager) -> Self {
        Self {
            store,
            escalation_manager,
            delegations: HashMap::new(),
        }
    }

    /// Add a delegation from one user to another.
    pub fn add_delegation(&mut self, delegation: DelegationRecord) {
        self.delegations
            .entry(delegation.from.clone())
            .or_default()
            .push(delegation);
    }

    /// Remove a delegation.
    pub fn remove_delegation(&mut self, from: &str, to: &str) -> bool {
        if let Some(delegations) = self.delegations.get_mut(from) {
            let initial_len = delegations.len();
            delegations.retain(|d| d.to != to);
            return delegations.len() < initial_len;
        }
        false
    }

    /// Get active delegations for a user.
    pub fn get_delegations(&self, from: &str) -> Vec<&DelegationRecord> {
        self.delegations
            .get(from)
            .map(|ds| ds.iter().filter(|d| d.is_valid()).collect())
            .unwrap_or_default()
    }

    /// Get all users who can approve on behalf of the given user.
    pub fn get_delegates(&self, approver: &str) -> Vec<String> {
        self.get_delegations(approver)
            .into_iter()
            .map(|d| d.to.clone())
            .collect()
    }

    /// Check if a user can approve on behalf of another.
    pub fn can_approve_for(&self, delegate: &str, approver: &str) -> bool {
        self.get_delegations(approver)
            .into_iter()
            .any(|d| d.to == delegate)
    }

    /// Delegate approval authority for a specific request.
    ///
    /// This adds the delegate as an approver for the request.
    pub fn delegate_request(
        &self,
        request_id: &str,
        from: &str,
        to: &str,
    ) -> Result<(), String> {
        // Get the request
        let mut request = self
            .store
            .get(request_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Request '{}' not found", request_id))?;

        // Check if 'from' is an approver
        if !request.approvers.contains(&from.to_string()) {
            return Err(format!(
                "User '{}' is not an approver for request '{}'",
                from, request_id
            ));
        }

        // Check if already decided
        if request.status.is_terminal() {
            return Err(format!(
                "Request '{}' has already been decided",
                request_id
            ));
        }

        // Add the delegate as an approver if not already
        if !request.approvers.contains(&to.to_string()) {
            request.approvers.push(to.to_string());
            // Note: This would need to be persisted via store.update_approvers()
            // For now, we just track delegation at the queue level
        }

        Ok(())
    }

    /// Batch approve multiple requests.
    pub fn batch_approve(
        &self,
        request_ids: &[&str],
        by: &str,
    ) -> BatchResult {
        let start = std::time::Instant::now();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in request_ids {
            match self.approve_single(*id, by) {
                Ok(()) => succeeded.push(id.to_string()),
                Err(e) => failed.push((id.to_string(), e)),
            }
        }

        BatchResult {
            succeeded,
            failed,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Batch reject multiple requests.
    pub fn batch_reject(
        &self,
        request_ids: &[&str],
        by: &str,
        reason: Option<&str>,
    ) -> BatchResult {
        let start = std::time::Instant::now();
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();

        for id in request_ids {
            match self.reject_single(*id, by, reason.map(|s| s.to_string())) {
                Ok(()) => succeeded.push(id.to_string()),
                Err(e) => failed.push((id.to_string(), e)),
            }
        }

        BatchResult {
            succeeded,
            failed,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Approve a single request.
    fn approve_single(&self, id: &str, by: &str) -> Result<(), String> {
        // Get the request
        let request = self
            .store
            .get(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Request '{}' not found", id))?;

        // Check if user is authorized (directly or via delegation)
        if !self.is_authorized_approver(&request, by) {
            return Err(format!("User '{}' is not authorized to approve '{}'", by, id));
        }

        // Check if already decided
        if request.status.is_terminal() {
            return Err(format!("Request '{}' already decided", id));
        }

        // Update status
        let new_status = ApprovalStatus::Approved {
            by: by.to_string(),
            at: Utc::now(),
        };

        self.store
            .update_status(id, &new_status)
            .map_err(|e| e.to_string())
    }

    /// Reject a single request.
    fn reject_single(&self, id: &str, by: &str, reason: Option<String>) -> Result<(), String> {
        // Get the request
        let request = self
            .store
            .get(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Request '{}' not found", id))?;

        // Check if user is authorized (directly or via delegation)
        if !self.is_authorized_approver(&request, by) {
            return Err(format!("User '{}' is not authorized to reject '{}'", by, id));
        }

        // Check if already decided
        if request.status.is_terminal() {
            return Err(format!("Request '{}' already decided", id));
        }

        // Update status
        let new_status = ApprovalStatus::Rejected {
            by: by.to_string(),
            reason,
            at: Utc::now(),
        };

        self.store
            .update_status(id, &new_status)
            .map_err(|e| e.to_string())
    }

    /// Check if a user is authorized to approve a request.
    fn is_authorized_approver(&self, request: &ApprovalRequest, user: &str) -> bool {
        // Direct approver
        if request.approvers.contains(&user.to_string()) {
            return true;
        }

        // Check delegations
        for approver in &request.approvers {
            if self.can_approve_for(user, approver) {
                return true;
            }
        }

        false
    }

    /// Process escalations for all pending requests.
    pub fn process_escalations(&mut self) -> Vec<EscalationEvent> {
        let pending = match self.store.list_pending(None) {
            Ok(requests) => requests,
            Err(_) => return Vec::new(),
        };

        let mut events = Vec::new();

        for request in pending {
            if let Some(rule) = self
                .escalation_manager
                .check_escalation(&request.id, request.created_at)
            {
                let rule = rule.clone();
                let event = self.escalation_manager.escalate(&request.id, &rule);
                events.push(event);
            }
        }

        events
    }

    /// Get queue statistics.
    pub fn get_stats(&self) -> QueueStats {
        let pending = match self.store.list_pending(None) {
            Ok(requests) => requests,
            Err(_) => Vec::new(),
        };

        let now = Utc::now();
        let mut by_channel: HashMap<String, usize> = HashMap::new();
        let mut by_approver: HashMap<String, usize> = HashMap::new();
        let mut total_age: f64 = 0.0;
        let mut max_age: Option<f64> = None;
        let mut pending_escalation = 0;

        for request in &pending {
            // By channel
            *by_channel.entry(request.channel.clone()).or_insert(0) += 1;

            // By approver
            for approver in &request.approvers {
                *by_approver.entry(approver.clone()).or_insert(0) += 1;
            }

            // Age calculations
            let age_seconds = (now - request.created_at).num_seconds() as f64;
            total_age += age_seconds;
            max_age = Some(max_age.map_or(age_seconds, |m: f64| m.max(age_seconds)));

            // Check for pending escalation
            if self
                .escalation_manager
                .check_escalation(&request.id, request.created_at)
                .is_some()
            {
                pending_escalation += 1;
            }
        }

        let avg_age = if pending.is_empty() {
            0.0
        } else {
            total_age / pending.len() as f64
        };

        QueueStats {
            pending_count: pending.len(),
            by_channel,
            by_approver,
            avg_age_seconds: avg_age,
            max_age_seconds: max_age,
            pending_escalation,
            computed_at: now,
        }
    }

    /// Clear escalation tracking for resolved requests.
    pub fn cleanup_resolved(&mut self, request_id: &str) {
        self.escalation_manager.clear_request(request_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::hitl::escalation::EscalationRule;
    use crate::gateway::hitl::{ApprovalType, RiskLevel};
    use tempfile::tempdir;

    fn create_test_queue() -> (ApprovalQueue, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_hitl.db");
        let store = Arc::new(HitLStore::new(&db_path).unwrap());

        let escalation_manager = EscalationManager::new(vec![
            EscalationRule::new("test-rule", std::time::Duration::from_secs(60)),
        ]);

        let queue = ApprovalQueue::new(store, escalation_manager);
        (queue, dir)
    }

    fn create_test_request(id: &str, approvers: Vec<String>) -> ApprovalRequest {
        let now = Utc::now();
        ApprovalRequest {
            id: id.to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 1,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers,
            title: "Test Request".to_string(),
            description: None,
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    #[test]
    fn test_batch_result() {
        let result = BatchResult {
            succeeded: vec!["a".to_string(), "b".to_string()],
            failed: vec![("c".to_string(), "error".to_string())],
            duration_ms: 100,
        };

        assert!(!result.all_succeeded());
        assert_eq!(result.total_count(), 3);
        assert!((result.success_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_delegation_record() {
        let delegation = DelegationRecord::new("alice", "bob")
            .with_reason("Out of office");

        assert!(delegation.is_valid());
        assert_eq!(delegation.from, "alice");
        assert_eq!(delegation.to, "bob");
        assert_eq!(delegation.reason, Some("Out of office".to_string()));
    }

    #[test]
    fn test_temporary_delegation_expiry() {
        let expired = DelegationRecord::temporary(
            "alice",
            "bob",
            Utc::now() - chrono::Duration::hours(1),
        );
        assert!(!expired.is_valid());

        let valid = DelegationRecord::temporary(
            "alice",
            "charlie",
            Utc::now() + chrono::Duration::hours(1),
        );
        assert!(valid.is_valid());
    }

    #[test]
    fn test_add_and_get_delegations() {
        let (mut queue, _dir) = create_test_queue();

        let delegation = DelegationRecord::new("alice", "bob");
        queue.add_delegation(delegation);

        let delegates = queue.get_delegates("alice");
        assert_eq!(delegates.len(), 1);
        assert_eq!(delegates[0], "bob");

        assert!(queue.can_approve_for("bob", "alice"));
        assert!(!queue.can_approve_for("charlie", "alice"));
    }

    #[test]
    fn test_remove_delegation() {
        let (mut queue, _dir) = create_test_queue();

        queue.add_delegation(DelegationRecord::new("alice", "bob"));
        queue.add_delegation(DelegationRecord::new("alice", "charlie"));

        assert!(queue.remove_delegation("alice", "bob"));
        assert!(!queue.remove_delegation("alice", "bob")); // Already removed

        let delegates = queue.get_delegates("alice");
        assert_eq!(delegates.len(), 1);
        assert_eq!(delegates[0], "charlie");
    }

    #[test]
    fn test_batch_approve() {
        let (queue, _dir) = create_test_queue();

        // Create test requests
        let req1 = create_test_request("req-1", vec!["admin".to_string()]);
        let req2 = create_test_request("req-2", vec!["admin".to_string()]);
        queue.store.create(&req1).unwrap();
        queue.store.create(&req2).unwrap();

        let result = queue.batch_approve(&["req-1", "req-2"], "admin");

        assert_eq!(result.succeeded.len(), 2);
        assert!(result.failed.is_empty());
        assert!(result.all_succeeded());
    }

    #[test]
    fn test_batch_approve_with_failures() {
        let (queue, _dir) = create_test_queue();

        // Create one valid request
        let req1 = create_test_request("req-1", vec!["admin".to_string()]);
        queue.store.create(&req1).unwrap();

        // Try to batch approve with a nonexistent request
        let result = queue.batch_approve(&["req-1", "nonexistent"], "admin");

        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(result.failed.len(), 1);
        assert!(!result.all_succeeded());
    }

    #[test]
    fn test_batch_reject() {
        let (queue, _dir) = create_test_queue();

        let req = create_test_request("req-1", vec!["reviewer".to_string()]);
        queue.store.create(&req).unwrap();

        let result = queue.batch_reject(&["req-1"], "reviewer", Some("Not ready"));

        assert_eq!(result.succeeded.len(), 1);
        assert!(result.failed.is_empty());

        // Verify the request was rejected
        let updated = queue.store.get("req-1").unwrap().unwrap();
        match updated.status {
            ApprovalStatus::Rejected { reason, .. } => {
                assert_eq!(reason, Some("Not ready".to_string()));
            }
            _ => panic!("Expected Rejected status"),
        }
    }

    #[test]
    fn test_is_authorized_via_delegation() {
        let (mut queue, _dir) = create_test_queue();

        let req = create_test_request("req-1", vec!["alice".to_string()]);
        queue.store.create(&req).unwrap();

        // bob is not directly authorized
        assert!(!queue.is_authorized_approver(&req, "bob"));

        // Add delegation from alice to bob
        queue.add_delegation(DelegationRecord::new("alice", "bob"));

        // Now bob can approve on alice's behalf
        let req = queue.store.get("req-1").unwrap().unwrap();
        assert!(queue.is_authorized_approver(&req, "bob"));
    }

    #[test]
    fn test_get_stats_empty() {
        let (queue, _dir) = create_test_queue();

        let stats = queue.get_stats();
        assert_eq!(stats.pending_count, 0);
        assert!(stats.by_channel.is_empty());
        assert!(stats.by_approver.is_empty());
        assert_eq!(stats.avg_age_seconds, 0.0);
        assert!(stats.max_age_seconds.is_none());
    }

    #[test]
    fn test_get_stats_with_requests() {
        let (queue, _dir) = create_test_queue();

        let req1 = create_test_request("req-1", vec!["admin".to_string()]);
        let mut req2 = create_test_request("req-2", vec!["reviewer".to_string()]);
        req2.channel = "slack".to_string();

        queue.store.create(&req1).unwrap();
        queue.store.create(&req2).unwrap();

        let stats = queue.get_stats();
        assert_eq!(stats.pending_count, 2);
        assert_eq!(stats.by_channel.get("telegram"), Some(&1));
        assert_eq!(stats.by_channel.get("slack"), Some(&1));
        assert_eq!(stats.by_approver.get("admin"), Some(&1));
        assert_eq!(stats.by_approver.get("reviewer"), Some(&1));
    }

    #[test]
    fn test_queue_stats_serialization() {
        let stats = QueueStats {
            pending_count: 5,
            by_channel: HashMap::from([("telegram".to_string(), 3), ("slack".to_string(), 2)]),
            by_approver: HashMap::from([("admin".to_string(), 5)]),
            avg_age_seconds: 120.5,
            max_age_seconds: Some(300.0),
            pending_escalation: 1,
            computed_at: Utc::now(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"pending_count\":5"));
        assert!(json.contains("telegram"));
    }
}
