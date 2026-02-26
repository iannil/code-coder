//! Inventory cycle (Kitchin cycle) analysis module.
//!
//! Implements the four-phase inventory cycle state machine for economic analysis:
//! - Passive Destocking (被动去库存): Recovery dawn, best buying opportunity
//! - Active Restocking (主动补库存): Prosperity peak, economic boom
//! - Passive Restocking (被动补库存): Recession warning, need caution
//! - Active Destocking (主动去库存): Winter hibernation, wait for bottom

use serde::{Deserialize, Serialize};

/// Inventory cycle phase (Kitchin cycle - 3-4 years)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryCyclePhase {
    /// 被动去库存 (Passive Destocking)
    /// - Demand recovering, inventory depleting
    /// - Sales ↑, Inventory ↓
    /// - Best buying opportunity, recovery dawn
    PassiveDestocking,

    /// 主动补库存 (Active Restocking)
    /// - Demand strong, actively building inventory
    /// - Sales ↑, Inventory ↑
    /// - Economic boom, prosperity peak
    ActiveRestocking,

    /// 被动补库存 (Passive Restocking)
    /// - Demand slowing, inventory accumulating
    /// - Sales ↓, Inventory ↑
    /// - Recession warning, need caution
    PassiveRestocking,

    /// 主动去库存 (Active Destocking)
    /// - Demand weak, actively reducing inventory
    /// - Sales ↓, Inventory ↓
    /// - Winter hibernation, wait for bottom
    ActiveDestocking,
}

impl InventoryCyclePhase {
    /// Get the investment implication for this phase
    pub fn investment_implication(&self) -> &'static str {
        match self {
            InventoryCyclePhase::PassiveDestocking => "🌅 复苏黎明，最佳买点",
            InventoryCyclePhase::ActiveRestocking => "☀️ 繁荣盛夏，景气高峰",
            InventoryCyclePhase::PassiveRestocking => "🌇 衰退黄昏，需警惕",
            InventoryCyclePhase::ActiveDestocking => "❄️ 寒冬蛰伏，等待筑底",
        }
    }

    /// Get the English description
    pub fn english_name(&self) -> &'static str {
        match self {
            InventoryCyclePhase::PassiveDestocking => "Passive Destocking",
            InventoryCyclePhase::ActiveRestocking => "Active Restocking",
            InventoryCyclePhase::PassiveRestocking => "Passive Restocking",
            InventoryCyclePhase::ActiveDestocking => "Active Destocking",
        }
    }

    /// Get recommended position multiplier for this phase
    pub fn position_multiplier(&self) -> f64 {
        match self {
            InventoryCyclePhase::PassiveDestocking => 1.2, // Recovery - increase position
            InventoryCyclePhase::ActiveRestocking => 1.0,  // Peak - maintain
            InventoryCyclePhase::PassiveRestocking => 0.7, // Warning - reduce
            InventoryCyclePhase::ActiveDestocking => 0.5,  // Bottom - defensive
        }
    }
}

impl std::fmt::Display for InventoryCyclePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cn = match self {
            InventoryCyclePhase::PassiveDestocking => "被动去库存",
            InventoryCyclePhase::ActiveRestocking => "主动补库存",
            InventoryCyclePhase::PassiveRestocking => "被动补库存",
            InventoryCyclePhase::ActiveDestocking => "主动去库存",
        };
        write!(f, "{}", cn)
    }
}

/// Input data for inventory cycle determination
#[derive(Debug, Clone, Default)]
pub struct InventoryCycleInput {
    /// Finished goods inventory YoY growth (产成品存货同比)
    pub inventory_yoy: Option<f64>,
    /// Finished goods inventory MoM change (环比变化)
    pub inventory_mom: Option<f64>,
    /// PMI new orders index
    pub pmi_new_orders: Option<f64>,
    /// PMI production index
    pub pmi_production: Option<f64>,
    /// PMI raw material inventory
    pub pmi_raw_material_inventory: Option<f64>,
    /// Industrial value added YoY
    pub industrial_va_yoy: Option<f64>,
    /// PPI YoY
    pub ppi_yoy: Option<f64>,
    /// PPI MoM
    pub ppi_mom: Option<f64>,
    /// Industrial profit YoY growth
    pub industrial_profit_yoy: Option<f64>,
}

/// Result of inventory cycle analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCycleResult {
    /// Determined phase
    pub phase: InventoryCyclePhase,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Supporting signals
    pub signals: Vec<String>,
    /// Contradicting signals
    pub contradictions: Vec<String>,
    /// Position multiplier recommendation
    pub position_multiplier: f64,
}

/// Analyzer for inventory cycle determination
pub struct InventoryCycleAnalyzer;

impl InventoryCycleAnalyzer {
    /// Determine the current inventory cycle phase based on input data
    pub fn analyze(input: &InventoryCycleInput) -> InventoryCycleResult {
        let mut signals: Vec<String> = Vec::new();
        let mut contradictions: Vec<String> = Vec::new();

        // Calculate scores for each phase
        let mut passive_destock_score = 0.0;
        let mut active_restock_score = 0.0;
        let mut passive_restock_score = 0.0;
        let mut active_destock_score = 0.0;

        // Inventory YoY signal
        if let Some(inv_yoy) = input.inventory_yoy {
            if inv_yoy < 0.0 {
                // Inventory declining
                passive_destock_score += 2.0;
                active_destock_score += 2.0;
                signals.push(format!("库存同比下降 {:.1}%", inv_yoy));
            } else if inv_yoy > 5.0 {
                // Inventory rising fast
                active_restock_score += 2.0;
                passive_restock_score += 2.0;
                signals.push(format!("库存同比上升 {:.1}%", inv_yoy));
            }
        }

        // PMI new orders vs production
        if let (Some(orders), Some(prod)) = (input.pmi_new_orders, input.pmi_production) {
            if orders > prod {
                // Demand > Supply -> destocking
                passive_destock_score += 3.0;
                signals.push(format!("PMI新订单 {:.1} > 生产 {:.1}，需求领先", orders, prod));
            } else if orders < prod && orders < 50.0 {
                // Supply > Demand, weak orders -> active destocking
                active_destock_score += 3.0;
                signals.push(format!("PMI新订单 {:.1} < 生产 {:.1}，产能过剩", orders, prod));
            }

            // Expansion vs contraction
            if orders > 50.0 {
                passive_destock_score += 1.0;
                active_restock_score += 1.0;
                signals.push(format!("PMI新订单 {:.1} > 50，需求扩张", orders));
            } else {
                passive_restock_score += 1.0;
                active_destock_score += 1.0;
                signals.push(format!("PMI新订单 {:.1} < 50，需求收缩", orders));
            }
        }

        // PPI signal
        if let Some(ppi_yoy) = input.ppi_yoy {
            if ppi_yoy > 2.0 {
                active_restock_score += 2.0;
                signals.push(format!("PPI同比 {:.1}%，价格上行", ppi_yoy));
            } else if ppi_yoy < -2.0 {
                active_destock_score += 2.0;
                signals.push(format!("PPI同比 {:.1}%，价格下行", ppi_yoy));
            }
        }

        // PPI MoM for turning point detection
        if let Some(ppi_mom) = input.ppi_mom {
            if ppi_mom > 0.0 && input.ppi_yoy.map(|y| y < 0.0).unwrap_or(false) {
                // MoM positive but YoY negative -> potential turning point
                passive_destock_score += 2.0;
                signals.push("PPI环比转正，同比仍负 → 潜在拐点".to_string());
            }
        }

        // Industrial profit signal
        if let Some(profit_yoy) = input.industrial_profit_yoy {
            if profit_yoy > 10.0 {
                active_restock_score += 1.5;
                signals.push(format!("工业利润同比 {:.1}%，盈利改善", profit_yoy));
            } else if profit_yoy < -10.0 {
                active_destock_score += 1.5;
                signals.push(format!("工业利润同比 {:.1}%，盈利恶化", profit_yoy));
            }
        }

        // Determine phase by highest score
        let scores = [
            (InventoryCyclePhase::PassiveDestocking, passive_destock_score),
            (InventoryCyclePhase::ActiveRestocking, active_restock_score),
            (InventoryCyclePhase::PassiveRestocking, passive_restock_score),
            (InventoryCyclePhase::ActiveDestocking, active_destock_score),
        ];

        let total_score: f64 = scores.iter().map(|(_, s)| s).sum();
        let (phase, max_score) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, s)| (*p, *s))
            .unwrap_or((InventoryCyclePhase::PassiveDestocking, 0.0));

        // Calculate confidence based on score distribution
        let confidence = if total_score > 0.0 {
            (max_score / total_score).clamp(0.3, 0.95)
        } else {
            0.3
        };

        // Identify contradictions (other phases with significant scores)
        for (other_phase, score) in &scores {
            if *other_phase != phase && *score > max_score * 0.5 {
                contradictions.push(format!(
                    "{} 信号也较强 (得分: {:.1})",
                    other_phase, score
                ));
            }
        }

        InventoryCycleResult {
            phase,
            confidence,
            signals,
            contradictions,
            position_multiplier: phase.position_multiplier(),
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
    fn test_passive_destocking_detection() {
        let input = InventoryCycleInput {
            inventory_yoy: Some(-2.0),           // Inventory falling
            pmi_new_orders: Some(51.0),          // Demand recovering
            pmi_production: Some(49.0),          // Production lagging
            ppi_mom: Some(0.2),                  // Prices turning
            ppi_yoy: Some(-1.0),                 // Still negative YoY
            ..Default::default()
        };

        let result = InventoryCycleAnalyzer::analyze(&input);
        assert_eq!(result.phase, InventoryCyclePhase::PassiveDestocking);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_active_restocking_detection() {
        let input = InventoryCycleInput {
            inventory_yoy: Some(8.0),            // Inventory rising
            pmi_new_orders: Some(53.0),          // Strong demand
            pmi_production: Some(52.0),          // Strong production
            ppi_yoy: Some(5.0),                  // Prices rising
            industrial_profit_yoy: Some(15.0),   // Profits improving
            ..Default::default()
        };

        let result = InventoryCycleAnalyzer::analyze(&input);
        assert_eq!(result.phase, InventoryCyclePhase::ActiveRestocking);
    }

    #[test]
    fn test_active_destocking_detection() {
        let input = InventoryCycleInput {
            inventory_yoy: Some(-5.0),           // Inventory falling fast
            pmi_new_orders: Some(47.0),          // Weak demand
            pmi_production: Some(48.0),          // Weak production
            ppi_yoy: Some(-4.0),                 // Prices falling
            industrial_profit_yoy: Some(-15.0),  // Profits declining
            ..Default::default()
        };

        let result = InventoryCycleAnalyzer::analyze(&input);
        assert_eq!(result.phase, InventoryCyclePhase::ActiveDestocking);
    }

    #[test]
    fn test_phase_display() {
        assert_eq!(
            InventoryCyclePhase::PassiveDestocking.to_string(),
            "被动去库存"
        );
        assert_eq!(
            InventoryCyclePhase::ActiveRestocking.to_string(),
            "主动补库存"
        );
    }

    #[test]
    fn test_position_multipliers() {
        assert!(InventoryCyclePhase::PassiveDestocking.position_multiplier() > 1.0);
        assert!(InventoryCyclePhase::ActiveDestocking.position_multiplier() < 1.0);
    }
}
