//! SMT (Smart Money Technique) divergence detection.
//!
//! SMT divergence occurs when correlated instruments fail to confirm each other:
//! - **Bullish SMT**: Primary makes a lower low, but reference makes a higher low
//! - **Bearish SMT**: Primary makes a higher high, but reference makes a lower high
//!
//! # Common A-Share Pairs
//!
//! | Primary | Reference | Use Case |
//! |---------|-----------|----------|
//! | CSI 300 | CSI 500 | Large vs Mid-cap rotation |
//! | SSE 50 | STAR 50 | Blue-chip vs Growth |
//! | Securities ETF | Bank ETF | Financial sector leading |

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::data::Candle;

/// Type of SMT divergence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceType {
    /// Primary lower low, reference higher low -> Expect reversal up
    Bullish,
    /// Primary higher high, reference lower high -> Expect reversal down
    Bearish,
}

/// Detected SMT divergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtDivergence {
    /// Type of divergence
    pub divergence_type: DivergenceType,
    /// Primary symbol
    pub primary_symbol: String,
    /// Reference symbol
    pub reference_symbol: String,
    /// Primary extreme price (the new high/low)
    pub primary_extreme: f64,
    /// Previous primary extreme
    pub primary_prev_extreme: f64,
    /// Reference extreme price (the non-confirming level)
    pub reference_extreme: f64,
    /// Previous reference extreme
    pub reference_prev_extreme: f64,
    /// Timestamp of detection
    pub detected_at: DateTime<Utc>,
    /// Bars since divergence started
    pub bars_ago: usize,
    /// Strength of the divergence (0-100)
    pub strength: u8,
}

/// SMT divergence detector
pub struct SmtDetector {
    /// Lookback period for finding swing highs/lows
    lookback_period: usize,
    /// Minimum bars between swings
    #[allow(dead_code)] // Reserved for swing validation logic
    min_swing_separation: usize,
}

impl SmtDetector {
    /// Create a new SMT detector with default settings
    pub fn new() -> Self {
        Self {
            lookback_period: 20,
            min_swing_separation: 3,
        }
    }

    /// Create with custom settings
    pub fn with_settings(lookback_period: usize, min_swing_separation: usize) -> Self {
        Self {
            lookback_period,
            min_swing_separation,
        }
    }

    /// Detect SMT divergence between two correlated instruments
    pub fn detect_divergence(
        &self,
        primary: &[Candle],
        reference: &[Candle],
    ) -> Option<SmtDivergence> {
        if primary.len() < self.lookback_period || reference.len() < self.lookback_period {
            return None;
        }

        // Find swing highs and lows for both
        let primary_swings = self.find_swings(primary);
        let reference_swings = self.find_swings(reference);

        // Check for bearish divergence (higher high vs lower high)
        if let Some(div) = self.check_bearish_divergence(
            primary,
            reference,
            &primary_swings,
            &reference_swings,
        ) {
            return Some(div);
        }

        // Check for bullish divergence (lower low vs higher low)
        if let Some(div) = self.check_bullish_divergence(
            primary,
            reference,
            &primary_swings,
            &reference_swings,
        ) {
            return Some(div);
        }

        None
    }

    /// Find swing highs and lows in candle data
    fn find_swings(&self, candles: &[Candle]) -> Swings {
        let mut highs = Vec::new();
        let mut lows = Vec::new();

        for i in 2..candles.len() - 2 {
            let prev2 = &candles[i - 2];
            let prev1 = &candles[i - 1];
            let curr = &candles[i];
            let next1 = &candles[i + 1];
            let next2 = &candles[i + 2];

            // Swing high: current high is higher than surrounding 2 bars
            if curr.high > prev1.high
                && curr.high > prev2.high
                && curr.high > next1.high
                && curr.high > next2.high
            {
                highs.push(SwingPoint {
                    index: i,
                    price: curr.high,
                    timestamp: curr.timestamp,
                });
            }

            // Swing low: current low is lower than surrounding 2 bars
            if curr.low < prev1.low
                && curr.low < prev2.low
                && curr.low < next1.low
                && curr.low < next2.low
            {
                lows.push(SwingPoint {
                    index: i,
                    price: curr.low,
                    timestamp: curr.timestamp,
                });
            }
        }

        Swings { highs, lows }
    }

    /// Check for bearish SMT divergence
    fn check_bearish_divergence(
        &self,
        primary: &[Candle],
        reference: &[Candle],
        primary_swings: &Swings,
        reference_swings: &Swings,
    ) -> Option<SmtDivergence> {
        // Need at least 2 swing highs to compare
        if primary_swings.highs.len() < 2 || reference_swings.highs.len() < 2 {
            return None;
        }

        // Get the last two swing highs
        let p_curr = primary_swings.highs.last()?;
        let p_prev = &primary_swings.highs[primary_swings.highs.len() - 2];

        // Find corresponding reference swings (by approximate time)
        let r_curr = self.find_nearest_swing(&reference_swings.highs, p_curr.index, reference.len())?;
        let r_prev = self.find_nearest_swing(&reference_swings.highs, p_prev.index, reference.len())?;

        // Check for divergence: primary higher high, reference lower high
        if p_curr.price > p_prev.price && r_curr.price < r_prev.price {
            let strength = self.calculate_divergence_strength(
                p_curr.price,
                p_prev.price,
                r_curr.price,
                r_prev.price,
            );

            return Some(SmtDivergence {
                divergence_type: DivergenceType::Bearish,
                primary_symbol: primary.first().map(|c| c.symbol.clone()).unwrap_or_default(),
                reference_symbol: reference.first().map(|c| c.symbol.clone()).unwrap_or_default(),
                primary_extreme: p_curr.price,
                primary_prev_extreme: p_prev.price,
                reference_extreme: r_curr.price,
                reference_prev_extreme: r_prev.price,
                detected_at: Utc::now(),
                bars_ago: primary.len() - p_curr.index,
                strength,
            });
        }

        None
    }

    /// Check for bullish SMT divergence
    fn check_bullish_divergence(
        &self,
        primary: &[Candle],
        reference: &[Candle],
        primary_swings: &Swings,
        reference_swings: &Swings,
    ) -> Option<SmtDivergence> {
        // Need at least 2 swing lows to compare
        if primary_swings.lows.len() < 2 || reference_swings.lows.len() < 2 {
            return None;
        }

        // Get the last two swing lows
        let p_curr = primary_swings.lows.last()?;
        let p_prev = &primary_swings.lows[primary_swings.lows.len() - 2];

        // Find corresponding reference swings
        let r_curr = self.find_nearest_swing(&reference_swings.lows, p_curr.index, reference.len())?;
        let r_prev = self.find_nearest_swing(&reference_swings.lows, p_prev.index, reference.len())?;

        // Check for divergence: primary lower low, reference higher low
        if p_curr.price < p_prev.price && r_curr.price > r_prev.price {
            let strength = self.calculate_divergence_strength(
                p_prev.price,
                p_curr.price,
                r_prev.price,
                r_curr.price,
            );

            return Some(SmtDivergence {
                divergence_type: DivergenceType::Bullish,
                primary_symbol: primary.first().map(|c| c.symbol.clone()).unwrap_or_default(),
                reference_symbol: reference.first().map(|c| c.symbol.clone()).unwrap_or_default(),
                primary_extreme: p_curr.price,
                primary_prev_extreme: p_prev.price,
                reference_extreme: r_curr.price,
                reference_prev_extreme: r_prev.price,
                detected_at: Utc::now(),
                bars_ago: primary.len() - p_curr.index,
                strength,
            });
        }

        None
    }

    /// Find the nearest swing point by index
    fn find_nearest_swing(
        &self,
        swings: &[SwingPoint],
        target_index: usize,
        _max_len: usize, // Reserved for bounds checking
    ) -> Option<SwingPoint> {
        let tolerance = self.lookback_period / 2;

        swings
            .iter()
            .filter(|s| {
                let diff = if s.index > target_index {
                    s.index - target_index
                } else {
                    target_index - s.index
                };
                diff <= tolerance
            })
            .min_by_key(|s| {
                if s.index > target_index {
                    s.index - target_index
                } else {
                    target_index - s.index
                }
            })
            .cloned()
    }

    /// Calculate divergence strength (0-100)
    fn calculate_divergence_strength(
        &self,
        p_higher: f64,
        p_lower: f64,
        r_higher: f64,
        r_lower: f64,
    ) -> u8 {
        // Strength based on how significant the divergence is
        let p_change = (p_higher - p_lower).abs() / p_lower * 100.0;
        let r_change = (r_higher - r_lower).abs() / r_lower * 100.0;

        // Average percentage change, capped at 100
        let avg = (p_change + r_change) / 2.0;
        (avg.min(100.0) as u8).max(10)
    }
}

impl Default for SmtDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of swing highs and lows
#[derive(Debug)]
struct Swings {
    highs: Vec<SwingPoint>,
    lows: Vec<SwingPoint>,
}

/// A swing high or low point
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future time-based analysis
struct SwingPoint {
    index: usize,
    price: f64,
    timestamp: DateTime<Utc>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Timeframe;

    fn make_candle(high: f64, low: f64) -> Candle {
        Candle {
            symbol: "TEST".to_string(),
            timeframe: Timeframe::Daily,
            timestamp: Utc::now(),
            open: (high + low) / 2.0,
            high,
            low,
            close: (high + low) / 2.0,
            volume: 1000.0,
            amount: 10000.0,
        }
    }

    #[test]
    fn test_smt_detector_creation() {
        let detector = SmtDetector::new();
        assert_eq!(detector.lookback_period, 20);
        assert_eq!(detector.min_swing_separation, 3);
    }

    #[test]
    fn test_find_swings() {
        // Create candles with clear swing high at index 5
        let candles: Vec<Candle> = vec![
            make_candle(10.0, 9.0),
            make_candle(10.5, 9.5),
            make_candle(11.0, 10.0),
            make_candle(11.5, 10.5),
            make_candle(12.0, 11.0),
            make_candle(13.0, 12.0), // Swing high
            make_candle(12.5, 11.5),
            make_candle(12.0, 11.0),
            make_candle(11.5, 10.5),
            make_candle(11.0, 10.0),
        ];

        let detector = SmtDetector::new();
        let swings = detector.find_swings(&candles);

        assert!(!swings.highs.is_empty());
    }

    #[test]
    fn test_divergence_type_serialization() {
        let div_type = DivergenceType::Bullish;
        let json = serde_json::to_string(&div_type).unwrap();
        assert!(json.contains("Bullish"));
    }

    #[test]
    fn test_divergence_strength() {
        let detector = SmtDetector::new();

        // 10% divergence
        let strength = detector.calculate_divergence_strength(110.0, 100.0, 100.0, 105.0);
        assert!(strength >= 10 && strength <= 100);
    }
}
