//! Three Dials Control System
//!
//! Independent control over three dimensions of autonomy:
//! - **Observe**: 0% = passive wait, 100% = active scanning
//! - **Decide**: 0% = suggest only, 100% = autonomous decision
//! - **Act**: 0% = wait for confirmation, 100% = immediate execution

use serde::{Deserialize, Serialize};

use super::presets::{GearPreset, GEAR_PRESETS};

// ══════════════════════════════════════════════════════════════════════════════
// Types
// ══════════════════════════════════════════════════════════════════════════════

/// Dial operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DialMode {
    /// Direct user control
    Manual,
    /// Auto-adjust within bounds
    Adaptive,
}

impl Default for DialMode {
    fn default() -> Self {
        Self::Manual
    }
}

/// Dial names for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DialName {
    Observe,
    Decide,
    Act,
}

/// Simple dial values (0-100)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialValues {
    pub observe: u8,
    pub decide: u8,
    pub act: u8,
}

impl Default for DialValues {
    fn default() -> Self {
        // Default to Drive preset
        Self {
            observe: 70,
            decide: 60,
            act: 40,
        }
    }
}

/// Configuration for a single dial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialConfig {
    /// Current value (0-100)
    pub value: u8,
    /// Operating mode
    pub mode: DialMode,
    /// Bounds for adaptive mode
    pub bounds: DialBounds,
}

impl Default for DialConfig {
    fn default() -> Self {
        Self {
            value: 50,
            mode: DialMode::Manual,
            bounds: DialBounds::default(),
        }
    }
}

/// Bounds for adaptive mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DialBounds {
    pub min: u8,
    pub max: u8,
}

impl Default for DialBounds {
    fn default() -> Self {
        Self { min: 0, max: 100 }
    }
}

/// Full three dials configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDialsConfig {
    pub observe: DialConfig,
    pub decide: DialConfig,
    pub act: DialConfig,
}

impl Default for ThreeDialsConfig {
    fn default() -> Self {
        Self {
            observe: DialConfig {
                value: 70,
                ..Default::default()
            },
            decide: DialConfig {
                value: 60,
                ..Default::default()
            },
            act: DialConfig {
                value: 40,
                ..Default::default()
            },
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Dial Struct
// ══════════════════════════════════════════════════════════════════════════════

/// A single dial with value, mode, and adaptive bounds
#[derive(Debug, Clone)]
pub struct Dial {
    value: u8,
    mode: DialMode,
    min: u8,
    max: u8,
}

impl Default for Dial {
    fn default() -> Self {
        Self::new(50)
    }
}

impl Dial {
    /// Create new dial with value
    pub fn new(value: u8) -> Self {
        Self {
            value: value.min(100),
            mode: DialMode::Manual,
            min: 0,
            max: 100,
        }
    }

    /// Create dial with full config
    pub fn with_config(value: u8, mode: DialMode, min: u8, max: u8) -> Self {
        let clamped_min = min.min(100);
        let clamped_max = max.min(100).max(clamped_min);
        let clamped_value = if mode == DialMode::Adaptive {
            value.min(100).max(clamped_min).min(clamped_max)
        } else {
            value.min(100)
        };

        Self {
            value: clamped_value,
            mode,
            min: clamped_min,
            max: clamped_max,
        }
    }

    /// Get current value
    pub fn value(&self) -> u8 {
        self.value
    }

    /// Set value, respecting bounds in adaptive mode
    pub fn set_value(&mut self, v: u8) {
        let clamped = v.min(100);
        self.value = if self.mode == DialMode::Adaptive {
            clamped.max(self.min).min(self.max)
        } else {
            clamped
        };
    }

    /// Get operating mode
    pub fn mode(&self) -> DialMode {
        self.mode
    }

    /// Set operating mode
    pub fn set_mode(&mut self, mode: DialMode) {
        self.mode = mode;
        if mode == DialMode::Adaptive {
            self.value = self.value.max(self.min).min(self.max);
        }
    }

    /// Get bounds
    pub fn bounds(&self) -> DialBounds {
        DialBounds {
            min: self.min,
            max: self.max,
        }
    }

    /// Set bounds (for adaptive mode)
    pub fn set_bounds(&mut self, min: u8, max: u8) {
        self.min = min.min(100);
        self.max = max.min(100).max(self.min);
        if self.mode == DialMode::Adaptive {
            self.value = self.value.max(self.min).min(self.max);
        }
    }

    /// Adjust value by delta
    pub fn adjust(&mut self, delta: i16) {
        let new_value = (self.value as i16 + delta).max(0).min(100) as u8;
        self.set_value(new_value);
    }

    /// Check if dial is active (value > 0)
    pub fn is_active(&self) -> bool {
        self.value > 0
    }

    /// Check if dial is above threshold (default 50)
    pub fn is_high(&self, threshold: Option<u8>) -> bool {
        self.value > threshold.unwrap_or(50)
    }

    /// Get value as float (0.0 - 1.0)
    pub fn as_float(&self) -> f32 {
        f32::from(self.value) / 100.0
    }

    /// Switch to manual mode
    pub fn to_manual(&mut self) {
        self.mode = DialMode::Manual;
    }

    /// Switch to adaptive mode with bounds
    pub fn to_adaptive(&mut self, min: u8, max: u8) {
        self.mode = DialMode::Adaptive;
        self.set_bounds(min, max);
    }

    /// Get config object
    pub fn to_config(&self) -> DialConfig {
        DialConfig {
            value: self.value,
            mode: self.mode,
            bounds: self.bounds(),
        }
    }

    /// Create from config
    pub fn from_config(config: &DialConfig) -> Self {
        Self::with_config(
            config.value,
            config.mode,
            config.bounds.min,
            config.bounds.max,
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ThreeDials Struct
// ══════════════════════════════════════════════════════════════════════════════

/// Three independent dials for observe/decide/act control
#[derive(Debug, Clone)]
pub struct ThreeDials {
    /// Observation intensity: 0% = passive, 100% = active scanning
    pub observe: Dial,
    /// Decision autonomy: 0% = suggest only, 100% = autonomous
    pub decide: Dial,
    /// Execution autonomy: 0% = wait for confirmation, 100% = immediate
    pub act: Dial,
    /// Current gear (None if custom values don't match any preset)
    gear: Option<GearPreset>,
}

impl Default for ThreeDials {
    fn default() -> Self {
        Self::from_gear(GearPreset::D)
    }
}

impl ThreeDials {
    /// Create with specific values
    pub fn new(observe: u8, decide: u8, act: u8) -> Self {
        let mut dials = Self {
            observe: Dial::new(observe),
            decide: Dial::new(decide),
            act: Dial::new(act),
            gear: None,
        };
        dials.gear = dials.detect_gear();
        dials
    }

    /// Create from a gear preset
    pub fn from_gear(gear: GearPreset) -> Self {
        let values = GEAR_PRESETS[&gear];
        Self {
            observe: Dial::new(values.observe),
            decide: Dial::new(values.decide),
            act: Dial::new(values.act),
            gear: Some(gear),
        }
    }

    /// Create with custom values (Manual mode)
    pub fn custom(observe: u8, decide: u8, act: u8) -> Self {
        Self {
            observe: Dial::new(observe),
            decide: Dial::new(decide),
            act: Dial::new(act),
            gear: Some(GearPreset::M),
        }
    }

    /// Create from config
    pub fn from_config(config: &ThreeDialsConfig) -> Self {
        let mut dials = Self {
            observe: Dial::from_config(&config.observe),
            decide: Dial::from_config(&config.decide),
            act: Dial::from_config(&config.act),
            gear: None,
        };
        dials.gear = dials.detect_gear();
        dials
    }

    /// Get current gear
    pub fn gear(&self) -> Option<GearPreset> {
        self.gear
    }

    /// Set gear preset
    pub fn set_gear(&mut self, gear: GearPreset, custom_values: Option<DialValues>) {
        if gear == GearPreset::M {
            if let Some(values) = custom_values {
                self.observe.set_value(values.observe);
                self.decide.set_value(values.decide);
                self.act.set_value(values.act);
            }
        } else {
            let values = GEAR_PRESETS[&gear];
            self.observe.set_value(values.observe);
            self.decide.set_value(values.decide);
            self.act.set_value(values.act);
        }
        self.gear = Some(gear);
    }

    /// Set a single dial and switch to Manual mode
    pub fn set_dial(&mut self, name: DialName, value: u8) {
        match name {
            DialName::Observe => self.observe.set_value(value),
            DialName::Decide => self.decide.set_value(value),
            DialName::Act => self.act.set_value(value),
        }
        self.gear = Some(GearPreset::M);
    }

    /// Get dial by name
    pub fn get_dial(&self, name: DialName) -> &Dial {
        match name {
            DialName::Observe => &self.observe,
            DialName::Decide => &self.decide,
            DialName::Act => &self.act,
        }
    }

    /// Get mutable dial by name
    pub fn get_dial_mut(&mut self, name: DialName) -> &mut Dial {
        match name {
            DialName::Observe => &mut self.observe,
            DialName::Decide => &mut self.decide,
            DialName::Act => &mut self.act,
        }
    }

    /// Check if observation should be active
    pub fn should_observe(&self) -> bool {
        self.observe.is_active()
    }

    /// Check if observation is in high/active mode
    pub fn is_observing_actively(&self) -> bool {
        self.observe.is_high(None)
    }

    /// Check if decisions should be made autonomously
    pub fn should_decide_autonomously(&self) -> bool {
        self.decide.is_high(None)
    }

    /// Check if actions should be executed immediately
    pub fn should_act_immediately(&self) -> bool {
        self.act.is_high(None)
    }

    /// Check if all dials are off (Park mode)
    pub fn is_parked(&self) -> bool {
        !self.observe.is_active() && !self.decide.is_active() && !self.act.is_active()
    }

    /// Get all values as object
    pub fn values(&self) -> DialValues {
        DialValues {
            observe: self.observe.value(),
            decide: self.decide.value(),
            act: self.act.value(),
        }
    }

    /// Get combined autonomy score (average of all dials)
    pub fn autonomy_score(&self) -> u8 {
        let sum = u16::from(self.observe.value())
            + u16::from(self.decide.value())
            + u16::from(self.act.value());
        (sum / 3) as u8
    }

    /// Get config object
    pub fn to_config(&self) -> ThreeDialsConfig {
        ThreeDialsConfig {
            observe: self.observe.to_config(),
            decide: self.decide.to_config(),
            act: self.act.to_config(),
        }
    }

    /// Detect which gear preset matches current values (if any)
    fn detect_gear(&self) -> Option<GearPreset> {
        let v = self.values();
        for (gear, preset) in GEAR_PRESETS.iter() {
            if v.observe == preset.observe && v.decide == preset.decide && v.act == preset.act {
                return Some(*gear);
            }
        }
        None
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dial_value_clamping() {
        let mut dial = Dial::new(50);
        dial.set_value(150);
        assert_eq!(dial.value(), 100);

        dial.set_value(0);
        assert_eq!(dial.value(), 0);
    }

    #[test]
    fn test_dial_adaptive_bounds() {
        let mut dial = Dial::with_config(50, DialMode::Adaptive, 30, 70);
        assert_eq!(dial.value(), 50);

        dial.set_value(10);
        assert_eq!(dial.value(), 30); // Clamped to min

        dial.set_value(90);
        assert_eq!(dial.value(), 70); // Clamped to max
    }

    #[test]
    fn test_dial_adjust() {
        let mut dial = Dial::new(50);
        dial.adjust(20);
        assert_eq!(dial.value(), 70);

        dial.adjust(-100);
        assert_eq!(dial.value(), 0);
    }

    #[test]
    fn test_dial_is_high() {
        let dial = Dial::new(60);
        assert!(dial.is_high(None)); // Default threshold 50
        assert!(dial.is_high(Some(50)));
        assert!(!dial.is_high(Some(70)));
    }

    #[test]
    fn test_three_dials_from_gear() {
        let dials = ThreeDials::from_gear(GearPreset::D);
        assert_eq!(dials.gear(), Some(GearPreset::D));
        assert_eq!(dials.observe.value(), 70);
        assert_eq!(dials.decide.value(), 60);
        assert_eq!(dials.act.value(), 40);
    }

    #[test]
    fn test_three_dials_set_dial_switches_to_manual() {
        let mut dials = ThreeDials::from_gear(GearPreset::D);
        dials.set_dial(DialName::Observe, 50);
        assert_eq!(dials.gear(), Some(GearPreset::M));
    }

    #[test]
    fn test_three_dials_is_parked() {
        let dials = ThreeDials::from_gear(GearPreset::P);
        assert!(dials.is_parked());

        let dials = ThreeDials::from_gear(GearPreset::D);
        assert!(!dials.is_parked());
    }

    #[test]
    fn test_three_dials_autonomy_score() {
        let dials = ThreeDials::from_gear(GearPreset::D);
        // (70 + 60 + 40) / 3 = 56.67 ≈ 56
        assert_eq!(dials.autonomy_score(), 56);

        let dials = ThreeDials::from_gear(GearPreset::S);
        // (90 + 80 + 70) / 3 = 80
        assert_eq!(dials.autonomy_score(), 80);
    }

    #[test]
    fn test_three_dials_detect_gear() {
        let dials = ThreeDials::new(70, 60, 40);
        assert_eq!(dials.gear(), Some(GearPreset::D));

        let dials = ThreeDials::new(50, 50, 50);
        assert_eq!(dials.gear(), Some(GearPreset::M));

        let dials = ThreeDials::new(75, 65, 45); // No matching preset
        assert_eq!(dials.gear(), None);
    }
}
