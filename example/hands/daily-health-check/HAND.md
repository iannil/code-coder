---
id: "daily-health-check"
name: "每日代码健康检查"
version: "1.0.0"
schedule: "0 0 8 * * 1-5"
agent: "verifier"
enabled: true
memory_path: "hands/daily-health-check/{date}.md"

autonomy:
  level: "bold"
  unattended: true
  max_iterations: 3
  auto_approve:
    enabled: true
    allowed_tools: ["Read", "Grep", "Glob", "Bash"]
    risk_threshold: "low"
    timeout_ms: 30000

decision:
  use_close: true
  web_search: false
  auto_continue: true

resources:
  max_tokens: 50000
  max_cost_usd: 2.0
  max_duration_sec: 300
---

# 每日代码健康检查

每个工作日早上 8:00 自动执行代码库健康检查。

## 检查项目

1. **Build 检查** - 验证项目可以成功编译
2. **Type 检查** - TypeScript/Rust 类型检查
3. **Lint 检查** - 代码风格和潜在问题
4. **Test 检查** - 单元测试和集成测试
5. **Console.log 审计** - 检查遗留的调试语句
6. **Git 状态** - 检查未提交的更改

## 输出格式

生成 Markdown 报告，包含：

- 检查摘要（通过/失败/警告数量）
- 详细问题列表
- 建议的修复操作
