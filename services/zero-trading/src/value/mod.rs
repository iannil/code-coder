//! Value Analysis Engine.
//!
//! This module implements the "Observer Constructionism" (观察者建构论) value investing
//! framework, operating on four progressive layers:
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                     观察者建构论 (Four Progressive Layers)                    │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Layer 1: National Consensus (国家共识)                                      │
//! │    └─ Identify "safety" themes: energy/food/financial/industrial security    │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Layer 2: Evaluation Power (评估权)                                          │
//! │    └─ Analyze "who is asking whom": primary/secondary evaluators vs evaluated│
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Layer 3: Financial Verification (财务验证)                                  │
//! │    └─ "Printing machine" checklist: cash flow DNA, PE/PB/DY coordinates      │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Layer 4: Trade Execution (交易执行)                                         │
//! │    └─ "Trim weak, nurture strong": three-tier pools, red/yellow lights       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::value::{ValueAnalyzer, FinancialData};
//!
//! let analyzer = ValueAnalyzer::new(&config);
//!
//! // Analyze a company's financial health
//! let checklist = analyzer.analyze_printing_machine(&financial_data).await?;
//!
//! // Get cash flow DNA classification
//! let dna = analyzer.classify_cash_flow(&financial_data);
//!
//! // Assess evaluation power
//! let power = analyzer.assess_evaluation_power("600519", &industry_data).await?;
//! ```

pub mod consensus;
pub mod financial;
pub mod power;
pub mod report;
pub mod types;

// Re-export main types for convenience
pub use types::{
    CashFlowDNA, ConsensusSignal, EvaluationPowerScore, EvaluationTier, FinancialData, MoatType,
    PolicyReference, PrintingMachineChecklist, RoeDriver, SafetyTheme,
};

pub use consensus::{
    ConsensusAnalysis, ConsensusAnalyzer, ConsensusAnalyzerConfig, PolicyChangeType,
    PolicyDocument, ThemeStrength,
};

pub use financial::{CashFlowAnalyzer, FinancialVerifier};
pub use power::{
    BargainingPower, EvaluationPowerAnalyzer, EvaluationPowerConfig, EvaluationPowerInput,
    SupplyChainPosition,
};
pub use report::{
    InvestmentVerdict, OpportunityScanReport, PortfolioReviewReport, PositionSignalAlert,
    StockDeepDiveReport, ValueReportGenerator, ValueReportType,
};

use anyhow::Result;
use chrono::{Duration, Utc};
use std::sync::Arc;
use zero_common::config::Config;

use crate::data::LocalStorage;

/// Value analysis engine that combines all layers of analysis.
pub struct ValueAnalyzer {
    /// Financial verifier for printing machine analysis
    pub financial_verifier: Arc<FinancialVerifier>,
    /// Cash flow analyzer for DNA classification
    pub cash_flow_analyzer: Arc<CashFlowAnalyzer>,
    /// Evaluation power analyzer for supply chain position analysis
    pub evaluation_power_analyzer: Arc<EvaluationPowerAnalyzer>,
    /// Local storage for caching analysis results
    local_storage: Option<Arc<LocalStorage>>,
}

impl ValueAnalyzer {
    /// Create a new value analyzer.
    pub fn new(_config: &Config) -> Self {
        Self::with_local_storage(_config, None)
    }

    /// Create a new value analyzer with local storage.
    pub fn with_local_storage(_config: &Config, local_storage: Option<Arc<LocalStorage>>) -> Self {
        Self {
            financial_verifier: Arc::new(FinancialVerifier::new()),
            cash_flow_analyzer: Arc::new(CashFlowAnalyzer::new()),
            evaluation_power_analyzer: Arc::new(EvaluationPowerAnalyzer::new()),
            local_storage,
        }
    }

    /// Analyze a company using the printing machine checklist.
    pub fn analyze_printing_machine(
        &self,
        data: &FinancialData,
        qualitative_inputs: QualitativeInputs,
    ) -> Result<PrintingMachineChecklist> {
        self.financial_verifier
            .analyze(data, qualitative_inputs)
    }

    /// Analyze a company with caching support.
    /// Uses local storage to cache results for 24 hours.
    pub async fn analyze_printing_machine_cached(
        &self,
        data: &FinancialData,
        qualitative_inputs: QualitativeInputs,
    ) -> Result<PrintingMachineChecklist> {
        const ANALYSIS_TYPE: &str = "printing_machine";
        const CACHE_HOURS: i64 = 24;

        // Try cache first
        if let Some(ref storage) = self.local_storage {
            if let Ok(Some(cached)) = storage
                .get_analysis_cache::<PrintingMachineChecklist>(&data.symbol, ANALYSIS_TYPE)
                .await
            {
                tracing::debug!(symbol = %data.symbol, "Using cached printing machine analysis");
                return Ok(cached);
            }
        }

        // Compute analysis
        let result = self.analyze_printing_machine(data, qualitative_inputs)?;

        // Cache result
        if let Some(ref storage) = self.local_storage {
            let expires_at = Utc::now() + Duration::hours(CACHE_HOURS);
            if let Err(e) = storage
                .save_analysis_cache(&data.symbol, ANALYSIS_TYPE, &result, Some(expires_at))
                .await
            {
                tracing::warn!(error = %e, "Failed to cache printing machine analysis");
            }
        }

        Ok(result)
    }

    /// Classify cash flow DNA pattern.
    pub fn classify_cash_flow(&self, data: &FinancialData) -> CashFlowDNA {
        self.cash_flow_analyzer.classify(data)
    }

    /// Determine ROE driver using DuPont analysis.
    pub fn analyze_roe_driver(&self, data: &FinancialData) -> RoeDriver {
        self.cash_flow_analyzer.determine_roe_driver(data)
    }

    /// Analyze evaluation power for a company based on supply chain position.
    pub fn analyze_evaluation_power(&self, input: &EvaluationPowerInput) -> EvaluationPowerScore {
        self.evaluation_power_analyzer.analyze(input)
    }
}

/// Qualitative inputs for printing machine analysis.
///
/// These are subjective assessments that require business understanding
/// and cannot be derived purely from financial statements.
#[derive(Debug, Clone, Default)]
pub struct QualitativeInputs {
    /// Does the company have evaluation power in its supply chain?
    pub has_evaluation_power: bool,
    /// Is the business model simple and understandable?
    pub is_simple_and_understandable: bool,
    /// Is there demand stickiness (recurring revenue, subscription, habit)?
    pub has_demand_stickiness: bool,
    /// Is supply stable (not subject to commodity price swings)?
    pub has_supply_stability: bool,
    /// Is the company a market leader in its niche?
    pub is_market_leader: bool,
}

impl QualitativeInputs {
    /// Create inputs for a high-quality business.
    pub fn high_quality() -> Self {
        Self {
            has_evaluation_power: true,
            is_simple_and_understandable: true,
            has_demand_stickiness: true,
            has_supply_stability: true,
            is_market_leader: true,
        }
    }

    /// Create inputs for an unknown/unverified business.
    pub fn unknown() -> Self {
        Self::default()
    }

    /// Calculate qualitative score (0-100).
    pub fn score(&self) -> f64 {
        let criteria = [
            self.has_evaluation_power,
            self.is_simple_and_understandable,
            self.has_demand_stickiness,
            self.has_supply_stability,
            self.is_market_leader,
        ];
        let passed = criteria.iter().filter(|&&x| x).count();
        (passed as f64 / criteria.len() as f64) * 100.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualitative_inputs_score() {
        let high_quality = QualitativeInputs::high_quality();
        assert!((high_quality.score() - 100.0).abs() < 0.01);

        let unknown = QualitativeInputs::unknown();
        assert!((unknown.score() - 0.0).abs() < 0.01);

        let partial = QualitativeInputs {
            has_evaluation_power: true,
            is_simple_and_understandable: true,
            has_demand_stickiness: false,
            has_supply_stability: false,
            is_market_leader: true,
        };
        assert!((partial.score() - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_value_analyzer_creation() {
        let config = Config::default();
        let _analyzer = ValueAnalyzer::new(&config);
        // Analyzer created successfully if we reach here
    }
}
