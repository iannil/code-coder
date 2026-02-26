//! Macro economic filter for trading decisions.
//!
//! Integrates with zero-workflow's economic_bridge to filter trades
//! based on macroeconomic conditions.

pub mod inventory_cycle;
pub mod hf_integration;

pub use inventory_cycle::{InventoryCycleAnalyzer, InventoryCycleInput, InventoryCyclePhase, InventoryCycleResult};
pub use hf_integration::{
    HighFrequencyEvidence,
    HighFrequencyMacroAnalyzer,
    IndicatorSignal,
    OfficialDataPrediction,
};

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
    /// Composite indicators for deeper analysis
    pub composite_indicators: CompositeIndicators,
    /// Policy cycle assessment
    pub policy_cycle: PolicyCycle,
}

/// Composite indicators derived from base data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompositeIndicators {
    /// PPI-CPI spread (positive = upstream profit pressure)
    pub ppi_cpi_spread: Option<f64>,
    /// M2-Social financing growth spread (liquidity adequacy)
    pub m2_sf_spread: Option<f64>,
    /// Real interest rate (LPR - CPI)
    pub real_interest_rate: Option<f64>,
    /// Export-Import spread (trade balance indicator)
    pub trade_spread: Option<f64>,
    /// PPI-CPI scissors signal
    pub scissors_signal: Option<ScissorsSignal>,
}

/// PPI-CPI scissors analysis signal
///
/// The "scissors" refers to the gap between PPI (producer prices) and CPI (consumer prices).
/// This spread reveals profit distribution across the industrial chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScissorsSignal {
    /// 正剪刀差 (PPI > CPI): Upstream profit squeeze, downstream pressure
    /// - Upper stream (raw materials, energy) profits high
    /// - Middle/downstream (manufacturing, retail) profits squeezed
    /// - "增收不增利" - revenue up but profit stagnant
    PositiveScissors,

    /// 负剪刀差 (PPI < CPI): Downstream profit improvement
    /// - Consumer goods companies benefit
    /// - Cost pressure easing for manufacturers
    /// - Favorable for consumption sector
    NegativeScissors,

    /// 剪刀差收窄: Scissors closing
    /// - Profit distribution normalizing
    /// - Economic balance improving
    ScissorsClosing,

    /// 中性: Balanced state
    /// - PPI ≈ CPI (within ±1%)
    /// - Normal profit distribution
    Neutral,
}

impl ScissorsSignal {
    /// Analyze PPI-CPI scissors and return signal
    pub fn analyze(ppi_yoy: f64, cpi_yoy: f64) -> Self {
        let spread = ppi_yoy - cpi_yoy;

        if spread > 3.0 {
            ScissorsSignal::PositiveScissors
        } else if spread < -2.0 {
            ScissorsSignal::NegativeScissors
        } else if spread.abs() < 1.0 {
            ScissorsSignal::Neutral
        } else {
            ScissorsSignal::ScissorsClosing
        }
    }

    /// Get investment implications
    pub fn investment_implication(&self) -> &'static str {
        match self {
            ScissorsSignal::PositiveScissors => "上游利润好，中下游承压 → 关注成本传导能力",
            ScissorsSignal::NegativeScissors => "下游利润改善 → 关注消费和制造业",
            ScissorsSignal::ScissorsClosing => "剪刀差收窄 → 利润分配趋于均衡",
            ScissorsSignal::Neutral => "价格传导顺畅 → 正常配置",
        }
    }

    /// Get affected sectors
    pub fn affected_sectors(&self) -> Vec<&'static str> {
        match self {
            ScissorsSignal::PositiveScissors => vec![
                "煤炭/有色/石化 ↑",
                "中游制造 ↓",
                "消费品 承压",
            ],
            ScissorsSignal::NegativeScissors => vec![
                "消费品 ↑",
                "制造业 ↑",
                "上游周期 ↓",
            ],
            ScissorsSignal::ScissorsClosing => vec!["各行业利润趋于均衡"],
            ScissorsSignal::Neutral => vec!["无明显行业倾向"],
        }
    }
}

impl std::fmt::Display for ScissorsSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ScissorsSignal::PositiveScissors => "正剪刀差（上游利好）",
            ScissorsSignal::NegativeScissors => "负剪刀差（下游利好）",
            ScissorsSignal::ScissorsClosing => "剪刀差收窄",
            ScissorsSignal::Neutral => "中性",
        };
        write!(f, "{}", s)
    }
}

/// Policy cycle assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PolicyCycle {
    /// Easing: rate cuts, RRR cuts, increased liquidity
    Easing,
    /// Neutral: stable policy stance
    #[default]
    Neutral,
    /// Tightening: rate hikes, liquidity withdrawal
    Tightening,
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
        // Determine cycle phase from PMI and industrial data
        let cycle_phase = self.determine_cycle_phase(data);

        // Calculate composite indicators
        let composite_indicators = self.calculate_composite_indicators(data);

        // Determine policy cycle
        let policy_cycle = self.determine_policy_cycle(data);

        // Calculate risk appetite
        let risk_appetite = self.calculate_risk_appetite(data);

        // Determine position multiplier
        let position_multiplier = self.calculate_position_multiplier(&cycle_phase, data);

        // Determine trading bias
        let trading_bias = self.determine_trading_bias(&cycle_phase, risk_appetite, data);

        // Generate notes
        let notes = self.generate_notes(&cycle_phase, &policy_cycle, &composite_indicators, data);

        MacroEnvironment {
            cycle_phase,
            m2_growth: data.m2_yoy,
            social_financing: data.social_financing,
            risk_appetite,
            pmi: data.pmi,
            position_multiplier,
            trading_bias,
            notes,
            composite_indicators,
            policy_cycle,
        }
    }

    /// Determine economic cycle phase using multiple indicators
    fn determine_cycle_phase(&self, data: &MacroDataResponse) -> EconomicCyclePhase {
        // Primary indicator: PMI
        let pmi_signal = match data.pmi {
            Some(pmi) if pmi > 52.0 => 2,  // Strong expansion
            Some(pmi) if pmi > 50.0 => 1,  // Mild expansion
            Some(pmi) if pmi > 48.0 => -1, // Mild contraction
            Some(_) => -2,                  // Strong contraction
            None => 0,                      // Unknown
        };

        // Secondary indicator: Industrial value added
        let industrial_signal = match data.industrial_value_added {
            Some(iva) if iva > 6.0 => 2,
            Some(iva) if iva > 4.0 => 1,
            Some(iva) if iva > 2.0 => -1,
            Some(_) => -2,
            None => 0,
        };

        // Tertiary indicator: Fixed asset investment
        let investment_signal = match data.fixed_asset_investment {
            Some(fai) if fai > 8.0 => 1,
            Some(fai) if fai > 4.0 => 0,
            Some(_) => -1,
            None => 0,
        };

        // Weighted composite score
        let composite_score = pmi_signal * 3 + industrial_signal * 2 + investment_signal;

        match composite_score {
            s if s >= 6 => EconomicCyclePhase::Expansion,
            s if s >= 2 => EconomicCyclePhase::EarlyRecovery,
            s if s >= -2 => EconomicCyclePhase::Slowdown,
            _ => EconomicCyclePhase::Contraction,
        }
    }

    /// Calculate composite indicators for deeper analysis
    fn calculate_composite_indicators(&self, data: &MacroDataResponse) -> CompositeIndicators {
        // PPI-CPI spread: positive indicates upstream profit pressure
        let ppi_cpi_spread = match (data.ppi_yoy, data.cpi_yoy) {
            (Some(ppi), Some(cpi)) => Some(ppi - cpi),
            _ => None,
        };

        // PPI-CPI scissors signal
        let scissors_signal = match (data.ppi_yoy, data.cpi_yoy) {
            (Some(ppi), Some(cpi)) => Some(ScissorsSignal::analyze(ppi, cpi)),
            _ => None,
        };

        // M2-Social financing spread: liquidity adequacy
        let m2_sf_spread = match (data.m2_yoy, data.social_financing) {
            (Some(m2), Some(sf)) => Some(m2 - sf),
            _ => None,
        };

        // Real interest rate: LPR - CPI
        let real_interest_rate = match (data.lpr_1y, data.cpi_yoy) {
            (Some(lpr), Some(cpi)) => Some(lpr - cpi),
            _ => None,
        };

        // Trade spread: Export - Import
        let trade_spread = match (data.export_yoy, data.import_yoy) {
            (Some(exp), Some(imp)) => Some(exp - imp),
            _ => None,
        };

        CompositeIndicators {
            ppi_cpi_spread,
            m2_sf_spread,
            real_interest_rate,
            trade_spread,
            scissors_signal,
        }
    }

    /// Determine policy cycle based on monetary indicators
    fn determine_policy_cycle(&self, data: &MacroDataResponse) -> PolicyCycle {
        // Check M2 growth trend
        let m2_signal = match data.m2_yoy {
            Some(m2) if m2 > 11.0 => 1,  // Easing
            Some(m2) if m2 < 8.0 => -1,  // Tightening
            _ => 0,                       // Neutral
        };

        // Check LPR level (lower = easing)
        let lpr_signal = match data.lpr_1y {
            Some(lpr) if lpr < 3.5 => 1,  // Easing
            Some(lpr) if lpr > 4.5 => -1, // Tightening
            _ => 0,                        // Neutral
        };

        // Combined signal
        match m2_signal + lpr_signal {
            s if s >= 1 => PolicyCycle::Easing,
            s if s <= -1 => PolicyCycle::Tightening,
            _ => PolicyCycle::Neutral,
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
    fn generate_notes(
        &self,
        cycle_phase: &EconomicCyclePhase,
        policy_cycle: &PolicyCycle,
        composite: &CompositeIndicators,
        data: &MacroDataResponse,
    ) -> String {
        let mut notes = Vec::new();

        // Economic cycle
        let cycle_str = match cycle_phase {
            EconomicCyclePhase::Expansion => "扩张期",
            EconomicCyclePhase::EarlyRecovery => "早期复苏",
            EconomicCyclePhase::Slowdown => "放缓期",
            EconomicCyclePhase::Contraction => "收缩期",
        };
        notes.push(format!("经济周期: {}", cycle_str));

        // Policy cycle
        let policy_str = match policy_cycle {
            PolicyCycle::Easing => "宽松",
            PolicyCycle::Neutral => "中性",
            PolicyCycle::Tightening => "紧缩",
        };
        notes.push(format!("政策周期: {}", policy_str));

        // PMI
        if let Some(pmi) = data.pmi {
            notes.push(format!("PMI: {:.1}", pmi));
        }

        // M2 and liquidity
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

        // PPI-CPI spread and scissors signal
        if let Some(spread) = composite.ppi_cpi_spread {
            let scissors_info = composite
                .scissors_signal
                .map(|s| format!(" [{}]", s))
                .unwrap_or_default();
            let impact = if spread > 2.0 {
                "上游承压"
            } else if spread < -2.0 {
                "下游承压"
            } else {
                "均衡"
            };
            notes.push(format!("PPI-CPI: {:.1}%pt ({}){}", spread, impact, scissors_info));
        }

        // Real interest rate
        if let Some(real_rate) = composite.real_interest_rate {
            let stance = if real_rate < 1.0 {
                "偏宽松"
            } else if real_rate > 2.5 {
                "偏紧"
            } else {
                "适中"
            };
            notes.push(format!("实际利率: {:.1}% ({})", real_rate, stance));
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
            composite_indicators: CompositeIndicators::default(),
            policy_cycle: PolicyCycle::Neutral,
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
    // Core indicators
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

    // Extended indicators - GDP and growth
    #[serde(default)]
    gdp_yoy: Option<f64>,
    #[serde(default)]
    industrial_value_added: Option<f64>,

    // Extended indicators - Demand side (Three drivers)
    #[serde(default)]
    fixed_asset_investment: Option<f64>,
    #[serde(default)]
    retail_sales: Option<f64>,
    #[serde(default)]
    export_yoy: Option<f64>,
    #[serde(default)]
    import_yoy: Option<f64>,

    // Extended indicators - Monetary policy
    #[serde(default)]
    lpr_1y: Option<f64>,
    #[serde(default)]
    mlf_rate: Option<f64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_data(pmi: Option<f64>, m2_yoy: Option<f64>) -> MacroDataResponse {
        MacroDataResponse {
            pmi,
            m2_yoy,
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
            gdp_yoy: None,
            industrial_value_added: None,
            fixed_asset_investment: None,
            retail_sales: None,
            export_yoy: None,
            import_yoy: None,
            lpr_1y: None,
            mlf_rate: None,
        }
    }

    #[test]
    fn test_cycle_phase_from_pmi() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        // Test expansion
        let data = make_test_data(Some(53.0), Some(10.0));
        let env = filter.analyze_macro_data(&data);
        assert_eq!(env.cycle_phase, EconomicCyclePhase::Expansion);

        // Test contraction
        let data = make_test_data(Some(47.0), Some(8.0));
        let env = filter.analyze_macro_data(&data);
        assert_eq!(env.cycle_phase, EconomicCyclePhase::Contraction);
    }

    #[test]
    fn test_position_multiplier() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        // Expansion should have higher multiplier
        let data = make_test_data(Some(53.0), Some(11.0));
        let env = filter.analyze_macro_data(&data);
        assert!(env.position_multiplier > 1.0);

        // Contraction should have lower multiplier
        let data = make_test_data(Some(46.0), Some(7.0));
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
        assert_eq!(env.policy_cycle, PolicyCycle::Neutral);
    }

    #[test]
    fn test_risk_appetite() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        let data = make_test_data(Some(55.0), Some(12.0));
        let risk = filter.calculate_risk_appetite(&data);

        // High PMI and loose M2 should give high risk appetite
        assert!(risk > 60.0);
    }

    #[test]
    fn test_composite_indicators() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        let data = MacroDataResponse {
            pmi: Some(50.0),
            m2_yoy: Some(10.0),
            social_financing: Some(9.0),
            cpi_yoy: Some(2.0),
            ppi_yoy: Some(4.0),
            gdp_yoy: Some(5.0),
            industrial_value_added: Some(5.5),
            fixed_asset_investment: Some(6.0),
            retail_sales: Some(7.0),
            export_yoy: Some(8.0),
            import_yoy: Some(5.0),
            lpr_1y: Some(3.45),
            mlf_rate: Some(2.5),
        };

        let composite = filter.calculate_composite_indicators(&data);

        // PPI-CPI spread should be 4.0 - 2.0 = 2.0
        assert!((composite.ppi_cpi_spread.unwrap() - 2.0).abs() < 0.001);

        // M2-SF spread should be 10.0 - 9.0 = 1.0
        assert!((composite.m2_sf_spread.unwrap() - 1.0).abs() < 0.001);

        // Real interest rate should be 3.45 - 2.0 = 1.45
        assert!((composite.real_interest_rate.unwrap() - 1.45).abs() < 0.001);

        // Trade spread should be 8.0 - 5.0 = 3.0
        assert!((composite.trade_spread.unwrap() - 3.0).abs() < 0.001);

        // Scissors signal should be ScissorsClosing (spread = 2.0, which is < 3.0)
        assert_eq!(composite.scissors_signal, Some(ScissorsSignal::ScissorsClosing));
    }

    #[test]
    fn test_scissors_signal_analysis() {
        // Positive scissors (PPI >> CPI)
        assert_eq!(ScissorsSignal::analyze(8.0, 2.0), ScissorsSignal::PositiveScissors);

        // Negative scissors (PPI << CPI)
        assert_eq!(ScissorsSignal::analyze(-1.0, 3.0), ScissorsSignal::NegativeScissors);

        // Neutral (PPI ≈ CPI)
        assert_eq!(ScissorsSignal::analyze(2.5, 2.0), ScissorsSignal::Neutral);

        // Scissors closing (moderate spread)
        assert_eq!(ScissorsSignal::analyze(4.0, 2.0), ScissorsSignal::ScissorsClosing);
    }

    #[test]
    fn test_policy_cycle_easing() {
        let config = Config::default();
        let filter = MacroFilter::new(&config);

        let data = MacroDataResponse {
            pmi: Some(50.0),
            m2_yoy: Some(12.0), // High M2 = easing
            social_financing: None,
            cpi_yoy: None,
            ppi_yoy: None,
            gdp_yoy: None,
            industrial_value_added: None,
            fixed_asset_investment: None,
            retail_sales: None,
            export_yoy: None,
            import_yoy: None,
            lpr_1y: Some(3.0), // Low LPR = easing
            mlf_rate: None,
        };

        let policy = filter.determine_policy_cycle(&data);
        assert_eq!(policy, PolicyCycle::Easing);
    }
}
