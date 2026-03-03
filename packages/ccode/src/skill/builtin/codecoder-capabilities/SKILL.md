---
name: codecoder-capabilities
description: CodeCoder 内置能力查询指南 - 包括 Hands、Scheduler、API、Agent 和工具的完整使用方法
---

# CodeCoder 内置能力指南

本文档描述 CodeCoder 的所有内置能力及其使用方法。当用户询问系统能力、定时任务、workflow 等问题时，使用本指南获取实时数据。

## 核心原则

**CRITICAL**: 查询系统状态时，必须调用 API 获取实时数据，不要依赖记忆或猜测。

## 一、Hands 系统 (自动化任务)

Hands 是 CodeCoder 的自动化 Agent 任务系统，运行在 `zero-workflow` 服务 (端口 4432)。

### 查询已注册的 Hands

```bash
# 使用 Bash 工具执行
curl -s http://localhost:4432/api/v1/hands | jq '.'
```

### Hands API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/hands` | GET | 列出所有已注册的 Hands |
| `/api/v1/hands/:id` | GET | 获取指定 Hand 详情 |
| `/api/v1/hands/:id/trigger` | POST | 手动触发执行 |
| `/api/v1/hands/:id/executions` | GET | 列出执行历史 |
| `/api/v1/hands/scheduler/status` | GET | 调度器状态 |

### Hands 存储位置

- **用户 Hands**: `~/.codecoder/workspace/hands/{hand-id}/HAND.md`
- **示例 Hands**: `example/hands/` (代码库内)

### 创建新 Hand

Hand 使用 `HAND.md` 文件定义 (YAML frontmatter + Markdown):

```yaml
---
id: "my-task"
name: "我的任务"
schedule: "0 0 9 * * 1-5"  # 6字段 Cron: 秒 分 时 日 月 周
agent: "macro"              # 使用的 Agent
enabled: true
memory_path: "hands/my-task/{date}.md"

autonomy:
  level: "crazy"           # lunatic/insane/crazy/wild/bold/timid
  unattended: true

decision:
  use_close: true
  web_search: true

resources:
  max_tokens: 100000
  max_cost_usd: 5.0
  max_duration_sec: 600
---

# 任务描述

这里写任务的详细说明...
```

### 可用于 Hands 的 Agent

| Agent | 用途 |
|-------|------|
| `macro` | 宏观经济分析 |
| `trader` | 交易决策 |
| `picker` | 选品分析 |
| `value-analyst` | 价值分析 |
| `ai-engineer` | AI 工程学习 |
| `miniproduct` | 极小产品追踪 |
| `code-reviewer` | 代码审查 |
| `security-reviewer` | 安全分析 |
| `verifier` | 代码健康检查 |
| `writer` | 长文写作 |
| `general` | 通用任务 |

---

## 二、Scheduler 系统 (通用定时任务)

Scheduler 是通用定时任务系统，运行在 CodeCoder API 服务 (端口 4400)。

### 查询定时任务

**方法 1**: 使用 `scheduler_list_tasks` 工具
```
直接调用 scheduler_list_tasks 工具即可
```

**方法 2**: 使用 Bash 调用 API
```bash
curl -s http://localhost:4400/api/v1/scheduler/tasks | jq '.'
```

### Scheduler 工具

| 工具名 | 说明 |
|--------|------|
| `scheduler_create_task` | 创建定时任务 |
| `scheduler_list_tasks` | 列出所有任务 |
| `scheduler_delete_task` | 删除任务 |
| `scheduler_run_task` | 立即执行任务 |

### 创建定时任务示例

```
使用 scheduler_create_task 工具:
  id: "daily-news"
  expression: "0 8 * * *"        # 每天 8:00
  agentName: "macro"
  prompt: "获取今日财经要闻"
  description: "每日财经新闻"
```

### Cron 表达式示例

| 表达式 | 说明 |
|--------|------|
| `0 8 * * *` | 每天 8:00 |
| `0 9 * * 1-5` | 工作日 9:00 |
| `*/30 * * * *` | 每 30 分钟 |
| `0 0 1 * *` | 每月 1 日 0:00 |

---

## 三、服务架构

### 端口配置

| 服务 | 端口 | 说明 |
|------|------|------|
| CodeCoder API | 4400 | 主 API (Scheduler, Chat, Task) |
| Web Frontend | 4401 | React 前端 |
| Zero Gateway | 4430 | 统一网关 |
| Zero Channels | 4431 | IM 渠道 (Telegram/Discord) |
| Zero Workflow | 4432 | Hands 系统 |
| Zero Browser | 4433 | 浏览器自动化 |

### 健康检查

```bash
# 检查所有服务状态
curl -s http://localhost:4430/health  # Gateway
curl -s http://localhost:4431/health  # Channels
curl -s http://localhost:4432/health  # Workflow
```

---

## 四、可用 Agent 列表

### 主模式 Agent

| Agent | 说明 | 使用场景 |
|-------|------|----------|
| `build` | 构建模式 | 默认开发模式 |
| `plan` | 计划模式 | 复杂功能规划 |
| `autonomous` | 自主模式 | 完全自主执行 |
| `writer` | 写作模式 | 长文写作 |

### 领域 Agent (祝融说系列)

| Agent | 说明 | 触发方式 |
|-------|------|----------|
| `macro` | 宏观经济分析 | `@macro` |
| `trader` | 交易决策 | `@trader` |
| `picker` | 选品分析 | `@picker` |
| `decision` | CLOSE 决策分析 | `@decision` |
| `miniproduct` | 极小产品 | `@miniproduct` |
| `ai-engineer` | AI 工程 | `@ai-engineer` |
| `value-analyst` | 价值分析 | `@value-analyst` |
| `observer` | 观察者理论 | `@observer` |

### 工程 Agent

| Agent | 说明 | 触发方式 |
|-------|------|----------|
| `code-reviewer` | 代码审查 | `@code-reviewer` |
| `security-reviewer` | 安全审查 | `@security-reviewer` |
| `tdd-guide` | TDD 指导 | `@tdd-guide` |
| `architect` | 架构设计 | `@architect` |
| `explore` | 代码探索 | `@explore` |
| `general` | 通用任务 | `@general` |

### 逆向工程 Agent

| Agent | 说明 |
|-------|------|
| `code-reverse` | 代码逆向分析 |
| `jar-code-reverse` | JAR 文件逆向 |

---

## 五、内置工具列表

### 文件操作

| 工具 | 说明 |
|------|------|
| `Read` | 读取文件 |
| `Write` | 写入文件 |
| `Edit` | 编辑文件 |
| `Glob` | 文件搜索 |
| `Grep` | 内容搜索 |

### 系统操作

| 工具 | 说明 |
|------|------|
| `Bash` | 执行命令 |
| `Task` | 启动子 Agent |
| `WebFetch` | 获取网页 |
| `WebSearch` | 网络搜索 |

### 定时任务

| 工具 | 说明 |
|------|------|
| `scheduler_create_task` | 创建定时任务 |
| `scheduler_list_tasks` | 列出定时任务 |
| `scheduler_delete_task` | 删除定时任务 |
| `scheduler_run_task` | 立即执行任务 |

### 其他

| 工具 | 说明 |
|------|------|
| `TodoWrite` | 任务追踪 |
| `TodoRead` | 读取任务 |
| `Skill` | 调用技能 |
| `CodeSearch` | 代码搜索 |
| `NetworkAnalyzer` | 网络分析 |

---

## 六、常见查询场景

### 场景 1: 查询所有定时任务

```bash
# 查询 Hands (workflow 自动化)
curl -s http://localhost:4432/api/v1/hands | jq '.data[] | {id, name, schedule, agent, enabled}'

# 查询 Scheduler (通用定时)
curl -s http://localhost:4400/api/v1/scheduler/tasks | jq '.data[] | {id, name, expression}'
```

### 场景 2: 手动触发 Hand

```bash
curl -X POST http://localhost:4432/api/v1/hands/market-sentinel/trigger
```

### 场景 3: 查看 Hand 执行历史

```bash
curl -s http://localhost:4432/api/v1/hands/daily-health-check/executions | jq '.'
```

### 场景 4: 检查服务健康状态

```bash
for port in 4430 4431 4432; do
  echo "Port $port: $(curl -s http://localhost:$port/health)"
done
```

---

## 七、Hands vs Scheduler 对比

| 特性 | Hands | Scheduler |
|------|-------|-----------|
| 服务 | zero-workflow (4432) | CodeCoder API (4400) |
| 配置方式 | HAND.md 文件 | API/工具创建 |
| 自主级别 | 支持 (6 级) | 不支持 |
| CLOSE 框架 | 支持 | 不支持 |
| Agent 支持 | 完整支持 | 仅调用 |
| 资源限制 | 支持 | 不支持 |
| 执行历史 | 完整记录 | 基础记录 |

**选择建议**:
- **复杂自主任务**: 使用 Hands
- **简单定时调用**: 使用 Scheduler

---

## 八、故障排除

### Hand 不执行

1. 检查 `enabled: true`
2. 验证 Cron 表达式
3. 确认 zero-workflow 运行: `curl http://localhost:4432/health`

### Scheduler 任务失败

1. 检查 CodeCoder API: `curl http://localhost:4400/health`
2. 使用 `scheduler_list_tasks` 查看状态
3. 检查日志: `~/.codecoder/logs/api.log`

### 服务未响应

```bash
# 检查服务状态
./ops.sh status

# 重启服务
./ops.sh restart
```
