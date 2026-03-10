//! Safety Guardrails for Autonomous Execution
//!
//! Detects and prevents dangerous patterns such as:
//! - State oscillation loops (A -> B -> A -> B)
//! - Tool call retry loops (same tool/input failing repeatedly)
//! - Decision hesitation (repeated same-type decisions)
//!
//! # Philosophy
//!
//! Following the "可用余量" principle, guardrails preserve optionality by
//! detecting when the system is stuck in unproductive patterns.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// Loop type detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopType {
    /// State oscillation (A <-> B)
    State,
    /// Repeated tool calls with same input
    Tool,
    /// Repeated decision hesitation
    Decision,
}

impl std::fmt::Display for LoopType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State => write!(f, "state"),
            Self::Tool => write!(f, "tool"),
            Self::Decision => write!(f, "decision"),
        }
    }
}

/// Configuration for guardrails
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailConfig {
    /// Maximum state transitions before limit triggered
    pub max_state_transitions: usize,
    /// Maximum tool retries before stopping
    pub max_tool_retries: usize,
    /// Maximum same-type decisions before hesitation detected
    pub max_decision_hesitation: usize,
    /// Enable loop detection
    pub loop_detection_enabled: bool,
    /// Number of repetitions to constitute a loop
    pub loop_threshold: usize,
    /// Automatically break detected loops
    pub auto_break_loops: bool,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            max_state_transitions: 100,
            max_tool_retries: 3,
            max_decision_hesitation: 5,
            loop_detection_enabled: true,
            loop_threshold: 3,
            auto_break_loops: true,
        }
    }
}

/// Result of a tool call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolResult {
    Success,
    Error,
}

/// State transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub timestamp: i64,
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub input: String,
    pub timestamp: i64,
    pub result: ToolResult,
}

/// Decision record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub decision_type: String,
    pub timestamp: i64,
    pub result: String,
}

/// Detected loop information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopDetection {
    pub loop_type: LoopType,
    pub pattern: Vec<String>,
    pub count: usize,
    pub broken: bool,
}

/// Result of safety limit check
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyCheckResult {
    pub safe: bool,
    pub reason: Option<String>,
    pub limit_type: Option<LimitType>,
}

/// Type of limit exceeded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitType {
    Transitions,
    ToolRetries,
    DecisionHesitation,
}

/// Statistics about guardrail activity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardrailStats {
    pub state_transitions: usize,
    pub tool_calls: usize,
    pub decisions: usize,
    pub loops_broken: usize,
}

// ══════════════════════════════════════════════════════════════════════════════
// SafetyGuardrails
// ══════════════════════════════════════════════════════════════════════════════

/// Safety guardrails for autonomous execution
///
/// Detects and prevents dangerous patterns during autonomous operation.
#[derive(Debug, Clone)]
pub struct SafetyGuardrails {
    session_id: String,
    config: GuardrailConfig,
    state_transitions: Vec<StateTransition>,
    tool_calls: Vec<ToolCall>,
    decisions: Vec<DecisionRecord>,
    loops_broken: HashSet<String>,
}

impl SafetyGuardrails {
    /// Create new guardrails for a session
    pub fn new(session_id: impl Into<String>, config: GuardrailConfig) -> Self {
        Self {
            session_id: session_id.into(),
            config,
            state_transitions: Vec::new(),
            tool_calls: Vec::new(),
            decisions: Vec::new(),
            loops_broken: HashSet::new(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(session_id: impl Into<String>) -> Self {
        Self::new(session_id, GuardrailConfig::default())
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get configuration
    pub fn config(&self) -> &GuardrailConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: GuardrailConfig) {
        self.config = config;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Recording
    // ─────────────────────────────────────────────────────────────────────────

    /// Record a state transition
    ///
    /// Returns loop detection if a state loop is found
    pub fn record_state_transition(&mut self, from: &str, to: &str) -> Option<LoopDetection> {
        let transition = StateTransition {
            from: from.to_string(),
            to: to.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };
        self.state_transitions.push(transition);
        self.trim_records();

        if self.config.loop_detection_enabled {
            self.detect_state_loop()
        } else {
            None
        }
    }

    /// Record a tool call
    ///
    /// Returns loop detection if a tool loop is found
    pub fn record_tool_call(
        &mut self,
        tool: &str,
        input: &str,
        result: ToolResult,
    ) -> Option<LoopDetection> {
        let call = ToolCall {
            tool: tool.to_string(),
            input: input.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            result,
        };
        self.tool_calls.push(call);
        self.trim_records();

        if self.config.loop_detection_enabled {
            self.detect_tool_loop()
        } else {
            None
        }
    }

    /// Record a decision
    ///
    /// Returns loop detection if decision hesitation is found
    pub fn record_decision(
        &mut self,
        id: &str,
        decision_type: &str,
        result: &str,
    ) -> Option<LoopDetection> {
        let record = DecisionRecord {
            id: id.to_string(),
            decision_type: decision_type.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            result: result.to_string(),
        };
        self.decisions.push(record);
        self.trim_records();

        if self.config.loop_detection_enabled {
            self.detect_decision_hesitation()
        } else {
            None
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Loop Detection
    // ─────────────────────────────────────────────────────────────────────────

    /// Detect state oscillation loops (A -> B -> A -> B)
    fn detect_state_loop(&mut self) -> Option<LoopDetection> {
        let threshold = self.config.loop_threshold * 2;
        let recent: Vec<_> = self
            .state_transitions
            .iter()
            .rev()
            .take(threshold)
            .collect();

        if recent.len() < threshold {
            return None;
        }

        // Look for A -> B -> A -> B pattern
        for i in 0..=recent.len().saturating_sub(4) {
            let t1 = &recent[i];
            let t2 = recent.get(i + 1)?;
            let t3 = recent.get(i + 2)?;
            let t4 = recent.get(i + 3)?;

            // Check for oscillation pattern
            if t1.from == t3.from
                && t1.to == t3.to
                && t2.from == t4.from
                && t2.to == t4.to
                && t1.from == t2.to
                && t2.from == t1.to
            {
                let loop_key = format!("state:{}<=>{}", t1.from, t2.from);

                if self.loops_broken.contains(&loop_key) {
                    continue;
                }

                let broken = if self.config.auto_break_loops {
                    self.loops_broken.insert(loop_key);
                    true
                } else {
                    false
                };

                return Some(LoopDetection {
                    loop_type: LoopType::State,
                    pattern: vec![t1.from.clone(), t2.from.clone()],
                    count: self.config.loop_threshold,
                    broken,
                });
            }
        }

        None
    }

    /// Detect tool call loops (same tool with same input failing repeatedly)
    fn detect_tool_loop(&mut self) -> Option<LoopDetection> {
        let threshold = self.config.loop_threshold;
        let recent: Vec<_> = self.tool_calls.iter().rev().take(threshold).collect();

        if recent.len() < threshold {
            return None;
        }

        let last = recent.first()?;

        // Count same tool+input with error result
        let count = recent
            .iter()
            .filter(|c| {
                c.tool == last.tool && c.input == last.input && c.result == ToolResult::Error
            })
            .count();

        if count >= threshold {
            let loop_key = format!("tool:{}", last.tool);

            if self.loops_broken.contains(&loop_key) {
                return Some(LoopDetection {
                    loop_type: LoopType::Tool,
                    pattern: vec![last.tool.clone()],
                    count,
                    broken: true, // Already broken
                });
            }

            let broken = if self.config.auto_break_loops {
                self.loops_broken.insert(loop_key);
                true
            } else {
                false
            };

            return Some(LoopDetection {
                loop_type: LoopType::Tool,
                pattern: vec![last.tool.clone(), last.input.clone()],
                count,
                broken,
            });
        }

        None
    }

    /// Detect decision hesitation (repeated same-type decisions)
    fn detect_decision_hesitation(&mut self) -> Option<LoopDetection> {
        let threshold = self.config.max_decision_hesitation;
        let recent: Vec<_> = self.decisions.iter().rev().take(threshold).collect();

        if recent.len() < threshold {
            return None;
        }

        // Check if all recent decisions are of the same type
        let decision_types: HashSet<_> = recent.iter().map(|d| &d.decision_type).collect();

        if decision_types.len() == 1 {
            let decision_type = recent.first()?.decision_type.clone();

            return Some(LoopDetection {
                loop_type: LoopType::Decision,
                pattern: vec![decision_type],
                count: recent.len(),
                broken: false, // Don't auto-break decision loops
            });
        }

        None
    }

    /// Get all detected loops
    pub fn detect_loops(&self) -> Vec<LoopDetection> {
        let mut loops = Vec::new();

        // Create a temporary mutable clone for detection
        // This is a bit wasteful but maintains immutability of the method
        let mut temp = self.clone();

        if let Some(state_loop) = temp.detect_state_loop() {
            loops.push(state_loop);
        }
        if let Some(tool_loop) = temp.detect_tool_loop() {
            loops.push(tool_loop);
        }
        if let Some(decision_loop) = temp.detect_decision_hesitation() {
            loops.push(decision_loop);
        }

        loops
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Limit Checking
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if any safety limits are exceeded
    pub fn check_limits(&self) -> SafetyCheckResult {
        // Check state transitions
        if self.state_transitions.len() >= self.config.max_state_transitions {
            return SafetyCheckResult {
                safe: false,
                reason: Some(format!(
                    "Maximum state transitions exceeded: {}/{}",
                    self.state_transitions.len(),
                    self.config.max_state_transitions
                )),
                limit_type: Some(LimitType::Transitions),
            };
        }

        // Count consecutive tool failures
        let consecutive_failures = self
            .tool_calls
            .iter()
            .rev()
            .take_while(|c| c.result == ToolResult::Error)
            .count();

        if consecutive_failures >= self.config.max_tool_retries {
            return SafetyCheckResult {
                safe: false,
                reason: Some(format!(
                    "Maximum tool retries exceeded: {}/{}",
                    consecutive_failures, self.config.max_tool_retries
                )),
                limit_type: Some(LimitType::ToolRetries),
            };
        }

        SafetyCheckResult {
            safe: true,
            reason: None,
            limit_type: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Statistics
    // ─────────────────────────────────────────────────────────────────────────

    /// Get guardrail statistics
    pub fn get_stats(&self) -> GuardrailStats {
        GuardrailStats {
            state_transitions: self.state_transitions.len(),
            tool_calls: self.tool_calls.len(),
            decisions: self.decisions.len(),
            loops_broken: self.loops_broken.len(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Maintenance
    // ─────────────────────────────────────────────────────────────────────────

    /// Trim old records to prevent unbounded memory growth
    fn trim_records(&mut self) {
        let max_records = self.config.max_state_transitions * 2;

        if self.state_transitions.len() > max_records {
            let drain_count = self.state_transitions.len() - max_records;
            self.state_transitions.drain(..drain_count);
        }

        if self.tool_calls.len() > max_records {
            let drain_count = self.tool_calls.len() - max_records;
            self.tool_calls.drain(..drain_count);
        }

        if self.decisions.len() > max_records {
            let drain_count = self.decisions.len() - max_records;
            self.decisions.drain(..drain_count);
        }
    }

    /// Clear all records
    pub fn clear(&mut self) {
        self.state_transitions.clear();
        self.tool_calls.clear();
        self.decisions.clear();
        self.loops_broken.clear();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Serialization
    // ─────────────────────────────────────────────────────────────────────────

    /// Serialize to JSON
    pub fn serialize(&self) -> serde_json::Value {
        serde_json::json!({
            "stateTransitions": self.state_transitions,
            "toolCalls": self.tool_calls,
            "decisions": self.decisions,
            "loopsBroken": self.loops_broken.iter().collect::<Vec<_>>()
        })
    }

    /// Deserialize from JSON
    pub fn deserialize(
        json: &serde_json::Value,
        session_id: &str,
        config: GuardrailConfig,
    ) -> Result<Self, String> {
        let state_transitions = json
            .get("stateTransitions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let tool_calls = json
            .get("toolCalls")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let decisions = json
            .get("decisions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let loops_broken: HashSet<String> = json
            .get("loopsBroken")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            session_id: session_id.to_string(),
            config,
            state_transitions,
            tool_calls,
            decisions,
            loops_broken,
        })
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardrails_default() {
        let guardrails = SafetyGuardrails::with_defaults("test-session");
        assert_eq!(guardrails.session_id(), "test-session");
        assert!(guardrails.config().loop_detection_enabled);
    }

    #[test]
    fn test_state_transition_recording() {
        let mut guardrails = SafetyGuardrails::with_defaults("test");
        guardrails.record_state_transition("idle", "planning");
        assert_eq!(guardrails.get_stats().state_transitions, 1);
    }

    #[test]
    fn test_tool_call_recording() {
        let mut guardrails = SafetyGuardrails::with_defaults("test");
        guardrails.record_tool_call("read", "/path", ToolResult::Success);
        assert_eq!(guardrails.get_stats().tool_calls, 1);
    }

    #[test]
    fn test_decision_recording() {
        let mut guardrails = SafetyGuardrails::with_defaults("test");
        guardrails.record_decision("d1", "action", "proceed");
        assert_eq!(guardrails.get_stats().decisions, 1);
    }

    #[test]
    fn test_state_loop_detection() {
        let config = GuardrailConfig {
            loop_threshold: 2,
            ..Default::default()
        };
        let mut guardrails = SafetyGuardrails::new("test", config);

        // Create A <-> B oscillation
        guardrails.record_state_transition("a", "b");
        guardrails.record_state_transition("b", "a");
        guardrails.record_state_transition("a", "b");
        let result = guardrails.record_state_transition("b", "a");

        assert!(result.is_some());
        let loop_info = result.unwrap();
        assert_eq!(loop_info.loop_type, LoopType::State);
        assert!(loop_info.broken);
    }

    #[test]
    fn test_tool_loop_detection() {
        let config = GuardrailConfig {
            loop_threshold: 3,
            ..Default::default()
        };
        let mut guardrails = SafetyGuardrails::new("test", config);

        // Same tool failing repeatedly
        guardrails.record_tool_call("bash", "failing-cmd", ToolResult::Error);
        guardrails.record_tool_call("bash", "failing-cmd", ToolResult::Error);
        let result = guardrails.record_tool_call("bash", "failing-cmd", ToolResult::Error);

        assert!(result.is_some());
        let loop_info = result.unwrap();
        assert_eq!(loop_info.loop_type, LoopType::Tool);
    }

    #[test]
    fn test_limits_check() {
        let config = GuardrailConfig {
            max_tool_retries: 2,
            ..Default::default()
        };
        let mut guardrails = SafetyGuardrails::new("test", config);

        guardrails.record_tool_call("bash", "cmd", ToolResult::Error);
        assert!(guardrails.check_limits().safe);

        guardrails.record_tool_call("bash", "cmd", ToolResult::Error);
        assert!(!guardrails.check_limits().safe);
    }

    #[test]
    fn test_clear() {
        let mut guardrails = SafetyGuardrails::with_defaults("test");
        guardrails.record_state_transition("a", "b");
        guardrails.record_tool_call("read", "/path", ToolResult::Success);

        guardrails.clear();

        let stats = guardrails.get_stats();
        assert_eq!(stats.state_transitions, 0);
        assert_eq!(stats.tool_calls, 0);
    }

    #[test]
    fn test_serialization() {
        let mut guardrails = SafetyGuardrails::with_defaults("test");
        guardrails.record_state_transition("a", "b");

        let json = guardrails.serialize();
        let restored =
            SafetyGuardrails::deserialize(&json, "test", GuardrailConfig::default()).unwrap();

        assert_eq!(restored.get_stats().state_transitions, 1);
    }
}
