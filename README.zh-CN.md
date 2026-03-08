# CodeCoder

> 个人智囊系统 (Personal Brain Trust System) - 融合工程能力与决策智慧的 AI 驱动开发工具

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.md) | [中文](README.zh-CN.md)

---

## CodeCoder 是什么？

CodeCoder 是一个融合**工程能力**与**决策智慧**的 AI 顾问平台。它不仅仅是一个编程助手，更是一个旨在同时增强技术执行和战略思维的综合系统。

### 三层智慧架构

| 层级 | 能力 |
|------|------|
| **工程层** | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| **领域层** | 宏观经济、交易分析、选品策略、极小产品、AI 工程 |
| **思维层** | 祝融说哲学体系、CLOSE 决策框架、观察者理论 |

---

## 核心特性

- **31 个专业 Agent** - 三模式设计 (@build, @writer, @decision)，覆盖工程、内容创作、决策咨询
- **30+ AI 提供商** - Claude、OpenAI、Google、Ollama、Groq、Mistral、Azure、Bedrock 等
- **MCP 协议** - 完整的模型上下文协议支持，支持本地和远程服务器及 OAuth 认证
- **30+ LSP 集成** - TypeScript、Python、Go、Rust、Java 等语言服务器
- **多模式界面** - TUI（终端）、CLI、Web、无头 API
- **双层记忆系统** - 透明的 Markdown 记忆系统（每日笔记 + 长期记忆）
- **安全沙箱** - Process、Docker、WASM 执行后端
- **Hands 自主代理** - 6 级自治系统，集成 CLOSE 决策框架

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              用户接入层                                  │
│     TUI (:4400)    │    Web (:4401)    │    CLI    │  Telegram/Discord │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                    packages/ccode (TypeScript/Bun)                      │
│      Agent 引擎 (31 个 Agent)  │  AI 提供商  │  记忆系统                 │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                      services/zero-* (Rust)                             │
│   zero-cli  │  zero-core  │  zero-hub  │  zero-trading  │  zero-common  │
└─────────────────────────────────────────────────────────────────────────┘
```

**核心原则：** TypeScript 管智能（agent、推理）；Rust 管安全边界（工具、沙箱、协议）。

---

## 快速开始

### 前置要求

- [Bun](https://bun.sh) 1.3+（必需）
- [Rust](https://rustup.rs) 1.75+（可选，用于 Rust 服务）

### 安装

```bash
# 克隆仓库
git clone https://github.com/iannil/code-coder.git
cd code-coder

# 安装依赖
bun install

# 启动 TUI（交互式终端界面）
bun dev

# 或启动无头 API 服务器
bun dev serve --port 4400
```

### 首次运行

首次运行时，至少需要配置一个 AI 提供商：

```bash
# 使用环境变量
export ANTHROPIC_API_KEY="your-api-key"

# 或通过配置文件配置 (~/.codecoder/config.json)
```

---

## Agent 系统

CodeCoder 采用 3 模式 Agent 系统，简化用户交互：

| 模式 | 主 Agent | 能力 |
|------|----------|------|
| **@build**（默认） | build | code-reviewer, security-reviewer, tdd-guide, architect, explore, general, code-reverse, jar-code-reverse, verifier, prd-generator, feasibility-assess |
| **@writer** | writer | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| **@decision** | decision | macro, trader, value-analyst, picker, miniproduct, ai-engineer, synton-assistant |

### 使用示例

```bash
# 默认 @build 模式
bun dev

# 写作模式，用于内容创作
bun dev -m writer

# 决策模式，用于战略分析
bun dev -m decision

# 访问特定能力
bun dev @build:security-reviewer
bun dev @decision:macro
```

### Agent 分类（共 31 个）

| 分类 | Agent | 用途 |
|------|-------|------|
| **主模式 (4)** | build, plan, writer, autonomous | 主要开发/创作模式 |
| **逆向工程 (2)** | code-reverse, jar-code-reverse | 代码分析与重建 |
| **工程质量 (6)** | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | 代码质量保障 |
| **内容创作 (5)** | expander, expander-fiction, expander-nonfiction, proofreader, verifier | 长文写作 |
| **祝融说系列 (8)** | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | 决策与领域咨询 |
| **产品 (2)** | prd-generator, feasibility-assess | 需求与可行性 |
| **辅助 (1)** | synton-assistant | SYNTON-DB 助手 |
| **系统隐藏 (3)** | compaction, title, summary | 内部使用 |

---

## 哲学：祝融说

CodeCoder 内置了独特的基于"祝融说"的决策与认知系统。

### 核心理念

- **可能性基底** - 宇宙的终极实在是包含一切潜能的无限场域
- **观察即收敛** - 观察是创造性行为，导致可能性"坍缩"为确定性
- **可用余量** - 尚未被固化的潜能空间，是自由意志和创造力的来源

### CLOSE 决策框架

每个决策都通过五个维度进行评估：

| 维度 | 含义 | 说明 |
|------|------|------|
| **C** (Capacity) | 能力 | 当前能力和资源 |
| **L** (Leverage) | 杠杆 | 放大潜力 |
| **O** (Opportunity) | 机会 | 时机和背景 |
| **S** (Sustainability) | 可持续性 | 长期可行性 |
| **E** (Exit) | 退出 | 可逆性和选项 |

**核心洞察：** 可持续决策 > 最优决策。保持"再来一次"的能力比追求"最优解"更重要。

### 自治级别（6 级）

| 级别 | 描述 |
|------|------|
| **Lunatic** | 完全自主，无需审批 |
| **Insane** | 高度自主，仅关键操作审批 |
| **Crazy** | 自主运行，大部分操作自动批准 |
| **Wild** | 中等自主，部分操作需审批 |
| **Bold** | 保守自主，多数操作需审批 |
| **Timid** | 最谨慎，几乎所有操作需审批 |

---

## 配置

配置文件位于 `~/.codecoder/`：

```
~/.codecoder/
├── config.json       # 核心配置
├── secrets.json      # 凭证（gitignored，600 权限）
├── providers.json    # LLM 提供商配置
├── trading.json      # 交易模块配置
└── channels.json     # IM 渠道配置
```

### 环境变量

```bash
ANTHROPIC_API_KEY     # Anthropic Claude API 密钥
OPENAI_API_KEY        # OpenAI API 密钥
CCODE_CONFIG          # 自定义配置文件路径
CCODE_CONFIG_CONTENT  # 内联配置（JSON）
```

---

## 开发命令

```bash
# 安装依赖
bun install

# 运行 TUI（开发）
bun dev

# 运行无头服务器
bun dev serve --port 4400

# 类型检查（所有包）
bun turbo typecheck

# 运行测试（从特定包）
cd packages/ccode && bun test

# 构建独立可执行文件
bun run --cwd packages/ccode build

# API 更改后重新生成 SDK
./script/generate.ts
```

### Rust 服务（可选）

```bash
# 构建 Rust 服务
./ops.sh build rust

# 启动所有服务
./ops.sh start all

# 检查状态
./ops.sh status

# 查看日志
./ops.sh logs zero-daemon
```

---

## 项目结构

```
codecoder/
├── packages/
│   ├── ccode/              # 核心 CLI 和 Agent 引擎（TypeScript）
│   │   ├── src/
│   │   │   ├── agent/      # 31 个 agent 定义
│   │   │   ├── tool/       # 工具实现
│   │   │   ├── provider/   # AI 提供商集成
│   │   │   ├── mcp/        # MCP 协议
│   │   │   └── cli/cmd/tui/# 终端 UI（Solid.js）
│   │   └── test/
│   ├── web/                # Web 前端（React）
│   └── util/               # 共享工具
├── services/               # Rust 服务（5 个 crate）
│   ├── zero-cli/           # CLI + Daemon（入口点）
│   ├── zero-core/          # 核心工具，NAPI 绑定
│   ├── zero-hub/           # 服务中枢（gateway/channels/workflow）
│   ├── zero-trading/       # 交易系统
│   └── zero-common/        # 共享库
├── memory/                 # 双层记忆系统
│   ├── daily/              # 每日笔记（仅追加）
│   └── MEMORY.md           # 长期记忆（经整理）
└── docs/                   # 文档
```

---

## 文档

- **架构：** [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md)
- **功能：** [`docs/FEATURES.md`](docs/FEATURES.md)
- **设计哲学：** [`docs/architecture/DESIGN_PHILOSOPHY.md`](docs/architecture/DESIGN_PHILOSOPHY.md)
- **项目概览：** [`docs/PROJECT-OVERVIEW.md`](docs/PROJECT-OVERVIEW.md)

---

## 贡献

欢迎贡献！请遵循以下准则：

1. **Fork** 仓库并创建功能分支
2. **引用** 现有 issue 或先创建一个
3. **遵循** 约定式提交（`feat:`、`fix:`、`docs:` 等）
4. **包含** 新功能的测试
5. **保持** PR 小而专注

---

## 许可证

MIT License

Copyright (c) 2024-2026 CodeCoder Contributors

详见 [LICENSE](LICENSE)。

---

<p align="center">
  <i>秉持观察者理念构建：每次交互都是将可能性坍缩为现实。</i>
</p>
