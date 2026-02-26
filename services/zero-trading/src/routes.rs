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
