//! Gear Presets and Metadata
//!
//! Defines the P/N/D/S/M gear presets with their dial values,
//! display metadata, and transition validation logic.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::dials::DialValues;

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// Gear presets for intuitive control
///
/// Like a car's gear selector:
/// - P (Park): Everything off, no resource consumption
/// - N (Neutral): Observe only, no intervention
/// - D (Drive): Balanced autonomy for daily use
/// - S (Sport): High autonomy, aggressive mode
/// - M (Manual): Full user control over each dial
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GearPreset {
    /// Park - System inactive, no resource consumption
    P,
    /// Neutral - Observe and record only, no intervention
    N,
    /// Drive - Balanced autonomy for daily operation
    D,
    /// Sport - High autonomy, aggressive mode
    S,
    /// Manual - Full manual control over each dial
    M,
}

impl Default for GearPreset {
    fn default() -> Self {
        Self::D
    }
}

impl std::fmt::Display for GearPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::P => write!(f, "P"),
            Self::N => write!(f, "N"),
            Self::D => write!(f, "D"),
            Self::S => write!(f, "S"),
            Self::M => write!(f, "M"),
        }
    }
}

impl std::str::FromStr for GearPreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().trim() {
            "P" | "PARK" => Ok(Self::P),
            "N" | "NEUTRAL" => Ok(Self::N),
            "D" | "DRIVE" => Ok(Self::D),
            "S" | "SPORT" => Ok(Self::S),
            "M" | "MANUAL" => Ok(Self::M),
            _ => Err(format!("Unknown gear preset: {s}")),
        }
    }
}

/// Risk level for a gear
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GearRiskLevel {
    None,
    Low,
    Medium,
    High,
}

/// Detailed gear preset with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GearPresetDetail {
    /// Gear identifier
    pub gear: GearPreset,
    /// Short name
    pub name: String,
    /// Full description
    pub description: String,
    /// Dial values
    pub dials: DialValues,
    /// Icon/emoji for display
    pub icon: String,
    /// Color theme (CSS color)
    pub color: String,
    /// Risk level
    pub risk_level: GearRiskLevel,
    /// Scenarios where this gear is appropriate
    pub scenarios: Vec<String>,
}

/// Result of validating a gear transition
#[derive(Debug, Clone)]
pub struct GearTransitionValidation {
    pub allowed: bool,
    pub requires_confirmation: bool,
    pub reason: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Static Data
// ══════════════════════════════════════════════════════════════════════════════

/// Preset dial values for each gear
pub static GEAR_PRESETS: Lazy<HashMap<GearPreset, DialValues>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        GearPreset::P,
        DialValues {
            observe: 0,
            decide: 0,
            act: 0,
        },
    );
    m.insert(
        GearPreset::N,
        DialValues {
            observe: 50,
            decide: 0,
            act: 0,
        },
    );
    m.insert(
        GearPreset::D,
        DialValues {
            observe: 70,
            decide: 60,
            act: 40,
        },
    );
    m.insert(
        GearPreset::S,
        DialValues {
            observe: 90,
            decide: 80,
            act: 70,
        },
    );
    m.insert(
        GearPreset::M,
        DialValues {
            observe: 50,
            decide: 50,
            act: 50,
        },
    );
    m
});

/// Full preset details for each gear
pub static GEAR_PRESET_DETAILS: Lazy<HashMap<GearPreset, GearPresetDetail>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        GearPreset::P,
        GearPresetDetail {
            gear: GearPreset::P,
            name: "Park".to_string(),
            description: "System inactive, no resource consumption".to_string(),
            dials: GEAR_PRESETS[&GearPreset::P],
            icon: "🅿️".to_string(),
            color: "#6b7280".to_string(), // gray
            risk_level: GearRiskLevel::None,
            scenarios: vec![
                "System maintenance".to_string(),
                "Explicit user pause".to_string(),
                "Resource conservation".to_string(),
            ],
        },
    );
    m.insert(
        GearPreset::N,
        GearPresetDetail {
            gear: GearPreset::N,
            name: "Neutral".to_string(),
            description: "Observe and record only, no intervention".to_string(),
            dials: GEAR_PRESETS[&GearPreset::N],
            icon: "🔵".to_string(),
            color: "#3b82f6".to_string(), // blue
            risk_level: GearRiskLevel::Low,
            scenarios: vec![
                "Monitoring without action".to_string(),
                "Learning phase".to_string(),
                "Data collection".to_string(),
                "Post-incident review".to_string(),
            ],
        },
    );
    m.insert(
        GearPreset::D,
        GearPresetDetail {
            gear: GearPreset::D,
            name: "Drive".to_string(),
            description: "Balanced autonomy for daily operation".to_string(),
            dials: GEAR_PRESETS[&GearPreset::D],
            icon: "🟢".to_string(),
            color: "#22c55e".to_string(), // green
            risk_level: GearRiskLevel::Medium,
            scenarios: vec![
                "Normal development workflow".to_string(),
                "Routine tasks".to_string(),
                "Standard operation".to_string(),
                "Daily coding sessions".to_string(),
            ],
        },
    );
    m.insert(
        GearPreset::S,
        GearPresetDetail {
            gear: GearPreset::S,
            name: "Sport".to_string(),
            description: "High autonomy, aggressive mode".to_string(),
            dials: GEAR_PRESETS[&GearPreset::S],
            icon: "🔴".to_string(),
            color: "#ef4444".to_string(), // red
            risk_level: GearRiskLevel::High,
            scenarios: vec![
                "Time-critical tasks".to_string(),
                "Automated pipelines".to_string(),
                "Trusted environments".to_string(),
                "Batch processing".to_string(),
            ],
        },
    );
    m.insert(
        GearPreset::M,
        GearPresetDetail {
            gear: GearPreset::M,
            name: "Manual".to_string(),
            description: "Full manual control over each dial".to_string(),
            dials: GEAR_PRESETS[&GearPreset::M],
            icon: "⚙️".to_string(),
            color: "#8b5cf6".to_string(), // purple
            risk_level: GearRiskLevel::Medium,
            scenarios: vec![
                "Custom configurations".to_string(),
                "Experimental settings".to_string(),
                "Fine-tuning behavior".to_string(),
                "Special requirements".to_string(),
            ],
        },
    );
    m
});

// ══════════════════════════════════════════════════════════════════════════════
// Functions
// ══════════════════════════════════════════════════════════════════════════════

/// Get preset details for a gear
pub fn get_gear_preset_detail(gear: GearPreset) -> GearPresetDetail {
    GEAR_PRESET_DETAILS[&gear].clone()
}

/// Get all gear presets in order
pub fn get_all_gear_presets() -> Vec<GearPresetDetail> {
    vec![
        GEAR_PRESET_DETAILS[&GearPreset::P].clone(),
        GEAR_PRESET_DETAILS[&GearPreset::N].clone(),
        GEAR_PRESET_DETAILS[&GearPreset::D].clone(),
        GEAR_PRESET_DETAILS[&GearPreset::S].clone(),
        GEAR_PRESET_DETAILS[&GearPreset::M].clone(),
    ]
}

/// Get risk level for a gear
pub fn get_gear_risk_level(gear: GearPreset) -> GearRiskLevel {
    match gear {
        GearPreset::P => GearRiskLevel::None,
        GearPreset::N => GearRiskLevel::Low,
        GearPreset::D => GearRiskLevel::Medium,
        GearPreset::S => GearRiskLevel::High,
        GearPreset::M => GearRiskLevel::Medium,
    }
}

/// Suggest a gear based on context
pub fn suggest_gear(context: &GearSuggestionContext) -> GearPreset {
    if context.is_maintenance_mode {
        return GearPreset::P;
    }
    if context.is_learning_mode {
        return GearPreset::N;
    }
    if context.requires_human_review {
        return GearPreset::D;
    }
    if context.is_time_critical && context.is_trusted_environment {
        return GearPreset::S;
    }
    GearPreset::D // Default to Drive
}

/// Context for gear suggestion
#[derive(Debug, Clone, Default)]
pub struct GearSuggestionContext {
    pub is_time_critical: bool,
    pub is_trusted_environment: bool,
    pub requires_human_review: bool,
    pub is_learning_mode: bool,
    pub is_maintenance_mode: bool,
}

/// Validate gear transition (some transitions may require confirmation)
pub fn validate_gear_transition(from: GearPreset, to: GearPreset) -> GearTransitionValidation {
    // All transitions are allowed
    // But upgrading to Sport requires confirmation
    if to == GearPreset::S && from != GearPreset::S {
        return GearTransitionValidation {
            allowed: true,
            requires_confirmation: true,
            reason: Some("Sport mode enables high autonomy. Confirm this is intentional.".to_string()),
        };
    }

    GearTransitionValidation {
        allowed: true,
        requires_confirmation: false,
        reason: None,
    }
}

/// Check if a gear allows autonomous decisions
pub fn gear_allows_autonomous_decisions(gear: GearPreset) -> bool {
    !matches!(gear, GearPreset::P | GearPreset::N)
}

/// Check if a gear allows autonomous actions
pub fn gear_allows_autonomous_actions(gear: GearPreset) -> bool {
    gear == GearPreset::S
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gear_preset_count() {
        assert_eq!(GEAR_PRESETS.len(), 5);
        assert_eq!(GEAR_PRESET_DETAILS.len(), 5);
    }

    #[test]
    fn test_gear_preset_from_str() {
        assert_eq!("P".parse::<GearPreset>().unwrap(), GearPreset::P);
        assert_eq!("park".parse::<GearPreset>().unwrap(), GearPreset::P);
        assert_eq!("DRIVE".parse::<GearPreset>().unwrap(), GearPreset::D);
        assert!("invalid".parse::<GearPreset>().is_err());
    }

    #[test]
    fn test_gear_preset_display() {
        assert_eq!(GearPreset::P.to_string(), "P");
        assert_eq!(GearPreset::D.to_string(), "D");
    }

    #[test]
    fn test_gear_presets_values() {
        assert_eq!(GEAR_PRESETS[&GearPreset::P].observe, 0);
        assert_eq!(GEAR_PRESETS[&GearPreset::D].observe, 70);
        assert_eq!(GEAR_PRESETS[&GearPreset::S].act, 70);
    }

    #[test]
    fn test_get_all_gear_presets_order() {
        let presets = get_all_gear_presets();
        assert_eq!(presets[0].gear, GearPreset::P);
        assert_eq!(presets[1].gear, GearPreset::N);
        assert_eq!(presets[2].gear, GearPreset::D);
        assert_eq!(presets[3].gear, GearPreset::S);
        assert_eq!(presets[4].gear, GearPreset::M);
    }

    #[test]
    fn test_gear_risk_level() {
        assert_eq!(get_gear_risk_level(GearPreset::P), GearRiskLevel::None);
        assert_eq!(get_gear_risk_level(GearPreset::S), GearRiskLevel::High);
    }

    #[test]
    fn test_validate_gear_transition_to_sport() {
        let validation = validate_gear_transition(GearPreset::D, GearPreset::S);
        assert!(validation.allowed);
        assert!(validation.requires_confirmation);
    }

    #[test]
    fn test_validate_gear_transition_normal() {
        let validation = validate_gear_transition(GearPreset::P, GearPreset::D);
        assert!(validation.allowed);
        assert!(!validation.requires_confirmation);
    }

    #[test]
    fn test_gear_allows_autonomous() {
        assert!(!gear_allows_autonomous_decisions(GearPreset::P));
        assert!(!gear_allows_autonomous_decisions(GearPreset::N));
        assert!(gear_allows_autonomous_decisions(GearPreset::D));
        assert!(gear_allows_autonomous_decisions(GearPreset::S));

        assert!(!gear_allows_autonomous_actions(GearPreset::D));
        assert!(gear_allows_autonomous_actions(GearPreset::S));
    }

    #[test]
    fn test_suggest_gear() {
        let context = GearSuggestionContext::default();
        assert_eq!(suggest_gear(&context), GearPreset::D);

        let context = GearSuggestionContext {
            is_maintenance_mode: true,
            ..Default::default()
        };
        assert_eq!(suggest_gear(&context), GearPreset::P);

        let context = GearSuggestionContext {
            is_time_critical: true,
            is_trusted_environment: true,
            ..Default::default()
        };
        assert_eq!(suggest_gear(&context), GearPreset::S);
    }
}
