# Zero Trading Service

Automated trading service using PO3 (Power of 3) + SMT divergence strategy, adapted for A-shares T+1 rules.

## Port

**4434** (HTTP API)

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    zero-trading (Rust Service)                      │
│                           :4434                                      │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  Market Data    │  │  Strategy       │  │  Execution      │     │
│  │  Aggregator     │  │  Engine         │  │  Engine         │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
│                                                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  Paper Trading  │  │  Backtest       │  │  Broker         │     │
│  │  Verification   │  │  Engine         │  │  Integration    │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

## Modules

### Data (`src/data/`)
- **mod.rs** - Core data types (Candle, Timeframe, SmtPair)
- **tushare.rs** - Tushare Pro API adapter
- **cache.rs** - Data cache with TTL
- **aggregator.rs** - Multi-timeframe data aggregation

### Strategy (`src/strategy/`)
- **mod.rs** - StrategyEngine for multi-timeframe analysis
- **po3.rs** - PO3 structure detection (Accumulation → Manipulation → Distribution)
- **smt.rs** - SMT divergence detection
- **signal.rs** - TradingSignal generation

### Execution (`src/execution/`)
- **mod.rs** - ExecutionEngine with T+1 compliance
- **position.rs** - Position management
- **order.rs** - Order types and status
- **t1_risk.rs** - T+1 risk rules (next-day decision)

### Broker (`src/broker/`)
- **mod.rs** - Broker trait definition
- **futu.rs** - Futu OpenAPI adapter (TCP to OpenD)

### Backtest (`src/backtest/`)
- **mod.rs** - Module entry
- **engine.rs** - BacktestEngine with T+1 simulation
- **metrics.rs** - Performance metrics (Sharpe, profit factor, etc.)
- **report.rs** - Report generation (text, Telegram)

### Paper Trading (`src/paper_trading/`)
- **mod.rs** - Paper trade types
- **runner.rs** - PaperTradingRunner for live simulation
- **validator.rs** - SignalValidator with configurable thresholds
- **report.rs** - Session reports with verification criteria

### Other
- **macro_filter.rs** - Macro economic environment filter
- **notification.rs** - Telegram notifications via zero-channels
- **routes.rs** - HTTP API endpoints

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/v1/signals` | GET | Get trading signals |
| `/api/v1/positions` | GET | Get positions |
| `/api/v1/status` | GET | System status |

## A-Share Adaptations

1. **T+1 Rule**: Cannot sell shares bought on same day
2. **Trading Hours**: 9:30-11:30, 13:00-15:00 Beijing time
3. **Auction Period**: 9:15-9:25 (pre-market)
4. **SMT Pairs**: CSI 300 vs CSI 500, SSE 50 vs STAR 50

## Tests

```bash
cargo test
# 81 tests passed
```

## Configuration

Configured via `~/.codecoder/config.json`:

```json
{
  "trading": {
    "host": "127.0.0.1",
    "port": 4434,
    "paper_trading": true,
    "tushare_token": "your_token",
    "futu": {
      "host": "127.0.0.1",
      "port": 11111,
      "real_trading": false
    }
  }
}
```
