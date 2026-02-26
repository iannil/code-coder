//! Portfolio Pools Manager.
//!
//! Manages the three-tier portfolio structure with core, satellite, and watchlist pools.

use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::*;

/// Portfolio pools manager.
pub struct PoolsManager {
    /// Portfolio pools state
    pools: Arc<RwLock<PortfolioPools>>,
    /// Configuration
    config: PoolsConfig,
}

/// Configuration for pools manager.
#[derive(Debug, Clone)]
pub struct PoolsConfig {
    /// Target core allocation percentage
    pub target_core_pct: f64,
    /// Target satellite allocation percentage
    pub target_satellite_pct: f64,
    /// Maximum single position weight (%)
    pub max_position_weight: f64,
    /// Minimum position size to add (currency units)
    pub min_position_size: f64,
    /// Maximum core positions
    pub max_core_positions: usize,
    /// Maximum satellite positions
    pub max_satellite_positions: usize,
    /// Rebalance threshold (% deviation from target)
    pub rebalance_threshold: f64,
}

impl Default for PoolsConfig {
    fn default() -> Self {
        Self {
            target_core_pct: 70.0,
            target_satellite_pct: 30.0,
            max_position_weight: 15.0,
            min_position_size: 1000.0,
            max_core_positions: 10,
            max_satellite_positions: 10,
            rebalance_threshold: 10.0,
        }
    }
}

impl PoolsManager {
    /// Create a new pools manager.
    pub fn new(config: PoolsConfig) -> Self {
        let pools = PortfolioPools::new(config.target_core_pct, config.target_satellite_pct);
        Self {
            pools: Arc::new(RwLock::new(pools)),
            config,
        }
    }

    /// Create with initial cash.
    pub fn with_cash(config: PoolsConfig, initial_cash: f64) -> Self {
        let mut pools = PortfolioPools::new(config.target_core_pct, config.target_satellite_pct);
        pools.cash = initial_cash;
        pools.total_value = initial_cash;
        Self {
            pools: Arc::new(RwLock::new(pools)),
            config,
        }
    }

    /// Get current portfolio state.
    pub async fn get_state(&self) -> PortfolioPools {
        self.pools.read().await.clone()
    }

    /// Get portfolio summary.
    pub async fn get_summary(&self) -> PortfolioSummary {
        self.pools.read().await.summary()
    }

    /// Add a new position.
    pub async fn add_position(&self, position: Position) -> Result<()> {
        let mut pools = self.pools.write().await;

        // Validate position
        self.validate_new_position(&pools, &position)?;

        pools.add_position(position);
        pools.recalculate_weights();

        Ok(())
    }

    /// Remove a position by symbol.
    pub async fn remove_position(&self, symbol: &str) -> Result<Option<Position>> {
        let mut pools = self.pools.write().await;
        let position = pools.remove_position(symbol);
        if let Some(ref pos) = position {
            pools.cash += pos.market_value();
        }
        pools.recalculate_weights();
        Ok(position)
    }

    /// Update position price.
    pub async fn update_price(&self, symbol: &str, price: f64) -> Result<bool> {
        let mut pools = self.pools.write().await;

        // Find and update in core
        if let Some(pos) = pools.core.iter_mut().find(|p| p.symbol == symbol) {
            pos.update_price(price);
            pools.recalculate_weights();
            return Ok(true);
        }

        // Find and update in satellite
        if let Some(pos) = pools.satellite.iter_mut().find(|p| p.symbol == symbol) {
            pos.update_price(price);
            pools.recalculate_weights();
            return Ok(true);
        }

        Ok(false)
    }

    /// Batch update prices.
    pub async fn update_prices(&self, prices: &[(String, f64)]) {
        let mut pools = self.pools.write().await;

        for (symbol, price) in prices {
            // Update in core
            if let Some(pos) = pools.core.iter_mut().find(|p| &p.symbol == symbol) {
                pos.update_price(*price);
            }
            // Update in satellite
            if let Some(pos) = pools.satellite.iter_mut().find(|p| &p.symbol == symbol) {
                pos.update_price(*price);
            }
        }

        pools.recalculate_weights();
    }

    /// Move position to different tier.
    pub async fn move_to_tier(&self, symbol: &str, new_tier: PoolTier) -> Result<bool> {
        let mut pools = self.pools.write().await;
        let moved = pools.move_position(symbol, new_tier);
        if moved {
            pools.recalculate_weights();
        }
        Ok(moved)
    }

    /// Promote from watchlist to active position.
    pub async fn promote_from_watchlist(
        &self,
        symbol: &str,
        price: f64,
        quantity: f64,
    ) -> Result<()> {
        let mut pools = self.pools.write().await;

        // Find in watchlist
        let watch_idx = pools
            .watchlist
            .iter()
            .position(|w| w.symbol == symbol)
            .ok_or_else(|| anyhow::anyhow!("Symbol {} not in watchlist", symbol))?;

        let watch_item = pools.watchlist.remove(watch_idx);

        // Create position
        let position = Position {
            symbol: watch_item.symbol,
            name: watch_item.name,
            tier: watch_item.target_pool,
            entry_price: price,
            current_price: price,
            quantity,
            weight: 0.0,
            entry_date: Utc::now(),
            investment_thesis: watch_item.investment_thesis,
            key_metrics: vec![],
            stop_loss_triggers: match watch_item.target_pool {
                PoolTier::Core => StopLossTriggers::core_defaults(),
                PoolTier::Satellite => StopLossTriggers::satellite_defaults(),
            },
            high_since_entry: price,
            updated_at: Utc::now(),
        };

        // Deduct cash
        let cost = price * quantity;
        if pools.cash < cost {
            return Err(anyhow::anyhow!(
                "Insufficient cash: need {:.2}, have {:.2}",
                cost,
                pools.cash
            ));
        }
        pools.cash -= cost;

        pools.add_position(position);
        pools.recalculate_weights();

        Ok(())
    }

    /// Add to watchlist.
    pub async fn add_to_watchlist(&self, item: WatchItem) -> Result<()> {
        let mut pools = self.pools.write().await;

        // Check if already exists
        if pools.watchlist.iter().any(|w| w.symbol == item.symbol) {
            return Err(anyhow::anyhow!("{} already in watchlist", item.symbol));
        }

        pools.watchlist.push(item);
        Ok(())
    }

    /// Remove from watchlist.
    pub async fn remove_from_watchlist(&self, symbol: &str) -> Result<Option<WatchItem>> {
        let mut pools = self.pools.write().await;

        if let Some(idx) = pools.watchlist.iter().position(|w| w.symbol == symbol) {
            return Ok(Some(pools.watchlist.remove(idx)));
        }

        Ok(None)
    }

    /// Get positions that need rebalancing.
    pub async fn get_rebalance_needs(&self) -> RebalanceNeeds {
        let pools = self.pools.read().await;

        let core_current = pools.core_allocation_pct();
        let satellite_current = pools.satellite_allocation_pct();

        let core_deviation = core_current - self.config.target_core_pct;
        let satellite_deviation = satellite_current - self.config.target_satellite_pct;

        let needs_rebalance = core_deviation.abs() > self.config.rebalance_threshold
            || satellite_deviation.abs() > self.config.rebalance_threshold;

        // Find positions over max weight
        let overweight_positions: Vec<String> = pools
            .all_positions()
            .into_iter()
            .filter(|p| p.weight > self.config.max_position_weight)
            .map(|p| p.symbol.clone())
            .collect();

        RebalanceNeeds {
            needs_rebalance,
            core_current_pct: core_current,
            core_target_pct: self.config.target_core_pct,
            core_deviation_pct: core_deviation,
            satellite_current_pct: satellite_current,
            satellite_target_pct: self.config.target_satellite_pct,
            satellite_deviation_pct: satellite_deviation,
            overweight_positions,
        }
    }

    /// Get the strongest performer (for "nurture strong" strategy).
    pub async fn get_strongest_performer(&self) -> Option<Position> {
        let pools = self.pools.read().await;
        pools
            .positions_by_performance()
            .first()
            .cloned()
            .cloned()
    }

    /// Get the weakest performer (for "trim weak" strategy).
    pub async fn get_weakest_performer(&self) -> Option<Position> {
        let pools = self.pools.read().await;
        pools
            .positions_by_performance()
            .last()
            .cloned()
            .cloned()
    }

    /// Get positions with triggered stops.
    pub async fn get_triggered_stops(&self) -> Vec<Position> {
        let pools = self.pools.read().await;
        pools
            .positions_with_stops_triggered()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Validate a new position before adding.
    fn validate_new_position(&self, pools: &PortfolioPools, position: &Position) -> Result<()> {
        // Check if already exists
        if pools.all_positions().iter().any(|p| p.symbol == position.symbol) {
            return Err(anyhow::anyhow!(
                "Position {} already exists",
                position.symbol
            ));
        }

        // Check position count limits
        match position.tier {
            PoolTier::Core if pools.core.len() >= self.config.max_core_positions => {
                return Err(anyhow::anyhow!(
                    "Core pool at maximum capacity ({})",
                    self.config.max_core_positions
                ));
            }
            PoolTier::Satellite if pools.satellite.len() >= self.config.max_satellite_positions => {
                return Err(anyhow::anyhow!(
                    "Satellite pool at maximum capacity ({})",
                    self.config.max_satellite_positions
                ));
            }
            _ => {}
        }

        // Check minimum size
        if position.market_value() < self.config.min_position_size {
            return Err(anyhow::anyhow!(
                "Position size {:.2} below minimum {:.2}",
                position.market_value(),
                self.config.min_position_size
            ));
        }

        Ok(())
    }

    /// Add cash to portfolio.
    pub async fn add_cash(&self, amount: f64) {
        let mut pools = self.pools.write().await;
        pools.cash += amount;
        pools.recalculate_weights();
    }

    /// Withdraw cash from portfolio.
    pub async fn withdraw_cash(&self, amount: f64) -> Result<()> {
        let mut pools = self.pools.write().await;
        if pools.cash < amount {
            return Err(anyhow::anyhow!(
                "Insufficient cash: need {:.2}, have {:.2}",
                amount,
                pools.cash
            ));
        }
        pools.cash -= amount;
        pools.recalculate_weights();
        Ok(())
    }
}

/// Rebalancing needs assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceNeeds {
    pub needs_rebalance: bool,
    pub core_current_pct: f64,
    pub core_target_pct: f64,
    pub core_deviation_pct: f64,
    pub satellite_current_pct: f64,
    pub satellite_target_pct: f64,
    pub satellite_deviation_pct: f64,
    pub overweight_positions: Vec<String>,
}

use serde::{Deserialize, Serialize};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pools_manager_basic() {
        let config = PoolsConfig::default();
        let manager = PoolsManager::with_cash(config, 100_000.0);

        let summary = manager.get_summary().await;
        assert!((summary.total_value - 100_000.0).abs() < 0.01);
        assert!((summary.cash_allocation_pct - 100.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_add_position() {
        let config = PoolsConfig::default();
        let manager = PoolsManager::with_cash(config, 100_000.0);

        let position = Position {
            symbol: "TEST".to_string(),
            name: "Test Company".to_string(),
            tier: PoolTier::Core,
            entry_price: 100.0,
            current_price: 100.0,
            quantity: 100.0,
            weight: 0.0,
            entry_date: Utc::now(),
            investment_thesis: "Test".to_string(),
            key_metrics: vec![],
            stop_loss_triggers: StopLossTriggers::default(),
            high_since_entry: 100.0,
            updated_at: Utc::now(),
        };

        manager.add_position(position).await.unwrap();

        let summary = manager.get_summary().await;
        assert_eq!(summary.core_positions, 1);
    }

    #[tokio::test]
    async fn test_watchlist_promotion() {
        let config = PoolsConfig::default();
        let manager = PoolsManager::with_cash(config, 100_000.0);

        // Add to watchlist
        let watch_item = WatchItem {
            symbol: "WATCH".to_string(),
            name: "Watch Company".to_string(),
            watch_reason: "Potential value".to_string(),
            target_entry_price: Some(50.0),
            target_pool: PoolTier::Satellite,
            investment_thesis: "Undervalued".to_string(),
            added_at: Utc::now(),
            last_reviewed: Utc::now(),
        };
        manager.add_to_watchlist(watch_item).await.unwrap();

        // Promote
        manager
            .promote_from_watchlist("WATCH", 50.0, 100.0)
            .await
            .unwrap();

        let state = manager.get_state().await;
        assert_eq!(state.watchlist.len(), 0);
        assert_eq!(state.satellite.len(), 1);
        assert!((state.cash - 95_000.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_rebalance_detection() {
        let config = PoolsConfig {
            target_core_pct: 70.0,
            target_satellite_pct: 30.0,
            rebalance_threshold: 10.0,
            ..Default::default()
        };
        let manager = PoolsManager::with_cash(config, 100_000.0);

        // Add only satellite positions (should trigger rebalance need)
        let position = Position {
            symbol: "SAT1".to_string(),
            name: "Satellite 1".to_string(),
            tier: PoolTier::Satellite,
            entry_price: 100.0,
            current_price: 100.0,
            quantity: 500.0, // 50% of portfolio
            weight: 0.0,
            entry_date: Utc::now(),
            investment_thesis: "Test".to_string(),
            key_metrics: vec![],
            stop_loss_triggers: StopLossTriggers::default(),
            high_since_entry: 100.0,
            updated_at: Utc::now(),
        };
        manager.add_position(position).await.unwrap();

        let needs = manager.get_rebalance_needs().await;
        assert!(needs.needs_rebalance);
        assert!(needs.core_deviation_pct < -10.0); // Core is underweight
    }
}
