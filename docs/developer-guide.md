# CodeCoder 开发者指南

本指南面向有经验的开发者，帮助你快速理解 CodeCoder 项目并开始贡献。

## 1. 项目概述

### 1.1 CodeCoder 是什么

CodeCoder 是一个开源 AI 编程代理，让 AI 像人类开发者一样工作——能够阅读代码、执行命令、编辑文件、搜索网络，并在需要时请求用户授权。

### 1.2 核心特性

- **多 AI 提供商支持**: Claude、OpenAI、Google Gemini、本地模型等 20+ 提供商
- **CLI/TUI 界面**: 基于 SolidJS + OpenTUI 的终端用户界面
- **MCP 协议支持**: Model Context Protocol，可扩展的工具生态系统
- **细粒度权限控制**: 用户对每个操作有完全控制权
- **GitHub 集成**: Actions、Webhook 和身份验证

### 1.3 当前版本状态

- **版本**: 0.0.1（开发中）
- **默认分支**: `dev`
- **最近重大变更**: 移除 ACP 架构，简化为直接 API 架构；精简包结构为核心 ccode 包

## 2. 快速开始

### 2.1 环境要求

- **Bun** 1.3 或更高版本
- **Git**

### 2.2 安装和运行

```bash
# 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd codecoder

# 安装依赖
bun install

# 启动 TUI
bun dev

# 在指定目录运行
bun dev <path>

# 启动无头 API 服务器（默认端口 4096）
bun dev serve
bun dev serve --port 8080
```

### 2.3 常用开发命令

```bash
# 运行测试（必须从特定包目录运行）
cd packages/ccode && bun test

# 类型检查
bun turbo typecheck

# 构建
bun run --cwd packages/ccode build

# SDK 重新生成（修改 API 后）
./script/generate.ts
```

## 3. 架构深度解析

### 3.1 Monorepo 结构

```
code-coder/
├── packages/
│   ├── ccode/         # 核心 CLI 工具（入口：src/index.ts）
│   ├── util/          # 共享工具库
│   ├── sdk/js/        # JavaScript/TypeScript SDK（自动生成）
│   └── script/        # 构建脚本工具
├── infra/             # 基础设施代码 (SST/Cloudflare)
├── docs/              # 项目文档
└── script/            # 项目级构建和生成脚本
```

### 3.2 ccode 核心目录结构

```
packages/ccode/src/
├── agent/      # Agent 实现和管理
├── api/        # 新 API 架构（会话、配置、事件、权限）
├── auth/       # 认证相关
├── cli/        # CLI 命令
│   └── cmd/
│       ├── serve.ts    # 启动 HTTP 服务器
│       ├── run.ts      # 运行会话
│       └── tui/        # 终端 UI（SolidJS + OpenTUI）
├── command/    # 命令系统
├── context/    # 上下文管理
├── file/       # 文件操作
├── lsp/        # LSP 集成
├── mcp/        # MCP 协议支持
├── memory/     # 记忆系统
├── permission/ # 权限系统
├── provider/   # AI 提供商抽象（20+ 提供商）
├── project/    # 项目管理
├── session/    # 会话系统
├── shell/      # Shell 命令执行
├── skill/      # 技能/斜杠命令
├── tool/       # 工具集成（46+ 工具）
└── util/       # 内部工具
```

### 3.3 新 API 架构

| 文件 | 功能 |
|------|------|
| `api/session.ts` | 会话管理 API |
| `api/config.ts` | 配置 API |
| `api/event.ts` | 事件 API |
| `api/permission.ts` | 权限 API |

### 3.4 包依赖关系

```
┌──────────────┐
│   ccode      │ ← 核心包，依赖 util
├──────────────┤
│   util       │ ← 共享工具（错误处理等）
├──────────────┤
│   sdk/js     │ ← 从 OpenAPI 生成，供外部使用
└──────────────┘
```

## 4. 核心系统详解

### 4.1 会话系统 (Session)

会话是 CodeCoder 的核心组织单位，包含完整的对话上下文。

**关键文件**:
- `packages/ccode/src/session/index.ts` - 会话管理
- `packages/ccode/src/session/processor.ts` - 消息处理
- `packages/ccode/src/session/llm.ts` - LLM 集成
- `packages/ccode/src/session/message-v2.ts` - 消息结构

**会话生命周期**:
```
创建会话 → 接收用户消息 → 调用 LLM → 解析工具调用 →
权限检查 → 执行工具 → 处理结果 → 返回给 LLM →
(循环直到完成) → 会话归档
```

### 4.2 Agent 系统

Agent 定义了 AI 的"人格"和能力限制。

**内置 Agent**:
| Agent | 模式 | 用途 |
|-------|------|------|
| `build` | primary | 主要编程 Agent，完整访问权限 |
| `plan` | primary | 只读探索和规划，限制编辑权限 |
| `general` | subagent | 通用子 Agent |
| `explore` | subagent | 快速代码搜索专家 |

**Agent 定义文件**: `packages/ccode/src/agent/agent.ts`

### 4.3 工具系统 (Tool)

工具让 AI 能够与外部世界交互。

**内置工具** (46+):
- 文件操作: `read`, `write`, `edit`, `glob`
- 搜索: `grep`, `codesearch`, `websearch`, `webfetch`
- 执行: `bash`, `task`
- 交互: `question`
- LSP: `lsp`

**工具目录**: `packages/ccode/src/tool/`

### 4.4 权限系统 (Permission)

权限系统确保 AI 的每个操作都在用户控制之下。

**权限动作**: `allow` | `deny` | `ask`

**权限文件**: `packages/ccode/src/permission/next.ts`

### 4.5 Provider 抽象

统一的 AI 提供商接口，支持 20+ 提供商。

**支持的提供商**:
- Anthropic (Claude)
- OpenAI (GPT-4, o1, o3)
- Google (Gemini, Vertex AI)
- Azure OpenAI
- Groq, Mistral, Cohere, DeepInfra, Cerebras, XAI, Perplexity, Together AI, OpenRouter 等

**Provider 文件**: `packages/ccode/src/provider/provider.ts`

### 4.6 MCP 集成

Model Context Protocol 支持，可扩展的工具生态系统。

**MCP 目录**: `packages/ccode/src/mcp/`

## 5. 开发规范

### 5.1 代码风格

来自 `CLAUDE.md` 和 `AGENTS.md`:

- 优先使用 `const` 和三元运算符，而非 `let` 和 `else`
- 避免不必要的解构 - 使用 `obj.a` 而不是 `const { a } = obj`
- 尽可能避免 `try`/`catch` - 优先使用 `.catch()`
- 避免使用 `any` 类型
- 优先使用单字变量名
- 尽可能使用 Bun API（例如 `Bun.file()`）

### 5.2 测试规范

- 测试文件位于 `packages/ccode/test/` 目录
- 使用 Bun 内置测试运行器
- 从特定包目录运行：`cd packages/ccode && bun test`
- 不要从仓库根目录运行测试

### 5.3 提交规范

- PR 必须引用现有的 issue
- PR 标题遵循约定式提交规范（`feat:`、`fix:`、`docs:` 等）
- 所有 PR 应保持小而专注

### 5.4 格式化

- **Prettier**: 120 字符行宽，无分号
- **EditorConfig**: 2 空格缩进，最大 80 字符行，LF 换行

## 6. 核心概念

### 6.1 实例管理模式 (Instance Pattern)

每个工作目录对应一个 `Instance`，管理该目录下的所有状态。

**文件**: `packages/ccode/src/project/instance.ts`

### 6.2 配置系统层级

配置遵循从低到高优先级合并的策略：

```
1. 远程 well-known 配置 (.well-known/codecoder)
   ↓ 合并
2. 全局用户配置 (~/.ccode/config.jsonc)
   ↓ 合并
3. 自定义配置路径 (CCODE_CONFIG 环境变量)
   ↓ 合并
4. 项目配置 (.ccode/ 或 .codecoder/ 目录)
   ↓ 合并
5. 环境变量覆盖 (CCODE_CONFIG_CONTENT)
```

**配置文件**: `packages/ccode/src/config/config.ts`

### 6.3 事件总线 (Bus)

发布-订阅模式进行组件间通信。

**主要事件类型**:
- `session.*` - 会话相关事件
- `permission.*` - 权限请求/响应事件
- `mcp.*` - MCP 工具变更事件

## 7. 扩展开发

### 7.1 添加新工具

在 `.ccode/tools/` 目录下创建工具：

```typescript
// .ccode/tools/my-tool.ts
import { z } from "zod"

export const tool = {
  id: "my-custom-tool",
  async init() {
    return {
      description: "描述这个工具做什么",
      parameters: z.object({
        input: z.string().describe("输入参数"),
      }),
      async execute(args, ctx) {
        // 请求权限
        await ctx.ask({
          permission: "my-permission",
          patterns: [args.input],
        })

        // 执行逻辑
        const result = await doSomething(args.input)

        return {
          title: "工具执行完成",
          metadata: {},
          output: result,
        }
      }
    }
  }
}
```

### 7.2 添加新 Provider

在 `packages/ccode/src/provider/` 中添加新的提供商配置。

### 7.3 添加新 Agent

在配置文件中定义：

```jsonc
// .ccode/config.jsonc
{
  "agent": {
    "reviewer": {
      "description": "Code review specialist",
      "mode": "subagent",
      "model": "anthropic/claude-3-5-sonnet",
      "permission": {
        "edit": "deny",
        "read": "allow"
      }
    }
  }
}
```

### 7.4 MCP Server 集成

```jsonc
// .ccode/config.jsonc
{
  "mcp": {
    "servers": {
      "github": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-github"],
        "env": {
          "GITHUB_TOKEN": "${GITHUB_TOKEN}"
        }
      }
    }
  }
}
```

## 8. 关键文件索引

| 文件 | 用途 |
|------|------|
| `src/index.ts` | CLI 入口点 |
| `src/api/session.ts` | 会话 API |
| `src/api/config.ts` | 配置 API |
| `src/api/event.ts` | 事件 API |
| `src/api/permission.ts` | 权限 API |
| `src/session/index.ts` | 会话管理 |
| `src/session/processor.ts` | 会话消息处理 |
| `src/session/llm.ts` | LLM 集成 |
| `src/agent/agent.ts` | Agent 系统 |
| `src/provider/provider.ts` | LLM 提供商抽象 |
| `src/tool/registry.ts` | 工具注册 |
| `src/tool/tool.ts` | 工具定义和执行 |
| `src/permission/next.ts` | 权限系统 |
| `src/config/config.ts` | 配置管理 |
| `src/project/instance.ts` | 实例和上下文管理 |
| `src/mcp/index.ts` | MCP 集成 |
| `src/cli/cmd/tui/app.tsx` | 终端 UI |

## 9. 故障排查

### 9.1 调试技巧

使用 `--print-logs` 和 `--log-level DEBUG` 查看详细日志。

### 9.2 常见问题

- 测试失败：确保从 `packages/ccode` 目录运行
- 类型错误：运行 `bun turbo typecheck`
- 构建失败：检查 Bun 版本是否 >= 1.3

## 10. 相关文档

| 文档 | 说明 |
|------|------|
| [项目 README](../../README.md) | 项目概述 |
| [架构指南](./Architecture-Guide.md) | 深度解析项目架构 |
| [开发指南](./guides/development.md) | 环境设置和开发说明 |
| [项目进度](./progress.md) | 当前功能状态和路线图 |
| [产品文档](https://code-coder.com/docs) | 用户指南和配置说明 |
