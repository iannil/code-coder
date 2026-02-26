//! Report generation module for screener results.
//!
//! Generates reports in various formats:
//! - Markdown (for documentation)
//! - JSON (for API/programmatic use)
//! - Telegram message (for notifications)

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

use super::engine::{ScreenerResult, ScreenedStock};

// ============================================================================
// Report Format
// ============================================================================

/// Supported report formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// Markdown format (human-readable)
    Markdown,
    /// JSON format (machine-readable)
    Json,
    /// Telegram message format (notification)
    Telegram,
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Json => write!(f, "json"),
            Self::Telegram => write!(f, "telegram"),
        }
    }
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "telegram" | "tg" => Ok(Self::Telegram),
            _ => Err(format!("Unknown report format: {}", s)),
        }
    }
}

// ============================================================================
// Screener Report
// ============================================================================

/// Report generator for screener results.
pub struct ScreenerReport {
    result: ScreenerResult,
}

impl ScreenerReport {
    /// Create a new report from screener results.
    pub fn new(result: ScreenerResult) -> Self {
        Self { result }
    }

    /// Generate report in the specified format.
    pub fn generate(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Markdown => self.to_markdown(),
            ReportFormat::Json => self.to_json(),
            ReportFormat::Telegram => self.to_telegram_message(20),
        }
    }

    /// Save report to file.
    pub fn save_to_file(&self, path: &Path, format: ReportFormat) -> Result<PathBuf> {
        let content = self.generate(format);
        let extension = match format {
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
            ReportFormat::Telegram => "txt",
        };

        let file_path = if path.extension().is_none() {
            path.with_extension(extension)
        } else {
            path.to_path_buf()
        };

        // Ensure directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create report directory")?;
        }

        std::fs::write(&file_path, content)
            .context("Failed to write report file")?;

        Ok(file_path)
    }

    /// Generate markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str(&format!(
            "# 全市场筛选报告\n\n**扫描ID**: {}\n**时间**: {}\n**耗时**: {:.1}秒\n\n",
            self.result.id,
            self.result.completed_at.format("%Y-%m-%d %H:%M:%S"),
            self.result.duration_secs
        ));

        // Summary
        md.push_str("## 筛选摘要\n\n");
        md.push_str(&format!("- **总扫描**: {} 只股票\n", self.result.total_scanned));
        md.push_str(&format!("- **最终通过**: {} 只股票\n", self.result.stocks.len()));
        md.push_str(&format!("- **筛选条件**: {}\n\n", self.result.config_summary));

        // Filter funnel
        md.push_str("### 筛选漏斗\n\n");
        md.push_str("| 阶段 | 通过 | 淘汰 | 淘汰率 |\n");
        md.push_str("|------|------|------|--------|\n");
        for fr in &self.result.filter_results {
            md.push_str(&format!(
                "| {} | {} | {} | {:.1}% |\n",
                fr.stage, fr.passed, fr.eliminated, fr.elimination_rate
            ));
        }
        md.push_str("\n");

        // Top stocks table
        md.push_str("## 优选股票\n\n");
        md.push_str("| 代码 | 名称 | 行业 | 得分 | ROE | 毛利率 | PE | PB | 股息率 |\n");
        md.push_str("|------|------|------|------|-----|--------|----|----|--------|\n");

        for stock in self.result.stocks.iter().take(50) {
            md.push_str(&format!(
                "| {} | {} | {} | {:.1} | {:.1}% | {:.1}% | {:.1} | {:.2} | {:.1}% |\n",
                stock.symbol,
                &stock.name,
                stock.industry.as_deref().unwrap_or("-"),
                stock.quant_score,
                stock.financials.roe.unwrap_or(0.0),
                stock.financials.gross_margin.unwrap_or(0.0),
                stock.financials.pe_ttm.unwrap_or(0.0),
                stock.financials.pb.unwrap_or(0.0),
                stock.financials.dividend_yield.unwrap_or(0.0),
            ));
        }
        md.push_str("\n");

        // Deep analysis results (if any)
        let with_deep: Vec<_> = self.result.stocks.iter()
            .filter(|s| s.deep_analysis.is_some())
            .take(20)
            .collect();

        if !with_deep.is_empty() {
            md.push_str("## 深度分析 (印钞机筛选)\n\n");
            md.push_str("| 代码 | 名称 | 印钞机得分 | 现金流DNA | 是否合格 |\n");
            md.push_str("|------|------|------------|-----------|----------|\n");

            for stock in with_deep {
                if let Some(deep) = &stock.deep_analysis {
                    md.push_str(&format!(
                        "| {} | {} | {:.1} | {} | {} |\n",
                        stock.symbol,
                        &stock.name,
                        deep.printing_machine_score,
                        &deep.cash_flow_dna,
                        if deep.is_printing_machine { "✅" } else { "❌" }
                    ));
                }
            }
            md.push_str("\n");
        }

        // Footer
        md.push_str("---\n\n");
        md.push_str(&format!(
            "*报告生成于 {} UTC*\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        ));

        md
    }

    /// Generate JSON report.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.result).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate Telegram notification message.
    pub fn to_telegram_message(&self, max_stocks: usize) -> String {
        let mut msg = String::new();

        // Header with emoji
        msg.push_str(&format!(
            "📊 *全市场筛选完成*\n\n扫描 {} 只 → 精选 {} 只\n筛选条件: {}\n耗时: {:.1}秒\n\n",
            self.result.total_scanned,
            self.result.stocks.len(),
            self.result.config_summary,
            self.result.duration_secs
        ));

        // Top stocks
        msg.push_str("*🏆 优选股票 TOP 20*\n\n");

        for (i, stock) in self.result.stocks.iter().take(max_stocks).enumerate() {
            let deep_indicator = if stock.deep_analysis.as_ref().map_or(false, |d| d.is_printing_machine) {
                " 🖨️"
            } else {
                ""
            };

            msg.push_str(&format!(
                "{}. `{}` {}{}\n   ROE: {:.1}% | PE: {:.1} | DY: {:.1}%\n\n",
                i + 1,
                stock.symbol,
                stock.name,
                deep_indicator,
                stock.financials.roe.unwrap_or(0.0),
                stock.financials.pe_ttm.unwrap_or(0.0),
                stock.financials.dividend_yield.unwrap_or(0.0),
            ));
        }

        // Footer
        if self.result.stocks.len() > max_stocks {
            msg.push_str(&format!(
                "_...及其他 {} 只股票_\n\n",
                self.result.stocks.len() - max_stocks
            ));
        }

        msg.push_str(&format!(
            "📅 {}\n🔗 详细报告已保存至本地",
            self.result.completed_at.format("%Y-%m-%d %H:%M")
        ));

        msg
    }

    /// Get the underlying result.
    pub fn result(&self) -> &ScreenerResult {
        &self.result
    }

    /// Get all screened stocks.
    pub fn stocks(&self) -> &[ScreenedStock] {
        &self.result.stocks
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screener::engine::{FinancialSummary, DeepAnalysisSummary};
    use crate::screener::quantitative::FilterResult;
    use crate::screener::quantitative::FilterStage;

    fn create_test_result() -> ScreenerResult {
        ScreenerResult {
            id: "test_scan".to_string(),
            stocks: vec![
                ScreenedStock {
                    symbol: "000001.SZ".to_string(),
                    name: "平安银行".to_string(),
                    exchange: "SZ".to_string(),
                    industry: Some("银行".to_string()),
                    quant_score: 85.0,
                    financials: FinancialSummary {
                        roe: Some(12.5),
                        gross_margin: Some(35.0),
                        net_margin: Some(25.0),
                        debt_to_equity: Some(40.0),
                        pe_ttm: Some(8.5),
                        pb: Some(0.8),
                        dividend_yield: Some(5.0),
                        operating_cash_flow: Some(150.0),
                        period_end: "2024-12-31".to_string(),
                    },
                    deep_analysis: Some(DeepAnalysisSummary {
                        printing_machine_score: 78.0,
                        is_printing_machine: true,
                        cash_flow_dna: "现金奶牛型".to_string(),
                        reasoning: "优质银行".to_string(),
                    }),
                    screened_at: Utc::now(),
                },
            ],
            filter_results: vec![
                FilterResult::new(FilterStage::Basic, 4000, 3500),
                FilterResult::new(FilterStage::Quality, 3500, 500),
                FilterResult::new(FilterStage::Valuation, 500, 100),
            ],
            total_scanned: 4000,
            config_summary: "ROE>10%, GM>20%, DE<70%".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            duration_secs: 5.5,
        }
    }

    #[test]
    fn test_markdown_generation() {
        let result = create_test_result();
        let report = ScreenerReport::new(result);
        let md = report.to_markdown();

        assert!(md.contains("# 全市场筛选报告"));
        assert!(md.contains("000001.SZ"));
        assert!(md.contains("平安银行"));
        assert!(md.contains("筛选漏斗"));
    }

    #[test]
    fn test_json_generation() {
        let result = create_test_result();
        let report = ScreenerReport::new(result);
        let json = report.to_json();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"stocks\""));
        assert!(json.contains("000001.SZ"));
    }

    #[test]
    fn test_telegram_generation() {
        let result = create_test_result();
        let report = ScreenerReport::new(result);
        let msg = report.to_telegram_message(20);

        assert!(msg.contains("📊"));
        assert!(msg.contains("000001.SZ"));
        assert!(msg.contains("🖨️")); // Printing machine indicator
    }

    #[test]
    fn test_report_format_parsing() {
        assert_eq!("markdown".parse::<ReportFormat>().unwrap(), ReportFormat::Markdown);
        assert_eq!("md".parse::<ReportFormat>().unwrap(), ReportFormat::Markdown);
        assert_eq!("json".parse::<ReportFormat>().unwrap(), ReportFormat::Json);
        assert_eq!("telegram".parse::<ReportFormat>().unwrap(), ReportFormat::Telegram);
    }
}
