//! Nurture Strong Capital Allocator.
//!
//! Implements the "斩弱养强" (Trim Weak, Nurture Strong) capital allocation strategy.
//!
//! # Philosophy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     养强资金分配优先级                                │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  第一优先级：加仓核心组合最强标的                                     │
//! │    └─ 强者恒强，核心持仓表现最好的优先获得增量资金                     │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  第二优先级：增持卫星组合兑现中的机会                                 │
//! │    └─ 卫星标的接近目标价时可适度加仓，确保收益落袋                    │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  第三优先级：从观察名单提拔新秀                                       │
//! │    └─ 若核心和卫星都已充分配置，则考虑新标的                          │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::portfolio::{CapitalAllocator, AllocationPriority};
//!
//! let allocator = CapitalAllocator::new(config);
//!
//! // Get allocation recommendations for available capital
//! let recommendations = allocator.recommend_allocation(&portfolio, 50_000.0).await?;
//!
//! // Execute allocations
//! for rec in recommendations.orders {
//!     execute_trade(rec)?;
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{PoolTier, PortfolioPools, Position};

// ============================================================================
// Allocation Priority
// ============================================================================

/// Priority level for capital allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllocationPriority {
    /// Highest priority: strengthen core positions
    CoreStrength,
    /// Second priority: capture satellite gains
    SatelliteCapture,
    /// Third priority: promote from watchlist
    WatchlistPromotion,
    /// No allocation recommended
    HoldCash,
}

impl std::fmt::Display for AllocationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoreStrength => write!(f, "加仓核心组合"),
            Self::SatelliteCapture => write!(f, "增持卫星机会"),
            Self::WatchlistPromotion => write!(f, "提拔观察名单"),
            Self::HoldCash => write!(f, "持有现金"),
        }
    }
}

// ============================================================================
// Trim Candidate
// ============================================================================

/// A position identified for trimming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrimCandidate {
    /// Symbol to trim
    pub symbol: String,
    /// Current weight in portfolio
    pub current_weight: f64,
    /// Current P&L percentage
    pub pnl_pct: f64,
    /// Suggested trim percentage (of position)
    pub trim_pct: f64,
    /// Estimated proceeds from trim
    pub estimated_proceeds: f64,
    /// Reason for trim recommendation
    pub reason: TrimReason,
}

/// Reason for trimming a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrimReason {
    /// Underperforming relative to peers
    Underperformance,
    /// Position overweight
    Overweight,
    /// Near or above target price
    TargetReached,
    /// Stop loss triggered
    StopTriggered,
    /// Rebalancing requirement
    Rebalance,
}

impl std::fmt::Display for TrimReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Underperformance => write!(f, "表现落后"),
            Self::Overweight => write!(f, "仓位超配"),
            Self::TargetReached => write!(f, "接近目标价"),
            Self::StopTriggered => write!(f, "触发止损"),
            Self::Rebalance => write!(f, "再平衡需要"),
        }
    }
}

// ============================================================================
// Add Candidate
// ============================================================================

/// A position identified for adding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCandidate {
    /// Symbol to add
    pub symbol: String,
    /// Pool tier
    pub tier: PoolTier,
    /// Current weight in portfolio
    pub current_weight: f64,
    /// Target weight after addition
    pub target_weight: f64,
    /// Suggested addition amount
    pub suggested_amount: f64,
    /// Allocation priority
    pub priority: AllocationPriority,
    /// Reason for addition recommendation
    pub reason: String,
    /// Strength score (0-100)
    pub strength_score: f64,
}

// ============================================================================
// Allocation Recommendation
// ============================================================================

/// A single allocation order recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationOrder {
    /// Symbol
    pub symbol: String,
    /// Order side (Buy/Sell)
    pub side: AllocationSide,
    /// Suggested amount (currency)
    pub amount: f64,
    /// Quantity to trade (shares)
    pub quantity: Option<f64>,
    /// Priority level
    pub priority: AllocationPriority,
    /// Reason for the order
    pub reason: String,
}

/// Side of allocation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AllocationSide {
    /// Buy / Add to position
    Add,
    /// Sell / Trim position
    Trim,
}

impl std::fmt::Display for AllocationSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "加仓"),
            Self::Trim => write!(f, "减仓"),
        }
    }
}

/// Complete allocation recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationRecommendation {
    /// Capital available for allocation
    pub available_capital: f64,
    /// Capital from trimming weak positions
    pub capital_from_trims: f64,
    /// Total capital to allocate
    pub total_to_allocate: f64,
    /// Positions to trim
    pub trim_candidates: Vec<TrimCandidate>,
    /// Positions to add
    pub add_candidates: Vec<AddCandidate>,
    /// Generated orders
    pub orders: Vec<AllocationOrder>,
    /// Overall allocation strategy
    pub strategy_summary: String,
    /// Timestamp
    pub generated_at: DateTime<Utc>,
}

// ============================================================================
// Allocator Configuration
// ============================================================================

/// Configuration for capital allocator.
#[derive(Debug, Clone)]
pub struct AllocatorConfig {
    /// Maximum weight for any single position (%)
    pub max_position_weight: f64,
    /// Minimum weight below which positions should be scaled up or exited (%)
    pub min_meaningful_weight: f64,
    /// Underperformance threshold for trim consideration (%)
    pub underperformance_threshold: f64,
    /// Maximum portion of available capital for single allocation (%)
    pub max_single_allocation_pct: f64,
    /// Minimum allocation size (currency)
    pub min_allocation_size: f64,
    /// Target cash reserve percentage
    pub target_cash_reserve_pct: f64,
    /// Enable watchlist promotions
    pub enable_watchlist_promotions: bool,
}

impl Default for AllocatorConfig {
    fn default() -> Self {
        Self {
            max_position_weight: 15.0,
            min_meaningful_weight: 2.0,
            underperformance_threshold: -15.0,
            max_single_allocation_pct: 30.0,
            min_allocation_size: 1000.0,
            target_cash_reserve_pct: 5.0,
            enable_watchlist_promotions: true,
        }
    }
}

// ============================================================================
// Capital Allocator
// ============================================================================

/// Capital allocator implementing the nurture strong strategy.
pub struct CapitalAllocator {
    config: AllocatorConfig,
}

impl CapitalAllocator {
    /// Create a new allocator with default config.
    pub fn new() -> Self {
        Self {
            config: AllocatorConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: AllocatorConfig) -> Self {
        Self { config }
    }

    /// Generate allocation recommendations.
    ///
    /// Takes the current portfolio state and available new capital,
    /// returns a prioritized list of allocation recommendations.
    pub fn recommend_allocation(
        &self,
        portfolio: &PortfolioPools,
        new_capital: f64,
    ) -> AllocationRecommendation {
        // Step 1: Identify trim candidates
        let trim_candidates = self.identify_trim_candidates(portfolio);
        let capital_from_trims: f64 = trim_candidates.iter().map(|t| t.estimated_proceeds).sum();

        // Step 2: Calculate total available capital
        let total_available = new_capital + capital_from_trims;

        // Reserve some cash
        let cash_reserve = total_available * (self.config.target_cash_reserve_pct / 100.0);
        let capital_to_allocate = (total_available - cash_reserve).max(0.0);

        // Step 3: Identify add candidates in priority order
        let add_candidates = self.identify_add_candidates(portfolio, capital_to_allocate);

        // Step 4: Generate orders
        let mut orders = Vec::new();

        // Add trim orders
        for trim in &trim_candidates {
            orders.push(AllocationOrder {
                symbol: trim.symbol.clone(),
                side: AllocationSide::Trim,
                amount: trim.estimated_proceeds,
                quantity: None, // Will be calculated based on trim_pct
                priority: AllocationPriority::HoldCash, // Trims free up capital
                reason: format!("{}", trim.reason),
            });
        }

        // Add buy orders in priority order
        let mut remaining = capital_to_allocate;
        for candidate in &add_candidates {
            if remaining < self.config.min_allocation_size {
                break;
            }

            let amount = candidate.suggested_amount.min(remaining);
            if amount >= self.config.min_allocation_size {
                orders.push(AllocationOrder {
                    symbol: candidate.symbol.clone(),
                    side: AllocationSide::Add,
                    amount,
                    quantity: None,
                    priority: candidate.priority,
                    reason: candidate.reason.clone(),
                });
                remaining -= amount;
            }
        }

        // Step 5: Generate summary
        let strategy_summary = self.generate_strategy_summary(
            &trim_candidates,
            &add_candidates,
            capital_to_allocate,
        );

        AllocationRecommendation {
            available_capital: new_capital,
            capital_from_trims,
            total_to_allocate: capital_to_allocate,
            trim_candidates,
            add_candidates,
            orders,
            strategy_summary,
            generated_at: Utc::now(),
        }
    }

    /// Identify positions that should be trimmed.
    fn identify_trim_candidates(&self, portfolio: &PortfolioPools) -> Vec<TrimCandidate> {
        let mut candidates = Vec::new();

        let all_positions = portfolio.all_positions();

        for position in all_positions {
            let pnl_pct = position.unrealized_pnl_pct();

            // Check for underperformance
            if pnl_pct < self.config.underperformance_threshold {
                candidates.push(TrimCandidate {
                    symbol: position.symbol.clone(),
                    current_weight: position.weight,
                    pnl_pct,
                    trim_pct: 50.0, // Trim half for underperformers
                    estimated_proceeds: position.market_value() * 0.5,
                    reason: TrimReason::Underperformance,
                });
                continue;
            }

            // Check for overweight
            if position.weight > self.config.max_position_weight {
                let excess_weight = position.weight - self.config.max_position_weight;
                let trim_pct = (excess_weight / position.weight) * 100.0;
                candidates.push(TrimCandidate {
                    symbol: position.symbol.clone(),
                    current_weight: position.weight,
                    pnl_pct,
                    trim_pct,
                    estimated_proceeds: position.market_value() * (trim_pct / 100.0),
                    reason: TrimReason::Overweight,
                });
                continue;
            }

            // Check for stop loss triggers
            if position.is_stop_triggered() {
                // Full exit for stop triggers
                candidates.push(TrimCandidate {
                    symbol: position.symbol.clone(),
                    current_weight: position.weight,
                    pnl_pct,
                    trim_pct: 100.0,
                    estimated_proceeds: position.market_value(),
                    reason: TrimReason::StopTriggered,
                });
            }
        }

        // Sort by urgency: stops first, then underperformers, then overweight
        candidates.sort_by(|a, b| {
            let order = |r: &TrimReason| match r {
                TrimReason::StopTriggered => 0,
                TrimReason::Underperformance => 1,
                TrimReason::Overweight => 2,
                TrimReason::TargetReached => 3,
                TrimReason::Rebalance => 4,
            };
            order(&a.reason).cmp(&order(&b.reason))
        });

        candidates
    }

    /// Identify positions to add capital to.
    fn identify_add_candidates(
        &self,
        portfolio: &PortfolioPools,
        available_capital: f64,
    ) -> Vec<AddCandidate> {
        let mut candidates = Vec::new();

        // Priority 1: Strongest core performers
        for position in &portfolio.core {
            if position.unrealized_pnl_pct() > 0.0 && position.weight < self.config.max_position_weight {
                let room_for_addition = self.config.max_position_weight - position.weight;
                let max_amount = portfolio.total_value * (room_for_addition / 100.0);
                let suggested = max_amount.min(available_capital * 0.25); // Max 25% of capital per position

                if suggested >= self.config.min_allocation_size {
                    candidates.push(AddCandidate {
                        symbol: position.symbol.clone(),
                        tier: PoolTier::Core,
                        current_weight: position.weight,
                        target_weight: position.weight + (suggested / portfolio.total_value) * 100.0,
                        suggested_amount: suggested,
                        priority: AllocationPriority::CoreStrength,
                        reason: format!(
                            "核心持仓表现强劲 (收益 {:.1}%)，继续加码",
                            position.unrealized_pnl_pct()
                        ),
                        strength_score: self.calculate_strength_score(position),
                    });
                }
            }
        }

        // Priority 2: Satellite positions showing momentum
        for position in &portfolio.satellite {
            // Only add to satellites with positive momentum
            if position.unrealized_pnl_pct() > 5.0 && position.weight < self.config.max_position_weight * 0.8 {
                let room_for_addition =
                    (self.config.max_position_weight * 0.8) - position.weight;
                let max_amount = portfolio.total_value * (room_for_addition / 100.0);
                let suggested = max_amount.min(available_capital * 0.15); // Max 15% per satellite

                if suggested >= self.config.min_allocation_size {
                    candidates.push(AddCandidate {
                        symbol: position.symbol.clone(),
                        tier: PoolTier::Satellite,
                        current_weight: position.weight,
                        target_weight: position.weight + (suggested / portfolio.total_value) * 100.0,
                        suggested_amount: suggested,
                        priority: AllocationPriority::SatelliteCapture,
                        reason: format!(
                            "卫星持仓正在兑现 (收益 {:.1}%)，适度增持",
                            position.unrealized_pnl_pct()
                        ),
                        strength_score: self.calculate_strength_score(position),
                    });
                }
            }
        }

        // Priority 3: Watchlist promotions
        if self.config.enable_watchlist_promotions {
            for watch_item in &portfolio.watchlist {
                if let Some(target_price) = watch_item.target_entry_price {
                    let suggested = available_capital * 0.1; // Max 10% for new positions

                    if suggested >= self.config.min_allocation_size {
                        candidates.push(AddCandidate {
                            symbol: watch_item.symbol.clone(),
                            tier: watch_item.target_pool,
                            current_weight: 0.0,
                            target_weight: (suggested / portfolio.total_value) * 100.0,
                            suggested_amount: suggested,
                            priority: AllocationPriority::WatchlistPromotion,
                            reason: format!(
                                "观察名单提拔：目标入场价 {:.2}",
                                target_price
                            ),
                            strength_score: 50.0, // Neutral score for new positions
                        });
                    }
                }
            }
        }

        // Sort by priority, then by strength score
        candidates.sort_by(|a, b| {
            let priority_order = |p: &AllocationPriority| match p {
                AllocationPriority::CoreStrength => 0,
                AllocationPriority::SatelliteCapture => 1,
                AllocationPriority::WatchlistPromotion => 2,
                AllocationPriority::HoldCash => 3,
            };
            priority_order(&a.priority)
                .cmp(&priority_order(&b.priority))
                .then(b.strength_score.partial_cmp(&a.strength_score).unwrap())
        });

        candidates
    }

    /// Calculate strength score for a position (0-100).
    fn calculate_strength_score(&self, position: &Position) -> f64 {
        let mut score: f64 = 50.0; // Base score

        // P&L contribution
        let pnl = position.unrealized_pnl_pct();
        if pnl > 20.0 {
            score += 25.0;
        } else if pnl > 10.0 {
            score += 15.0;
        } else if pnl > 0.0 {
            score += 5.0;
        } else if pnl < -10.0 {
            score -= 15.0;
        }

        // Core positions get bonus
        if position.tier == PoolTier::Core {
            score += 10.0;
        }

        // Weight efficiency (not too small, not too big)
        if position.weight >= 3.0 && position.weight <= 10.0 {
            score += 10.0;
        }

        // Momentum (price vs high)
        let momentum = position.current_price / position.high_since_entry;
        if momentum >= 0.95 {
            score += 5.0;
        }

        score.clamp(0.0, 100.0)
    }

    /// Generate human-readable strategy summary.
    fn generate_strategy_summary(
        &self,
        trim_candidates: &[TrimCandidate],
        add_candidates: &[AddCandidate],
        capital_to_allocate: f64,
    ) -> String {
        let mut parts = Vec::new();

        // Trim summary
        if !trim_candidates.is_empty() {
            let trim_total: f64 = trim_candidates.iter().map(|t| t.estimated_proceeds).sum();
            parts.push(format!(
                "斩弱：{}个标的待减仓，预计释放资金 {:.0}",
                trim_candidates.len(),
                trim_total
            ));
        }

        // Add summary
        let core_adds: Vec<_> = add_candidates
            .iter()
            .filter(|c| c.priority == AllocationPriority::CoreStrength)
            .collect();
        let satellite_adds: Vec<_> = add_candidates
            .iter()
            .filter(|c| c.priority == AllocationPriority::SatelliteCapture)
            .collect();
        let watchlist_adds: Vec<_> = add_candidates
            .iter()
            .filter(|c| c.priority == AllocationPriority::WatchlistPromotion)
            .collect();

        if !core_adds.is_empty() {
            let core_symbols: Vec<_> = core_adds.iter().map(|c| c.symbol.as_str()).collect();
            parts.push(format!(
                "养强(核心)：加仓 {}",
                core_symbols.join("、")
            ));
        }

        if !satellite_adds.is_empty() {
            let sat_symbols: Vec<_> = satellite_adds.iter().map(|c| c.symbol.as_str()).collect();
            parts.push(format!(
                "养强(卫星)：增持 {}",
                sat_symbols.join("、")
            ));
        }

        if !watchlist_adds.is_empty() {
            let watch_symbols: Vec<_> = watchlist_adds.iter().map(|c| c.symbol.as_str()).collect();
            parts.push(format!(
                "新建仓：{}",
                watch_symbols.join("、")
            ));
        }

        if parts.is_empty() {
            parts.push(format!(
                "当前无明确配置建议，资金 {:.0} 暂作保留",
                capital_to_allocate
            ));
        } else {
            parts.push(format!("可配置资金：{:.0}", capital_to_allocate));
        }

        parts.join("；")
    }
}

impl Default for CapitalAllocator {
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
    use crate::portfolio::types::{StopLossTriggers, WatchItem};

    fn make_position(symbol: &str, tier: PoolTier, entry: f64, current: f64, qty: f64) -> Position {
        Position {
            symbol: symbol.to_string(),
            name: format!("{} Inc", symbol),
            tier,
            entry_price: entry,
            current_price: current,
            quantity: qty,
            weight: 0.0, // Will be calculated
            entry_date: Utc::now(),
            investment_thesis: "Test".to_string(),
            key_metrics: vec![],
            stop_loss_triggers: StopLossTriggers::default(),
            high_since_entry: current.max(entry),
            updated_at: Utc::now(),
        }
    }

    fn make_portfolio() -> PortfolioPools {
        let mut pools = PortfolioPools::new(70.0, 30.0);
        pools.cash = 50_000.0;

        // Core positions (smaller quantities to stay under max_position_weight)
        pools.core.push(make_position("STRONG", PoolTier::Core, 100.0, 130.0, 50.0)); // +30%, value=6500
        pools.core.push(make_position("WEAK", PoolTier::Core, 100.0, 80.0, 50.0)); // -20%, value=4000

        // Satellite positions
        pools.satellite.push(make_position("SAT1", PoolTier::Satellite, 50.0, 55.0, 50.0)); // +10%, value=2750
        pools.satellite.push(make_position("SAT2", PoolTier::Satellite, 50.0, 45.0, 50.0)); // -10%, value=2250

        // Add watchlist item
        pools.watchlist.push(WatchItem {
            symbol: "WATCH1".to_string(),
            name: "Watch Company".to_string(),
            watch_reason: "Potential value".to_string(),
            target_entry_price: Some(40.0),
            target_pool: PoolTier::Satellite,
            investment_thesis: "Undervalued".to_string(),
            added_at: Utc::now(),
            last_reviewed: Utc::now(),
        });

        pools.recalculate_weights();
        // Total: 6500+4000+2750+2250+50000 = 65500
        // STRONG weight ~ 10%, SAT1 weight ~ 4%
        pools
    }

    #[test]
    fn test_identify_trim_candidates() {
        let allocator = CapitalAllocator::new();
        let portfolio = make_portfolio();

        let candidates = allocator.identify_trim_candidates(&portfolio);

        // WEAK should be identified as underperformer
        assert!(candidates.iter().any(|c| c.symbol == "WEAK"));
        let weak = candidates.iter().find(|c| c.symbol == "WEAK").unwrap();
        assert_eq!(weak.reason, TrimReason::Underperformance);
    }

    #[test]
    fn test_identify_add_candidates() {
        let allocator = CapitalAllocator::new();
        let portfolio = make_portfolio();

        let candidates = allocator.identify_add_candidates(&portfolio, 20_000.0);

        // STRONG should be first priority
        assert!(!candidates.is_empty());
        let first = &candidates[0];
        assert_eq!(first.symbol, "STRONG");
        assert_eq!(first.priority, AllocationPriority::CoreStrength);
    }

    #[test]
    fn test_full_allocation_recommendation() {
        let allocator = CapitalAllocator::new();
        let portfolio = make_portfolio();

        let recommendation = allocator.recommend_allocation(&portfolio, 10_000.0);

        // Should have trim orders for weak positions
        let trim_orders: Vec<_> = recommendation
            .orders
            .iter()
            .filter(|o| o.side == AllocationSide::Trim)
            .collect();
        assert!(!trim_orders.is_empty());

        // Should have add orders for strong positions
        let add_orders: Vec<_> = recommendation
            .orders
            .iter()
            .filter(|o| o.side == AllocationSide::Add)
            .collect();
        assert!(!add_orders.is_empty());

        // Strategy summary should be non-empty
        assert!(!recommendation.strategy_summary.is_empty());
    }

    #[test]
    fn test_strength_score_calculation() {
        let allocator = CapitalAllocator::new();

        // Strong performer
        let strong = make_position("STRONG", PoolTier::Core, 100.0, 130.0, 100.0);
        let strong_score = allocator.calculate_strength_score(&strong);
        assert!(strong_score > 70.0);

        // Weak performer
        let weak = make_position("WEAK", PoolTier::Satellite, 100.0, 80.0, 100.0);
        let weak_score = allocator.calculate_strength_score(&weak);
        assert!(weak_score < 50.0);
    }

    #[test]
    fn test_allocation_priority_ordering() {
        let allocator = CapitalAllocator::new();
        let portfolio = make_portfolio();

        let candidates = allocator.identify_add_candidates(&portfolio, 50_000.0);

        // Verify priority ordering
        let mut seen_satellite = false;
        let mut seen_watchlist = false;

        for candidate in &candidates {
            match candidate.priority {
                AllocationPriority::CoreStrength => {
                    assert!(!seen_satellite && !seen_watchlist, "Core should come first");
                }
                AllocationPriority::SatelliteCapture => {
                    seen_satellite = true;
                    assert!(!seen_watchlist, "Satellite should come before watchlist");
                }
                AllocationPriority::WatchlistPromotion => {
                    seen_watchlist = true;
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_max_allocation_limits() {
        let config = AllocatorConfig {
            max_single_allocation_pct: 20.0,
            ..Default::default()
        };
        let allocator = CapitalAllocator::with_config(config);
        let portfolio = make_portfolio();

        let recommendation = allocator.recommend_allocation(&portfolio, 100_000.0);

        // No single allocation should exceed max
        for order in &recommendation.orders {
            if order.side == AllocationSide::Add {
                assert!(order.amount <= 100_000.0 * 0.25); // Max 25% per position
            }
        }
    }
}
