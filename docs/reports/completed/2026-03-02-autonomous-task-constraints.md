# 自主任务行为规范改进

**日期**: 2026-03-02
**状态**: 已完成

## 问题背景

用户通过 Telegram 发送 "每天早上8点自动执行获取财经要闻并发给我"，agent（通用助手）错误地：
1. 在源码目录 `packages/ccode/` 创建了 3 个 shell 脚本
2. 使用硬编码的假数据，而不是调用现有的 macro agent
3. 没有使用 scheduler API 创建真正的定时任务

## 解决方案

### Phase 1: 清理错误创建的文件 ✅

删除了以下文件：
- `packages/ccode/auto_finance_email.sh`
- `packages/ccode/daily_finance_alert.sh`
- `packages/ccode/setup_auto_finance.sh`

### Phase 2: 增强 Agent 约束 ✅

**文件**: `packages/ccode/src/agent/prompt/general.txt`

添加了自主任务约束规则：
- 文件创建限制：只能在 workspace 目录创建文件
- 优先使用内置能力：调用 scheduler API 而非创建脚本
- 数据获取：使用对应 agent 而非硬编码

**文件**: `packages/ccode/src/agent/prompt/build.txt`

添加了相同的约束规则。

**文件**: `packages/ccode/src/agent/prompt/autonomous.txt`

添加了 "Task Constraints (自主任务约束)" 章节，因为来自 IM（Telegram 等）的消息默认使用 autonomous agent。包含：
- 文件创建限制
- 能力优先级
- 定时任务规范
- 数据获取规范

### Phase 3: 更新 CLAUDE.md 规范 ✅

**文件**: `/CLAUDE.md`

在 "项目指南" 部分添加了 "自主任务规范" 章节，明确规定：
- 文件位置限制
- 能力优先级
- 定时任务必须使用 Scheduler API

### Phase 4: 添加全局 Permission 约束 ✅

**文件**: `packages/ccode/src/agent/agent.ts`

在 `defaults` 变量中添加了全局 write permission 约束：
```typescript
write: {
  "*": "allow",
  "packages/**": "ask",
  "services/**": "ask",
  "src/**": "ask",
  "scripts/**": "ask",
  "~/.codecoder/workspace/**": "allow",
}
```

### Phase 5: 创建 Scheduler MCP Tool ✅

**新文件**: `packages/ccode/src/tool/scheduler.ts`

创建了 4 个 MCP 工具：
- `scheduler_create_task` - 创建定时任务
- `scheduler_list_tasks` - 列出所有定时任务
- `scheduler_delete_task` - 删除定时任务
- `scheduler_run_task` - 手动触发任务执行

**文件**: `packages/ccode/src/tool/registry.ts`

注册了上述 4 个工具。

## 验证方法

1. ✅ 删除错误文件后确认 git status 干净
2. ✅ 类型检查通过（bun tsc --noEmit）
3. 待测试：
   - 重新发送 Telegram 消息测试
   - 确认使用 `scheduler_create_task` MCP tool
   - 确认使用 macro agent 获取数据
   - 确认不创建任何 .sh 脚本

## 关键文件变更

| 文件 | 修改内容 |
|------|---------|
| `packages/ccode/src/agent/prompt/general.txt` | 添加自主任务约束说明 |
| `packages/ccode/src/agent/prompt/build.txt` | 添加相同约束 |
| `packages/ccode/src/agent/prompt/autonomous.txt` | 添加 Task Constraints 章节（IM 消息默认使用此 agent） |
| `/CLAUDE.md` | 添加自主任务规范章节 |
| `packages/ccode/src/agent/agent.ts` | 在 defaults 中添加全局 write permission 约束 |
| `packages/ccode/src/tool/scheduler.ts` | **新建** scheduler MCP tool |
| `packages/ccode/src/tool/registry.ts` | 注册 scheduler tools |

## 技术洞察

`★ Insight ─────────────────────────────────────`
**权限分层设计**:
- `defaults` 定义所有 agent 的基础权限
- 各 agent 可以通过 `PermissionNext.merge()` 覆盖或扩展
- `ask` 策略要求用户确认，适合敏感操作
- `allow` 策略自动通过，适合安全操作

**MCP 工具架构**:
- 工具通过 `Tool.define()` 定义，包含 id、description、parameters、execute
- 注册到 `ToolRegistry` 后自动暴露给 MCP server
- 使用 Zod schema 进行参数验证
`─────────────────────────────────────────────────`
