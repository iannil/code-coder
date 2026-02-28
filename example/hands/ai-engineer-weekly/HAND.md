---
id: "ai-engineer-weekly"
name: "AI 工程学习周报"
version: "1.0.0"
schedule: "0 0 18 * * 5"
agent: "ai-engineer"
enabled: true
memory_path: "hands/ai-engineer/{date}.md"

autonomy:
  level: "wild"
  unattended: true
  max_iterations: 3

decision:
  use_close: true
  web_search: true
  auto_continue: true

resources:
  max_tokens: 60000
  max_cost_usd: 2.5
  max_duration_sec: 480

params:
  topics:
    - "LLM 应用开发"
    - "RAG 系统"
    - "Agent 架构"
    - "Prompt Engineering"
---

# AI 工程学习周报

每周五下午 6:00 生成 AI 工程领域的学习总结。

## 内容范围

1. **本周技术动态** - 新模型、新工具、新框架
2. **实践总结** - 本周 coding 中的 AI 应用经验
3. **下周学习计划** - 基于 CLOSE 框架评估优先级

## 输出要求

- 简明扼要，聚焦实用
- 包含代码示例或配置参考
- 给出学习资源链接
