---
id: "daily-summary"
name: "Daily Summary"
version: "1.0.0"
schedule: "0 0 9 * * *"
agent: "writer"
enabled: false
memory_path: "hands/daily-summary/{date}.md"
params:
  sections:
    - "market_overview"
    - "trading_activity"
    - "key_events"
    - "action_items"
---

# Daily Summary

Generates a comprehensive daily summary of market conditions, trading activity, and key events.

## Schedule

Runs every day at 9:00 AM UTC.

## Sections

1. **Market Overview**: Summary of major market indices and key economic indicators
2. **Trading Activity**: Review of recent trades and positions
3. **Key Events**: Important news, earnings, or economic releases
4. **Action Items**: Tasks to follow up on

## Context

This hand has access to:

- Previous daily summaries (for trend comparison)
- Current portfolio state
- Recent economic data
- Trading history
