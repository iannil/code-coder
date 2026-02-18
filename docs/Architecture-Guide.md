# CodeCoder 项目架构与实现深度解析

> 本文档面向有一定开发基础但不了解本项目的开发者，详细介绍 CodeCoder 的组织结构、实现思路和底层设计思想。

## 目录

1. [快速开始](#快速开始)
2. [项目概述](#1-项目概述)
3. [技术栈](#2-技术栈)
4. [Monorepo 结构](#3-monorepo-结构)
5. [核心架构设计](#4-核心架构设计)
6. [会话系统详解](#5-会话系统详解)
7. [工具系统](#6-工具系统)
8. [权限系统](#7-权限系统)
9. [Agent 系统](#8-agent-系统)
10. [LLM 集成层](#9-llm-集成层)
11. [数据流分析](#10-数据流分析)
12. [开发工作流](#11-开发工作流)
13. [扩展开发](#12-扩展开发)
14. [总结](#13-总结)

---

## 快速开始

```bash
# 克隆并启动 (3 个命令)
git clone https://github.com/iannil/code-coder.git
cd codecoder && bun install
bun dev  # 启动 TUI
```

**常用开发命令**:

```bash
# 运行测试
cd packages/ccode && bun test

# 类型检查
bun turbo typecheck

# 构建
bun run --cwd packages/ccode build
```

---

## 1. 项目概述

CodeCoder 是一个开源 AI 编程代理（AI Coding Agent），其核心理念是**让 AI 像人类开发者一样工作**——能够阅读代码、执行命令、编辑文件、搜索网络，并在需要时请求用户授权。

### 1.1 核心特性

- **多 AI 提供商支持**: Claude、OpenAI、Google Gemini、本地模型等 20+ 提供商
- **多平台支持**: CLI TUI、Web、桌面端
- **客户端/服务器架构**: 支持远程操作和多客户端连接
- **MCP 协议支持**: 可扩展的工具生态系统
- **细粒度权限控制**: 用户对每个操作有完全控制权

### 1.2 设计哲学

```
用户请求 → Agent 理解 → 工具调用 → 权限检查 → 执行 → 结果反馈
```

整个系统围绕**会话（Session）** 组织，每个会话是一个完整的对话上下文，包含消息历史、工具调用记录、权限状态等。

---

## 2. 技术栈

| 层次 | 技术选型 | 说明 |
|-----|---------|------|
| **运行时** | Bun 1.3+ | 高性能 JavaScript 运行时 |
| **构建系统** | Turborepo | Monorepo 任务编排 |
| **HTTP 框架** | Hono | 轻量级、高性能 |
| **前端框架** | Solid.js | 响应式 UI 框架 |
| **终端 UI** | OpenTUI + Solid.js | 基于 Solid 的终端渲染 |
| **AI SDK** | Vercel AI SDK | 统一的 AI 提供商接口 |
| **类型验证** | Zod | 运行时类型验证 |
| **代码搜索** | ripgrep (rg) | 高性能文本搜索 |

---

## 3. Monorepo 结构

```
code-coder/
├── packages/
│   ├── ccode/              # 核心 CLI 工具（主要业务逻辑）
│   │   ├── src/
│   │   │   ├── index.ts    # CLI 入口
│   │   │   ├── agent/      # Agent 定义和管理
│   │   │   ├── session/    # 会话管理核心
│   │   │   ├── tool/       # 工具系统
│   │   │   ├── permission/ # 权限系统
│   │   │   ├── provider/   # LLM 提供商抽象
│   │   │   ├── server/     # HTTP 服务器
│   │   │   ├── mcp/        # MCP 协议集成
│   │   │   ├── cli/cmd/    # CLI 命令
│   │   │   └── cli/cmd/tui/# 终端 UI
│   │   └── package.json
│   └── util/               # 共享工具库
├── script/                 # 构建和发布脚本
├── scripts/                # 项目级工具脚本
└── package.json            # 根配置（workspaces）
```

### 3.1 包依赖关系

```
┌──────────────┐
│   ccode      │ ← 核心包，依赖 util
├──────────────┤
│   util       │ ← 共享工具（错误处理等）
└──────────────┘
```

### 3.2 关键目录说明

| 目录 | 用途 |
|-----|------|
| `packages/ccode/src/agent/` | Agent 定义、权限配置、系统提示 |
| `packages/ccode/src/session/` | 会话管理、消息处理、LLM 调用 |
| `packages/ccode/src/tool/` | 工具定义、注册、执行 |
| `packages/ccode/src/permission/` | 权限评估、请求、响应 |
| `packages/ccode/src/provider/` | AI 提供商抽象、模型配置 |
| `packages/ccode/src/server/` | HTTP API、SSE 事件、路由 |
| `packages/ccode/src/mcp/` | MCP 协议客户端、工具桥接 |
| `packages/ccode/src/cli/cmd/tui/` | 终端 UI 组件和渲染 |

---

## 4. 核心架构设计

### 4.1 实例管理模式 (Instance Pattern)

这是整个项目最核心的设计模式。每个工作目录对应一个 `Instance`，管理该目录下的所有状态。

**文件**: `packages/ccode/src/project/instance.ts`

```typescript
// 伪代码展示核心思想
export namespace Instance {
  // 缓存：目录 → 实例
  const cache = new Map<string, InstanceData>()

  // 获取当前实例（基于工作目录）
  export function current(): InstanceData {
    const dir = process.cwd()
    if (!cache.has(dir)) {
      cache.set(dir, createInstance(dir))
    }
    return cache.get(dir)
  }

  // 状态工厂：懒加载 + 自动清理
  export function state<T>(factory: () => Promise<T>): () => Promise<T> {
    let cached: T | undefined
    return async () => {
      if (!cached) cached = await factory()
      return cached
    }
  }
}
```

**设计意图**:
- 每个项目目录有独立的状态空间
- 懒加载避免不必要的初始化开销
- 实例销毁时自动清理相关状态

### 4.2 配置系统层级

配置遵循**从低到高优先级合并**的策略：

```
1. 远程 well-known 配置 (.well-known/codecoder)
   ↓ 合并
2. 全局用户配置 (~/.codecoder/config.jsonc)
   ↓ 合并
3. 自定义配置路径 (CCODE_CONFIG 环境变量)
   ↓ 合并
4. 项目配置 (.codecoder/ 或 .codecoder/ 目录)
   ↓ 合并
5. 环境变量覆盖 (CCODE_CONFIG_CONTENT)
```

**文件**: `packages/ccode/src/config/config.ts`

配置文件使用 JSONC 格式（支持注释的 JSON），支持深度合并和数组连接。

### 4.3 事件总线 (Bus)

项目使用发布-订阅模式进行组件间通信：

```typescript
namespace Bus {
  // 发布事件
  export function publish<T>(event: BusEvent<T>, data: T)

  // 订阅事件
  export function subscribe<T>(event: BusEvent<T>, handler: (data: T) => void)
}
```

主要事件类型：
- `session.*` - 会话相关事件
- `permission.*` - 权限请求/响应事件
- `mcp.*` - MCP 工具变更事件

---

## 5. 会话系统详解

会话系统是 CodeCoder 的心脏，负责管理用户与 AI 的完整对话流程。

### 5.1 会话生命周期

```
创建会话 → 接收用户消息 → 调用 LLM → 解析工具调用 →
权限检查 → 执行工具 → 处理结果 → 返回给 LLM →
(循环直到完成) → 会话归档
```

### 5.2 消息结构 (MessageV2)

**文件**: `packages/ccode/src/session/message-v2.ts`

```typescript
namespace MessageV2 {
  // 消息部分类型
  type Part =
    | TextPart      // 文本内容
    | ToolPart      // 工具调用
    | ReasoningPart // 推理过程（o1 等模型）
    | FilePart      // 文件附件
    | StepStartPart // 步骤开始标记
    | StepFinishPart// 步骤结束标记
    | PatchPart     // 文件变更记录

  // 用户消息
  interface User {
    id: string
    sessionID: string
    role: "user"
    content: Part[]
    system?: string  // 可选的系统提示
    variant?: string // 模型变体
  }

  // AI 助手消息
  interface Assistant {
    id: string
    sessionID: string
    role: "assistant"
    agent: string    // 使用的 Agent 名称
    cost: number     // 成本统计
    tokens: { input: number; output: number }
    error?: Error    // 错误信息
    finish?: string  // 完成原因
  }
}
```

### 5.3 会话处理器 (SessionProcessor)

**文件**: `packages/ccode/src/session/processor.ts`

核心流程分析：

```typescript
export namespace SessionProcessor {
  export function create(input: {
    assistantMessage: MessageV2.Assistant
    sessionID: string
    model: Provider.Model
    abort: AbortSignal
  }) {
    return {
      async process(streamInput: LLM.StreamInput) {
        while (true) {
          // 1. 创建 LLM 流
          const stream = await LLM.stream(streamInput)

          for await (const value of stream.fullStream) {
            switch (value.type) {
              // 2. 处理文本流
              case "text-delta":
                await Session.updatePart({ part: currentText, delta: value.text })
                break

              // 3. 处理工具调用
              case "tool-call":
                // 检测死循环（相同工具+参数连续调用3次）
                if (isDoomLoop(value)) {
                  await PermissionNext.ask({ permission: "doom_loop", ... })
                }
                break

              // 4. 处理工具结果
              case "tool-result":
                await Session.updatePart({ state: { status: "completed", ... } })
                break

              // 5. 处理错误
              case "tool-error":
                if (value.error instanceof PermissionNext.RejectedError) {
                  blocked = true // 用户拒绝权限，停止循环
                }
                break

              // 6. 步骤完成
              case "finish-step":
                // 计算 token 使用量和成本
                // 检查是否需要压缩历史
                if (needsCompaction) return "compact"
                break
            }
          }

          // 7. 决定下一步
          if (blocked) return "stop"
          if (error) return "stop"
          return "continue"
        }
      }
    }
  }
}
```

**关键设计点**:

1. **死循环检测 (Doom Loop Detection)**: 检测 AI 是否陷入重复相同工具调用的循环（连续 3 次相同工具+参数）
2. **自动压缩 (Compaction)**: 当 token 使用超过阈值时，自动压缩对话历史
3. **快照 (Snapshot)**: 每个步骤记录文件系统快照，支持回滚
4. **重试机制**: 遇到可重试错误（如 rate limit）时自动重试

---

## 6. 工具系统

工具系统让 AI 能够与外部世界交互——读写文件、执行命令、搜索代码等。

### 6.1 工具定义接口

**文件**: `packages/ccode/src/tool/tool.ts`

```typescript
namespace Tool {
  // 工具执行上下文
  interface Context {
    sessionID: string
    messageID: string
    agent: string
    abort: AbortSignal
    callID?: string
    // 更新元数据（显示给用户）
    metadata(input: { title?: string; metadata?: any }): void
    // 请求权限
    ask(input: PermissionRequest): Promise<void>
  }

  // 工具信息
  interface Info<P extends z.ZodType, M extends Metadata> {
    id: string
    init: (ctx?: InitContext) => Promise<{
      description: string          // 给 LLM 看的描述
      parameters: P                // Zod schema 定义参数
      execute(args, ctx): Promise<{
        title: string              // 执行结果标题
        metadata: M                // 元数据
        output: string             // 输出内容（给 LLM）
        attachments?: FilePart[]   // 附件
      }>
    }>
  }

  // 定义工具的辅助函数
  export function define<P, M>(id: string, init: ...): Info<P, M> {
    return {
      id,
      init: async (initCtx) => {
        const toolInfo = await init(initCtx)

        // 包装 execute 函数，添加通用逻辑
        const originalExecute = toolInfo.execute
        toolInfo.execute = async (args, ctx) => {
          // 1. 参数验证
          toolInfo.parameters.parse(args)

          // 2. PreToolUse Hook
          const preResult = await Hook.run("PreToolUse", { tool: id, input: args })
          if (preResult.blocked) throw new HookBlockedError(...)

          // 3. 执行工具
          const result = await originalExecute(args, ctx)

          // 4. PostToolUse Hook
          const postResult = await Hook.run("PostToolUse", { tool: id, output: result })
          if (postResult.blocked) throw new HookBlockedError(...)

          // 5. 输出截断（防止超长输出）
          return Truncate.output(result)
        }

        return toolInfo
      }
    }
  }
}
```

### 6.2 内置工具列表

| 工具 | 功能 | 权限类型 |
|-----|------|---------|
| `read` | 读取文件 | read |
| `write` | 写入文件 | edit |
| `edit` | 编辑文件 | edit |
| `glob` | 文件模式匹配 | glob |
| `grep` | 内容搜索 | grep |
| `bash` | 执行命令 | bash |
| `webfetch` | 获取网页 | webfetch |
| `websearch` | 网络搜索 | websearch |
| `task` | 启动子 Agent | task |
| `question` | 询问用户 | question |
| `codesearch` | 代码语义搜索 | codesearch |
| `lsp` | LSP 语言服务 | lsp |

### 6.3 工具执行流水线

```
LLM 输出 tool_call
       ↓
┌──────────────────┐
│  参数验证 (Zod)   │
└────────┬─────────┘
         ↓
┌──────────────────┐
│  PreToolUse Hook │  ← 用户自定义脚本
└────────┬─────────┘
         ↓
┌──────────────────┐
│    权限检查       │  ← PermissionNext.evaluate()
└────────┬─────────┘
         ↓ (如果需要询问)
┌──────────────────┐
│  等待用户授权     │  ← 发布 permission.asked 事件
└────────┬─────────┘
         ↓
┌──────────────────┐
│    执行工具       │
└────────┬─────────┘
         ↓
┌──────────────────┐
│ PostToolUse Hook │
└────────┬─────────┘
         ↓
┌──────────────────┐
│    输出截断       │  ← 防止超长输出
└────────┬─────────┘
         ↓
     返回给 LLM
```

### 6.4 工具注册机制

**文件**: `packages/ccode/src/tool/registry.ts`

工具注册支持：
1. **内置工具**: 在 `tool/` 目录下定义
2. **自定义工具**: 在 `.codecoder/tools/` 目录下的 JS/TS 模块
3. **MCP 工具**: 通过 MCP 协议从外部服务器加载

---

## 7. 权限系统

权限系统是 CodeCoder 安全性的核心，确保 AI 的每个操作都在用户控制之下。

### 7.1 权限模型

**文件**: `packages/ccode/src/permission/next.ts`

```typescript
namespace PermissionNext {
  // 权限动作
  type Action = "allow" | "deny" | "ask"

  // 权限规则
  interface Rule {
    permission: string  // 权限类型：read, edit, bash, etc.
    pattern: string     // 匹配模式：支持通配符
    action: Action      // 动作
  }

  // 规则集 = 规则数组
  type Ruleset = Rule[]

  // 评估权限
  export function evaluate(
    permission: string,
    pattern: string,
    ...rulesets: Ruleset[]
  ): Rule {
    const merged = merge(...rulesets)
    // 从后往前查找匹配的规则（后面的优先级高）
    const match = merged.findLast(rule =>
      Wildcard.match(permission, rule.permission) &&
      Wildcard.match(pattern, rule.pattern)
    )
    // 默认：询问用户
    return match ?? { action: "ask", permission, pattern: "*" }
  }
}
```

### 7.2 默认权限配置

```typescript
const defaults = {
  "*": "allow",              // 默认允许
  doom_loop: "ask",          // 死循环检测需要询问
  question: "deny",          // 默认禁止主动提问
  plan_enter: "deny",        // 默认禁止进入计划模式
  external_directory: {
    "*": "ask",              // 访问外部目录需要询问
  },
  read: {
    "*": "allow",
    "*.env": "ask",          // 读取 .env 文件需要询问
    "*.env.*": "ask",
  },
}
```

### 7.3 权限请求流程

```typescript
// 请求权限
export async function ask(input: {
  permission: string
  patterns: string[]
  ruleset: Ruleset
}) {
  for (const pattern of input.patterns) {
    const rule = evaluate(input.permission, pattern, input.ruleset)

    if (rule.action === "deny") {
      throw new DeniedError(...)  // 直接拒绝
    }

    if (rule.action === "ask") {
      // 发布事件，等待用户响应
      return new Promise((resolve, reject) => {
        const request = { id: generateId(), ...input }
        pending[request.id] = { resolve, reject }
        Bus.publish(Event.Asked, request)  // TUI 会显示授权对话框
      })
    }

    // action === "allow"，继续检查下一个 pattern
  }
}

// 用户响应
export async function reply(input: {
  requestID: string
  reply: "once" | "always" | "reject"
}) {
  const request = pending[input.requestID]

  if (input.reply === "reject") {
    request.reject(new RejectedError())
    // 同时拒绝该会话所有待处理的权限请求
  }

  if (input.reply === "once") {
    request.resolve()  // 仅本次允许
  }

  if (input.reply === "always") {
    // 添加到已批准规则中（仅本次会话）
    approved.push({ permission, pattern, action: "allow" })
    request.resolve()
    // 自动批准其他符合条件的待处理请求
  }
}
```

### 7.4 权限错误类型

| 错误类型 | 说明 | 行为 |
|---------|------|------|
| `RejectedError` | 用户拒绝（无消息） | 停止执行 |
| `CorrectedError` | 用户拒绝并提供反馈 | 继续执行，带反馈 |
| `DeniedError` | 配置规则自动拒绝 | 停止执行 |

---

## 8. Agent 系统

Agent 是 CodeCoder 的"人格"，不同 Agent 有不同的能力和限制。

### 8.1 Agent 定义

**文件**: `packages/ccode/src/agent/agent.ts`

```typescript
namespace Agent {
  interface Info {
    name: string           // 唯一标识
    description?: string   // 描述（给用户/LLM 看）
    mode: "subagent" | "primary" | "all"  // 模式
    native?: boolean       // 是否内置
    hidden?: boolean       // 是否隐藏
    temperature?: number   // LLM 温度参数
    topP?: number
    permission: Ruleset    // 权限规则集
    model?: { providerID, modelID }  // 指定模型
    prompt?: string        // 系统提示
    options: Record<string, any>     // 其他选项
    steps?: number         // 最大步数
  }
}
```

### 8.2 内置 Agent

| Agent | 模式 | 用途 |
|-------|------|------|
| `build` | primary | 主要编程 Agent，完整访问权限 |
| `plan` | primary | 只读探索和规划，限制编辑权限 |
| `general` | subagent | 通用子 Agent，处理复杂多步骤任务 |
| `explore` | subagent | 快速代码搜索专家 |
| `compaction` | hidden | 对话压缩（内部使用） |
| `title` | hidden | 生成会话标题（内部使用） |
| `summary` | hidden | 生成会话摘要（内部使用） |

### 8.3 Agent 权限差异

```typescript
// build Agent - 完整权限
const buildAgent = {
  permission: merge(defaults, {
    question: "allow",    // 可以询问用户
    plan_enter: "allow",  // 可以进入计划模式
  })
}

// plan Agent - 只读权限
const planAgent = {
  permission: merge(defaults, {
    question: "allow",
    plan_exit: "allow",
    edit: {
      "*": "deny",        // 禁止编辑所有文件
      ".codecoder/plans/*.md": "allow",  // 只能编辑计划文件
    }
  })
}

// explore Agent - 搜索专用
const exploreAgent = {
  permission: merge(defaults, {
    "*": "deny",          // 禁止所有
    grep: "allow",        // 只允许搜索相关
    glob: "allow",
    read: "allow",
    bash: "allow",        // 允许 bash（用于 rg）
    webfetch: "allow",
    websearch: "allow",
  })
}
```

### 8.4 Agent 模式说明

| 模式 | 说明 |
|------|------|
| `primary` | 主 Agent，可被用户直接选择使用 |
| `subagent` | 子 Agent，只能被其他 Agent 通过 `task` 工具调用 |
| `all` | 两种模式都可以 |

### 8.5 自定义 Agent

用户可以在配置文件中定义自己的 Agent：

```jsonc
// .codecoder/config.jsonc
{
  "agent": {
    "reviewer": {
      "description": "Code review specialist",
      "mode": "subagent",
      "model": "anthropic/claude-3-5-sonnet",
      "temperature": 0.3,
      "prompt": "You are a code review expert. Focus on code quality, security issues, and best practices.",
      "permission": {
        "edit": "deny",  // 禁止编辑
        "read": "allow"
      }
    }
  }
}
```

---

## 9. LLM 集成层

### 9.1 Provider 抽象

**文件**: `packages/ccode/src/provider/provider.ts`

支持 20+ AI 提供商的统一接口：

```typescript
namespace Provider {
  interface Model {
    id: string
    providerID: string
    name: string
    cost: { input: number; output: number }
    limit: { context: number; output: number }
    api: { npm: string; id: string }
    capabilities: {
      temperature: boolean
      topP: boolean
      // ...
    }
    variants?: Record<string, any>
    headers?: Record<string, string>
    options?: Record<string, any>
  }

  // 获取 LLM 语言模型实例
  export async function getLanguage(model: Model): LanguageModel {
    const sdk = await loadSDK(model.api.npm)  // 动态加载 @ai-sdk/*
    return sdk.createModel(model.id, options)
  }
}
```

### 9.2 支持的提供商

- **Anthropic**: Claude 系列
- **OpenAI**: GPT-4、o1、o3 系列
- **Google**: Gemini、Vertex AI
- **Azure**: Azure OpenAI
- **其他**: Groq、Mistral、Cohere、DeepInfra、Cerebras、XAI、Perplexity、Together AI、OpenRouter 等

### 9.3 LLM 流式调用

**文件**: `packages/ccode/src/session/llm.ts`

```typescript
namespace LLM {
  export async function stream(input: StreamInput) {
    const language = await Provider.getLanguage(input.model)

    // 构建系统提示
    const system = [
      SystemPrompt.header(input.model.providerID),  // 提供商特定头部
      input.agent.prompt || SystemPrompt.provider(input.model),  // Agent/提供商提示
      ...input.system,      // 自定义系统提示
      input.user.system,    // 用户消息中的系统提示
    ]

    // 解析可用工具（根据权限过滤）
    const tools = await resolveTools(input)

    return streamText({
      model: wrapLanguageModel({
        model: language,
        middleware: [
          // 消息转换中间件
          { transformParams: ... },
          // 推理提取中间件（支持 <think> 标签）
          extractReasoningMiddleware({ tagName: "think" }),
        ],
      }),
      messages: [
        ...system.map(x => ({ role: "system", content: x })),
        ...input.messages,
      ],
      tools,
      temperature: input.agent.temperature,
      maxOutputTokens: 32_000,
      abortSignal: input.abort,

      // 工具调用修复（处理大小写等问题）
      experimental_repairToolCall(failed) {
        const lower = failed.toolCall.toolName.toLowerCase()
        if (tools[lower]) return { ...failed.toolCall, toolName: lower }
        return { toolName: "invalid", input: JSON.stringify(failed.error) }
      }
    })
  }
}
```

### 9.4 工具过滤逻辑

```typescript
async function resolveTools(input) {
  // 1. 根据 Agent 权限禁用某些工具
  const disabled = PermissionNext.disabled(Object.keys(input.tools), input.agent.permission)

  // 2. 根据用户消息中的 tools 配置过滤
  for (const tool of Object.keys(input.tools)) {
    if (input.user.tools?.[tool] === false || disabled.has(tool)) {
      delete input.tools[tool]
    }
  }

  return input.tools
}
```

---

## 10. 数据流分析

### 10.1 完整请求流程

```
┌─────────────────────────────────────────────────────────────────┐
│  用户输入 (CLI/Web/Desktop)                                      │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  HTTP Server (Hono)                                              │
│  POST /session/:id/message                                       │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  Session.send()                                                  │
│  - 创建 User Message                                             │
│  - 创建 Assistant Message                                        │
│  - 创建 SessionProcessor                                         │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  SessionProcessor.process()                                      │
│  - 构建 LLM 输入                                                 │
│  - 加载可用工具                                                   │
│  - 获取 Agent 配置                                               │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  LLM.stream()                                                    │
│  - 选择 Provider                                                 │
│  - 构建系统提示                                                   │
│  - 调用 Vercel AI SDK                                            │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  流式处理循环                                                     │
│  for await (const event of stream) {                             │
│    case "text-delta": 更新文本                                   │
│    case "tool-call": 执行工具                                    │
│    case "tool-result": 处理结果                                  │
│    case "finish-step": 检查是否需要压缩                          │
│  }                                                               │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  工具执行                                                        │
│  - PreToolUse Hook                                               │
│  - PermissionNext.ask()                                          │
│  - tool.execute()                                                │
│  - PostToolUse Hook                                              │
│  - Truncate.output()                                             │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│  结果返回                                                        │
│  - 更新 Message Parts                                            │
│  - 触发 SSE 事件                                                 │
│  - 更新 Session 状态                                             │
└─────────────────────────────────────────────────────────────────┘
```

### 10.2 SSE 实时推送

```typescript
// HTTP Server
app.get("/event", async (c) => {
  return c.stream(async (stream) => {
    // 订阅所有事件
    const subscriptions = [
      Bus.subscribe(Session.Event.Created, (data) =>
        stream.write(`event: session.created\ndata: ${JSON.stringify(data)}\n\n`)
      ),
      Bus.subscribe(Permission.Event.Asked, (data) =>
        stream.write(`event: permission.asked\ndata: ${JSON.stringify(data)}\n\n`)
      ),
      // ...
    ]

    // 心跳保活（30秒）
    const heartbeat = setInterval(() =>
      stream.write(`event: heartbeat\ndata: {}\n\n`), 30000)

    // 清理
    c.req.signal.addEventListener("abort", () => {
      subscriptions.forEach(unsub => unsub())
      clearInterval(heartbeat)
    })
  })
})
```

### 10.3 主要 API 路由

| 路由 | 方法 | 功能 |
|------|------|------|
| `/session` | POST | 创建会话 |
| `/session/:id/message` | POST | 发送消息 |
| `/session/:id` | GET | 获取会话信息 |
| `/event` | GET | SSE 事件流 |
| `/permission/:id` | POST | 权限响应 |
| `/config` | GET/PUT | 配置管理 |
| `/provider` | GET | 获取提供商列表 |
| `/mcp` | GET/POST | MCP 服务器管理 |

---

## 11. 开发工作流

### 11.1 完整开发工作流

```bash
# 1. 克隆项目
git clone https://github.com/iannil/code-coder.git
cd codecoder

# 2. 安装依赖
bun install

# 3. 启动开发环境
bun dev  # 启动 TUI

# 4. 运行测试
cd packages/ccode && bun test

# 5. 类型检查
bun turbo typecheck

# 6. 构建可执行文件
bun run --cwd packages/ccode build
```

### 11.2 项目文件结构速查

```
packages/ccode/src/
├── index.ts              # CLI 入口点
├── agent/                # Agent 定义和管理
│   ├── agent.ts          # Agent 核心定义
│   └── build.ts          # build Agent 配置
├── session/              # 会话管理核心
│   ├── index.ts          # 会话管理
│   ├── processor.ts      # 消息处理
│   ├── llm.ts            # LLM 集成
│   └── message-v2.ts     # 消息结构
├── tool/                 # 工具系统
│   ├── registry.ts       # 工具注册
│   ├── tool.ts           # 工具定义
│   ├── read.ts           # 读取文件工具
│   ├── edit.ts           # 编辑文件工具
│   └── bash.ts           # 执行命令工具
├── permission/           # 权限系统
│   └── next.ts           # 权限评估和请求
├── provider/             # LLM 提供商抽象
│   └── provider.ts       # 提供商定义
├── server/               # HTTP 服务器
│   └── server.ts         # API 路由
├── mcp/                  # MCP 协议集成
│   └── index.ts          # MCP 客户端
├── config/               # 配置管理
│   └── config.ts         # 配置加载和合并
├── project/              # 项目管理
│   └── instance.ts       # 实例模式
└── cli/cmd/
    └── tui/              # 终端 UI
        └── app.tsx       # TUI 入口
```

---

## 12. 扩展开发

### 12.1 自定义工具

在 `.codecoder/tools/` 目录下创建工具：

```typescript
// .codecoder/tools/my-tool.ts
import { z } from "zod"

export const tool = {
  id: "my-custom-tool",
  async init() {
    return {
      description: "描述这个工具做什么，这会显示给 LLM",
      parameters: z.object({
        input: z.string().describe("输入参数的描述"),
        optional: z.number().optional().describe("可选参数"),
      }),
      async execute(args, ctx) {
        // 可以请求权限
        await ctx.ask({
          permission: "my-permission",
          patterns: [args.input],
          metadata: { reason: "需要访问..." }
        })

        // 更新执行状态（显示给用户）
        ctx.metadata({ title: "正在处理..." })

        // 执行逻辑
        const result = await doSomething(args.input)

        return {
          title: "工具执行完成",
          metadata: { custom: "data" },
          output: result,  // 这个会返回给 LLM
        }
      }
    }
  }
}
```

### 12.2 自定义 Hook

Hooks 允许在工具执行前后运行自定义脚本：

```jsonc
// .codecoder/config.jsonc
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": { "tool": "bash" },
        "command": "./scripts/check-command.sh"
      }
    ],
    "PostToolUse": [
      {
        "matcher": { "tool": "edit" },
        "command": "npm run lint --fix"
      }
    ]
  }
}
```

Hook 脚本接收环境变量：
- `CCODE_TOOL` - 工具名称
- `CCODE_INPUT` - JSON 格式的输入参数
- `CCODE_OUTPUT` - JSON 格式的输出（仅 PostToolUse）
- `CCODE_FILE_PATH` - 文件路径（如果适用）

### 12.3 MCP Server 集成

```jsonc
// .codecoder/config.jsonc
{
  "mcp": {
    "servers": {
      "github": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-github"],
        "env": {
          "GITHUB_TOKEN": "${GITHUB_TOKEN}"
        }
      },
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"]
      },
      "database": {
        "url": "http://localhost:3000/mcp",
        "transport": "sse"
      }
    }
  }
}
```

支持的传输方式：
- `stdio` - 子进程通信（默认）
- `sse` - Server-Sent Events
- `http` - HTTP StreamableHTTP

### 12.4 HTTP API 使用

JavaScript SDK 已被移除。你可以直接使用 HTTP API 与 CodeCoder 服务器通信：

```typescript
// 使用原生 fetch API
const baseUrl = "http://localhost:4096"

// 创建会话
const session = await fetch(`${baseUrl}/session`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ agent: "build" })
}).then(r => r.json())

// 发送消息
const response = await fetch(`${baseUrl}/session/${session.id}/message`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ content: "帮我写一个 hello world 程序" })
})
```

### 12.5 自定义配置示例

```jsonc
// .codecoder/config.jsonc
{
  // 默认 Agent
  "default_agent": "build",

  // 默认模型
  "model": "anthropic/claude-sonnet-4-20250514",

  // 权限配置
  "permission": {
    "bash": {
      "*": "ask",
      "git *": "allow",
      "npm *": "allow"
    },
    "edit": {
      "*.lock": "deny",
      "node_modules/**": "deny"
    }
  },

  // 自定义 Agent
  "agent": {
    "docs-writer": {
      "description": "Documentation specialist",
      "prompt": "You are a technical writer...",
      "permission": {
        "edit": {
          "*": "deny",
          "docs/**": "allow",
          "*.md": "allow"
        }
      }
    }
  },

  // 指令文件
  "instructions": [
    ".codecoder/instructions.md",
    "CONTRIBUTING.md"
  ]
}
```

---

## 13. 总结

### 13.1 核心设计思想

1. **安全第一**: 细粒度权限系统确保用户对 AI 行为有完全控制
2. **可扩展性**: 工具系统、Hook 系统、MCP 协议支持灵活扩展
3. **多 Agent 协作**: 不同 Agent 有不同专长，可以相互调用
4. **流式处理**: 全程流式传输，实时反馈
5. **状态隔离**: Instance 模式确保不同项目状态独立

### 13.2 关键文件速查

| 文件                      | 用途                           |
| ------------------------- | --------------------------------- |
| `src/index.ts`            | CLI 入口点                   |
| `src/server/server.ts`    | HTTP 服务器和路由           |
| `src/session/index.ts`    | 会话管理                |
| `src/session/processor.ts`| 会话消息处理        |
| `src/session/llm.ts`      | LLM 集成                   |
| `src/agent/agent.ts`      | Agent 系统                      |
| `src/provider/provider.ts`| LLM 提供商抽象          |
| `src/tool/registry.ts`    | 工具注册                 |
| `src/tool/tool.ts`        | 工具定义和执行      |
| `src/permission/next.ts`  | 权限系统                 |
| `src/config/config.ts`    | 配置管理                 |
| `src/project/instance.ts` | 实例和上下文管理   |
| `src/mcp/index.ts`        | MCP 集成                   |
| `src/cli/cmd/tui/app.tsx` | 终端 UI                       |

### 13.3 开发建议

1. **阅读顺序建议**: `index.ts` → `session/` → `tool/` → `permission/` → `agent/`
2. **调试技巧**: 使用 `--print-logs` 和 `--log-level DEBUG` 查看详细日志
3. **测试**: 在 `packages/ccode` 目录下运行 `bun test`
4. **代码风格**: 遵循 CLAUDE.md 中的规范（const > let，避免 else，单字变量名等）

理解这些核心概念后，你就可以深入探索具体模块的实现细节，或者开始为项目贡献代码了。

---

## 附录

### A. 相关文档链接

| 文档        | 说明                              |
| --------------- | ---------------------------------------- |
| [项目进度](./progress.md) | 当前功能状态和路线图 |
| [开发指南](./guides/development.md) | 环境设置和开发 |
| [归档文档](./archive/) | 历史参考文档 |
