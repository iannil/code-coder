# CodeCoder

[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue.svg)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Bun](https://img.shields.io/badge/Bun-1.3+-pink.svg)](https://bun.sh/)

<!-- Logo 占位符：在此添加您的 Logo -->

---

## 超越代码补全

CodeCoder 不仅仅是另一个 AI 编程助手。它是一个融合工程能力与决策智慧的**工作台系统**。

其核心是**祝融说**哲学中的**观察者理论**：每一次观察都将无限可能性收敛为确定性现实。当你请求 CodeCoder 审查代码、设计架构或分析市场趋势时，你不只是在获取答案——你正在参与一个从可能性场域中塑造结果的创造性行为。

这一哲学基础体现在 **CLOSE 决策框架**中：

- **C** (Clear) — 清晰评估可逆性
- **L** (Leverage) — 保持选择权杠杆
- **O** (Option) — 扩展选项空间
- **S** (Sustainability) — 可持续性优于最优性
- **E** (Exit) — 规划退出策略

核心洞见：**可持续的决策胜过最优的决策**。CodeCoder 帮助你保持"可用余量"——尚未被固化的潜能空间，它是适应力、创造力以及"再来一次"能力的源泉。

---

## CodeCoder 是什么？

超越代码的三层智慧架构：

| 层级 | 聚焦 | 能力 |
|------|------|------|
| **工程层** | 代码与系统 | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| **领域层** | 专业知识 | 宏观经济、交易分析、选品策略、极小产品、AI 工程 |
| **思维层** | 决策框架 | 祝融说哲学、CLOSE 框架、观察者理论 |

**核心特性：**

- **20+ AI 提供商** — Claude、OpenAI、Google、Amazon Bedrock、Azure、xAI 等
- **24 个专业 Agent** — 各具独特专长和个性
- **MCP 协议** — 本地、远程及 OAuth 认证的模型上下文协议服务器
- **30+ LSP 集成** — TypeScript、Rust、Go、Python、Java 等
- **多种模式** — CLI、TUI（终端界面）和无头 API 服务器
- **Markdown 记忆系统** — 人类可读、Git 友好的知识持久化

---

## 快速开始

### 环境要求

- **Bun** 1.3+ ([安装](https://bun.sh/docs/installation))
- **Node.js** 22+ (部分依赖需要)
- **Rust** 1.75+ (可选，用于 Rust 服务)

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

# 以 CLI 模式运行并发送消息
bun dev run "Review the authentication code in src/auth"

# 启动无头 API 服务器
bun dev serve --port 4400

# 使用指定 Agent
bun dev run --agent code-reviewer "Analyze src/api/server.ts"

# 使用 @ 语法调用 Agent
bun dev run "@decision 用CLOSE框架分析这个职业选择"
```

---

## 架构

```
codecoder/
├── packages/                    # TypeScript 包
│   ├── ccode/                   # 核心 CLI 与业务逻辑
│   │   ├── src/
│   │   │   ├── agent/           # Agent 定义与 Prompt
│   │   │   ├── api/             # HTTP API 服务器 (Hono)
│   │   │   ├── cli/             # CLI 命令与 TUI
│   │   │   ├── config/          # 配置管理
│   │   │   ├── mcp/             # MCP 协议支持
│   │   │   ├── provider/        # AI 提供商适配器
│   │   │   ├── session/         # 会话管理
│   │   │   └── tool/            # 工具定义
│   │   └── test/                # 测试
│   ├── util/                    # 共享工具库
│   └── web/                     # Web UI (React + Vite)
├── services/                    # Rust 服务
│   ├── zero-cli/                # CLI 守护进程（组合服务）
│   ├── zero-gateway/            # 认证、路由、配额
│   ├── zero-channels/           # Telegram、Discord、Slack
│   ├── zero-workflow/           # Webhook、Cron、Git
│   ├── zero-common/             # 共享配置
│   ├── zero-agent/              # Agent 执行（库）
│   ├── zero-memory/             # 内存持久化（库）
│   └── zero-tools/              # 工具定义（库）
├── memory/                      # Markdown 记忆系统
│   ├── MEMORY.md                # 长期记忆
│   └── daily/                   # 每日笔记
├── docs/                        # 文档
└── script/                      # 构建脚本
```

### 端口配置

| 服务 | 端口 | 技术栈 |
|------|------|--------|
| CodeCoder API Server | 4400 | Bun/TypeScript |
| Web Frontend | 4401 | Vite/React |
| Zero CLI Daemon | 4402 | Rust (组合：gateway + channels + scheduler) |
| Whisper STT Server | 4403 | Docker |
| Zero Gateway | 4410 | Rust (独立) |
| Zero Channels | 4411 | Rust (独立) |
| Zero Workflow | 4412 | Rust (独立) |
| MCP Server (HTTP) | 4420 | 协议 (Model Context Protocol) |

---

## Agent 系统

CodeCoder 包含 24 个专业 Agent，分为 6 个类别：

### 主模式 (3)

| Agent | 描述 |
|-------|------|
| `build` | 默认开发模式，具备完整功能 |
| `plan` | 计划模式，用于结构化实现设计 |
| `autonomous` | 自主执行模式，采用 CLOSE 决策框架 |

### 逆向工程 (2)

| Agent | 描述 |
|-------|------|
| `code-reverse` | 像素级网站复刻规划 |
| `jar-code-reverse` | Java JAR 文件反编译与重建 |

### 工程类 (7)

| Agent | 描述 |
|-------|------|
| `general` | 多步骤任务执行和研究 |
| `explore` | 快速代码库探索和模式搜索 |
| `code-reviewer` | 全面的代码质量审查 |
| `security-reviewer` | 安全漏洞分析 |
| `tdd-guide` | 测试驱动开发执行 |
| `architect` | 系统架构设计 |
| `verifier` | 形式化验证和属性测试 |

### 内容创作 (5)

| Agent | 描述 |
|-------|------|
| `writer` | 长文写作（20k+ 字） |
| `proofreader` | 语法、风格和一致性检查 |
| `expander` | 系统化内容扩展框架 |
| `expander-fiction` | 小说专用：世界观构建与叙事 |
| `expander-nonfiction` | 非虚构专用：论证与证据 |

### 祝融说系列 (8)

| Agent | 描述 |
|-------|------|
| `observer` | 观察者理论分析——揭示可能性空间 |
| `decision` | CLOSE 框架决策顾问 |
| `macro` | 宏观经济数据解读（GDP、政策等） |
| `trader` | 超短线交易模式识别 |
| `picker` | "七宗罪"选品法产品选择 |
| `miniproduct` | 独立开发者 0-1 产品教练 |
| `ai-engineer` | AI/ML 工程导师 |
| `synton-assistant` | SYNTON-DB 记忆数据库助手 |

### 系统级 (3, 隐藏)

| Agent | 描述 |
|-------|------|
| `compaction` | 长会话上下文压缩 |
| `title` | 自动会话标题生成 |
| `summary` | 会话摘要生成 |

### 使用示例

```bash
# 代码审查
bun dev run --agent code-reviewer "Review src/api/server.ts"

# 安全分析
bun dev run --agent security-reviewer "Audit the authentication system"

# 决策咨询
bun dev run "@decision 用CLOSE框架分析这个职业选择"

# 宏观分析
bun dev run "@macro 解读本月的PMI数据"

# 架构设计
bun dev run --agent architect "Design a microservices migration plan"
```

---

## AI 提供商

CodeCoder 开箱即用支持 20+ AI 提供商：

| 提供商 | 认证方式 |
|--------|----------|
| Anthropic Claude | API Key, Claude Max (OAuth) |
| OpenAI | API Key, ChatGPT Plus/Pro (OAuth) |
| Google Gemini | API Key |
| Google Vertex AI | 服务账户 |
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

### 配置示例

```json
{
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "sk-ant-..."
      }
    },
    "openai": {
      "options": {
        "apiKey": "sk-proj-..."
      }
    }
  },
  "model": "anthropic/claude-sonnet-4-5"
}
```

---

## Rust 服务

对于生产部署，CodeCoder 提供高性能 Rust 服务：

### Zero Daemon（组合服务）

`zero-daemon` 在单进程中运行所有服务——适合开发和单机部署：

```bash
# 构建 Rust 服务
./ops.sh build rust

# 启动守护进程
./ops.sh start zero-daemon
```

### 独立服务（模块化部署）

对于分布式部署，可独立运行各服务：

| 服务 | 用途 |
|------|------|
| `zero-gateway` | 认证、路由、限流、沙箱 |
| `zero-channels` | Telegram、Discord、Slack、Email 集成 |
| `zero-workflow` | Webhook 处理、定时任务、Git 操作 |

```bash
# 启动独立服务
./ops.sh start zero-gateway
./ops.sh start zero-channels
./ops.sh start zero-workflow

# 查看所有服务状态
./ops.sh status

# 查看日志
./ops.sh logs zero-workflow
```

---

## 配置

配置文件按优先级加载（后者覆盖前者）：

1. Well-Known 远程配置
2. 全局配置：`~/.config/codecoder/codecoder.json`
3. `CCODE_CONFIG` 环境变量
4. 项目配置：`./codecoder.json`
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
    },
    "remote-server": {
      "type": "remote",
      "url": "https://mcp.example.com/sse",
      "oauth": {
        "clientId": "...",
        "scope": "read write"
      }
    }
  },

  "permission": {
    "*": "allow",
    "bash": {
      "rm -rf *": "deny",
      "git *": "allow"
    }
  }
}
```

---

## 记忆系统

CodeCoder 采用透明、Git 友好的双层记忆架构：

### 第一层：每日笔记（流）

- **路径：** `./memory/daily/{YYYY-MM-DD}.md`
- **类型：** 仅追加日志
- **用途：** 记录每日交互、决策和任务

### 第二层：长期记忆（沉积）

- **路径：** `./memory/MEMORY.md`
- **类型：** 经过整理的结构化知识
- **分类：** 用户偏好、项目上下文、关键决策、经验教训

### 操作规则

| 操作 | 时机 | 行为 |
|------|------|------|
| **读取** | 会话初始化 | 加载 MEMORY.md + 当日/前日笔记 |
| **即时写入** | 重要交互后 | 追加到每日笔记（不可变） |
| **整合写入** | 检测到重要信息 | 更新 MEMORY.md（合并/替换过时内容） |

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

# 运行测试（需在包目录内执行）
cd packages/ccode && bun test

# 构建独立可执行文件
bun run --cwd packages/ccode build

# API 更改后重新生成 SDK
./script/generate.ts

# 服务运维
./ops.sh start          # 启动核心服务
./ops.sh start all      # 启动所有服务
./ops.sh stop           # 停止所有服务
./ops.sh status         # 查看服务状态
./ops.sh build rust     # 构建 Rust 服务
./ops.sh logs api       # 查看 API 日志
```

---

## 贡献指南

欢迎贡献！请遵循以下规范：

- PR 必须引用现有 issue
- PR 标题遵循约定式提交规范（`feat:`、`fix:`、`docs:` 等）
- UI 更改需提供截图/视频
- PR 应保持小而专注

---

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

---

<p align="center">
  <i>以观察者理论为指导构建：每一次交互都将可能性收敛为现实。</i>
</p>
