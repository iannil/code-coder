# CodeCoder 自主性提升实现报告

**日期**: 2026-02-28
**状态**: ✅ 已完成

## 概述

实现了 CodeCoder 的自主执行能力，使 Hands 系统能够：
1. 基于风险等级自动批准低风险操作
2. 在无人值守模式下自主执行 Hands
3. 敏感操作自动进入审批队列
4. TUI 中实时查看和处理审批请求

## 实现内容

### 阶段 1: 工具调用拦截与风险评估 (Rust)

**新增文件**:
- `services/zero-workflow/src/hands/risk.rs` - 风险评估器
- `services/zero-workflow/src/hands/auto_approve.rs` - 自动批准逻辑

**关键类型**:
- `RiskLevel`: Safe → Low → Medium → High → Critical
- `RiskEvaluator`: 评估工具调用的风险级别
- `AutoApprover`: 决定自动批准/排队/拒绝
- `ApprovalResult`: 包含决策、风险评估和超时配置

**风险评估规则**:
- 静态分类：每个工具有基础风险级别
- 动态调整：参数模式匹配可升降风险
- 例如：`Bash` 基础为 High，但 `git status` 降为 Medium，`rm -rf` 升为 Critical

### 阶段 2: HITL 与 Hands 执行集成 (Rust)

**修改文件**:
- `services/zero-workflow/src/hands/state.rs` - 添加 `WaitingApproval` 状态
- `services/zero-workflow/src/hands/executor.rs` - 添加审批检查和等待逻辑
- `services/zero-gateway/src/hitl/mod.rs` - 添加 `ToolExecution` 审批类型
- `services/zero-gateway/src/hitl/actions.rs` - 添加 `ToolExecutionAction` 处理器
- `services/zero-gateway/src/hitl/cards/mod.rs` - 添加工具执行卡片格式化

**新增方法**:
- `HandExecutor::check_tool_approval()` - 检查工具是否需要审批
- `HandExecutor::create_hitl_approval_request()` - 创建 HITL 审批请求
- `HandExecutor::poll_approval_status()` - 轮询审批状态

### 阶段 3: 超时自动批准 (Rust)

**新增方法**:
- `HandExecutor::wait_for_approval()` - 等待审批并处理超时

**超时规则**:
- 仅在 `unattended` 模式下生效
- Critical 风险操作永不超时批准
- 默认超时时间可在 Hand 配置中设置
- 最大等待时间 1 小时

### 阶段 4: TUI 集成 (TypeScript)

**修改文件**:
- `packages/ccode/src/cli/cmd/tui/app.tsx` - 添加审批队列命令的 keybind
- `packages/ccode/src/config/config.ts` - 添加 `approval_queue` 键绑定 (`<leader>o`)
- `packages/ccode/src/cli/cmd/tui/component/dialog-approval-queue.tsx` - 添加 tool_execution 图标

**快捷键**: `<leader>o` (默认 Ctrl+X 后按 o)

### 阶段 5: TypeScript 类型更新

**修改文件**:
- `packages/ccode/src/hitl/client.ts` - 添加 `tool_execution` 审批类型
- `packages/ccode/src/autonomous/hands/bridge.ts` - 添加审批相关类型

## 配置示例

```yaml
# HAND.md 示例
---
id: "test-auto"
name: "Test Auto Approve"
schedule: "0 */5 * * * *"
agent: "general"
autonomy:
  level: "crazy"
  unattended: true
  auto_approve:
    enabled: true
    allowed_tools: ["Read", "Glob", "Grep", "WebSearch"]
    risk_threshold: "low"
    timeout_ms: 30000
---
```

## 测试结果

- **zero-workflow**: 20 tests passed
- **zero-gateway**: 127 tests passed
- **TypeScript**: typecheck passed

## 默认风险配置

| 风险级别 | 示例工具 | 说明 |
|---------|----------|------|
| Safe | Read, Glob, LS | 只读，无副作用 |
| Low | Grep, WebSearch, WebFetch | 只读，可能有网络请求 |
| Medium | Edit (small), NotebookEdit | 有限写操作 |
| High | Bash, Write | 破坏性写操作 |
| Critical | rm -rf, sudo, git push --force | 不可逆操作 |

## 后续优化建议

1. **审批队列持久化**: 当前使用内存存储，可考虑持久化到 SQLite
2. **审批通知**: 集成 IM 通道推送审批请求
3. **批量审批**: 允许一次性审批多个低风险操作
4. **审批历史**: 记录审批决策用于审计
