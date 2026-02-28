---
id: "daily-market-review"
name: "每日市场回顾"
version: "1.0.0"
schedule: "0 0 9 * * 1-5"
agent: "macro"
enabled: true
memory_path: "hands/market-review/{date}.md"
autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 5
decision:
  use_close: true
  web_search: true
  evolution: false
  auto_continue: true
resources:
  max_tokens: 50000
  max_cost_usd: 2.0
  max_duration_sec: 600
params:
  regions: ["US", "CN", "EU"]
  indicators:
    - "PMI"
    - "CPI"
    - "GDP"
    - "Employment"
    - "Interest Rates"
---

# 每日市场回顾

工作日早上 9:00 自动分析全球宏观经济数据，生成市场简报。

## 职责

1. **数据收集**: 收集前一交易日的主要经济指标
2. **趋势分析**: 分析主要市场的价格走势和成交量
3. **风险评估**: 评估当前市场风险水平
4. **简报生成**: 生成简明的市场分析报告

## 关注指标

### 美国 (US)
- 联邦基金利率
- 非农就业数据
- CPI 通胀率
- 10Y 国债收益率

### 中国 (CN)
- LPR 利率
- PMI 制造业指数
- 社会融资规模
- 房地产销售数据

### 欧洲 (EU)
- ECB 基准利率
- 欧元区 CPI
- 德国 IFO 指数

## 输出格式

报告包含以下部分：

```markdown
# 市场简报 - {date}

## 一句话总结
{简明的市场状态描述}

## 关键指标
| 指标 | 值 | 变化 | 评估 |
|------|-----|------|------|
| ... | ... | ... | ... |

## 风险提示
- {风险点1}
- {风险点2}

## 操作建议
{基于 CLOSE 框架的决策建议}
```

## CLOSE 决策配置

使用 CLOSE 框架评估市场状况时的权重配置：

| 维度 | 权重 | 说明 |
|------|------|------|
| C (Cost/可用余量) | 25% | 评估可投入资源 |
| L (Loss/最大损失) | 30% | 风险控制优先 |
| O (Odds/成功概率) | 20% | 市场机会评估 |
| S (Sustainability/可持续性) | 15% | 长期影响 |
| E (Evolution/进化空间) | 10% | 学习价值 |

## 使用说明

1. 将此文件复制到 `~/.codecoder/hands/daily-market-review/`
2. 根据需要调整 `params.regions` 和 `params.indicators`
3. 确保 `zero-workflow` 服务正在运行
4. 查看执行结果：`~/.codecoder/memory/hands/market-review/{date}.md`
