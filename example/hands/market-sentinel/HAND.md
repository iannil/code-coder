---
id: "market-sentinel"
name: "Market Sentinel"
version: "1.0.0"
schedule: "0 */30 * * * *"
agent: "macro"
enabled: false

# Autonomous Configuration
autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 5

# Decision Configuration (CLOSE Framework)
decision:
  use_close: true
  web_search: true
  evolution: true
  auto_continue: true

# Resource Limits
resources:
  max_tokens: 100000
  max_cost_usd: 5.0
  max_duration_sec: 600

# Memory output
memory_path: "hands/market-sentinel/{date}.md"
---

# Market Sentinel

An autonomous market analysis agent that monitors economic indicators, market conditions, and trading opportunities.

## Mission

Continuously monitor and analyze market conditions to identify:

- Macro economic shifts
- Trading opportunities
- Risk factors
- Portfolio adjustments

## Analysis Framework

This agent uses the CLOSE decision framework:

- **Convergence**: Focus on specific market sectors
- **Leverage**: High-impact insights with minimal monitoring
- **Optionality**: Reversible recommendations
- **Surplus**: Operates within risk budgets
- **Evolution**: Learns from past predictions

## Output Format

Each execution produces:

1. Market condition summary
2. Key indicators watch
3. Risk assessment
4. Actionable recommendations
5. CLOSE evaluation scores

## Schedule

Runs every 30 minutes via cron: `0 */30 * * * *`
