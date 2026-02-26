# CodeCoder 技术架构与能力评估报告

## 上下文

本评估基于对 CodeCoder 项目源代码的深度分析，旨在全面了解项目的技术架构、核心能力和工程质量。

---

## 架构图索引

已生成 6 张架构图，位于 `docs/diagrams/`:

| 图表 | 文件 | 说明 |
|------|------|------|
| 系统总览 | `01-system-overview.svg` | 高层次模块关系 |
| 请求流程 | `02-request-flow.svg` | 用户请求处理管道 |
| Agent 系统 | `03-agent-system.svg` | 23+ Agent 分类架构 |
| 自主模式状态机 | `04-autonomous-state.svg` | Autonomous Mode 状态流转 |
| 数据流 | `05-data-flow.svg` | 输入/配置/存储/输出流向 |
| 提供商集成 | `06-provider-integration.svg` | 20+ AI 提供商 SDK 矩阵 |

**查看方式:**
```bash
# 在浏览器中打开
open docs/diagrams/*.svg

# 或使用 VS Code
code docs/diagrams/
```

---

## 1. 项目概览

**CodeCoder** 是一个融合工程能力与决策智慧的 AI 驱动个人工作台，具有以下核心特征：

| 维度 | 描述 |
|------|------|
| **技术栈** | Bun 1.3+ / TypeScript / Turborepo / Solid.js / Hono |
| **架构风格** | Monorepo + 客户端/服务器分离 |
| **AI 集成** | 多提供商支持 (20+ AI SDK 集成) |
| **交互方式** | CLI / TUI / HTTP API |

---

## 2. Monorepo 结构

```
packages/
├── ccode/           # 核心 CLI 工具 (主包)
│   ├── src/
│   │   ├── agent/        # Agent 定义与服务
│   │   ├── api/          # HTTP API 服务器
│   │   ├── autonomous/   # 自主模式实现
│   │   ├── cli/          # CLI 命令与 TUI
│   │   ├── mcp/          # Model Context Protocol
│   │   ├── provider/     # AI 提供商集成
│   │   ├── storage/      # 存储层
│   │   └── tool/         # 内置工具集
│   └── test/
└── util/            # 共享工具库
```

---

## 3. Agent 系统架构

### 3.1 Agent 定义 (`packages/ccode/src/agent/agent.ts`)

系统支持 **23+ 个预定义 Agent**，分为三类模式：

| 模式 | 说明 | 示例 |
|------|------|------|
| `primary` | 主要开发模式 | build, plan, writer, autonomous |
| `subagent` | 子代理/专业工具 | explore, code-reviewer, tdd-guide |
| `all` | 通用模式 | 自定义 agent |

### 3.2 Agent 分类详情

**主模式 Agent:**
- `build` - 默认开发模式
- `plan` - 计划模式 (最大输出 128K tokens)
- `writer` - 长文写作 (20K+ 字)
- `autonomous` - 完全自主执行模式

**工程质量 Agent:**
- `code-reviewer` - 代码审查
- `security-reviewer` - 安全分析
- `tdd-guide` - TDD 方法论指导
- `architect` - 系统架构设计
- `verifier` - 综合验证 (构建、类型、测试、覆盖率)

**内容创作 Agent:**
- `writer` - 长文写作主模式
- `expander` / `expander-fiction` / `expander-nonfiction` - 内容扩展
- `proofreader` - 校对

**逆向工程 Agent:**
- `code-reverse` - 网站逆向分析
- `jar-code-reverse` - JAR 文件逆向

**祝融说系列 (ZRS):**
- `observer` - 观察者理论分析
- `decision` - CLOSE 五维决策框架
- `macro` - 宏观经济分析
- `trader` - 交易分析
- `picker` - 选品专家
- `miniproduct` - 极小产品教练
- `ai-engineer` - AI 工程师导师

### 3.3 权限系统

基于 `PermissionNext.Ruleset` 的细粒度权限控制：

```typescript
permission: PermissionNext.merge(
  defaults,
  PermissionNext.fromConfig({
    question: "allow",
    plan_enter: "allow",
    external_directory: { ... }
  }),
  user
)
```

---

## 4. AI 提供商集成

### 4.1 支持的提供商 (20+)

| 提供商 | SDK | 特殊功能 |
|--------|-----|----------|
| Anthropic | @ai-sdk/anthropic | claude-code-20250219, interleaved-thinking |
| OpenAI | @ai-sdk/openai | Responses API |
| Google | @ai-sdk/google | Gemini |
| Google Vertex | @ai-sdk/google-vertex | Vertex AI + Anthropic |
| Amazon Bedrock | @ai-sdk/amazon-bedrock | 区域前缀自动处理 |
| Azure | @ai-sdk/azure | Cognitive Services |
| GitHub Copilot | 自定义实现 | Enterprise 支持 |
| OpenRouter | @openrouter/ai-sdk-provider | 多模型路由 |
| xAI, Mistral, Groq, DeepInfra, Cerebras, Cohere, TogetherAI, Perplexity, Vercel | 各自 SDK | - |

### 4.2 模型能力追踪

```typescript
capabilities: {
  temperature: boolean
  reasoning: boolean
  attachment: boolean
  toolcall: boolean
  input: { text, audio, image, video, pdf }
  output: { text, audio, image, video, pdf }
  interleaved: boolean | { field: "reasoning_content" | "reasoning_details" }
}
```

---

## 5. MCP (Model Context Protocol) 支持

### 5.1 传输类型

- **远程 (remote):** StreamableHTTP / SSE
- **本地 (local):** Stdio

### 5.2 OAuth 集成

完整的 OAuth 2.0 PKCE 流程：
- `startAuth()` - 启动认证
- `authenticate()` - 完成认证
- `finishAuth()` - 代码交换
- 支持动态客户端注册

### 5.3 功能

- 工具调用 (`callTool`)
- Prompts 管理
- Resources 读取
- 工具列表变更通知

---

## 6. 自主模式 (Autonomous Mode)

### 6.1 架构组件

```
autonomous/
├── agent/           # 自主代理封装
├── config/          # 配置管理
├── decision/        # 决策引擎 (CLOSE 框架)
├── execution/       # 执行器 (Git, Test Runner)
├── integration/     # 钩子与报告
├── metrics/         # 评分系统
├── orchestration/   # 任务编排
├── safety/          # 安全约束与回滚
└── state/           # 状态机
```

### 6.2 核心能力

- **状态机驱动:** IDLE → RUNNING → PAUSED/COMPLETED
- **决策引擎:** 基于 CLOSE 五维评估法
- **安全护栏:** 约束检查、回滚支持
- **指标追踪:** 质量分数、疯狂度分数、成本统计

---

## 7. HTTP API 服务器

### 7.1 路由体系 (`packages/ccode/src/api/server/router.ts`)

| 端点 | 功能 |
|------|------|
| `/api/sessions` | 会话 CRUD |
| `/api/sessions/:id/messages` | 消息收发 |
| `/api/config` | 配置读写 |
| `/api/permissions` | 权限管理 |
| `/api/files` | 文件搜索 |
| `/api/events` | SSE 事件流 |
| `/api/agents` | Agent 列表与调用 |
| `/api/v1/tasks` | 异步任务流 (ZeroBot 集成) |

### 7.2 任务 API (v1)

```typescript
POST /api/v1/tasks       # 创建任务
GET  /api/v1/tasks       # 列出任务
GET  /api/v1/tasks/:id   # 获取任务详情
GET  /api/v1/tasks/:id/events  # SSE 事件流
POST /api/v1/tasks/:id/interact # 交互
DELETE /api/v1/tasks/:id # 删除
```

---

## 8. 工具系统

### 8.1 内置工具 (25+)

| 类别 | 工具 |
|------|------|
| **文件操作** | read, write, edit, multiedit, glob, grep, ls |
| **代码执行** | bash, task, lsp, codesearch |
| **网络** | webfetch, websearch, network-analyzer |
| **工作流** | plan, question, skill, todo |
| **特殊** | apply_patch, truncation, batch, external-directory |

### 8.2 工具扩展

- 通过 MCP 动态加载外部工具
- 支持工具级权限控制
- 统一的执行超时管理

---

## 9. 存储系统

### 9.1 数据完整性

```typescript
// 自动恢复机制
try {
  return JSON.parse(text)
} catch (parseError) {
  await isolateCorrupted(target, text)
  const restored = await restore(key)
  if (restored) return JSON.parse(recoveredText)
  throw new CorruptedError(...)
}
```

### 9.2 备份策略

- 保留天数: 7 天
- 最大备份数: 3 个
- 自动清理过期备份

### 9.3 健康检查

```typescript
interface HealthReport {
  total: number
  healthy: number
  corrupted: { key: string[]; error: string }[]
  orphaned: string[]
}
```

---

## 10. 内容创作能力

### 10.1 Writer 服务

- 章节草稿管理 (`ChapterDraftManager`)
- 写作统计监控 (`WriterStatsMonitor`)
- 超时监控 (`WriterTimeoutMonitor`)

### 10.2 书籍扩展

- `expander` - 通用扩展
- `expander-fiction` - 小说扩展 (温度 0.8)
- `expander-nonfiction` - 非虚构扩展 (温度 0.6)

---

## 11. 工程质量

### 11.1 代码风格约束

- **不可变性:** 优先 `const`，使用扩展而非修改
- **文件组织:** 200-400 行典型，800 行上限
- **错误处理:** 综合 try/catch + `.catch()`
- **类型安全:** Zod schema 驱动

### 11.2 测试策略

```bash
bun test                    # 全量测试
bun test:tui:coverage       # TUI 覆盖率
bun test:verify             # 验证覆盖率
```

### 11.3 可观测性

- 结构化日志 (JSON 格式)
- 全链路追踪 (trace_id, span_id)
- 执行轨迹报告

---

## 12. 记忆系统

### 12.1 双层架构

| 层 | 路径 | 用途 |
|----|------|------|
| 流层 (日常) | `./memory/daily/{YYYY-MM-DD}.md` | 每日追加日志 |
| 沉积层 (长期) | `./memory/MEMORY.md` | 整理后的知识 |

### 12.2 操作规则

- 即时操作: 追加到每日笔记
- 整合操作: 智能合并到 MEMORY.md
- 透明性: 标准 Markdown，Git 友好

---

## 13. 安全设计

### 13.1 权限分层

1. 默认权限 (Agent 级别)
2. 配置覆盖 (用户级别)
3. 运行时检查 (工具级别)

### 13.2 敏感文件保护

```typescript
read: {
  "*": "allow",
  "*.env": "ask",
  "*.env.*": "ask",
  "*.env.example": "allow"
}
```

---

## 14. 评估总结

### 优势

1. **多模型支持:** 20+ AI 提供商无缝切换
2. **Agent 专业化:** 23+ 专业 Agent 覆盖开发全流程
3. **自主执行:** 完整的自主模式与决策框架
4. **MCP 集成:** 标准协议支持工具扩展
5. **数据可靠:** 备份、恢复、健康检查机制
6. **哲学融合:** 祝融说决策框架贯穿设计

### 待改进领域

1. **测试覆盖:** 部分模块测试待补充
2. **文档同步:** 部分新功能文档滞后
3. **错误上报:** 可增强用户级错误反馈

### 技术债务

参见 `docs/DEBT.md` 追踪已知技术债务。

---

## 验证方法

1. **单元测试:** `cd packages/ccode && bun test`
2. **类型检查:** `bun turbo typecheck`
3. **API 测试:** `bun dev serve` 后使用 HTTP 客户端测试端点
4. **TUI 测试:** `bun dev` 启动交互式界面

---

---

## 15. 深度架构分析

### 15.1 请求处理流程

```
用户输入 → CLI/TUI → Session → LLM.stream() → Provider → AI Model
                              ↓
                         Tool 执行 → 工具结果
                              ↓
                         响应流 → 用户
```

**核心流程组件:**

| 组件 | 位置 | 职责 |
|------|------|------|
| `LLM.stream()` | `session/llm.ts` | 统一的 LLM 流式调用 |
| `SystemPrompt` | `session/system.ts` | 系统提示构建 |
| `Provider.getLanguage()` | `provider/provider.ts` | 模型实例获取 |
| `ProviderTransform` | `provider/transform.ts` | 提供商特定转换 |

### 15.2 配置系统层次

```
┌─────────────────────────────────────────────────┐
│ 优先级 (从低到高)                                 │
├─────────────────────────────────────────────────┤
│ 1. 全局配置 (~/.codecoder/codecoder.jsonc)           │
│ 2. 自定义路径 (CCODE_CONFIG 环境变量)            │
│ 3. 项目配置 (./codecoder.jsonc)                 │
│ 4. 内联配置 (CCODE_CONFIG_CONTENT)              │
│ 5. 运行时 Flag 覆盖                              │
└─────────────────────────────────────────────────┘
```

**配置加载源码:** `config/config.ts:40-162`

**扩展点:**
- `{rule,rules}/**/*.md` - 规则文件
- `{agent,agents}/**/*.md` - Agent 定义
- `{command,commands}/**/*.md` - 命令模板
- `{mode,modes}/*.md` - 模式定义 (已废弃)

### 15.3 会话系统架构

```
Session
├── id: string (ULID)
├── projectID: string
├── messages: MessageV2[]
├── agent: Agent.Info
├── model: Provider.Model
└── state: SessionState

MessageV2
├── User (用户消息)
│   ├── content: string
│   ├── attachments: Attachment[]
│   ├── tools: Record<string, boolean>
│   └── variant: string
└── Assistant (助手消息)
    ├── content: ContentPart[]
    ├── usage: TokenUsage
    └── finishReason: string
```

**会话文件:**
- `session/index.ts` - 主入口
- `session/message-v2.ts` - 消息类型定义
- `session/compaction.ts` - 上下文压缩
- `session/processor.ts` - 消息处理管道

### 15.4 事件总线系统

```typescript
// 定义事件 (bus/bus-event.ts)
BusEvent.define("event.type", z.object({ ... }))

// 发布事件
Bus.publish(EventType, payload)

// 订阅事件
Bus.subscribe(EventType, async (event) => { ... })
```

**核心事件类型:**
- `TuiEvent` - TUI 界面事件
- `AutonomousEvent` - 自主模式事件
- `Session.Event` - 会话生命周期事件
- `MCP.ToolsChanged` - MCP 工具变更

### 15.5 自主模式编排器详解

```typescript
Orchestrator 执行周期:
┌──────────────────────────────────────────────────┐
│ 1. PLANNING → Understand & Plan                  │
│ 2. DECIDING → CLOSE 五维评估                     │
│ 3. EXECUTING → TDD 循环执行                      │
│ 4. TESTING → 运行测试套件                        │
│ 5. VERIFYING → 验证通过                          │
│ 6. EVALUATING → 计算分数                         │
│ 7. CONTINUING/COMPLETED → 循环或结束            │
└──────────────────────────────────────────────────┘

关键配置:
- autonomyLevel: lunatic|insane|crazy|wild|bold|timid
- resourceBudget: { maxTokens, maxCostUSD, maxDuration }
- unattended: boolean (无人值守模式)
```

**安全护栏:**
- `SafetyGuard` - 资源使用监控
- `SafetyIntegration` - doom loop 桥接、破坏性操作保护
- 自动回滚支持

### 15.6 LLM 调用详解

```typescript
// session/llm.ts:53-281
LLM.stream({
  user: MessageV2.User,      // 用户消息
  model: Provider.Model,     // 模型信息
  agent: Agent.Info,         // Agent 配置
  system: string[],          // 系统提示
  messages: ModelMessage[],  // 历史消息
  tools: Record<string, Tool>,// 可用工具
  abort: AbortSignal,        // 中断信号
})

// 中间件链:
1. ProviderTransform.message() - 消息转换
2. extractReasoningMiddleware() - 推理提取
3. experimental_repairToolCall() - 工具调用修复
```

**特殊处理:**
- Gemini 2.5: 强制禁用 thinking 防止工具调用中断
- LiteLLM 代理: 自动添加 `_noop` 占位工具
- 成本追踪: 自动计算 token 使用和费用

### 15.7 工具权限系统

```typescript
// permission/next.ts
PermissionNext.Ruleset = Array<{
  permission: string,   // 权限类型
  pattern: string,      // 匹配模式
  action: "allow" | "deny" | "ask"
}>

// 权限合并优先级:
1. Agent 默认权限
2. 用户配置覆盖
3. 运行时动态检查
```

**支持的权限类型:**
- `read`, `edit`, `glob`, `grep`, `list`, `bash`
- `task`, `external_directory`, `webfetch`, `websearch`
- `question`, `plan_enter`, `plan_exit`, `doom_loop`
- `todowrite`, `todoread`, `lsp`, `codesearch`

### 15.8 系统提示构建

```typescript
// session/system.ts
SystemPrompt.header(providerID)     // 提供商特定头部
SystemPrompt.provider(model)        // 模型特定提示
SystemPrompt.environment(model)     // 环境信息
SystemPrompt.custom()               // 自定义指令 (CLAUDE.md, rules/)
SystemPrompt.markdownMemory()       // 记忆层内容
```

**提示文件加载顺序:**
1. `AGENTS.md` / `CLAUDE.md` / `CONTEXT.md` (项目)
2. `~/.claude/CLAUDE.md` (全局)
3. 配置中的 `instructions` 数组
4. `{rules,rule}/**/*.md` 目录

---

## 16. 数据流架构

### 16.1 请求-响应数据流

```
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐
│  User   │───▶│   CLI   │───▶│ Session  │───▶│    LLM   │
└─────────┘    └─────────┘    └──────────┘    └──────────┘
                                   │                │
                                   ▼                ▼
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐
│  TUI    │◀───│   Bus   │◀───│Processor │◀───│ Tool Exec│
└─────────┘    └─────────┘    └──────────┘    └──────────┘
```

### 16.2 存储数据流

```
内存状态 ←→ Storage Module ←→ 文件系统 (JSON)
              │
              ├── 自动备份 (_backup/)
              ├── 损坏隔离 (_corrupted/)
              └── 健康检查
```

### 16.3 事件数据流

```
组件 A ──publish──▶ Bus ──subscribe──▶ 组件 B
                     │
                     └── SSE ──stream──▶ HTTP Client
```

---

## 17. 扩展性设计

### 17.1 Agent 扩展

```
~/.codecoder/agents/my-agent.md
---
description: "自定义 Agent"
model: anthropic/claude-sonnet-4-20250514
temperature: 0.7
---
# Agent 系统提示
```

### 17.2 命令扩展

```
~/.codecoder/commands/my-command.md
---
description: "自定义命令"
agent: build
---
执行模板: {arg1} {arg2}
```

### 17.3 MCP 工具扩展

```jsonc
{
  "mcp": {
    "my-server": {
      "type": "local",
      "command": ["node", "server.js"]
    }
  }
}
```

### 17.4 提供商扩展

```jsonc
{
  "provider": {
    "custom-provider": {
      "api": "https://api.example.com/v1",
      "npm": "@ai-sdk/openai-compatible",
      "models": {
        "custom-model": {
          "limit": { "context": 128000, "output": 8192 }
        }
      }
    }
  }
}
```

---

## 18. 性能特征

| 特征 | 值/描述 |
|------|---------|
| **启动时间** | < 1s (热启动) |
| **模型切换** | 懒加载 SDK |
| **并发请求** | 支持多 Session |
| **流式输出** | 实时 token 流 |
| **上下文压缩** | 自动/手动 compaction |
| **缓存** | SDK 实例、提供商状态 |

---

## 19. 依赖架构

### 19.1 核心依赖

| 类别 | 依赖 | 用途 |
|------|------|------|
| AI SDK | ai (Vercel) | 统一 LLM 接口 |
| MCP | @modelcontextprotocol/sdk | 工具协议 |
| UI | solid-js, @opentui/* | TUI 渲染 |
| 验证 | zod | Schema 验证 |
| 工具 | remeda | 函数式工具 |

### 19.2 提供商 SDK 矩阵

```
@ai-sdk/anthropic    @ai-sdk/openai       @ai-sdk/google
@ai-sdk/azure        @ai-sdk/amazon-bedrock @ai-sdk/google-vertex
@ai-sdk/xai          @ai-sdk/mistral      @ai-sdk/groq
@ai-sdk/deepinfra    @ai-sdk/cerebras     @ai-sdk/cohere
@ai-sdk/togetherai   @ai-sdk/perplexity   @ai-sdk/vercel
@ai-sdk/gateway      @openrouter/ai-sdk-provider
```

---

## 20. 架构亮点

1. **模块化设计:** 清晰的模块边界，低耦合
2. **类型驱动:** Zod schema 作为类型源和验证源
3. **事件驱动:** 松耦合的组件通信
4. **懒加载:** SDK 和状态按需初始化
5. **可观测性:** 结构化日志和追踪
6. **容错设计:** 自动备份、恢复、健康检查
7. **哲学融合:** CLOSE 决策框架内置于自主模式

---

*报告生成时间: 2026-02-16*
