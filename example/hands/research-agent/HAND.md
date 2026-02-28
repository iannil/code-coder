---
id: "research-agent"
name: "Autonomous Research Agent"
version: "1.0.0"
schedule: "0 0 * * * *"
agent: "general"
enabled: false

# Autonomous Configuration
autonomy:
  level: "insane"
  unattended: true
  max_iterations: 10

# Decision Configuration (CLOSE Framework)
decision:
  use_close: true
  web_search: true
  evolution: true
  auto_continue: true

# Resource Limits
resources:
  max_tokens: 200000
  max_cost_usd: 10.0
  max_duration_sec: 1800

# Memory output
memory_path: "hands/research-agent/{date}.md"

# Additional parameters
params:
  search_depth: "deep"
  include_sOURCES: true
---

# Autonomous Research Agent

A self-directed research agent that explores topics, synthesizes information, and generates comprehensive reports.

## Capabilities

- Deep web research with source verification
- Multi-source synthesis
- Trend analysis
- Knowledge sedimentation (learns from previous research)

## Research Process

1. **Understand**: Parse research requirements
2. **Explore**: Search and gather information
3. **Synthesize**: Combine insights from multiple sources
4. **Evaluate**: Apply CLOSE framework to validate findings
5. **Report**: Generate structured output

## CLOSE Integration

Each research step is evaluated:

- **Convergence**: How well-defined is the research question?
- **Leverage**: Value vs time investment
- **Optionality**: Can findings be applied elsewhere?
- **Surplus**: Within resource budgets?
- **Evolution**: What did we learn?

## Schedule

Runs hourly: `0 0 * * * *`

Trigger manually for immediate research needs.
