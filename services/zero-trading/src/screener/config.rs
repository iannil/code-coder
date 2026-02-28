//! Screener configuration module.
//!
//! Defines configuration structures for the full market screener.

use serde::{Deserialize, Serialize};

// ============================================================================
// Main Screener Configuration
// ============================================================================

/// Configuration for the full market screener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerConfig {
    /// Whether the screener is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Cron expression for daily scan schedule (e.g., "0 18 * * 1-5")
    #[serde(default = "default_schedule_cron")]
    pub schedule_cron: String,

    /// Cron expression for weekly data sync (e.g., "0 20 * * 7" for Sundays)
    ///
    /// Note: Use 7 (or SUN/SUNDAY) for Sunday, not 0, due to cron crate limitations.
    #[serde(default = "default_data_sync_cron")]
    pub data_sync_cron: String,

    /// Filter configuration
    #[serde(default)]
    pub filters: FilterConfig,

    /// Number of top stocks to pass to deep analysis phase
    #[serde(default = "default_deep_analysis_threshold")]
    pub deep_analysis_threshold: usize,

    /// Output configuration
    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for ScreenerConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            schedule_cron: default_schedule_cron(),
            data_sync_cron: default_data_sync_cron(),
            filters: FilterConfig::default(),
            deep_analysis_threshold: default_deep_analysis_threshold(),
            output: OutputConfig::default(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_schedule_cron() -> String {
    "0 18 * * 1-5".to_string() // 6 PM on weekdays
}

fn default_data_sync_cron() -> String {
    "0 20 * * 7".to_string() // 8 PM on Sundays (use 7, not 0, for cron crate compatibility)
}

fn default_deep_analysis_threshold() -> usize {
    200
}

// ============================================================================
// Filter Configuration
// ============================================================================

/// Filter configuration for screening stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    // === Scope Filters ===

    /// Target symbols to screen (optional).
    /// If set, only these symbols will be screened.
    /// If empty, all market stocks will be screened.
    #[serde(default)]
    pub target_symbols: Vec<String>,

    /// Include only these industries (optional).
    /// If set, only stocks in these industries will be screened.
    #[serde(default)]
    pub include_industries: Vec<String>,

    /// Exclude these industries (optional).
    /// Stocks in these industries will be filtered out.
    #[serde(default)]
    pub exclude_industries: Vec<String>,

    // === Basic Filters ===

    /// Exclude ST (Special Treatment) stocks
    #[serde(default = "default_true")]
    pub exclude_st: bool,

    /// Exclude Beijing Stock Exchange (BJ) stocks
    #[serde(default = "default_true")]
    pub exclude_bj: bool,

    /// Minimum days since listing (to exclude new stocks)
    #[serde(default = "default_min_listing_days")]
    pub min_listing_days: u32,

    /// Minimum market cap (in billion yuan)
    #[serde(default)]
    pub min_market_cap: Option<f64>,

    /// Maximum market cap (in billion yuan)
    #[serde(default)]
    pub max_market_cap: Option<f64>,

    // === Quality Filters ===

    /// Minimum 3-year average ROE (%)
    #[serde(default = "default_min_roe")]
    pub min_roe_3y: f64,

    /// Minimum gross margin (%)
    #[serde(default = "default_min_gross_margin")]
    pub min_gross_margin: f64,

    /// Maximum debt to equity ratio (%)
    #[serde(default = "default_max_debt_ratio")]
    pub max_debt_ratio: f64,

    /// Require healthy cash flow DNA (OCF+, ICF-, FCF+/0)
    #[serde(default = "default_true")]
    pub healthy_cash_flow_dna: bool,

    // === Valuation Filters ===

    #[serde(default)]
    pub valuation: ValuationFilterConfig,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            target_symbols: Vec::new(),
            include_industries: Vec::new(),
            exclude_industries: Vec::new(),
            exclude_st: true,
            exclude_bj: true,
            min_listing_days: default_min_listing_days(),
            min_market_cap: None,
            max_market_cap: None,
            min_roe_3y: default_min_roe(),
            min_gross_margin: default_min_gross_margin(),
            max_debt_ratio: default_max_debt_ratio(),
            healthy_cash_flow_dna: true,
            valuation: ValuationFilterConfig::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_min_listing_days() -> u32 {
    365 // At least 1 year since listing
}

fn default_min_roe() -> f64 {
    10.0 // 10% minimum ROE
}

fn default_min_gross_margin() -> f64 {
    20.0 // 20% minimum gross margin
}

fn default_max_debt_ratio() -> f64 {
    70.0 // Maximum 70% debt ratio
}

// ============================================================================
// Valuation Filter Configuration
// ============================================================================

/// Valuation-based filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationFilterConfig {
    /// Maximum PE percentile (historical comparison)
    /// e.g., 50 means only stocks below their historical median PE
    #[serde(default = "default_max_pe_percentile")]
    pub max_pe_percentile: u8,

    /// Maximum absolute PE ratio (optional hard cap)
    #[serde(default)]
    pub max_pe_absolute: Option<f64>,

    /// Maximum PB ratio
    #[serde(default)]
    pub max_pb: Option<f64>,

    /// Minimum dividend yield (%)
    #[serde(default = "default_min_dividend_yield")]
    pub min_dividend_yield: f64,

    /// Whether to use PB-ROE model for valuation
    #[serde(default = "default_true")]
    pub use_pb_roe_model: bool,
}

impl Default for ValuationFilterConfig {
    fn default() -> Self {
        Self {
            max_pe_percentile: default_max_pe_percentile(),
            max_pe_absolute: Some(30.0),
            max_pb: Some(5.0),
            min_dividend_yield: default_min_dividend_yield(),
            use_pb_roe_model: true,
        }
    }
}

fn default_max_pe_percentile() -> u8 {
    50 // Below historical median
}

fn default_min_dividend_yield() -> f64 {
    2.0 // 2% minimum dividend yield
}

// ============================================================================
// Output Configuration
// ============================================================================

/// Output configuration for screener results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Whether to send results via Telegram
    #[serde(default)]
    pub telegram_enabled: bool,

    /// Telegram chat ID for notifications (optional, uses default if not set)
    #[serde(default)]
    pub telegram_chat_id: Option<String>,

    /// Whether to save local report files
    #[serde(default = "default_true")]
    pub local_report_enabled: bool,

    /// Directory for local reports
    #[serde(default = "default_report_dir")]
    pub report_dir: String,

    /// Report formats to generate
    #[serde(default = "default_report_formats")]
    pub report_format: Vec<String>,

    /// Maximum number of stocks to include in notification
    #[serde(default = "default_notification_max_stocks")]
    pub notification_max_stocks: usize,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            telegram_enabled: false,
            telegram_chat_id: None,
            local_report_enabled: true,
            report_dir: default_report_dir(),
            report_format: default_report_formats(),
            notification_max_stocks: default_notification_max_stocks(),
        }
    }
}

fn default_report_dir() -> String {
    "~/.codecoder/reports/screener".to_string()
}

fn default_report_formats() -> Vec<String> {
    vec!["markdown".to_string(), "json".to_string()]
}

fn default_notification_max_stocks() -> usize {
    20
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScreenerConfig::default();
        assert!(config.enabled);
        assert!(config.filters.exclude_st);
        assert!(config.filters.exclude_bj);
        assert_eq!(config.filters.min_listing_days, 365);
        assert!((config.filters.min_roe_3y - 10.0).abs() < 0.001);
        assert!(config.filters.target_symbols.is_empty());
        assert!(config.filters.include_industries.is_empty());
        assert!(config.filters.exclude_industries.is_empty());
    }

    #[test]
    fn test_target_symbols_filter() {
        let mut config = FilterConfig::default();
        config.target_symbols = vec!["000001.SZ".to_string(), "600000.SH".to_string()];
        assert_eq!(config.target_symbols.len(), 2);
    }

    #[test]
    fn test_industry_filters() {
        let mut config = FilterConfig::default();
        config.include_industries = vec!["银行".to_string(), "医药".to_string()];
        config.exclude_industries = vec!["房地产".to_string()];
        assert_eq!(config.include_industries.len(), 2);
        assert_eq!(config.exclude_industries.len(), 1);
    }

    #[test]
    fn test_filter_config_default() {
        let config = FilterConfig::default();
        assert!(config.healthy_cash_flow_dna);
        assert!((config.max_debt_ratio - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_valuation_config_default() {
        let config = ValuationFilterConfig::default();
        assert_eq!(config.max_pe_percentile, 50);
        assert!(config.use_pb_roe_model);
    }

    #[test]
    fn test_output_config_default() {
        let config = OutputConfig::default();
        assert!(config.local_report_enabled);
        assert!(!config.telegram_enabled);
        assert_eq!(config.report_format.len(), 2);
    }

    #[test]
    fn test_config_serialization() {
        let config = ScreenerConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("enabled"));
        assert!(json.contains("filters"));
        assert!(json.contains("schedule_cron"));

        // Deserialize back
        let parsed: ScreenerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, config.enabled);
    }
}
