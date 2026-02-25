//! PO3 (Power of 3) structure detection.
//!
//! The PO3 concept from ICT/Kane trading methodology:
//! 1. **Accumulation**: Sideways consolidation, building orders
//! 2. **Manipulation**: False breakout to trap traders
//! 3. **Distribution**: True reversal toward the range midpoint
//!
//! # A-Share Adaptation
//!
//! Due to T+1 rules:
//! - We look for Day 1 signals to enter before close
//! - Day 2 we evaluate the position based on opening auction

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::data::Candle;
use super::SignalDirection;

/// PO3 phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Po3Phase {
    /// Building orders in a range
    Accumulation,
    /// False breakout to trap traders
    Manipulation,
    /// True reversal, distribution toward midpoint
    Distribution,
}

/// Detected PO3 structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Po3Structure {
    /// Direction of the expected move
    pub direction: SignalDirection,
    /// Current phase
    pub current_phase: Po3Phase,
    /// Accumulation range high
    pub range_high: f64,
    /// Accumulation range low
    pub range_low: f64,
    /// Range midpoint (50% retracement target)
    pub midpoint: f64,
    /// Manipulation extreme (false breakout level)
    pub manipulation_extreme: f64,
    /// Whether manipulation breakout was clear
    pub manipulation_clear: bool,
    /// Whether distribution has started
    pub distribution_started: bool,
    /// Ideal entry price
    pub ideal_entry: f64,
    /// Stop loss level
    pub stop_loss: f64,
    /// Timestamp of detection
    pub detected_at: DateTime<Utc>,
    /// Number of candles in accumulation
    pub accumulation_bars: usize,
}

/// PO3 structure detector
pub struct Po3Detector {
    /// Minimum bars for accumulation phase
    min_accumulation_bars: usize,
    /// Manipulation threshold (ATR multiple for breakout)
    manipulation_threshold: f64,
}

impl Po3Detector {
    /// Create a new PO3 detector
    pub fn new(min_accumulation_bars: usize, manipulation_threshold: f64) -> Self {
        Self {
            min_accumulation_bars,
            manipulation_threshold,
        }
    }

    /// Detect PO3 structure in candle data
    pub fn detect(&self, candles: &[Candle]) -> Option<Po3Structure> {
        if candles.len() < self.min_accumulation_bars + 3 {
            return None;
        }

        // Calculate ATR for threshold
        let atr = self.calculate_atr(candles, 14);

        // Try to find accumulation range
        let (range, start_idx) = self.find_accumulation_range(candles)?;

        // Look for manipulation (false breakout)
        let manipulation = self.find_manipulation(candles, start_idx, &range, atr)?;

        // Check for distribution (reversal)
        let distribution = self.check_distribution(candles, &manipulation, &range);

        // Determine direction based on manipulation type
        let direction = if manipulation.broke_high {
            SignalDirection::Short // Broke high, expect reversal down
        } else {
            SignalDirection::Long // Broke low, expect reversal up
        };

        // Calculate midpoint and entry
        let midpoint = (range.high + range.low) / 2.0;
        let ideal_entry = match direction {
            SignalDirection::Long => manipulation.extreme + (atr * 0.5), // Enter on reversal
            SignalDirection::Short => manipulation.extreme - (atr * 0.5),
        };
        let stop_loss = manipulation.extreme;

        Some(Po3Structure {
            direction,
            current_phase: if distribution {
                Po3Phase::Distribution
            } else {
                Po3Phase::Manipulation
            },
            range_high: range.high,
            range_low: range.low,
            midpoint,
            manipulation_extreme: manipulation.extreme,
            manipulation_clear: manipulation.clear,
            distribution_started: distribution,
            ideal_entry,
            stop_loss,
            detected_at: Utc::now(),
            accumulation_bars: start_idx,
        })
    }

    /// Find the accumulation range (consolidation zone)
    fn find_accumulation_range(&self, candles: &[Candle]) -> Option<(ConsolidationRange, usize)> {
        // Look back from recent candles
        let lookback = candles.len().min(50);
        let recent = &candles[candles.len() - lookback..];

        // Find potential consolidation zones
        for window_size in (self.min_accumulation_bars..=20).rev() {
            for i in 0..=(recent.len() - window_size - 3) {
                let window = &recent[i..i + window_size];

                // Calculate range metrics
                let high = window.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
                let low = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
                let range_size = high - low;

                // Check if this is a consolidation (relatively tight range)
                let avg_body: f64 = window.iter().map(|c| c.body_size()).sum::<f64>() / window.len() as f64;
                let avg_range: f64 = window.iter().map(|c| c.range()).sum::<f64>() / window.len() as f64;

                // Consolidation criteria:
                // 1. Range should be relatively tight
                // 2. Average body should be small compared to range
                let is_tight = range_size < avg_range * window.len() as f64 * 0.5;
                let small_bodies = avg_body < avg_range * 0.6;

                if is_tight && small_bodies {
                    return Some((
                        ConsolidationRange { high, low },
                        i + window_size,
                    ));
                }
            }
        }

        None
    }

    /// Find manipulation (false breakout)
    fn find_manipulation(
        &self,
        candles: &[Candle],
        start_idx: usize,
        range: &ConsolidationRange,
        atr: f64,
    ) -> Option<ManipulationBreakout> {
        let threshold = atr * self.manipulation_threshold;

        // Look at candles after the consolidation
        let post_consolidation = &candles[candles.len().saturating_sub(candles.len() - start_idx)..];
        if post_consolidation.is_empty() {
            return None;
        }

        // Check for breakout above high
        for (i, candle) in post_consolidation.iter().enumerate() {
            // Breakout above
            if candle.high > range.high + threshold {
                // Check if it's a false breakout (closed back inside or below)
                let closed_back = candle.close < range.high || (i + 1 < post_consolidation.len() && post_consolidation[i + 1].close < range.high);

                if closed_back {
                    return Some(ManipulationBreakout {
                        broke_high: true,
                        extreme: candle.high,
                        clear: candle.high > range.high + threshold * 1.5,
                    });
                }
            }

            // Breakout below
            if candle.low < range.low - threshold {
                let closed_back = candle.close > range.low || (i + 1 < post_consolidation.len() && post_consolidation[i + 1].close > range.low);

                if closed_back {
                    return Some(ManipulationBreakout {
                        broke_high: false,
                        extreme: candle.low,
                        clear: candle.low < range.low - threshold * 1.5,
                    });
                }
            }
        }

        None
    }

    /// Check if distribution (reversal) has started
    fn check_distribution(
        &self,
        candles: &[Candle],
        manipulation: &ManipulationBreakout,
        range: &ConsolidationRange,
    ) -> bool {
        if candles.len() < 2 {
            return false;
        }

        let last = &candles[candles.len() - 1];
        let midpoint = (range.high + range.low) / 2.0;

        // Distribution started when price moves back toward midpoint
        if manipulation.broke_high {
            // After false high breakout, looking for move down
            last.close < manipulation.extreme && last.close > midpoint - (range.high - range.low) * 0.25
        } else {
            // After false low breakout, looking for move up
            last.close > manipulation.extreme && last.close < midpoint + (range.high - range.low) * 0.25
        }
    }

    /// Calculate Average True Range
    fn calculate_atr(&self, candles: &[Candle], period: usize) -> f64 {
        if candles.len() < period + 1 {
            return candles.last().map(|c| c.range()).unwrap_or(1.0);
        }

        let start = candles.len() - period - 1;
        let tr_sum: f64 = candles[start..candles.len() - 1]
            .windows(2)
            .map(|w| {
                let prev = &w[0];
                let curr = &w[1];
                let tr1 = curr.high - curr.low;
                let tr2 = (curr.high - prev.close).abs();
                let tr3 = (curr.low - prev.close).abs();
                tr1.max(tr2).max(tr3)
            })
            .sum();

        tr_sum / period as f64
    }
}

/// Consolidation range
#[derive(Debug, Clone)]
struct ConsolidationRange {
    high: f64,
    low: f64,
}

/// Manipulation breakout info
#[derive(Debug, Clone)]
struct ManipulationBreakout {
    broke_high: bool,
    extreme: f64,
    clear: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Timeframe;

    fn make_candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            symbol: "TEST".to_string(),
            timeframe: Timeframe::Daily,
            timestamp: Utc::now(),
            open,
            high,
            low,
            close,
            volume: 1000.0,
            amount: 10000.0,
        }
    }

    #[test]
    fn test_po3_detector_creation() {
        let detector = Po3Detector::new(5, 1.5);
        assert_eq!(detector.min_accumulation_bars, 5);
        assert!((detector.manipulation_threshold - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_atr_calculation() {
        let candles = vec![
            make_candle(10.0, 11.0, 9.5, 10.5),
            make_candle(10.5, 11.5, 10.0, 11.0),
            make_candle(11.0, 12.0, 10.5, 11.5),
            make_candle(11.5, 12.5, 11.0, 12.0),
            make_candle(12.0, 13.0, 11.5, 12.5),
        ];

        let detector = Po3Detector::new(5, 1.5);
        let atr = detector.calculate_atr(&candles, 3);

        // ATR should be positive and reasonable for these candle ranges
        // Each candle has a range of ~1.5 but TR considers prev close
        assert!(atr > 0.5 && atr < 3.0, "ATR was {}", atr);
    }

    #[test]
    fn test_po3_phase_serialization() {
        let phase = Po3Phase::Manipulation;
        let json = serde_json::to_string(&phase).unwrap();
        assert!(json.contains("Manipulation"));
    }
}
