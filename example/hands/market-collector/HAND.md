---
# Collector - 市场情报收集 Hand
# 宏观经济分析 + 观察者理论的条件执行 Pipeline

id: "market-collector"
name: "Market Intelligence Collector"
description: "持续收集市场情报，结合宏观经济分析和观察者理论进行解读"
version: "1.0.0"
author: "CodeCoder"

# 调度配置 - 每4小时执行一次
schedule: "0 */4 * * *"

# 条件 Pipeline - 基于 macro 决策是否执行 observer
pipeline:
  mode: "conditional"
  agents:
    - name: "macro"
      role: "market_analyzer"
      params:
        indicators: ["PMI", "CPI", "unemployment", "fed_rate"]
        regions: ["US", "EU", "CN"]
      decision_point: true
      condition:
        metric: "volatility_score"
        threshold: 0.7
        comparison: "gt"
    - name: "observer"
      role: "meaning_interpreter"
      params:
        framework: "observer_theory"
        focus: "market_convergence"
      depends_on: "macro"

# 自治级别
autonomy:
  level: "wild"
  score_threshold: 50
  approval_threshold: 6.5

# CLOSE 框架集成
decision:
  use_close: true
  auto_continue: true
  web_search: true
  evolution: true
  close_dimensions:
    - convergence
    - surplus
    - evolution

# 风险控制
risk_control:
  max_tokens: 12000
  max_cost_usd: 0.40
  max_duration_sec: 240

# 记忆存储路径
memory_path: "hands/market/{date}/{hour}.md"

# 输出配置
output:
  format: "markdown"
  include_indicators: true
  include_interpretation: true

# 启用状态
enabled: true
---

# Market Intelligence Collector Hand

## 概述

此 Hand 实现条件执行的多 Agent Pipeline，首先进行宏观经济数据分析，只有在市场波动超过阈值时才启动观察者理论解读。

## 工作流程

### 阶段 1: Macro 分析 (始终执行)

```
macro agent → 计算波动率分数
```

**分析指标**:

- PMI (制造业/服务业)
- CPI (通胀率)
- 失业率
- 联邦基金利率

**决策点**:

- 波动率分数 > 0.7 → 继续 observer
- 波动率分数 ≤ 0.7 → 保存简要报告并结束

### 阶段 2: Observer 解读 (条件执行)

```
observer agent → 观察者理论解读
```

**解读框架**:

- 可能性收敛: 市场共识形成
- 观察效应: 数据发布对市场的影响
- 可用余量: 市场的反应空间

## CLOSE 框架应用

| 维度 | 应用 |
|------|------|
| **Convergence** | 识别市场共识形成点 |
| **Surplus** | 评估市场过度/不足反应 |
| **Evolution** | 追踪模式演变历史 |

## 输出示例

```markdown
## 市场情报报告 - {date} {hour}:00

### 📈 宏观指标
- PMI: 52.3 (↑ 0.5)
- CPI: 3.2% (↓ 0.1%)
- 波动率分数: 0.75

### 🔮 观察者解读
[仅当波动率 > 0.7 时显示]
市场正在收敛至 [具体观点]...
观察者效应导致 [具体现象]...

### 📊 历史对比
与 [日期] 相似度: 85%
```

## 使用场景

- 日内交易参考
- 宏观风险监控
- 市场情绪追踪
