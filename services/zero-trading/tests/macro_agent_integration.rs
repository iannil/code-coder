//! Integration tests for macro agent enhancement.
//!
//! Tests the complete analysis flow including:
//! - Inventory cycle detection
//! - PPI-CPI scissors analysis
//! - Report generation
//! - Data linkage analysis

use zero_trading::macro_filter::{
    CompositeIndicators, EconomicCyclePhase, InventoryCycleAnalyzer, InventoryCycleInput,
    InventoryCyclePhase, PolicyCycle, ScissorsSignal, TradingBias,
};

// ============================================================================
// Inventory Cycle Integration Tests
// ============================================================================

#[test]
fn test_full_inventory_cycle_detection() {
    // Test data simulating different economic phases

    // Phase 1: Passive Destocking (Recovery Dawn)
    let recovery_input = InventoryCycleInput {
        inventory_yoy: Some(-3.0),
        pmi_new_orders: Some(51.5),
        pmi_production: Some(49.0),
        ppi_mom: Some(0.3),
        ppi_yoy: Some(-0.5),
        industrial_profit_yoy: Some(2.0),
        ..Default::default()
    };
    let recovery_result = InventoryCycleAnalyzer::analyze(&recovery_input);
    assert_eq!(recovery_result.phase, InventoryCyclePhase::PassiveDestocking);
    assert!(recovery_result.position_multiplier > 1.0);
    assert!(!recovery_result.signals.is_empty());

    // Phase 2: Active Restocking (Prosperity Peak)
    let boom_input = InventoryCycleInput {
        inventory_yoy: Some(10.0),
        pmi_new_orders: Some(54.0),
        pmi_production: Some(53.0),
        ppi_yoy: Some(6.0),
        industrial_profit_yoy: Some(20.0),
        ..Default::default()
    };
    let boom_result = InventoryCycleAnalyzer::analyze(&boom_input);
    assert_eq!(boom_result.phase, InventoryCyclePhase::ActiveRestocking);

    // Phase 3: Passive Restocking (Warning)
    let warning_input = InventoryCycleInput {
        inventory_yoy: Some(8.0),
        pmi_new_orders: Some(48.0),
        pmi_production: Some(50.0),
        ppi_yoy: Some(3.0),
        industrial_profit_yoy: Some(-5.0),
        ..Default::default()
    };
    let warning_result = InventoryCycleAnalyzer::analyze(&warning_input);
    // This could be passive restocking or active destocking depending on signals
    assert!(warning_result.position_multiplier <= 1.0);

    // Phase 4: Active Destocking (Winter)
    let winter_input = InventoryCycleInput {
        inventory_yoy: Some(-8.0),
        pmi_new_orders: Some(45.0),
        pmi_production: Some(46.0),
        ppi_yoy: Some(-5.0),
        industrial_profit_yoy: Some(-20.0),
        ..Default::default()
    };
    let winter_result = InventoryCycleAnalyzer::analyze(&winter_input);
    assert_eq!(winter_result.phase, InventoryCyclePhase::ActiveDestocking);
    assert!(winter_result.position_multiplier < 1.0);
}

// ============================================================================
// PPI-CPI Scissors Analysis Tests
// ============================================================================

#[test]
fn test_scissors_signal_comprehensive() {
    // Test various spread scenarios

    // Large positive spread (upstream profits)
    let positive = ScissorsSignal::analyze(10.0, 2.0);
    assert_eq!(positive, ScissorsSignal::PositiveScissors);
    assert!(positive.investment_implication().contains("上游"));
    assert!(positive.affected_sectors().iter().any(|s| s.contains("煤炭")));

    // Large negative spread (downstream profits)
    let negative = ScissorsSignal::analyze(-2.0, 3.0);
    assert_eq!(negative, ScissorsSignal::NegativeScissors);
    assert!(negative.investment_implication().contains("下游"));
    assert!(negative.affected_sectors().iter().any(|s| s.contains("消费品")));

    // Neutral (balanced)
    let neutral = ScissorsSignal::analyze(2.5, 2.0);
    assert_eq!(neutral, ScissorsSignal::Neutral);

    // Scissors closing
    let closing = ScissorsSignal::analyze(4.5, 2.0);
    assert_eq!(closing, ScissorsSignal::ScissorsClosing);
}

#[test]
fn test_scissors_edge_cases() {
    // Boundary conditions
    // spread = 3.0, which is NOT > 3.0, so it's ScissorsClosing
    assert_eq!(ScissorsSignal::analyze(3.0, 0.0), ScissorsSignal::ScissorsClosing);
    // spread = 4.0 > 3.0, so PositiveScissors
    assert_eq!(ScissorsSignal::analyze(5.0, 1.0), ScissorsSignal::PositiveScissors);
    // spread = -2.5 < -2.0, so NegativeScissors
    assert_eq!(ScissorsSignal::analyze(0.0, 2.5), ScissorsSignal::NegativeScissors);
}

// ============================================================================
// Composite Analysis Tests
// ============================================================================

#[test]
fn test_composite_indicators_with_scissors() {
    // Verify that CompositeIndicators properly includes scissors signal
    let indicators = CompositeIndicators {
        ppi_cpi_spread: Some(5.0),
        m2_sf_spread: Some(1.0),
        real_interest_rate: Some(1.5),
        trade_spread: Some(3.0),
        scissors_signal: Some(ScissorsSignal::PositiveScissors),
    };

    assert!(indicators.scissors_signal.is_some());
    assert_eq!(
        indicators.scissors_signal.unwrap(),
        ScissorsSignal::PositiveScissors
    );
}

// ============================================================================
// Economic Cycle Phase Tests
// ============================================================================

#[test]
fn test_economic_cycle_phases() {
    // Verify all phases are properly defined
    let phases = [
        EconomicCyclePhase::Expansion,
        EconomicCyclePhase::EarlyRecovery,
        EconomicCyclePhase::Slowdown,
        EconomicCyclePhase::Contraction,
    ];

    for phase in phases {
        // Just verify they exist and can be compared
        assert!(phase == phase);
    }
}

#[test]
fn test_policy_cycle() {
    let cycles = [PolicyCycle::Easing, PolicyCycle::Neutral, PolicyCycle::Tightening];

    for cycle in cycles {
        assert!(cycle == cycle);
    }

    // Default should be Neutral
    assert_eq!(PolicyCycle::default(), PolicyCycle::Neutral);
}

// ============================================================================
// Trading Bias Tests
// ============================================================================

#[test]
fn test_trading_bias_variants() {
    let biases = [
        TradingBias::Bullish,
        TradingBias::Neutral,
        TradingBias::Bearish,
        TradingBias::AvoidTrading,
    ];

    for bias in biases {
        assert!(bias == bias);
    }
}

// ============================================================================
// Inventory Cycle Phase Position Multiplier Tests
// ============================================================================

#[test]
fn test_inventory_cycle_position_multipliers() {
    // Recovery phase should have higher multiplier
    assert!(InventoryCyclePhase::PassiveDestocking.position_multiplier() > 1.0);

    // Boom phase should be neutral
    assert!((InventoryCyclePhase::ActiveRestocking.position_multiplier() - 1.0).abs() < 0.01);

    // Warning and winter phases should have lower multipliers
    assert!(InventoryCyclePhase::PassiveRestocking.position_multiplier() < 1.0);
    assert!(InventoryCyclePhase::ActiveDestocking.position_multiplier() < 1.0);

    // Winter should have lowest
    assert!(
        InventoryCyclePhase::ActiveDestocking.position_multiplier()
            <= InventoryCyclePhase::PassiveRestocking.position_multiplier()
    );
}

// ============================================================================
// Display Trait Tests
// ============================================================================

#[test]
fn test_display_traits() {
    // Inventory cycle phases
    assert_eq!(InventoryCyclePhase::PassiveDestocking.to_string(), "被动去库存");
    assert_eq!(InventoryCyclePhase::ActiveRestocking.to_string(), "主动补库存");
    assert_eq!(InventoryCyclePhase::PassiveRestocking.to_string(), "被动补库存");
    assert_eq!(InventoryCyclePhase::ActiveDestocking.to_string(), "主动去库存");

    // Scissors signals
    assert!(ScissorsSignal::PositiveScissors.to_string().contains("正剪刀差"));
    assert!(ScissorsSignal::NegativeScissors.to_string().contains("负剪刀差"));
}

// ============================================================================
// English Names Tests
// ============================================================================

#[test]
fn test_english_names() {
    assert_eq!(
        InventoryCyclePhase::PassiveDestocking.english_name(),
        "Passive Destocking"
    );
    assert_eq!(
        InventoryCyclePhase::ActiveRestocking.english_name(),
        "Active Restocking"
    );
}

// ============================================================================
// Investment Implication Tests
// ============================================================================

#[test]
fn test_investment_implications() {
    // Inventory cycle implications
    assert!(InventoryCyclePhase::PassiveDestocking
        .investment_implication()
        .contains("复苏"));
    assert!(InventoryCyclePhase::ActiveRestocking
        .investment_implication()
        .contains("繁荣"));
    assert!(InventoryCyclePhase::PassiveRestocking
        .investment_implication()
        .contains("警惕"));
    assert!(InventoryCyclePhase::ActiveDestocking
        .investment_implication()
        .contains("寒冬"));
}
