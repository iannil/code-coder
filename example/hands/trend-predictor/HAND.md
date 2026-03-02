---
# Predictor - 趋势预测器 Hand
# 涉及资金决策，使用最谨慎的 timid 级别

id: "trend-predictor"
name: "Trend Predictor"
description: "市场趋势预测和交易信号分析，严格风险控制"
version: "1.0.0"
author: "CodeCoder"

# 调度配置 - 每个工作日早8点执行
schedule: "0 8 * * 1-5"

# 单 Agent - trader
agent: "trader"

# 最谨慎自治级别 - 涉及资金决策
autonomy:
  level: "timid"
  score_threshold: 15
  approval_threshold: 8.0
  human_in_loop: true
  require_confirmation: true

# 严格风险控制
risk_control:
  max_tokens: 5000
  max_cost_usd: 0.20
  max_duration_sec: 120
  # 交易相关限制
  max_position_size: 0.05  # 最大5%仓位
  max_daily_trades: 3
  require_stop_loss: true

# Trader 参数
params:
  strategy: "trend_following"
  timeframes: ["daily", "weekly"]
  indicators:
    - "moving_averages"
    - "rsi"
    - "macd"
    - "volume_profile"
  assets:
    - type: "crypto"
      symbols: ["BTC", "ETH"]
    - type: "forex"
      pairs: ["EUR/USD", "USD/JPY"]

# CLOSE 框架 - 强调 Optionality
decision:
  use_close: true
  auto_continue: false  # 需要人工确认
  web_search: true
  evolution: true
  close_dimensions:
    - optionality  # 强调可逆性
    - surplus
    - leverage

# 审批流程
approval:
  timeout: 3600  # 1小时无响应则跳过
  auto_approve: false  # 永不自动批准
  notification:
    - type: "webhook"
      url: "${ZERO_WEBHOOK_URL}/trading-alerts"
    - type: "console"
      level: "warn"

# 记忆存储路径
memory_path: "hands/predictions/{date}.md"

# 输出配置
output:
  format: "structured"
  include_signals: true
  include_confidence: true
  include_risk_assessment: true

# 启用状态
enabled: true
---

# Trend Predictor Hand

## 概述

此 Hand 专注于市场趋势预测，由于涉及资金决策，使用最谨慎的 `timid` 自治级别，所有预测需要人工确认后才能执行。

## 安全第一设计

```
┌─────────────────────────────────────────────────────────────┐
│                      安全层级                                │
├─────────────────────────────────────────────────────────────┤
│  Level 1: Timid 自治 (score < 15)                           │
│           ↓                                                 │
│  Level 2: 人工审批 (approval_threshold: 8.0)                │
│           ↓                                                 │
│  Level 3: 仓位限制 (max 5%)                                 │
│           ↓                                                 │
│  Level 4: 止损强制 (require_stop_loss)                      │
└─────────────────────────────────────────────────────────────┘
```

## 工作流程

1. **数据分析**
   - 多时间框架技术分析
   - 趋势识别和强度评估
   - 支撑/阻力位计算

2. **CLOSE 评估**
   - **Optionality**: 决策是否可逆？
   - **Surplus**: 风险预算是否充足？
   - **Leverage**: 风险收益比是否合理？

3. **生成信号**
   - 买入/卖出/持有建议
   - 置信度评分 (0-100)
   - 风险评级 (低/中/高)

4. **人工审批**
   - 发送通知到配置的端点
   - 等待人工确认
   - 1小时无响应则跳过本次信号

## 输出格式

```markdown
## 趋势预测报告 - {date}

### 📊 信号摘要
| 资产 | 信号 | 置信度 | 风险 | 建议仓位 |
|------|------|--------|------|----------|
| BTC  | 买入 | 72%    | 中   | 3%       |
| ETH  | 持有 | 45%    | 低   | 0%       |

### 🔍 技术分析
- 趋势: [上升/下降/震荡]
- 关键位: 支撑 $XX,XXX / 阻力 $XX,XXX
- 指标信号: [MA金叉/RSI超买/...]

### ⚖️ CLOSE 评估
- Optionality: ✅ 可快速止损
- Surplus: 📊 风险预算剩余 85%
- Leverage: 💰 风险收益比 1:3

### ⚠️ 风险提示
[具体风险说明]

### ✅ 需要操作
[ ] 批准 - BTC 买入 3%
[ ] 拒绝
[ ] 跳过
```

## 使用建议

1. **每日早上检查**: 报告生成后及时审阅
2. **长期追踪**: 记录预测准确性，优化策略
3. **风险管理**: 永远不要超出设定的仓位限制

## 免责声明

此 Hand 提供的分析仅供参考，不构成任何投资建议。所有交易决策需自行判断并承担风险。
