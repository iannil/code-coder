//! CLOSE Decision Framework
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
//!
//! # Philosophy
//!
//! This implements the "祝融说" decision philosophy where:
//! - Optionality (能否再来一次) is weighted highest
//! - Decisions prioritize sustainability over optimality
//! - Risk is calculated from low convergence, optionality, and surplus

use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// Gear preset for recommended autonomy level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GearRecommendation {
    /// Park - System inactive
    P,
    /// Neutral - Observe only
    N,
    /// Drive - Balanced autonomy
    D,
    /// Sport - High autonomy
    S,
}

impl Default for GearRecommendation {
    fn default() -> Self {
        Self::D
    }
}

impl std::fmt::Display for GearRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P => write!(f, "P"),
            Self::N => write!(f, "N"),
            Self::D => write!(f, "D"),
            Self::S => write!(f, "S"),
        }
    }
}

/// A single CLOSE dimension evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEDimension {
    /// Score from 0-10
    pub score: f64,
    /// Confidence in this evaluation (0.0-1.0)
    pub confidence: f64,
    /// Factors that contributed to this score
    pub factors: Vec<String>,
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
    pub total: f64,
    /// Computed risk score (0-10)
    pub risk: f64,
    /// Overall confidence (0.0-1.0)
    pub confidence: f64,
    /// Recommended gear based on evaluation
    pub recommended_gear: GearRecommendation,
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
            recommended_gear: GearRecommendation::D,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Weights for each CLOSE dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEWeights {
    pub convergence: f64,
    pub leverage: f64,
    pub optionality: f64,
    pub surplus: f64,
    pub evolution: f64,
}

impl Default for CLOSEWeights {
    fn default() -> Self {
        Self {
            convergence: 1.0,
            leverage: 1.2,
            optionality: 1.5, // Highest weight: "能否再来一次" is key
            surplus: 1.3,
            evolution: 0.8,
        }
    }
}

/// Input data for CLOSE evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CLOSEInput {
    // Convergence factors
    /// Snapshot confidence (0.0-1.0)
    pub snapshot_confidence: f64,
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
    pub decision_quality: f64,
    /// Number of recent errors
    pub recent_errors: usize,

    // Surplus factors
    /// Token usage
    pub token_usage: usize,
    /// Cost in dollars
    pub cost: f64,
    /// Consensus strength (0.0-1.0)
    pub consensus_strength: f64,
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

    // Historical trend (optional)
    /// Recent average total score (for evolution)
    pub recent_trend_avg: Option<f64>,
    /// Older average total score (for evolution)
    pub older_trend_avg: Option<f64>,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CLOSETrend {
    Improving,
    Declining,
    Stable,
}

impl Default for CLOSETrend {
    fn default() -> Self {
        Self::Stable
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Evaluation Functions
// ══════════════════════════════════════════════════════════════════════════════

/// Evaluate CLOSE dimensions from input
pub fn evaluate_close(input: &CLOSEInput, weights: Option<CLOSEWeights>) -> CLOSEEvaluation {
    let weights = weights.unwrap_or_default();

    let convergence = evaluate_convergence(input);
    let leverage = evaluate_leverage(input);
    let optionality = evaluate_optionality(input);
    let surplus = evaluate_surplus(input);
    let evolution = evaluate_evolution(input);

    // Calculate weighted total
    let weight_sum = weights.convergence
        + weights.leverage
        + weights.optionality
        + weights.surplus
        + weights.evolution;

    let raw_total = convergence.score * weights.convergence
        + leverage.score * weights.leverage
        + optionality.score * weights.optionality
        + surplus.score * weights.surplus
        + evolution.score * weights.evolution;

    let total = (raw_total / weight_sum).clamp(0.0, 10.0);

    // Calculate risk
    let risk = calculate_risk(&convergence, &optionality, &surplus, input);

    // Calculate overall confidence
    let confidence = calculate_overall_confidence(&convergence, &leverage, &optionality, &surplus, &evolution);

    // Recommend gear
    let recommended_gear = recommend_gear(total, risk, confidence);

    CLOSEEvaluation {
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
    }
}

fn evaluate_convergence(input: &CLOSEInput) -> CLOSEDimension {
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
        score -= input.critical_anomalies as f64 * 1.5;
        factors.push(format!("{} critical anomalies", input.critical_anomalies));
    }

    // Pattern support
    if input.strong_patterns > 0 {
        score += input.strong_patterns as f64 * 0.3;
        factors.push(format!("{} strong patterns", input.strong_patterns));
    }

    CLOSEDimension {
        score: score.clamp(0.0, 10.0),
        confidence: input.snapshot_confidence,
        factors,
    }
}

fn evaluate_leverage(input: &CLOSEInput) -> CLOSEDimension {
    let mut factors = Vec::new();
    let mut score = 5.0;

    // High-impact opportunities
    score += input.high_impact_opportunities as f64 * 1.5;
    if input.high_impact_opportunities > 0 {
        factors.push(format!(
            "{} high-impact opportunities",
            input.high_impact_opportunities
        ));
    }

    // Medium-impact opportunities
    score += input.medium_impact_opportunities as f64 * 0.5;
    if input.medium_impact_opportunities > 0 {
        factors.push(format!(
            "{} medium-impact opportunities",
            input.medium_impact_opportunities
        ));
    }

    // External opportunities
    if input.external_opportunities > 0 {
        score += (input.external_opportunities as f64 * 0.5).min(2.0);
        factors.push(format!(
            "{} external opportunities",
            input.external_opportunities
        ));
    }

    // External risks reduce leverage
    if input.external_risks > 0 {
        score -= (input.external_risks as f64 * 0.5).min(2.0);
        factors.push(format!("{} external risks", input.external_risks));
    }

    CLOSEDimension {
        score: score.clamp(0.0, 10.0),
        confidence: input.snapshot_confidence * 0.9,
        factors,
    }
}

fn evaluate_optionality(input: &CLOSEInput) -> CLOSEDimension {
    let mut factors = Vec::new();
    let mut score = 5.0;

    // More opportunities = more options
    score += (input.total_opportunities as f64 * 0.5).min(3.0);
    if input.total_opportunities > 0 {
        factors.push(format!("{} opportunities available", input.total_opportunities));
    }

    // Diverse patterns suggest more paths
    score += (input.pattern_types as f64 * 0.5).min(2.0);
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

fn evaluate_surplus(input: &CLOSEInput) -> CLOSEDimension {
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
        score -= input.coverage_gaps as f64 * 0.3;
        factors.push(format!("{} coverage gaps", input.coverage_gaps));
    }

    CLOSEDimension {
        score: score.clamp(0.0, 10.0),
        confidence: input.snapshot_confidence * 0.9,
        factors,
    }
}

fn evaluate_evolution(input: &CLOSEInput) -> CLOSEDimension {
    let mut factors = Vec::new();
    let mut score = 5.0;

    // Historical trend
    let trend = compute_trend(input);
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
        score += input.learning_opportunities as f64 * 0.5;
        factors.push(format!(
            "{} learning opportunities",
            input.learning_opportunities
        ));
    }

    // Active development
    if input.recent_changes > 0 {
        score += (input.recent_changes as f64 * 0.2).min(1.5);
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

fn compute_trend(input: &CLOSEInput) -> CLOSETrend {
    match (input.recent_trend_avg, input.older_trend_avg) {
        (Some(recent), Some(older)) => {
            if recent > older + 0.5 {
                CLOSETrend::Improving
            } else if recent < older - 0.5 {
                CLOSETrend::Declining
            } else {
                CLOSETrend::Stable
            }
        }
        _ => CLOSETrend::Stable,
    }
}

fn calculate_risk(
    convergence: &CLOSEDimension,
    optionality: &CLOSEDimension,
    surplus: &CLOSEDimension,
    input: &CLOSEInput,
) -> f64 {
    let mut risk = 0.0;

    // Low convergence = high risk
    risk += (10.0 - convergence.score) * 0.3;

    // Low optionality = high risk (can't "再来一次")
    risk += (10.0 - optionality.score) * 0.4;

    // Low surplus = high risk (no margin for error)
    risk += (10.0 - surplus.score) * 0.3;

    // Critical anomalies are direct risk factors
    risk += input.critical_anomalies as f64 * 1.5;

    risk.clamp(0.0, 10.0)
}

fn calculate_overall_confidence(
    convergence: &CLOSEDimension,
    leverage: &CLOSEDimension,
    optionality: &CLOSEDimension,
    surplus: &CLOSEDimension,
    evolution: &CLOSEDimension,
) -> f64 {
    let confidences = [
        convergence.confidence,
        leverage.confidence,
        optionality.confidence,
        surplus.confidence,
        evolution.confidence,
    ];
    confidences.iter().sum::<f64>() / confidences.len() as f64
}

fn recommend_gear(total: f64, risk: f64, confidence: f64) -> GearRecommendation {
    // Low confidence = stay conservative
    if confidence < 0.3 {
        return GearRecommendation::N;
    }

    // High risk = reduce autonomy
    if risk > 7.0 {
        return GearRecommendation::N;
    }
    if risk > 5.0 {
        return GearRecommendation::D;
    }

    // Based on total score
    if total >= 8.0 && risk < 4.0 {
        GearRecommendation::S
    } else if total >= 5.0 {
        GearRecommendation::D
    } else if total >= 3.0 {
        GearRecommendation::N
    } else {
        GearRecommendation::P
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_evaluation_default() {
        let input = CLOSEInput::default();
        let result = evaluate_close(&input, None);

        assert!(result.total >= 0.0 && result.total <= 10.0);
        assert!(result.risk >= 0.0 && result.risk <= 10.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_close_evaluation_healthy() {
        let input = CLOSEInput {
            snapshot_confidence: 0.9,
            build_status: "passing".to_string(),
            session_health: "healthy".to_string(),
            high_impact_opportunities: 3,
            total_opportunities: 5,
            consensus_strength: 0.8,
            ..Default::default()
        };
        let result = evaluate_close(&input, None);

        assert!(result.total > 5.0);
        assert!(result.risk < 5.0);
        assert_eq!(result.recommended_gear, GearRecommendation::S);
    }

    #[test]
    fn test_close_evaluation_risky() {
        let input = CLOSEInput {
            snapshot_confidence: 0.3,
            build_status: "failing".to_string(),
            session_health: "critical".to_string(),
            critical_anomalies: 5,
            external_risks: 3,
            ..Default::default()
        };
        let result = evaluate_close(&input, None);

        assert!(result.total < 5.0);
        assert!(result.risk > 5.0);
        assert!(matches!(
            result.recommended_gear,
            GearRecommendation::N | GearRecommendation::P
        ));
    }

    #[test]
    fn test_gear_recommendation_display() {
        assert_eq!(GearRecommendation::P.to_string(), "P");
        assert_eq!(GearRecommendation::D.to_string(), "D");
        assert_eq!(GearRecommendation::S.to_string(), "S");
    }

    #[test]
    fn test_trend_calculation() {
        let input_improving = CLOSEInput {
            recent_trend_avg: Some(7.0),
            older_trend_avg: Some(5.0),
            ..Default::default()
        };
        assert_eq!(compute_trend(&input_improving), CLOSETrend::Improving);

        let input_declining = CLOSEInput {
            recent_trend_avg: Some(4.0),
            older_trend_avg: Some(7.0),
            ..Default::default()
        };
        assert_eq!(compute_trend(&input_declining), CLOSETrend::Declining);
    }
}
