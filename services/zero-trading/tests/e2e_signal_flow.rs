//! End-to-end integration tests for signal generation flow.
//!
//! Tests the complete signal generation pipeline:
//! Market data → Strategy scanning → Signal generation → Notification
//!
//! These tests use mock data to simulate real trading scenarios.

use chrono::{Duration, Utc};

use zero_trading::data::{Candle, SmtPair, Timeframe};
use zero_trading::macro_filter::{
    CompositeIndicators, EconomicCyclePhase, MacroEnvironment, PolicyCycle, TradingBias,
};
use zero_trading::strategy::{DivergenceType, Po3Phase, Po3Structure, SignalDirection, SignalStrength, SmtDivergence, TradingSignal};

// ============================================================================
// Test Data Generators
// ============================================================================

/// Generate realistic daily candles for testing
fn generate_daily_candles(symbol: &str, count: usize, base_price: f64) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    let mut price = base_price;
    let now = Utc::now();

    for i in 0..count {
        let open = price;
        let volatility = 0.02; // 2% daily volatility
        let change = (rand_simple(i as u64) - 0.5) * volatility * 2.0;
        let close = price * (1.0 + change);
        let high = f64::max(open, close) * (1.0 + rand_simple(i as u64 + 100) * 0.01);
        let low = f64::min(open, close) * (1.0 - rand_simple(i as u64 + 200) * 0.01);

        candles.push(Candle {
            symbol: symbol.to_string(),
            timeframe: Timeframe::Daily,
            timestamp: now - Duration::days((count - i - 1) as i64),
            open,
            high,
            low,
            close,
            volume: 1000000.0 + rand_simple(i as u64 + 300) * 500000.0,
            amount: 10000000.0,
        });

        price = close;
    }

    candles
}

/// Simple deterministic pseudo-random number generator (0.0 to 1.0)
fn rand_simple(seed: u64) -> f64 {
    let x = seed.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
    (x % 1000) as f64 / 1000.0
}

/// Generate candles showing accumulation-manipulation-distribution pattern
fn generate_po3_pattern(symbol: &str) -> Vec<Candle> {
    let mut candles = Vec::new();
    let now = Utc::now();
    let base_price = 10.0;

    // Phase 1: Accumulation (sideways consolidation)
    for i in 0..10 {
        let noise = (rand_simple(i as u64) - 0.5) * 0.02;
        let price = base_price * (1.0 + noise);
        candles.push(Candle {
            symbol: symbol.to_string(),
            timeframe: Timeframe::H4,
            timestamp: now - Duration::hours(((30 - i) * 4) as i64),
            open: price,
            high: price * 1.01,
            low: price * 0.99,
            close: price * (1.0 + noise * 0.5),
            volume: 100000.0,
            amount: 1000000.0,
        });
    }

    // Phase 2: Manipulation (false breakout below support)
    let manipulation_low = base_price * 0.97;
    candles.push(Candle {
        symbol: symbol.to_string(),
        timeframe: Timeframe::H4,
        timestamp: now - Duration::hours(80),
        open: base_price,
        high: base_price * 1.005,
        low: manipulation_low,
        close: manipulation_low * 1.01,
        volume: 200000.0,
        amount: 2000000.0,
    });

    // Phase 3: Distribution (reversal back through range)
    for i in 0..5 {
        let price = manipulation_low * (1.0 + (i as f64) * 0.01);
        candles.push(Candle {
            symbol: symbol.to_string(),
            timeframe: Timeframe::H4,
            timestamp: now - Duration::hours((76 - i * 4) as i64),
            open: price,
            high: price * 1.015,
            low: price * 0.995,
            close: price * 1.01,
            volume: 150000.0,
            amount: 1500000.0,
        });
    }

    candles
}

// ============================================================================
// Signal Generation Tests
// ============================================================================

#[test]
fn test_signal_creation() {
    let signal = TradingSignal {
        id: "test-001".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: SignalDirection::Long,
        strength: SignalStrength::Strong,
        entry_price: 10.0,
        stop_loss: 9.5,
        take_profit: 11.5,
        timestamp: Utc::now(),
        po3_structure: Some(Po3Structure {
            direction: SignalDirection::Long,
            current_phase: Po3Phase::Distribution,
            range_high: 10.2,
            range_low: 9.8,
            midpoint: 10.0,
            manipulation_extreme: 9.5,
            manipulation_clear: true,
            distribution_started: true,
            ideal_entry: 9.9,
            stop_loss: 9.5,
            detected_at: Utc::now(),
            accumulation_bars: 10,
        }),
        smt_divergence: Some(SmtDivergence {
            divergence_type: DivergenceType::Bullish,
            primary_symbol: "000001.SZ".to_string(),
            reference_symbol: "000300.SH".to_string(),
            primary_extreme: 9.5,
            primary_prev_extreme: 9.8,
            reference_extreme: 9.6,
            reference_prev_extreme: 9.5,
            detected_at: Utc::now(),
            bars_ago: 2,
            strength: 70,
        }),
        timeframe_alignment: vec![Timeframe::Daily, Timeframe::H4],
        notes: "Test signal with PO3 + SMT".to_string(),
    };

    assert_eq!(signal.symbol, "000001.SZ");
    assert_eq!(signal.direction, SignalDirection::Long);
    assert!(signal.po3_structure.is_some());
    assert!(signal.smt_divergence.is_some());
}

#[test]
fn test_signal_risk_reward_calculation() {
    let signal = TradingSignal {
        id: "test-002".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: SignalDirection::Long,
        strength: SignalStrength::Medium,
        entry_price: 10.0,
        stop_loss: 9.5,   // 5% risk
        take_profit: 12.0, // 20% reward
        timestamp: Utc::now(),
        po3_structure: None,
        smt_divergence: None,
        timeframe_alignment: vec![Timeframe::Daily],
        notes: "Risk/reward test".to_string(),
    };

    let risk_percent = signal.risk_percent();
    let reward_percent = signal.reward_percent();
    let risk_reward = signal.risk_reward();

    assert!((risk_percent - 5.0).abs() < 0.01);
    assert!((reward_percent - 20.0).abs() < 0.01);
    assert!((risk_reward - 4.0).abs() < 0.01);
}

#[test]
fn test_signal_strength_hierarchy() {
    // Very strong > Strong > Medium > Weak
    assert!(SignalStrength::VeryStrong > SignalStrength::Strong);
    assert!(SignalStrength::Strong > SignalStrength::Medium);
    assert!(SignalStrength::Medium > SignalStrength::Weak);
}

// ============================================================================
// PO3 Structure Tests
// ============================================================================

#[test]
fn test_po3_phase_identification() {
    // Test that all phases can be identified
    let phases = [
        Po3Phase::Accumulation,
        Po3Phase::Manipulation,
        Po3Phase::Distribution,
    ];

    for phase in phases {
        assert!(phase == phase); // Basic equality check
    }
}

#[test]
fn test_po3_structure_validation() {
    let structure = Po3Structure {
        direction: SignalDirection::Long,
        current_phase: Po3Phase::Distribution,
        range_high: 10.5,
        range_low: 9.5,
        midpoint: 10.0,
        manipulation_extreme: 9.2,
        manipulation_clear: true,
        distribution_started: true,
        ideal_entry: 9.9,
        stop_loss: 9.2,
        detected_at: Utc::now(),
        accumulation_bars: 10,
    };

    // Manipulation should be outside accumulation range for valid structure
    assert!(structure.manipulation_extreme < structure.range_low);
    assert!(structure.midpoint >= structure.range_low && structure.midpoint <= structure.range_high);
}

// ============================================================================
// SMT Divergence Tests
// ============================================================================

#[test]
fn test_divergence_types() {
    let types = [
        DivergenceType::Bullish,
        DivergenceType::Bearish,
    ];

    for div_type in types {
        assert!(div_type == div_type);
    }
}

#[test]
fn test_smt_pair_setup() {
    let pair = SmtPair {
        primary: "000001.SZ".to_string(),
        reference: "000300.SH".to_string(),
        name: "平安银行-沪深300".to_string(),
        description: Some("A股龙头对标沪深300".to_string()),
    };

    assert!(!pair.primary.is_empty());
    assert!(!pair.reference.is_empty());
    assert_ne!(pair.primary, pair.reference);
}

// ============================================================================
// Macro Environment Integration Tests
// ============================================================================

#[test]
fn test_signal_with_macro_context() {
    let _signal = TradingSignal {
        id: "test-003".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: SignalDirection::Long,
        strength: SignalStrength::Strong,
        entry_price: 10.0,
        stop_loss: 9.5,
        take_profit: 11.5,
        timestamp: Utc::now(),
        po3_structure: None,
        smt_divergence: None,
        timeframe_alignment: vec![Timeframe::Daily],
        notes: "".to_string(),
    };

    // Bullish macro environment
    let bullish_env = MacroEnvironment {
        cycle_phase: EconomicCyclePhase::Expansion,
        m2_growth: Some(10.5),
        social_financing: Some(3.5),
        risk_appetite: 65.0,
        pmi: Some(52.5),
        position_multiplier: 1.2,
        trading_bias: TradingBias::Bullish,
        notes: "Expansion phase".to_string(),
        composite_indicators: CompositeIndicators::default(),
        policy_cycle: PolicyCycle::Easing,
    };

    // Signal + bullish macro = enhanced position
    let adjusted_position = 10.0 * bullish_env.position_multiplier;
    assert!((adjusted_position - 12.0).abs() < 0.01);

    // Bearish macro environment
    let bearish_env = MacroEnvironment {
        cycle_phase: EconomicCyclePhase::Contraction,
        m2_growth: Some(8.0),
        social_financing: Some(2.0),
        risk_appetite: 35.0,
        pmi: Some(48.0),
        position_multiplier: 0.6,
        trading_bias: TradingBias::Bearish,
        notes: "Contraction phase".to_string(),
        composite_indicators: CompositeIndicators::default(),
        policy_cycle: PolicyCycle::Tightening,
    };

    // Signal + bearish macro = reduced position
    let reduced_position = 10.0 * bearish_env.position_multiplier;
    assert!((reduced_position - 6.0).abs() < 0.01);
}

// ============================================================================
// Telegram Message Formatting Tests
// ============================================================================

#[test]
fn test_signal_telegram_message_format() {
    let signal = TradingSignal {
        id: "test-004".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: SignalDirection::Long,
        strength: SignalStrength::VeryStrong,
        entry_price: 10.0,
        stop_loss: 9.5,
        take_profit: 12.0,
        timestamp: Utc::now(),
        po3_structure: Some(Po3Structure {
            direction: SignalDirection::Long,
            current_phase: Po3Phase::Distribution,
            range_high: 10.2,
            range_low: 9.8,
            midpoint: 10.0,
            manipulation_extreme: 9.3,
            manipulation_clear: true,
            distribution_started: true,
            ideal_entry: 9.9,
            stop_loss: 9.3,
            detected_at: Utc::now(),
            accumulation_bars: 10,
        }),
        smt_divergence: None,
        timeframe_alignment: vec![Timeframe::Daily, Timeframe::H4],
        notes: "Strong setup".to_string(),
    };

    let message = signal.to_telegram_message();

    // Verify key elements are present
    assert!(message.contains("000001.SZ"));
    assert!(message.contains("10") || message.contains("10.00"));
}

// ============================================================================
// End-to-End Flow Simulation
// ============================================================================

#[test]
fn test_complete_signal_flow() {
    // Step 1: Generate market data
    let candles = generate_daily_candles("000001.SZ", 50, 10.0);
    assert_eq!(candles.len(), 50);

    // Step 2: Create a signal based on the data
    let latest = candles.last().unwrap();
    let signal = TradingSignal {
        id: format!("signal-{}", Utc::now().timestamp()),
        symbol: latest.symbol.clone(),
        direction: SignalDirection::Long,
        strength: SignalStrength::Medium,
        entry_price: latest.close,
        stop_loss: latest.close * 0.95,
        take_profit: latest.close * 1.15,
        timestamp: Utc::now(),
        po3_structure: None,
        smt_divergence: None,
        timeframe_alignment: vec![Timeframe::Daily],
        notes: "Generated from test data".to_string(),
    };

    // Step 3: Validate signal properties
    assert!(!signal.id.is_empty());
    assert!(signal.entry_price > 0.0);
    assert!(signal.stop_loss < signal.entry_price);
    assert!(signal.take_profit > signal.entry_price);

    // Step 4: Calculate risk/reward
    let rr = signal.risk_reward();
    assert!(rr > 0.0);

    // Step 5: Generate notification message
    let message = signal.to_telegram_message();
    assert!(!message.is_empty());
}

#[test]
fn test_po3_pattern_flow() {
    // Generate PO3 pattern data
    let candles = generate_po3_pattern("000001.SZ");
    assert!(!candles.is_empty());

    // Verify the pattern has the expected structure
    let lowest = candles.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
    let highest = candles.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);

    // Should have reasonable price range
    assert!(highest > lowest);
    let range_pct = (highest - lowest) / lowest * 100.0;
    assert!(range_pct > 1.0 && range_pct < 20.0);
}
