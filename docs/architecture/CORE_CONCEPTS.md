# CodeCoder 核心概念

> 生成时间: 2026-03-01

本文档梳理 CodeCoder 系统的八个核心概念：AGENT、PROMPT、SKILL、TOOL、CHANNEL、MEMORY、WORKFLOW、HAND，以及它们之间的关系。

---

## 目录

1. [概念关系总览](#概念关系总览)
2. [AGENT - 智能执行单元](#agent---智能执行单元)
3. [PROMPT - Agent 行为定义](#prompt---agent-行为定义)
4. [SKILL - 可复用能力](#skill---可复用能力)
5. [TOOL - 执行工具](#tool---执行工具)
6. [CHANNEL - 消息渠道](#channel---消息渠道)
7. [MEMORY - 记忆系统](#memory---记忆系统)
8. [WORKFLOW - 工作流引擎](#workflow---工作流引擎)
9. [HAND - 自主代理](#hand---自主代理)
10. [概念对比表](#概念对比表)
11. [层次架构](#层次架构)
12. [概念关系详解](#概念关系详解)

---

## 概念关系总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              用户交互层                                      │
│    CLI / TUI / Web / Telegram / Discord / Slack / Feishu / WhatsApp        │
└─────────────────────────────────────────────────┬───────────────────────────┘
                                                  │
                                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CHANNEL (消息渠道)                                 │
│                   zero-channels (Rust) :4431                                │
│           接收外部消息 → 转换格式 → 路由 → 返回响应                           │
└─────────────────────────────────────────────────┬───────────────────────────┘
                                                  │ HTTP API
                                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AGENT (智能单元)                                │
│                     packages/ccode (TypeScript)                             │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │  PROMPT (行为定义)                                                      ││
│  │  packages/ccode/src/agent/prompt/*.txt                                 ││
│  │  "你是 Build Agent，负责执行开发任务..."                                 ││
│  └────────────────────────────────────────────────────────────────────────┘│
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │  SKILL (可复用能力)              │  TOOL (执行工具)                     ││
│  │  .codecoder/skills/*/SKILL.md    │  Bash, Read, Edit, Grep, Task...   ││
│  │  "如何进行 TDD"                  │  Agent 通过 Tool 与环境交互          ││
│  └────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────┬───────────────────────────┘
                                                  │ 读写
                                                  ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              MEMORY (记忆系统)                               │
│             packages/ccode/src/memory-markdown/ (TypeScript)                │
│  ┌───────────────────────────┐    ┌───────────────────────────┐            │
│  │  流层 (Daily Notes)       │    │  沉积层 (Long-term)       │            │
│  │  memory/daily/YYYY-MM-DD  │ →→ │  memory/MEMORY.md         │            │
│  │  仅追加、不可变           │    │  结构化、可编辑            │            │
│  └───────────────────────────┘    └───────────────────────────┘            │
└─────────────────────────────────────────────────┬───────────────────────────┘
                                                  │
         ┌────────────────────────────────────────┴────────────────────────┐
         │                                                                 │
         ▼                                                                 ▼
┌─────────────────────────────────────┐    ┌──────────────────────────────────┐
│       WORKFLOW (工作流引擎)          │    │       HAND (自主代理)             │
│   zero-workflow (Rust) :4432        │    │  HAND.md 定义 + Cron 调度        │
│  ┌─────────────────────────────────┐│    │  ┌────────────────────────────┐  │
│  │ • Cron 定时任务                 ││    │  │ 持久状态 + 自主决策         │  │
│  │ • Webhook 触发器                ││    │  │ CLOSE 决策框架              │  │
│  │ • Git 事件监听                  ││    │  │ 风险评估 + 人在回路         │  │
│  │ • 工作流 DSL                    ││    │  └────────────────────────────┘  │
│  └─────────────────────────────────┘│    │  零代码编写实现定时 AI 任务      │
└─────────────────────────────────────┘    └──────────────────────────────────┘
```

---

## AGENT - 智能执行单元

### 定义

Agent 是具有特定职责和行为的 AI 执行单元，每个 Agent 专注于一类任务。Agent 是 CodeCoder 系统中最核心的概念，所有 AI 能力都通过 Agent 来实现。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Agent 定义 | `packages/ccode/src/agent/agent.ts` |
| Agent 注册 | `packages/ccode/src/agent/registry.ts` |
| Prompt 文件 | `packages/ccode/src/agent/prompt/*.txt` |

### 数据结构

```typescript
export const Info = z.object({
  name: z.string(),                              // 标识符: "build", "macro"
  description: z.string().optional(),            // 功能描述
  mode: z.enum(["subagent", "primary", "all"]),  // 运行模式
  native: z.boolean().optional(),                // 是否内置
  hidden: z.boolean().optional(),                // 是否隐藏
  topP: z.number().optional(),                   // 采样参数
  temperature: z.number().optional(),            // 创造性程度
  color: z.string().optional(),                  // 显示颜色
  permission: PermissionNext.Ruleset,            // 权限规则
  model: z.object({                              // 指定模型
    modelID: z.string(),
    providerID: z.string(),
  }).optional(),
  prompt: z.string().optional(),                 // 行为定义 (PROMPT)
  options: z.record(z.string(), z.any()),        // 额外选项
  steps: z.number().int().positive().optional(), // 最大步数
  autoApprove: AutoApproveConfigSchema.optional(), // 自动审批配置
})
```

### 运行模式

| 模式 | 说明 | 示例 |
| ------ | ------ | ------ |
| `primary` | 可作为主入口，用户直接调用 | build, plan, writer, autonomous |
| `subagent` | 被其他 Agent 调用的专用 Agent | code-reviewer, macro, decision |
| `hidden` | 系统内部使用，用户不可见 | compaction, title, summary |
| `all` | 既可作为主入口也可被调用 | 自定义 Agent 默认模式 |

### Agent 列表 (31 个)

#### 主模式 Agent (4)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `build` | Build Agent | 主要开发模式，协调其他 Agent |
| `plan` | Plan Agent | 任务规划和设计 |
| `writer` | Writer Agent | 长文写作 (20k+ 字) |
| `autonomous` | Autonomous Agent | 完全自主执行，使用 CLOSE 决策 |

#### 工程质量 Agent (7)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `code-reviewer` | 代码审查员 | 代码质量审查 |
| `security-reviewer` | 安全审查员 | 安全漏洞分析 |
| `tdd-guide` | TDD 指南 | 测试驱动开发 |
| `architect` | 架构师 | 系统设计和架构 |
| `explore` | 探索者 | 快速代码库探索 |
| `general` | 通用 Agent | 多步骤并行任务 |
| `verifier` | 验证者 | 综合验证 (构建/类型/测试) |

#### 逆向工程 Agent (2)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `code-reverse` | 网站逆向 | 网站重建规划 |
| `jar-code-reverse` | JAR 逆向 | Java 源码重构 |

#### 内容创作 Agent (4)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `expander` | 扩展器 | 内容扩展 |
| `expander-fiction` | 小说扩展器 | 小说写作 |
| `expander-nonfiction` | 非虚构扩展器 | 非虚构写作 |
| `proofreader` | 校对员 | 文本校对 |

#### 祝融说系列 Agent (8)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `observer` | 观察者 | 观察者理论分析 |
| `decision` | 决策者 | CLOSE 五维决策 |
| `macro` | 宏观分析师 | 宏观经济分析 |
| `trader` | 交易指南 | 超短线交易分析 |
| `picker` | 选品专家 | 爆品识别 |
| `miniproduct` | 极小产品教练 | 独立开发者指导 |
| `ai-engineer` | AI 工程师导师 | AI 开发教学 |
| `value-analyst` | 价值分析师 | 价值投资分析 |

#### 产品与可行性 Agent (2)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `prd-generator` | PRD 生成器 | 产品需求文档 |
| `feasibility-assess` | 可行性评估 | 技术可行性分析 |

#### 系统 Agent (4)

| Agent | 描述 | 用途 |
| ------- | ------ | ------ |
| `synton-assistant` | SYNTON-DB 助手 | 记忆数据库使用 |
| `compaction` | 压缩 (隐藏) | 上下文压缩 |
| `title` | 标题 (隐藏) | 生成会话标题 |
| `summary` | 摘要 (隐藏) | 生成会话摘要 |

### 使用方式

```bash
# TUI 中切换 Agent
> @macro 分析最新 PMI 数据

# CLI 中指定 Agent
ccode --agent=decision "用 CLOSE 框架分析这个选择"

# 代码中调用
const agent = await Agent.get("code-reviewer")
```

### 自定义 Agent

在 `~/.codecoder/config.json` 中配置:

```json
{
  "agent": {
    "my-agent": {
      "prompt": "你是一个专注于...",
      "description": "自定义 Agent 描述",
      "temperature": 0.7,
      "mode": "all"
    }
  }
}
```

---

## PROMPT - Agent 行为定义

### 定义

Prompt 是 Agent 的"灵魂"，定义了 Agent 的身份、职责、行为准则和决策框架。每个 Agent 都有一个对应的 Prompt 文件。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Prompt 目录 | `packages/ccode/src/agent/prompt/` |
| Prompt 格式 | `.txt` 文件 |

### 当前 Prompt 文件 (30 个)

```
packages/ccode/src/agent/prompt/
├── ai-engineer.txt
├── architect.txt
├── autonomous.txt
├── build.txt
├── code-reverse.txt
├── code-reviewer.txt
├── compaction.txt
├── decision.txt
├── expander-fiction.txt
├── expander-nonfiction.txt
├── expander.txt
├── explore.txt
├── feasibility-assess.txt
├── general.txt
├── jar-code-reverse.txt
├── macro.txt
├── miniproduct.txt
├── observer.txt
├── picker.txt
├── prd-generator.txt
├── proofreader.txt
├── security-reviewer.txt
├── summary.txt
├── synton-assistant.txt
├── tdd-guide.txt
├── title.txt
├── trader.txt
├── value-analyst.txt
├── verifier.txt
└── writer.txt
```

### Prompt 结构

一个典型的 Prompt 包含以下部分:

```text
# 1. 身份定义
You are the Build Agent - responsible for executing development tasks
and coordinating with specialist agents.

# 2. 核心职责
## Core Responsibilities
1. Understand user intent
2. Use appropriate tools
3. Coordinate with specialists

# 3. 行为准则
## Agent Delegation
When you see @<agent-name>:
1. Stop your current work
2. Use Task tool to call the specified agent
3. Provide clear context

# 4. 决策框架
## Decision Framework
1. Can I handle this directly?
   - Yes: Use appropriate tools
   - No: Delegate to specialist

# 5. 可用工具
## Tool Usage
- Read: Read files
- Edit: Modify files
- Bash: Run commands
- Task: Delegate to agents
```

### 与 Agent 的关系

```
Agent = {
  name: "build",
  mode: "primary",
  permission: [...],
  prompt: PROMPT_BUILD,  ◄── Prompt 是 Agent 的一部分
  temperature: 0.6,
  ...
}
```

### 设计原则

1. **单一职责**: 每个 Prompt 只描述一类任务的处理方式
2. **明确边界**: 清晰定义什么该做、什么不该做
3. **决策树**: 提供明确的决策路径
4. **工具指导**: 说明何时使用什么工具

---

## SKILL - 可复用能力

### 定义

Skill 是跨项目、跨 Agent 的可复用知识和方法论，以 `SKILL.md` 文件形式存储。Skill 可以被任何 Agent 动态引用，用于扩展 Agent 的能力。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Skill 定义 | `packages/ccode/src/skill/skill.ts` |
| Skill 工具 | `packages/ccode/src/tool/skill.ts` |
| 内置 Skill | `packages/ccode/src/skill/builtin/` |

### 存储层级

Skill 按以下优先级加载:

```
1. packages/ccode/src/skill/builtin/*/SKILL.md  # 内置 (最低优先级)
2. ~/.claude/skills/*/SKILL.md                   # 全局
3. .claude/skills/*/SKILL.md                     # 项目级 (Claude Code 兼容)
4. .codecoder/skills/*/SKILL.md                  # 项目级 (最高优先级)
```

### 数据结构

```typescript
export const Info = z.object({
  name: z.string(),        // 技能名称: "tdd", "git-workflow"
  description: z.string(), // 何时使用此技能
  location: z.string(),    // SKILL.md 文件路径
})
```

### SKILL.md 格式

```markdown
---
name: tdd
description: Use when implementing features or fixing bugs - enforces test-first methodology
---

# Test-Driven Development

## When to Use
- Implementing new features
- Fixing bugs
- Refactoring code

## Workflow

### 1. RED - Write failing test
Write a test that describes the desired behavior.

### 2. GREEN - Make it pass
Write minimal code to make the test pass.

### 3. REFACTOR - Improve
Refactor while keeping tests green.

## Best Practices
- One assertion per test
- Test behavior, not implementation
- Keep tests fast
```

### 内置 Skill

```
packages/ccode/src/skill/builtin/
├── best-practices/SKILL.md   # 编码最佳实践
├── crystallize/SKILL.md      # 知识结晶化
├── debugging/SKILL.md        # 调试方法论
└── git-workflow/SKILL.md     # Git 工作流
```

### Agent vs Skill 对比

| 维度 | Agent | Skill |
| ------ | ------- | ------- |
| 定义 | 独立执行单元 | 可共享的知识/方法论 |
| 身份 | 有自己的身份和权限 | 无身份，被引用 |
| 调用方式 | 通过 Task 工具或 @mention | 自动加载到 Agent 上下文 |
| 执行 | 主动执行任务 | 被动提供指导 |
| 类比 | 工程师 | 培训手册 |

### 使用方式

```bash
# 查看可用 Skill
ccode /skills

# 调用 Skill (在 Agent 会话中)
> /tdd  # 加载 TDD 技能到当前上下文
```

---

## TOOL - 执行工具

### 定义

Tool 是 Agent 与外部环境交互的接口，提供了读写文件、执行命令、搜索代码、访问网络等能力。每个 Tool 都有明确的输入参数和输出格式，Agent 通过调用 Tool 来完成实际操作。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Tool 定义 | `packages/ccode/src/tool/tool.ts` |
| Tool 注册 | `packages/ccode/src/tool/registry.ts` |
| 各 Tool 实现 | `packages/ccode/src/tool/*.ts` |
| 自定义 Tool | `.codecoder/tools/*.{js,ts}` |

### 数据结构

```typescript
export interface Info<Parameters extends z.ZodType, Metadata> {
  id: string                    // Tool 标识符: "bash", "read", "edit"
  init: (ctx?: InitContext) => Promise<{
    description: string         // Tool 描述 (用于 LLM 理解)
    parameters: Parameters      // 输入参数 Schema (Zod)
    execute(                    // 执行函数
      args: z.infer<Parameters>,
      ctx: Context,
    ): Promise<{
      title: string             // 执行标题
      metadata: Metadata        // 元数据
      output: string            // 输出内容
      attachments?: FilePart[]  // 附件 (图片等)
    }>
  }>
}
```

### 内置 Tool 列表 (20+)

#### 文件操作

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `read` | 读取文件 | 读取文件内容、支持分页 |
| `write` | 写入文件 | 创建或覆盖文件 |
| `edit` | 编辑文件 | 精确字符串替换 |
| `glob` | 文件搜索 | 按模式匹配文件路径 |
| `grep` | 内容搜索 | 正则表达式搜索文件内容 |
| `ls` | 目录列表 | 列出目录内容 |

#### 系统操作

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `bash` | 执行命令 | 运行 Shell 命令 |
| `task` | 子 Agent | 启动子 Agent 执行任务 |

#### 网络操作

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `webfetch` | 获取网页 | 抓取 URL 内容 |
| `websearch` | 网络搜索 | 搜索引擎查询 |
| `codesearch` | 代码搜索 | 语义代码搜索 |

#### 任务管理

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `todowrite` | 写入任务 | 创建/更新任务列表 |
| `todoread` | 读取任务 | 读取任务列表 |

#### 交互操作

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `question` | 提问用户 | 向用户提问并获取回答 |
| `skill` | 加载技能 | 加载 SKILL.md 到上下文 |

#### 计划模式

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `plan_enter` | 进入计划 | 进入计划模式 |
| `plan_exit` | 退出计划 | 退出计划模式并执行 |

#### 高级操作

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `apply_patch` | 应用补丁 | 应用 unified diff 格式补丁 |
| `lsp` | LSP 操作 | 语言服务协议操作 (实验性) |
| `batch` | 批量操作 | 批量执行多个操作 (实验性) |
| `network_analyzer` | 网络分析 | 分析网络请求 |
| `get_credential` | 获取凭证 | 安全获取凭证 |

#### Reach 系列 (社交媒体)

| Tool | 描述 | 用途 |
| ------ | ------ | ------ |
| `reach_youtube` | YouTube | 获取视频信息 |
| `reach_bilibili` | Bilibili | 获取 B 站内容 |
| `reach_twitter` | Twitter/X | 获取推文 |
| `reach_reddit` | Reddit | 获取 Reddit 内容 |
| `reach_xiaohongshu` | 小红书 | 获取笔记 |
| `reach_douyin` | 抖音 | 获取视频 |
| `reach_linkedin` | LinkedIn | 获取职业信息 |
| `reach_bosszhipin` | Boss 直聘 | 获取招聘信息 |
| `reach_rss` | RSS | 获取 RSS 订阅 |

### Tool 执行流程

```
Agent 发起 Tool 调用
         │
         ▼
┌─────────────────────┐
│ 1. 参数验证 (Zod)    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 2. PreToolUse Hook  │ ←── 可被 Hook 阻止
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 3. CLOSE 评估       │ ←── 自主模式下评估风险
│   (Autonomous 模式)  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 4. 执行 Tool        │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 5. PostToolUse Hook │ ←── 可被 Hook 阻止
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ 6. 输出截断         │ ←── 防止输出过长
└──────────┬──────────┘
           │
           ▼
     返回结果给 Agent
```

### Hook 集成

Tool 执行前后会触发 Hook，允许自定义行为:

```typescript
// PreToolUse Hook - 执行前
const preResult = await Hook.run("PreToolUse", {
  tool: "bash",
  input: { command: "rm -rf /" },
  sessionID: "...",
})
if (preResult.blocked) {
  // 操作被阻止
}

// PostToolUse Hook - 执行后
const postResult = await Hook.run("PostToolUse", {
  tool: "edit",
  input: { filePath: "..." },
  output: "...",
  diff: "...",
})
```

### 权限控制

Tool 的使用受 Agent 权限规则控制:

```typescript
// Agent 权限配置示例
const permission = PermissionNext.fromConfig({
  "*": "allow",              // 默认允许
  bash: "ask",               // Bash 需要确认
  write: "allow",            // 写入允许
  external_directory: {
    "*": "deny",             // 外部目录默认拒绝
    "/tmp/*": "allow",       // /tmp 允许
  },
})
```

### 自定义 Tool

在 `.codecoder/tools/` 目录下创建 Tool:

```typescript
// .codecoder/tools/my-tool.ts
import { Tool } from "ccode/tool"
import z from "zod"

export const myTool = Tool.define("my_tool", {
  description: "My custom tool description",
  parameters: z.object({
    input: z.string().describe("Input parameter"),
  }),
  async execute(args, ctx) {
    // 执行逻辑
    return {
      title: "My Tool Result",
      metadata: {},
      output: `Processed: ${args.input}`,
    }
  },
})
myTool.tool = myTool // 标记为 Tool
```

### 与其他概念的关系

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                  AGENT                                      │
│                           (理解意图、决策)                                   │
│                                    │                                        │
│                                    │ 调用                                   │
│                                    ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                              TOOL                                     │  │
│  │                      (执行具体操作)                                    │  │
│  │                                                                       │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │  │
│  │  │    Bash     │  │    Read     │  │    Edit     │  │    Task     │ │  │
│  │  │  执行命令   │  │  读取文件   │  │  编辑文件   │  │  子 Agent   │ │  │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘ │  │
│  │         │                │                │                │        │  │
│  │         └────────────────┼────────────────┼────────────────┘        │  │
│  │                          │                │                          │  │
│  │                          ▼                ▼                          │  │
│  │                    ┌──────────────────────────┐                      │  │
│  │                    │     外部环境              │                      │  │
│  │                    │  文件系统 / Shell / 网络  │                      │  │
│  │                    └──────────────────────────┘                      │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘
```

### 设计原则

1. **参数验证**: 所有输入通过 Zod Schema 验证，确保类型安全
2. **Hook 集成**: 支持 PreToolUse/PostToolUse Hook，允许自定义行为
3. **权限控制**: 受 Agent 权限规则控制，支持细粒度权限
4. **输出截断**: 自动截断过长输出，防止上下文溢出
5. **可扩展**: 支持自定义 Tool，放在 `.codecoder/tools/` 目录

---

## CHANNEL - 消息渠道

### 定义

Channel 是连接外部世界与 CodeCoder 的消息适配器，负责协议转换和消息路由。Channel 属于 Rust 微服务层，处理高确定性的协议解析任务。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Channel 服务 | `services/zero-channels/src/lib.rs` |
| 各渠道实现 | `services/zero-channels/src/{telegram,discord,slack,...}/` |

### 服务端口

- **zero-channels**: `:4431`

### 架构

```
User IM ──► webhook/polling ──► zero-channels ──► CodeCoder
                                     ↓                 ↓
User ◄──────── send ◄─────── OutboundRouter ◄── Response
```

### 支持的渠道 (12 个)

| 渠道 | 模块 | 说明 |
| ------ | ------ | ------ |
| Telegram | `telegram/` | Telegram Bot (长轮询) |
| Discord | `discord/` | Discord Bot |
| Slack | `slack/` | Slack App |
| 飞书 | `feishu.rs` | Feishu/Lark |
| 企业微信 | `wecom.rs` | WeCom |
| 钉钉 | `dingtalk.rs` | DingTalk |
| WhatsApp | `whatsapp.rs` | WhatsApp Business |
| Email | `email.rs` | 邮件 |
| iMessage | `imessage.rs` | Apple iMessage |
| Matrix | `matrix.rs` | Matrix 协议 |
| CLI | `cli.rs` | 本地测试 |
| SSE | `sse.rs` | Server-Sent Events |

### 核心组件

```rust
// 消息类型
pub struct ChannelMessage {
    pub channel_type: ChannelType,  // telegram, discord, etc.
    pub user_id: String,
    pub content: MessageContent,
    pub attachments: Vec<Attachment>,
}

// 出站路由
pub struct OutboundRouter {
    telegram: Option<Arc<TelegramChannel>>,
    discord: Option<Arc<DiscordChannel>>,
    slack: Option<Arc<SlackChannel>>,
    // ...
}

// Channel trait
pub trait Channel {
    async fn send(&self, message: OutgoingMessage) -> ChannelResult<SendResult>;
    async fn listen<F>(&self, callback: F) -> ChannelResult<()>;
}
```

### 配置示例

```json
// ~/.codecoder/config.json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "allowed_users": ["123456789"]
    },
    "feishu": {
      "enabled": true,
      "app_id": "cli_xxx",
      "app_secret": "xxx"
    }
  }
}
```

### 设计原则

1. **确定性任务**: 协议解析格式固定，用 Rust 保证安全和性能
2. **适配器模式**: 每个渠道实现统一的 `Channel` trait
3. **异步处理**: 使用 Tokio 处理并发连接
4. **格式转换**: 将各平台消息统一为 `ChannelMessage`

---

## MEMORY - 记忆系统

### 定义

Memory 是 Agent 的持久化知识存储，采用透明的双层 Markdown 架构。设计原则是"透明可读 > 智能黑盒"，拒绝向量数据库的复杂性。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Memory 模块 | `packages/ccode/src/memory-markdown/` |
| 流层存储 | `memory/daily/{YYYY-MM-DD}.md` |
| 沉积层存储 | `memory/MEMORY.md` |

### 双层架构

```
┌───────────────────────────────────────────────────────────────────────────┐
│                        双层记忆架构 (流 + 沉积)                             │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   第一层: 流层 (Stream/Flow)           第二层: 沉积层 (Sediment)           │
│   ─────────────────────────           ─────────────────────────           │
│                                                                            │
│   memory/daily/2026-03-01.md          memory/MEMORY.md                    │
│   ┌─────────────────────────┐         ┌─────────────────────────┐        │
│   │ ## 10:30 - 任务A        │         │ ## 用户偏好              │        │
│   │ - 执行了什么            │  整合   │ - 代码风格: 函数式       │        │
│   │ - 结果如何              │ ──────► │                          │        │
│   │                         │  提炼   │ ## 关键决策              │        │
│   │ ## 14:00 - 任务B        │         │ - 2026-02-20: ...        │        │
│   │ ...                     │         │                          │        │
│   └─────────────────────────┘         └─────────────────────────┘        │
│                                                                            │
│   特点:                                特点:                              │
│   • 仅追加、不可修改                   • 可编辑、结构化                   │
│   • 按时间线记录                       • 代表当前真实状态                 │
│   • 类比: 河流 (flow)                  • 类比: 沉积岩                     │
│                                                                            │
└───────────────────────────────────────────────────────────────────────────┘
```

### 模块结构

```
packages/ccode/src/memory-markdown/
├── index.ts        # 入口和导出
├── types.ts        # 类型定义
├── config.ts       # 存储配置
├── storage.ts      # 存储提供者
├── project.ts      # 项目检测
├── daily.ts        # 流层操作 (每日笔记)
├── long-term.ts    # 沉积层操作 (长期记忆)
├── loader.ts       # 上下文加载
├── consolidate.ts  # 整合/提炼
└── util.ts         # 工具函数
```

### API 示例

```typescript
// 流层操作
import { appendDailyNote, getTodayNotes, loadDailyNotes } from "@/memory-markdown"

// 追加今日笔记
await appendDailyNote({
  type: "task",
  content: "完成了用户认证模块",
  metadata: { agent: "build" }
})

// 获取今日笔记
const notes = await getTodayNotes()

// 沉积层操作
import { loadCategory, updateCategory, mergeToCategory } from "@/memory-markdown"

// 读取用户偏好
const prefs = await loadCategory("用户偏好")

// 更新偏好
await updateCategory("用户偏好", "- 代码风格: 函数式\n- 语言: 中文")

// 整合
import { consolidateMemory } from "@/memory-markdown"

// 将近期笔记整合到长期记忆
await consolidateMemory({ days: 7 })
```

### 配置

```json
// ~/.codecoder/config.json
{
  "memory": {
    "storage": {
      "type": "local",
      "basePath": "./memory"
    }
  }
}

// 或环境变量
// CCODE_MEMORY_DIR=/path/to/memory
// CCODE_MEMORY_PROJECT_ID=my-project
```

### 设计原则

1. **透明可读**: 纯 Markdown，人类和 AI 都能直接阅读编辑
2. **Git 友好**: 所有记忆变更可追溯、可回滚
3. **无嵌入检索**: 拒绝向量数据库的复杂性
4. **分层职责**: 流层记录过程，沉积层记录结论
5. **项目隔离**: 支持按项目 ID 隔离记忆

---

## WORKFLOW - 工作流引擎

### 定义

Workflow 是事件驱动的自动化引擎，处理定时任务 (Cron)、Webhook 触发和 Git 事件，属于 Rust 微服务层。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Workflow 服务 | `services/zero-workflow/src/lib.rs` |
| Cron 调度器 | `services/zero-workflow/src/scheduler.rs` |
| Webhook 处理 | `services/zero-workflow/src/webhook.rs` |
| Hands 系统 | `services/zero-workflow/src/hands/` |

### 服务端口

- **zero-workflow**: `:4432`

### 功能模块

```rust
pub mod dsl;              // 工作流 DSL
pub mod scheduler;        // Cron 定时任务
pub mod webhook;          // Webhook 触发器
pub mod github;           // GitHub 事件 (PR, Issue)
pub mod gitlab;           // GitLab 事件 (MR)
pub mod review_bridge;    // 代码审查桥接
pub mod ticket_bridge;    // Issue 自动创建
pub mod monitor_bridge;   // 竞品监控
pub mod economic_bridge;  // 经济数据监控
pub mod risk_monitor;     // 风险监控
pub mod trading_review;   // 交易复盘
pub mod hands;            // Hands 自主代理系统
```

### 触发方式

| 类型 | 说明 | 示例 |
| ------ | ------ | ------ |
| Cron | 定时任务 | `"0 9 * * *"` (每天 9 点) |
| Webhook | HTTP 触发 | `POST /webhook` |
| GitHub | Git 事件 | PR created, Issue opened |
| GitLab | Git 事件 | MR created |
| Monitor | 监控任务 | 竞品变更检测 |

### 配置示例

```json
// ~/.codecoder/config.json
{
  "workflow": {
    "cron": {
      "enabled": true,
      "tasks": [
        {
          "id": "daily-report",
          "schedule": "0 9 * * *",
          "command": "generate-daily-report"
        }
      ]
    },
    "webhook": {
      "enabled": true,
      "secret": "your-webhook-secret"
    },
    "git": {
      "enabled": true,
      "github_token": "ghp_xxx",
      "github_secret": "webhook-secret"
    },
    "hands": {
      "enabled": true
    }
  }
}
```

### 与 Agent 的关系

```
┌───────────────────────────────────────────────────────────────┐
│                      WORKFLOW                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │ Cron 触发   │  │ Webhook 触发 │  │ Git 事件触发 │        │
│  │ "0 9 * * *" │  │ POST /webhook│  │ PR created   │        │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘        │
│         │                 │                 │                 │
│         └─────────────────┼─────────────────┘                 │
│                           │                                   │
│                           ▼                                   │
│                   HTTP POST /api/chat                         │
└───────────────────────────┼───────────────────────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │  ccode AGENT  │
                    │  (执行任务)    │
                    └───────────────┘
```

---

## HAND - 自主代理

### 定义

Hand 是持久化、有状态的自主 AI 代理，通过 `HAND.md` 文件声明式定义，按 Cron 调度执行。Hand 是 CodeCoder 最高层的抽象，组合了 WORKFLOW + AGENT + MEMORY，实现了"零代码编写定时 AI 任务"。

### 文件位置

| 类型 | 路径 |
| ------ | ------ |
| Hands 模块 | `services/zero-workflow/src/hands/mod.rs` |
| Manifest 解析 | `services/zero-workflow/src/hands/manifest.rs` |
| 执行器 | `services/zero-workflow/src/hands/executor.rs` |
| 状态存储 | `services/zero-workflow/src/hands/state.rs` |
| CLOSE 决策 | `services/zero-workflow/src/hands/close.rs` |
| 风险评估 | `services/zero-workflow/src/hands/risk.rs` |

### 存储位置

```
~/.codecoder/hands/        # 全局 Hands
.codecoder/hands/          # 项目级 Hands
services/memory/hands/     # Hand 执行记忆
```

### HAND.md 格式

```markdown
---
id: "market-sentinel"
name: "Market Sentinel"
version: "1.0.0"
schedule: "0 */30 * * * *"        # 每 30 分钟
agent: "macro"                     # 使用 macro Agent
enabled: true
autonomy:
  level: "crazy"                   # 自主等级
  unattended: true                 # 无人值守
  max_iterations: 5                # 最大迭代次数
decision:
  use_close: true                  # 使用 CLOSE 决策框架
  web_search: true                 # 允许网络搜索
  evolution: true                  # 允许自我演进
resources:
  max_tokens: 100000               # 最大 Token
  max_cost_usd: 5.0                # 最大成本 (美元)
  max_duration_sec: 600            # 最大执行时间 (秒)
memory_path: "hands/market-sentinel/{date}.md"
notification:
  on_success: false
  on_failure: true
  channel: "telegram"
---

# Market Sentinel

监控宏观经济数据变化，识别市场转折信号。

## 任务目标

1. 每 30 分钟检查关键经济指标
2. 对比历史数据识别异常
3. 生成分析报告

## 数据来源

- PMI 数据
- GDP 增速
- 利率变化
```

### 自主等级 (Autonomy Level)

| 等级 | 分数 | CLOSE 阈值 | 描述 |
| ------ | ------ | ------------ | ------ |
| `lunatic` | 95 | (5.0, 3.0) | 完全自主，无需人工干预 |
| `insane` | 85 | (5.5, 3.5) | 高度自主，关键决策前通知 |
| `crazy` | 75 | (6.0, 4.0) | 显著自主，半自动执行 |
| `wild` | 60 | (6.5, 4.5) | 部分自主，仅执行简单任务 |
| `bold` | 40 | (7.0, 5.0) | 谨慎自主，仅执行已定义步骤 |
| `timid` | 15 | (8.0, 6.0) | 基本不自主，仅收集信息 |

### 核心组件

```rust
// 自主等级
pub enum AutonomyLevel {
    Lunatic, Insane, Crazy, Wild, Bold, Timid
}

// Hand 配置
pub struct HandConfig {
    pub id: String,
    pub name: String,
    pub schedule: String,        // Cron 表达式
    pub agent: String,           // 使用的 Agent
    pub enabled: bool,
    pub autonomy: AutonomyConfig,
    pub decision: DecisionConfig,
    pub resources: ResourceLimits,
    pub memory_path: String,
    pub notification: HandNotificationConfig,
}

// CLOSE 决策评估器
pub struct CloseEvaluator {
    pub criteria: CloseCriteria,
}

// 风险评估
pub struct RiskEvaluator {
    pub threshold: RiskThreshold,
}
```

### 与其他概念的关系

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                  HAND                                       │
│                        (持久化自主代理)                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  HAND.md 定义                                                         │  │
│  │  - 调度规则 (Cron)        ──► WORKFLOW 调度                          │  │
│  │  - 使用哪个 AGENT         ──► AGENT 执行                             │  │
│  │  - 自主等级 (Autonomy)                                               │  │
│  │  - CLOSE 决策框架配置                                                │  │
│  │  - 资源限制 (tokens, cost, duration)                                 │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    │ 调用                                   │
│                                    ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  AGENT (如 macro)                                                     │  │
│  │  └── PROMPT (定义行为)                                               │  │
│  │  └── SKILL (可选引用)                                                │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│                                    │ 读写                                   │
│                                    ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  MEMORY                                                               │  │
│  │  hands/market-sentinel/{date}.md  (Hand 专属记忆)                    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  由 WORKFLOW (Hands Scheduler) 调度执行                                     │
└────────────────────────────────────────────────────────────────────────────┘
```

### 使用方式

```bash
# 创建 Hand
mkdir -p ~/.codecoder/hands/market-sentinel
cat > ~/.codecoder/hands/market-sentinel/HAND.md << 'EOF'
---
id: "market-sentinel"
name: "Market Sentinel"
schedule: "0 */30 * * * *"
agent: "macro"
enabled: true
autonomy:
  level: "crazy"
---
# Market Sentinel
监控宏观经济数据...
EOF

# 启动服务 (Hands 自动加载)
./ops.sh start all

# 查看 Hand 状态
curl http://localhost:4432/api/v1/hands

# 手动触发执行
curl -X POST http://localhost:4432/api/v1/hands/market-sentinel/run
```

### 设计创新

1. **零代码定义**: 通过 HAND.md 声明式定义 AI 代理
2. **CLOSE 决策集成**: 内置祝融说的可持续决策框架
3. **自主等级控制**: 六档自主程度，从 "Timid" 到 "Lunatic"
4. **HITL 内置**: 根据自主等级和风险评估决定是否需要人类确认
5. **资源限制**: 防止失控消耗 (tokens, cost, duration)
6. **专属记忆**: 每个 Hand 有独立的执行记忆

---

## 概念对比表

| 概念 | 定义 | 语言/位置 | 职责 | 依赖关系 |
| ------ | ------ | ----------- | ------ | ---------- |
| **AGENT** | AI 执行单元 | TypeScript/ccode | 理解意图、执行任务 | 包含 PROMPT，调用 TOOL，可引用 SKILL |
| **PROMPT** | Agent 行为定义 | .txt 文件 | 定义身份、职责、决策 | 是 AGENT 的一部分 |
| **SKILL** | 可复用知识 | SKILL.md 文件 | 跨 Agent 共享方法论 | 被 AGENT 动态引用 |
| **TOOL** | 执行工具 | TypeScript/ccode | 与外部环境交互 | 被 AGENT 调用，受权限控制 |
| **CHANNEL** | 消息渠道 | Rust/zero-channels | 连接外部世界与 ccode | 调用 AGENT |
| **MEMORY** | 记忆系统 | TypeScript/ccode | 持久化知识存储 | 被 AGENT 读写 |
| **WORKFLOW** | 自动化引擎 | Rust/zero-workflow | Cron/Webhook/Git 调度 | 触发 AGENT 执行 |
| **HAND** | 自主代理 | HAND.md + Rust | 持久化有状态 AI | 组合 WORKFLOW + AGENT + MEMORY |

### 确定性划分

| 概念 | 确定性 | 原因 |
| ------ | -------- | ------ |
| CHANNEL | 高 | 协议解析格式固定 |
| WORKFLOW | 高 | Cron/Webhook 规则明确 |
| MEMORY (存储) | 高 | 文件读写操作确定 |
| TOOL (执行) | 高 | 明确的输入输出规范 |
| AGENT | 低 | 需要理解和推理 |
| PROMPT | 低 | 定义不确定性任务的行为 |
| SKILL | 低 | 提供方法论指导 |
| HAND | 混合 | 调度确定，执行不确定 |

---

## 层次架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Layer 6: HAND (最高抽象层)                                                  │
│  声明式定义持久化自主代理，集成 CLOSE 决策框架                               │
│  组合: WORKFLOW + AGENT + MEMORY                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 5: WORKFLOW (自动化调度层)                                            │
│  Cron 定时、Webhook 触发、Git 事件监听                                      │
│  服务: zero-workflow (Rust) :4432                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 4: AGENT + SKILL (智能执行层)                                         │
│  31 个专用 Agent + 可复用技能库                                             │
│  包: packages/ccode (TypeScript)                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 3: TOOL (工具执行层)                                                  │
│  Bash, Read, Edit, Grep, Task, WebFetch... (20+ 内置工具)                   │
│  Agent 通过 Tool 与外部环境交互                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 2: PROMPT + MEMORY (知识层)                                           │
│  Agent 行为定义 + 双层 Markdown 记忆                                        │
│  存储: .txt 文件 + memory/ 目录                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│  Layer 1: CHANNEL (接入层)                                                   │
│  Telegram/Discord/Slack/飞书/钉钉/WhatsApp/...                              │
│  服务: zero-channels (Rust) :4431                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 数据流

```
用户消息
    │
    ▼
CHANNEL (协议转换)
    │
    ▼
AGENT (理解 + 决策)
    │
    ├──► 读取 PROMPT (行为定义)
    │
    ├──► 加载 SKILL (可复用能力)
    │
    ├──► 调用 TOOL (执行操作)
    │         │
    │         ├──► Bash (执行命令)
    │         ├──► Read/Edit/Write (文件操作)
    │         ├──► Grep/Glob (搜索)
    │         ├──► Task (子 Agent)
    │         └──► WebFetch (网络请求)
    │
    ├──► 读写 MEMORY (记忆系统)
    │
    ▼
响应用户
```

```
定时任务 / Webhook / Git 事件
    │
    ▼
WORKFLOW (触发调度)
    │
    ▼
HAND (自主决策)
    │
    ├──► CLOSE 评估
    │
    ├──► 风险评估
    │
    ├──► 调用 AGENT
    │         │
    │         └──► 调用 TOOL (受 CLOSE 控制)
    │
    ├──► 写入专属 MEMORY
    │
    ▼
完成 / 通知
```

---

## 概念关系详解

本节详细阐述八个核心概念之间的包含关系和调用关系。

### 包含关系 (Composition)

```
┌────────────────────────────────────────────────────────────────────────────┐
│                                  HAND                                       │
│                            (最高层组合)                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  HAND.md 配置文件                                                     │  │
│  │  ├── schedule (Cron)  ──────────────► 依赖 WORKFLOW 调度             │  │
│  │  ├── agent: "macro"   ──────────────► 指定使用的 AGENT               │  │
│  │  ├── memory_path      ──────────────► 指定 MEMORY 路径               │  │
│  │  └── autonomy/decision ─────────────► CLOSE 决策配置                 │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│                                 AGENT                                       │
│                            (核心执行单元)                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  Agent.Info 数据结构                                                  │  │
│  │  ├── name: string                                                    │  │
│  │  ├── prompt: string   ──────────────► 内嵌 PROMPT (行为定义)          │  │
│  │  ├── permission       ──────────────► 控制 TOOL 调用权限              │  │
│  │  ├── temperature                                                      │  │
│  │  └── options                                                          │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│                                WORKFLOW                                     │
│                             (调度引擎)                                       │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │  WorkflowService                                                      │  │
│  │  ├── scheduler        ──────────────► Cron 任务调度                   │  │
│  │  ├── webhook          ──────────────► HTTP 触发器                     │  │
│  │  ├── github/gitlab    ──────────────► Git 事件监听                    │  │
│  │  └── hands            ──────────────► 内含 HAND 调度器                │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────────┘
```

**包含关系总结**:

| 父概念 | 包含 | 说明 |
| ------ | ------ | ------ |
| **AGENT** | PROMPT | Prompt 是 Agent 的属性，定义其行为 |
| **AGENT** | permission | 权限规则控制 Tool 调用 |
| **WORKFLOW** | HandsScheduler | Workflow 服务内含 Hands 调度 |
| **HAND** | agent 引用 | Hand 配置指定使用哪个 Agent |
| **HAND** | memory_path | Hand 配置指定 Memory 路径 |
| **HAND** | schedule | Hand 配置指定 Cron 表达式 |

### 调用关系 (Invocation)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              调用链路图                                      │
└─────────────────────────────────────────────────────────────────────────────┘

                          ┌─────────────────┐
                          │  外部触发源      │
                          │ User/Cron/Hook  │
                          └────────┬────────┘
                                   │
          ┌────────────────────────┼────────────────────────┐
          │                        │                        │
          ▼                        ▼                        ▼
   ┌──────────────┐      ┌──────────────┐        ┌──────────────┐
   │   CHANNEL    │      │   WORKFLOW   │        │   直接调用    │
   │  (IM 消息)   │      │  (定时/事件)  │        │   (CLI/API)  │
   └──────┬───────┘      └──────┬───────┘        └──────┬───────┘
          │                     │                       │
          │                     │                       │
          │         ┌───────────┴───────────┐           │
          │         │                       │           │
          │         ▼                       ▼           │
          │  ┌──────────────┐      ┌──────────────┐     │
          │  │    HAND      │      │  直接触发    │     │
          │  │ (自主决策)   │      │  AGENT      │     │
          │  └──────┬───────┘      └──────────────┘     │
          │         │                                   │
          │         │ CLOSE 评估后                      │
          │         ▼                                   │
          └────────►┌──────────────────────────────────┐◄┘
                    │            AGENT                 │
                    │         (智能执行)                │
                    └──────────────┬───────────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
             ┌──────────┐   ┌──────────┐   ┌──────────┐
             │  SKILL   │   │   TOOL   │   │  MEMORY  │
             │ (加载)   │   │  (调用)  │   │ (读写)   │
             └──────────┘   └────┬─────┘   └──────────┘
                                 │
                    ┌────────────┼────────────┐
                    │            │            │
                    ▼            ▼            ▼
              ┌─────────┐  ┌─────────┐  ┌─────────┐
              │  Bash   │  │  Read   │  │  Task   │
              │ (命令)  │  │ (文件)  │  │(子Agent)│
              └─────────┘  └─────────┘  └────┬────┘
                                             │
                                             ▼
                                      ┌──────────────┐
                                      │    AGENT     │
                                      │   (递归)     │
                                      └──────────────┘
```

### 详细调用关系表

| 调用方 | 被调用方 | 调用方式 | 说明 |
| ------ | ------ | ------ | ------ |
| **CHANNEL** | AGENT | HTTP API | 消息转发到 ccode API |
| **WORKFLOW** | AGENT | HTTP API | 定时/事件触发 Agent |
| **WORKFLOW** | HAND | 内部调度 | HandsScheduler 调度 Hand |
| **HAND** | AGENT | HTTP API | Hand 执行时调用指定 Agent |
| **AGENT** | TOOL | 函数调用 | Agent 通过 Tool 执行操作 |
| **AGENT** | SKILL | 动态加载 | 加载 SKILL.md 到上下文 |
| **AGENT** | MEMORY | 函数调用 | 读写记忆系统 |
| **TOOL (Task)** | AGENT | 递归调用 | Task 工具启动子 Agent |
| **TOOL** | 外部环境 | 系统调用 | 文件系统、Shell、网络 |

### 概念分层与依赖方向

```
                    依赖方向 (上层依赖下层)
                           ▲
                           │
┌──────────────────────────┴───────────────────────────────────────────────┐
│  Layer 6: HAND                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  依赖: WORKFLOW (调度) + AGENT (执行) + MEMORY (状态)                │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Layer 5: WORKFLOW                                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  依赖: AGENT (通过 HTTP 调用)                                        │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Layer 4: AGENT                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  包含: PROMPT                                                        │ │
│  │  依赖: TOOL + SKILL + MEMORY                                         │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Layer 3: TOOL                                                           │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  依赖: 外部环境 (文件系统/Shell/网络)                                 │ │
│  │  特殊: Task Tool 可递归调用 AGENT                                    │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Layer 2: SKILL + MEMORY + PROMPT                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  SKILL: 独立存在，被 AGENT 加载                                      │ │
│  │  MEMORY: 独立存在，被 AGENT 读写                                     │ │
│  │  PROMPT: 嵌入 AGENT，不独立存在                                      │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
├──────────────────────────────────────────────────────────────────────────┤
│  Layer 1: CHANNEL                                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  依赖: AGENT (通过 HTTP 调用)                                        │ │
│  │  被依赖: 外部 IM 平台                                                │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

### 关系类型图例

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              关系类型说明                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ══════════►  包含关系 (Composition)    A 包含 B，B 是 A 的一部分          │
│                                                                             │
│   ──────────►  调用关系 (Invocation)     A 调用 B 的功能                    │
│                                                                             │
│   - - - - - ►  引用关系 (Reference)      A 引用/加载 B                      │
│                                                                             │
│   ◄─────────►  双向关系                  A 和 B 互相调用                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

                              ┌──────────┐
                              │   HAND   │
                              └────┬─────┘
                                   │ 调用
              ┌────────────────────┼────────────────────┐
              │                    │                    │
              ▼                    ▼                    ▼
       ┌──────────┐         ┌──────────┐         ┌──────────┐
       │ WORKFLOW │         │  AGENT   │         │  MEMORY  │
       │  (调度)  │         │  (执行)  │         │  (状态)  │
       └────┬─────┘         └────┬─────┘         └──────────┘
            │                    │ 包含
            │ 调用               ▼
            │              ┌──────────┐
            └─────────────►│  PROMPT  │
                           └──────────┘
                                 ▲
                                 │ 嵌入
       ┌──────────┐         ┌───┴──────┐         ┌──────────┐
       │ CHANNEL  │────────►│  AGENT   │◄────────│  SKILL   │
       │  (接入)  │  调用   │  (核心)  │  引用   │  (知识)  │
       └──────────┘         └────┬─────┘         └──────────┘
                                 │ 调用
                                 ▼
                           ┌──────────┐
                           │   TOOL   │
                           │  (工具)  │
                           └────┬─────┘
                                │ 操作
                                ▼
                        ┌────────────────┐
                        │   外部环境      │
                        │ 文件/Shell/网络 │
                        └────────────────┘
```

### 完整关系矩阵

|  | AGENT | PROMPT | SKILL | TOOL | CHANNEL | MEMORY | WORKFLOW | HAND |
|--|-------|--------|-------|------|---------|--------|----------|------|
| **AGENT** | - | **包含** | 引用 | **调用** | 被调用 | 读写 | 被调用 | 被调用 |
| **PROMPT** | 嵌入于 | - | - | - | - | - | - | - |
| **SKILL** | 被加载 | - | - | - | - | - | - | - |
| **TOOL** | 被调用 | - | - | Task递归 | - | 可读写 | - | - |
| **CHANNEL** | **调用** | - | - | - | - | - | - | - |
| **MEMORY** | 被读写 | - | - | 被读写 | - | - | - | 被读写 |
| **WORKFLOW** | **调用** | - | - | - | - | - | - | **调度** |
| **HAND** | **调用** | - | - | - | - | **读写** | 依赖调度 | - |

**图例**: **粗体** = 主要关系，普通 = 次要关系，`-` = 无直接关系

### 核心关系一句话总结

```
CHANNEL 接收消息 ──► AGENT 理解意图
                         │
                         ├── 读取 PROMPT (内嵌行为定义)
                         ├── 加载 SKILL (外部知识)
                         ├── 调用 TOOL (执行操作)
                         │       └── Task Tool 可递归调用 AGENT
                         └── 读写 MEMORY (持久化)

WORKFLOW 定时触发 ──► HAND 自主决策 ──► AGENT 执行
                          │
                          └── 写入专属 MEMORY
```

**六大核心关系**:

1. **PROMPT ⊂ AGENT** — Prompt 是 Agent 的内嵌属性
2. **AGENT → TOOL** — Agent 通过 Tool 执行操作
3. **AGENT ← SKILL** — Agent 按需加载 Skill
4. **AGENT ↔ MEMORY** — Agent 读写 Memory
5. **CHANNEL/WORKFLOW → AGENT** — 外部触发 Agent 执行
6. **HAND = WORKFLOW + AGENT + MEMORY** — Hand 是最高层组合

---

## 附录

### 相关文档

- [ARCHITECTURE.md](./ARCHITECTURE.md) - 系统架构总览
- [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md) - ccode 与 zero-* 的关系
- [DESIGN_PHILOSOPHY.md](./DESIGN_PHILOSOPHY.md) - 设计哲学

### 配置文件位置

| 文件 | 路径 | 用途 |
| ------ | ------ | ------ |
| 主配置 | `~/.codecoder/config.json` | 全局配置 |
| 密钥 | `~/.codecoder/secrets.json` | 凭证存储 |
| Agent 配置 | 主配置的 `agent` 字段 | 自定义 Agent |
| Channel 配置 | 主配置的 `channels` 字段 | 渠道配置 |
| Workflow 配置 | 主配置的 `workflow` 字段 | 工作流配置 |
| Memory 配置 | 主配置的 `memory` 字段 | 记忆配置 |

### 端口分配

| 服务 | 端口 | 职责 |
| ------ | ------ | ------ |
| ccode API | 4400 | Agent 引擎、记忆系统 |
| Web Frontend | 4401 | Web 界面 |
| Zero Daemon | 4402 | 进程编排 |
| Whisper | 4403 | 语音转写 |
| MCP Server | 4420 | Model Context Protocol |
| Zero Gateway | 4430 | 认证、路由、配额 |
| Zero Channels | 4431 | IM 渠道适配 |
| Zero Workflow | 4432 | 工作流调度 |
| Zero Browser | 4433 | 浏览器自动化 |
| Zero Trading | 4434 | 交易系统 |
