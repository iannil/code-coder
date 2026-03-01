# CodeCoder

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue.svg)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Bun](https://img.shields.io/badge/Bun-1.3+-pink.svg)](https://bun.sh/)

[English](README.md)

---

## 超越代码补全

**CodeCoder** 是一个融合工程能力与决策智慧的 AI 驱动工作台。

基于**祝融说 (Zhurongsuo)** 哲学的观察者理论：每次观察都是将无限可能性坍缩为具体现实的创造性行为。当你让 CodeCoder 审查代码、设计架构或分析市场时，你不仅仅是在获取答案——你正在参与一个从潜能场中塑造结果的创造性过程。

这一哲学体现在用于可持续决策的 **CLOSE 框架**中：

- **C**lear reversibility assessment — 可逆性评估
- **L**everage option preservation — 杠杆选项保留
- **O**ption space expansion — 选项空间扩展
- **S**ustainability over optimality — 可持续性优于最优性
- **E**xit strategy planning — 退出策略规划

**核心洞察：** 可持续决策优于最优决策。CodeCoder 帮助你保持"可用余量"——那部分未被固化的潜能空间，使你能够适应变化并拥有"再来一次"的能力。

---

## CodeCoder 是什么？

一个超越代码的三层架构 Monorepo：

| 层级 | 定位 | 能力 |
|-------|-------|--------------|
| **工程层** | 代码与系统 | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| **领域层** | 专家知识 | 宏观经济、交易分析、选品策略、AI 工程 |
| **思维层** | 决策框架 | 观察者理论、CLOSE 框架、可持续决策 |

### 核心能力

- **31 个专用 Agent** — 各具专长和个性
- **20+ AI 提供商** — Claude、OpenAI、Google、Amazon Bedrock、xAI 等
- **多种接入方式** — CLI、TUI（终端界面）、Web、IM 机器人
- **MCP 协议** — 本地、远程和 OAuth 认证服务器
- **Rust 微服务** — 用于生产部署的高性能服务
- **Markdown 记忆** — 人类可读、Git 友好的知识持久化

---

## 快速开始

### 前置要求

- **Bun** 1.3+ ([安装](https://bun.sh/docs/installation))
- **Node.js** 22+（部分依赖需要）
- **Rust** 1.75+（可选，用于 Rust 服务）

### 安装

```bash
# 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd code-coder

# 安装依赖
bun install

# 运行 CodeCoder TUI
bun dev

# 或在指定目录运行
bun dev /path/to/your/project
```

### 基本用法

```bash
# 启动 TUI 界面（默认）
bun dev

# CLI 模式运行消息
bun dev run "审查 src/auth 中的认证代码"

# 启动无头 API 服务器
bun dev serve --port 4400

# 使用特定 agent
bun dev run --agent code-reviewer "分析 src/api/server.ts"

# 使用 @ 语法调用 agent
bun dev run "@decision 用 CLOSE 框架分析这个职业选择"
```

---

## 架构概览

```
codecoder/
├── packages/                    # TypeScript 包
│   ├── ccode/                   # 核心 CLI 和业务逻辑
│   │   ├── src/
│   │   │   ├── agent/           # Agent 定义和 Prompts
│   │   │   ├── api/             # HTTP API 服务器 (Hono)
│   │   │   ├── cli/             # CLI 命令和 TUI
│   │   │   ├── config/          # 配置管理
│   │   │   ├── mcp/             # MCP 协议支持
│   │   │   ├── provider/        # AI 提供商适配器
│   │   │   ├── session/         # 会话管理
│   │   │   └── tool/            # 工具定义
│   │   └── test/                # 测试
│   ├── util/                    # 共享工具
│   └── web/                     # Web UI (React + Vite)
├── services/                    # Rust 服务
│   ├── zero-cli/                # CLI 守护进程（组合服务）
│   ├── zero-gateway/            # 认证、路由、配额
│   ├── zero-channels/           # Telegram、Discord、Slack
│   ├── zero-workflow/           # Webhook、Cron、Git
│   ├── zero-common/             # 共享配置
│   ├── zero-agent/              # Agent 执行（库）
│   ├── zero-memory/             # 记忆持久化（库）
│   └── zero-tools/              # 工具定义（库）
├── memory/                      # Markdown 记忆系统
│   ├── MEMORY.md                # 长期知识
│   └── daily/                   # 每日笔记
├── docs/                        # 文档
└── script/                      # 构建脚本
```

### 端口配置

| 服务 | 端口 | 技术栈 |
|---------|------|------------|
| CodeCoder API 服务器 | 4400 | Bun/TypeScript |
| Web 前端 | 4401 | Vite/React |
| Zero CLI 守护进程 | 4402 | Rust（组合） |
| Whisper STT 服务器 | 4403 | Docker |
| MCP 服务器 | 4420 | Model Context Protocol |
| Zero Gateway | 4430 | Rust |
| Zero Channels | 4431 | Rust |
| Zero Workflow | 4432 | Rust |
| Zero Browser | 4433 | Rust |
| Zero Trading | 4434 | Rust |

---

## 核心概念

CodeCoder 建立在 8 个核心概念之上：

### 1. AGENT — 智能执行单元

具有特定角色、行为和决策框架的 AI 代理。每个 agent 都有专门的 prompt 定义其身份和专业能力。

**位置：** `packages/ccode/src/agent/`

### 2. PROMPT — Agent 行为定义

定义每个 agent 的个性、职责和决策模式的文本文件。

**位置：** `packages/ccode/src/agent/prompt/*.txt`

### 3. SKILL — 可复用能力

以 `SKILL.md` 文件存储的跨项目、跨 agent 知识。Skills 可以动态加载到任何 agent 的上下文中。

**位置：** `~/.codecoder/skills/*/SKILL.md`

### 4. TOOL — 执行工具

Agent 与环境交互的接口：文件操作、命令执行、网络请求等。

**内置工具：** Bash、Read、Edit、Write、Grep、Glob、Task、WebFetch、WebSearch 等

### 5. CHANNEL — 消息渠道

将外部平台（Telegram、Discord、Slack 等）连接到 CodeCoder API 的 Rust 微服务。

**位置：** `services/zero-channels/`

### 6. MEMORY — 记忆系统

透明的双层 Markdown 架构：
- **流层：** 每日笔记（`memory/daily/{YYYY-MM-DD}.md`）
- **沉积层：** 长期知识（`memory/MEMORY.md`）

### 7. WORKFLOW — 自动化引擎

用于定时任务、webhook 和 Git 事件的事件驱动自动化。

**位置：** `services/zero-workflow/`

### 8. HAND — 自主代理

通过 `HAND.md` 文件声明式定义的持久化、有状态 AI 代理，支持定时执行和 CLOSE 决策框架集成。

```
HAND = WORKFLOW（调度）+ AGENT（执行）+ MEMORY（状态）
```

---

## Agent 列表

CodeCoder 包含 31 个专用 Agent：

### 主模式 (4)

| Agent | 描述 |
|-------|-------------|
| `build` | 默认开发模式，具备完整能力 |
| `plan` | 规划模式，用于结构化实现设计 |
| `writer` | 长文写作（20k+ 字） |
| `autonomous` | 使用 CLOSE 决策框架的自主执行 |

### 工程质量 (7)

| Agent | 描述 |
|-------|-------------|
| `code-reviewer` | 综合代码质量审查 |
| `security-reviewer` | 安全漏洞分析 |
| `tdd-guide` | 测试驱动开发强制执行 |
| `architect` | 系统架构设计 |
| `explore` | 快速代码库探索和模式搜索 |
| `general` | 多步骤任务执行和研究 |
| `verifier` | 构建、类型和测试验证 |

### 逆向工程 (2)

| Agent | 描述 |
|-------|-------------|
| `code-reverse` | 像素级网站重建规划 |
| `jar-code-reverse` | Java JAR 反编译和重构 |

### 内容创作 (5)

| Agent | 描述 |
|-------|-------------|
| `expander` | 系统化内容扩展框架 |
| `expander-fiction` | 小说专用世界观和叙事 |
| `expander-nonfiction` | 非虚构论证和证据 |
| `proofreader` | 语法、风格和一致性检查 |
| `writer` | 长文内容写作 |

### 祝融说系列 (8)

| Agent | 描述 |
|-------|-------------|
| `observer` | 观察者理论分析 |
| `decision` | CLOSE 框架决策顾问 |
| `macro` | 宏观经济数据解读 |
| `trader` | 超短线交易模式识别 |
| `picker` | 使用"七宗罪"方法的选品 |
| `miniproduct` | 独立开发者 0 到 1 产品指导 |
| `ai-engineer` | AI/ML 工程导师 |
| `value-analyst` | 价值投资分析 |

### 产品与可行性 (2)

| Agent | 描述 |
|-------|-------------|
| `prd-generator` | 产品需求文档生成 |
| `feasibility-assess` | 技术可行性分析 |

### 系统 (4)

| Agent | 描述 |
|-------|-------------|
| `synton-assistant` | SYNTON-DB 记忆数据库助手 |
| `compaction` | 上下文压缩（隐藏） |
| `title` | 自动会话标题生成（隐藏） |
| `summary` | 会话摘要生成（隐藏） |

---

## AI 提供商

CodeCoder 原生支持 20+ AI 提供商：

| 提供商 | 认证方式 |
|----------|--------------|
| Anthropic Claude | API Key, Claude Max (OAuth) |
| OpenAI | API Key, ChatGPT Plus/Pro (OAuth) |
| Google Gemini | API Key |
| Google Vertex AI | Service Account |
| Amazon Bedrock | IAM, Profile, Web Identity |
| Azure OpenAI | API Key |
| GitHub Copilot | OAuth |
| xAI | API Key |
| Mistral AI | API Key |
| Groq | API Key |
| DeepInfra | API Key |
| Cerebras | API Key |
| Cohere | API Key |
| Together AI | API Key |
| Perplexity | API Key |
| OpenRouter | API Key |
| Vercel AI | API Key |
| GitLab Duo | OAuth |

---

## Rust 微服务

CodeCoder 包含用于生产部署的高性能 Rust 服务：

### Zero Daemon（组合服务）

`zero-daemon` 在单个进程中运行所有服务——适合开发和单机部署。

```bash
# 构建 Rust 服务
./ops.sh build rust

# 启动守护进程
./ops.sh start zero-daemon
```

### 独立服务

用于分布式部署：

| 服务 | 用途 | 端口 |
|---------|---------|------|
| `zero-gateway` | 认证、路由、限流 | 4430 |
| `zero-channels` | IM 集成 | 4431 |
| `zero-workflow` | 自动化引擎 | 4432 |
| `zero-browser` | 浏览器自动化 | 4433 |

```bash
# 启动独立服务
./ops.sh start zero-gateway
./ops.sh start zero-channels
./ops.sh start zero-workflow

# 查看状态
./ops.sh status

# 查看日志
./ops.sh logs zero-workflow
```

---

## 配置

配置文件按优先级加载（后者覆盖前者）：

1. Well-Known 远程配置
2. 全局：`~/.config/codecoder/codecoder.json`
3. `CCODE_CONFIG` 环境变量
4. 项目：`./codecoder.json`
5. `.codecoder/` 目录
6. `CCODE_CONFIG_CONTENT` 环境变量

### 配置示例

```json
{
  "$schema": "https://codecoder.ai/schema/config.json",
  "model": "anthropic/claude-sonnet-4-5",
  "small_model": "anthropic/claude-haiku-4-5",
  "default_agent": "build",

  "provider": {
    "anthropic": {
      "options": { "apiKey": "{env:ANTHROPIC_API_KEY}" }
    }
  },

  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "@anthropic/mcp-server-filesystem", "/home/user"]
    }
  },

  "permission": {
    "*": "allow",
    "bash": {
      "rm -rf *": "deny"
    }
  }
}
```

---

## 记忆系统

CodeCoder 使用透明的、Git 友好的双层记忆架构：

### 第一层：每日笔记（流）

- **路径：** `./memory/daily/{YYYY-MM-DD}.md`
- **类型：** 仅追加日志
- **用途：** 记录每日交互、决策和任务

### 第二层：长期记忆（沉积）

- **路径：** `./memory/MEMORY.md`
- **类型：** 经整理的结构化知识
- **分类：** 用户偏好、项目上下文、关键决策

### 操作规则

| 操作 | 时机 | 行为 |
|-----------|------|----------|
| **读取** | 会话初始化 | 加载 MEMORY.md + 当前/之前的每日笔记 |
| **立即写入** | 重要交互后 | 追加到每日笔记（不可变） |
| **整合** | 检测到重要信息 | 更新 MEMORY.md |

所有记忆文件都是标准 Markdown——需要时可直接编辑。

---

## 开发命令

```bash
# 安装依赖
bun install

# 运行 TUI 界面
bun dev

# 在指定目录运行
bun dev /path/to/project

# 启动 API 服务器
bun dev serve --port 4400

# 类型检查
bun turbo typecheck

# 运行测试（从包目录）
cd packages/ccode && bun test

# 构建独立可执行文件
bun run --cwd packages/ccode build

# API 更改后重新生成 SDK
./script/generate.ts

# 服务操作
./ops.sh start          # 启动核心服务
./ops.sh start all      # 启动所有服务
./ops.sh stop           # 停止所有服务
./ops.sh status         # 查看服务状态
./ops.sh build rust     # 构建 Rust 服务
./ops.sh logs api       # 查看 API 日志
```

---

## 文档

- [架构概览](docs/architecture/README.md)
- [核心概念](docs/architecture/CORE_CONCEPTS.md) — AGENT、PROMPT、SKILL、TOOL、CHANNEL、MEMORY、WORKFLOW、HAND 详解
- [设计哲学](docs/architecture/DESIGN_PHILOSOPHY.md) — 祝融说哲学和 CLOSE 框架
- [CLAUDE.md](CLAUDE.md) — Claude Code 的项目特定说明

---

## 贡献

欢迎贡献！请：

- PR 必须引用现有 issue
- PR 标题遵循约定式提交（`feat:`、`fix:`、`docs:` 等）
- UI 更改需要提供截图/视频
- 保持 PR 小而专注

---

## 许可证

MIT License — 详见 [LICENSE](LICENSE)

---

<p align="center">
  <i>秉持观察者理念构建：每次交互都是将可能性坍缩为现实。</i>
</p>
