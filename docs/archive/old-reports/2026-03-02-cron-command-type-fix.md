# Cron Command Type Support Fix

**日期**: 2026-03-02
**状态**: 已完成
**关联 Trace**:
- f0177aa9-d415-4092-afa3-45a5bef8cdcf (agent 命令执行失败)
- e5428794-da08-4e03-b7f1-7e7c448be88c (延迟任务调度到错误年份)

## 问题 1: Agent 命令执行失败

### 根本原因

`zero-workflow` 服务的 `execute_command` 函数将所有命令都当作 shell 命令执行，导致 JSON 格式的 agent/api 命令失败：

```
ERROR: Cron command failed
command={"type":"agent","agent":"general","prompt":"给我发一条随机内容的消息"}
error=Command failed: sh: type:agent: command not found
```

### 修复方案

修改 `services/zero-workflow/src/lib.rs`：

1. 添加 `CronCommand` 枚举，支持 `Agent`、`Api`、`Shell` 三种类型
2. 添加 `execute_cron_command` 路由分发函数
3. 添加 `execute_agent_command` 调用 CodeCoder API
4. 添加 `execute_api_command` 执行 HTTP 请求
5. 更新调度循环，使用 `due_tasks()` 和 `reschedule_after_run()`

## 问题 2: 延迟任务调度到错误年份

### 根本原因

AI 使用 `scheduler_create_task` 创建"1分钟后发消息"任务时，生成的 cron 表达式 `41 2 2 3 *` 表示"每年3月2日 02:41"，由于当前时间已过该时刻，下次执行变成了明年。

```
expression: 41 2 2 3 *
next_run: 2027-03-02T02:41:00+00:00  ← 明年！
```

### 修复方案

新增 `scheduler_delay_task` 工具，专门处理一次性延迟任务：

修改 `packages/ccode/src/tool/scheduler.ts`:
- 新增 `SchedulerDelayTaskTool`
- 支持 `delayMinutes` 和 `delaySeconds` 参数
- 自动计算目标时间并生成精确的 cron 表达式
- 支持简化的 `message` 参数直接发送消息

修改 `packages/ccode/src/tool/registry.ts`:
- 注册新工具 `SchedulerDelayTaskTool`

### 工具使用场景

| 场景 | 使用工具 |
|------|----------|
| "每天早上8点提醒我" | `scheduler_create_task` |
| "1分钟后发消息" | `scheduler_delay_task` ← 新增 |
| "30秒后检查状态" | `scheduler_delay_task` ← 新增 |

## 测试验证

### Agent 命令执行测试

```bash
# 创建 agent 类型任务
curl -X POST http://127.0.0.1:4400/api/v1/scheduler/tasks \
  -d '{"id": "test-agent-task", "expression": "* * * * *",
       "command": {"type": "agent", "agentName": "general", "prompt": "Say hello"}}'

# 结果
{
  "last_run": "2026-03-02T11:47:17.474557Z",
  "last_status": "ok"  // ✅ 成功
}
```

### 延迟任务测试

```bash
# 创建 2 分钟后执行的任务
curl -X POST http://127.0.0.1:4400/api/v1/scheduler/tasks \
  -d '{"id": "test-delay", "expression": "0 01 12 02 03 *",
       "command": {"type": "agent", "agentName": "general", "prompt": "测试"}}'

# 结果
{
  "next_run": "2026-03-02T12:01:00Z",  // ✅ 正确的当天时间
  "last_run": "2026-03-02T12:01:55.101697Z",
  "last_status": "ok"
}
```

## 修改文件

### Rust (zero-workflow)

- `services/zero-workflow/src/lib.rs`
  - 添加 `CronCommand` 枚举
  - 添加 `execute_cron_command` 函数
  - 添加 `execute_agent_command` 函数
  - 添加 `execute_api_command` 函数
  - 重命名 `execute_command` 为 `execute_shell_command`
  - 更新调度循环

### TypeScript (ccode)

- `packages/ccode/src/tool/scheduler.ts`
  - 添加 `SchedulerDelayTaskTool`

- `packages/ccode/src/tool/registry.ts`
  - 注册 `SchedulerDelayTaskTool`

## 后续建议

1. 考虑为延迟任务添加自动清理机制（执行后自动删除）
2. 考虑添加任务执行历史的持久化存储
3. 考虑在 AI 提示中强调 `scheduler_delay_task` 用于一次性任务
