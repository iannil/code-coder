---
id: "miniproduct-tracker"
name: "极小产品进度追踪"
version: "1.0.0"
schedule: "0 0 20 * * 7"
agent: "miniproduct"
enabled: true
memory_path: "hands/miniproduct/{date}.md"

autonomy:
  level: "crazy"
  unattended: true
  max_iterations: 4

decision:
  use_close: true
  web_search: false
  evolution: true
  auto_continue: true

resources:
  max_tokens: 70000
  max_cost_usd: 2.5
  max_duration_sec: 500

params:
  methodology: "indie-hacker"
  focus:
    - "需求验证"
    - "MVP 进度"
    - "变现路径"
    - "退出策略"
---

# 极小产品进度追踪

每周日晚上 8:00 追踪极小产品项目进度。

## 追踪维度

1. **需求验证** - 用户反馈、市场验证状态
2. **开发进度** - MVP 完成度、技术债务
3. **变现状态** - 付费用户、收入指标
4. **下周目标** - 使用 CLOSE 框架评估优先级

## 输出格式

- 进度仪表盘（完成百分比）
- 关键指标趋势
- 阻塞项和解决方案
- 下周 OKR
