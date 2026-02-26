//! HTTP routes for the trading service.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::TradingState;
use crate::strategy::TradingSignal;
use crate::execution::Position;
use crate::macro_filter::TradingBias;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

#[derive(Debug, Serialize)]
pub struct SignalsResponse {
    pub signals: Vec<TradingSignal>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct PositionsResponse {
    pub positions: Vec<Position>,
    pub total_value: f64,
    pub open_count: usize,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub market_connected: bool,
    pub broker_connected: bool,
    pub last_scan: Option<String>,
    pub active_signals: usize,
    pub open_positions: usize,
}

/// Macro decision response
#[derive(Debug, Serialize)]
pub struct MacroDecisionResponse {
    pub source: String,
    pub cycle_phase: String,
    pub position_multiplier: f64,
    pub trading_bias: String,
    pub risk_appetite: f64,
    pub risk_warnings: Vec<String>,
    pub summary: String,
    pub confidence: f64,
    pub trading_recommended: bool,
}

/// Macro report response
#[derive(Debug, Serialize)]
pub struct MacroReportResponse {
    pub report_type: String,
    pub title: String,
    pub period: String,
    pub content: String,
    pub highlights: Vec<String>,
    pub generated_at: String,
}

/// Force analysis request
#[derive(Debug, Deserialize)]
pub struct ForceAnalysisRequest {
    #[serde(default)]
    pub send_notification: bool,
}

// ============================================================================
// Paper Trading Request/Response Types
// ============================================================================

/// Paper trading start request
#[derive(Debug, Deserialize)]
pub struct PaperStartRequest {
    #[serde(default = "default_capital")]
    pub initial_capital: f64,
    pub duration_secs: Option<u64>,
    #[serde(default = "default_max_positions")]
    pub max_positions: usize,
    #[serde(default = "default_true")]
    pub enable_notifications: bool,
}

fn default_capital() -> f64 {
    100_000.0
}

fn default_max_positions() -> usize {
    5
}

fn default_true() -> bool {
    true
}

/// Paper trading status response
#[derive(Debug, Serialize)]
pub struct PaperStatusResponse {
    pub state: String,
    pub start_time: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub trades_count: usize,
    pub current_pnl: Option<f64>,
}

/// Paper trading trades response
#[derive(Debug, Serialize)]
pub struct PaperTradesResponse {
    pub trades: Vec<crate::paper_trading::PaperTrade>,
    pub count: usize,
}

/// Paper trading report response
#[derive(Debug, Serialize)]
pub struct PaperReportResponse {
    pub title: String,
    pub period: String,
    pub summary: crate::paper_trading::SessionSummary,
    pub verification: crate::paper_trading::VerificationResult,
    pub text_report: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "zero-trading".to_string(),
    })
}

/// Get current trading signals
pub async fn get_signals(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<SignalsResponse>, StatusCode> {
    let signals = state.strategy.get_active_signals().await;
    let count = signals.len();

    Ok(Json(SignalsResponse { signals, count }))
}

/// Get current positions
pub async fn get_positions(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<PositionsResponse>, StatusCode> {
    let execution = state.execution.read().await;
    let positions = execution.get_positions();
    let total_value = positions.iter().map(|p| p.current_value()).sum();
    let open_count = positions.iter().filter(|p| p.is_open()).count();

    Ok(Json(PositionsResponse {
        positions,
        total_value,
        open_count,
    }))
}

/// Get service status
pub async fn get_status(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let market_connected = state.data.is_connected();
    let signals = state.strategy.get_active_signals().await;
    let execution = state.execution.read().await;
    let positions = execution.get_positions();

    Ok(Json(StatusResponse {
        market_connected,
        broker_connected: false, // Broker integration removed - signals sent via IM for manual execution
        last_scan: state.strategy.last_scan_time().map(|t| t.to_rfc3339()),
        active_signals: signals.len(),
        open_positions: positions.iter().filter(|p| p.is_open()).count(),
    }))
}

// ============================================================================
// Macro Agent Routes
// ============================================================================

/// Get current macro decision (hybrid mode)
pub async fn get_macro_decision(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<MacroDecisionResponse>, StatusCode> {
    match state.macro_orchestrator.evaluate().await {
        Ok(decision) => {
            let trading_recommended = decision.trading_bias != TradingBias::AvoidTrading;

            Ok(Json(MacroDecisionResponse {
                source: format!("{:?}", decision.source),
                cycle_phase: format!("{:?}", decision.cycle_phase),
                position_multiplier: decision.position_multiplier,
                trading_bias: format!("{:?}", decision.trading_bias),
                risk_appetite: decision.risk_appetite,
                risk_warnings: decision.risk_warnings,
                summary: decision.summary,
                confidence: decision.confidence,
                trading_recommended,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get macro decision");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Force agent analysis (bypass cache and triggers)
pub async fn force_macro_analysis(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<MacroDecisionResponse>, StatusCode> {
    match state.macro_orchestrator.force_analyze().await {
        Ok(decision) => {
            let trading_recommended = decision.trading_bias != TradingBias::AvoidTrading;

            Ok(Json(MacroDecisionResponse {
                source: format!("{:?}", decision.source),
                cycle_phase: format!("{:?}", decision.cycle_phase),
                position_multiplier: decision.position_multiplier,
                trading_bias: format!("{:?}", decision.trading_bias),
                risk_appetite: decision.risk_appetite,
                risk_warnings: decision.risk_warnings,
                summary: decision.summary,
                confidence: decision.confidence,
                trading_recommended,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to force macro analysis");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Generate ad-hoc macro report
pub async fn generate_macro_report(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<MacroReportResponse>, StatusCode> {
    match state.report_generator.generate_adhoc_report().await {
        Ok(report) => Ok(Json(MacroReportResponse {
            report_type: format!("{}", report.report_type),
            title: report.title,
            period: report.period,
            content: report.content,
            highlights: report.highlights,
            generated_at: report.generated_at.to_rfc3339(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to generate macro report");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Send ad-hoc macro report to Telegram
pub async fn send_macro_report(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.report_generator.send_adhoc_report().await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Macro report sent to Telegram"
        }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to send macro report");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Check agent availability
pub async fn check_agent_status(
    State(state): State<Arc<TradingState>>,
) -> Json<serde_json::Value> {
    let agent_available = state.macro_orchestrator.is_agent_available().await;
    let report_available = state.report_generator.is_available().await;

    Json(serde_json::json!({
        "agent_available": agent_available,
        "report_generator_available": report_available,
        "codecoder_endpoint": state.config.codecoder_endpoint()
    }))
}

// ============================================================================
// Paper Trading Routes
// ============================================================================

/// POST /api/v1/paper/start - Start paper trading session
pub async fn paper_start(
    State(state): State<Arc<TradingState>>,
    Json(req): Json<PaperStartRequest>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    use crate::paper_trading::PaperTradingConfig;
    use crate::strategy::SignalStrength;

    let config = PaperTradingConfig {
        initial_capital: req.initial_capital,
        max_position_pct: 20.0,
        min_signal_strength: SignalStrength::Medium,
        max_positions: req.max_positions,
        enable_notifications: req.enable_notifications,
        scan_interval_secs: 60,
        max_duration: req.duration_secs.map(std::time::Duration::from_secs),
    };

    let duration = req.duration_secs.map(std::time::Duration::from_secs);

    match state.paper_manager.start_session(config, duration).await {
        Ok(()) => Ok(Json(SuccessResponse {
            success: true,
            message: "Paper trading session started".to_string(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to start paper trading");
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// POST /api/v1/paper/stop - Stop paper trading session
pub async fn paper_stop(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<SuccessResponse>, StatusCode> {
    match state.paper_manager.stop_session().await {
        Ok(()) => Ok(Json(SuccessResponse {
            success: true,
            message: "Paper trading session stopped".to_string(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to stop paper trading");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/paper/status - Get paper trading status
pub async fn paper_status(
    State(state): State<Arc<TradingState>>,
) -> Json<PaperStatusResponse> {
    let status = state.paper_manager.get_status().await;
    let trades = state.paper_manager.get_trades().await;
    let result = state.paper_manager.get_result().await;

    let current_pnl = result.as_ref().map(|r| r.summary.net_profit);

    Json(PaperStatusResponse {
        state: format!("{:?}", status.state),
        start_time: status.start_time.map(|t| t.to_rfc3339()),
        elapsed_seconds: status.elapsed_seconds,
        trades_count: trades.len(),
        current_pnl,
    })
}

/// GET /api/v1/paper/trades - Get paper trades
pub async fn paper_trades(
    State(state): State<Arc<TradingState>>,
) -> Json<PaperTradesResponse> {
    let trades = state.paper_manager.get_trades().await;
    let count = trades.len();
    Json(PaperTradesResponse { trades, count })
}

/// GET /api/v1/paper/report - Get paper trading report
pub async fn paper_report(
    State(state): State<Arc<TradingState>>,
) -> Result<Json<PaperReportResponse>, StatusCode> {
    match state.paper_manager.get_report().await {
        Some(report) => {
            let verification = report.meets_verification_criteria();
            let text_report = report.to_text_report();

            Ok(Json(PaperReportResponse {
                title: report.title,
                period: report.period,
                summary: report.summary,
                verification,
                text_report,
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// Value Analysis Routes
// ============================================================================

/// Value analysis request
#[derive(Debug, Deserialize)]
pub struct ValueAnalyzeRequest {
    pub symbol: String,
    pub revenue: f64,
    pub gross_profit: f64,
    pub operating_income: f64,
    pub net_income: f64,
    pub interest_expense: f64,
    pub total_assets: f64,
    pub total_equity: f64,
    pub total_liabilities: f64,
    pub cash: f64,
    pub total_debt: f64,
    pub operating_cash_flow: f64,
    pub investing_cash_flow: f64,
    pub financing_cash_flow: f64,
    pub capex: f64,
    pub free_cash_flow: f64,
    // Qualitative inputs
    #[serde(default)]
    pub has_evaluation_power: bool,
    #[serde(default)]
    pub is_simple_and_understandable: bool,
    #[serde(default)]
    pub has_demand_stickiness: bool,
    #[serde(default)]
    pub has_supply_stability: bool,
    #[serde(default)]
    pub is_market_leader: bool,
}

/// Value analysis response
#[derive(Debug, Serialize)]
pub struct ValueAnalyzeResponse {
    pub symbol: String,
    pub cash_flow_dna: String,
    pub roe_driver: String,
    pub is_printing_machine: bool,
    pub qualitative_score: f64,
    pub quantitative_score: f64,
    pub overall_score: f64,
    pub reasoning: String,
}

/// POST /api/v1/value/analyze - Analyze company using printing machine checklist
pub async fn value_analyze(
    State(state): State<Arc<TradingState>>,
    Json(req): Json<ValueAnalyzeRequest>,
) -> Result<Json<ValueAnalyzeResponse>, StatusCode> {
    use crate::value::{FinancialData, QualitativeInputs};
    use chrono::Utc;

    let financial_data = FinancialData {
        symbol: req.symbol.clone(),
        period_end: Utc::now(),
        revenue: req.revenue,
        gross_profit: req.gross_profit,
        operating_income: req.operating_income,
        net_income: req.net_income,
        interest_expense: req.interest_expense,
        total_assets: req.total_assets,
        total_equity: req.total_equity,
        total_liabilities: req.total_liabilities,
        cash: req.cash,
        total_debt: req.total_debt,
        operating_cash_flow: req.operating_cash_flow,
        investing_cash_flow: req.investing_cash_flow,
        financing_cash_flow: req.financing_cash_flow,
        capex: req.capex,
        free_cash_flow: req.free_cash_flow,
        avg_roe_5y: None,
        avg_gross_margin_5y: None,
        avg_net_margin_5y: None,
    };

    let qualitative = QualitativeInputs {
        has_evaluation_power: req.has_evaluation_power,
        is_simple_and_understandable: req.is_simple_and_understandable,
        has_demand_stickiness: req.has_demand_stickiness,
        has_supply_stability: req.has_supply_stability,
        is_market_leader: req.is_market_leader,
    };

    match state.value_analyzer.analyze_printing_machine(&financial_data, qualitative) {
        Ok(checklist) => Ok(Json(ValueAnalyzeResponse {
            symbol: checklist.symbol.clone(),
            cash_flow_dna: format!("{}", checklist.cash_flow_dna),
            roe_driver: format!("{}", checklist.roe_driver),
            is_printing_machine: checklist.is_printing_machine(),
            qualitative_score: checklist.qualitative_score,
            quantitative_score: checklist.quantitative_score,
            overall_score: checklist.overall_score,
            reasoning: checklist.reasoning,
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to analyze value");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// Valuation Routes
// ============================================================================

/// Valuation analysis request
#[derive(Debug, Deserialize)]
pub struct ValuationAnalyzeRequest {
    pub symbol: String,
    pub price: f64,
    pub eps_ttm: f64,
    pub eps_forward: Option<f64>,
    pub eps_growth_rate: Option<f64>,
    pub book_value_per_share: f64,
    pub roe: f64,
    pub dividend_per_share: f64,
    pub payout_ratio: f64,
    pub dividend_growth_5y: Option<f64>,
    pub consecutive_dividend_years: Option<u32>,
    #[serde(default = "default_risk_free_rate")]
    pub risk_free_rate: f64,
}

fn default_risk_free_rate() -> f64 {
    3.0
}

/// Valuation analysis response
#[derive(Debug, Serialize)]
pub struct ValuationAnalyzeResponse {
    pub symbol: String,
    pub pe_ttm: f64,
    pub pe_position: String,
    pub pb: f64,
    pub pb_fair_value: f64,
    pub pb_premium_discount: f64,
    pub dividend_yield: f64,
    pub overall_score: f64,
    pub valuation_zone: String,
    pub investor_type_fit: String,
    pub margin_of_safety: f64,
    pub highlights: Vec<String>,
}

/// POST /api/v1/valuation/analyze - Three-dimensional valuation analysis
pub async fn valuation_analyze(
    State(state): State<Arc<TradingState>>,
    Json(req): Json<ValuationAnalyzeRequest>,
) -> Result<Json<ValuationAnalyzeResponse>, StatusCode> {
    use crate::valuation::ValuationInput;

    let input = ValuationInput {
        symbol: req.symbol,
        price: req.price,
        eps_ttm: req.eps_ttm,
        eps_forward: req.eps_forward,
        eps_growth_rate: req.eps_growth_rate,
        book_value_per_share: req.book_value_per_share,
        roe: req.roe,
        dividend_per_share: req.dividend_per_share,
        payout_ratio: req.payout_ratio,
        dividend_growth_5y: req.dividend_growth_5y,
        consecutive_dividend_years: req.consecutive_dividend_years,
        historical_pe: vec![], // Would need historical data in production
        risk_free_rate: req.risk_free_rate,
    };

    match state.valuation_analyzer.analyze(&input) {
        Ok(result) => Ok(Json(ValuationAnalyzeResponse {
            symbol: result.symbol,
            pe_ttm: result.pe_analysis.pe_ttm,
            pe_position: format!("{}", result.pe_analysis.position),
            pb: result.pb_analysis.pb,
            pb_fair_value: result.pb_analysis.fair_pb,
            pb_premium_discount: result.pb_analysis.premium_discount,
            dividend_yield: result.dy_analysis.dividend_yield,
            overall_score: result.overall_score,
            valuation_zone: format!("{}", result.valuation_zone),
            investor_type_fit: format!("{}", result.investor_type_fit),
            margin_of_safety: result.margin_of_safety,
            highlights: result.highlights,
        })),
        Err(e) => {
            tracing::error!(error = %e, "Failed to analyze valuation");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ============================================================================
// Portfolio Management Routes
// ============================================================================

/// Portfolio summary response
#[derive(Debug, Serialize)]
pub struct PortfolioSummaryResponse {
    pub total_positions: usize,
    pub core_positions: usize,
    pub satellite_positions: usize,
    pub watchlist_count: usize,
    pub total_value: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub winners: usize,
    pub losers: usize,
    pub core_allocation_pct: f64,
    pub satellite_allocation_pct: f64,
    pub cash_allocation_pct: f64,
}

/// GET /api/v1/portfolio/summary - Get portfolio summary
pub async fn portfolio_summary(
    State(state): State<Arc<TradingState>>,
) -> Json<PortfolioSummaryResponse> {
    let summary = state.portfolio_manager.get_summary().await;

    Json(PortfolioSummaryResponse {
        total_positions: summary.total_positions,
        core_positions: summary.core_positions,
        satellite_positions: summary.satellite_positions,
        watchlist_count: summary.watchlist_count,
        total_value: summary.total_value,
        total_pnl: summary.total_pnl,
        total_pnl_pct: summary.total_pnl_pct,
        winners: summary.winners,
        losers: summary.losers,
        core_allocation_pct: summary.core_allocation_pct,
        satellite_allocation_pct: summary.satellite_allocation_pct,
        cash_allocation_pct: summary.cash_allocation_pct,
    })
}

/// GET /api/v1/portfolio/positions - Get all portfolio positions
pub async fn portfolio_positions(
    State(state): State<Arc<TradingState>>,
) -> Json<serde_json::Value> {
    let pools = state.portfolio_manager.get_state().await;

    Json(serde_json::json!({
        "core": pools.core,
        "satellite": pools.satellite,
        "watchlist": pools.watchlist
    }))
}

/// Signal assessment response
#[derive(Debug, Serialize)]
pub struct SignalAssessmentResponse {
    pub symbol: String,
    pub level: String,
    pub recommended_action: String,
    pub triggers: Vec<String>,
    pub reasoning: String,
}

/// GET /api/v1/portfolio/signals - Get signal assessments for all positions
pub async fn portfolio_signals(
    State(state): State<Arc<TradingState>>,
) -> Json<Vec<SignalAssessmentResponse>> {
    use crate::portfolio::SignalContext;

    let pools = state.portfolio_manager.get_state().await;
    let mut assessments = Vec::new();

    // Analyze all positions
    for position in pools.core.iter().chain(pools.satellite.iter()) {
        let context = SignalContext::default(); // Would need real market data
        let assessment = state.signal_analyzer.analyze(position, &context);

        assessments.push(SignalAssessmentResponse {
            symbol: assessment.symbol,
            level: format!("{}", assessment.level),
            recommended_action: format!("{}", assessment.recommended_action),
            triggers: assessment.triggers.iter().map(|t| t.description()).collect(),
            reasoning: assessment.reasoning,
        });
    }

    Json(assessments)
}

/// Dip assessment request
#[derive(Debug, Deserialize)]
pub struct DipAssessmentRequest {
    pub symbol: String,
    pub decline_driver: String,
    pub decline_reasons: Vec<String>,
    pub moat_status: String,
    pub moat_notes: String,
    pub balance_sheet_health: String,
    #[serde(default = "default_true")]
    pub cash_runway_adequate: bool,
    #[serde(default = "default_true")]
    pub debt_manageable: bool,
    pub insider_activity: String,
    pub insider_notes: String,
    #[serde(default)]
    pub valuation_attractive: bool,
    #[serde(default)]
    pub margin_of_safety: f64,
    pub valuation_notes: String,
    pub recovery_catalyst: Option<String>,
    #[serde(default)]
    pub red_flags: Vec<String>,
}

/// Dip assessment response
#[derive(Debug, Serialize)]
pub struct DipAssessmentResponse {
    pub symbol: String,
    pub is_golden_pit: bool,
    pub score: f64,
    pub confidence: f64,
    pub recommended_strategy: String,
    pub suggested_sizing: String,
    pub reasoning: String,
}

/// POST /api/v1/portfolio/dip-assessment - Assess dip buying opportunity
pub async fn dip_assessment(
    State(state): State<Arc<TradingState>>,
    Json(req): Json<DipAssessmentRequest>,
) -> Result<Json<DipAssessmentResponse>, StatusCode> {
    use crate::portfolio::{
        BalanceSheetHealth, DeclineDriver, DipChecklist, InsiderActivity, MoatStatus,
    };

    // Parse enums from strings
    let decline_driver = match req.decline_driver.to_lowercase().as_str() {
        "external" => DeclineDriver::External,
        "internal" => DeclineDriver::Internal,
        "mixed" => DeclineDriver::Mixed,
        _ => DeclineDriver::Unknown,
    };

    let moat_status = match req.moat_status.to_lowercase().as_str() {
        "intact" => MoatStatus::Intact,
        "strengthened" => MoatStatus::Strengthened,
        "under_pressure" | "underpressure" => MoatStatus::UnderPressure,
        "breached" => MoatStatus::Breached,
        _ => MoatStatus::Intact,
    };

    let balance_sheet_health = match req.balance_sheet_health.to_lowercase().as_str() {
        "fortress" => BalanceSheetHealth::Fortress,
        "healthy" => BalanceSheetHealth::Healthy,
        "adequate" => BalanceSheetHealth::Adequate,
        "stressed" => BalanceSheetHealth::Stressed,
        "distressed" => BalanceSheetHealth::Distressed,
        _ => BalanceSheetHealth::Adequate,
    };

    let insider_activity = match req.insider_activity.to_lowercase().as_str() {
        "buying" => InsiderActivity::Buying,
        "buyback" => InsiderActivity::Buyback,
        "selling" => InsiderActivity::Selling,
        _ => InsiderActivity::Neutral,
    };

    let checklist = DipChecklist {
        decline_driver,
        decline_reasons: req.decline_reasons,
        moat_status,
        moat_notes: req.moat_notes,
        balance_sheet_health,
        cash_runway_adequate: req.cash_runway_adequate,
        debt_manageable: req.debt_manageable,
        insider_activity,
        insider_notes: req.insider_notes,
        valuation_attractive: req.valuation_attractive,
        margin_of_safety: req.margin_of_safety,
        valuation_notes: req.valuation_notes,
        industry_cycle_position: None,
        recovery_catalyst: req.recovery_catalyst,
        red_flags: req.red_flags,
    };

    let assessment = state.dip_analyzer.assess(&req.symbol, checklist);

    Ok(Json(DipAssessmentResponse {
        symbol: assessment.symbol.clone(),
        is_golden_pit: assessment.is_golden_pit,
        score: assessment.score,
        confidence: assessment.confidence,
        recommended_strategy: format!("{}", assessment.recommended_strategy),
        suggested_sizing: assessment.suggested_sizing().to_string(),
        reasoning: assessment.reasoning,
    }))
}

// ============================================================================
// Consensus Analysis Routes
// ============================================================================

/// Consensus analysis request
#[derive(Debug, Deserialize)]
pub struct ConsensusAnalyzeRequest {
    pub title: String,
    pub document_type: String,
    pub content: String,
    pub source_url: Option<String>,
}

/// Consensus analysis response
#[derive(Debug, Serialize)]
pub struct ConsensusAnalyzeResponse {
    pub document_title: String,
    pub analyzed_at: String,
    pub priority_ranking: Vec<String>,
    pub policy_tone: String,
    pub highlights: Vec<String>,
    pub confidence: f64,
    pub theme_strengths: Vec<ThemeStrengthResponse>,
}

/// Theme strength in response
#[derive(Debug, Serialize)]
pub struct ThemeStrengthResponse {
    pub theme: String,
    pub strength: f64,
    pub key_phrases: Vec<String>,
    pub change_type: String,
    pub reasoning: String,
}

/// POST /api/v1/value/consensus - Analyze policy document for national consensus
pub async fn consensus_analyze(
    State(_state): State<Arc<TradingState>>,
    Json(req): Json<ConsensusAnalyzeRequest>,
) -> Result<Json<ConsensusAnalyzeResponse>, StatusCode> {
    use crate::value::consensus::{ConsensusAnalyzer, PolicyDocument};
    use chrono::Utc;

    let document = PolicyDocument {
        title: req.title,
        document_type: req.document_type,
        published_at: Utc::now(),
        content: req.content,
        source_url: req.source_url,
    };

    let mut analyzer = ConsensusAnalyzer::default();

    match analyzer.analyze(&document).await {
        Ok(analysis) => {
            let theme_strengths: Vec<ThemeStrengthResponse> = analysis
                .theme_strengths
                .iter()
                .map(|ts| ThemeStrengthResponse {
                    theme: format!("{}", ts.theme),
                    strength: ts.strength,
                    key_phrases: ts.key_phrases.clone(),
                    change_type: format!("{}", ts.change_type),
                    reasoning: ts.reasoning.clone(),
                })
                .collect();

            Ok(Json(ConsensusAnalyzeResponse {
                document_title: analysis.document_title,
                analyzed_at: analysis.analyzed_at.to_rfc3339(),
                priority_ranking: analysis
                    .priority_ranking
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect(),
                policy_tone: analysis.policy_tone,
                highlights: analysis.highlights,
                confidence: analysis.confidence,
                theme_strengths,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to analyze consensus");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}


