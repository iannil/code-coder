//! Escalation rules for approval requests.
//!
//! This module provides time-based escalation rules that automatically
//! escalate stale approval requests to designated users or channels.
//!
//! ## Design Principle
//!
//! Escalation follows deterministic rules based on time thresholds.
//! No LLM reasoning is required - just rule matching and action execution.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Escalation rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    /// Unique name for this rule
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Duration in seconds after which to trigger escalation
    pub trigger_after_secs: u64,
    /// Users to escalate to
    #[serde(default)]
    pub escalate_to: Vec<String>,
    /// Channels to notify (e.g., "telegram", "slack")
    #[serde(default)]
    pub notify_channels: Vec<String>,
    /// Priority level (higher = more urgent)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Whether to add escalation targets as approvers
    #[serde(default)]
    pub add_as_approvers: bool,
    /// Maximum number of escalations for a single request
    #[serde(default = "default_max_escalations")]
    pub max_escalations: u32,
}

fn default_priority() -> u8 {
    1
}

fn default_max_escalations() -> u32 {
    3
}

impl EscalationRule {
    /// Create a new escalation rule.
    pub fn new(name: impl Into<String>, trigger_after: std::time::Duration) -> Self {
        Self {
            name: name.into(),
            description: None,
            trigger_after_secs: trigger_after.as_secs(),
            escalate_to: Vec::new(),
            notify_channels: Vec::new(),
            priority: default_priority(),
            add_as_approvers: false,
            max_escalations: default_max_escalations(),
        }
    }

    /// Get the trigger duration.
    pub fn trigger_after(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.trigger_after_secs)
    }

    /// Add users to escalate to.
    pub fn with_users(mut self, users: Vec<String>) -> Self {
        self.escalate_to = users;
        self
    }

    /// Add channels to notify.
    pub fn with_channels(mut self, channels: Vec<String>) -> Self {
        self.notify_channels = channels;
        self
    }

    /// Set priority level.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set whether to add escalation targets as approvers.
    pub fn add_targets_as_approvers(mut self) -> Self {
        self.add_as_approvers = true;
        self
    }
}

/// Record of an escalation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationEvent {
    /// Request ID that was escalated
    pub request_id: String,
    /// Rule that triggered the escalation
    pub rule_name: String,
    /// Users who were notified
    pub escalated_to: Vec<String>,
    /// Channels that were notified
    pub notified_channels: Vec<String>,
    /// Timestamp of escalation
    pub escalated_at: DateTime<Utc>,
    /// Current escalation level (1 = first escalation, 2 = second, etc.)
    pub escalation_level: u32,
}

/// Tracks escalation state for requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EscalationTracker {
    /// Map of request_id -> escalation count
    escalation_counts: HashMap<String, u32>,
    /// Map of request_id -> last escalation time
    last_escalation: HashMap<String, DateTime<Utc>>,
    /// Map of request_id -> escalated rule names
    applied_rules: HashMap<String, Vec<String>>,
}

impl EscalationTracker {
    /// Create a new escalation tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an escalation for a request.
    pub fn record_escalation(&mut self, request_id: &str, rule_name: &str) {
        let count = self.escalation_counts.entry(request_id.to_string()).or_insert(0);
        *count += 1;

        self.last_escalation.insert(request_id.to_string(), Utc::now());

        self.applied_rules
            .entry(request_id.to_string())
            .or_default()
            .push(rule_name.to_string());
    }

    /// Get the current escalation count for a request.
    pub fn get_count(&self, request_id: &str) -> u32 {
        self.escalation_counts.get(request_id).copied().unwrap_or(0)
    }

    /// Get the last escalation time for a request.
    pub fn get_last_escalation(&self, request_id: &str) -> Option<DateTime<Utc>> {
        self.last_escalation.get(request_id).copied()
    }

    /// Check if a rule has already been applied to a request.
    pub fn has_rule_applied(&self, request_id: &str, rule_name: &str) -> bool {
        self.applied_rules
            .get(request_id)
            .map(|rules| rules.contains(&rule_name.to_string()))
            .unwrap_or(false)
    }

    /// Clear tracking for a request (e.g., when resolved).
    pub fn clear_request(&mut self, request_id: &str) {
        self.escalation_counts.remove(request_id);
        self.last_escalation.remove(request_id);
        self.applied_rules.remove(request_id);
    }
}

/// Escalation manager that evaluates rules against pending requests.
pub struct EscalationManager {
    rules: Vec<EscalationRule>,
    tracker: EscalationTracker,
}

impl EscalationManager {
    /// Create a new escalation manager with the given rules.
    pub fn new(rules: Vec<EscalationRule>) -> Self {
        Self {
            rules,
            tracker: EscalationTracker::new(),
        }
    }

    /// Add a rule to the manager.
    pub fn add_rule(&mut self, rule: EscalationRule) {
        self.rules.push(rule);
    }

    /// Get rules sorted by priority (descending).
    pub fn rules_by_priority(&self) -> Vec<&EscalationRule> {
        let mut rules: Vec<_> = self.rules.iter().collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    /// Check if a request should be escalated based on creation time.
    ///
    /// Returns the matching rule if escalation is needed.
    pub fn check_escalation(
        &self,
        request_id: &str,
        created_at: DateTime<Utc>,
    ) -> Option<&EscalationRule> {
        let now = Utc::now();
        let age = now - created_at;

        // Get current escalation count
        let current_count = self.tracker.get_count(request_id);

        // Find the first applicable rule (highest priority first)
        for rule in self.rules_by_priority() {
            // Skip if already at max escalations for this rule
            if current_count >= rule.max_escalations {
                continue;
            }

            // Skip if this specific rule was already applied
            if self.tracker.has_rule_applied(request_id, &rule.name) {
                continue;
            }

            // Check time threshold
            let threshold = Duration::from_std(rule.trigger_after())
                .unwrap_or_else(|_| Duration::hours(1));

            if age > threshold {
                return Some(rule);
            }
        }

        None
    }

    /// Execute escalation for a request.
    ///
    /// Returns the escalation event if successful.
    pub fn escalate(
        &mut self,
        request_id: &str,
        rule: &EscalationRule,
    ) -> EscalationEvent {
        // Record the escalation
        self.tracker.record_escalation(request_id, &rule.name);
        let level = self.tracker.get_count(request_id);

        EscalationEvent {
            request_id: request_id.to_string(),
            rule_name: rule.name.clone(),
            escalated_to: rule.escalate_to.clone(),
            notified_channels: rule.notify_channels.clone(),
            escalated_at: Utc::now(),
            escalation_level: level,
        }
    }

    /// Clear escalation tracking for a request.
    pub fn clear_request(&mut self, request_id: &str) {
        self.tracker.clear_request(request_id);
    }

    /// Get the escalation tracker (for serialization/persistence).
    pub fn tracker(&self) -> &EscalationTracker {
        &self.tracker
    }

    /// Restore tracker state (for loading from persistence).
    pub fn restore_tracker(&mut self, tracker: EscalationTracker) {
        self.tracker = tracker;
    }
}

/// Default escalation rules for common scenarios.
pub fn default_rules() -> Vec<EscalationRule> {
    vec![
        EscalationRule::new("standard-30m", std::time::Duration::from_secs(30 * 60))
            .with_channels(vec!["telegram".to_string()])
            .with_priority(1),
        EscalationRule::new("urgent-1h", std::time::Duration::from_secs(60 * 60))
            .with_users(vec!["manager".to_string()])
            .with_channels(vec!["telegram".to_string(), "email".to_string()])
            .with_priority(2),
        EscalationRule::new("critical-2h", std::time::Duration::from_secs(2 * 60 * 60))
            .with_users(vec!["admin".to_string(), "cto".to_string()])
            .with_channels(vec!["telegram".to_string(), "slack".to_string(), "email".to_string()])
            .with_priority(3)
            .add_targets_as_approvers(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_rule_builder() {
        let rule = EscalationRule::new("test", std::time::Duration::from_secs(3600))
            .with_users(vec!["admin".to_string()])
            .with_channels(vec!["slack".to_string()])
            .with_priority(5)
            .add_targets_as_approvers();

        assert_eq!(rule.name, "test");
        assert_eq!(rule.escalate_to, vec!["admin"]);
        assert_eq!(rule.notify_channels, vec!["slack"]);
        assert_eq!(rule.priority, 5);
        assert!(rule.add_as_approvers);
    }

    #[test]
    fn test_escalation_tracker() {
        let mut tracker = EscalationTracker::new();

        assert_eq!(tracker.get_count("req-1"), 0);
        assert!(!tracker.has_rule_applied("req-1", "rule-a"));

        tracker.record_escalation("req-1", "rule-a");
        assert_eq!(tracker.get_count("req-1"), 1);
        assert!(tracker.has_rule_applied("req-1", "rule-a"));
        assert!(!tracker.has_rule_applied("req-1", "rule-b"));
        assert!(tracker.get_last_escalation("req-1").is_some());

        tracker.record_escalation("req-1", "rule-b");
        assert_eq!(tracker.get_count("req-1"), 2);
        assert!(tracker.has_rule_applied("req-1", "rule-b"));

        tracker.clear_request("req-1");
        assert_eq!(tracker.get_count("req-1"), 0);
        assert!(!tracker.has_rule_applied("req-1", "rule-a"));
    }

    #[test]
    fn test_escalation_manager_check() {
        let rules = vec![
            EscalationRule::new("quick", std::time::Duration::from_secs(60)),
        ];
        let manager = EscalationManager::new(rules);

        // Request created 2 minutes ago should trigger escalation
        let created_2m_ago = Utc::now() - Duration::minutes(2);
        assert!(manager.check_escalation("req-1", created_2m_ago).is_some());

        // Request created 30 seconds ago should not trigger
        let created_30s_ago = Utc::now() - Duration::seconds(30);
        assert!(manager.check_escalation("req-2", created_30s_ago).is_none());
    }

    #[test]
    fn test_escalation_manager_escalate() {
        let rules = vec![
            EscalationRule::new("test-rule", std::time::Duration::from_secs(60))
                .with_users(vec!["admin".to_string()]),
        ];
        let mut manager = EscalationManager::new(rules);

        let event = manager.escalate("req-1", &manager.rules[0].clone());

        assert_eq!(event.request_id, "req-1");
        assert_eq!(event.rule_name, "test-rule");
        assert_eq!(event.escalated_to, vec!["admin"]);
        assert_eq!(event.escalation_level, 1);

        // Check that the rule is now applied
        assert!(manager.tracker.has_rule_applied("req-1", "test-rule"));
    }

    #[test]
    fn test_rules_by_priority() {
        let rules = vec![
            EscalationRule::new("low", std::time::Duration::from_secs(60)).with_priority(1),
            EscalationRule::new("high", std::time::Duration::from_secs(60)).with_priority(3),
            EscalationRule::new("medium", std::time::Duration::from_secs(60)).with_priority(2),
        ];
        let manager = EscalationManager::new(rules);

        let sorted = manager.rules_by_priority();
        assert_eq!(sorted[0].name, "high");
        assert_eq!(sorted[1].name, "medium");
        assert_eq!(sorted[2].name, "low");
    }

    #[test]
    fn test_max_escalations() {
        // Create multiple rules with different names but same threshold
        // Each rule can only be applied once, but max_escalations limits
        // the total number of escalations per request
        let rules = vec![
            EscalationRule::new("rule-1", std::time::Duration::from_secs(1))
                .with_priority(1),
            EscalationRule::new("rule-2", std::time::Duration::from_secs(1))
                .with_priority(2),
            EscalationRule::new("rule-3", std::time::Duration::from_secs(1))
                .with_priority(3),
        ];
        let mut manager = EscalationManager::new(rules);

        let created_old = Utc::now() - Duration::hours(1);

        // Each rule can only trigger once
        // First escalation: rule-3 (highest priority)
        let rule = manager.check_escalation("req-1", created_old).cloned();
        assert!(rule.is_some(), "First escalation should trigger");
        assert_eq!(rule.as_ref().unwrap().name, "rule-3");
        manager.escalate("req-1", &rule.unwrap());

        // Second escalation: rule-2 (rule-3 already applied)
        let rule = manager.check_escalation("req-1", created_old).cloned();
        assert!(rule.is_some(), "Second escalation should trigger");
        assert_eq!(rule.as_ref().unwrap().name, "rule-2");
        manager.escalate("req-1", &rule.unwrap());

        // Third escalation: rule-1 (rule-3, rule-2 already applied)
        let rule = manager.check_escalation("req-1", created_old).cloned();
        assert!(rule.is_some(), "Third escalation should trigger");
        assert_eq!(rule.as_ref().unwrap().name, "rule-1");
        manager.escalate("req-1", &rule.unwrap());

        // Fourth should not trigger (all rules applied)
        assert!(manager.check_escalation("req-1", created_old).is_none());
    }

    #[test]
    fn test_default_rules() {
        let rules = default_rules();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].name, "standard-30m");
        assert_eq!(rules[1].name, "urgent-1h");
        assert_eq!(rules[2].name, "critical-2h");
    }

    #[test]
    fn test_escalation_event_serialization() {
        let event = EscalationEvent {
            request_id: "req-123".to_string(),
            rule_name: "urgent".to_string(),
            escalated_to: vec!["admin".to_string()],
            notified_channels: vec!["telegram".to_string()],
            escalated_at: Utc::now(),
            escalation_level: 2,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("req-123"));
        assert!(json.contains("urgent"));
        assert!(json.contains("\"escalation_level\":2"));
    }
}
