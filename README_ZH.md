# CodeCoder

> 既有自动档、也有手动档的 AI 代理观察系统 —— 像汽车变速箱一样自由切换自主级别

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Bun](https://img.shields.io/badge/Bun-1.3+-black?logo=bun)](https://bun.sh/)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org/)

[English](./README.md)

## 概述

CodeCoder 是一个基于"祝融说"哲学体系构建的 AI 观察系统。与传统的二元模式 AI 编程助手（要么全自动、要么全手动）不同，CodeCoder 引入了**类变速箱的控制系统**，让你精确调节 AI 的自主程度。

核心洞察：**AI 自主性不是二元的——它是一个连续的光谱。**

## 档位系统：你的自主性旋钮

像汽车变速箱一样，CodeCoder 让你控制 AI 的自主程度：

```
┌─────────────────────────────────────────────────────────────────────┐
│                      档位选择器: P  N  D  S  M                       │
│                                                                     │
│              ┌───────────┬───────────┬───────────┐                  │
│              │   观察    │   决策    │   执行    │    三旋钮         │
│              │   0-100   │   0-100   │   0-100   │   (手动模式)      │
│              └───────────┴───────────┴───────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 五档预设

| 档位 | 模式 | 观察 | 决策 | 执行 | 使用场景 |
|------|------|------|------|------|----------|
| **P** | 驻车 | 0% | 0% | 0% | 系统待机，不消耗资源 |
| **N** | 空档 | 50% | 0% | 0% | 纯观察，不干预 |
| **D** | 行车 | 70% | 60% | 40% | 日常平衡模式（默认） |
| **S** | 运动 | 90% | 80% | 70% | 全自动模式 |
| **M** | 手动 | 自定义 | 自定义 | 自定义 | 通过三旋钮精细控制 |

### 三旋钮

在手动 (M) 模式下，你可以独立控制三个维度：

- **观察 (0-100%)**：系统扫描变化的积极程度
  - 0% = 被动等待，100% = 主动扫描
- **决策 (0-100%)**：系统自主决策的程度
  - 0% = 仅建议，100% = 无需询问直接决定
- **执行 (0-100%)**：系统自主执行的程度
  - 0% = 等待确认，100% = 立即执行

### 快速上手档位

```bash
# 以行车模式启动（默认 - 平衡自主）
bun dev

# 以运动模式启动（高自主）
bun dev --gear S

# 以空档模式启动（仅观察）
bun dev --gear N

# 手动模式 + 自定义旋钮值
bun dev --gear M --observe 80 --decide 30 --act 10
```

## 特性

- **类变速箱的自主性控制** - P/N/D/S/M 五档 + 三个精调旋钮
- **31 个 AI Agent** - 按 3 种模式组织（build、writer、decision）
- **观察者网络** - 四大观察者（Code、World、Self、Meta）+ 共识引擎
- **多提供商支持** - Claude、GPT、Gemini、Ollama 等 20+ 提供商（通过 Vercel AI SDK）
- **双语言架构** - TypeScript 负责智能，Rust 负责安全边界
- **Markdown 记忆系统** - 透明、Git 友好的知识管理
- **多平台 IM 集成** - Telegram、Discord、Slack、飞书、Email

## 快速开始

### 环境要求

- [Bun](https://bun.sh/) 1.3+（必需）
- [Rust](https://www.rust-lang.org/) 1.75+（可选，用于 zero-* 服务）

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

### 配置

创建 `~/.codecoder/config.json`：

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "your-api-key"
    }
  }
}
```

敏感凭证请使用 `~/.codecoder/secrets.json`（权限设为 600）。

## 架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              档位控制层                                  │
│              ┌─────────────────────────────────────────┐                │
│              │     档位选择器: P  N  D  S  M            │                │
│              ├───────────┬───────────┬───────────┬─────┤                │
│              │   观察    │   决策    │   执行    │档位 │                │
│              │   0-100   │   0-100   │   0-100   │ ↑↓  │                │
│              └───────────┴───────────┴───────────┴─────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                             观察者网络                                   │
│   ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐              │
│   │ CodeWatch │ │WorldWatch │ │ SelfWatch │ │ MetaWatch │              │
│   │  (代码)   │ │  (市场)   │ │ (Agent)   │ │  (系统)   │              │
│   └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘              │
│         └─────────────┴─────────────┴─────────────┘                     │
│                              │                                          │
│                    ┌─────────▼─────────┐                                │
│                    │     共识引擎      │                                │
│                    └───────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                              用户接入层                                  │
│   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐  │
│   │   TUI   │   │   Web   │   │   CLI   │   │Telegram │   │ Discord │  │
│   │  :4400  │   │  :4401  │   │         │   │   Bot   │   │   Bot   │  │
│   └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘  │
└────────┼─────────────┼─────────────┼─────────────┼─────────────┼────────┘
         │             │             │             │             │
         └─────────────┴──────┬──────┴─────────────┴─────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       核心服务层 (TypeScript/Bun)                         │
│   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                │
│   │  API Server  │   │ Agent 引擎   │   │  记忆系统    │                │
│   │    :4400     │◄─►│  (31 Agents) │◄─►│  (Markdown)  │                │
│   └──────────────┘   └──────────────┘   └──────────────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Rust 服务层 (5 Crates)                           │
│              ┌──────────────────────────────────────┐                   │
│              │       Zero CLI Daemon :4402          │                   │
│              │       (统一入口 + 进程编排)            │                   │
│              └─────────────────┬────────────────────┘                   │
│   ┌────────────────────────────┼────────────────────────────┐           │
│   │                            │                            │           │
│   ▼                            ▼                            ▼           │
│ ┌──────────┐            ┌──────────────┐            ┌──────────┐        │
│ │zero-core │            │  zero-hub    │            │zero-     │        │
│ │ (工具库) │            │(网关+渠道+   │            │trading   │        │
│ │          │            │  工作流)     │            │ (交易)   │        │
│ └──────────┘            └──────────────┘            └──────────┘        │
└─────────────────────────────────────────────────────────────────────────┘
```

### 核心原则

> **"TypeScript 负责智能，Rust 负责安全边界"**

| 任务类型 | 最佳工具 | 原因 |
|---------|---------|------|
| 协议解析、签名验证、调度 | Rust (zero-*) | 规则明确，需要高性能和安全性 |
| 意图理解、代码生成、决策建议 | TypeScript (LLM) | 需要推理和领域知识 |

## 观察者网络

观察者网络是 CodeCoder 以观察为中心设计的核心，将系统从"执行中心"转变为"观察中心"。

### 四大观察者

| 观察者 | 观察对象 | 关联 Agent |
|--------|----------|-----------|
| **CodeWatch** | 代码库变化、Git 活动、构建状态 | explore |
| **WorldWatch** | 市场数据、新闻舆情、API 变化 | macro, trader |
| **SelfWatch** | Agent 行为、决策日志、错误模式 | code-reviewer, security-reviewer, decision |
| **MetaWatch** | 观察质量、系统健康、观察盲点 | observer |

### 观察 → 共识 → 响应

```
事件 → 缓冲 → 聚合 → 共识 → 模式控制器 → 响应
                               │
               ┌───────────────┼───────────────┐
               ▼               ▼               ▼
            通知器          分析器          执行器
```

系统持续观察，形成对当前状态的共识，并根据你的档位设置做出响应。

## Agent 系统

CodeCoder 拥有 31 个专用 AI Agent，按 3 模式系统组织：

### 模式

| 模式 | 主 Agent | 用途 |
|------|---------|------|
| **@build**（默认） | `build` | 软件开发 |
| **@writer** | `writer` | 长文创作 |
| **@decision** | `decision` | 基于祝融说的决策分析 |

### 使用方式

```bash
# 默认 build 模式
bun dev

# 写作模式
bun dev -m writer

# 决策模式
bun dev -m decision

# 访问特定能力
bun dev @build:security-review
bun dev @decision:macro
```

### Agent 分类

| 分类 | Agent | 用途 |
|------|-------|------|
| **主模式 (4)** | build, plan, writer, autonomous | 主要交互模式 |
| **逆向工程 (2)** | code-reverse, jar-code-reverse | 代码分析 |
| **工程质量 (6)** | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | 代码质量保障 |
| **内容创作 (5)** | expander, expander-fiction, expander-nonfiction, proofreader, verifier | 写作辅助 |
| **祝融说系列 (8)** | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | 决策与领域专长 |
| **产品 (2)** | prd-generator, feasibility-assess | 产品需求 |
| **系统 (3)** | compaction, title, summary | 内部使用（隐藏） |

## 开发

### 命令

```bash
# 安装依赖
bun install

# 在当前目录运行 TUI
bun dev

# 启动无头 API 服务器
bun dev serve --port 4400

# 类型检查
bun turbo typecheck

# 运行测试（需在特定包内）
cd packages/ccode && bun test

# 构建独立可执行文件
cd packages/ccode && bun run build

# 构建 Rust 服务
./ops.sh build rust

# 服务管理
./ops.sh start all      # 启动所有服务
./ops.sh stop           # 停止服务
./ops.sh status         # 查看状态
./ops.sh health         # 健康检查
```

### 端口配置

| 服务 | 端口 | 说明 |
|------|------|------|
| CodeCoder API | 4400 | 主 API 服务器 |
| Web 前端 | 4401 | React Web UI |
| Zero Daemon | 4402 | Rust 统一入口 |
| Whisper | 4403 | 语音转写 |
| MCP Server | 4420 | Model Context Protocol |

### Monorepo 结构

```
codecoder/
├── packages/                    # TypeScript 包
│   ├── ccode/                   # 核心 CLI（入口点）
│   │   ├── src/agent/           # 31 个 Agent 定义
│   │   ├── src/cli/cmd/tui/     # 终端 UI (Solid.js)
│   │   └── src/observer/        # 观察者网络 + 档位系统
│   ├── memory/                  # 记忆模块
│   ├── util/                    # 共享工具
│   └── web/                     # Web 前端 (React)
│
├── services/                    # Rust 服务 (5 crates)
│   ├── zero-cli/                # CLI + Daemon
│   ├── zero-core/               # 核心工具 (grep/glob/edit, NAPI)
│   ├── zero-hub/                # 网关 + 渠道 + 工作流
│   ├── zero-trading/            # 交易系统
│   └── zero-common/             # 共享配置、日志、事件
│
├── memory/                      # 项目记忆
│   ├── daily/                   # 每日笔记（流）
│   └── MEMORY.md                # 长期记忆（沉积）
│
└── docs/                        # 文档
```

## 设计哲学

CodeCoder 建立在"祝融说"哲学之上：

> **"可持续决策比最优决策更重要"**

这体现在档位系统的设计中：
- **驻车 (P)**：不需要时保存资源
- **空档 (N)**：先观察再行动 —— 理解先于干预
- **行车 (D)**：平衡自主性与人类监督
- **运动 (S)**：对常规任务信任系统
- **手动 (M)**：始终提供精细的人工控制

CLOSE 框架用于评估决策：
- **C**apacity（能力）：我能做到吗？
- **L**everage（杠杆）：投入产出比如何？
- **O**pportunity（机会）：我会错过什么？
- **S**ustainability（可持续性）：我能持续下去吗？← **一票否决权**
- **E**xit（退出）：失败了如何全身而退？

详见 [设计哲学](./docs/architecture/DESIGN_PHILOSOPHY.md)。

## 文档

- [架构概览](./docs/architecture/ARCHITECTURE.md)
- [设计哲学](./docs/architecture/DESIGN_PHILOSOPHY.md)
- [入门指南](./docs/guides/beginners-guide.md)
- [项目说明](./CLAUDE.md)

## 贡献

欢迎贡献！请：

1. 先开 issue 讨论重大变更
2. 遵循约定式提交格式（`feat:`、`fix:`、`docs:` 等）
3. 保持 PR 小而专注
4. 为新功能编写测试

## 许可证

[MIT](./LICENSE)
