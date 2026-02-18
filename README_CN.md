# CodeCoder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0-blue.svg)](https://www.typescriptlang.org/)
[![Bun](https://img.shields.io/badge/Runtime-Bun-black.svg)](https://bun.sh/)
[![Rust](https://img.shields.io/badge/Gateway-Rust-orange.svg)](https://www.rust-lang.org/)

[English](./README.md)

**融合工程能力与决策智慧的 AI 个人工作台。**

CodeCoder 不只是一个编程助手。它是为所有正在被 AI 改变工作方式的人设计的综合平台——开发者、分析师、写作者、决策者、独立创作者。基于[祝融说](https://zhurongshuo.com)哲学框架构建，将结构化思维与实用工具融为一体。

## 为什么选择 CodeCoder

在 AI 普及的时代，挑战不在于如何访问 AI，而在于**如何与 AI 一起思考**。

大多数 AI 工具专注于回答问题。CodeCoder 专注于帮助你提出更好的问题、做出更好的决策：

- **多维度辅助** — 不止于代码，还有决策、分析和内容创作
- **哲学根基** — CLOSE 框架支撑可持续决策
- **提供商灵活性** — 通过统一接口连接 30+ AI 提供商
- **专业化能力** — 25+ 面向不同领域的专业 Agent

## 核心功能

| 功能 | 描述 |
|------|------|
| **25+ 专业 Agent** | 工程、领域分析、决策支持、内容创作 |
| **30+ AI 提供商** | Anthropic、OpenAI、Google、AWS Bedrock、Azure、本地模型（通过 MCP） |
| **20+ 内置工具** | 文件操作、代码搜索、网络获取、任务管理 |
| **LSP 支持** | 30+ 语言服务器，自动安装 |
| **MCP 协议** | 本地/远程服务器、OAuth 2.0、动态工具发现 |
| **ZeroBot 网关** | 轻量级 Rust 网关（~3.4MB），多通道访问 |
| **记忆系统** | 透明、Git 友好的 Markdown 记忆架构 |

## 哲学框架

### 祝融说

CodeCoder 构建在**祝融说**之上，这是一套独特的哲学框架，它重新定义了我们应对不确定性和做出决策的方式。

| 核心概念 | 描述 |
|----------|------|
| **可能性基底** | 终极实在不是静态的「实体」，而是包含一切潜能的无限场域。一切确定的现实都是从这个「可能性海洋」中涌现的。 |
| **观察即收敛** | 「观察」是一种创造性行为，而非被动接收。观察导致可能性「坍缩」为确定性。宏观世界的稳定性源于多层级观察者的「共同投票」。 |
| **可用余量** | 尚未被固化的潜能空间。自由意志、创造力和系统韧性的来源。可持续决策保留余量；最优决策往往消耗余量。 |

### CLOSE 五维评估框架

一套用于做出可持续选择的五维评估系统：

| 维度 | 核心问题 | 关注点 |
|------|----------|--------|
| **C**onvergence 收敛 | 这个选择会收敛多少可能性？ | 保留未来选项 |
| **L**everage 杠杆 | 是否存在非对称收益？ | 风险收益不对称性 |
| **O**ptionality 选择权 | 这个决定可以反悔吗？成本多大？ | 可逆性 |
| **S**urplus 余量 | 消耗多少缓冲资源？ | 资源保护 |
| **E**volution 演化 | 能带来成长和学习机会吗？ | 学习潜力 |

> 「可持续决策 > 最优决策。能够继续玩下去，比赢一次更重要。」

## 架构

### 三层智慧架构

```
┌─────────────────────────────────────────────────────────────┐
│                        思维智囊层                             │
│   observer · decision                                        │
├─────────────────────────────────────────────────────────────┤
│                        领域智囊层                             │
│   macro · trader · picker · miniproduct · ai-engineer        │
├─────────────────────────────────────────────────────────────┤
│                        工程智囊层                             │
│   code-reviewer · security-reviewer · tdd-guide · architect  │
│   code-reverse · jar-code-reverse · explore · general        │
├─────────────────────────────────────────────────────────────┤
│                        内容智囊层                             │
│   writer · proofreader · expander                            │
└─────────────────────────────────────────────────────────────┘
```

### 技术栈

- **运行时**：Bun 1.3+ + TypeScript ESM
- **TUI 框架**：OpenTUI + SolidJS
- **AI SDK**：Vercel AI SDK 多提供商支持
- **网关**：ZeroBot（Rust，~3.4MB 二进制文件）
- **验证**：Zod schemas

## Agent 概览

### 主模式

| Agent | 描述 |
|-------|------|
| `build` | 主开发模式，具备完整能力 |
| `plan` | 只读探索和规划模式 |
| `autonomous` | 自主任务完成，配备安全护栏 |

### 工程类 Agent

| Agent | 用途 |
|-------|------|
| `general` | 多步骤任务、并行工作执行 |
| `code-reviewer` | 代码质量、命名、可维护性 |
| `security-reviewer` | OWASP Top 10、注入风险、认证问题 |
| `tdd-guide` | 红-绿-重构循环、覆盖率 |
| `architect` | 系统设计、接口定义、设计模式 |
| `verifier` | 构建、类型、lint 检查、测试套件 |
| `explore` | 快速代码库探索 |

### 逆向工程

| Agent | 用途 |
|-------|------|
| `code-reverse` | 网站像素级复刻规划 |
| `jar-code-reverse` | Java JAR 分析与源码重建 |

### 领域类 Agent（祝融说系列）

| Agent | 用途 |
|-------|------|
| `macro` | 宏观经济分析——GDP、通胀、货币政策 |
| `trader` | 超短线交易指导（仅供教育参考） |
| `picker` | 选品策略——七宗罪选品法 |
| `miniproduct` | 极小产品教练——MVP、变现策略 |
| `ai-engineer` | AI 工程师导师——Python、LLM 应用、RAG、MLOps |

### 思维类 Agent（祝融说系列）

| Agent | 用途 |
|-------|------|
| `observer` | 可能性空间分析、认知框架 |
| `decision` | CLOSE 框架评估、可持续选择 |

### 内容类 Agent

| Agent | 用途 |
|-------|------|
| `writer` | 长文写作（2 万字以上）、章节规划 |
| `proofreader` | 语法、风格、PROOF 框架验证 |
| `expander` | 将想法转化为完整书籍 |

## 快速开始

### 环境要求

- [Bun](https://bun.sh/) 1.3 或更高版本
- macOS、Linux 或 Windows（推荐 WSL）

### 安装

```bash
git clone https://github.com/iannil/code-coder.git
cd code-coder
bun install
```

### 运行

```bash
# 启动交互式 TUI
bun dev

# 或指定工作目录
bun dev /path/to/project

# 启动无头 API 服务器
bun dev serve --port 4096
```

### 构建独立可执行文件

```bash
bun run --cwd packages/ccode build
```

## 配置

### 配置文件位置（优先级顺序）

1. 全局：`~/.config/codecoder/codecoder.json`
2. 项目：`./codecoder.json` 或 `./.codecoder/codecoder.json`
3. 环境变量：`CCODE_CONFIG` 或 `CCODE_CONFIG_CONTENT`

### 配置示例

```json
{
  "$schema": "https://raw.githubusercontent.com/iannil/code-coder/main/packages/ccode/schema.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "default_agent": "build",
  "provider": {
    "anthropic": {
      "options": { "apiKey": "{env:ANTHROPIC_API_KEY}" }
    }
  },
  "permission": {
    "bash": {
      "*": "ask",
      "git *": "allow",
      "npm *": "allow"
    }
  }
}
```

## 使用示例

### 代码审查

```bash
ccode run --agent code-reviewer "审查认证模块的代码"
```

### 决策分析（CLOSE 框架）

```bash
ccode run --agent decision "我应该接受这份工作邀请吗？"
```

### 经济数据分析

```bash
ccode run --agent macro "解读最新的 PMI 数据"
```

### 产品选品

```bash
ccode run --agent picker "分析 AI 写作工具的市场机会"
```

## 项目结构

```
codecoder/
├── packages/
│   ├── ccode/           # 核心 CLI 和业务逻辑
│   │   ├── src/
│   │   │   ├── agent/   # Agent 定义和提示词
│   │   │   ├── cli/     # CLI 命令和 TUI
│   │   │   ├── provider/# AI 提供商集成
│   │   │   ├── mcp/     # MCP 协议支持
│   │   │   ├── lsp/     # LSP 集成
│   │   │   └── tool/    # 内置工具
│   │   └── test/        # 测试套件
│   └── util/            # 共享工具库
├── services/
│   └── zero-bot/        # Rust 消息网关
├── script/              # 构建和生成脚本
├── docs/                # 文档
└── memory/              # 记忆存储
```

## 开发

```bash
# 安装依赖
bun install

# 开发模式运行
bun dev

# 类型检查
bun turbo typecheck

# 运行测试（从包目录）
cd packages/ccode && bun test

# 构建可执行文件
bun run --cwd packages/ccode build

# 重新生成 SDK
./script/generate.ts
```

### 端口配置

| 服务 | 端口 |
|------|------|
| CodeCoder API Server | 4400 |
| Web Frontend (Vite) | 4401 |
| ZeroBot Daemon | 4402 |
| Faster Whisper Server | 4403 |

## 贡献

欢迎贡献！详情请参阅[贡献指南](./docs/CONTRIB.md)。

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feature/amazing-feature`）
3. 按照[约定式提交](https://www.conventionalcommits.org/zh-hans/)规范提交更改
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 创建 Pull Request

## 许可证

本项目基于 MIT 许可证开源——详情请参阅 [LICENSE](LICENSE) 文件。

## 致谢

- 使用 [Bun](https://bun.sh) 构建——快速的一体化 JavaScript 运行时
- 由 [Vercel AI SDK](https://sdk.vercel.ai) 驱动——多提供商 AI 集成
- UI 由 [OpenTUI](https://github.com/sst/opentui) 提供支持——终端 UI 框架
