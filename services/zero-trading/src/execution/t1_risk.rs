//! T+1 Risk management for A-shares.
//!
//! Due to T+1 rules in A-share market:
//! - Cannot sell on the same day as purchase
//! - Must make decisions based on next-day opening auction
//! - Risk management focuses on position sizing and overnight gap risk

use serde::{Deserialize, Serialize};

use crate::data::AuctionData;
use super::Position;

/// T+1 risk configuration
#[derive(Debug, Clone)]
pub struct T1RiskConfig {
    /// Stop loss percentage
    pub stop_loss_pct: f64,
    /// Take profit percentage
    pub take_profit_pct: f64,
    /// Maximum loss per trade as percentage of capital
    pub max_loss_per_trade_pct: f64,
}

impl Default for T1RiskConfig {
    fn default() -> Self {
        Self {
            stop_loss_pct: 5.0,
            take_profit_pct: 10.0,
            max_loss_per_trade_pct: 2.0,
        }
    }
}

/// Next-day decision for T+1 positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum T1Decision {
    /// Sell at market open (stop loss or better opportunity)
    SellAtOpen,
    /// Hold and wait for target
    HoldToTarget,
    /// Wait and observe first 30 minutes
    WaitAndSee,
    /// Hold but move stop to breakeven
    HoldWithBreakeven,
}

/// T+1 position manager
pub struct T1RiskManager {
    config: T1RiskConfig,
}

impl T1RiskManager {
    /// Create a new T+1 risk manager
    pub fn new(config: T1RiskConfig) -> Self {
        Self { config }
    }

    /// Make next-day decision based on auction data
    ///
    /// Called during 9:15-9:25 auction period to decide what to do at 9:30 open
    pub fn next_day_decision(&self, position: &Position, auction: &AuctionData) -> T1Decision {
        let entry = position.entry_price;
        let expected_open = auction.expected_price;
        let expected_return = (expected_open - entry) / entry * 100.0;

        // Check for stop loss hit at open
        let stop_loss_pct = ((entry - position.stop_loss) / entry) * 100.0;
        if expected_return < -stop_loss_pct {
            tracing::info!(
                symbol = %position.symbol,
                expected_open,
                expected_return,
                "Stop loss triggered at auction"
            );
            return T1Decision::SellAtOpen;
        }

        // Check for take profit region
        let take_profit_pct = ((position.take_profit - entry) / entry) * 100.0;
        if expected_return >= take_profit_pct * 0.8 {
            // Near target - hold but be ready to exit
            return T1Decision::HoldToTarget;
        }

        // Gap down beyond tolerance - sell
        if expected_return < -2.0 {
            tracing::info!(
                symbol = %position.symbol,
                expected_return,
                "Gap down beyond tolerance"
            );
            return T1Decision::SellAtOpen;
        }

        // Gap up nicely - hold with breakeven stop
        if expected_return > 3.0 {
            return T1Decision::HoldWithBreakeven;
        }

        // Modest gap or flat - wait and see
        T1Decision::WaitAndSee
    }

    /// Calculate position size based on risk
    ///
    /// Uses fixed percentage risk per trade
    pub fn calculate_position_size(
        &self,
        total_capital: f64,
        entry_price: f64,
        stop_loss: f64,
    ) -> f64 {
        let risk_per_trade = total_capital * (self.config.max_loss_per_trade_pct / 100.0);
        let risk_per_share = (entry_price - stop_loss).abs();

        if risk_per_share <= 0.0 {
            return 0.0;
        }

        let shares = risk_per_trade / risk_per_share;

        // Round down to nearest 100 (A-share lot size)
        (shares / 100.0).floor() * 100.0
    }

    /// Check if position size is valid
    pub fn validate_position_size(
        &self,
        quantity: f64,
        price: f64,
        available_capital: f64,
        max_position_pct: f64,
        total_capital: f64,
    ) -> PositionSizeValidation {
        let required = quantity * price;

        if required > available_capital {
            return PositionSizeValidation::InsufficientCapital {
                required,
                available: available_capital,
            };
        }

        let position_pct = (required / total_capital) * 100.0;
        if position_pct > max_position_pct {
            return PositionSizeValidation::ExceedsLimit {
                position_pct,
                limit: max_position_pct,
            };
        }

        // A-share minimum lot size is 100
        if quantity < 100.0 {
            return PositionSizeValidation::BelowMinimum {
                quantity,
                minimum: 100.0,
            };
        }

        // A-share lot size must be multiple of 100
        if quantity % 100.0 != 0.0 {
            return PositionSizeValidation::InvalidLotSize { quantity };
        }

        PositionSizeValidation::Valid
    }

    /// Calculate risk/reward for a potential trade
    pub fn calculate_risk_reward(
        &self,
        entry: f64,
        stop_loss: f64,
        take_profit: f64,
    ) -> RiskReward {
        let risk = (entry - stop_loss).abs();
        let reward = (take_profit - entry).abs();
        let ratio = if risk > 0.0 { reward / risk } else { 0.0 };

        RiskReward {
            risk_amount: risk,
            reward_amount: reward,
            ratio,
            risk_pct: (risk / entry) * 100.0,
            reward_pct: (reward / entry) * 100.0,
        }
    }

    /// Evaluate overnight gap risk
    ///
    /// A-shares can gap up/down 10% on limit moves
    pub fn evaluate_gap_risk(&self, position: &Position) -> GapRiskAssessment {
        let entry = position.entry_price;

        // Maximum gap scenarios
        let limit_down = entry * 0.9; // -10% limit down
        let limit_up = entry * 1.1; // +10% limit up

        let max_loss = (entry - limit_down) * position.quantity;
        let max_gain = (limit_up - entry) * position.quantity;

        // Expected gap based on typical overnight moves (~2%)
        let typical_gap = 0.02;
        let expected_gap_loss = entry * typical_gap * position.quantity;

        GapRiskAssessment {
            max_potential_loss: max_loss,
            max_potential_gain: max_gain,
            expected_gap_loss,
            stop_loss_at_limit_down: position.stop_loss <= limit_down,
        }
    }
}

/// Position size validation result
#[derive(Debug, Clone)]
pub enum PositionSizeValidation {
    Valid,
    InsufficientCapital { required: f64, available: f64 },
    ExceedsLimit { position_pct: f64, limit: f64 },
    BelowMinimum { quantity: f64, minimum: f64 },
    InvalidLotSize { quantity: f64 },
}

impl PositionSizeValidation {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Risk/reward calculation
#[derive(Debug, Clone)]
pub struct RiskReward {
    pub risk_amount: f64,
    pub reward_amount: f64,
    pub ratio: f64,
    pub risk_pct: f64,
    pub reward_pct: f64,
}

/// Gap risk assessment
#[derive(Debug, Clone)]
pub struct GapRiskAssessment {
    /// Maximum loss if limit down
    pub max_potential_loss: f64,
    /// Maximum gain if limit up
    pub max_potential_gain: f64,
    /// Expected loss from typical overnight gap
    pub expected_gap_loss: f64,
    /// Whether stop loss would be triggered at limit down
    pub stop_loss_at_limit_down: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_position() -> Position {
        Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0)
    }

    fn make_test_auction(expected_price: f64) -> AuctionData {
        use chrono::{Local, Utc};

        AuctionData {
            symbol: "000001.SZ".to_string(),
            date: Local::now().date_naive(),
            expected_price,
            volume: 1000000.0,
            change_percent: ((expected_price - 10.0) / 10.0) * 100.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_next_day_decision_stop_loss() {
        let manager = T1RiskManager::new(T1RiskConfig::default());
        let position = make_test_position();
        let auction = make_test_auction(9.0); // Below stop loss

        let decision = manager.next_day_decision(&position, &auction);
        assert_eq!(decision, T1Decision::SellAtOpen);
    }

    #[test]
    fn test_next_day_decision_near_target() {
        let manager = T1RiskManager::new(T1RiskConfig::default());
        let position = make_test_position();
        let auction = make_test_auction(10.9); // Near take profit

        let decision = manager.next_day_decision(&position, &auction);
        assert_eq!(decision, T1Decision::HoldToTarget);
    }

    #[test]
    fn test_next_day_decision_gap_up() {
        let manager = T1RiskManager::new(T1RiskConfig::default());
        let position = make_test_position();
        let auction = make_test_auction(10.5); // 5% gap up

        let decision = manager.next_day_decision(&position, &auction);
        assert_eq!(decision, T1Decision::HoldWithBreakeven);
    }

    #[test]
    fn test_position_size_calculation() {
        let manager = T1RiskManager::new(T1RiskConfig::default());

        // 100k capital, 2% risk = 2k risk
        // Entry 10, stop 9.5 = 0.5 risk per share
        // 2000 / 0.5 = 4000 shares, rounded to 4000
        let size = manager.calculate_position_size(100000.0, 10.0, 9.5);
        assert!((size - 4000.0).abs() < 1.0);
    }

    #[test]
    fn test_position_size_validation() {
        let manager = T1RiskManager::new(T1RiskConfig::default());

        // Valid case
        let valid = manager.validate_position_size(100.0, 10.0, 5000.0, 20.0, 10000.0);
        assert!(valid.is_valid());

        // Insufficient capital
        let insufficient = manager.validate_position_size(1000.0, 10.0, 5000.0, 20.0, 10000.0);
        assert!(!insufficient.is_valid());

        // Below minimum
        let below_min = manager.validate_position_size(50.0, 10.0, 5000.0, 20.0, 10000.0);
        assert!(!below_min.is_valid());
    }

    #[test]
    fn test_risk_reward_calculation() {
        let manager = T1RiskManager::new(T1RiskConfig::default());
        let rr = manager.calculate_risk_reward(10.0, 9.5, 11.0);

        assert!((rr.ratio - 2.0).abs() < 0.01); // 2:1 R:R
        assert!((rr.risk_pct - 5.0).abs() < 0.01); // 5% risk
        assert!((rr.reward_pct - 10.0).abs() < 0.01); // 10% reward
    }

    #[test]
    fn test_gap_risk_assessment() {
        let manager = T1RiskManager::new(T1RiskConfig::default());
        let position = make_test_position();
        let risk = manager.evaluate_gap_risk(&position);

        // Max loss at limit down: (10 - 9) * 100 = 100
        assert!((risk.max_potential_loss - 100.0).abs() < 1.0);

        // Max gain at limit up: (11 - 10) * 100 = 100
        assert!((risk.max_potential_gain - 100.0).abs() < 1.0);
    }
}
