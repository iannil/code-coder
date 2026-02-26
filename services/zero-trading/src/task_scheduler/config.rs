//! Configuration for preparation tasks that run 24/7.
//!
//! Preparation tasks are background jobs that can run at any time to
//! prepare data and computations for faster execution during trading hours.
//!
//! This module re-exports `PreparationTaskConfig` from `zero_common`
//! to maintain a single source of truth for configuration.

pub use zero_common::config::PreparationTaskConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PreparationTaskConfig::default();
        assert_eq!(config.data_preload_interval_secs, 300);
        assert_eq!(config.parameter_precompute_interval_secs, 600);
        assert_eq!(config.macro_analysis_interval_secs, 3600);
        assert!(config.enabled);
    }

    #[test]
    fn test_serialization() {
        let config = PreparationTaskConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: PreparationTaskConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.data_preload_interval_secs, parsed.data_preload_interval_secs);
    }
}
