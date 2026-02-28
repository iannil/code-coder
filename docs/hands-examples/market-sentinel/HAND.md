---
id: "market-sentinel"
name: "Market Sentinel"
version: "1.1.0"
schedule: "0 30 9,14,15 * * *"
agent: "macro"
enabled: true
memory_path: "hands/market-sentinel/{date}.md"
params:
  markets:
    - "A股"
    - "港股"
    - "美股"
  threshold: 0.7
autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 3
  auto_approve:
    enabled: true
    allowed_tools:
      - "Read"
      - "Glob"
      - "Grep"
      - "WebFetch"
      - "WebSearch"
    risk_threshold: "low"
    timeout_ms: 30000
decision:
  use_close: true
  web_search: true
  evolution: false
  auto_continue: true
resources:
  max_tokens: 50000
  max_cost_usd: 2.0
  max_duration_sec: 300
---

# Market Sentinel

实时监控市场异动，识别重要信号。

## 任务职责

1. **盘前分析** (09:30): 分析隔夜美股、期货、亚太市场对A股的影响
2. **午盘监控** (14:00): 检查盘中异动、板块轮动、资金流向
3. **收盘总结** (15:00): 生成当日市场复盘报告

## 监控指标

- 大盘指数走势和量能
- 北向资金流向
- 板块涨跌分布
- 涨停/跌停家数
- 融资融券余额变化

## 输出格式

每次执行生成 Markdown 报告，包含：
- 市场概况
- 关键数据点
- 风险提示
- 明日关注点
