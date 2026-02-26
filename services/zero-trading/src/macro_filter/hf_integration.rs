//! Integration between high-frequency data and macro analysis.
//!
//! This module provides tools to enhance macro analysis with high-frequency
//! economic indicators, following the "三步法" (three-step method):
//! 1. Cross-validation with multiple high-frequency sources
//! 2. Prediction of official data releases
//! 3. Validation and correction after official release

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::data::{HighFrequencyCollector, HighFrequencyConfig};
use crate::macro_agent::types::{HighFrequencyDataPoint, HighFrequencyIndicator};

// ============================================================================
// High-Frequency Evidence
// ============================================================================

/// Evidence from high-frequency indicators supporting a macro thesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighFrequencyEvidence {
    /// Indicators supporting bullish thesis
    pub bullish_signals: Vec<IndicatorSignal>,
    /// Indicators supporting bearish thesis
    pub bearish_signals: Vec<IndicatorSignal>,
    /// Neutral/mixed signals
    pub neutral_signals: Vec<IndicatorSignal>,
    /// Overall evidence strength (-100 to +100)
    pub net_score: f64,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Summary text
    pub summary: String,
}

/// A single indicator signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSignal {
    /// Indicator name
    pub indicator: String,
    /// Current value
    pub value: f64,
    /// Unit
    pub unit: String,
    /// Year-over-year change
    pub yoy_change: Option<f64>,
    /// Signal direction: positive = bullish, negative = bearish
    pub signal_direction: f64,
    /// Interpretation
    pub interpretation: String,
}

// ============================================================================
// Prediction Report
// ============================================================================

/// Prediction for an upcoming official data release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficialDataPrediction {
    /// Target indicator being predicted
    pub target_indicator: String,
    /// Predicted value
    pub predicted_value: f64,
    /// Confidence interval lower bound
    pub confidence_low: f64,
    /// Confidence interval upper bound
    pub confidence_high: f64,
    /// Consensus estimate (if available)
    pub consensus: Option<f64>,
    /// Our prediction vs consensus
    pub vs_consensus: Option<f64>,
    /// Supporting evidence
    pub evidence: Vec<(String, String)>, // (indicator, interpretation)
    /// Prediction date
    pub prediction_date: NaiveDate,
}

// ============================================================================
// High-Frequency Macro Analyzer
// ============================================================================

/// Analyzer that integrates high-frequency data with macro analysis
pub struct HighFrequencyMacroAnalyzer {
    /// High-frequency data collector
    collector: Arc<RwLock<HighFrequencyCollector>>,
}

impl HighFrequencyMacroAnalyzer {
    /// Create a new analyzer with the given collector
    pub fn new(collector: Arc<RwLock<HighFrequencyCollector>>) -> Self {
        Self { collector }
    }

    /// Create with default collector
    pub fn with_default_collector() -> anyhow::Result<Self> {
        let collector = HighFrequencyCollector::new(HighFrequencyConfig::default())?;
        Ok(Self {
            collector: Arc::new(RwLock::new(collector)),
        })
    }

    /// Build evidence from high-frequency indicators for industrial production
    pub async fn build_industrial_evidence(&self) -> anyhow::Result<HighFrequencyEvidence> {
        let collector = self.collector.read().await;

        let mut bullish = Vec::new();
        let mut bearish = Vec::new();
        let mut neutral = Vec::new();

        // Production indicators
        let production_indicators = [
            HighFrequencyIndicator::PowerCoalConsumption,
            HighFrequencyIndicator::BlastFurnaceRate,
            HighFrequencyIndicator::PtaLoadRate,
            HighFrequencyIndicator::TruckTireRate,
        ];

        for indicator in production_indicators {
            if let Ok(Some(data)) = collector.get_latest(indicator).await {
                let signal = self.evaluate_production_signal(&data);
                if signal.signal_direction > 0.3 {
                    bullish.push(signal);
                } else if signal.signal_direction < -0.3 {
                    bearish.push(signal);
                } else {
                    neutral.push(signal);
                }
            }
        }

        let net_score = self.calculate_net_score(&bullish, &bearish, &neutral);
        let confidence = self.calculate_confidence(&bullish, &bearish, &neutral);
        let summary = self.generate_industrial_summary(&bullish, &bearish, net_score);

        Ok(HighFrequencyEvidence {
            bullish_signals: bullish,
            bearish_signals: bearish,
            neutral_signals: neutral,
            net_score,
            confidence,
            summary,
        })
    }

    /// Build evidence for investment activity
    pub async fn build_investment_evidence(&self) -> anyhow::Result<HighFrequencyEvidence> {
        let collector = self.collector.read().await;

        let mut bullish = Vec::new();
        let mut bearish = Vec::new();
        let mut neutral = Vec::new();

        // Investment indicators
        let investment_indicators = [
            HighFrequencyIndicator::ExcavatorSales,
            HighFrequencyIndicator::AsphaltProductionRate,
            HighFrequencyIndicator::CementPriceIndex,
            HighFrequencyIndicator::RebarPrice,
        ];

        for indicator in investment_indicators {
            if let Ok(Some(data)) = collector.get_latest(indicator).await {
                let signal = self.evaluate_investment_signal(&data);
                if signal.signal_direction > 0.3 {
                    bullish.push(signal);
                } else if signal.signal_direction < -0.3 {
                    bearish.push(signal);
                } else {
                    neutral.push(signal);
                }
            }
        }

        let net_score = self.calculate_net_score(&bullish, &bearish, &neutral);
        let confidence = self.calculate_confidence(&bullish, &bearish, &neutral);
        let summary = self.generate_investment_summary(&bullish, &bearish, net_score);

        Ok(HighFrequencyEvidence {
            bullish_signals: bullish,
            bearish_signals: bearish,
            neutral_signals: neutral,
            net_score,
            confidence,
            summary,
        })
    }

    /// Build evidence for consumption activity
    pub async fn build_consumption_evidence(&self) -> anyhow::Result<HighFrequencyEvidence> {
        let collector = self.collector.read().await;

        let mut bullish = Vec::new();
        let mut bearish = Vec::new();
        let mut neutral = Vec::new();

        // Consumption indicators
        let consumption_indicators = [
            HighFrequencyIndicator::CityTrafficIndex,
            HighFrequencyIndicator::MetroPassengers,
            HighFrequencyIndicator::BoxOffice,
            HighFrequencyIndicator::ExpressDeliveryVolume,
        ];

        for indicator in consumption_indicators {
            if let Ok(Some(data)) = collector.get_latest(indicator).await {
                let signal = self.evaluate_consumption_signal(&data);
                if signal.signal_direction > 0.3 {
                    bullish.push(signal);
                } else if signal.signal_direction < -0.3 {
                    bearish.push(signal);
                } else {
                    neutral.push(signal);
                }
            }
        }

        let net_score = self.calculate_net_score(&bullish, &bearish, &neutral);
        let confidence = self.calculate_confidence(&bullish, &bearish, &neutral);
        let summary = self.generate_consumption_summary(&bullish, &bearish, net_score);

        Ok(HighFrequencyEvidence {
            bullish_signals: bullish,
            bearish_signals: bearish,
            neutral_signals: neutral,
            net_score,
            confidence,
            summary,
        })
    }

    /// Build evidence for real estate sector
    pub async fn build_real_estate_evidence(&self) -> anyhow::Result<HighFrequencyEvidence> {
        let collector = self.collector.read().await;

        let mut bullish = Vec::new();
        let mut bearish = Vec::new();
        let mut neutral = Vec::new();

        // Real estate indicators
        let real_estate_indicators = [
            HighFrequencyIndicator::HouseSales30City,
            HighFrequencyIndicator::LandTransaction100City,
            HighFrequencyIndicator::LandPremiumRate,
        ];

        for indicator in real_estate_indicators {
            if let Ok(Some(data)) = collector.get_latest(indicator).await {
                let signal = self.evaluate_real_estate_signal(&data);
                if signal.signal_direction > 0.3 {
                    bullish.push(signal);
                } else if signal.signal_direction < -0.3 {
                    bearish.push(signal);
                } else {
                    neutral.push(signal);
                }
            }
        }

        let net_score = self.calculate_net_score(&bullish, &bearish, &neutral);
        let confidence = self.calculate_confidence(&bullish, &bearish, &neutral);
        let summary = self.generate_real_estate_summary(&bullish, &bearish, net_score);

        Ok(HighFrequencyEvidence {
            bullish_signals: bullish,
            bearish_signals: bearish,
            neutral_signals: neutral,
            net_score,
            confidence,
            summary,
        })
    }

    /// Predict PMI based on high-frequency indicators
    pub async fn predict_pmi(&self) -> anyhow::Result<Option<OfficialDataPrediction>> {
        let industrial = self.build_industrial_evidence().await?;
        let investment = self.build_investment_evidence().await?;

        if industrial.confidence < 0.3 && investment.confidence < 0.3 {
            return Ok(None);
        }

        // Weighted average of net scores, normalized to PMI range
        let combined_score = industrial.net_score * 0.6 + investment.net_score * 0.4;

        // Map score (-100 to +100) to PMI range (typically 45-55)
        let predicted_pmi = 50.0 + combined_score * 0.05;
        let confidence_range = 1.5 / industrial.confidence.max(0.3);

        let mut evidence = Vec::new();
        for signal in industrial.bullish_signals.iter().chain(industrial.bearish_signals.iter()).take(3) {
            evidence.push((signal.indicator.clone(), signal.interpretation.clone()));
        }

        Ok(Some(OfficialDataPrediction {
            target_indicator: "制造业PMI".to_string(),
            predicted_value: predicted_pmi,
            confidence_low: predicted_pmi - confidence_range,
            confidence_high: predicted_pmi + confidence_range,
            consensus: None,
            vs_consensus: None,
            evidence,
            prediction_date: chrono::Local::now().date_naive(),
        }))
    }

    /// Get comprehensive high-frequency summary
    pub async fn get_comprehensive_summary(&self) -> anyhow::Result<String> {
        let industrial = self.build_industrial_evidence().await.ok();
        let investment = self.build_investment_evidence().await.ok();
        let consumption = self.build_consumption_evidence().await.ok();
        let real_estate = self.build_real_estate_evidence().await.ok();

        let mut sections = Vec::new();

        if let Some(ind) = industrial {
            sections.push(format!("【工业生产】{}", ind.summary));
        }
        if let Some(inv) = investment {
            sections.push(format!("【固定投资】{}", inv.summary));
        }
        if let Some(con) = consumption {
            sections.push(format!("【消费活动】{}", con.summary));
        }
        if let Some(re) = real_estate {
            sections.push(format!("【房地产】{}", re.summary));
        }

        if sections.is_empty() {
            Ok("高频数据暂无".to_string())
        } else {
            Ok(sections.join("\n"))
        }
    }

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    fn evaluate_production_signal(&self, data: &HighFrequencyDataPoint) -> IndicatorSignal {
        let signal_direction = data.yoy_change.map(|yoy| {
            // Normalize YoY change to signal direction
            (yoy / 10.0).clamp(-1.0, 1.0)
        }).unwrap_or(0.0);

        let interpretation = match data.indicator {
            HighFrequencyIndicator::PowerCoalConsumption => {
                if signal_direction > 0.3 { "发电耗煤量上升，工业用电回暖" }
                else if signal_direction < -0.3 { "发电耗煤量下降，工业活动放缓" }
                else { "发电耗煤量持平" }
            }
            HighFrequencyIndicator::BlastFurnaceRate => {
                if data.value > 85.0 { "高炉开工率高位，钢铁需求旺盛" }
                else if data.value < 75.0 { "高炉开工率偏低，需求疲软" }
                else { "高炉开工率正常" }
            }
            _ => "生产指标观察中",
        };

        IndicatorSignal {
            indicator: data.indicator.chinese_name().to_string(),
            value: data.value,
            unit: data.unit.clone(),
            yoy_change: data.yoy_change,
            signal_direction,
            interpretation: interpretation.to_string(),
        }
    }

    fn evaluate_investment_signal(&self, data: &HighFrequencyDataPoint) -> IndicatorSignal {
        let signal_direction = data.yoy_change.map(|yoy| {
            (yoy / 15.0).clamp(-1.0, 1.0)
        }).unwrap_or(0.0);

        let interpretation = match data.indicator {
            HighFrequencyIndicator::ExcavatorSales => {
                if signal_direction > 0.3 { "挖掘机销量回升，基建开工积极" }
                else if signal_direction < -0.3 { "挖掘机销量下滑，施工活动减弱" }
                else { "挖掘机销量平稳" }
            }
            HighFrequencyIndicator::CementPriceIndex => {
                if signal_direction > 0.3 { "水泥价格上涨，建筑需求改善" }
                else if signal_direction < -0.3 { "水泥价格下跌，需求偏弱" }
                else { "水泥价格稳定" }
            }
            _ => "投资指标观察中",
        };

        IndicatorSignal {
            indicator: data.indicator.chinese_name().to_string(),
            value: data.value,
            unit: data.unit.clone(),
            yoy_change: data.yoy_change,
            signal_direction,
            interpretation: interpretation.to_string(),
        }
    }

    fn evaluate_consumption_signal(&self, data: &HighFrequencyDataPoint) -> IndicatorSignal {
        let signal_direction = data.yoy_change.map(|yoy| {
            (yoy / 10.0).clamp(-1.0, 1.0)
        }).unwrap_or(0.0);

        let interpretation = match data.indicator {
            HighFrequencyIndicator::CityTrafficIndex => {
                if signal_direction > 0.3 { "城市拥堵指数上升，出行活跃" }
                else if signal_direction < -0.3 { "城市拥堵指数下降，出行减少" }
                else { "城市拥堵指数正常" }
            }
            HighFrequencyIndicator::BoxOffice => {
                if signal_direction > 0.3 { "票房回暖，娱乐消费复苏" }
                else if signal_direction < -0.3 { "票房低迷，消费意愿偏弱" }
                else { "票房表现平稳" }
            }
            _ => "消费指标观察中",
        };

        IndicatorSignal {
            indicator: data.indicator.chinese_name().to_string(),
            value: data.value,
            unit: data.unit.clone(),
            yoy_change: data.yoy_change,
            signal_direction,
            interpretation: interpretation.to_string(),
        }
    }

    fn evaluate_real_estate_signal(&self, data: &HighFrequencyDataPoint) -> IndicatorSignal {
        let signal_direction = data.yoy_change.map(|yoy| {
            (yoy / 20.0).clamp(-1.0, 1.0)
        }).unwrap_or(0.0);

        let interpretation = match data.indicator {
            HighFrequencyIndicator::HouseSales30City => {
                if signal_direction > 0.3 { "30城商品房成交回暖" }
                else if signal_direction < -0.3 { "30城商品房成交低迷" }
                else { "30城商品房成交平稳" }
            }
            HighFrequencyIndicator::LandPremiumRate => {
                if data.value > 15.0 { "土地溢价率较高，开发商拿地积极" }
                else if data.value < 5.0 { "土地溢价率低，市场预期谨慎" }
                else { "土地市场表现正常" }
            }
            _ => "房地产指标观察中",
        };

        IndicatorSignal {
            indicator: data.indicator.chinese_name().to_string(),
            value: data.value,
            unit: data.unit.clone(),
            yoy_change: data.yoy_change,
            signal_direction,
            interpretation: interpretation.to_string(),
        }
    }

    fn calculate_net_score(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        neutral: &[IndicatorSignal],
    ) -> f64 {
        let bullish_sum: f64 = bullish.iter().map(|s| s.signal_direction.abs()).sum();
        let bearish_sum: f64 = bearish.iter().map(|s| s.signal_direction.abs()).sum();
        let total = bullish_sum + bearish_sum + neutral.len() as f64 * 0.1;

        if total < 0.01 {
            return 0.0;
        }

        ((bullish_sum - bearish_sum) / total * 100.0).clamp(-100.0, 100.0)
    }

    fn calculate_confidence(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        neutral: &[IndicatorSignal],
    ) -> f64 {
        let total = bullish.len() + bearish.len() + neutral.len();
        if total == 0 {
            return 0.0;
        }

        // More data points = higher confidence, up to a limit
        let data_confidence = (total as f64 / 5.0).min(1.0);

        // Consensus (many same-direction signals) = higher confidence
        let majority = bullish.len().max(bearish.len()) as f64 / total as f64;
        let consensus_confidence = if majority > 0.7 { 1.0 } else { majority };

        data_confidence * 0.5 + consensus_confidence * 0.5
    }

    fn generate_industrial_summary(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        net_score: f64,
    ) -> String {
        let direction = if net_score > 20.0 {
            "工业生产活跃度改善"
        } else if net_score < -20.0 {
            "工业生产活跃度下降"
        } else {
            "工业生产活跃度平稳"
        };

        let details: Vec<String> = bullish.iter().chain(bearish.iter())
            .take(2)
            .map(|s| s.interpretation.clone())
            .collect();

        if details.is_empty() {
            direction.to_string()
        } else {
            format!("{}（{}）", direction, details.join("；"))
        }
    }

    fn generate_investment_summary(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        net_score: f64,
    ) -> String {
        let direction = if net_score > 20.0 {
            "固定资产投资回暖"
        } else if net_score < -20.0 {
            "固定资产投资放缓"
        } else {
            "固定资产投资平稳"
        };

        let details: Vec<String> = bullish.iter().chain(bearish.iter())
            .take(2)
            .map(|s| s.interpretation.clone())
            .collect();

        if details.is_empty() {
            direction.to_string()
        } else {
            format!("{}（{}）", direction, details.join("；"))
        }
    }

    fn generate_consumption_summary(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        net_score: f64,
    ) -> String {
        let direction = if net_score > 20.0 {
            "消费活动活跃"
        } else if net_score < -20.0 {
            "消费活动偏弱"
        } else {
            "消费活动平稳"
        };

        let details: Vec<String> = bullish.iter().chain(bearish.iter())
            .take(2)
            .map(|s| s.interpretation.clone())
            .collect();

        if details.is_empty() {
            direction.to_string()
        } else {
            format!("{}（{}）", direction, details.join("；"))
        }
    }

    fn generate_real_estate_summary(
        &self,
        bullish: &[IndicatorSignal],
        bearish: &[IndicatorSignal],
        net_score: f64,
    ) -> String {
        let direction = if net_score > 20.0 {
            "房地产市场回暖"
        } else if net_score < -20.0 {
            "房地产市场低迷"
        } else {
            "房地产市场平稳"
        };

        let details: Vec<String> = bullish.iter().chain(bearish.iter())
            .take(2)
            .map(|s| s.interpretation.clone())
            .collect();

        if details.is_empty() {
            direction.to_string()
        } else {
            format!("{}（{}）", direction, details.join("；"))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indicator_signal_creation() {
        let signal = IndicatorSignal {
            indicator: "发电耗煤量".to_string(),
            value: 75.5,
            unit: "万吨".to_string(),
            yoy_change: Some(5.2),
            signal_direction: 0.52,
            interpretation: "发电耗煤量上升，工业用电回暖".to_string(),
        };

        assert!(signal.signal_direction > 0.0);
        assert!(signal.yoy_change.is_some());
    }

    #[test]
    fn test_evidence_structure() {
        let evidence = HighFrequencyEvidence {
            bullish_signals: vec![],
            bearish_signals: vec![],
            neutral_signals: vec![],
            net_score: 0.0,
            confidence: 0.0,
            summary: "No data".to_string(),
        };

        assert_eq!(evidence.net_score, 0.0);
    }

    #[test]
    fn test_prediction_structure() {
        let prediction = OfficialDataPrediction {
            target_indicator: "PMI".to_string(),
            predicted_value: 51.2,
            confidence_low: 50.0,
            confidence_high: 52.5,
            consensus: Some(50.5),
            vs_consensus: Some(0.7),
            evidence: vec![("发电耗煤".to_string(), "上升".to_string())],
            prediction_date: chrono::Local::now().date_naive(),
        };

        assert!(prediction.predicted_value > 50.0);
        assert!(prediction.vs_consensus.is_some());
    }
}
