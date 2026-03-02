//! Integration tests for API routes.
//!
//! Tests HTTP response types, serialization, and API contract validation.

use zero_trading::routes::*;

// ============================================================================
// Response Type Tests
// ============================================================================

#[test]
fn test_health_response_serialization() {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        service: "zero-trading".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("healthy"));
    assert!(json.contains("zero-trading"));
    assert!(json.contains("1.0.0"));
}

#[test]
fn test_health_response_fields() {
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "zero-trading".to_string(),
    };

    assert_eq!(response.status, "healthy");
    assert_eq!(response.service, "zero-trading");
    assert!(!response.version.is_empty());
}

#[test]
fn test_signals_response_serialization() {
    let response = SignalsResponse {
        signals: vec![],
        count: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("signals"));
    assert!(json.contains("count"));
}

#[test]
fn test_signals_response_with_count() {
    let response = SignalsResponse {
        signals: vec![],
        count: 5,
    };

    assert_eq!(response.count, 5);
}

#[test]
fn test_positions_response_serialization() {
    let response = PositionsResponse {
        positions: vec![],
        total_value: 100000.0,
        open_count: 3,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("positions"));
    assert!(json.contains("total_value"));
    assert!(json.contains("open_count"));
}

#[test]
fn test_positions_response_values() {
    let response = PositionsResponse {
        positions: vec![],
        total_value: 50000.0,
        open_count: 2,
    };

    assert!((response.total_value - 50000.0).abs() < 0.01);
    assert_eq!(response.open_count, 2);
}

#[test]
fn test_status_response_serialization() {
    let response = StatusResponse {
        market_connected: true,
        broker_connected: false,
        last_scan: Some("2026-03-02T10:00:00Z".to_string()),
        active_signals: 5,
        open_positions: 2,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("market_connected"));
    assert!(json.contains("broker_connected"));
    assert!(json.contains("active_signals"));
}

#[test]
fn test_status_response_optional_fields() {
    let response = StatusResponse {
        market_connected: false,
        broker_connected: false,
        last_scan: None,
        active_signals: 0,
        open_positions: 0,
    };

    assert!(response.last_scan.is_none());
    assert!(!response.market_connected);
}

// ============================================================================
// Macro Decision Response Tests
// ============================================================================

#[test]
fn test_macro_decision_response_serialization() {
    let response = MacroDecisionResponse {
        source: "RuleEngine".to_string(),
        cycle_phase: "Expansion".to_string(),
        position_multiplier: 1.0,
        trading_bias: "Bullish".to_string(),
        risk_appetite: 65.0,
        risk_warnings: vec!["PMI declining".to_string()],
        summary: "Favorable conditions".to_string(),
        confidence: 0.85,
        trading_recommended: true,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("RuleEngine"));
    assert!(json.contains("Bullish"));
    assert!(json.contains("trading_recommended"));
}

#[test]
fn test_macro_decision_avoid_trading() {
    let response = MacroDecisionResponse {
        source: "Agent".to_string(),
        cycle_phase: "Recession".to_string(),
        position_multiplier: 0.0,
        trading_bias: "AvoidTrading".to_string(),
        risk_appetite: 20.0,
        risk_warnings: vec!["High volatility".to_string(), "Economic uncertainty".to_string()],
        summary: "Avoid trading during high uncertainty".to_string(),
        confidence: 0.9,
        trading_recommended: false,
    };

    assert!(!response.trading_recommended);
    assert_eq!(response.risk_warnings.len(), 2);
    assert!((response.position_multiplier - 0.0).abs() < 0.01);
}

#[test]
fn test_macro_report_response_serialization() {
    let response = MacroReportResponse {
        report_type: "Weekly".to_string(),
        title: "Weekly Macro Report".to_string(),
        period: "2026-W09".to_string(),
        content: "Market analysis...".to_string(),
        highlights: vec!["PMI stable".to_string()],
        generated_at: "2026-03-02T10:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Weekly"));
    assert!(json.contains("highlights"));
}

// ============================================================================
// Paper Trading Response Tests
// ============================================================================

#[test]
fn test_paper_start_request_default_values() {
    let json = r#"{}"#;
    let req: PaperStartRequest = serde_json::from_str(json).unwrap();

    // Default values should be applied
    assert!((req.initial_capital - 100_000.0).abs() < 0.01);
    assert_eq!(req.max_positions, 5);
    assert!(req.enable_notifications);
    assert!(req.duration_secs.is_none());
}

#[test]
fn test_paper_start_request_custom_values() {
    let json = r#"{
        "initial_capital": 50000.0,
        "duration_secs": 3600,
        "max_positions": 3,
        "enable_notifications": false
    }"#;
    let req: PaperStartRequest = serde_json::from_str(json).unwrap();

    assert!((req.initial_capital - 50000.0).abs() < 0.01);
    assert_eq!(req.duration_secs, Some(3600));
    assert_eq!(req.max_positions, 3);
    assert!(!req.enable_notifications);
}

#[test]
fn test_paper_status_response_serialization() {
    let response = PaperStatusResponse {
        state: "Running".to_string(),
        start_time: Some("2026-03-02T09:30:00Z".to_string()),
        elapsed_seconds: Some(3600),
        trades_count: 5,
        current_pnl: Some(1500.0),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Running"));
    assert!(json.contains("elapsed_seconds"));
    assert!(json.contains("current_pnl"));
}

#[test]
fn test_paper_status_response_stopped() {
    let response = PaperStatusResponse {
        state: "Stopped".to_string(),
        start_time: None,
        elapsed_seconds: None,
        trades_count: 0,
        current_pnl: None,
    };

    assert_eq!(response.state, "Stopped");
    assert!(response.start_time.is_none());
    assert!(response.current_pnl.is_none());
}

#[test]
fn test_success_response_serialization() {
    let response = SuccessResponse {
        success: true,
        message: "Operation completed".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("success"));
    assert!(json.contains("Operation completed"));
}

// ============================================================================
// Value Analysis Response Tests
// ============================================================================

#[test]
fn test_value_analyze_request_deserialization() {
    let json = r#"{
        "symbol": "000001.SZ",
        "revenue": 1000000.0,
        "gross_profit": 300000.0,
        "operating_income": 200000.0,
        "net_income": 150000.0,
        "interest_expense": 10000.0,
        "total_assets": 5000000.0,
        "total_equity": 2000000.0,
        "total_liabilities": 3000000.0,
        "cash": 500000.0,
        "total_debt": 1000000.0,
        "operating_cash_flow": 180000.0,
        "investing_cash_flow": -50000.0,
        "financing_cash_flow": -30000.0,
        "capex": 40000.0,
        "free_cash_flow": 140000.0
    }"#;

    let req: ValueAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.symbol, "000001.SZ");
    assert!((req.revenue - 1000000.0).abs() < 0.01);
    assert!(!req.has_evaluation_power); // Default false
}

#[test]
fn test_value_analyze_request_with_qualitative() {
    let json = r#"{
        "symbol": "600519.SH",
        "revenue": 1000000.0,
        "gross_profit": 300000.0,
        "operating_income": 200000.0,
        "net_income": 150000.0,
        "interest_expense": 10000.0,
        "total_assets": 5000000.0,
        "total_equity": 2000000.0,
        "total_liabilities": 3000000.0,
        "cash": 500000.0,
        "total_debt": 1000000.0,
        "operating_cash_flow": 180000.0,
        "investing_cash_flow": -50000.0,
        "financing_cash_flow": -30000.0,
        "capex": 40000.0,
        "free_cash_flow": 140000.0,
        "has_evaluation_power": true,
        "is_simple_and_understandable": true,
        "has_demand_stickiness": true,
        "has_supply_stability": true,
        "is_market_leader": true
    }"#;

    let req: ValueAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert!(req.has_evaluation_power);
    assert!(req.is_market_leader);
}

#[test]
fn test_value_analyze_response_serialization() {
    let response = ValueAnalyzeResponse {
        symbol: "600519.SH".to_string(),
        cash_flow_dna: "Cow".to_string(),
        roe_driver: "HighMargin".to_string(),
        is_printing_machine: true,
        qualitative_score: 4.5,
        quantitative_score: 4.2,
        overall_score: 4.35,
        reasoning: "Strong moat and consistent cash flow".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Cow"));
    assert!(json.contains("printing_machine"));
}

// ============================================================================
// Valuation Response Tests
// ============================================================================

#[test]
fn test_valuation_analyze_request_deserialization() {
    let json = r#"{
        "symbol": "000001.SZ",
        "price": 15.0,
        "eps_ttm": 1.5,
        "book_value_per_share": 10.0,
        "roe": 15.0,
        "dividend_per_share": 0.5,
        "payout_ratio": 33.3
    }"#;

    let req: ValuationAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.symbol, "000001.SZ");
    assert!((req.price - 15.0).abs() < 0.01);
    assert!((req.risk_free_rate - 3.0).abs() < 0.01); // Default
}

#[test]
fn test_valuation_analyze_request_with_optional_fields() {
    let json = r#"{
        "symbol": "000001.SZ",
        "price": 15.0,
        "eps_ttm": 1.5,
        "eps_forward": 1.8,
        "eps_growth_rate": 20.0,
        "book_value_per_share": 10.0,
        "roe": 15.0,
        "dividend_per_share": 0.5,
        "payout_ratio": 33.3,
        "dividend_growth_5y": 10.0,
        "consecutive_dividend_years": 10,
        "risk_free_rate": 2.5
    }"#;

    let req: ValuationAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.eps_forward, Some(1.8));
    assert_eq!(req.consecutive_dividend_years, Some(10));
    assert!((req.risk_free_rate - 2.5).abs() < 0.01);
}

#[test]
fn test_valuation_analyze_response_serialization() {
    let response = ValuationAnalyzeResponse {
        symbol: "000001.SZ".to_string(),
        pe_ttm: 10.0,
        pe_position: "Low".to_string(),
        pb: 1.5,
        pb_fair_value: 1.8,
        pb_premium_discount: -16.7,
        dividend_yield: 3.3,
        overall_score: 75.0,
        valuation_zone: "Undervalued".to_string(),
        investor_type_fit: "Value".to_string(),
        margin_of_safety: 20.0,
        highlights: vec!["Low PE".to_string(), "High dividend yield".to_string()],
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Undervalued"));
    assert!(json.contains("margin_of_safety"));
}

// ============================================================================
// Portfolio Response Tests
// ============================================================================

#[test]
fn test_portfolio_summary_response_serialization() {
    let response = PortfolioSummaryResponse {
        total_positions: 10,
        core_positions: 5,
        satellite_positions: 5,
        watchlist_count: 20,
        total_value: 500000.0,
        total_pnl: 25000.0,
        total_pnl_pct: 5.0,
        winners: 7,
        losers: 3,
        core_allocation_pct: 60.0,
        satellite_allocation_pct: 30.0,
        cash_allocation_pct: 10.0,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("core_positions"));
    assert!(json.contains("satellite_positions"));
    assert!(json.contains("core_allocation_pct"));
}

#[test]
fn test_portfolio_allocation_sum() {
    let response = PortfolioSummaryResponse {
        total_positions: 10,
        core_positions: 5,
        satellite_positions: 5,
        watchlist_count: 20,
        total_value: 500000.0,
        total_pnl: 25000.0,
        total_pnl_pct: 5.0,
        winners: 7,
        losers: 3,
        core_allocation_pct: 60.0,
        satellite_allocation_pct: 30.0,
        cash_allocation_pct: 10.0,
    };

    // Allocations should sum to 100%
    let total = response.core_allocation_pct + response.satellite_allocation_pct + response.cash_allocation_pct;
    assert!((total - 100.0).abs() < 0.01);
}

#[test]
fn test_signal_assessment_response_serialization() {
    let response = SignalAssessmentResponse {
        symbol: "000001.SZ".to_string(),
        level: "Yellow".to_string(),
        recommended_action: "Hold".to_string(),
        triggers: vec!["Price near resistance".to_string()],
        reasoning: "Wait for confirmation".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Yellow"));
    assert!(json.contains("Hold"));
}

// ============================================================================
// Dip Assessment Tests
// ============================================================================

#[test]
fn test_dip_assessment_request_deserialization() {
    let json = r#"{
        "symbol": "000001.SZ",
        "decline_driver": "external",
        "decline_reasons": ["Market correction", "Sector rotation"],
        "moat_status": "intact",
        "moat_notes": "Brand strength maintained",
        "balance_sheet_health": "healthy",
        "cash_runway_adequate": true,
        "debt_manageable": true,
        "insider_activity": "buying",
        "insider_notes": "CEO increased stake",
        "valuation_attractive": true,
        "margin_of_safety": 25.0,
        "valuation_notes": "Trading below intrinsic value"
    }"#;

    let req: DipAssessmentRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.symbol, "000001.SZ");
    assert_eq!(req.decline_driver, "external");
    assert!(req.valuation_attractive);
}

#[test]
fn test_dip_assessment_request_with_red_flags() {
    let json = r#"{
        "symbol": "000001.SZ",
        "decline_driver": "internal",
        "decline_reasons": ["Accounting issues"],
        "moat_status": "breached",
        "moat_notes": "Competition eroding",
        "balance_sheet_health": "stressed",
        "cash_runway_adequate": false,
        "debt_manageable": false,
        "insider_activity": "selling",
        "insider_notes": "Multiple executives selling",
        "valuation_attractive": false,
        "margin_of_safety": -10.0,
        "valuation_notes": "Still overvalued",
        "red_flags": ["Auditor change", "Restatements", "Executive departures"]
    }"#;

    let req: DipAssessmentRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.red_flags.len(), 3);
    assert!(!req.cash_runway_adequate);
}

#[test]
fn test_dip_assessment_response_serialization() {
    let response = DipAssessmentResponse {
        symbol: "000001.SZ".to_string(),
        is_golden_pit: true,
        score: 85.0,
        confidence: 0.8,
        recommended_strategy: "Accumulate".to_string(),
        suggested_sizing: "Full".to_string(),
        reasoning: "Strong fundamentals with temporary setback".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("golden_pit"));
    assert!(json.contains("Accumulate"));
}

// ============================================================================
// Screener Response Tests
// ============================================================================

#[test]
fn test_screener_status_response_serialization() {
    let response = ScreenerStatusResponse {
        state: "idle".to_string(),
        last_scan_at: Some("2026-03-02T10:00:00Z".to_string()),
        last_sync_at: Some("2026-03-02T09:00:00Z".to_string()),
        last_scan_id: Some("scan-123".to_string()),
        last_scan_stock_count: Some(50),
        error_message: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("idle"));
    assert!(json.contains("scan-123"));
}

#[test]
fn test_screener_status_unavailable() {
    let response = ScreenerStatusResponse {
        state: "unavailable".to_string(),
        last_scan_at: None,
        last_sync_at: None,
        last_scan_id: None,
        last_scan_stock_count: None,
        error_message: Some("API token not configured".to_string()),
    };

    assert_eq!(response.state, "unavailable");
    assert!(response.error_message.is_some());
}

#[test]
fn test_screener_run_request_default() {
    let json = r#"{}"#;
    let req: ScreenerRunRequest = serde_json::from_str(json).unwrap();

    assert!(!req.quick); // Default is full scan
}

#[test]
fn test_screener_run_request_quick() {
    let json = r#"{"quick": true}"#;
    let req: ScreenerRunRequest = serde_json::from_str(json).unwrap();

    assert!(req.quick);
}

#[test]
fn test_screened_stock_response_serialization() {
    let stock = ScreenedStockResponse {
        symbol: "000001.SZ".to_string(),
        name: "Ping An Bank".to_string(),
        exchange: "SZSE".to_string(),
        industry: Some("Banking".to_string()),
        quant_score: 85.0,
        roe: Some(12.5),
        gross_margin: Some(45.0),
        pe_ttm: Some(8.5),
        pb: Some(0.8),
        dividend_yield: Some(4.5),
        is_printing_machine: Some(true),
    };

    let json = serde_json::to_string(&stock).unwrap();
    assert!(json.contains("Ping An Bank"));
    assert!(json.contains("quant_score"));
}

// ============================================================================
// Data Sync Response Tests
// ============================================================================

#[test]
fn test_data_sync_request_deserialization() {
    let json = r#"{"symbol": "000001.SZ"}"#;
    let req: DataSyncRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.symbol, Some("000001.SZ".to_string()));
}

#[test]
fn test_data_sync_request_empty() {
    let json = r#"{}"#;
    let req: DataSyncRequest = serde_json::from_str(json).unwrap();

    assert!(req.symbol.is_none());
}

#[test]
fn test_data_sync_response_serialization() {
    let response = DataSyncResponse {
        total_symbols: 100,
        successful: 95,
        failed: 5,
        total_candles: 50000,
        errors: vec!["Failed: 000001.SZ".to_string()],
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("total_symbols"));
    assert!(json.contains("total_candles"));
}

#[test]
fn test_storage_stats_response_serialization() {
    let response = StorageStatsResponse {
        candle_count: 100000,
        financial_count: 500,
        valuation_count: 500,
        unique_symbols: 200,
        db_size_mb: 150.5,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("candle_count"));
    assert!(json.contains("db_size_mb"));
}

#[test]
fn test_add_symbols_request_deserialization() {
    let json = r#"{"symbols": ["000001.SZ", "000002.SZ", "600000.SH"]}"#;
    let req: AddSymbolsRequest = serde_json::from_str(json).unwrap();

    assert_eq!(req.symbols.len(), 3);
    assert!(req.symbols.contains(&"000001.SZ".to_string()));
}

// ============================================================================
// Force Analysis Request Tests
// ============================================================================

#[test]
fn test_force_analysis_request_default() {
    let json = r#"{}"#;
    let req: ForceAnalysisRequest = serde_json::from_str(json).unwrap();

    assert!(!req.send_notification);
}

#[test]
fn test_force_analysis_request_with_notification() {
    let json = r#"{"send_notification": true}"#;
    let req: ForceAnalysisRequest = serde_json::from_str(json).unwrap();

    assert!(req.send_notification);
}

// ============================================================================
// Consensus Analysis Tests
// ============================================================================

#[test]
fn test_consensus_analyze_request_deserialization() {
    let json = r#"{
        "title": "Government Work Report 2026",
        "document_type": "government_work_report",
        "content": "This year's GDP target is..."
    }"#;

    let req: ConsensusAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.title, "Government Work Report 2026");
    assert!(req.source_url.is_none());
}

#[test]
fn test_consensus_analyze_request_with_url() {
    let json = r#"{
        "title": "Central Economic Work Conference",
        "document_type": "conference",
        "content": "Policy direction...",
        "source_url": "https://example.com/doc"
    }"#;

    let req: ConsensusAnalyzeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.source_url, Some("https://example.com/doc".to_string()));
}

#[test]
fn test_theme_strength_response_serialization() {
    let response = ThemeStrengthResponse {
        theme: "Technology".to_string(),
        strength: 0.85,
        key_phrases: vec!["AI".to_string(), "Digital economy".to_string()],
        change_type: "Strengthened".to_string(),
        reasoning: "Increased mentions compared to last year".to_string(),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("Technology"));
    assert!(json.contains("key_phrases"));
}
