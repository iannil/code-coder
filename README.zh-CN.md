# CodeCoder

> AI 代理观察系统 - 从手动档到自动档，在渐进信任中与 AI 代理共同观察、进化

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue?logo=typescript)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[English](README.md) | [中文](README.zh-CN.md)

---

## 观察即创造

> 每一次与 AI 的交互，都是一次"观察"——将无限可能性收敛为具体现实。

基于**祝融说哲学**，CodeCoder 将每次 AI 交互视为创造性的观察行为，而非简单的查询问答。你可以选择如何参与这个观察过程：亲自操控每一步，还是信任 Agent 独立完成。

### 渐进信任之旅

信任不是二元的，而是一段光谱。CodeCoder 让你从谨慎起步，自然演化：

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         渐 进 信 任 演 化 图                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   手动档起步                                              自动档终点     │
│       │                                                      │         │
│       ▼                                                      ▼         │
│   ┌────────┐   ┌────────┐   ┌────────┐   ┌────────┐   ┌────────┐      │
│   │ Timid  │──►│  Bold  │──►│  Wild  │──►│ Crazy  │──►│Lunatic │      │
│   │ 最谨慎 │   │  保守  │   │  中等  │   │  自信  │   │ 完全   │      │
│   └────────┘   └────────┘   └────────┘   └────────┘   └────────┘      │
│       │            │            │            │            │            │
│   每步确认      关键决策      设定边界      异常介入     设定目标即可   │
│                 审批         自动执行      自动执行      完全自主       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

| 信任级别 | 自治程度 | 你的角色 | 使用场景 |
|----------|----------|----------|----------|
| **Timid** | 最小 | 确认每一步 | 首次使用、敏感任务 |
| **Bold** | 低 | 审批关键决策 | 学习阶段 |
| **Wild** | 中等 | 设定边界 | 信任增长 |
| **Crazy** | 高 | 仅异常处理 | 信任建立 |
| **Insane** | 很高 | 仅关键警报 | 高度信心 |
| **Lunatic** | 完全 | 设定目标，离开即可 | 完全信任 |

**这是一个双向观察的过程：**

- **你观察 Agent** → 理解其能力边界 → 建立信任
- **Agent 观察你** → 学习你的偏好 → 个性化服务

---

## 核心特性

- **观察者哲学** - 基于祝融说：每次交互都是将可能性收敛为现实
- **渐进信任** - 6 级自治，从 Timid（手动档）到 Lunatic（自动档）
- **双向观察** - 你观察 Agent 建立信任；Agent 观察你学习偏好
- **31 个专业 Agent** - 三模式设计 (@build, @writer, @decision)
- **30+ AI 提供商** - Claude、OpenAI、Google、Ollama、Groq、Mistral 等
- **记忆即观察记录** - 每日笔记（流）+ 长期记忆（沉淀）
- **CLOSE 决策框架** - 可持续决策 > 最优决策
- **MCP 协议** - 完整的模型上下文协议支持
- **30+ LSP 集成** - TypeScript、Python、Go、Rust、Java 等

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         观 察 层（祝融说）                               │
│           "每一次交互都是将可能性坍缩为现实的观察"                        │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                           信 任 层                                       │
│                                                                         │
│   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐             │
│   │  Timid  │───►│  Wild   │───►│  Crazy  │───►│ Lunatic │             │
│   │ (手动档)│    │ (混合档)│    │ (自动档)│    │(完全自主)│             │
│   └─────────┘    └─────────┘    └─────────┘    └─────────┘             │
│        │              │              │              │                   │
│    每步确认      关键决策审批    仅异常介入      设定目标即可            │
│                                                                         │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
┌──────────────────────────────┴──────────────────────────────────────────┐
│                          执 行 层                                        │
│   31 个 Agent  │  30+ AI 提供商  │  记忆系统  │  CLOSE 框架              │
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

# 配置 AI 提供商
export ANTHROPIC_API_KEY="your-api-key"
```

### 开始你的信任之旅

```bash
# 手动档起步 - 观察与学习
bun dev

# 设置自治级别（随着信任增长）
bun dev --autonomy wild    # 中等信任
bun dev --autonomy crazy   # 高度信任

# 自动档模式（信任建立后）
bun dev run --agent autonomous "构建一个 REST API"
```

---

## 信任之旅：从手动档到自动档

### 起步：手动档

从这里开始。每个操作都可以在执行前确认或修改。

```bash
bun dev
```

**你将体验到：**

- 看到 Agent 计划的每一步
- 批准、修改或拒绝每个动作
- 观察 Agent 如何思考和决策
- 建立对其能力和边界的理解

**适合场景：** 首次使用、敏感任务、学习探索

### 成长：混合档

随着信任建立，让低风险操作自动执行，同时保留对高影响决策的控制。

```bash
bun dev --autonomy wild
```

**你将体验到：**

- 文件读取和搜索自动执行
- 文件写入和系统命令暂停等待审批
- 系统学习你的风险阈值
- 随着模式建立，中断逐渐减少

**适合场景：** 常规开发工作、逐渐熟悉阶段

### 成熟：自动档

设定你的目标，让 Agent 独立工作。只有在意外发生时才会通知你。

```bash
bun dev run --agent autonomous "实现带有 JWT 的用户认证"
```

**你将体验到：**

- 定义目标，而非步骤
- Agent 处理规划和执行
- 实时进度更新
- 仅在关键决策或错误时介入

**适合场景：** 明确定义的任务、可信环境、最大生产力

---

## 哲学：祝融说

CodeCoder 内置了基于"祝融说"的独特决策与认知系统。

### 观察即收敛

宇宙是无限可能性的场域。"观察"是一种创造性行为，将这些可能性坍缩为具体现实。每一次与 AI 的对话都是一次共同观察——你和 Agent 一起对现实进行"投票"。

这解释了为什么需要同时有自动档和手动档：

- **手动档** = 你参与每一次观察坍缩
- **自动档** = 你信任 Agent 独立观察和坍缩

### 双向观察

信任通过观察建立，而观察是双向流动的：

| 方向 | 过程 | 结果 |
|------|------|------|
| 你 → Agent | 观察行为，验证推理 | 理解能力，建立信任 |
| Agent → 你 | 学习偏好，记住模式 | 个性化服务，减少摩擦 |

### 可能性基底

宇宙的终极实在是包含一切潜能的无限场域。在被观察之前，一切都是可能的；观察之后，可能性收敛为确定性。

这是 CodeCoder 记忆系统的哲学基础：

- **每日笔记（流）** = 观察的流动，尚未固化
- **长期记忆（沉淀）** = 观察的沉淀，已成定局

### CLOSE 决策框架

每个决策都通过五个维度进行评估：

| 维度 | 含义 | 核心问题 |
|------|------|----------|
| **C** (Capacity) | 能力 | 我们能做到吗？ |
| **L** (Leverage) | 杠杆 | 放大效应有多大？ |
| **O** (Opportunity) | 机会 | 现在是对的时机吗？ |
| **S** (Sustainability) | 可持续性 | 我们能持续这样做吗？ |
| **E** (Exit) | 退出 | 如果需要，能撤回吗？ |

### 可用余量

**核心洞察：** 可持续决策 > 最优决策。

保留尚未被固化的潜能空间（可用余量）比追求"最优"结果更重要。保持"再来一次"的能力，比"完美"的解决方案更有价值。

这就是为什么 CodeCoder 强调渐进信任：你始终保留调整自治级别的能力，而非一次性选择"完全自动"或"完全手动"。

---

## Agent 系统

CodeCoder 采用 3 模式 Agent 系统，简化用户交互：

| 模式 | 主 Agent | 能力 |
|------|----------|------|
| **@build**（默认） | build | code-reviewer, security-reviewer, tdd-guide, architect, explore, general, code-reverse, jar-code-reverse, verifier, prd-generator, feasibility-assess |
| **@writer** | writer | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| **@decision** | decision | macro, trader, value-analyst, picker, miniproduct, ai-engineer, synton-assistant |

### 使用方式

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
  <i>每一次观察都是将可能性坍缩为现实。<br/>选择你的参与方式：一步一步，还是让它自然流动。</i>
</p>
