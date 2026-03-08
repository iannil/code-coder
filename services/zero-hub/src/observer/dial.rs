//! Three-dial control system for the Observer Network.
//!
//! Provides independent control over three dimensions:
//! - **Observe**: 0% = passive wait, 100% = active scanning
//! - **Decide**: 0% = suggest only, 100% = autonomous decision
//! - **Act**: 0% = wait for confirmation, 100% = immediate execution
//!
//! Each dial can operate in two modes:
//! - **Manual**: Fixed value set by user
//! - **Adaptive**: Value adjusts within bounds based on context

use serde::{Deserialize, Serialize};
use std::fmt;

/// Dial operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DialMode {
    /// Fixed value, does not change automatically
    Manual,
    /// Value adjusts within bounds based on context
    Adaptive,
}

impl Default for DialMode {
    fn default() -> Self {
        Self::Manual
    }
}

impl fmt::Display for DialMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manual => write!(f, "manual"),
            Self::Adaptive => write!(f, "adaptive"),
        }
    }
}

/// A single dial with value, mode, and adaptive bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dial {
    /// Current value (0-100)
    pub value: u8,
    /// Operating mode (manual or adaptive)
    pub mode: DialMode,
    /// Minimum value for adaptive mode
    pub min: u8,
    /// Maximum value for adaptive mode
    pub max: u8,
}

impl Dial {
    /// Create a new dial with a fixed value.
    pub fn new(value: u8) -> Self {
        Self {
            value: value.min(100),
            mode: DialMode::Manual,
            min: 0,
            max: 100,
        }
    }

    /// Create an adaptive dial with bounds.
    pub fn adaptive(value: u8, min: u8, max: u8) -> Self {
        let min = min.min(100);
        let max = max.min(100).max(min);
        let value = value.min(100).clamp(min, max);

        Self {
            value,
            mode: DialMode::Adaptive,
            min,
            max,
        }
    }

    /// Set the dial value, respecting bounds in adaptive mode.
    pub fn set(&mut self, value: u8) {
        let value = value.min(100);
        self.value = match self.mode {
            DialMode::Manual => value,
            DialMode::Adaptive => value.clamp(self.min, self.max),
        };
    }

    /// Adjust the dial by a delta amount.
    pub fn adjust(&mut self, delta: i16) {
        let new_value = (self.value as i16 + delta).clamp(0, 100) as u8;
        self.set(new_value);
    }

    /// Check if the dial is active (value > 0).
    pub fn is_active(&self) -> bool {
        self.value > 0
    }

    /// Check if the dial is above threshold (default 50).
    pub fn is_high(&self) -> bool {
        self.value > 50
    }

    /// Check if the dial is at or above a custom threshold.
    pub fn above(&self, threshold: u8) -> bool {
        self.value >= threshold
    }

    /// Get value as a float (0.0 - 1.0).
    pub fn as_float(&self) -> f32 {
        self.value as f32 / 100.0
    }

    /// Switch to manual mode with current value.
    pub fn to_manual(&mut self) {
        self.mode = DialMode::Manual;
    }

    /// Switch to adaptive mode with bounds.
    pub fn to_adaptive(&mut self, min: u8, max: u8) {
        self.mode = DialMode::Adaptive;
        self.min = min.min(100);
        self.max = max.min(100).max(self.min);
        self.value = self.value.clamp(self.min, self.max);
    }
}

impl Default for Dial {
    fn default() -> Self {
        Self::new(50)
    }
}

impl fmt::Display for Dial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            DialMode::Manual => write!(f, "{}%", self.value),
            DialMode::Adaptive => write!(f, "{}% [{}-{}]", self.value, self.min, self.max),
        }
    }
}

/// Three independent dials for observe/decide/act control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDials {
    /// Observation intensity: 0% = passive, 100% = active scanning
    pub observe: Dial,
    /// Decision autonomy: 0% = suggest only, 100% = autonomous
    pub decide: Dial,
    /// Execution autonomy: 0% = wait for confirmation, 100% = immediate
    pub act: Dial,
}

impl ThreeDials {
    /// Create dials with specific values.
    pub fn new(observe: u8, decide: u8, act: u8) -> Self {
        Self {
            observe: Dial::new(observe),
            decide: Dial::new(decide),
            act: Dial::new(act),
        }
    }

    /// Create dials with custom values (alias for new).
    pub fn custom(observe: u8, decide: u8, act: u8) -> Self {
        Self::new(observe, decide, act)
    }

    /// Create dials from a gear preset.
    pub fn from_gear(gear: super::gear::Gear) -> Self {
        use super::gear::Gear;

        match gear {
            Gear::P => Self::new(0, 0, 0),     // Park: everything off
            Gear::N => Self::new(50, 0, 0),    // Neutral: observe only
            Gear::D => Self::new(70, 60, 40),  // Drive: balanced autonomy
            Gear::S => Self::new(90, 80, 70),  // Sport: high autonomy
            Gear::M => Self::new(50, 50, 50),  // Manual: neutral starting point
        }
    }

    /// Check if observation should be active.
    pub fn should_observe(&self) -> bool {
        self.observe.is_active()
    }

    /// Check if observation is in high/active mode.
    pub fn is_observing_actively(&self) -> bool {
        self.observe.is_high()
    }

    /// Check if decisions should be made autonomously.
    pub fn should_decide_autonomously(&self) -> bool {
        self.decide.is_high()
    }

    /// Check if actions should be executed immediately.
    pub fn should_act_immediately(&self) -> bool {
        self.act.is_high()
    }

    /// Check if all dials are off (Park mode).
    pub fn is_parked(&self) -> bool {
        !self.observe.is_active() && !self.decide.is_active() && !self.act.is_active()
    }

    /// Get all values as a tuple.
    pub fn values(&self) -> (u8, u8, u8) {
        (self.observe.value, self.decide.value, self.act.value)
    }

    /// Get combined autonomy score (average of all dials).
    pub fn autonomy_score(&self) -> u8 {
        ((self.observe.value as u16 + self.decide.value as u16 + self.act.value as u16) / 3) as u8
    }

    /// Check if current values match a gear preset.
    pub fn matches_gear(&self) -> Option<super::gear::Gear> {
        use super::gear::Gear;

        let (o, d, a) = self.values();

        if o == 0 && d == 0 && a == 0 {
            Some(Gear::P)
        } else if o == 50 && d == 0 && a == 0 {
            Some(Gear::N)
        } else if o == 70 && d == 60 && a == 40 {
            Some(Gear::D)
        } else if o == 90 && d == 80 && a == 70 {
            Some(Gear::S)
        } else {
            None // Manual or custom
        }
    }
}

impl Default for ThreeDials {
    fn default() -> Self {
        Self::from_gear(super::gear::Gear::D)
    }
}

impl fmt::Display for ThreeDials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Observe: {}, Decide: {}, Act: {}",
            self.observe, self.decide, self.act
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dial_new() {
        let dial = Dial::new(75);
        assert_eq!(dial.value, 75);
        assert_eq!(dial.mode, DialMode::Manual);
    }

    #[test]
    fn test_dial_clamps_to_100() {
        let dial = Dial::new(150);
        assert_eq!(dial.value, 100);
    }

    #[test]
    fn test_dial_adaptive() {
        let dial = Dial::adaptive(50, 30, 80);
        assert_eq!(dial.value, 50);
        assert_eq!(dial.min, 30);
        assert_eq!(dial.max, 80);
        assert_eq!(dial.mode, DialMode::Adaptive);
    }

    #[test]
    fn test_dial_adaptive_clamps() {
        let mut dial = Dial::adaptive(50, 30, 80);

        dial.set(90); // Should clamp to max
        assert_eq!(dial.value, 80);

        dial.set(10); // Should clamp to min
        assert_eq!(dial.value, 30);
    }

    #[test]
    fn test_dial_adjust() {
        let mut dial = Dial::new(50);

        dial.adjust(20);
        assert_eq!(dial.value, 70);

        dial.adjust(-30);
        assert_eq!(dial.value, 40);

        dial.adjust(-100);
        assert_eq!(dial.value, 0);

        dial.adjust(200);
        assert_eq!(dial.value, 100);
    }

    #[test]
    fn test_dial_thresholds() {
        let low = Dial::new(30);
        let high = Dial::new(70);
        let zero = Dial::new(0);

        assert!(!low.is_high());
        assert!(high.is_high());
        assert!(!zero.is_active());
        assert!(low.is_active());
    }

    #[test]
    fn test_three_dials_from_gear() {
        use super::super::gear::Gear;

        let park = ThreeDials::from_gear(Gear::P);
        assert_eq!(park.values(), (0, 0, 0));

        let drive = ThreeDials::from_gear(Gear::D);
        assert_eq!(drive.values(), (70, 60, 40));

        let sport = ThreeDials::from_gear(Gear::S);
        assert_eq!(sport.values(), (90, 80, 70));
    }

    #[test]
    fn test_three_dials_behavior() {
        let drive = ThreeDials::from_gear(super::super::gear::Gear::D);

        assert!(drive.should_observe());
        assert!(drive.is_observing_actively());
        assert!(drive.should_decide_autonomously());
        assert!(!drive.should_act_immediately()); // 40% < 50%
    }

    #[test]
    fn test_three_dials_parked() {
        let park = ThreeDials::from_gear(super::super::gear::Gear::P);
        assert!(park.is_parked());

        let drive = ThreeDials::from_gear(super::super::gear::Gear::D);
        assert!(!drive.is_parked());
    }

    #[test]
    fn test_autonomy_score() {
        let drive = ThreeDials::from_gear(super::super::gear::Gear::D);
        // (70 + 60 + 40) / 3 = 56.67 -> 56
        assert_eq!(drive.autonomy_score(), 56);
    }

    #[test]
    fn test_matches_gear() {
        use super::super::gear::Gear;

        let drive = ThreeDials::from_gear(Gear::D);
        assert_eq!(drive.matches_gear(), Some(Gear::D));

        let custom = ThreeDials::new(50, 50, 50);
        assert_eq!(custom.matches_gear(), None);
    }
}
