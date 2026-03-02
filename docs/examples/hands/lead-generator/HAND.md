---
# Lead - 销售线索生成器 Hand
# 利用 picker agent 内置选品策略自动生成和筛选销售线索

id: "lead-generator"
name: "Lead Generator"
description: "自动化销售线索发现、验证和优先级排序"
version: "1.0.0"
author: "CodeCoder"

# 调度配置 - 工作日早9点执行
schedule: "0 9 * * 1-5"

# 单 Agent - picker 内置选品策略
agent: "picker"

# 高自主级别 - 需要大量自主收集
autonomy:
  level: "crazy"
  score_threshold: 65
  approval_threshold: 6.0

# Picker 参数配置
params:
  strategy: "lead_scoring"
  sources:
    - type: "linkedin"
      keywords: ["SaaS", "startup", "CTO", "VP Engineering"]
    - type: "crunchbase"
      stage: ["Series A", "Series B"]
      industries: ["B2B", "Enterprise Software"]
    - type: "twitter"
      hashtags: ["#saas", "#startup", "#enterprise"]
  filters:
    company_size: "50-500"
    geography: ["US", "UK", "EU"]
    intent_signals: ["hiring", "funding", "expansion"]

# CLOSE 框架 - Leverage 优化
decision:
  use_close: true
  focus_dimension: "leverage"
  web_search: true
  evolution: true

# 风险控制
risk_control:
  max_tokens: 8000
  max_cost_usd: 0.30
  max_duration_sec: 180

# 记忆存储路径
memory_path: "hands/leads/{date}.md"

# 输出配置
output:
  format: "structured"
  fields: ["company", "contacts", "score", "reason"]

# 启用状态
enabled: true
---

# Lead Generator Hand

## 概述

此 Hand 利用 CodeCoder 的 picker agent 内置的选品策略，自动发现、验证和优先级排序 B2B 销售线索。

## 核心能力

### 1. 多源数据收集
- LinkedIn: 目标职位和公司扫描
- Crunchbase: 融资阶段筛选
- Twitter: 意向信号监测

### 2. 自动评分系统
基于以下维度计算线索质量：
- 公司规模匹配度
- 融资阶段适宜性
- 意向信号强度
- 地理位置偏好

### 3. Leverage 优化
应用 CLOSE 框架的 Leverage 维度：
- 优先处理高价值/低时间成本线索
- 自动标记需要人工跟进的复杂线索

## 输出格式

```markdown
## 销售线索报告 - {date}

### 🔥 高优先级 (Score > 80)
- [公司名] - [接触点] - [理由]
- ...

### 📊 中优先级 (Score 60-80)
- ...

### 📝 低优先级 (Score < 60)
- ...
```

## 使用建议

1. 每日早上检查生成报告
2. 优先联系高优先级线索
3. 定期反馈线索质量，优化评分模型
