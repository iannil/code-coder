//! Macro economic filter for trading decisions.
//!
//! Integrates with zero-workflow's economic_bridge to filter trades
//! based on macroeconomic conditions.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zero_common::config::Config;

/// Economic cycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomicCyclePhase {
    /// Economy expanding (PMI > 52)
    Expansion,
    /// Early signs of recovery (PMI 50-52)
    EarlyRecovery,
    /// Growth slowing (PMI 48-50)
    Slowdown,
    /// Contraction (PMI < 48)
    Contraction,
}

/// Macro environment assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEnvironment {
    /// Current economic cycle phase
    pub cycle_phase: EconomicCyclePhase,
    /// M2 year-over-year growth rate
    pub m2_growth: Option<f64>,
    /// Social financing data
    pub social_financing: Option<f64>,
    /// Risk appetite indicator (0-100)
    pub risk_appetite: f64,
    /// PMI reading
    pub pmi: Option<f64>,
    /// Position multiplier based on macro (0.3 - 1.5)
    pub position_multiplier: f64,
    /// Trading bias based on macro
    pub trading_bias: TradingBias,
    /// Notes/analysis
    pub notes: String,
}

/// Trading bias from macro analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingBias {
    /// Favor long positions
    Bullish,
    /// Neutral stance
    Neutral,
    /// Favor short/defensive positions
    Bearish,
    /// Avoid trading, high uncertainty
    AvoidTrading,
}

/// Macro filter configuration
#[derive(Debug, Clone)]
pub struct MacroFilterConfig {
    /// Enable macro filtering
    pub enabled: bool,
    /// Workflow service endpoint
    pub workflow_endpoint: String,
    /// Cache duration in seconds
    pub cache_duration_secs: u64,
}

impl Default for MacroFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            workflow_endpoint: "http://127.0.0.1:4432".to_string(),
            cache_duration_secs: 3600, // 1 hour
        }
    }
}

/// Macro economic filter
pub struct MacroFilter {
    /// Configuration
    config: MacroFilterConfig,
    /// HTTP client
    client: reqwest::Client,
    /// Cached environment
    cached_env: tokio::sync::RwLock<Option<(MacroEnvironment, std::time::Instant)>>,
}

impl MacroFilter {
    /// Create a new macro filter
    pub fn new(config: &Config) -> Self {
        let filter_config = config
            .trading
            .as_ref()
            .map(|t| MacroFilterConfig {
                enabled: t.macro_filter_enabled.unwrap_or(true),
                // Use centralized workflow_endpoint from Config
                workflow_endpoint: config.workflow_endpoint(),
                cache_duration_secs: t.macro_cache_secs.unwrap_or(3600),
            })
            .unwrap_or_else(|| MacroFilterConfig {
                enabled: true,
                workflow_endpoint: config.workflow_endpoint(),
                cache_duration_secs: 3600,
            });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config: filter_config,
            client,
            cached_env: tokio::sync::RwLock::new(None),
        }
    }

    /// Get current macro environment
    pub async fn get_environment(&self) -> Result<MacroEnvironment> {
        if !self.config.enabled {
            return Ok(self.default_environment());
        }

        // Check cache
        {
            let cache = self.cached_env.read().await;
            if let Some((ref env, instant)) = *cache {
                if instant.elapsed().as_secs() < self.config.cache_duration_secs {
                    return Ok(env.clone());
                }
            }
        }

        // Fetch fresh data
        match self.fetch_macro_data().await {
            Ok(env) => {
                let mut cache = self.cached_env.write().await;
                *cache = Some((env.clone(), std::time::Instant::now()));
                Ok(env)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch macro data, using defaults");
                Ok(self.default_environment())
            }
        }
    }

    /// Fetch macro data from workflow service
    async fn fetch_macro_data(&self) -> Result<MacroEnvironment> {
        let url = format!("{}/api/v1/economic/china", self.config.workflow_endpoint);

        let response = self
            .client
            .get(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch macro data: {}", response.status());
        }

        let data: MacroDataResponse = response.json().await?;
        Ok(self.analyze_macro_data(&data))
    }

    /// Analyze raw macro data and produce environment assessment
    fn analyze_macro_data(&self, data: &MacroDataResponse) -> MacroEnvironment {
        // Determine cycle phase from PMI
        let cycle_phase = match data.pmi {
            Some(pmi) if pmi > 52.0 => EconomicCyclePhase::Expansion,
            Some(pmi) if pmi > 50.0 => EconomicCyclePhase::EarlyRecovery,
            Some(pmi) if pmi > 48.0 => EconomicCyclePhase::Slowdown,
            Some(_) => EconomicCyclePhase::Contraction,
            None => EconomicCyclePhase::Neutral,
        };

        // Calculate risk appetite
        let risk_appetite = self.calculate_risk_appetite(data);

        // Determine position multiplier
        let position_multiplier = self.calculate_position_multiplier(&cycle_phase, data);

        // Determine trading bias
        let trading_bias = self.determine_trading_bias(&cycle_phase, risk_appetite, data);

        // Generate notes
        let notes = self.generate_notes(&cycle_phase, data);

        MacroEnvironment {
            cycle_phase,
            m2_growth: data.m2_yoy,
            social_financing: data.social_financing,
            risk_appetite,
            pmi: data.pmi,
            position_multiplier,
            trading_bias,
            notes,
        }
    }

    /// Calculate risk appetite indicator (0-100)
    fn calculate_risk_appetite(&self, data: &MacroDataResponse) -> f64 {
        let mut score = 50.0; // Start neutral

        // PMI contribution
        if let Some(pmi) = data.pmi {
            score += (pmi - 50.0) * 2.0; // +/- 10 points per PMI point from 50
        }

        // M2 growth contribution
        if let Some(m2) = data.m2_yoy {
            if m2 > 10.0 {
                score += 10.0; // Loose liquidity
            } else if m2 < 8.0 {
                score -= 10.0; // Tight liquidity
            }
        }

        // Clamp to 0-100
        score.clamp(0.0, 100.0)
    }

    /// Calculate position size multiplier based on macro conditions
    fn calculate_position_multiplier(
        &self,
        cycle_phase: &EconomicCyclePhase,
        data: &MacroDataResponse,
    ) -> f64 {
        let mut multiplier: f64 = 1.0;

        // Cycle phase adjustment
        match cycle_phase {
            EconomicCyclePhase::Expansion => multiplier *= 1.2,
            EconomicCyclePhase::EarlyRecovery => multiplier *= 1.0,
            EconomicCyclePhase::Slowdown => multiplier *= 0.7,
            EconomicCyclePhase::Contraction => multiplier *= 0.5,
        }

        // M2 adjustment
        if let Some(m2) = data.m2_yoy {
            if m2 < 8.0 {
                multiplier *= 0.8; // Tight liquidity
            }
        }

        // Clamp to reasonable range
        multiplier.clamp(0.3, 1.5)
    }

    /// Determine trading bias
    fn determine_trading_bias(
        &self,
        cycle_phase: &EconomicCyclePhase,
        risk_appetite: f64,
        data: &MacroDataResponse,
    ) -> TradingBias {
        // High uncertainty - avoid trading
        if risk_appetite < 20.0 || risk_appetite > 80.0 {
            return TradingBias::AvoidTrading;
        }

        match cycle_phase {
            EconomicCyclePhase::Expansion => {
                if risk_appetite > 60.0 {
                    TradingBias::Bullish
                } else {
                    TradingBias::Neutral
                }
            }
            EconomicCyclePhase::EarlyRecovery => TradingBias::Bullish,
            EconomicCyclePhase::Slowdown => {
                if let Some(m2) = data.m2_yoy {
                    if m2 > 10.0 {
                        TradingBias::Neutral // Liquidity support
                    } else {
                        TradingBias::Bearish
                    }
                } else {
                    TradingBias::Bearish
                }
            }
            EconomicCyclePhase::Contraction => TradingBias::Bearish,
        }
    }

    /// Generate human-readable notes
    fn generate_notes(&self, cycle_phase: &EconomicCyclePhase, data: &MacroDataResponse) -> String {
        let mut notes = Vec::new();

        notes.push(format!("经济周期: {:?}", cycle_phase));

        if let Some(pmi) = data.pmi {
            notes.push(format!("PMI: {:.1}", pmi));
        }

        if let Some(m2) = data.m2_yoy {
            let liquidity = if m2 > 10.0 {
                "宽松"
            } else if m2 > 8.0 {
                "适中"
            } else {
                "偏紧"
            };
            notes.push(format!("M2同比: {:.1}% (流动性{})", m2, liquidity));
        }

        notes.join(" | ")
    }

    /// Default environment when macro data is unavailable
    fn default_environment(&self) -> MacroEnvironment {
        MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Neutral,
            m2_growth: None,
            social_financing: None,
            risk_appetite: 50.0,
            pmi: None,
            position_multiplier: 1.0,
            trading_bias: TradingBias::Neutral,
            notes: "宏观数据不可用，使用默认配置".to_string(),
        }
    }

    /// Adjust position size based on macro environment
    pub async fn adjust_position_size(&self, base_size: f64) -> f64 {
        match self.get_environment().await {
            Ok(env) => base_size * env.position_multiplier,
            Err(_) => base_size, // Use base size if macro unavailable
        }
    }

    /// Check if trading is recommended based on macro
    pub async fn is_trading_recommended(&self) -> bool {
        match self.get_environment().await {
            Ok(env) => env.trading_bias != TradingBias::AvoidTrading,
            Err(_) => true, // Default to allowing trading
        }
    }
}

// ============================================================================
// Additional Types
// ============================================================================

/// Add Neutral variant for EconomicCyclePhase when PMI is unknown
impl EconomicCyclePhase {
    #[allow(non_upper_case_globals)] // Semantic constant, not a flag
    const Neutral: Self = Self::EarlyRecovery; // Use EarlyRecovery as neutral default
}

/// Response from workflow service
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // API response struct - fields reserved for future use
struct MacroDataResponse {
    #[serde(default)]
    pmi: Option<f64>,
    #[serde(default)]
    m2_yoy: Option<f64>,
    #[serde(default)]
    social_financing: Option<f64>,
    #[serde(default)]
    cpi_yoy: Option<f64>,
    #[serde(default)]
    ppi_yoy: Option<f64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_phase_from_pmi() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        // Test expansion
        let data = MacroDataResponse {
            pmi: Some(53.0),
            m2_yoy: Some(10.0),
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
        };
        let env = filter.analyze_macro_data(&data);
        assert_eq!(env.cycle_phase, EconomicCyclePhase::Expansion);

        // Test contraction
        let data = MacroDataResponse {
            pmi: Some(47.0),
            m2_yoy: Some(8.0),
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
        };
        let env = filter.analyze_macro_data(&data);
        assert_eq!(env.cycle_phase, EconomicCyclePhase::Contraction);
    }

    #[test]
    fn test_position_multiplier() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        // Expansion should have higher multiplier
        let data = MacroDataResponse {
            pmi: Some(53.0),
            m2_yoy: Some(11.0),
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
        };
        let env = filter.analyze_macro_data(&data);
        assert!(env.position_multiplier > 1.0);

        // Contraction should have lower multiplier
        let data = MacroDataResponse {
            pmi: Some(46.0),
            m2_yoy: Some(7.0),
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
        };
        let env = filter.analyze_macro_data(&data);
        assert!(env.position_multiplier < 1.0);
    }

    #[test]
    fn test_default_environment() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);
        let env = filter.default_environment();

        assert_eq!(env.position_multiplier, 1.0);
        assert_eq!(env.trading_bias, TradingBias::Neutral);
    }

    #[test]
    fn test_risk_appetite() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        let data = MacroDataResponse {
            pmi: Some(55.0),
            m2_yoy: Some(12.0),
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
        };
        let risk = filter.calculate_risk_appetite(&data);

        // High PMI and loose M2 should give high risk appetite
        assert!(risk > 60.0);
    }
}
