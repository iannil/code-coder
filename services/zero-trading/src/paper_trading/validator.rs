//! Signal validation for paper trading.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::strategy::{TradingSignal, SignalDirection, SignalStrength};

/// Signal validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Signal ID
    pub signal_id: String,
    /// Whether signal passed validation
    pub is_valid: bool,
    /// Validation score (0-100)
    pub score: u8,
    /// Reason for pass/fail
    pub reason: String,
    /// Individual check results
    pub checks: Vec<ValidationCheck>,
    /// Timestamp of validation
    pub timestamp: DateTime<Utc>,
}

/// Individual validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Check name
    pub name: String,
    /// Whether check passed
    pub passed: bool,
    /// Check details
    pub details: String,
    /// Weight in overall score
    pub weight: u8,
}

/// Validation metrics aggregated over multiple signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Total signals validated
    pub total_signals: usize,
    /// Signals that passed
    pub passed: usize,
    /// Signals that failed
    pub failed: usize,
    /// Pass rate percentage
    pub pass_rate: f64,
    /// Average score
    pub avg_score: f64,
    /// Score distribution
    pub score_distribution: ScoreDistribution,
}

/// Score distribution buckets
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreDistribution {
    /// Scores 0-20
    pub very_low: usize,
    /// Scores 21-40
    pub low: usize,
    /// Scores 41-60
    pub medium: usize,
    /// Scores 61-80
    pub high: usize,
    /// Scores 81-100
    pub very_high: usize,
}

/// Signal validator
pub struct SignalValidator {
    /// Minimum risk:reward ratio
    min_risk_reward: f64,
    /// Maximum risk percentage
    max_risk_pct: f64,
    /// Require SMT divergence
    require_smt: bool,
    /// Require PO3 structure
    require_po3: bool,
    /// Minimum timeframe alignment count
    min_timeframe_alignment: usize,
}

impl Default for SignalValidator {
    fn default() -> Self {
        Self {
            min_risk_reward: 1.5,
            max_risk_pct: 5.0,
            require_smt: false,
            require_po3: false,
            min_timeframe_alignment: 2,
        }
    }
}

impl SignalValidator {
    /// Create a new signal validator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(
        min_risk_reward: f64,
        max_risk_pct: f64,
        require_smt: bool,
        require_po3: bool,
    ) -> Self {
        Self {
            min_risk_reward,
            max_risk_pct,
            require_smt,
            require_po3,
            min_timeframe_alignment: 2,
        }
    }

    /// Validate a trading signal
    pub fn validate(&self, signal: &TradingSignal) -> ValidationResult {
        let mut checks = Vec::new();
        let mut total_weight = 0u8;
        let mut weighted_score = 0u16;

        // Check 1: Risk/Reward ratio
        let rr = signal.risk_reward();
        let rr_passed = rr >= self.min_risk_reward;
        checks.push(ValidationCheck {
            name: "Risk/Reward".to_string(),
            passed: rr_passed,
            details: format!("R:R = {:.2}:1 (min: {:.1}:1)", rr, self.min_risk_reward),
            weight: 25,
        });
        total_weight += 25;
        if rr_passed {
            weighted_score += 25;
        }

        // Check 2: Risk percentage
        let risk_pct = signal.risk_percent();
        let risk_passed = risk_pct <= self.max_risk_pct;
        checks.push(ValidationCheck {
            name: "Risk Percentage".to_string(),
            passed: risk_passed,
            details: format!("Risk = {:.2}% (max: {:.1}%)", risk_pct, self.max_risk_pct),
            weight: 20,
        });
        total_weight += 20;
        if risk_passed {
            weighted_score += 20;
        }

        // Check 3: Signal strength
        let strength_passed = signal.strength >= SignalStrength::Medium;
        checks.push(ValidationCheck {
            name: "Signal Strength".to_string(),
            passed: strength_passed,
            details: format!("Strength = {:?}", signal.strength),
            weight: 15,
        });
        total_weight += 15;
        if strength_passed {
            weighted_score += 15;
        }

        // Check 4: Timeframe alignment
        let tf_count = signal.timeframe_alignment.len();
        let tf_passed = tf_count >= self.min_timeframe_alignment;
        checks.push(ValidationCheck {
            name: "Timeframe Alignment".to_string(),
            passed: tf_passed,
            details: format!("{} timeframes aligned (min: {})", tf_count, self.min_timeframe_alignment),
            weight: 15,
        });
        total_weight += 15;
        if tf_passed {
            weighted_score += 15;
        }

        // Check 5: PO3 structure (optional)
        let po3_present = signal.po3_structure.is_some();
        let po3_passed = !self.require_po3 || po3_present;
        checks.push(ValidationCheck {
            name: "PO3 Structure".to_string(),
            passed: po3_passed,
            details: if po3_present {
                format!("PO3 detected: {:?}", signal.po3_structure.as_ref().map(|p| &p.current_phase))
            } else {
                "No PO3 structure".to_string()
            },
            weight: 10,
        });
        total_weight += 10;
        if po3_passed {
            weighted_score += 10;
        }

        // Check 6: SMT divergence (optional)
        let smt_present = signal.smt_divergence.is_some();
        let smt_passed = !self.require_smt || smt_present;
        checks.push(ValidationCheck {
            name: "SMT Divergence".to_string(),
            passed: smt_passed,
            details: if smt_present {
                format!("SMT detected: {:?}", signal.smt_divergence.as_ref().map(|s| &s.divergence_type))
            } else {
                "No SMT divergence".to_string()
            },
            weight: 10,
        });
        total_weight += 10;
        if smt_passed {
            weighted_score += 10;
        }

        // Check 7: Signal freshness (not expired)
        let fresh = signal.is_valid(30); // 30 minutes expiry
        checks.push(ValidationCheck {
            name: "Signal Freshness".to_string(),
            passed: fresh,
            details: if fresh {
                "Signal is fresh".to_string()
            } else {
                "Signal has expired".to_string()
            },
            weight: 5,
        });
        total_weight += 5;
        if fresh {
            weighted_score += 5;
        }

        // Calculate final score
        let score = if total_weight > 0 {
            ((weighted_score as f64 / total_weight as f64) * 100.0) as u8
        } else {
            0
        };

        // Determine if valid (all required checks passed)
        let is_valid = rr_passed && risk_passed && strength_passed && tf_passed && po3_passed && smt_passed && fresh;

        // Build reason
        let reason = if is_valid {
            format!("Signal passed all checks (score: {})", score)
        } else {
            let failed_checks: Vec<_> = checks.iter()
                .filter(|c| !c.passed)
                .map(|c| c.name.as_str())
                .collect();
            format!("Failed checks: {}", failed_checks.join(", "))
        };

        ValidationResult {
            signal_id: signal.id.clone(),
            is_valid,
            score,
            reason,
            checks,
            timestamp: Utc::now(),
        }
    }

    /// Calculate metrics from multiple validations
    pub fn calculate_metrics(validations: &[ValidationResult]) -> ValidationMetrics {
        if validations.is_empty() {
            return ValidationMetrics {
                total_signals: 0,
                passed: 0,
                failed: 0,
                pass_rate: 0.0,
                avg_score: 0.0,
                score_distribution: ScoreDistribution::default(),
            };
        }

        let total = validations.len();
        let passed = validations.iter().filter(|v| v.is_valid).count();
        let failed = total - passed;
        let pass_rate = (passed as f64 / total as f64) * 100.0;
        let avg_score = validations.iter().map(|v| v.score as f64).sum::<f64>() / total as f64;

        let mut distribution = ScoreDistribution::default();
        for v in validations {
            match v.score {
                0..=20 => distribution.very_low += 1,
                21..=40 => distribution.low += 1,
                41..=60 => distribution.medium += 1,
                61..=80 => distribution.high += 1,
                81..=100 => distribution.very_high += 1,
                _ => {}
            }
        }

        ValidationMetrics {
            total_signals: total,
            passed,
            failed,
            pass_rate,
            avg_score,
            score_distribution: distribution,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Timeframe;

    fn make_test_signal() -> TradingSignal {
        TradingSignal {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: SignalDirection::Long,
            strength: SignalStrength::Strong,
            entry_price: 10.0,
            stop_loss: 9.5,  // 5% risk
            take_profit: 11.5, // 15% reward, R:R = 3:1
            timestamp: Utc::now(),
            po3_structure: None,
            smt_divergence: None,
            timeframe_alignment: vec![Timeframe::Daily, Timeframe::H4],
            notes: "Test signal".to_string(),
        }
    }

    #[test]
    fn test_validator_creation() {
        let validator = SignalValidator::new();
        assert!((validator.min_risk_reward - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_signal_validation() {
        let validator = SignalValidator::new();
        let signal = make_test_signal();
        let result = validator.validate(&signal);

        assert!(result.is_valid);
        assert!(result.score >= 70);
    }

    #[test]
    fn test_signal_validation_fail_rr() {
        let validator = SignalValidator::new();
        let mut signal = make_test_signal();
        signal.stop_loss = 9.8;  // Very tight stop, low R:R
        signal.take_profit = 10.1;

        let result = validator.validate(&signal);
        assert!(!result.is_valid);
        assert!(result.reason.contains("Risk/Reward"));
    }

    #[test]
    fn test_validation_metrics() {
        let results = vec![
            ValidationResult {
                signal_id: "1".to_string(),
                is_valid: true,
                score: 85,
                reason: "Passed".to_string(),
                checks: vec![],
                timestamp: Utc::now(),
            },
            ValidationResult {
                signal_id: "2".to_string(),
                is_valid: false,
                score: 45,
                reason: "Failed".to_string(),
                checks: vec![],
                timestamp: Utc::now(),
            },
        ];

        let metrics = SignalValidator::calculate_metrics(&results);
        assert_eq!(metrics.total_signals, 2);
        assert_eq!(metrics.passed, 1);
        assert_eq!(metrics.failed, 1);
        assert!((metrics.pass_rate - 50.0).abs() < 0.01);
    }
}
