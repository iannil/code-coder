//! Screener engine module.
//!
//! The central orchestrator for full market screening operations.

use std::sync::Arc;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use zero_common::config::Config;

use crate::data::{DataProvider, LocalStorage, FinancialStatementData, StockInfo};
use crate::value::ValueAnalyzer;
use crate::value::QualitativeInputs;

use super::config::ScreenerConfig;
use super::quantitative::{QuantitativeFilter, FilterResult, FilterStage};

// ============================================================================
// Screened Stock
// ============================================================================

/// A stock that passed the screening process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenedStock {
    /// Stock symbol (e.g., "000001.SZ")
    pub symbol: String,
    /// Stock name
    pub name: String,
    /// Exchange
    pub exchange: String,
    /// Industry
    pub industry: Option<String>,
    /// Quantitative score (0-100)
    pub quant_score: f64,
    /// Financial data summary
    pub financials: FinancialSummary,
    /// Deep analysis result (if performed)
    pub deep_analysis: Option<DeepAnalysisSummary>,
    /// Screening timestamp
    pub screened_at: DateTime<Utc>,
}

/// Summary of financial metrics for a screened stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialSummary {
    /// ROE (%)
    pub roe: Option<f64>,
    /// Gross margin (%)
    pub gross_margin: Option<f64>,
    /// Net margin (%)
    pub net_margin: Option<f64>,
    /// Debt to equity ratio (%)
    pub debt_to_equity: Option<f64>,
    /// PE TTM
    pub pe_ttm: Option<f64>,
    /// PB
    pub pb: Option<f64>,
    /// Dividend yield (%)
    pub dividend_yield: Option<f64>,
    /// Operating cash flow
    pub operating_cash_flow: Option<f64>,
    /// Report period
    pub period_end: String,
}

impl From<&FinancialStatementData> for FinancialSummary {
    fn from(data: &FinancialStatementData) -> Self {
        Self {
            roe: data.roe,
            gross_margin: data.gross_margin,
            net_margin: data.net_margin,
            debt_to_equity: data.debt_to_equity,
            pe_ttm: data.pe_ttm,
            pb: data.pb,
            dividend_yield: data.dividend_yield,
            operating_cash_flow: data.operating_cash_flow,
            period_end: data.period_end.to_string(),
        }
    }
}

/// Summary of deep analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepAnalysisSummary {
    /// Printing machine checklist score (0-100)
    pub printing_machine_score: f64,
    /// Whether it qualifies as a "printing machine"
    pub is_printing_machine: bool,
    /// Cash flow DNA pattern
    pub cash_flow_dna: String,
    /// Brief reasoning
    pub reasoning: String,
}

// ============================================================================
// Screener Result
// ============================================================================

/// Result of a screening operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerResult {
    /// Screening ID (timestamp-based)
    pub id: String,
    /// Screened stocks (sorted by score descending)
    pub stocks: Vec<ScreenedStock>,
    /// Filter stage results
    pub filter_results: Vec<FilterResult>,
    /// Total stocks scanned
    pub total_scanned: usize,
    /// Screening configuration used
    pub config_summary: String,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub completed_at: DateTime<Utc>,
    /// Duration in seconds
    pub duration_secs: f64,
}

impl ScreenerResult {
    /// Get the top N stocks by score.
    pub fn top(&self, n: usize) -> Vec<&ScreenedStock> {
        self.stocks.iter().take(n).collect()
    }

    /// Get stocks in a specific industry.
    pub fn by_industry(&self, industry: &str) -> Vec<&ScreenedStock> {
        self.stocks
            .iter()
            .filter(|s| s.industry.as_deref() == Some(industry))
            .collect()
    }

    /// Summary string for logging.
    pub fn summary(&self) -> String {
        format!(
            "Screened {} stocks in {:.1}s: {} passed ({:.1}%)",
            self.total_scanned,
            self.duration_secs,
            self.stocks.len(),
            if self.total_scanned > 0 {
                (self.stocks.len() as f64 / self.total_scanned as f64) * 100.0
            } else {
                0.0
            }
        )
    }
}

// ============================================================================
// Screener Engine
// ============================================================================

/// The main screener engine.
///
/// Orchestrates the full market screening process:
/// 1. Load stock list from local storage or fetch from provider
/// 2. Apply quantitative filters (basic → quality → valuation)
/// 3. Optionally run deep analysis on top candidates
/// 4. Generate and return results
pub struct ScreenerEngine<P: DataProvider> {
    config: ScreenerConfig,
    provider: Arc<P>,
    storage: Arc<LocalStorage>,
    quant_filter: QuantitativeFilter,
    value_analyzer: ValueAnalyzer,
}

impl<P: DataProvider> ScreenerEngine<P> {
    /// Create a new screener engine.
    pub fn new(
        config: ScreenerConfig,
        provider: Arc<P>,
        storage: Arc<LocalStorage>,
        global_config: &Config,
    ) -> Self {
        let quant_filter = QuantitativeFilter::new(config.filters.clone());
        let value_analyzer = ValueAnalyzer::new(global_config);

        Self {
            config,
            provider,
            storage,
            quant_filter,
            value_analyzer,
        }
    }

    /// Run a full screening scan.
    ///
    /// This performs:
    /// 1. Basic filtering on stock list
    /// 2. Quality and valuation filtering on financial data
    /// 3. Deep analysis on top candidates
    /// 4. Final scoring and ranking
    pub async fn run_full_scan(&self) -> Result<ScreenerResult> {
        let started_at = Utc::now();
        let id = format!("scan_{}", started_at.format("%Y%m%d_%H%M%S"));

        info!(scan_id = %id, "Starting full market scan");

        // Phase 1: Basic filtering on stock list
        let (stocks, basic_result) = self.filter_stocks().await?;
        info!(
            passed = stocks.len(),
            eliminated = basic_result.eliminated,
            "Phase 1 (Basic filter) complete"
        );

        // Phase 2: Quality and valuation filtering
        let (financials, quality_result, valuation_result) =
            self.filter_financials(&stocks).await?;
        info!(
            passed = financials.len(),
            quality_eliminated = quality_result.eliminated,
            valuation_eliminated = valuation_result.eliminated,
            "Phase 2 (Quality + Valuation filter) complete"
        );

        // Phase 3: Score and rank
        let mut scored: Vec<(FinancialStatementData, f64)> = financials
            .iter()
            .map(|f| (f.clone(), self.quant_filter.score_stock(f)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Phase 4: Deep analysis on top candidates
        let top_count = self.config.deep_analysis_threshold.min(scored.len());
        let top_candidates: Vec<_> = scored.iter().take(top_count).collect();

        info!(
            count = top_candidates.len(),
            "Phase 3 (Deep analysis) starting"
        );

        let mut screened_stocks = Vec::with_capacity(top_candidates.len());
        for (data, score) in top_candidates {
            let stock_info = self.get_stock_info(&data.symbol).await;
            let deep_analysis = self.run_deep_analysis(&data.symbol).await;

            screened_stocks.push(ScreenedStock {
                symbol: data.symbol.clone(),
                name: stock_info
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| data.symbol.clone()),
                exchange: stock_info
                    .as_ref()
                    .map(|s| s.exchange.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                industry: stock_info.as_ref().and_then(|s| s.industry.clone()),
                quant_score: *score,
                financials: FinancialSummary::from(data),
                deep_analysis,
                screened_at: Utc::now(),
            });
        }

        let completed_at = Utc::now();
        let duration_secs = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;

        let result = ScreenerResult {
            id,
            stocks: screened_stocks,
            filter_results: vec![basic_result, quality_result, valuation_result],
            total_scanned: stocks.len(),
            config_summary: format!(
                "ROE>{}%, GM>{}%, DE<{}%",
                self.config.filters.min_roe_3y,
                self.config.filters.min_gross_margin,
                self.config.filters.max_debt_ratio
            ),
            started_at,
            completed_at,
            duration_secs,
        };

        info!(
            scan_id = %result.id,
            stocks = result.stocks.len(),
            duration = format!("{:.1}s", duration_secs),
            "Full market scan complete"
        );

        Ok(result)
    }

    /// Run a quick scan (quantitative filters only, no deep analysis).
    ///
    /// This is faster than full_scan and suitable for:
    /// - Quick market overview
    /// - Pre-filtering before detailed research
    /// - Testing filter configurations
    pub async fn run_quick_scan(&self) -> Result<ScreenerResult> {
        let started_at = Utc::now();
        let id = format!("quick_{}", started_at.format("%Y%m%d_%H%M%S"));

        info!(scan_id = %id, "Starting quick market scan");

        // Phase 1: Basic filtering
        let (stocks, basic_result) = self.filter_stocks().await?;

        // Phase 2: Quality and valuation filtering
        let (financials, quality_result, valuation_result) =
            self.filter_financials(&stocks).await?;

        // Score and rank (no deep analysis)
        let mut scored: Vec<(FinancialStatementData, f64)> = financials
            .iter()
            .map(|f| (f.clone(), self.quant_filter.score_stock(f)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Build results without deep analysis
        let mut screened_stocks = Vec::with_capacity(scored.len());
        for (data, score) in scored {
            let stock_info = self.get_stock_info(&data.symbol).await;

            screened_stocks.push(ScreenedStock {
                symbol: data.symbol.clone(),
                name: stock_info
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| data.symbol.clone()),
                exchange: stock_info
                    .as_ref()
                    .map(|s| s.exchange.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                industry: stock_info.as_ref().and_then(|s| s.industry.clone()),
                quant_score: score,
                financials: FinancialSummary::from(&data),
                deep_analysis: None,
                screened_at: Utc::now(),
            });
        }

        let completed_at = Utc::now();
        let duration_secs = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;

        let result = ScreenerResult {
            id,
            stocks: screened_stocks,
            filter_results: vec![basic_result, quality_result, valuation_result],
            total_scanned: stocks.len(),
            config_summary: format!(
                "ROE>{}%, GM>{}%, DE<{}%",
                self.config.filters.min_roe_3y,
                self.config.filters.min_gross_margin,
                self.config.filters.max_debt_ratio
            ),
            started_at,
            completed_at,
            duration_secs,
        };

        info!(
            scan_id = %result.id,
            stocks = result.stocks.len(),
            duration = format!("{:.1}s", duration_secs),
            "Quick market scan complete"
        );

        Ok(result)
    }

    // ========================================================================
    // Internal Methods
    // ========================================================================

    async fn filter_stocks(&self) -> Result<(Vec<StockInfo>, FilterResult)> {
        // Try local storage first
        let stocks = self
            .storage
            .get_stocks_by_filter(
                self.config.filters.exclude_st,
                self.config.filters.exclude_bj,
                Some(self.config.filters.min_listing_days),
            )
            .await
            .context("Failed to get stocks from local storage")?;

        if !stocks.is_empty() {
            debug!(count = stocks.len(), "Loaded stocks from local storage");
            let result = FilterResult::new(FilterStage::Basic, stocks.len(), stocks.len());
            return Ok((stocks, result));
        }

        // Fall back to provider
        info!("Local storage empty, fetching from provider");
        let all_stocks = self
            .provider
            .list_all_stocks()
            .await
            .context("Failed to fetch stock list from provider")?;

        // Save to local storage for next time
        let _ = self.storage.save_stock_list(&all_stocks, self.provider.name()).await;

        // Apply basic filter
        let (filtered, result) = self.quant_filter.filter_basic(&all_stocks);
        Ok((filtered, result))
    }

    async fn filter_financials(
        &self,
        stocks: &[StockInfo],
    ) -> Result<(Vec<FinancialStatementData>, FilterResult, FilterResult)> {
        // Get financial data for all stocks
        let symbols: Vec<String> = stocks.iter().map(|s| s.symbol()).collect();

        // Try local storage first
        let mut financials = self
            .storage
            .get_financial_statements_by_filter(
                Some(self.config.filters.min_roe_3y),
                Some(self.config.filters.min_gross_margin),
                Some(self.config.filters.max_debt_ratio),
                self.config.filters.healthy_cash_flow_dna,
            )
            .await
            .unwrap_or_default();

        // Filter to only stocks in our list
        let symbol_set: std::collections::HashSet<_> = symbols.iter().collect();
        financials.retain(|f| symbol_set.contains(&f.symbol));

        if financials.is_empty() {
            // Fall back to provider
            warn!("No local financial data, fetching from provider (this may take a while)");

            financials = self
                .provider
                .batch_get_financial_data(&symbols, None)
                .await
                .unwrap_or_default();

            // Save to local storage
            let _ = self
                .storage
                .save_financial_statements(&financials, self.provider.name())
                .await;
        }

        let input_count = financials.len();

        // Apply quality filter
        let (after_quality, quality_result) = self.quant_filter.filter_quality(&financials);

        // Apply valuation filter
        let (after_valuation, valuation_result) =
            self.quant_filter.filter_valuation(&after_quality);

        debug!(
            input = input_count,
            after_quality = after_quality.len(),
            after_valuation = after_valuation.len(),
            "Financial filtering complete"
        );

        Ok((after_valuation, quality_result, valuation_result))
    }

    async fn get_stock_info(&self, symbol: &str) -> Option<StockInfo> {
        // Try to find from local storage
        let stocks = self.storage.get_all_stocks().await.ok()?;
        let code = symbol.split('.').next().unwrap_or(symbol);
        stocks.into_iter().find(|s| s.code == code)
    }

    async fn run_deep_analysis(&self, symbol: &str) -> Option<DeepAnalysisSummary> {
        // Get financial data for deep analysis
        let financial = self.storage.get_financial_statement(symbol, None).await.ok()??;

        // Convert to the format expected by ValueAnalyzer
        let financial_data = convert_to_financial_data(&financial)?;

        // Use default qualitative inputs (conservative assumptions)
        let qualitative_inputs = QualitativeInputs::default();

        // Run printing machine analysis
        match self.value_analyzer.analyze_printing_machine(&financial_data, qualitative_inputs) {
            Ok(checklist) => Some(DeepAnalysisSummary {
                printing_machine_score: checklist.overall_score,
                is_printing_machine: checklist.is_printing_machine(),
                cash_flow_dna: checklist.cash_flow_dna.to_string(),
                reasoning: checklist.reasoning.clone(),
            }),
            Err(e) => {
                warn!(symbol, error = %e, "Failed deep analysis");
                None
            }
        }
    }
}

/// Convert FinancialStatementData to FinancialData for value analysis.
fn convert_to_financial_data(
    data: &FinancialStatementData,
) -> Option<crate::value::types::FinancialData> {
    use crate::value::types::FinancialData;

    let period_end = data
        .period_end
        .and_hms_opt(0, 0, 0)?
        .and_utc();

    // Calculate free cash flow
    let free_cash_flow = match (data.operating_cash_flow, data.capex) {
        (Some(ocf), Some(capex)) => ocf + capex,
        (Some(ocf), None) => ocf,
        _ => 0.0,
    };

    Some(FinancialData {
        symbol: data.symbol.clone(),
        period_end,
        revenue: data.revenue.unwrap_or(0.0),
        gross_profit: data.gross_profit.unwrap_or(0.0),
        operating_income: data.operating_income.unwrap_or(0.0),
        net_income: data.net_income.unwrap_or(0.0),
        interest_expense: data.interest_expense.unwrap_or(0.0),
        total_assets: data.total_assets.unwrap_or(0.0),
        total_equity: data.total_equity.unwrap_or(0.0),
        total_liabilities: data.total_liabilities.unwrap_or(0.0),
        cash: data.cash.unwrap_or(0.0),
        total_debt: data.total_debt.unwrap_or(0.0),
        operating_cash_flow: data.operating_cash_flow.unwrap_or(0.0),
        investing_cash_flow: data.investing_cash_flow.unwrap_or(0.0),
        financing_cash_flow: data.financing_cash_flow.unwrap_or(0.0),
        capex: data.capex.unwrap_or(0.0),
        free_cash_flow,
        avg_roe_5y: data.roe,
        avg_gross_margin_5y: data.gross_margin,
        avg_net_margin_5y: data.net_margin,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_summary_from_data() {
        let data = FinancialStatementData {
            symbol: "000001.SZ".to_string(),
            period_end: chrono::NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            report_type: "annual".to_string(),
            revenue: Some(1000.0),
            gross_profit: Some(300.0),
            operating_income: Some(150.0),
            net_income: Some(100.0),
            interest_expense: Some(10.0),
            total_assets: Some(5000.0),
            total_equity: Some(2000.0),
            total_liabilities: Some(3000.0),
            cash: Some(500.0),
            total_debt: Some(1000.0),
            shares_outstanding: Some(100.0),
            operating_cash_flow: Some(120.0),
            investing_cash_flow: Some(-80.0),
            financing_cash_flow: Some(-40.0),
            capex: Some(-50.0),
            roe: Some(15.0),
            roa: Some(5.0),
            gross_margin: Some(30.0),
            net_margin: Some(10.0),
            debt_to_equity: Some(50.0),
            current_ratio: Some(1.5),
            pe_ttm: Some(15.0),
            pb: Some(2.0),
            dividend_yield: Some(3.0),
        };

        let summary = FinancialSummary::from(&data);
        assert_eq!(summary.roe, Some(15.0));
        assert_eq!(summary.gross_margin, Some(30.0));
        assert_eq!(summary.pe_ttm, Some(15.0));
    }

    #[test]
    fn test_screener_result_top() {
        let result = ScreenerResult {
            id: "test".to_string(),
            stocks: vec![
                ScreenedStock {
                    symbol: "A".to_string(),
                    name: "A".to_string(),
                    exchange: "SZ".to_string(),
                    industry: None,
                    quant_score: 90.0,
                    financials: FinancialSummary {
                        roe: Some(20.0),
                        gross_margin: Some(30.0),
                        net_margin: Some(10.0),
                        debt_to_equity: Some(50.0),
                        pe_ttm: Some(15.0),
                        pb: Some(2.0),
                        dividend_yield: Some(3.0),
                        operating_cash_flow: Some(100.0),
                        period_end: "2024-12-31".to_string(),
                    },
                    deep_analysis: None,
                    screened_at: Utc::now(),
                },
                ScreenedStock {
                    symbol: "B".to_string(),
                    name: "B".to_string(),
                    exchange: "SH".to_string(),
                    industry: None,
                    quant_score: 80.0,
                    financials: FinancialSummary {
                        roe: Some(15.0),
                        gross_margin: Some(25.0),
                        net_margin: Some(8.0),
                        debt_to_equity: Some(60.0),
                        pe_ttm: Some(20.0),
                        pb: Some(3.0),
                        dividend_yield: Some(2.0),
                        operating_cash_flow: Some(80.0),
                        period_end: "2024-12-31".to_string(),
                    },
                    deep_analysis: None,
                    screened_at: Utc::now(),
                },
            ],
            filter_results: vec![],
            total_scanned: 100,
            config_summary: "test".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            duration_secs: 1.0,
        };

        let top = result.top(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].symbol, "A");
    }
}
