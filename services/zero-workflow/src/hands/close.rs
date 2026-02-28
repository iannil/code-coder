//! CLOSE Decision Framework (Rust implementation)
//!
//! CLOSE is a decision-making framework based on "祝融说" (ZhuRong Theory)
//! that evaluates decisions across five dimensions:
//!
//! - **C**onvergence: How focused/limited the decision is (lower is better)
//! - **L**everage: Impact vs effort ratio (higher is better)
//! - **O**ptionality: Flexibility and reversibility (higher is better)
//! - **S**urplus: Resource availability and slack (higher is better)
//! - **E**volution: Learning and adaptation value (higher is better)
//!
//! # Scoring
//!
//! Each dimension is scored 0-10, then weighted:
//! - Convergence: 1.0x (inverted - lower score = higher convergence)
//! - Leverage: 1.2x
//! - Optionality: 1.5x (highest weight - sustainability focus)
//! - Surplus: 1.3x
//! - Evolution: 0.8x
//!
//! Total score: 0-10, where higher = better.
//!
//! # Autonomy Thresholds
//!
//! | Level | Approval | Caution | Description |
//! |-------|----------|---------|-------------|
//! | Lunatic | 5.0 | 3.0 | 完全自主 |
//! | Insane | 5.5 | 3.5 | 高度自主 |
//! | Crazy | 6.0 | 4.0 | 显著自主 |
//! | Wild | 6.5 | 4.5 | 部分自主 |
//! | Bold | 7.0 | 5.0 | 谨慎自主 |
//! | Timid | 8.0 | 6.0 | 基本不自主 |

use super::manifest::AutonomyLevel;
use serde::{Deserialize, Serialize};

/// CLOSE decision criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseCriteria {
    /// Convergence: How focused/limited (0-10, lower is better)
    pub convergence: f32,

    /// Leverage: Impact vs effort ratio (0-10, higher is better)
    pub leverage: f32,

    /// Optionality: Flexibility and reversibility (0-10, higher is better)
    pub optionality: f32,

    /// Surplus: Resource availability (0-10, higher is better)
    pub surplus: f32,

    /// Evolution: Learning value (0-10, higher is better)
    pub evolution: f32,

    /// Optional description for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl CloseCriteria {
    /// Calculate the weighted CLOSE score.
    ///
    /// Returns a value from 0-10, where higher is better.
    pub fn calculate_score(&self) -> f32 {
        let weights = ScoreWeights::default();

        // Invert convergence (lower convergence = higher score)
        let inverted_convergence = 10.0 - self.convergence;

        let max_score = 10.0 * (weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution);

        let total = (inverted_convergence * weights.convergence
            + self.leverage * weights.leverage
            + self.optionality * weights.optionality
            + self.surplus * weights.surplus
            + self.evolution * weights.evolution)
            / max_score
            * 10.0;

        (total * 100.0).round() / 100.0
    }

    /// Check if this criteria meets the approval threshold for a given autonomy level.
    pub fn meets_threshold(&self, level: AutonomyLevel) -> bool {
        let score = self.calculate_score();
        let (approval_threshold, _) = level.thresholds();
        score >= approval_threshold
    }

    /// Get the decision result for a given autonomy level.
    pub fn decide(&self, level: AutonomyLevel) -> CloseDecision {
        let score = self.calculate_score();
        let (approval_threshold, caution_threshold) = level.thresholds();

        let action = if score >= approval_threshold {
            CloseAction::Proceed
        } else if score >= caution_threshold {
            CloseAction::ProceedWithCaution
        } else {
            CloseAction::Pause
        };

        CloseDecision {
            score,
            action,
            reasoning: self.format_reasoning(score, approval_threshold, caution_threshold),
        }
    }

    fn format_reasoning(&self, score: f32, approval: f32, caution: f32) -> String {
        format!(
            "CLOSE score: {:.2}/10 (C={:.1}, L={:.1}, O={:.1}, S={:.1}, E={:.1}), thresholds: approval={:.1}, caution={:.1}",
            score, self.convergence, self.leverage, self.optionality, self.surplus, self.evolution, approval, caution
        )
    }
}

impl Default for CloseCriteria {
    fn default() -> Self {
        Self {
            convergence: 5.0,
            leverage: 5.0,
            optionality: 5.0,
            surplus: 5.0,
            evolution: 5.0,
            description: None,
        }
    }
}

/// Score weights for CLOSE calculation.
#[derive(Debug, Clone, Copy)]
pub struct ScoreWeights {
    pub convergence: f32,
    pub leverage: f32,
    pub optionality: f32,
    pub surplus: f32,
    pub evolution: f32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            convergence: 1.0,
            leverage: 1.2,
            optionality: 1.5,
            surplus: 1.3,
            evolution: 0.8,
        }
    }
}

/// Decision action from CLOSE evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseAction {
    /// Proceed with the action
    Proceed,

    /// Proceed but with caution/monitoring
    ProceedWithCaution,

    /// Pause and require human input
    Pause,

    /// Block the action entirely
    Block,
}

/// CLOSE decision result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseDecision {
    /// Calculated CLOSE score (0-10)
    pub score: f32,

    /// Recommended action
    pub action: CloseAction,

    /// Human-readable reasoning
    pub reasoning: String,
}

impl CloseDecision {
    /// Check if the decision allows proceeding.
    pub fn can_proceed(&self) -> bool {
        matches!(self.action, CloseAction::Proceed | CloseAction::ProceedWithCaution)
    }

    /// Check if the decision is blocked.
    pub fn is_blocked(&self) -> bool {
        matches!(self.action, CloseAction::Pause | CloseAction::Block)
    }
}

/// CLOSE decision evaluator.
#[derive(Debug, Clone)]
pub struct CloseEvaluator {
    /// Default autonomy level for decisions
    default_level: AutonomyLevel,
}

impl CloseEvaluator {
    /// Create a new CLOSE evaluator.
    pub fn new(default_level: AutonomyLevel) -> Self {
        Self { default_level }
    }

    /// Evaluate criteria using the default autonomy level.
    pub fn evaluate(&self, criteria: &CloseCriteria) -> CloseDecision {
        criteria.decide(self.default_level)
    }

    /// Evaluate criteria with a specific autonomy level.
    pub fn evaluate_with_level(&self, criteria: &CloseCriteria, level: AutonomyLevel) -> CloseDecision {
        criteria.decide(level)
    }

    /// Quick check if criteria meets approval threshold.
    pub fn quick_check(&self, criteria: &CloseCriteria) -> bool {
        criteria.meets_threshold(self.default_level)
    }
}

impl Default for CloseEvaluator {
    fn default() -> Self {
        Self::new(AutonomyLevel::Crazy)
    }
}

/// Pre-built CLOSE criteria templates.
impl CloseCriteria {
    /// Low-risk implementation (high optionality, low convergence)
    pub fn low_risk_implementation(description: impl Into<String>) -> Self {
        Self {
            convergence: 3.0,
            leverage: 7.0,
            optionality: 8.0,
            surplus: 7.0,
            evolution: 5.0,
            description: Some(description.into()),
        }
    }

    /// High-risk architecture (low optionality, high convergence)
    pub fn high_risk_architecture(description: impl Into<String>) -> Self {
        Self {
            convergence: 8.0,
            leverage: 6.0,
            optionality: 3.0,
            surplus: 4.0,
            evolution: 7.0,
            description: Some(description.into()),
        }
    }

    /// Test writing (very high optionality)
    pub fn test_writing(description: impl Into<String>) -> Self {
        Self {
            convergence: 2.0,
            leverage: 8.0,
            optionality: 9.0,
            surplus: 8.0,
            evolution: 6.0,
            description: Some(description.into()),
        }
    }

    /// Rollback operation (maximum optionality)
    pub fn rollback(description: impl Into<String>) -> Self {
        Self {
            convergence: 5.0,
            leverage: 7.0,
            optionality: 10.0,
            surplus: 9.0,
            evolution: 4.0,
            description: Some(description.into()),
        }
    }

    /// Resource search vs build (high leverage, high optionality)
    pub fn search_vs_build(description: impl Into<String>) -> Self {
        Self {
            convergence: 8.0,
            leverage: 9.0,
            optionality: 9.0,
            surplus: 9.0,
            evolution: 7.0,
            description: Some(description.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_calculation() {
        let criteria = CloseCriteria {
            convergence: 5.0,
            leverage: 7.0,
            optionality: 8.0,
            surplus: 7.0,
            evolution: 6.0,
            description: None,
        };

        let score = criteria.calculate_score();
        assert!(score > 5.0, "Score should be above 5.0");
        assert!(score <= 10.0, "Score should not exceed 10.0");
    }

    #[test]
    fn test_thresholds() {
        let (approval, caution) = AutonomyLevel::Crazy.thresholds();
        assert_eq!(approval, 6.0);
        assert_eq!(caution, 4.0);
    }

    #[test]
    fn test_decision_proceed() {
        let criteria = CloseCriteria::search_vs_build("Test decision");
        let decision = criteria.decide(AutonomyLevel::Crazy);

        // High optionality/leverage should auto-approve
        assert!(decision.can_proceed());
        assert_eq!(decision.action, CloseAction::Proceed);
    }

    #[test]
    fn test_decision_pause() {
        let criteria = CloseCriteria::high_risk_architecture("Risky architecture");
        let decision = criteria.decide(AutonomyLevel::Timid);

        // High convergence with timid level should pause
        assert!(decision.is_blocked());
    }

    #[test]
    fn test_evaluator() {
        let evaluator = CloseEvaluator::new(AutonomyLevel::Insane);
        let criteria = CloseCriteria::low_risk_implementation("Test");

        assert!(evaluator.quick_check(&criteria));
    }

    #[test]
    fn test_craziness_score() {
        assert_eq!(AutonomyLevel::Lunatic.craziness_score(), 95);
        assert_eq!(AutonomyLevel::Crazy.craziness_score(), 75);
        assert_eq!(AutonomyLevel::Timid.craziness_score(), 15);
    }

    #[test]
    fn test_default_criteria() {
        let criteria = CloseCriteria::default();
        assert_eq!(criteria.convergence, 5.0);
        assert_eq!(criteria.leverage, 5.0);
    }
}
