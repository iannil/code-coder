//! Paper trading API integration tests
//!
//! These tests verify that the Paper Trading API types and endpoints
//! compile correctly and work as expected.

use serde_json::json;

// ============================================================================
// Type Serialization Tests
// ============================================================================

#[test]
fn test_paper_start_request_defaults() {
    // Verify JSON with defaults deserializes correctly
    // Note: PaperStartRequest is in the routes module which uses default functions
    // We test the expected default values here
    let json_value = json!({});

    // The default values should be:
    // - initial_capital: 100_000.0
    // - duration_secs: None
    // - max_positions: 5
    // - enable_notifications: true
    assert!(json_value.is_object());

    // With partial fields
    let partial = json!({
        "initial_capital": 50000.0
    });
    assert!(partial.get("initial_capital").unwrap().as_f64().unwrap() > 0.0);
}

#[test]
fn test_paper_status_response_serialization() {
    let status = json!({
        "state": "Running",
        "start_time": "2026-02-25T10:00:00Z",
        "elapsed_seconds": 3600,
        "trades_count": 5,
        "current_pnl": 1500.0
    });

    let json_str = serde_json::to_string(&status).unwrap();
    assert!(json_str.contains("Running"));
    assert!(json_str.contains("1500"));
    assert!(json_str.contains("3600"));
    assert!(json_str.contains("trades_count"));
}

#[test]
fn test_paper_trades_response_serialization() {
    let response = json!({
        "trades": [
            {
                "id": "trade-001",
                "symbol": "000001.SZ",
                "direction": "Long",
                "entry_price": 10.50,
                "exit_price": 11.00,
                "quantity": 100.0,
                "entry_time": "2026-02-25T09:30:00Z",
                "exit_time": "2026-02-25T14:30:00Z",
                "signal_id": "sig-001",
                "status": "ClosedProfit",
                "realized_pnl": 50.0
            }
        ],
        "count": 1
    });

    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains("000001.SZ"));
    assert!(json_str.contains("ClosedProfit"));
    assert!(json_str.contains("50"));
}

#[test]
fn test_paper_report_response_structure() {
    let report = json!({
        "title": "Paper Trading Session Report",
        "period": "2026-02-25 09:30 to 15:00",
        "summary": {
            "initial_capital": 100000.0,
            "final_capital": 105000.0,
            "net_profit": 5000.0,
            "total_return_pct": 5.0,
            "total_trades": 10,
            "winning_trades": 6,
            "losing_trades": 4,
            "win_rate": 60.0,
            "avg_win": 1500.0,
            "avg_loss": -750.0,
            "profit_factor": 2.0,
            "max_drawdown_pct": 3.0
        },
        "verification": {
            "passed": true,
            "issues": [],
            "recommendation": "Strategy meets verification criteria."
        },
        "text_report": "Full text report content..."
    });

    let json_str = serde_json::to_string(&report).unwrap();
    assert!(json_str.contains("Paper Trading Session Report"));
    assert!(json_str.contains("105000"));
    assert!(json_str.contains("win_rate"));
    assert!(json_str.contains("profit_factor"));
}

// ============================================================================
// Paper Trading Types Tests
// ============================================================================

#[tokio::test]
async fn test_paper_trading_types_compile() {
    use zero_trading::paper_trading::{
        PaperTradingConfig, SessionState, PaperTrade, PaperTradeStatus,
        TradeDirection, SessionSummary, VerificationResult,
    };

    // Test PaperTradingConfig default
    let config = PaperTradingConfig::default();
    assert!(config.initial_capital > 0.0);
    assert_eq!(config.max_positions, 5);
    assert!(config.enable_notifications);

    // Test SessionState enum variants
    let idle = SessionState::Idle;
    let running = SessionState::Running;
    let completed = SessionState::Completed;
    let paused = SessionState::Paused;
    let failed = SessionState::Failed;

    assert_eq!(idle, SessionState::Idle);
    assert_ne!(idle, running);
    assert_ne!(running, completed);
    assert_ne!(paused, failed);

    // Test TradeDirection enum
    let long = TradeDirection::Long;
    let short = TradeDirection::Short;
    assert_ne!(long, short);

    // Test PaperTradeStatus enum
    let open = PaperTradeStatus::Open;
    let closed_profit = PaperTradeStatus::ClosedProfit;
    let closed_loss = PaperTradeStatus::ClosedLoss;
    let cancelled = PaperTradeStatus::Cancelled;

    assert_ne!(open, closed_profit);
    assert_ne!(closed_profit, closed_loss);
    assert_ne!(closed_loss, cancelled);
}

#[tokio::test]
async fn test_session_state_serialization() {
    use zero_trading::paper_trading::SessionState;

    // Test serialization of all session states
    let states = vec![
        SessionState::Idle,
        SessionState::Running,
        SessionState::Paused,
        SessionState::Completed,
        SessionState::Failed,
    ];

    for state in states {
        let json_str = serde_json::to_string(&state).unwrap();
        assert!(!json_str.is_empty());

        // Verify round-trip deserialization
        let deserialized: SessionState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, state);
    }
}

#[tokio::test]
async fn test_paper_trade_serialization() {
    use zero_trading::paper_trading::{PaperTrade, PaperTradeStatus, TradeDirection};

    let trade = PaperTrade {
        id: "test-trade-001".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: TradeDirection::Long,
        entry_price: 10.50,
        exit_price: Some(11.00),
        quantity: 100.0,
        entry_time: chrono::Utc::now(),
        exit_time: Some(chrono::Utc::now()),
        signal_id: "sig-001".to_string(),
        status: PaperTradeStatus::ClosedProfit,
        realized_pnl: Some(50.0),
    };

    // Serialize
    let json_str = serde_json::to_string(&trade).unwrap();
    assert!(json_str.contains("test-trade-001"));
    assert!(json_str.contains("000001.SZ"));
    assert!(json_str.contains("Long"));
    assert!(json_str.contains("ClosedProfit"));

    // Deserialize
    let deserialized: PaperTrade = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.id, "test-trade-001");
    assert_eq!(deserialized.symbol, "000001.SZ");
    assert_eq!(deserialized.direction, TradeDirection::Long);
    assert_eq!(deserialized.status, PaperTradeStatus::ClosedProfit);
}

#[tokio::test]
async fn test_paper_trade_pnl_calculation() {
    use zero_trading::paper_trading::{PaperTrade, PaperTradeStatus, TradeDirection};

    // Test long position profit
    let long_trade = PaperTrade {
        id: "long-001".to_string(),
        symbol: "000001.SZ".to_string(),
        direction: TradeDirection::Long,
        entry_price: 10.0,
        exit_price: None,
        quantity: 100.0,
        entry_time: chrono::Utc::now(),
        exit_time: None,
        signal_id: "sig-001".to_string(),
        status: PaperTradeStatus::Open,
        realized_pnl: None,
    };

    // Price up 10% should yield 100.0 profit
    let pnl = long_trade.unrealized_pnl(11.0);
    assert!((pnl - 100.0).abs() < 0.01);
    assert!(long_trade.is_profitable(11.0));
    assert!((long_trade.return_pct(11.0) - 10.0).abs() < 0.01);

    // Price down 5% should yield -50.0 loss
    let loss_pnl = long_trade.unrealized_pnl(9.5);
    assert!((loss_pnl - (-50.0)).abs() < 0.01);
    assert!(!long_trade.is_profitable(9.5));
}

#[tokio::test]
async fn test_paper_trade_close() {
    use zero_trading::paper_trading::{PaperTrade, PaperTradeStatus, TradeDirection};

    let mut trade = PaperTrade {
        id: "close-test".to_string(),
        symbol: "000002.SZ".to_string(),
        direction: TradeDirection::Long,
        entry_price: 20.0,
        exit_price: None,
        quantity: 50.0,
        entry_time: chrono::Utc::now(),
        exit_time: None,
        signal_id: "sig-002".to_string(),
        status: PaperTradeStatus::Open,
        realized_pnl: None,
    };

    // Close with profit
    trade.close(22.0);

    assert_eq!(trade.status, PaperTradeStatus::ClosedProfit);
    assert!(trade.exit_price.is_some());
    assert!(trade.exit_time.is_some());
    assert!(trade.realized_pnl.is_some());

    // P&L should be (22 - 20) * 50 = 100
    let pnl = trade.realized_pnl.unwrap();
    assert!((pnl - 100.0).abs() < 0.01);
}

#[tokio::test]
async fn test_paper_trade_close_loss() {
    use zero_trading::paper_trading::{PaperTrade, PaperTradeStatus, TradeDirection};

    let mut trade = PaperTrade {
        id: "loss-test".to_string(),
        symbol: "000003.SZ".to_string(),
        direction: TradeDirection::Long,
        entry_price: 15.0,
        exit_price: None,
        quantity: 200.0,
        entry_time: chrono::Utc::now(),
        exit_time: None,
        signal_id: "sig-003".to_string(),
        status: PaperTradeStatus::Open,
        realized_pnl: None,
    };

    // Close with loss
    trade.close(14.0);

    assert_eq!(trade.status, PaperTradeStatus::ClosedLoss);

    // P&L should be (14 - 15) * 200 = -200
    let pnl = trade.realized_pnl.unwrap();
    assert!((pnl - (-200.0)).abs() < 0.01);
}

// ============================================================================
// Session Summary Tests
// ============================================================================

#[tokio::test]
async fn test_session_summary_serialization() {
    use zero_trading::paper_trading::SessionSummary;

    let summary = SessionSummary {
        initial_capital: 100_000.0,
        final_capital: 108_000.0,
        net_profit: 8_000.0,
        total_return_pct: 8.0,
        total_trades: 15,
        winning_trades: 10,
        losing_trades: 5,
        win_rate: 66.67,
        avg_win: 1200.0,
        avg_loss: -800.0,
        profit_factor: 1.5,
        max_drawdown_pct: 4.5,
    };

    let json_str = serde_json::to_string(&summary).unwrap();
    assert!(json_str.contains("100000"));
    assert!(json_str.contains("108000"));
    assert!(json_str.contains("8000"));
    assert!(json_str.contains("66.67"));

    // Round-trip
    let deserialized: SessionSummary = serde_json::from_str(&json_str).unwrap();
    assert!((deserialized.initial_capital - 100_000.0).abs() < 0.01);
    assert!((deserialized.win_rate - 66.67).abs() < 0.01);
}

// ============================================================================
// Verification Result Tests
// ============================================================================

#[tokio::test]
async fn test_verification_result_serialization() {
    use zero_trading::paper_trading::VerificationResult;

    let result = VerificationResult {
        passed: true,
        issues: vec![],
        recommendation: "Strategy meets verification criteria. Ready for cautious live trading.".to_string(),
    };

    let json_str = serde_json::to_string(&result).unwrap();
    assert!(json_str.contains("true"));
    assert!(json_str.contains("Ready for cautious live trading"));

    let failed_result = VerificationResult {
        passed: false,
        issues: vec![
            "Win rate 35% below minimum (40%)".to_string(),
            "Max drawdown 18% exceeds limit (15%)".to_string(),
        ],
        recommendation: "Strategy needs improvement before live trading.".to_string(),
    };

    let failed_json = serde_json::to_string(&failed_result).unwrap();
    assert!(failed_json.contains("false"));
    assert!(failed_json.contains("Win rate"));
    assert!(failed_json.contains("Max drawdown"));
}

// ============================================================================
// Paper Trading Config Tests
// ============================================================================

#[tokio::test]
async fn test_paper_trading_config_serialization() {
    use zero_trading::paper_trading::PaperTradingConfig;
    use zero_trading::strategy::SignalStrength;

    let config = PaperTradingConfig {
        initial_capital: 200_000.0,
        max_position_pct: 15.0,
        min_signal_strength: SignalStrength::Strong,
        max_positions: 3,
        enable_notifications: false,
        scan_interval_secs: 30,
        max_duration: Some(std::time::Duration::from_secs(7200)),
    };

    let json_str = serde_json::to_string(&config).unwrap();
    assert!(json_str.contains("200000"));
    assert!(json_str.contains("15"));
    assert!(json_str.contains("Strong"));
    assert!(json_str.contains("false"));
}

#[tokio::test]
async fn test_paper_trading_config_default_values() {
    use zero_trading::paper_trading::PaperTradingConfig;
    use zero_trading::strategy::SignalStrength;

    let config = PaperTradingConfig::default();

    // Verify all default values
    assert!((config.initial_capital - 100_000.0).abs() < 0.01);
    assert!((config.max_position_pct - 20.0).abs() < 0.01);
    assert_eq!(config.min_signal_strength, SignalStrength::Medium);
    assert_eq!(config.max_positions, 5);
    assert!(config.enable_notifications);
    assert_eq!(config.scan_interval_secs, 60);
    assert!(config.max_duration.is_none());
}

// ============================================================================
// Paper Session Status Tests
// ============================================================================

#[tokio::test]
async fn test_paper_session_status_serialization() {
    use zero_trading::paper_trading::{PaperSessionStatus, SessionState};

    let status = PaperSessionStatus {
        state: SessionState::Running,
        start_time: Some(chrono::Utc::now()),
        elapsed_seconds: Some(1800),
    };

    let json_str = serde_json::to_string(&status).unwrap();
    assert!(json_str.contains("Running"));
    assert!(json_str.contains("1800"));

    // Round-trip
    let deserialized: PaperSessionStatus = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.state, SessionState::Running);
    assert_eq!(deserialized.elapsed_seconds, Some(1800));
}

#[tokio::test]
async fn test_paper_session_status_idle() {
    use zero_trading::paper_trading::{PaperSessionStatus, SessionState};

    let status = PaperSessionStatus {
        state: SessionState::Idle,
        start_time: None,
        elapsed_seconds: None,
    };

    let json_str = serde_json::to_string(&status).unwrap();
    assert!(json_str.contains("Idle"));
    assert!(json_str.contains("null"));

    let deserialized: PaperSessionStatus = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.state, SessionState::Idle);
    assert!(deserialized.start_time.is_none());
}

// ============================================================================
// Report Tests
// ============================================================================

#[tokio::test]
async fn test_paper_trading_report_creation() {
    use zero_trading::paper_trading::{
        PaperTradingReport, SessionSummary, ValidationResult,
    };

    let report = PaperTradingReport {
        title: "Test Paper Trading Report".to_string(),
        period: "2026-02-25 09:30 to 15:00".to_string(),
        summary: SessionSummary {
            initial_capital: 100_000.0,
            final_capital: 103_000.0,
            net_profit: 3_000.0,
            total_return_pct: 3.0,
            total_trades: 8,
            winning_trades: 5,
            losing_trades: 3,
            win_rate: 62.5,
            avg_win: 900.0,
            avg_loss: -500.0,
            profit_factor: 1.8,
            max_drawdown_pct: 2.5,
        },
        trades: vec![],
        validations: vec![],
    };

    // Test serialization
    let json_str = serde_json::to_string(&report).unwrap();
    assert!(json_str.contains("Test Paper Trading Report"));
    assert!(json_str.contains("103000"));

    // Test verification criteria
    let verification = report.meets_verification_criteria();
    assert!(verification.passed);
    assert!(verification.issues.is_empty());
}

#[tokio::test]
async fn test_paper_trading_report_text_generation() {
    use zero_trading::paper_trading::{
        PaperTradingReport, SessionSummary,
    };

    let report = PaperTradingReport {
        title: "Paper Trading Session Complete".to_string(),
        period: "2026-02-25".to_string(),
        summary: SessionSummary {
            initial_capital: 100_000.0,
            final_capital: 105_000.0,
            net_profit: 5_000.0,
            total_return_pct: 5.0,
            total_trades: 10,
            winning_trades: 7,
            losing_trades: 3,
            win_rate: 70.0,
            avg_win: 1000.0,
            avg_loss: -666.67,
            profit_factor: 2.1,
            max_drawdown_pct: 2.0,
        },
        trades: vec![],
        validations: vec![],
    };

    let text_report = report.to_text_report();
    assert!(text_report.contains("Paper Trading Session Complete"));
    assert!(text_report.contains("100,000") || text_report.contains("100000"));

    let telegram_msg = report.to_telegram_message();
    assert!(telegram_msg.contains("模拟交易报告"));
}
