//! Hybrid Decision Making Framework.
//!
//! This module provides a trait-based abstraction for the "hybrid decision making" pattern:
//! - **Fast path**: Deterministic rule engine for 95% of cases
//! - **Slow path**: LLM-powered agent analysis for edge cases and anomalies
//!
//! # Design Principle
//!
//! > "高确定性任务用规则引擎保证效率，高不确定性任务用大模型保证正确反应"
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    HybridDecisionMaker                        │
//! ├──────────────────────────────────────────────────────────────┤
//! │                                                               │
//! │  ┌─────────────────┐     ┌─────────────────┐                │
//! │  │   Rule Engine   │     │   Agent Bridge  │                │
//! │  │  (Deterministic)│     │  (CodeCoder API)│                │
//! │  │   Fast: ~1ms    │     │   Slow: ~2-5s   │                │
//! │  └────────┬────────┘     └────────┬────────┘                │
//! │           │                       │                          │
//! │           └───────────┬───────────┘                          │
//! │                       ▼                                      │
//! │              ┌────────────────┐                              │
//! │              │ Final Decision │                              │
//! │              └────────────────┘                              │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Workflow
//!
//! 1. `rule_evaluate()` - Always runs first, provides fast deterministic result
//! 2. `check_triggers()` - Examines rule result for anomaly conditions
//! 3. If triggers found AND agent enabled:
//!    - `agent_analyze()` - Calls LLM for deep analysis
//!    - `merge_decisions()` - Combines rule and agent results
//! 4. Return final decision with source tracking
//!
//! # Example Implementation
//!
//! See `zero-trading/src/macro_agent/orchestrator.rs` for a complete implementation
//! that uses this pattern for macro-economic trading decisions.

use std::fmt::Debug;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trigger reasons that cause agent analysis to be invoked.
///
/// Implementors should define their own trigger types that capture
/// the specific anomaly conditions relevant to their domain.
pub trait AnalysisTrigger: Debug + Clone + Send + Sync + 'static {
    /// Human-readable description of the trigger.
    fn description(&self) -> &str;
}

/// Source of a hybrid decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionSource {
    /// Decision came purely from rule engine (fast path).
    RuleEngine,
    /// Decision came purely from agent analysis.
    AgentAnalysis,
    /// Decision merged from both rule engine and agent.
    Merged,
    /// Fallback decision due to agent failure.
    Fallback,
}

/// Result of agent analysis with confidence score.
pub trait AgentResult: Debug + Clone + Send + Sync + 'static {
    /// Confidence level of the analysis (0.0 - 1.0).
    fn confidence(&self) -> f64;
}

/// Final decision from hybrid evaluation.
pub trait HybridDecision: Debug + Clone + Send + Sync + 'static {
    /// The source of this decision.
    fn source(&self) -> DecisionSource;

    /// Confidence level of this decision (0.0 - 1.0).
    fn confidence(&self) -> f64;
}

/// Configuration for hybrid decision making behavior.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Whether agent analysis is enabled.
    pub agent_enabled: bool,
    /// Minimum confidence required to trust agent analysis.
    pub min_agent_confidence: f64,
    /// Cache duration for agent analysis results (in seconds).
    pub cache_duration_secs: u64,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            agent_enabled: true,
            min_agent_confidence: 0.6,
            cache_duration_secs: 3600,
        }
    }
}

/// Trait for hybrid decision making that combines deterministic rules with LLM analysis.
///
/// # Type Parameters
///
/// - `C`: Context type containing input data for evaluation
/// - `R`: Rule engine result type (deterministic, fast)
/// - `A`: Agent analysis result type (LLM-powered, slow)
/// - `D`: Final decision type
/// - `T`: Trigger type indicating why agent should be invoked
///
/// # Implementation Guide
///
/// 1. `rule_evaluate`: Should be fast (~1ms) and deterministic
/// 2. `check_triggers`: Examine rule result for anomaly conditions
/// 3. `agent_analyze`: Call CodeCoder API or similar LLM service
/// 4. `merge_decisions`: Combine rule and agent results intelligently
///
/// # Example
///
/// ```rust,ignore
/// #[async_trait]
/// impl HybridDecisionMaker for MacroOrchestrator {
///     type Context = MacroContext;
///     type RuleResult = MacroEnvironment;
///     type AgentResult = AgentAnalysis;
///     type Decision = MacroDecision;
///     type Trigger = AnalysisTrigger;
///
///     async fn rule_evaluate(&self, ctx: &Self::Context) -> Result<Self::RuleResult> {
///         self.rule_engine.evaluate(ctx).await
///     }
///
///     fn check_triggers(&self, result: &Self::RuleResult) -> Vec<Self::Trigger> {
///         let mut triggers = Vec::new();
///         if result.risk_appetite < 30.0 || result.risk_appetite > 70.0 {
///             triggers.push(AnalysisTrigger::ExtremeRiskAppetite);
///         }
///         triggers
///     }
///
///     async fn agent_analyze(&self, ctx: &Self::Context) -> Result<Self::AgentResult> {
///         self.agent_bridge.analyze(ctx).await
///     }
///
///     fn merge_decisions(
///         &self,
///         rule: Self::RuleResult,
///         agent: Self::AgentResult,
///         triggers: Vec<Self::Trigger>,
///     ) -> Self::Decision {
///         // Weighted combination, agent gets more weight for triggered conditions
///         MacroDecision::merged(rule, agent, triggers)
///     }
/// }
/// ```
#[async_trait]
pub trait HybridDecisionMaker: Send + Sync {
    /// Context type containing input data for evaluation.
    type Context: Debug + Clone + Send + Sync;

    /// Result type from the deterministic rule engine.
    type RuleResult: Debug + Clone + Send + Sync;

    /// Result type from the LLM-powered agent.
    type AgentResult: AgentResult;

    /// Final decision type.
    type Decision: HybridDecision;

    /// Trigger type indicating anomaly conditions.
    type Trigger: AnalysisTrigger;

    /// Evaluate using the deterministic rule engine (fast path).
    ///
    /// This should be a fast (~1ms), deterministic operation that handles
    /// the majority of cases without needing LLM analysis.
    async fn rule_evaluate(&self, context: &Self::Context) -> anyhow::Result<Self::RuleResult>;

    /// Check if the rule result contains anomaly conditions that warrant agent analysis.
    ///
    /// Returns a list of triggers that explain why agent analysis is needed.
    /// An empty list means the rule engine result is sufficient.
    fn check_triggers(&self, rule_result: &Self::RuleResult) -> Vec<Self::Trigger>;

    /// Request deep analysis from the LLM-powered agent (slow path).
    ///
    /// This typically involves calling the CodeCoder API or similar service.
    /// Should only be called when `check_triggers` returns non-empty.
    async fn agent_analyze(&self, context: &Self::Context) -> anyhow::Result<Self::AgentResult>;

    /// Convert rule engine result to a decision (used when agent is not invoked).
    fn rule_to_decision(
        &self,
        rule_result: Self::RuleResult,
        source: DecisionSource,
    ) -> Self::Decision;

    /// Merge rule engine and agent analysis results into a final decision.
    ///
    /// Implementors should consider:
    /// - Weighting: Agent results may get higher weight for triggered conditions
    /// - Conservatism: When in doubt, prefer the more conservative option
    /// - Explanation: Include trigger reasons in the final decision
    fn merge_decisions(
        &self,
        rule_result: Self::RuleResult,
        agent_result: Self::AgentResult,
        triggers: Vec<Self::Trigger>,
    ) -> Self::Decision;

    /// Get the hybrid configuration.
    fn config(&self) -> &HybridConfig;

    /// Perform hybrid evaluation following the standard workflow.
    ///
    /// This is the main entry point that orchestrates:
    /// 1. Rule evaluation (always)
    /// 2. Trigger checking
    /// 3. Agent analysis (if triggered and enabled)
    /// 4. Result merging or fallback
    async fn evaluate(&self, context: &Self::Context) -> anyhow::Result<Self::Decision> {
        // Step 1: Always run rule engine first
        let rule_result = self.rule_evaluate(context).await?;

        // Step 2: Check if we need agent analysis
        let triggers = self.check_triggers(&rule_result);
        let config = self.config();

        if triggers.is_empty() || !config.agent_enabled {
            return Ok(self.rule_to_decision(rule_result, DecisionSource::RuleEngine));
        }

        // Step 3: Request agent analysis
        match self.agent_analyze(context).await {
            Ok(agent_result) => {
                // Only trust agent if confidence is high enough
                if agent_result.confidence() >= config.min_agent_confidence {
                    Ok(self.merge_decisions(rule_result, agent_result, triggers))
                } else {
                    // Agent confidence too low, use rule engine
                    Ok(self.rule_to_decision(rule_result, DecisionSource::RuleEngine))
                }
            }
            Err(_) => {
                // Agent failed, fall back to rule engine
                Ok(self.rule_to_decision(rule_result, DecisionSource::Fallback))
            }
        }
    }

    /// Force agent analysis, bypassing trigger checks.
    ///
    /// Useful for manual deep analysis requests.
    async fn force_analyze(&self, context: &Self::Context) -> anyhow::Result<Self::Decision> {
        let rule_result = self.rule_evaluate(context).await?;
        let agent_result = self.agent_analyze(context).await?;
        Ok(self.merge_decisions(rule_result, agent_result, vec![]))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert!(config.agent_enabled);
        assert!((config.min_agent_confidence - 0.6).abs() < 0.001);
        assert_eq!(config.cache_duration_secs, 3600);
    }

    #[test]
    fn test_decision_source_serialization() {
        let source = DecisionSource::Merged;
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("Merged"));

        let parsed: DecisionSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, DecisionSource::Merged);
    }
}
