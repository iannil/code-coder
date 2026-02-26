//! Portfolio Types and Structures.
//!
//! Defines the three-tier portfolio structure and related types for
//! the "Trim Weak, Nurture Strong" (斩弱养强) investment discipline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Pool Types
// ============================================================================

/// Portfolio pool tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PoolTier {
    /// Core holdings (核心组合): 60-80% of portfolio, long-term hold unless red light
    Core,
    /// Satellite holdings (卫星组合): 20-40% of portfolio, flexible management
    Satellite,
}

impl std::fmt::Display for PoolTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core => write!(f, "核心组合"),
            Self::Satellite => write!(f, "卫星组合"),
        }
    }
}

/// Watch list item (观察名单).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchItem {
    /// Stock symbol
    pub symbol: String,
    /// Company name
    pub name: String,
    /// Why it's being watched
    pub watch_reason: String,
    /// Target entry price
    pub target_entry_price: Option<f64>,
    /// Target pool if entered
    pub target_pool: PoolTier,
    /// Investment thesis
    pub investment_thesis: String,
    /// When added to watchlist
    pub added_at: DateTime<Utc>,
    /// Last review date
    pub last_reviewed: DateTime<Utc>,
}

// ============================================================================
// Position Types
// ============================================================================

/// Stop loss trigger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossTriggers {
    /// Absolute loss percentage trigger (e.g., -20%)
    pub absolute_loss_pct: Option<f64>,
    /// Technical stop (e.g., below 200-day MA)
    pub below_ma_days: Option<u32>,
    /// Time stop: months without positive return
    pub time_stop_months: Option<u32>,
    /// Trailing stop percentage from high
    pub trailing_stop_pct: Option<f64>,
}

impl Default for StopLossTriggers {
    fn default() -> Self {
        Self {
            absolute_loss_pct: Some(-25.0),
            below_ma_days: Some(200),
            time_stop_months: Some(24),
            trailing_stop_pct: Some(-15.0),
        }
    }
}

impl StopLossTriggers {
    /// Core position defaults (wider stops for long-term holds)
    pub fn core_defaults() -> Self {
        Self {
            absolute_loss_pct: Some(-30.0),
            below_ma_days: Some(250),
            time_stop_months: Some(36),
            trailing_stop_pct: Some(-20.0),
        }
    }

    /// Satellite position defaults (tighter stops for tactical positions)
    pub fn satellite_defaults() -> Self {
        Self {
            absolute_loss_pct: Some(-20.0),
            below_ma_days: Some(120),
            time_stop_months: Some(12),
            trailing_stop_pct: Some(-12.0),
        }
    }
}

/// A portfolio position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Stock symbol
    pub symbol: String,
    /// Company name
    pub name: String,
    /// Pool tier
    pub tier: PoolTier,
    /// Entry price per share
    pub entry_price: f64,
    /// Current price per share
    pub current_price: f64,
    /// Number of shares
    pub quantity: f64,
    /// Portfolio weight (0-100%)
    pub weight: f64,
    /// Entry date
    pub entry_date: DateTime<Utc>,
    /// Investment thesis
    pub investment_thesis: String,
    /// Key metrics to monitor
    pub key_metrics: Vec<String>,
    /// Stop loss triggers
    pub stop_loss_triggers: StopLossTriggers,
    /// Highest price since entry (for trailing stop)
    pub high_since_entry: f64,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Position {
    /// Calculate current market value.
    pub fn market_value(&self) -> f64 {
        self.current_price * self.quantity
    }

    /// Calculate cost basis.
    pub fn cost_basis(&self) -> f64 {
        self.entry_price * self.quantity
    }

    /// Calculate unrealized gain/loss amount.
    pub fn unrealized_pnl(&self) -> f64 {
        self.market_value() - self.cost_basis()
    }

    /// Calculate unrealized gain/loss percentage.
    pub fn unrealized_pnl_pct(&self) -> f64 {
        if self.cost_basis() > 0.0 {
            (self.unrealized_pnl() / self.cost_basis()) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate drawdown from high.
    pub fn drawdown_from_high(&self) -> f64 {
        if self.high_since_entry > 0.0 {
            ((self.current_price - self.high_since_entry) / self.high_since_entry) * 100.0
        } else {
            0.0
        }
    }

    /// Check if any stop loss trigger is hit.
    pub fn is_stop_triggered(&self) -> bool {
        let triggers = &self.stop_loss_triggers;

        // Check absolute loss
        if let Some(threshold) = triggers.absolute_loss_pct {
            if self.unrealized_pnl_pct() <= threshold {
                return true;
            }
        }

        // Check trailing stop
        if let Some(threshold) = triggers.trailing_stop_pct {
            if self.drawdown_from_high() <= threshold {
                return true;
            }
        }

        // Note: MA and time stops require external data not available in Position
        false
    }

    /// Update current price and high watermark.
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        if price > self.high_since_entry {
            self.high_since_entry = price;
        }
        self.updated_at = Utc::now();
    }
}

// ============================================================================
// Portfolio Pools Structure
// ============================================================================

/// Three-tier portfolio pools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPools {
    /// Core holdings (60-80% target allocation)
    pub core: Vec<Position>,
    /// Satellite holdings (20-40% target allocation)
    pub satellite: Vec<Position>,
    /// Watch list (0% allocation, candidates)
    pub watchlist: Vec<WatchItem>,
    /// Target core allocation percentage
    pub target_core_pct: f64,
    /// Target satellite allocation percentage
    pub target_satellite_pct: f64,
    /// Total portfolio value
    pub total_value: f64,
    /// Cash position
    pub cash: f64,
    /// Last rebalance date
    pub last_rebalance: Option<DateTime<Utc>>,
}

impl Default for PortfolioPools {
    fn default() -> Self {
        Self {
            core: Vec::new(),
            satellite: Vec::new(),
            watchlist: Vec::new(),
            target_core_pct: 70.0,
            target_satellite_pct: 30.0,
            total_value: 0.0,
            cash: 0.0,
            last_rebalance: None,
        }
    }
}

impl PortfolioPools {
    /// Create new portfolio pools with target allocations.
    pub fn new(target_core_pct: f64, target_satellite_pct: f64) -> Self {
        Self {
            target_core_pct,
            target_satellite_pct,
            ..Default::default()
        }
    }

    /// Calculate current core allocation percentage.
    pub fn core_allocation_pct(&self) -> f64 {
        if self.total_value > 0.0 {
            let core_value: f64 = self.core.iter().map(|p| p.market_value()).sum();
            (core_value / self.total_value) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate current satellite allocation percentage.
    pub fn satellite_allocation_pct(&self) -> f64 {
        if self.total_value > 0.0 {
            let satellite_value: f64 = self.satellite.iter().map(|p| p.market_value()).sum();
            (satellite_value / self.total_value) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate cash allocation percentage.
    pub fn cash_allocation_pct(&self) -> f64 {
        if self.total_value > 0.0 {
            (self.cash / self.total_value) * 100.0
        } else {
            100.0
        }
    }

    /// Check if core allocation is within target range.
    pub fn is_core_balanced(&self) -> bool {
        let current = self.core_allocation_pct();
        let tolerance = 10.0; // +/- 10%
        (current - self.target_core_pct).abs() <= tolerance
    }

    /// Get all positions.
    pub fn all_positions(&self) -> Vec<&Position> {
        self.core
            .iter()
            .chain(self.satellite.iter())
            .collect()
    }

    /// Get positions sorted by performance (best to worst).
    pub fn positions_by_performance(&self) -> Vec<&Position> {
        let mut positions = self.all_positions();
        positions.sort_by(|a, b| {
            b.unrealized_pnl_pct()
                .partial_cmp(&a.unrealized_pnl_pct())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        positions
    }

    /// Get positions that have stop triggers hit.
    pub fn positions_with_stops_triggered(&self) -> Vec<&Position> {
        self.all_positions()
            .into_iter()
            .filter(|p| p.is_stop_triggered())
            .collect()
    }

    /// Add position to appropriate pool.
    pub fn add_position(&mut self, position: Position) {
        match position.tier {
            PoolTier::Core => self.core.push(position),
            PoolTier::Satellite => self.satellite.push(position),
        }
        self.recalculate_total();
    }

    /// Remove position by symbol.
    pub fn remove_position(&mut self, symbol: &str) -> Option<Position> {
        // Try core first
        if let Some(idx) = self.core.iter().position(|p| p.symbol == symbol) {
            let position = self.core.remove(idx);
            self.recalculate_total();
            return Some(position);
        }

        // Try satellite
        if let Some(idx) = self.satellite.iter().position(|p| p.symbol == symbol) {
            let position = self.satellite.remove(idx);
            self.recalculate_total();
            return Some(position);
        }

        None
    }

    /// Move position between tiers.
    pub fn move_position(&mut self, symbol: &str, new_tier: PoolTier) -> bool {
        if let Some(mut position) = self.remove_position(symbol) {
            position.tier = new_tier;
            position.stop_loss_triggers = match new_tier {
                PoolTier::Core => StopLossTriggers::core_defaults(),
                PoolTier::Satellite => StopLossTriggers::satellite_defaults(),
            };
            self.add_position(position);
            true
        } else {
            false
        }
    }

    /// Recalculate total portfolio value.
    fn recalculate_total(&mut self) {
        let core_value: f64 = self.core.iter().map(|p| p.market_value()).sum();
        let satellite_value: f64 = self.satellite.iter().map(|p| p.market_value()).sum();
        self.total_value = core_value + satellite_value + self.cash;
    }

    /// Update all position weights based on current total value.
    pub fn recalculate_weights(&mut self) {
        self.recalculate_total();

        if self.total_value > 0.0 {
            for position in self.core.iter_mut() {
                position.weight = (position.market_value() / self.total_value) * 100.0;
            }
            for position in self.satellite.iter_mut() {
                position.weight = (position.market_value() / self.total_value) * 100.0;
            }
        }
    }

    /// Get portfolio summary statistics.
    pub fn summary(&self) -> PortfolioSummary {
        let total_cost: f64 = self.all_positions().iter().map(|p| p.cost_basis()).sum();
        let total_market: f64 = self.all_positions().iter().map(|p| p.market_value()).sum();

        let total_pnl = total_market - total_cost;
        let total_pnl_pct = if total_cost > 0.0 {
            (total_pnl / total_cost) * 100.0
        } else {
            0.0
        };

        let winners = self
            .all_positions()
            .iter()
            .filter(|p| p.unrealized_pnl_pct() > 0.0)
            .count();
        let losers = self
            .all_positions()
            .iter()
            .filter(|p| p.unrealized_pnl_pct() < 0.0)
            .count();

        PortfolioSummary {
            total_positions: self.core.len() + self.satellite.len(),
            core_positions: self.core.len(),
            satellite_positions: self.satellite.len(),
            watchlist_count: self.watchlist.len(),
            total_value: self.total_value,
            total_cost,
            total_pnl,
            total_pnl_pct,
            winners,
            losers,
            core_allocation_pct: self.core_allocation_pct(),
            satellite_allocation_pct: self.satellite_allocation_pct(),
            cash_allocation_pct: self.cash_allocation_pct(),
        }
    }
}

/// Portfolio summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_positions: usize,
    pub core_positions: usize,
    pub satellite_positions: usize,
    pub watchlist_count: usize,
    pub total_value: f64,
    pub total_cost: f64,
    pub total_pnl: f64,
    pub total_pnl_pct: f64,
    pub winners: usize,
    pub losers: usize,
    pub core_allocation_pct: f64,
    pub satellite_allocation_pct: f64,
    pub cash_allocation_pct: f64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_position(symbol: &str, tier: PoolTier, entry: f64, current: f64, qty: f64) -> Position {
        Position {
            symbol: symbol.to_string(),
            name: format!("Test {}", symbol),
            tier,
            entry_price: entry,
            current_price: current,
            quantity: qty,
            weight: 0.0,
            entry_date: Utc::now(),
            investment_thesis: "Test position".to_string(),
            key_metrics: vec!["ROE".to_string()],
            stop_loss_triggers: StopLossTriggers::default(),
            high_since_entry: current.max(entry),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_position_calculations() {
        let position = make_test_position("TEST", PoolTier::Core, 100.0, 120.0, 10.0);

        assert!((position.market_value() - 1200.0).abs() < 0.01);
        assert!((position.cost_basis() - 1000.0).abs() < 0.01);
        assert!((position.unrealized_pnl() - 200.0).abs() < 0.01);
        assert!((position.unrealized_pnl_pct() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_position_stop_trigger() {
        let mut position = make_test_position("TEST", PoolTier::Core, 100.0, 70.0, 10.0);
        position.stop_loss_triggers.absolute_loss_pct = Some(-25.0);
        position.stop_loss_triggers.trailing_stop_pct = None; // Disable trailing stop

        // -30% loss should trigger stop
        assert!(position.is_stop_triggered());

        // -20% loss should not trigger -25% stop
        position.current_price = 80.0;
        assert!(!position.is_stop_triggered());
    }

    #[test]
    fn test_portfolio_pools_allocation() {
        let mut pools = PortfolioPools::new(70.0, 30.0);
        pools.cash = 10000.0;

        // Add core position
        pools.add_position(make_test_position("CORE1", PoolTier::Core, 100.0, 100.0, 70.0));

        // Add satellite position
        pools.add_position(make_test_position("SAT1", PoolTier::Satellite, 100.0, 100.0, 20.0));

        pools.recalculate_weights();

        // Total: 7000 + 2000 + 10000 = 19000
        assert!((pools.total_value - 19000.0).abs() < 0.01);

        // Core: 7000/19000 = 36.8%
        assert!((pools.core_allocation_pct() - 36.84).abs() < 1.0);
    }

    #[test]
    fn test_move_position() {
        let mut pools = PortfolioPools::default();
        pools.add_position(make_test_position("TEST", PoolTier::Satellite, 100.0, 120.0, 10.0));

        assert_eq!(pools.satellite.len(), 1);
        assert_eq!(pools.core.len(), 0);

        pools.move_position("TEST", PoolTier::Core);

        assert_eq!(pools.satellite.len(), 0);
        assert_eq!(pools.core.len(), 1);
        assert_eq!(pools.core[0].tier, PoolTier::Core);
    }

    #[test]
    fn test_positions_by_performance() {
        let mut pools = PortfolioPools::default();
        pools.add_position(make_test_position("A", PoolTier::Core, 100.0, 90.0, 10.0)); // -10%
        pools.add_position(make_test_position("B", PoolTier::Core, 100.0, 130.0, 10.0)); // +30%
        pools.add_position(make_test_position("C", PoolTier::Satellite, 100.0, 110.0, 10.0)); // +10%

        let sorted = pools.positions_by_performance();

        assert_eq!(sorted[0].symbol, "B"); // Best performer
        assert_eq!(sorted[1].symbol, "C");
        assert_eq!(sorted[2].symbol, "A"); // Worst performer
    }

    #[test]
    fn test_portfolio_summary() {
        let mut pools = PortfolioPools::default();
        pools.cash = 5000.0;
        pools.add_position(make_test_position("WINNER", PoolTier::Core, 100.0, 120.0, 10.0));
        pools.add_position(make_test_position("LOSER", PoolTier::Satellite, 100.0, 80.0, 5.0));

        let summary = pools.summary();

        assert_eq!(summary.total_positions, 2);
        assert_eq!(summary.winners, 1);
        assert_eq!(summary.losers, 1);
    }
}
