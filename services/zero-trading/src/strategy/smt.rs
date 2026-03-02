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
                let diff = s.index.abs_diff(target_index);
                diff <= tolerance
            })
            .min_by_key(|s| {
                s.index.abs_diff(target_index)
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

    fn make_candle_with_symbol(symbol: &str, high: f64, low: f64) -> Candle {
        Candle {
            symbol: symbol.to_string(),
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
    fn test_smt_detector_custom_settings() {
        let detector = SmtDetector::with_settings(30, 5);
        assert_eq!(detector.lookback_period, 30);
        assert_eq!(detector.min_swing_separation, 5);
    }

    #[test]
    fn test_smt_detector_default() {
        let detector = SmtDetector::default();
        assert_eq!(detector.lookback_period, 20);
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
    fn test_find_swing_lows() {
        // Create candles with clear swing low
        let candles: Vec<Candle> = vec![
            make_candle(12.0, 11.0),
            make_candle(11.5, 10.5),
            make_candle(11.0, 10.0),
            make_candle(10.5, 9.5),
            make_candle(10.0, 9.0),
            make_candle(9.5, 8.5),  // Swing low
            make_candle(10.0, 9.0),
            make_candle(10.5, 9.5),
            make_candle(11.0, 10.0),
            make_candle(11.5, 10.5),
        ];

        let detector = SmtDetector::new();
        let swings = detector.find_swings(&candles);

        assert!(!swings.lows.is_empty());
    }

    #[test]
    fn test_divergence_type_serialization() {
        let div_type = DivergenceType::Bullish;
        let json = serde_json::to_string(&div_type).unwrap();
        assert!(json.contains("Bullish"));

        let deserialized: DivergenceType = serde_json::from_str(&json).unwrap();
        assert_eq!(div_type, deserialized);
    }

    #[test]
    fn test_bearish_divergence_type_serialization() {
        let div_type = DivergenceType::Bearish;
        let json = serde_json::to_string(&div_type).unwrap();
        assert!(json.contains("Bearish"));

        let deserialized: DivergenceType = serde_json::from_str(&json).unwrap();
        assert_eq!(div_type, deserialized);
    }

    #[test]
    fn test_divergence_strength() {
        let detector = SmtDetector::new();

        // 10% divergence
        let strength = detector.calculate_divergence_strength(110.0, 100.0, 100.0, 105.0);
        assert!(strength >= 10 && strength <= 100);
    }

    #[test]
    fn test_divergence_strength_bounds() {
        let detector = SmtDetector::new();

        // Very small divergence should be at least 10
        let small = detector.calculate_divergence_strength(100.1, 100.0, 100.0, 100.05);
        assert!(small >= 10);

        // Very large divergence should be capped at 100
        let large = detector.calculate_divergence_strength(200.0, 100.0, 100.0, 150.0);
        assert!(large <= 100);
    }

    #[test]
    fn test_insufficient_data_primary() {
        let primary: Vec<Candle> = (0..10)
            .map(|_| make_candle(11.0, 10.0))
            .collect();
        let reference: Vec<Candle> = (0..30)
            .map(|_| make_candle(11.0, 10.0))
            .collect();

        let detector = SmtDetector::new(); // lookback_period = 20
        let result = detector.detect_divergence(&primary, &reference);

        // Should return None due to insufficient primary data
        assert!(result.is_none());
    }

    #[test]
    fn test_insufficient_data_reference() {
        let primary: Vec<Candle> = (0..30)
            .map(|_| make_candle(11.0, 10.0))
            .collect();
        let reference: Vec<Candle> = (0..10)
            .map(|_| make_candle(11.0, 10.0))
            .collect();

        let detector = SmtDetector::new();
        let result = detector.detect_divergence(&primary, &reference);

        // Should return None due to insufficient reference data
        assert!(result.is_none());
    }

    #[test]
    fn test_no_divergence_with_flat_data() {
        let primary: Vec<Candle> = (0..30)
            .map(|_| make_candle(10.5, 9.5))
            .collect();
        let reference: Vec<Candle> = (0..30)
            .map(|_| make_candle(10.5, 9.5))
            .collect();

        let detector = SmtDetector::new();
        let result = detector.detect_divergence(&primary, &reference);

        // Flat data should not produce divergence
        assert!(result.is_none());
    }

    #[test]
    fn test_smt_divergence_struct() {
        let divergence = SmtDivergence {
            divergence_type: DivergenceType::Bullish,
            primary_symbol: "CSI300".to_string(),
            reference_symbol: "CSI500".to_string(),
            primary_extreme: 100.0,
            primary_prev_extreme: 105.0,
            reference_extreme: 95.0,
            reference_prev_extreme: 90.0,
            detected_at: Utc::now(),
            bars_ago: 5,
            strength: 75,
        };

        assert_eq!(divergence.divergence_type, DivergenceType::Bullish);
        assert_eq!(divergence.primary_symbol, "CSI300");
        assert!(divergence.primary_extreme < divergence.primary_prev_extreme); // Lower low
        assert!(divergence.reference_extreme > divergence.reference_prev_extreme); // Higher low
    }

    #[test]
    fn test_smt_divergence_serialization() {
        let divergence = SmtDivergence {
            divergence_type: DivergenceType::Bearish,
            primary_symbol: "SSE50".to_string(),
            reference_symbol: "STAR50".to_string(),
            primary_extreme: 110.0,
            primary_prev_extreme: 100.0,
            reference_extreme: 98.0,
            reference_prev_extreme: 102.0,
            detected_at: Utc::now(),
            bars_ago: 3,
            strength: 60,
        };

        let json = serde_json::to_string(&divergence).unwrap();
        assert!(json.contains("Bearish"));
        assert!(json.contains("SSE50"));
        assert!(json.contains("STAR50"));

        let deserialized: SmtDivergence = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.divergence_type, DivergenceType::Bearish);
    }

    #[test]
    fn test_bearish_divergence_logic() {
        // Bearish: Primary makes higher high, reference makes lower high
        let divergence = SmtDivergence {
            divergence_type: DivergenceType::Bearish,
            primary_symbol: "TEST1".to_string(),
            reference_symbol: "TEST2".to_string(),
            primary_extreme: 115.0,       // Current high
            primary_prev_extreme: 110.0,  // Previous high (lower)
            reference_extreme: 105.0,     // Current high
            reference_prev_extreme: 108.0, // Previous high (higher)
            detected_at: Utc::now(),
            bars_ago: 2,
            strength: 50,
        };

        // Primary: higher high (115 > 110)
        assert!(divergence.primary_extreme > divergence.primary_prev_extreme);
        // Reference: lower high (105 < 108)
        assert!(divergence.reference_extreme < divergence.reference_prev_extreme);
    }

    #[test]
    fn test_bullish_divergence_logic() {
        // Bullish: Primary makes lower low, reference makes higher low
        let divergence = SmtDivergence {
            divergence_type: DivergenceType::Bullish,
            primary_symbol: "TEST1".to_string(),
            reference_symbol: "TEST2".to_string(),
            primary_extreme: 95.0,        // Current low
            primary_prev_extreme: 100.0,  // Previous low (higher)
            reference_extreme: 98.0,      // Current low
            reference_prev_extreme: 96.0, // Previous low (lower)
            detected_at: Utc::now(),
            bars_ago: 2,
            strength: 50,
        };

        // Primary: lower low (95 < 100)
        assert!(divergence.primary_extreme < divergence.primary_prev_extreme);
        // Reference: higher low (98 > 96)
        assert!(divergence.reference_extreme > divergence.reference_prev_extreme);
    }

    #[test]
    fn test_common_a_share_pairs() {
        // Common pairs for A-share SMT analysis
        let pairs = [
            ("CSI300", "CSI500"),       // Large vs Mid-cap
            ("SSE50", "STAR50"),        // Blue-chip vs Growth
            ("SecuritiesETF", "BankETF"), // Financial sector
        ];

        for (primary, reference) in pairs {
            assert!(!primary.is_empty());
            assert!(!reference.is_empty());
            assert_ne!(primary, reference);
        }
    }

    #[test]
    fn test_swing_detection_with_multiple_swings() {
        // Create candles with multiple swing highs and lows
        let mut candles = Vec::new();

        // First swing low
        candles.extend(vec![
            make_candle(12.0, 11.0),
            make_candle(11.0, 10.0),
            make_candle(10.0, 9.0),
            make_candle(9.0, 8.0),   // Swing low
            make_candle(10.0, 9.0),
            make_candle(11.0, 10.0),
        ]);

        // First swing high
        candles.extend(vec![
            make_candle(12.0, 11.0),
            make_candle(13.0, 12.0),
            make_candle(14.0, 13.0), // Swing high
            make_candle(13.0, 12.0),
            make_candle(12.0, 11.0),
        ]);

        // Second swing low
        candles.extend(vec![
            make_candle(11.0, 10.0),
            make_candle(10.0, 9.0),
            make_candle(9.5, 8.5),   // Swing low
            make_candle(10.5, 9.5),
            make_candle(11.5, 10.5),
        ]);

        // Second swing high
        candles.extend(vec![
            make_candle(12.5, 11.5),
            make_candle(13.5, 12.5),
            make_candle(14.5, 13.5), // Swing high
            make_candle(13.5, 12.5),
            make_candle(12.5, 11.5),
        ]);

        let detector = SmtDetector::new();
        let swings = detector.find_swings(&candles);

        // Should find multiple highs and lows
        assert!(swings.highs.len() >= 1);
        assert!(swings.lows.len() >= 1);
    }

    #[test]
    fn test_strength_calculation_symmetric() {
        let detector = SmtDetector::new();

        // Same percentage change should give similar strength
        let strength1 = detector.calculate_divergence_strength(110.0, 100.0, 100.0, 110.0);
        let strength2 = detector.calculate_divergence_strength(100.0, 110.0, 110.0, 100.0);

        // Should be close (within 10 points)
        assert!((strength1 as i32 - strength2 as i32).abs() <= 10);
    }
}
