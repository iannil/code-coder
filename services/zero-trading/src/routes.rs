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
        broker_connected: execution.is_connected(),
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
        "codecoder_endpoint": state.config.codecoder.endpoint
    }))
}
