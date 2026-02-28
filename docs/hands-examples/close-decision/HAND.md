---
id: "close-decision"
name: "CLOSE Decision Advisor"
version: "1.0.0"
schedule: "0 0 21 * * *"
agent: "decision"
enabled: true
memory_path: "hands/close-decision/{date}.md"
params:
  review_period: "daily"
  include_pending: true
autonomy:
  level: "bold"
  unattended: false
  max_iterations: 1
decision:
  use_close: true
  web_search: false
  evolution: false
  auto_continue: false
resources:
  max_tokens: 20000
  max_cost_usd: 0.5
  max_duration_sec: 120
---

# CLOSE Decision Advisor

基于祝融说哲学框架，回顾当日决策质量。

## CLOSE 五维评估

- **C**onvergence (收敛): 决策是否聚焦于核心目标
- **L**everage (杠杆): 投入产出比是否合理
- **O**ptionality (可选性): 是否保留了足够的选择空间
- **S**urplus (余量): 资源是否留有冗余
- **E**volution (演化): 是否为未来学习积累经验

## 每日回顾

1. 收集当日做出的重要决策
2. 为每个决策评估 CLOSE 五维得分
3. 识别低分决策并分析原因
4. 提出改进建议

## 输出示例

```markdown
# CLOSE Decision Review - {date}

## 决策清单

### 决策 1: 重构认证模块
- C: 8/10 - 目标明确
- L: 6/10 - 工作量较大
- O: 9/10 - 可回滚
- S: 5/10 - 时间略紧
- E: 7/10 - 学到新模式
- **总分: 7.0/10** ✅ 继续执行

### 决策 2: 购买新服务器
- C: 4/10 - 需求不明确
- L: 3/10 - 成本较高
- O: 2/10 - 难以退货
- S: 6/10 - 预算充足
- E: 4/10 - 学习价值低
- **总分: 3.8/10** ❌ 建议暂缓

## 改进建议
...
```
