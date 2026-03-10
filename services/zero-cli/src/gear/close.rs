//! CLOSE Evaluation Framework
//!
//! Five-dimension framework for evaluating operational context and
//! recommending appropriate autonomy levels.
//!
//! # CLOSE Framework
//!
//! - **C**onvergence: How well do observations agree?
//! - **L**everage: What's the potential impact?
//! - **O**ptionality: How many valid choices exist? (key to "再来一次")
//! - **S**urplus: Do we have resources/margin?
//! - **E**volution: Is the situation improving?

use serde::{Deserialize, Serialize};

use super::presets::GearPreset;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// A single CLOSE dimension evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLOSEDimension {
    /// Score from 0-10
    pub score: f32,
    /// Confidence in this evaluation (0.0-1.0)
    pub confidence: f32,
    /// Factors that contributed to this score
    pub factors: Vec<String>,
}

impl Default for CLOSEDimension {
    fn default() -> Self {
        Self {
            score: 5.0,
            confidence: 0.5,
            factors: Vec::new(),
        }
    }
}

/// Complete CLOSE evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEEvaluation {
    /// Convergence: How well do observations agree?
    pub convergence: CLOSEDimension,
    /// Leverage: What's the potential impact?
    pub leverage: CLOSEDimension,
    /// Optionality: How many valid choices exist?
    pub optionality: CLOSEDimension,
    /// Surplus: Do we have resources/margin?
    pub surplus: CLOSEDimension,
    /// Evolution: Is the situation improving?
    pub evolution: CLOSEDimension,
    /// Weighted total score (0-10)
    pub total: f32,
    /// Computed risk score (0-10)
    pub risk: f32,
    /// Overall confidence (0.0-1.0)
    pub confidence: f32,
    /// Recommended gear based on evaluation
    pub recommended_gear: GearPreset,
    /// Evaluation timestamp (Unix milliseconds)
    pub timestamp: i64,
}

impl Default for CLOSEEvaluation {
    fn default() -> Self {
        Self {
            convergence: CLOSEDimension::default(),
            leverage: CLOSEDimension::default(),
            optionality: CLOSEDimension::default(),
            surplus: CLOSEDimension::default(),
            evolution: CLOSEDimension::default(),
            total: 5.0,
            risk: 5.0,
            confidence: 0.5,
            recommended_gear: GearPreset::D,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Weights for each CLOSE dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLOSEWeights {
    pub convergence: f32,
    pub leverage: f32,
    pub optionality: f32, // Higher weight: optionality is key to "再来一次"
    pub surplus: f32,
    pub evolution: f32,
}

impl Default for CLOSEWeights {
    fn default() -> Self {
        Self {
            convergence: 1.0,
            leverage: 1.2,
            optionality: 1.5, // Higher weight: optionality is key
            surplus: 1.3,
            evolution: 0.8,
        }
    }
}

/// Configuration for the CLOSE evaluator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEEvaluatorConfig {
    pub weights: CLOSEWeights,
    /// How much to weight recent history vs current snapshot (0.0-1.0)
    pub history_weight: f32,
}

impl Default for CLOSEEvaluatorConfig {
    fn default() -> Self {
        Self {
            weights: CLOSEWeights::default(),
            history_weight: 0.3,
        }
    }
}

/// Input data for CLOSE evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEInput {
    // Convergence factors
    /// Snapshot confidence (0.0-1.0)
    pub snapshot_confidence: f32,
    /// Build status: "passing", "failing", "unknown"
    pub build_status: String,
    /// Session health: "healthy", "degraded", "critical"
    pub session_health: String,
    /// Number of critical anomalies
    pub critical_anomalies: usize,
    /// Number of strong patterns detected
    pub strong_patterns: usize,

    // Leverage factors
    /// Number of high-impact opportunities
    pub high_impact_opportunities: usize,
    /// Number of medium-impact opportunities
    pub medium_impact_opportunities: usize,
    /// Number of external opportunities
    pub external_opportunities: usize,
    /// Number of external risks
    pub external_risks: usize,

    // Optionality factors
    /// Total opportunities available
    pub total_opportunities: usize,
    /// Number of distinct pattern types
    pub pattern_types: usize,
    /// Total anomaly count
    pub anomaly_count: usize,
    /// Decision quality score (0.0-1.0)
    pub decision_quality: f32,
    /// Number of recent errors
    pub recent_errors: usize,

    // Surplus factors
    /// Token usage
    pub token_usage: usize,
    /// Cost in dollars
    pub cost: f32,
    /// Consensus strength (0.0-1.0)
    pub consensus_strength: f32,
    /// Number of coverage gaps
    pub coverage_gaps: usize,

    // Evolution factors
    /// Number of learning opportunities
    pub learning_opportunities: usize,
    /// Number of recent code changes
    pub recent_changes: usize,
    /// Tech debt level: "low", "medium", "high"
    pub tech_debt_level: String,
    /// Number of dismissed anomalies
    pub dismissed_anomalies: usize,
    /// Number of active anomalies
    pub active_anomalies: usize,
}

// ══════════════════════════════════════════════════════════════════════════════
// CLOSE Evaluator
// ══════════════════════════════════════════════════════════════════════════════

/// Evaluates operational context using the CLOSE framework
#[derive(Debug, Clone)]
pub struct CLOSEEvaluator {
    config: CLOSEEvaluatorConfig,
    history: Vec<CLOSEEvaluation>,
    max_history: usize,
}

impl Default for CLOSEEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl CLOSEEvaluator {
    /// Create new evaluator with default config
    pub fn new() -> Self {
        Self {
            config: CLOSEEvaluatorConfig::default(),
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Create evaluator with custom config
    pub fn with_config(config: CLOSEEvaluatorConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Evaluate input data
    pub fn evaluate(&mut self, input: &CLOSEInput) -> CLOSEEvaluation {
        let convergence = self.evaluate_convergence(input);
        let leverage = self.evaluate_leverage(input);
        let optionality = self.evaluate_optionality(input);
        let surplus = self.evaluate_surplus(input);
        let evolution = self.evaluate_evolution(input);

        // Calculate weighted total
        let weights = &self.config.weights;
        let weight_sum =
            weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution;

        let raw_total = convergence.score * weights.convergence
            + leverage.score * weights.leverage
            + optionality.score * weights.optionality
            + surplus.score * weights.surplus
            + evolution.score * weights.evolution;

        let total = (raw_total / weight_sum).clamp(0.0, 10.0);

        // Calculate risk
        let risk = self.calculate_risk(&convergence, &optionality, &surplus, input);

        // Calculate overall confidence
        let confidence = self.calculate_overall_confidence(&convergence, &leverage, &optionality, &surplus, &evolution);

        // Recommend gear based on evaluation
        let recommended_gear = self.recommend_gear(total, risk, confidence);

        let evaluation = CLOSEEvaluation {
            convergence,
            leverage,
            optionality,
            surplus,
            evolution,
            total: (total * 100.0).round() / 100.0,
            risk: (risk * 100.0).round() / 100.0,
            confidence: (confidence * 100.0).round() / 100.0,
            recommended_gear,
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        // Update history
        self.history.push(evaluation.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        evaluation
    }

    /// Get evaluation history
    pub fn get_history(&self, limit: Option<usize>) -> &[CLOSEEvaluation] {
        let limit = limit.unwrap_or(20);
        let start = self.history.len().saturating_sub(limit);
        &self.history[start..]
    }

    /// Get trend of CLOSE scores
    pub fn get_trend(&self) -> CLOSETrend {
        if self.history.len() < 3 {
            return CLOSETrend::Stable;
        }

        let recent: Vec<_> = self.history.iter().rev().take(3).collect();
        let older: Vec<_> = self.history.iter().rev().skip(3).take(3).collect();

        if older.is_empty() {
            return CLOSETrend::Stable;
        }

        let recent_avg: f32 = recent.iter().map(|e| e.total).sum::<f32>() / recent.len() as f32;
        let older_avg: f32 = older.iter().map(|e| e.total).sum::<f32>() / older.len() as f32;

        if recent_avg > older_avg + 0.5 {
            CLOSETrend::Improving
        } else if recent_avg < older_avg - 0.5 {
            CLOSETrend::Declining
        } else {
            CLOSETrend::Stable
        }
    }

    /// Clear evaluation history
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Update configuration
    pub fn update_config(&mut self, config: CLOSEEvaluatorConfig) {
        self.config = config;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Dimension Evaluators
    // ─────────────────────────────────────────────────────────────────────────

    fn evaluate_convergence(&self, input: &CLOSEInput) -> CLOSEDimension {
        let mut factors = Vec::new();
        let mut score = input.snapshot_confidence * 10.0;

        // Build status
        match input.build_status.as_str() {
            "passing" => {
                score += 1.0;
                factors.push("Build passing".to_string());
            }
            "failing" => {
                score -= 2.0;
                factors.push("Build failing".to_string());
            }
            _ => {}
        }

        // Session health
        match input.session_health.as_str() {
            "healthy" => {
                score += 1.0;
                factors.push("Session healthy".to_string());
            }
            "critical" => {
                score -= 2.0;
                factors.push("Session critical".to_string());
            }
            _ => {}
        }

        // Anomalies impact
        if input.critical_anomalies > 0 {
            score -= input.critical_anomalies as f32 * 1.5;
            factors.push(format!("{} critical anomalies", input.critical_anomalies));
        }

        // Pattern support
        if input.strong_patterns > 0 {
            score += input.strong_patterns as f32 * 0.3;
            factors.push(format!("{} strong patterns", input.strong_patterns));
        }

        CLOSEDimension {
            score: score.clamp(0.0, 10.0),
            confidence: input.snapshot_confidence,
            factors,
        }
    }

    fn evaluate_leverage(&self, input: &CLOSEInput) -> CLOSEDimension {
        let mut factors = Vec::new();
        let mut score = 5.0;

        // High-impact opportunities
        score += input.high_impact_opportunities as f32 * 1.5;
        if input.high_impact_opportunities > 0 {
            factors.push(format!("{} high-impact opportunities", input.high_impact_opportunities));
        }

        // Medium-impact opportunities
        score += input.medium_impact_opportunities as f32 * 0.5;
        if input.medium_impact_opportunities > 0 {
            factors.push(format!("{} medium-impact opportunities", input.medium_impact_opportunities));
        }

        // External opportunities
        if input.external_opportunities > 0 {
            score += (input.external_opportunities as f32 * 0.5).min(2.0);
            factors.push(format!("{} external opportunities", input.external_opportunities));
        }

        // External risks reduce leverage
        if input.external_risks > 0 {
            score -= (input.external_risks as f32 * 0.5).min(2.0);
            factors.push(format!("{} external risks", input.external_risks));
        }

        CLOSEDimension {
            score: score.clamp(0.0, 10.0),
            confidence: input.snapshot_confidence * 0.9,
            factors,
        }
    }

    fn evaluate_optionality(&self, input: &CLOSEInput) -> CLOSEDimension {
        let mut factors = Vec::new();
        let mut score = 5.0;

        // More opportunities = more options
        score += (input.total_opportunities as f32 * 0.5).min(3.0);
        if input.total_opportunities > 0 {
            factors.push(format!("{} opportunities available", input.total_opportunities));
        }

        // Diverse patterns suggest more paths
        score += (input.pattern_types as f32 * 0.5).min(2.0);
        if input.pattern_types > 1 {
            factors.push(format!("{} pattern types", input.pattern_types));
        }

        // High anomaly count reduces options
        if input.anomaly_count > 5 {
            score -= 2.0;
            factors.push("Many anomalies limiting options".to_string());
        } else if input.anomaly_count > 2 {
            score -= 1.0;
            factors.push("Some anomalies limiting options".to_string());
        }

        // Low decision quality suggests fewer good options
        if input.decision_quality < 0.5 {
            score -= 1.0;
            factors.push("Low decision quality".to_string());
        }

        // Many recent errors suggest constrained options
        if input.recent_errors > 3 {
            score -= 1.5;
            factors.push("Frequent errors".to_string());
        }

        CLOSEDimension {
            score: score.clamp(0.0, 10.0),
            confidence: input.snapshot_confidence * 0.85,
            factors,
        }
    }

    fn evaluate_surplus(&self, input: &CLOSEInput) -> CLOSEDimension {
        let mut factors = Vec::new();
        let mut score = 5.0;

        // Token efficiency
        if input.token_usage < 50000 {
            score += 1.0;
            factors.push("Low token usage".to_string());
        } else if input.token_usage > 200000 {
            score -= 1.0;
            factors.push("High token usage".to_string());
        }

        // Cost efficiency
        if input.cost < 1.0 {
            score += 1.0;
            factors.push("Low cost".to_string());
        } else if input.cost > 10.0 {
            score -= 1.5;
            factors.push("High cost".to_string());
        }

        // Session health
        match input.session_health.as_str() {
            "healthy" => {
                score += 1.5;
                factors.push("Healthy session margin".to_string());
            }
            "degraded" => {
                score -= 1.0;
                factors.push("Degraded session".to_string());
            }
            "critical" => {
                score -= 2.0;
                factors.push("Critical session".to_string());
            }
            _ => {}
        }

        // Consensus strength
        if input.consensus_strength > 0.7 {
            score += 1.0;
            factors.push("Strong consensus".to_string());
        } else if input.consensus_strength < 0.4 {
            score -= 1.0;
            factors.push("Weak consensus".to_string());
        }

        // Coverage gaps
        if input.coverage_gaps > 0 {
            score -= input.coverage_gaps as f32 * 0.3;
            factors.push(format!("{} coverage gaps", input.coverage_gaps));
        }

        CLOSEDimension {
            score: score.clamp(0.0, 10.0),
            confidence: input.snapshot_confidence * 0.9,
            factors,
        }
    }

    fn evaluate_evolution(&self, input: &CLOSEInput) -> CLOSEDimension {
        let mut factors = Vec::new();
        let mut score = 5.0;

        // Historical trend
        let trend = self.get_trend();
        match trend {
            CLOSETrend::Improving => {
                score += 2.0;
                factors.push("Improving trend".to_string());
            }
            CLOSETrend::Declining => {
                score -= 2.0;
                factors.push("Declining trend".to_string());
            }
            CLOSETrend::Stable => {}
        }

        // Learning opportunities
        if input.learning_opportunities > 0 {
            score += input.learning_opportunities as f32 * 0.5;
            factors.push(format!("{} learning opportunities", input.learning_opportunities));
        }

        // Active development
        if input.recent_changes > 0 {
            score += (input.recent_changes as f32 * 0.2).min(1.5);
            factors.push(format!("{} recent changes", input.recent_changes));
        }

        // Tech debt
        match input.tech_debt_level.as_str() {
            "low" => {
                score += 1.0;
                factors.push("Low tech debt".to_string());
            }
            "high" => {
                score -= 1.0;
                factors.push("High tech debt".to_string());
            }
            _ => {}
        }

        // Anomaly resolution
        if input.dismissed_anomalies > input.active_anomalies {
            score += 1.0;
            factors.push("Anomalies being resolved".to_string());
        } else if input.active_anomalies > input.dismissed_anomalies * 2 {
            score -= 1.0;
            factors.push("Anomalies accumulating".to_string());
        }

        CLOSEDimension {
            score: score.clamp(0.0, 10.0),
            confidence: input.snapshot_confidence * 0.8,
            factors,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Risk and Confidence
    // ─────────────────────────────────────────────────────────────────────────

    fn calculate_risk(
        &self,
        convergence: &CLOSEDimension,
        optionality: &CLOSEDimension,
        surplus: &CLOSEDimension,
        input: &CLOSEInput,
    ) -> f32 {
        let mut risk = 0.0;

        // Low convergence = high risk
        risk += (10.0 - convergence.score) * 0.3;

        // Low optionality = high risk (can't "再来一次")
        risk += (10.0 - optionality.score) * 0.4;

        // Low surplus = high risk (no margin for error)
        risk += (10.0 - surplus.score) * 0.3;

        // Critical anomalies are direct risk factors
        risk += input.critical_anomalies as f32 * 1.5;

        risk.clamp(0.0, 10.0)
    }

    fn calculate_overall_confidence(
        &self,
        convergence: &CLOSEDimension,
        leverage: &CLOSEDimension,
        optionality: &CLOSEDimension,
        surplus: &CLOSEDimension,
        evolution: &CLOSEDimension,
    ) -> f32 {
        let confidences = [
            convergence.confidence,
            leverage.confidence,
            optionality.confidence,
            surplus.confidence,
            evolution.confidence,
        ];
        confidences.iter().sum::<f32>() / confidences.len() as f32
    }

    fn recommend_gear(&self, total: f32, risk: f32, confidence: f32) -> GearPreset {
        // Low confidence = stay conservative
        if confidence < 0.3 {
            return GearPreset::N;
        }

        // High risk = reduce autonomy
        if risk > 7.0 {
            return GearPreset::N;
        }
        if risk > 5.0 {
            return GearPreset::D;
        }

        // Based on total score
        if total >= 8.0 && risk < 4.0 {
            GearPreset::S
        } else if total >= 5.0 {
            GearPreset::D
        } else if total >= 3.0 {
            GearPreset::N
        } else {
            GearPreset::P
        }
    }
}

/// Trend direction for CLOSE evaluations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CLOSETrend {
    Improving,
    Declining,
    Stable,
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_evaluator_default() {
        let mut evaluator = CLOSEEvaluator::new();
        let input = CLOSEInput::default();
        let result = evaluator.evaluate(&input);

        assert!(result.total >= 0.0 && result.total <= 10.0);
        assert!(result.risk >= 0.0 && result.risk <= 10.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_close_evaluator_healthy_input() {
        let mut evaluator = CLOSEEvaluator::new();
        let input = CLOSEInput {
            snapshot_confidence: 0.9,
            build_status: "passing".to_string(),
            session_health: "healthy".to_string(),
            high_impact_opportunities: 3,
            total_opportunities: 5,
            consensus_strength: 0.8,
            ..Default::default()
        };
        let result = evaluator.evaluate(&input);

        assert!(result.total > 5.0);
        assert!(result.risk < 5.0);
        // Very healthy system with high score and low risk should recommend Sport mode
        assert_eq!(result.recommended_gear, GearPreset::S);
    }

    #[test]
    fn test_close_evaluator_risky_input() {
        let mut evaluator = CLOSEEvaluator::new();
        let input = CLOSEInput {
            snapshot_confidence: 0.3,
            build_status: "failing".to_string(),
            session_health: "critical".to_string(),
            critical_anomalies: 5,
            external_risks: 3,
            ..Default::default()
        };
        let result = evaluator.evaluate(&input);

        assert!(result.total < 5.0);
        assert!(result.risk > 5.0);
        assert!(matches!(
            result.recommended_gear,
            GearPreset::N | GearPreset::P
        ));
    }

    #[test]
    fn test_close_evaluator_history() {
        let mut evaluator = CLOSEEvaluator::new();
        let input = CLOSEInput::default();

        for _ in 0..5 {
            evaluator.evaluate(&input);
        }

        assert_eq!(evaluator.get_history(None).len(), 5);
        assert_eq!(evaluator.get_history(Some(3)).len(), 3);
    }

    #[test]
    fn test_close_trend() {
        let mut evaluator = CLOSEEvaluator::new();

        // Not enough history
        assert_eq!(evaluator.get_trend(), CLOSETrend::Stable);

        // Add some evaluations
        let input = CLOSEInput::default();
        for _ in 0..6 {
            evaluator.evaluate(&input);
        }

        // Should be stable with same input
        assert_eq!(evaluator.get_trend(), CLOSETrend::Stable);
    }

    #[test]
    fn test_close_dimension_factors() {
        let mut evaluator = CLOSEEvaluator::new();
        let input = CLOSEInput {
            build_status: "passing".to_string(),
            session_health: "healthy".to_string(),
            ..Default::default()
        };
        let result = evaluator.evaluate(&input);

        assert!(result
            .convergence
            .factors
            .iter()
            .any(|f| f.contains("Build passing")));
        assert!(result
            .convergence
            .factors
            .iter()
            .any(|f| f.contains("Session healthy")));
    }
}
