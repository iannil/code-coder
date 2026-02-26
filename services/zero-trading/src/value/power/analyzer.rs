//! Evaluation Power Analyzer.
//!
//! Analyzes a company's position in the supply chain to determine its "evaluation power"
//! (评估权). The core question: "Who is asking whom?" (谁在求谁)
//!
//! # Evaluation Power Hierarchy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    评估权金字塔                                   │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  一级评估者 (Primary)                                            │
//! │    └─ 生态主宰者：茅台、腾讯、苹果                                │
//! │    └─ 特征：定价权绝对，供应商/客户都在"求"它                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  二级评估者 (Secondary)                                          │
//! │    └─ 关键节点：宁德时代、台积电、隐形冠军                        │
//! │    └─ 特征：在特定环节有强话语权，但非全链条主宰                  │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  被评估者 (Evaluated)                                            │
//! │    └─ 苦力模式：代工、组件封装、渠道商                            │
//! │    └─ 特征：利润率取决于上下游的"施舍"                           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Analysis Dimensions
//!
//! 1. **Pricing Power** (定价权): Can the company raise prices without losing customers?
//! 2. **Upstream Dependency** (上游依赖): How much can suppliers squeeze margins?
//! 3. **Downstream Dependency** (下游依赖): How much can customers squeeze margins?
//! 4. **Moat Type** (护城河类型): What protects the company's position?
//! 5. **Substitution Risk** (替代风险): How easily can customers switch?

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::value::types::{EvaluationPowerScore, EvaluationTier, MoatType};

// ============================================================================
// Industry Position Types
// ============================================================================

/// Industry position in supply chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SupplyChainPosition {
    /// Raw material / resource provider (原材料/资源)
    RawMaterial,
    /// Component manufacturer (零部件制造)
    Component,
    /// System integrator / assembly (系统集成/组装)
    SystemIntegrator,
    /// Brand owner / product company (品牌/产品公司)
    BrandOwner,
    /// Platform / ecosystem owner (平台/生态主宰)
    PlatformOwner,
    /// Distributor / channel (渠道/分销)
    Distributor,
    /// Service provider (服务提供商)
    ServiceProvider,
}

impl std::fmt::Display for SupplyChainPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawMaterial => write!(f, "原材料/资源"),
            Self::Component => write!(f, "零部件制造"),
            Self::SystemIntegrator => write!(f, "系统集成/组装"),
            Self::BrandOwner => write!(f, "品牌/产品公司"),
            Self::PlatformOwner => write!(f, "平台/生态主宰"),
            Self::Distributor => write!(f, "渠道/分销"),
            Self::ServiceProvider => write!(f, "服务提供商"),
        }
    }
}

/// Bargaining power level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BargainingPower {
    /// Absolute power - can dictate terms (绝对话语权)
    Absolute,
    /// Strong power - favorable negotiations (强话语权)
    Strong,
    /// Balanced - equal footing (均衡)
    Balanced,
    /// Weak - limited negotiating power (弱话语权)
    Weak,
    /// None - price taker (无话语权)
    None,
}

impl std::fmt::Display for BargainingPower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absolute => write!(f, "绝对话语权"),
            Self::Strong => write!(f, "强话语权"),
            Self::Balanced => write!(f, "均衡"),
            Self::Weak => write!(f, "弱话语权"),
            Self::None => write!(f, "无话语权"),
        }
    }
}

impl BargainingPower {
    /// Convert to numeric score (0-100).
    pub fn to_score(&self) -> f64 {
        match self {
            Self::Absolute => 100.0,
            Self::Strong => 80.0,
            Self::Balanced => 50.0,
            Self::Weak => 25.0,
            Self::None => 0.0,
        }
    }
}

// ============================================================================
// Analysis Input
// ============================================================================

/// Input data for evaluation power analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationPowerInput {
    /// Stock symbol
    pub symbol: String,
    /// Company name
    pub name: String,
    /// Industry / sector
    pub industry: String,

    // === Supply Chain Position ===
    /// Position in supply chain
    pub supply_chain_position: SupplyChainPosition,
    /// Number of major suppliers
    pub supplier_count: u32,
    /// Concentration of top 5 suppliers (% of COGS)
    pub top5_supplier_concentration: f64,
    /// Number of major customers
    pub customer_count: u32,
    /// Concentration of top 5 customers (% of revenue)
    pub top5_customer_concentration: f64,

    // === Pricing Analysis ===
    /// Has the company raised prices in last 3 years?
    pub has_raised_prices: bool,
    /// Price increase retained (% not lost to competition)
    pub price_increase_retention: f64,
    /// Gross margin trend (positive = improving)
    pub gross_margin_trend: f64,

    // === Moat Indicators ===
    /// Brand recognition score (0-100)
    pub brand_score: f64,
    /// Market share in primary segment (%)
    pub market_share: f64,
    /// R&D spending as % of revenue
    pub rd_intensity: f64,
    /// Number of key patents
    pub patent_count: u32,
    /// Customer switching cost (0-100)
    pub switching_cost: f64,
    /// Network effect strength (0-100)
    pub network_effect: f64,
    /// Regulatory/license barrier (0-100)
    pub regulatory_barrier: f64,

    // === Qualitative Indicators ===
    /// Is the product/service essential (not discretionary)?
    pub is_essential: bool,
    /// Is demand recurring (subscription, consumable)?
    pub is_recurring_demand: bool,
    /// Does company set industry standards?
    pub sets_industry_standards: bool,
}

impl Default for EvaluationPowerInput {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            name: String::new(),
            industry: String::new(),
            supply_chain_position: SupplyChainPosition::Component,
            supplier_count: 10,
            top5_supplier_concentration: 50.0,
            customer_count: 100,
            top5_customer_concentration: 30.0,
            has_raised_prices: false,
            price_increase_retention: 50.0,
            gross_margin_trend: 0.0,
            brand_score: 30.0,
            market_share: 10.0,
            rd_intensity: 3.0,
            patent_count: 0,
            switching_cost: 30.0,
            network_effect: 0.0,
            regulatory_barrier: 0.0,
            is_essential: false,
            is_recurring_demand: false,
            sets_industry_standards: false,
        }
    }
}

// ============================================================================
// Evaluation Power Analyzer
// ============================================================================

/// Configuration for evaluation power analysis.
#[derive(Debug, Clone)]
pub struct EvaluationPowerConfig {
    /// Weight for pricing power dimension
    pub pricing_power_weight: f64,
    /// Weight for upstream dependency dimension
    pub upstream_weight: f64,
    /// Weight for downstream dependency dimension
    pub downstream_weight: f64,
    /// Weight for moat strength dimension
    pub moat_weight: f64,
    /// Threshold for Primary tier (score)
    pub primary_threshold: f64,
    /// Threshold for Secondary tier (score)
    pub secondary_threshold: f64,
}

impl Default for EvaluationPowerConfig {
    fn default() -> Self {
        Self {
            pricing_power_weight: 0.30,
            upstream_weight: 0.20,
            downstream_weight: 0.20,
            moat_weight: 0.30,
            primary_threshold: 80.0,
            secondary_threshold: 55.0,
        }
    }
}

/// Analyzer for evaluation power assessment.
pub struct EvaluationPowerAnalyzer {
    config: EvaluationPowerConfig,
}

impl EvaluationPowerAnalyzer {
    /// Create a new analyzer with default config.
    pub fn new() -> Self {
        Self {
            config: EvaluationPowerConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: EvaluationPowerConfig) -> Self {
        Self { config }
    }

    /// Analyze evaluation power for a company.
    pub fn analyze(&self, input: &EvaluationPowerInput) -> EvaluationPowerScore {
        // Calculate dimension scores
        let pricing_power = self.calculate_pricing_power(input);
        let upstream_independence = self.calculate_upstream_independence(input);
        let downstream_power = self.calculate_downstream_power(input);
        let moat_strength = self.calculate_moat_strength(input);

        // Determine primary moat type
        let (moat_type, secondary_moats) = self.determine_moat_types(input);

        // Calculate overall score
        let score = self.config.pricing_power_weight * pricing_power
            + self.config.upstream_weight * upstream_independence
            + self.config.downstream_weight * downstream_power
            + self.config.moat_weight * moat_strength;

        // Determine tier
        let tier = if score >= self.config.primary_threshold {
            EvaluationTier::Primary
        } else if score >= self.config.secondary_threshold {
            EvaluationTier::Secondary
        } else {
            EvaluationTier::Evaluated
        };

        // Generate reasoning
        let reasoning = self.generate_reasoning(
            input,
            tier,
            pricing_power,
            upstream_independence,
            downstream_power,
            moat_strength,
            &moat_type,
        );

        EvaluationPowerScore {
            symbol: input.symbol.clone(),
            tier,
            score,
            moat_type,
            secondary_moats,
            pricing_power,
            upstream_dependency: 100.0 - upstream_independence,
            downstream_dependency: 100.0 - downstream_power,
            reasoning,
            analyzed_at: Utc::now(),
        }
    }

    /// Calculate pricing power score (0-100).
    fn calculate_pricing_power(&self, input: &EvaluationPowerInput) -> f64 {
        let mut score: f64 = 0.0;

        // Has raised prices successfully
        if input.has_raised_prices {
            score += 30.0;
            // How much was retained
            score += input.price_increase_retention * 0.3;
        }

        // Gross margin trend
        if input.gross_margin_trend > 0.0 {
            score += (input.gross_margin_trend * 5.0).min(20.0);
        }

        // Brand power contributes to pricing
        score += input.brand_score * 0.2;

        // Market leadership
        if input.market_share >= 30.0 {
            score += 15.0;
        } else if input.market_share >= 20.0 {
            score += 10.0;
        } else if input.market_share >= 10.0 {
            score += 5.0;
        }

        // Sets industry standards
        if input.sets_industry_standards {
            score += 15.0;
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate upstream independence score (0-100).
    /// Higher = less dependent on suppliers.
    fn calculate_upstream_independence(&self, input: &EvaluationPowerInput) -> f64 {
        let mut score: f64 = 50.0; // Base score

        // Supplier diversification
        if input.supplier_count >= 20 {
            score += 20.0;
        } else if input.supplier_count >= 10 {
            score += 10.0;
        } else if input.supplier_count <= 3 {
            score -= 15.0;
        }

        // Supplier concentration (lower is better)
        if input.top5_supplier_concentration <= 30.0 {
            score += 20.0;
        } else if input.top5_supplier_concentration <= 50.0 {
            score += 10.0;
        } else if input.top5_supplier_concentration >= 70.0 {
            score -= 20.0;
        }

        // Position advantage
        match input.supply_chain_position {
            SupplyChainPosition::PlatformOwner => score += 15.0,
            SupplyChainPosition::BrandOwner => score += 10.0,
            SupplyChainPosition::RawMaterial => score += 5.0,
            _ => {}
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate downstream power score (0-100).
    /// Higher = more power over customers.
    fn calculate_downstream_power(&self, input: &EvaluationPowerInput) -> f64 {
        let mut score: f64 = 50.0;

        // Customer diversification
        if input.customer_count >= 1000 {
            score += 20.0;
        } else if input.customer_count >= 100 {
            score += 10.0;
        } else if input.customer_count <= 10 {
            score -= 20.0;
        }

        // Customer concentration (lower is better)
        if input.top5_customer_concentration <= 20.0 {
            score += 20.0;
        } else if input.top5_customer_concentration <= 40.0 {
            score += 10.0;
        } else if input.top5_customer_concentration >= 60.0 {
            score -= 15.0;
        }

        // Switching cost creates power
        score += input.switching_cost * 0.2;

        // Essential products create power
        if input.is_essential {
            score += 10.0;
        }

        // Recurring demand creates stickiness
        if input.is_recurring_demand {
            score += 10.0;
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate moat strength score (0-100).
    fn calculate_moat_strength(&self, input: &EvaluationPowerInput) -> f64 {
        let mut score: f64 = 0.0;

        // Brand (max 25)
        score += (input.brand_score * 0.25).min(25.0);

        // Network effect (max 25)
        score += (input.network_effect * 0.25).min(25.0);

        // Switching cost (max 20)
        score += (input.switching_cost * 0.2).min(20.0);

        // Regulatory barrier (max 15)
        score += (input.regulatory_barrier * 0.15).min(15.0);

        // R&D / Patents (max 15)
        let rd_score = if input.rd_intensity >= 15.0 {
            10.0
        } else if input.rd_intensity >= 10.0 {
            7.0
        } else if input.rd_intensity >= 5.0 {
            4.0
        } else {
            0.0
        };
        let patent_score = if input.patent_count >= 1000 {
            5.0
        } else if input.patent_count >= 100 {
            3.0
        } else if input.patent_count >= 10 {
            1.0
        } else {
            0.0
        };
        score += rd_score + patent_score;

        score.clamp(0.0, 100.0)
    }

    /// Determine moat types based on input.
    fn determine_moat_types(&self, input: &EvaluationPowerInput) -> (MoatType, Vec<MoatType>) {
        let mut moat_scores: Vec<(MoatType, f64)> = vec![
            (MoatType::SpiritualTotem, input.brand_score),
            (MoatType::NetworkEffect, input.network_effect),
            (MoatType::SwitchingCost, input.switching_cost),
            (MoatType::RegulatoryBarrier, input.regulatory_barrier),
            (
                MoatType::TechnicalPatent,
                if input.patent_count >= 100 {
                    70.0
                } else {
                    input.patent_count as f64 * 0.7
                },
            ),
            (
                MoatType::ProcessKnowhow,
                if input.rd_intensity >= 10.0 {
                    60.0
                } else {
                    input.rd_intensity * 6.0
                },
            ),
        ];

        // Resource monopoly for raw material companies
        if input.supply_chain_position == SupplyChainPosition::RawMaterial
            && input.market_share >= 20.0
        {
            moat_scores.push((MoatType::ResourceMonopoly, input.market_share * 2.0));
        }

        // Cost advantage for high market share + scale
        if input.market_share >= 30.0 {
            moat_scores.push((MoatType::CostAdvantage, input.market_share));
        }

        // Sort by score
        moat_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Primary moat is the strongest
        let primary = if moat_scores[0].1 >= 50.0 {
            moat_scores[0].0
        } else {
            MoatType::None
        };

        // Secondary moats are others with score >= 40
        let secondary: Vec<MoatType> = moat_scores
            .iter()
            .skip(1)
            .filter(|(_, s)| *s >= 40.0)
            .map(|(m, _)| *m)
            .collect();

        (primary, secondary)
    }

    /// Generate human-readable reasoning.
    #[allow(clippy::too_many_arguments)]
    fn generate_reasoning(
        &self,
        input: &EvaluationPowerInput,
        tier: EvaluationTier,
        pricing_power: f64,
        upstream_independence: f64,
        downstream_power: f64,
        moat_strength: f64,
        moat_type: &MoatType,
    ) -> String {
        let mut parts = Vec::new();

        // Tier assessment
        parts.push(format!(
            "评估权层级：{}。综合评分反映公司在产业链中的地位。",
            tier
        ));

        // Position in supply chain
        parts.push(format!(
            "产业链位置：{}。",
            input.supply_chain_position
        ));

        // Pricing power
        let pricing_desc = if pricing_power >= 80.0 {
            "定价权极强，可主动提价"
        } else if pricing_power >= 60.0 {
            "定价权较强，有一定议价空间"
        } else if pricing_power >= 40.0 {
            "定价权一般，需跟随市场"
        } else {
            "定价权较弱，被动接受市场价格"
        };
        parts.push(format!("定价权评分 {:.0}：{}。", pricing_power, pricing_desc));

        // Upstream analysis
        let upstream_desc = if upstream_independence >= 80.0 {
            "上游依赖低，供应商分散"
        } else if upstream_independence >= 60.0 {
            "上游依赖适中"
        } else {
            "上游依赖较高，供应商集中"
        };
        parts.push(format!(
            "上游独立性 {:.0}：{}。",
            upstream_independence, upstream_desc
        ));

        // Downstream analysis
        let downstream_desc = if downstream_power >= 80.0 {
            "对下游话语权强，客户分散"
        } else if downstream_power >= 60.0 {
            "对下游话语权适中"
        } else {
            "对下游话语权弱，客户集中"
        };
        parts.push(format!(
            "下游话语权 {:.0}：{}。",
            downstream_power, downstream_desc
        ));

        // Moat
        if *moat_type != MoatType::None {
            parts.push(format!(
                "主要护城河：{}，护城河强度 {:.0}。",
                moat_type, moat_strength
            ));
        } else {
            parts.push("未发现明显护城河。".to_string());
        }

        parts.join("\n")
    }
}

impl Default for EvaluationPowerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_evaluator() {
        let analyzer = EvaluationPowerAnalyzer::new();

        // Moutai-like company: strong brand, pricing power, diversified customers
        let input = EvaluationPowerInput {
            symbol: "600519".to_string(),
            name: "贵州茅台".to_string(),
            industry: "白酒".to_string(),
            supply_chain_position: SupplyChainPosition::BrandOwner,
            supplier_count: 50,
            top5_supplier_concentration: 20.0,
            customer_count: 10000,
            top5_customer_concentration: 5.0,
            has_raised_prices: true,
            price_increase_retention: 95.0,
            gross_margin_trend: 2.0,
            brand_score: 95.0,
            market_share: 35.0,
            rd_intensity: 1.0,
            patent_count: 10,
            switching_cost: 70.0,
            network_effect: 20.0,
            regulatory_barrier: 80.0,
            is_essential: false,
            is_recurring_demand: true,
            sets_industry_standards: true,
        };

        let result = analyzer.analyze(&input);

        assert_eq!(result.tier, EvaluationTier::Primary);
        assert!(result.score >= 80.0, "Score: {}", result.score);
        assert_eq!(result.moat_type, MoatType::SpiritualTotem);
    }

    #[test]
    fn test_secondary_evaluator() {
        let analyzer = EvaluationPowerAnalyzer::new();

        // CATL-like company: technology leader but not ecosystem owner
        let input = EvaluationPowerInput {
            symbol: "300750".to_string(),
            name: "宁德时代".to_string(),
            industry: "动力电池".to_string(),
            supply_chain_position: SupplyChainPosition::Component,
            supplier_count: 30,
            top5_supplier_concentration: 40.0,
            customer_count: 50,
            top5_customer_concentration: 45.0,
            has_raised_prices: true,
            price_increase_retention: 70.0,
            gross_margin_trend: 1.0,
            brand_score: 60.0,
            market_share: 35.0,
            rd_intensity: 8.0,
            patent_count: 3000,
            switching_cost: 75.0,
            network_effect: 10.0,
            regulatory_barrier: 30.0,
            is_essential: true,
            is_recurring_demand: true,
            sets_industry_standards: true,
        };

        let result = analyzer.analyze(&input);

        assert_eq!(result.tier, EvaluationTier::Secondary);
        assert!(
            result.score >= 55.0 && result.score < 80.0,
            "Score: {}",
            result.score
        );
    }

    #[test]
    fn test_evaluated_entity() {
        let analyzer = EvaluationPowerAnalyzer::new();

        // Generic component supplier: no moat, price taker
        let input = EvaluationPowerInput {
            symbol: "000001".to_string(),
            name: "某代工厂".to_string(),
            industry: "电子制造".to_string(),
            supply_chain_position: SupplyChainPosition::SystemIntegrator,
            supplier_count: 5,
            top5_supplier_concentration: 80.0,
            customer_count: 3,
            top5_customer_concentration: 95.0,
            has_raised_prices: false,
            price_increase_retention: 0.0,
            gross_margin_trend: -1.0,
            brand_score: 10.0,
            market_share: 5.0,
            rd_intensity: 1.0,
            patent_count: 5,
            switching_cost: 20.0,
            network_effect: 0.0,
            regulatory_barrier: 0.0,
            is_essential: false,
            is_recurring_demand: false,
            sets_industry_standards: false,
        };

        let result = analyzer.analyze(&input);

        assert_eq!(result.tier, EvaluationTier::Evaluated);
        assert!(result.score < 55.0, "Score: {}", result.score);
        assert_eq!(result.moat_type, MoatType::None);
    }

    #[test]
    fn test_moat_type_detection() {
        let analyzer = EvaluationPowerAnalyzer::new();

        // Network effect company (like WeChat)
        let mut input = EvaluationPowerInput::default();
        input.network_effect = 90.0;
        input.brand_score = 40.0;

        let result = analyzer.analyze(&input);
        assert_eq!(result.moat_type, MoatType::NetworkEffect);

        // Regulatory barrier company (like a bank)
        input.network_effect = 30.0;
        input.regulatory_barrier = 85.0;

        let result = analyzer.analyze(&input);
        assert_eq!(result.moat_type, MoatType::RegulatoryBarrier);
    }

    #[test]
    fn test_bargaining_power_score() {
        assert!((BargainingPower::Absolute.to_score() - 100.0).abs() < 0.01);
        assert!((BargainingPower::None.to_score() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_supply_chain_position_display() {
        assert_eq!(SupplyChainPosition::PlatformOwner.to_string(), "平台/生态主宰");
        assert_eq!(SupplyChainPosition::Component.to_string(), "零部件制造");
    }
}
