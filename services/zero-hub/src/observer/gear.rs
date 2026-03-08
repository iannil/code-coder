//! Gear presets for the Observer Network.
//!
//! Provides car-like gear metaphor for intuitive control:
//! - **P (Park)**: Everything off, no resource consumption
//! - **N (Neutral)**: Observe only, no intervention
//! - **D (Drive)**: Balanced autonomy for daily use
//! - **S (Sport)**: High autonomy, aggressive mode
//! - **M (Manual)**: Full user control over each dial

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Gear preset for the Observer Network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gear {
    /// Park: All dials at 0%, system inactive
    #[serde(rename = "P")]
    P,
    /// Neutral: Observe at 50%, decide/act at 0%
    #[serde(rename = "N")]
    N,
    /// Drive: Balanced autonomy (70/60/40)
    #[serde(rename = "D")]
    D,
    /// Sport: High autonomy (90/80/70)
    #[serde(rename = "S")]
    S,
    /// Manual: User controls all dials individually
    #[serde(rename = "M")]
    M,
}

impl Gear {
    /// Get dial values for this gear preset.
    /// Returns (observe, decide, act) as percentages.
    pub fn dial_values(&self) -> (u8, u8, u8) {
        match self {
            Self::P => (0, 0, 0),
            Self::N => (50, 0, 0),
            Self::D => (70, 60, 40),
            Self::S => (90, 80, 70),
            Self::M => (50, 50, 50), // Default starting point for manual
        }
    }

    /// Get a human-readable name for this gear.
    pub fn name(&self) -> &'static str {
        match self {
            Self::P => "Park",
            Self::N => "Neutral",
            Self::D => "Drive",
            Self::S => "Sport",
            Self::M => "Manual",
        }
    }

    /// Get a description of what this gear mode does.
    pub fn description(&self) -> &'static str {
        match self {
            Self::P => "System inactive, no resource consumption",
            Self::N => "Observe and record only, no intervention",
            Self::D => "Balanced autonomy for daily operation",
            Self::S => "High autonomy, aggressive mode",
            Self::M => "Full manual control over each dial",
        }
    }

    /// Check if this gear allows autonomous decisions.
    pub fn allows_autonomous_decisions(&self) -> bool {
        !matches!(self, Self::P | Self::N)
    }

    /// Check if this gear allows autonomous actions.
    pub fn allows_autonomous_actions(&self) -> bool {
        matches!(self, Self::S)
    }

    /// Check if this is the manual gear.
    pub fn is_manual(&self) -> bool {
        matches!(self, Self::M)
    }

    /// Check if this gear is inactive (Park).
    pub fn is_parked(&self) -> bool {
        matches!(self, Self::P)
    }

    /// Get all available gears.
    pub fn all() -> &'static [Gear] {
        &[Gear::P, Gear::N, Gear::D, Gear::S, Gear::M]
    }
}

impl Default for Gear {
    fn default() -> Self {
        Self::D
    }
}

impl fmt::Display for Gear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::P => write!(f, "P"),
            Self::N => write!(f, "N"),
            Self::D => write!(f, "D"),
            Self::S => write!(f, "S"),
            Self::M => write!(f, "M"),
        }
    }
}

impl FromStr for Gear {
    type Err = GearParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "P" | "PARK" => Ok(Self::P),
            "N" | "NEUTRAL" => Ok(Self::N),
            "D" | "DRIVE" => Ok(Self::D),
            "S" | "SPORT" => Ok(Self::S),
            "M" | "MANUAL" => Ok(Self::M),
            _ => Err(GearParseError(s.to_string())),
        }
    }
}

/// Error when parsing a gear string.
#[derive(Debug, Clone)]
pub struct GearParseError(String);

impl fmt::Display for GearParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid gear '{}'. Valid gears: P, N, D, S, M",
            self.0
        )
    }
}

impl std::error::Error for GearParseError {}

/// Represents a gear transition for logging and events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GearTransition {
    /// Previous gear
    pub from: Gear,
    /// New gear
    pub to: Gear,
    /// Reason for transition
    pub reason: String,
    /// Timestamp of transition
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl GearTransition {
    /// Create a new gear transition.
    pub fn new(from: Gear, to: Gear, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            reason: reason.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Check if this is an upgrade (more autonomy).
    pub fn is_upgrade(&self) -> bool {
        let from_level = gear_autonomy_level(self.from);
        let to_level = gear_autonomy_level(self.to);
        to_level > from_level
    }

    /// Check if this is a downgrade (less autonomy).
    pub fn is_downgrade(&self) -> bool {
        let from_level = gear_autonomy_level(self.from);
        let to_level = gear_autonomy_level(self.to);
        to_level < from_level
    }
}

/// Get autonomy level for a gear (for comparison).
fn gear_autonomy_level(gear: Gear) -> u8 {
    match gear {
        Gear::P => 0,
        Gear::N => 1,
        Gear::D => 2,
        Gear::M => 2, // Manual is same level as Drive (depends on settings)
        Gear::S => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gear_dial_values() {
        assert_eq!(Gear::P.dial_values(), (0, 0, 0));
        assert_eq!(Gear::N.dial_values(), (50, 0, 0));
        assert_eq!(Gear::D.dial_values(), (70, 60, 40));
        assert_eq!(Gear::S.dial_values(), (90, 80, 70));
        assert_eq!(Gear::M.dial_values(), (50, 50, 50));
    }

    #[test]
    fn test_gear_from_str() {
        assert_eq!(Gear::from_str("P").unwrap(), Gear::P);
        assert_eq!(Gear::from_str("park").unwrap(), Gear::P);
        assert_eq!(Gear::from_str("DRIVE").unwrap(), Gear::D);
        assert_eq!(Gear::from_str("s").unwrap(), Gear::S);
        assert!(Gear::from_str("invalid").is_err());
    }

    #[test]
    fn test_gear_display() {
        assert_eq!(Gear::P.to_string(), "P");
        assert_eq!(Gear::D.to_string(), "D");
    }

    #[test]
    fn test_gear_autonomy() {
        assert!(!Gear::P.allows_autonomous_decisions());
        assert!(!Gear::N.allows_autonomous_decisions());
        assert!(Gear::D.allows_autonomous_decisions());
        assert!(Gear::S.allows_autonomous_decisions());

        assert!(!Gear::D.allows_autonomous_actions());
        assert!(Gear::S.allows_autonomous_actions());
    }

    #[test]
    fn test_gear_transition() {
        let transition = GearTransition::new(Gear::D, Gear::S, "User requested sport mode");
        assert!(transition.is_upgrade());
        assert!(!transition.is_downgrade());

        let downgrade = GearTransition::new(Gear::S, Gear::N, "High risk detected");
        assert!(!downgrade.is_upgrade());
        assert!(downgrade.is_downgrade());
    }

    #[test]
    fn test_gear_all() {
        let all = Gear::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&Gear::P));
        assert!(all.contains(&Gear::M));
    }
}
