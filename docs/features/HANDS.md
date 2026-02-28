# Hands 系统用户指南

Hands 是 CodeCoder 的自动化任务系统，允许用户定义定时或触发式执行的 Agent 任务。每个 Hand 是一个独立的任务单元，配置在 `HAND.md` 文件中。

## 概述

### 什么是 Hands？

Hands（手）是自动化的 Agent 任务，可以：
- 按照 Cron 调度定时执行
- 响应 Webhook、Git 事件或文件变化触发
- 使用 CLOSE 决策框架进行自主决策
- 管理资源限制和成本控制
- 记录执行历史和输出

### 核心概念

| 概念 | 说明 |
|------|------|
| Hand | 一个自动化任务单元 |
| HAND.md | Hand 的配置文件（YAML frontmatter + Markdown） |
| Autonomy Level | 自主级别，决定 Agent 的自主程度 |
| CLOSE Framework | 决策框架，用于评估和执行决策 |
| Memory Path | 执行结果的存储路径模板 |

## HAND.md 格式

每个 Hand 使用 `HAND.md` 文件定义，采用 YAML frontmatter 配置 + Markdown 文档的格式：

```yaml
---
# 必填字段
id: "market-sentinel"           # 唯一标识符
schedule: "0 */30 * * * *"      # Cron 表达式 (6 字段: 秒 分 时 日 月 周)
agent: "macro"                   # 使用的 Agent

# 可选字段
name: "Market Sentinel"          # 显示名称 (默认: "Unnamed Hand")
version: "1.0.0"                 # 版本号 (默认: "1.0.0")
enabled: true                    # 是否启用 (默认: true)
memory_path: "hands/{id}/{date}.md"  # 输出路径模板

# 自主配置 (可选)
autonomy:
  level: "crazy"                 # lunatic/insane/crazy/wild/bold/timid
  unattended: true               # 无人值守模式
  max_iterations: 5              # 最大迭代次数
  auto_approve:                  # 自动批准配置
    enabled: false
    allowed_tools: []
    risk_threshold: "medium"     # safe/low/medium/high
    timeout_ms: 30000

# 决策配置 (可选)
decision:
  use_close: true                # 使用 CLOSE 框架
  web_search: true               # 启用网络搜索
  evolution: false               # 启用进化循环
  auto_continue: true            # 自动继续执行

# 资源限制 (可选)
resources:
  max_tokens: 100000             # 最大 Token 数
  max_cost_usd: 5.0              # 最大成本 (美元)
  max_duration_sec: 600          # 最大执行时长 (秒)

# 自定义参数 (可选)
params:
  threshold: 0.7
  symbols: ["BTC", "ETH"]
---

# Market Sentinel

每 30 分钟分析全球宏观经济数据，生成市场简报。

## 职责

1. 收集最新的经济指标数据
2. 分析市场趋势和风险
3. 生成简明的市场报告

## 输出格式

报告将保存到 `memory/hands/market-sentinel/{date}.md`
```

## 自主级别 (Autonomy Level)

自主级别决定 Hand 执行时的决策阈值：

| 级别 | 分数范围 | 批准阈值 | 说明 |
|------|---------|---------|------|
| `lunatic` | 90+ | 5.0 | 完全自主 - 无需人工干预 |
| `insane` | 75-89 | 5.5 | 高度自主 - 关键决策前通知 |
| `crazy` | 60-74 | 6.0 | 显著自主 - 半自动执行 |
| `wild` | 40-59 | 6.5 | 部分自主 - 仅执行简单任务 |
| `bold` | 20-39 | 7.0 | 谨慎自主 - 仅执行已定义步骤 |
| `timid` | <20 | 8.0 | 基本不自主 - 仅收集信息 |

## Agent 管道 (Pipeline)

Hand 支持多 Agent 协作的管道执行模式。使用 `agents` 和 `pipeline` 字段配置：

```yaml
---
id: "research-pipeline"
name: "研究管道"
agents:
  - explore      # 第1步：探索
  - general      # 第2步：分析
  - writer       # 第3步：报告
pipeline: "sequential"
---
```

### 管道模式

| 模式 | 说明 | 适用场景 |
|------|------|----------|
| `sequential` | 顺序执行，前一个 Agent 的输出作为下一个的输入 | 研究流程、内容创作、代码审查 |
| `parallel` | 并行执行，合并所有 Agent 的输出 | 多角度分析、多源数据收集 |
| `conditional` | 根据 CLOSE 框架评估决定下一个 Agent | 决策树、异常处理 |

### Sequential 示例

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   explore   │ ───▶ │   general   │ ───▶ │   writer    │
│  (探索发现)  │      │  (深度研究)  │      │  (报告生成)  │
└─────────────┘      └─────────────┘      └─────────────┘
```

每个 Agent 会接收到前一个 Agent 的输出作为上下文。

### Parallel 示例

```yaml
agents:
  - security-reviewer
  - code-reviewer
  - architect
pipeline: "parallel"
```

所有 Agent 同时执行，各自独立分析，最后合并为一份综合报告。

## 触发类型

### Cron 调度

使用 6 字段 Cron 表达式（秒 分 时 日 月 周）：

```yaml
schedule: "0 */30 * * * *"    # 每 30 分钟
schedule: "0 0 9 * * 1-5"     # 工作日 9:00
schedule: "0 0 0 1 * *"       # 每月 1 日 0:00
```

### Webhook 触发 (规划中)

```yaml
trigger:
  type: "webhook"
  path: "/api/hands/market-sentinel/trigger"
  method: "POST"
```

### Git 事件触发 (规划中)

```yaml
trigger:
  type: "git"
  repo: "org/repo"
  events: ["push", "pull_request"]
```

### 文件监控触发 (规划中)

```yaml
trigger:
  type: "file_watch"
  patterns: ["*.md", "src/**/*.ts"]
```

## 目录结构

Hands 存储在 `~/.codecoder/hands/` 目录下：

```
~/.codecoder/hands/
├── market-sentinel/
│   └── HAND.md
├── daily-report/
│   └── HAND.md
└── code-review/
    └── HAND.md
```

每个 Hand 可以是：
- 子目录中的 `HAND.md` 文件
- 直接在 `hands/` 目录下的 `{id}.md` 文件

## TypeScript API

### 使用 HandsBridge

```typescript
import { HandsBridge, triggerHands, listHands } from "@/autonomous/hands"

// 创建客户端
const bridge = new HandsBridge({
  baseUrl: "http://127.0.0.1:4432",
  timeoutMs: 30000
})

// 列出所有 Hands
const hands = await bridge.list()

// 获取特定 Hand
const hand = await bridge.get("market-sentinel")

// 手动触发执行
const response = await bridge.trigger({
  handId: "market-sentinel",
  params: { symbol: "BTC" }
})

// 获取执行状态
const execution = await bridge.getExecution(response.executionId)

// 启用/禁用 Hand
await bridge.enable("market-sentinel")
await bridge.disable("market-sentinel")

// 便捷函数
await triggerHands("market-sentinel", { symbol: "ETH" })
const allHands = await listHands()
```

### 调度器控制

```typescript
// 获取调度器状态
const status = await bridge.getSchedulerStatus()
console.log(`Running: ${status.running}`)
console.log(`Active Hands: ${status.activeHands}`)
console.log(`Next: ${status.nextExecution?.handId}`)

// 启动/停止调度器
await bridge.startScheduler()
await bridge.stopScheduler()
```

### 执行管理

```typescript
// 列出历史执行
const executions = await bridge.listExecutions("market-sentinel", 10)

// 暂停/恢复/取消执行
await bridge.pauseExecution(executionId)
await bridge.resumeExecution(executionId)
await bridge.cancelExecution(executionId)
```

## HTTP API

Hand 服务运行在 `zero-workflow` (端口 4432)：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/hands` | GET | 列出所有 Hands |
| `/api/v1/hands/:id` | GET | 获取 Hand 详情 |
| `/api/v1/hands` | POST | 注册新 Hand |
| `/api/v1/hands/:id` | PATCH | 更新 Hand 配置 |
| `/api/v1/hands/:id` | DELETE | 删除 Hand |
| `/api/v1/hands/:id/trigger` | POST | 手动触发执行 |
| `/api/v1/hands/:id/executions` | GET | 列出执行历史 |
| `/api/v1/hands/executions/:id` | GET | 获取执行详情 |
| `/api/v1/hands/scheduler/status` | GET | 调度器状态 |
| `/api/v1/hands/scheduler/start` | POST | 启动调度器 |
| `/api/v1/hands/scheduler/stop` | POST | 停止调度器 |

## 可用 Agent

Hand 可以使用以下 Agent：

| Agent | 用途 |
|-------|------|
| `macro` | 宏观经济分析 |
| `trader` | 交易决策 |
| `picker` | 选品分析 |
| `code-reviewer` | 代码审查 |
| `security-reviewer` | 安全分析 |
| `explore` | 代码库探索 |
| `general` | 通用任务 |
| `writer` | 长文写作 |
| `decision` | CLOSE 决策分析 |

## 示例

### 每日市场回顾

```yaml
---
id: "daily-market-review"
name: "每日市场回顾"
schedule: "0 0 9 * * 1-5"
agent: "macro"
enabled: true
autonomy:
  level: "crazy"
  unattended: true
decision:
  use_close: true
  web_search: true
resources:
  max_tokens: 50000
  max_cost_usd: 2.0
memory_path: "hands/market-review/{date}.md"
---

# 每日市场回顾

每个工作日早上 9:00 生成市场分析报告。
```

### 周代码审计

```yaml
---
id: "weekly-code-audit"
name: "周代码审计"
schedule: "0 0 10 * * 1"
agent: "security-reviewer"
enabled: true
autonomy:
  level: "bold"
decision:
  use_close: true
resources:
  max_tokens: 100000
  max_duration_sec: 1800
memory_path: "hands/code-audit/{date}.md"
params:
  directories: ["src", "services"]
---

# 周代码审计

每周一 10:00 进行安全代码审计。
```

## 故障排除

### Hand 未执行

1. 检查 `enabled` 是否为 `true`
2. 验证 Cron 表达式格式
3. 确认 `zero-workflow` 服务正在运行
4. 查看服务日志：`./ops.sh logs zero-workflow`

### 执行失败

1. 检查资源限制是否过低
2. 验证 Agent 名称正确
3. 查看执行历史获取错误详情

### 调度器问题

```bash
# 检查调度器状态
curl http://localhost:4432/api/v1/hands/scheduler/status

# 重启调度器
./ops.sh restart zero-workflow
```

## 相关文档

- [HITL 审批系统](./HITL.md) - Human-in-the-Loop 审批队列
- [Agent 架构](../architecture/README.md) - Agent 系统概述
