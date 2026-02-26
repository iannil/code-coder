# Daily Macro Economic Analysis Implementation Report

## Summary

Implemented daily macro economic analysis scheduled push feature for the zero-trading service. The feature generates morning (pre-market) and afternoon (post-market) reports using the macro agent and pushes them to Telegram at 9:00 AM and 4:00 PM Beijing time on weekdays.

## Changes Made

### 1. Extended Report Types (`types.rs`)

Added `DailyMorning` and `DailyAfternoon` variants to `ReportType` enum with corresponding Display implementations:

```rust
pub enum ReportType {
    Weekly,
    Monthly,
    DailyMorning,    // 新增
    DailyAfternoon,  // 新增
    AdHoc,
}
```

### 2. Extended Configuration (`config.rs`)

Added new configuration fields to `MacroAgentConfig`:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `daily_morning_enabled` | `bool` | `true` | Enable daily morning reports |
| `daily_morning_cron` | `Option<String>` | `None` (9:00) | Cron expression |
| `daily_afternoon_enabled` | `bool` | `true` | Enable daily afternoon reports |
| `daily_afternoon_cron` | `Option<String>` | `None` (16:00) | Cron expression |
| `include_index_data` | `bool` | `true` | Include index data in reports |
| `index_symbols` | `Vec<String>` | `["000300.SH", "000905.SH", "000001.SH"]` | Indices to track |

### 3. Daily Report Prompts (`bridge.rs`)

Added specialized prompt templates:

- **Morning Report**: Market overview (overnight markets, futures, north-bound flow), macro dynamics (PMI/CPI/M2), trading recommendations
- **Afternoon Report**: Market summary (index performance, volume, breadth), fund flows (north-bound, sector flows), next-day outlook

### 4. Scheduling Logic (`report.rs`)

Extended `ReportGeneratorConfig` and `MacroReportGenerator`:

- Added `daily_morning_enabled/cron` and `daily_afternoon_enabled/cron` fields
- Implemented `should_generate_daily_morning()` and `should_generate_daily_afternoon()` methods
- Added weekend skipping (Saturday/Sunday)
- Updated state tracking with `last_daily_morning` and `last_daily_afternoon`

### 5. Index Overview Interface (`aggregator.rs`)

Added `IndexOverview` and `IndexData` structs for daily report data:

```rust
pub struct IndexOverview {
    pub indices: Vec<IndexData>,
    pub as_of: DateTime<Utc>,
}

pub struct IndexData {
    pub symbol: String,
    pub name: String,
    pub close: f64,
    pub change_pct: f64,
    pub volume: f64,
    pub ma5: Option<f64>,
    pub ma20: Option<f64>,
}
```

Added `get_index_overview()` method to `MarketDataAggregator`.

### 6. Updated Config Factory (`mod.rs`)

Extended `create_report_config()` to extract daily report settings.

### 7. Configuration Example (`config-example-full.jsonc`)

Updated with new daily report configuration options.

## Files Modified

| File | Change Type |
|------|-------------|
| `services/zero-trading/src/macro_agent/types.rs` | Modified |
| `services/zero-common/src/config.rs` | Modified |
| `services/zero-trading/src/macro_agent/bridge.rs` | Modified |
| `services/zero-trading/src/macro_agent/report.rs` | Modified |
| `services/zero-trading/src/macro_agent/mod.rs` | Modified |
| `services/zero-trading/src/data/mod.rs` | Modified |
| `services/zero-trading/src/data/aggregator.rs` | Modified |
| `docs/config-example-full.jsonc` | Modified |

## Test Results

All 28 macro_agent tests pass:

```
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 178 filtered out
```

## Configuration Example

```jsonc
"macro_agent": {
  "enabled": true,
  "timeout_secs": 30,
  "cache_duration_secs": 3600,
  "weekly_report_enabled": true,
  "monthly_report_enabled": true,
  "daily_morning_enabled": true,
  "daily_morning_cron": "0 0 9 * * *",
  "daily_afternoon_enabled": true,
  "daily_afternoon_cron": "0 0 16 * * *",
  "include_index_data": true,
  "index_symbols": ["000300.SH", "000905.SH", "000001.SH"]
}
```

## Data Flow

```
每日定时触发 (9:00/16:00 北京时间)
        ↓
MacroReportGenerator.check_and_generate_reports()
        ↓
should_generate_daily_*() → true (weekday, correct hour, not generated today)
        ↓
AgentBridge.generate_report(DailyMorning/DailyAfternoon)
        ↓
build_daily_*_prompt() → POST /api/v1/chat → macro agent
        ↓
NotificationClient.send_alert() → Telegram
```

## Verification Steps

```bash
# Build
cd services/zero-trading && cargo build

# Run tests
cargo test macro_agent

# Configure and start service
# Edit ~/.codecoder/config.json to enable daily reports
./ops.sh start zero-trading

# Check logs
./ops.sh logs zero-trading | grep "daily"
```

## Date

2026-02-26
