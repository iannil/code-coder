//! Gear Control System
//!
//! Provides control over the Observer Network's autonomy level through:
//! - **Three Dials**: Independent control over Observe/Decide/Act dimensions
//! - **Gear Presets**: P/N/D/S/M presets for intuitive mode selection
//! - **CLOSE Evaluation**: Five-dimension framework for mode recommendations
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           Gear Control System                                │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │   Three Dials:           Gear Presets:           CLOSE Evaluation:          │
//! │   ┌─────────────┐        ┌──────────────────┐   ┌──────────────────┐        │
//! │   │ Observe: 70 │        │ P: Park (休眠)   │   │ Convergence      │        │
//! │   │ Decide:  60 │  ◄──── │ N: Neutral (观察)│   │ Leverage         │        │
//! │   │ Act:     40 │        │ D: Drive (默认)  │   │ Optionality      │        │
//! │   └─────────────┘        │ S: Sport (自主)  │   │ Surplus          │        │
//! │         │                │ M: Manual (自定义)│   │ Evolution        │        │
//! │         ▼                └──────────────────┘   └────────┬─────────┘        │
//! │   Controls Observer Network behavior                     │                  │
//! │                                                          ▼                  │
//! │                                              Recommends optimal gear        │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # API Endpoints
//!
//! - `GET  /api/v1/gear/current`  - Get current gear and dial values
//! - `POST /api/v1/gear/switch`   - Switch gear preset
//! - `POST /api/v1/gear/dials`    - Set individual dial values (Manual mode)
//! - `GET  /api/v1/gear/presets`  - Get all gear preset details
//! - `GET  /api/v1/gear/close`    - Get current CLOSE evaluation

pub mod close;
pub mod dials;
pub mod presets;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub use close::{CLOSEEvaluation, CLOSEEvaluator, CLOSEInput};
pub use dials::{DialName, DialValues, ThreeDials};
pub use presets::{
    GearPreset, GearPresetDetail, GearRiskLevel,
};

// ══════════════════════════════════════════════════════════════════════════════
// Gear State
// ══════════════════════════════════════════════════════════════════════════════

/// Gear system state with current gear, dials, and CLOSE evaluator
#[derive(Clone)]
pub struct GearState {
    /// Current gear preset
    pub current_gear: Arc<RwLock<GearPreset>>,
    /// Three dials configuration
    pub dials: Arc<RwLock<ThreeDials>>,
    /// CLOSE evaluator for mode recommendations
    pub close_evaluator: Arc<RwLock<CLOSEEvaluator>>,
    /// Whether auto-switch based on CLOSE is enabled
    pub auto_switch_enabled: Arc<RwLock<bool>>,
}

impl Default for GearState {
    fn default() -> Self {
        Self::new()
    }
}

impl GearState {
    /// Create new gear state with default Drive mode
    pub fn new() -> Self {
        Self {
            current_gear: Arc::new(RwLock::new(GearPreset::D)),
            dials: Arc::new(RwLock::new(ThreeDials::from_gear(GearPreset::D))),
            close_evaluator: Arc::new(RwLock::new(CLOSEEvaluator::new())),
            auto_switch_enabled: Arc::new(RwLock::new(false)),
        }
    }

    /// Create gear state with specific initial gear
    pub fn with_gear(gear: GearPreset) -> Self {
        Self {
            current_gear: Arc::new(RwLock::new(gear)),
            dials: Arc::new(RwLock::new(ThreeDials::from_gear(gear))),
            close_evaluator: Arc::new(RwLock::new(CLOSEEvaluator::new())),
            auto_switch_enabled: Arc::new(RwLock::new(false)),
        }
    }

    /// Get current gear
    pub async fn get_gear(&self) -> GearPreset {
        *self.current_gear.read().await
    }

    /// Get current dial values
    pub async fn get_dials(&self) -> DialValues {
        self.dials.read().await.values()
    }

    /// Switch to a gear preset
    pub async fn switch_gear(&self, gear: GearPreset) -> GearSwitchResult {
        let current = *self.current_gear.read().await;
        let validation = presets::validate_gear_transition(current, gear);

        if !validation.allowed {
            return GearSwitchResult {
                success: false,
                previous_gear: current,
                new_gear: current,
                dials: self.dials.read().await.values(),
                requires_confirmation: false,
                message: validation.reason,
            };
        }

        // Apply gear preset
        {
            let mut dials = self.dials.write().await;
            dials.set_gear(gear, None);
        }

        *self.current_gear.write().await = gear;

        GearSwitchResult {
            success: true,
            previous_gear: current,
            new_gear: gear,
            dials: self.dials.read().await.values(),
            requires_confirmation: validation.requires_confirmation,
            message: validation.reason,
        }
    }

    /// Set individual dial values (switches to Manual mode)
    pub async fn set_dials(&self, observe: u8, decide: u8, act: u8) -> DialValues {
        {
            let mut dials = self.dials.write().await;
            dials.set_dial(DialName::Observe, observe);
            dials.set_dial(DialName::Decide, decide);
            dials.set_dial(DialName::Act, act);
        }

        *self.current_gear.write().await = GearPreset::M;
        self.dials.read().await.values()
    }

    /// Set a single dial value (switches to Manual mode)
    pub async fn set_dial(&self, name: DialName, value: u8) -> DialValues {
        {
            let mut dials = self.dials.write().await;
            dials.set_dial(name, value);
        }

        *self.current_gear.write().await = GearPreset::M;
        self.dials.read().await.values()
    }

    /// Get current status
    pub async fn status(&self) -> GearStatus {
        let gear = *self.current_gear.read().await;
        let dials = self.dials.read().await;
        let auto_switch = *self.auto_switch_enabled.read().await;

        GearStatus {
            gear,
            dials: dials.values(),
            autonomy_score: dials.autonomy_score(),
            is_parked: dials.is_parked(),
            should_observe: dials.should_observe(),
            should_decide_autonomously: dials.should_decide_autonomously(),
            should_act_immediately: dials.should_act_immediately(),
            auto_switch_enabled: auto_switch,
            risk_level: presets::get_gear_risk_level(gear),
        }
    }

    /// Enable/disable auto-switch based on CLOSE evaluation
    pub async fn set_auto_switch(&self, enabled: bool) {
        *self.auto_switch_enabled.write().await = enabled;
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Response Types
// ══════════════════════════════════════════════════════════════════════════════

/// Result of a gear switch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GearSwitchResult {
    pub success: bool,
    pub previous_gear: GearPreset,
    pub new_gear: GearPreset,
    pub dials: DialValues,
    pub requires_confirmation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Current gear system status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GearStatus {
    pub gear: GearPreset,
    pub dials: DialValues,
    pub autonomy_score: u8,
    pub is_parked: bool,
    pub should_observe: bool,
    pub should_decide_autonomously: bool,
    pub should_act_immediately: bool,
    pub auto_switch_enabled: bool,
    pub risk_level: GearRiskLevel,
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gear_state_default() {
        let state = GearState::new();
        assert_eq!(state.get_gear().await, GearPreset::D);

        let dials = state.get_dials().await;
        assert_eq!(dials.observe, 70);
        assert_eq!(dials.decide, 60);
        assert_eq!(dials.act, 40);
    }

    #[tokio::test]
    async fn test_gear_switch() {
        let state = GearState::new();

        let result = state.switch_gear(GearPreset::S).await;
        assert!(result.success);
        assert_eq!(result.new_gear, GearPreset::S);
        assert!(result.requires_confirmation);

        let dials = state.get_dials().await;
        assert_eq!(dials.observe, 90);
        assert_eq!(dials.decide, 80);
        assert_eq!(dials.act, 70);
    }

    #[tokio::test]
    async fn test_manual_dial_setting() {
        let state = GearState::new();

        let dials = state.set_dials(50, 50, 50).await;
        assert_eq!(dials.observe, 50);
        assert_eq!(dials.decide, 50);
        assert_eq!(dials.act, 50);

        assert_eq!(state.get_gear().await, GearPreset::M);
    }

    #[tokio::test]
    async fn test_gear_status() {
        let state = GearState::with_gear(GearPreset::P);
        let status = state.status().await;

        assert_eq!(status.gear, GearPreset::P);
        assert!(status.is_parked);
        assert!(!status.should_observe);
        assert_eq!(status.risk_level, GearRiskLevel::None);
    }
}
