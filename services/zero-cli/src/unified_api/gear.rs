//! Gear Control API Routes
//!
//! Provides HTTP API for controlling the gear system.
//!
//! # Endpoints
//!
//! - `GET  /api/v1/gear/current`  - Get current gear and dial values
//! - `POST /api/v1/gear/switch`   - Switch gear preset
//! - `POST /api/v1/gear/dials`    - Set individual dial values
//! - `GET  /api/v1/gear/presets`  - Get all gear preset details
//! - `GET  /api/v1/gear/close`    - Get current CLOSE evaluation

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::gear::{
    CLOSEEvaluation, CLOSEInput, DialName, DialValues, GearPreset, GearPresetDetail,
    GearStatus, GearSwitchResult,
    presets::{get_all_gear_presets, get_gear_preset_detail},
};
use super::state::UnifiedApiState;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct GearCurrentResponse {
    pub success: bool,
    pub status: GearStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GearSwitchRequest {
    /// Target gear (P, N, D, S, M)
    pub gear: String,
    /// Confirm switching to high-autonomy modes
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct GearSwitchResponse {
    pub success: bool,
    pub result: GearSwitchResult,
}

#[derive(Debug, Deserialize)]
pub struct SetDialsRequest {
    pub observe: u8,
    pub decide: u8,
    pub act: u8,
}

#[derive(Debug, Serialize)]
pub struct SetDialsResponse {
    pub success: bool,
    pub gear: GearPreset,
    pub dials: DialValues,
}

#[derive(Debug, Deserialize)]
pub struct SetSingleDialRequest {
    pub name: String,
    pub value: u8,
}

#[derive(Debug, Serialize)]
pub struct GearPresetsResponse {
    pub success: bool,
    pub presets: Vec<GearPresetDetail>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearCloseResponse {
    pub success: bool,
    pub evaluation: CLOSEEvaluation,
    pub history_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateCloseRequest {
    pub input: CLOSEInput,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/gear/current - Get current gear and dial values
pub async fn get_gear_current(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    let status = state.gear.status().await;

    Json(GearCurrentResponse {
        success: true,
        status,
    })
}

/// POST /api/v1/gear/switch - Switch gear preset
pub async fn switch_gear(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<GearSwitchRequest>,
) -> impl IntoResponse {
    // Parse gear
    let gear: GearPreset = match request.gear.parse() {
        Ok(g) => g,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: e,
                }),
            )
                .into_response();
        }
    };

    // Check if confirmation is required
    let current = state.gear.get_gear().await;
    if gear == GearPreset::S && current != GearPreset::S && !request.confirm {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: "Switching to Sport mode requires confirmation. Set 'confirm: true' to proceed.".to_string(),
            }),
        )
            .into_response();
    }

    let result = state.gear.switch_gear(gear).await;

    Json(GearSwitchResponse {
        success: result.success,
        result,
    })
    .into_response()
}

/// POST /api/v1/gear/dials - Set dial values (switches to Manual mode)
pub async fn set_dials(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<SetDialsRequest>,
) -> impl IntoResponse {
    // Validate values
    if request.observe > 100 || request.decide > 100 || request.act > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: "Dial values must be between 0 and 100".to_string(),
            }),
        )
            .into_response();
    }

    let dials = state
        .gear
        .set_dials(request.observe, request.decide, request.act)
        .await;

    Json(SetDialsResponse {
        success: true,
        gear: GearPreset::M,
        dials,
    })
    .into_response()
}

/// POST /api/v1/gear/dial - Set a single dial value
pub async fn set_single_dial(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<SetSingleDialRequest>,
) -> impl IntoResponse {
    // Parse dial name
    let dial_name = match request.name.to_lowercase().as_str() {
        "observe" => DialName::Observe,
        "decide" => DialName::Decide,
        "act" => DialName::Act,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Invalid dial name: {}. Must be 'observe', 'decide', or 'act'", request.name),
                }),
            )
                .into_response();
        }
    };

    // Validate value
    if request.value > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: "Dial value must be between 0 and 100".to_string(),
            }),
        )
            .into_response();
    }

    let dials = state.gear.set_dial(dial_name, request.value).await;

    Json(SetDialsResponse {
        success: true,
        gear: GearPreset::M,
        dials,
    })
    .into_response()
}

/// GET /api/v1/gear/presets - Get all gear preset details
pub async fn get_gear_presets() -> impl IntoResponse {
    let presets = get_all_gear_presets();
    let total = presets.len();

    Json(GearPresetsResponse {
        success: true,
        presets,
        total,
    })
}

/// GET /api/v1/gear/presets/:gear - Get specific gear preset details
pub async fn get_gear_preset(
    axum::extract::Path(gear): axum::extract::Path<String>,
) -> impl IntoResponse {
    let gear_preset: GearPreset = match gear.parse() {
        Ok(g) => g,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: e,
                }),
            )
                .into_response();
        }
    };

    let preset = get_gear_preset_detail(gear_preset);

    Json(serde_json::json!({
        "success": true,
        "preset": preset
    }))
    .into_response()
}

/// GET /api/v1/gear/close - Get current CLOSE evaluation
pub async fn get_close_evaluation(
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    let evaluator = state.gear.close_evaluator.read().await;
    let history = evaluator.get_history(None);
    let history_count = history.len();

    // Return last evaluation or a default
    let evaluation = history.last().cloned().unwrap_or_default();

    Json(GearCloseResponse {
        success: true,
        evaluation,
        history_count,
    })
}

/// POST /api/v1/gear/close - Run CLOSE evaluation with input data
pub async fn evaluate_close(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<EvaluateCloseRequest>,
) -> impl IntoResponse {
    let mut evaluator = state.gear.close_evaluator.write().await;
    let evaluation = evaluator.evaluate(&request.input);
    let history_count = evaluator.get_history(None).len();

    Json(GearCloseResponse {
        success: true,
        evaluation,
        history_count,
    })
}

/// POST /api/v1/gear/auto-switch - Enable/disable auto-switch
pub async fn set_auto_switch(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    let enabled = request.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    state.gear.set_auto_switch(enabled).await;

    Json(serde_json::json!({
        "success": true,
        "autoSwitchEnabled": enabled
    }))
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gear::GearState;

    #[tokio::test]
    async fn test_gear_state_default() {
        let state = GearState::new();
        let status = state.status().await;
        assert_eq!(status.gear, GearPreset::D);
    }

    #[test]
    fn test_gear_switch_request_parse() {
        let json = r#"{"gear": "S", "confirm": true}"#;
        let request: GearSwitchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.gear, "S");
        assert!(request.confirm);
    }

    #[test]
    fn test_set_dials_request_parse() {
        let json = r#"{"observe": 70, "decide": 60, "act": 40}"#;
        let request: SetDialsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.observe, 70);
        assert_eq!(request.decide, 60);
        assert_eq!(request.act, 40);
    }
}
