---
id: "weekly-value-analysis"
name: "周度价值分析"
version: "1.0.0"
schedule: "0 0 9 * * 1"
agent: "value-analyst"
enabled: true
memory_path: "hands/value-analysis/{date}.md"

autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 5

decision:
  use_close: true
  web_search: true
  evolution: true
  auto_continue: true

resources:
  max_tokens: 80000
  max_cost_usd: 3.0
  max_duration_sec: 600

params:
  framework: "observer-constructionism"
  focus_areas:
    - "国家共识"
    - "商业评估权"
    - "财务硬实在"
---

# 周度价值分析

每周一早上 9:00 生成基于《价值逻辑》框架的分析报告。

## 分析框架

使用"观察者建构论"三层分析：

1. **国家共识层** - 政策导向、产业支持、监管环境
2. **商业评估权层** - 行业地位、定价能力、护城河
3. **财务硬实在层** - 现金流、资产质量、盈利能力

## 输出要求

- 识别核心资产机会
- 评估可用余量（optionality）
- 给出可持续性评分
