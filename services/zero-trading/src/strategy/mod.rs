//! Trading strategy module.
//!
//! Implements the PO3 (Power of 3) + SMT (Smart Money Technique) divergence strategy.

mod po3;
mod smt;
mod signal;

pub use po3::{Po3Detector, Po3Structure, Po3Phase};
pub use smt::{SmtDetector, SmtDivergence, DivergenceType};
pub use signal::{TradingSignal, SignalDirection, SignalStrength};

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use zero_common::config::Config;

use crate::data::{MarketDataAggregator, SmtPair, Timeframe};

/// Multi-timeframe PO3+SMT strategy engine
pub struct StrategyEngine {
    /// PO3 detector
    po3: Po3Detector,
    /// SMT detector
    smt: SmtDetector,
    /// Active signals
    signals: Arc<RwLock<Vec<TradingSignal>>>,
    /// Last scan timestamp
    last_scan: RwLock<Option<DateTime<Utc>>>,
    /// Configuration
    config: StrategyConfig,
}

/// Strategy configuration
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    /// Minimum accumulation bars for PO3
    pub min_accumulation_bars: usize,
    /// Manipulation threshold (ATR multiple)
    pub manipulation_threshold: f64,
    /// Timeframes for multi-timeframe analysis
    pub timeframes: Vec<Timeframe>,
    /// Require all timeframes to align
    pub require_alignment: bool,
    /// Signal expiry in minutes
    pub signal_expiry_minutes: i64,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            min_accumulation_bars: 5,
            manipulation_threshold: 1.5,
            timeframes: vec![Timeframe::Daily],
            require_alignment: true,
            signal_expiry_minutes: 60,
        }
    }
}

impl StrategyConfig {
    /// Create from config
    pub fn from_config(config: &Config) -> Self {
        config
            .trading
            .as_ref()
            .map(|t| {
                // Convert string timeframes to Timeframe enum
                let timeframes = t.timeframes.as_ref()
                    .map(|tfs| {
                        tfs.iter()
                            .filter_map(|s| Timeframe::from_str(s))
                            .collect()
                    })
                    .unwrap_or_else(|| vec![Timeframe::Daily]);

                Self {
                    min_accumulation_bars: t.min_accumulation_bars.unwrap_or(5),
                    manipulation_threshold: t.manipulation_threshold.unwrap_or(1.5),
                    timeframes,
                    require_alignment: t.require_alignment.unwrap_or(true),
                    signal_expiry_minutes: t.signal_expiry_minutes.unwrap_or(60),
                }
            })
            .unwrap_or_default()
    }
}

impl StrategyEngine {
    /// Create a new strategy engine
    pub fn new(config: &Config) -> Self {
        let strategy_config = StrategyConfig::from_config(config);

        Self {
            po3: Po3Detector::new(
                strategy_config.min_accumulation_bars,
                strategy_config.manipulation_threshold,
            ),
            smt: SmtDetector::new(),
            signals: Arc::new(RwLock::new(Vec::new())),
            last_scan: RwLock::new(None),
            config: strategy_config,
        }
    }

    /// Scan for trading signals across all configured pairs
    pub async fn scan_for_signals(&self, data: &MarketDataAggregator) -> Result<Vec<TradingSignal>> {
        let mut new_signals = Vec::new();

        // Scan each SMT pair
        for pair in data.get_smt_pairs() {
            if let Some(signal) = self.scan_pair(data, pair).await? {
                new_signals.push(signal);
            }
        }

        // Update active signals
        {
            let mut signals = self.signals.write().await;

            // Remove expired signals
            let now = Utc::now();
            let expiry = chrono::Duration::minutes(self.config.signal_expiry_minutes);
            signals.retain(|s| now.signed_duration_since(s.timestamp) < expiry);

            // Add new signals
            for signal in &new_signals {
                // Avoid duplicates
                if !signals.iter().any(|s| s.symbol == signal.symbol && s.direction == signal.direction) {
                    signals.push(signal.clone());
                }
            }
        }

        // Update last scan time
        *self.last_scan.write().await = Some(Utc::now());

        Ok(new_signals)
    }

    /// Scan a single SMT pair for signals
    async fn scan_pair(
        &self,
        data: &MarketDataAggregator,
        pair: &SmtPair,
    ) -> Result<Option<TradingSignal>> {
        // Check for multi-timeframe alignment
        let mut po3_signals: Vec<(Timeframe, Po3Structure)> = Vec::new();
        let mut smt_divergence: Option<SmtDivergence> = None;

        for tf in &self.config.timeframes {
            // Get candles for both symbols
            let (primary, reference) = data.get_smt_pair_data(pair, *tf, 100).await?;

            // Check for PO3 structure on primary
            if let Some(po3) = self.po3.detect(&primary) {
                po3_signals.push((*tf, po3));
            }

            // Check for SMT divergence (only on higher timeframes)
            if *tf == Timeframe::Daily || *tf == Timeframe::H4 {
                if let Some(div) = self.smt.detect_divergence(&primary, &reference) {
                    smt_divergence = Some(div);
                }
            }
        }

        // Generate signal if criteria met
        self.evaluate_signals(&pair.primary, &po3_signals, smt_divergence)
    }

    /// Evaluate whether we have a valid trading signal
    fn evaluate_signals(
        &self,
        symbol: &str,
        po3_signals: &[(Timeframe, Po3Structure)],
        smt_divergence: Option<SmtDivergence>,
    ) -> Result<Option<TradingSignal>> {
        if po3_signals.is_empty() {
            return Ok(None);
        }

        // Check multi-timeframe alignment
        if self.config.require_alignment && po3_signals.len() < 2 {
            return Ok(None);
        }

        // Determine direction from the highest timeframe PO3
        let primary_po3 = &po3_signals[0].1;
        let direction = primary_po3.direction;

        // Calculate signal strength
        let strength = self.calculate_strength(po3_signals, &smt_divergence);

        // Check SMT divergence alignment
        if let Some(ref div) = smt_divergence {
            // SMT must align with PO3 direction
            let smt_direction = match div.divergence_type {
                DivergenceType::Bullish => SignalDirection::Long,
                DivergenceType::Bearish => SignalDirection::Short,
            };

            if smt_direction != direction {
                return Ok(None);
            }
        }

        // Generate notes before moving smt_divergence
        let notes = self.generate_notes(po3_signals, &smt_divergence);

        // Generate signal
        let signal = TradingSignal {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            direction,
            strength,
            entry_price: primary_po3.ideal_entry,
            stop_loss: primary_po3.stop_loss,
            take_profit: primary_po3.midpoint,
            timestamp: Utc::now(),
            po3_structure: Some(primary_po3.clone()),
            smt_divergence,
            timeframe_alignment: po3_signals.iter().map(|(tf, _)| *tf).collect(),
            notes,
        };

        tracing::info!(
            symbol,
            direction = ?signal.direction,
            strength = ?signal.strength,
            entry = signal.entry_price,
            stop = signal.stop_loss,
            target = signal.take_profit,
            "Generated trading signal"
        );

        Ok(Some(signal))
    }

    /// Calculate signal strength based on confluences
    fn calculate_strength(
        &self,
        po3_signals: &[(Timeframe, Po3Structure)],
        smt_divergence: &Option<SmtDivergence>,
    ) -> SignalStrength {
        let mut score = 0;

        // Multi-timeframe alignment
        score += po3_signals.len().min(3);

        // SMT divergence
        if smt_divergence.is_some() {
            score += 2;
        }

        // PO3 quality (check manipulation/distribution clarity)
        if let Some((_, po3)) = po3_signals.first() {
            if po3.manipulation_clear {
                score += 1;
            }
            if po3.distribution_started {
                score += 1;
            }
        }

        match score {
            0..=2 => SignalStrength::Weak,
            3..=4 => SignalStrength::Medium,
            5..=6 => SignalStrength::Strong,
            _ => SignalStrength::VeryStrong,
        }
    }

    /// Generate human-readable notes for the signal
    fn generate_notes(
        &self,
        po3_signals: &[(Timeframe, Po3Structure)],
        smt_divergence: &Option<SmtDivergence>,
    ) -> String {
        let mut notes = Vec::new();

        // Timeframe alignment
        let tfs: Vec<String> = po3_signals.iter().map(|(tf, _)| tf.to_string()).collect();
        notes.push(format!("PO3 on: {}", tfs.join(", ")));

        // SMT info
        if let Some(div) = smt_divergence {
            notes.push(format!("SMT {:?} divergence detected", div.divergence_type));
        }

        // Phase info from primary PO3
        if let Some((_, po3)) = po3_signals.first() {
            notes.push(format!("Current phase: {:?}", po3.current_phase));
            notes.push(format!("Target: 50% at {:.2}", po3.midpoint));
        }

        notes.join(" | ")
    }

    /// Get active signals
    pub async fn get_active_signals(&self) -> Vec<TradingSignal> {
        self.signals.read().await.clone()
    }

    /// Get last scan time
    pub fn last_scan_time(&self) -> Option<DateTime<Utc>> {
        self.last_scan.blocking_read().clone()
    }

    /// Clear all signals
    pub async fn clear_signals(&self) {
        self.signals.write().await.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_config_default() {
        let config = StrategyConfig::default();
        assert_eq!(config.min_accumulation_bars, 5);
        assert_eq!(config.timeframes.len(), 1);
        assert_eq!(config.timeframes[0], Timeframe::Daily);
        assert!(config.require_alignment);
    }
}
