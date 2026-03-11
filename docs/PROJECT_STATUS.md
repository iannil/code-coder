# CodeCoder 项目状态文档

> 生成时间: 2026-03-11
> 状态: 开发中 (功能完成度 99%+)

本文档为 LLM 提供项目的结构化概览，便于快速理解和协作。

---

## 项目概览

| 属性 | 值 |
|------|-----|
| **名称** | CodeCoder |
| **版本** | 0.0.1 (开发中) |
| **定位** | 观察中心的 AI Agent 系统 |
| **特色** | 档位控制 (P/N/D/S/M) + 观察者网络 + 31 个专业 Agent |
| **技术栈** | TypeScript (Bun) + Rust + Turborepo Monorepo |
| **Rust 代码量** | 200,785 行 |
| **Agent 数量** | 31 个 |

### 核心特性

1. **观察者网络 (Observer Network)**: 四大观察者 (CodeWatch, WorldWatch, SelfWatch, MetaWatch) 持续监控
2. **档位控制 (Gear System)**: P/N/D/S/M 五档 + 三旋钮 (Observe/Decide/Act) 精细控制
3. **响应代理 (Response Agents)**: 31 个专业 AI Agent 作为响应能力
4. **Rust-First 架构**: 高确定性任务用 Rust，高不确定性任务用 LLM

---

## 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                    TypeScript Layer (高不确定性)                     │
│     packages/ccode/     TUI, SDK Client, Document, Session          │
└─────────────────────────────┬───────────────────────────────────────┘
                              │ NAPI-RS / HTTP / WebSocket
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Rust Layer (高确定性)                             │
│                                                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │ zero-cli    │  │ zero-core   │  │ zero-hub    │  │zero-trading│ │
│  │ (Daemon)    │  │ (Tools)     │  │ (Gateway)   │  │ (量化)     │ │
│  │ :4402       │  │ NAPI        │  │ Auth/Channel│  │            │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 目录结构

```
/
├── packages/                    # TypeScript 包
│   ├── ccode/                   # 核心 CLI (182k 行, 主要是迁移遗留)
│   │   ├── src/
│   │   │   ├── agent/           # Agent 定义 (@deprecated → Rust)
│   │   │   ├── cli/cmd/tui/     # 终端 UI (Solid.js + OpenTUI)
│   │   │   ├── sdk/             # Rust daemon 客户端 SDK
│   │   │   ├── observer/        # 观察者 TypeScript 客户端
│   │   │   ├── session/         # 会话管理 (@deprecated 部分)
│   │   │   └── provider/        # LLM 提供商 (@deprecated → Rust)
│   │   └── test/                # 测试 (74.93% 覆盖率)
│   ├── core/                    # NAPI 绑定包装
│   ├── web/                     # Web 前端 (React + Vite)
│   └── util/                    # 共享工具库
│
├── services/                    # Rust 服务 (4 Crates, 200k 行)
│   ├── zero-cli/                # CLI + Daemon (统一入口)
│   │   ├── src/server/          # HTTP/WebSocket 服务 :4402
│   │   ├── src/agent/           # Agent 定义 + 执行
│   │   ├── src/observer/        # 观察者网络
│   │   └── src/gear/            # 档位控制
│   ├── zero-core/               # 核心工具库
│   │   ├── src/tools/           # 18 个原生工具
│   │   ├── src/security/        # 安全模块
│   │   ├── src/trace/           # 追踪系统
│   │   └── src/napi/            # NAPI 绑定
│   ├── zero-hub/                # 服务中枢
│   │   ├── src/gateway/         # 认证 + RBAC + 配额
│   │   ├── src/channels/        # IM 渠道 (Telegram/飞书等)
│   │   └── src/workflow/        # 调度 + Webhook
│   └── zero-trading/            # 量化交易模块
│
├── memory/                      # 双层记忆系统
│   ├── MEMORY.md                # 长期记忆 (沉积层)
│   └── daily/                   # 每日笔记 (流动层)
│
├── docs/                        # 文档
│   ├── architecture/            # 架构文档 (13 个)
│   ├── philosophy/              # 知识库 (宏观经济学等)
│   ├── progress/                # 进行中的工作
│   └── reports/completed/       # 已完成的报告 (23 个)
│
└── example/                     # 示例和测试数据
```

---

## Agent 清单 (31 个)

### 主模式 (4)

| Agent | 用途 | 模式 |
|-------|------|------|
| `build` | 软件开发 (默认) | @build |
| `plan` | 实现规划 | @build |
| `writer` | 长篇写作 | @writer |
| `autonomous` | 自主执行 | @build |

### 工程质量 (6)

| Agent | 用途 |
|-------|------|
| `general` | 通用助手 |
| `explore` | 代码库探索 |
| `code-reviewer` | 代码审查 |
| `security-reviewer` | 安全审查 |
| `tdd-guide` | TDD 指导 |
| `architect` | 架构设计 |

### 逆向工程 (2)

| Agent | 用途 |
|-------|------|
| `code-reverse` | 代码逆向 |
| `jar-code-reverse` | JAR 逆向 |

### 内容创作 (4)

| Agent | 用途 |
|-------|------|
| `expander` | 内容扩展 |
| `expander-fiction` | 小说扩展 |
| `expander-nonfiction` | 非虚构扩展 |
| `proofreader` | 校对 |

### 祝融说系列 (8)

| Agent | 用途 | 模式 |
|-------|------|------|
| `observer` | 系统观察 | @decision |
| `decision` | CLOSE 决策 | @decision |
| `macro` | 宏观经济分析 | @decision |
| `trader` | 交易策略 | @decision |
| `picker` | 选品分析 | @decision |
| `miniproduct` | 微产品设计 | @decision |
| `ai-engineer` | AI 工程 | @decision |
| `value-analyst` | 价值分析 | @decision |

### 产品运营 (3)

| Agent | 用途 |
|-------|------|
| `verifier` | 形式化验证 |
| `prd-generator` | PRD 生成 |
| `feasibility-assess` | 可行性评估 |

### 辅助 (1)

| Agent | 用途 |
|-------|------|
| `synton-assistant` | 协同助手 |

### 系统隐藏 (3)

| Agent | 用途 |
|-------|------|
| `compaction` | 上下文压缩 |
| `title` | 标题生成 |
| `summary` | 摘要生成 |

---

## 端口配置

| 服务 | 端口 | 技术 |
|------|------|------|
| CodeCoder API Server | 4400 | Bun/TypeScript |
| Web Frontend | 4401 | Vite (React) |
| Zero CLI Daemon | 4402 | Rust/Axum |
| Faster Whisper Server | 4403 | Docker |
| Redis Server | 4410 | Docker |
| MCP Server | 4420 | HTTP |

---

## 废弃代码清单 (待清理)

以下模块已迁移到 Rust，TypeScript 实现标记为 `@deprecated`:

| 文件 | 行数 | Rust 替代 |
|------|------|-----------|
| `agent/agent.ts` | ~800 | `zero-cli/src/agent/` |
| `provider/provider.ts` | ~1,400 | `zero-cli/src/providers/` |
| `session/index.ts` | ~500 | `zero-cli/src/session/` |
| `session/prompt.ts` | ~300 | `zero-cli/src/session/` |
| `session/message-v2.ts` | ~400 | `zero-cli/src/session/` |
| `security/index.ts` | ~200 | `zero-core/src/security/` |
| `autonomous/index.ts` | ~300 | `zero-cli/src/autonomous/` |
| `config/config.ts` | ~500 | `zero-cli/src/config/` |
| `observer/types.ts` | ~600 | 保留 (仅类型定义) |

**累计可删除**: ~4,500 行 (迁移完成后)

---

## 核心 API 端点

### Session API (`/api/v1/sessions`)

```
GET    /api/v1/sessions           # 列出会话
POST   /api/v1/sessions           # 创建会话
GET    /api/v1/sessions/:id       # 获取会话
DELETE /api/v1/sessions/:id       # 删除会话
POST   /api/v1/sessions/:id/fork  # 分叉会话
POST   /api/v1/sessions/:id/compact # 压缩会话
```

### Agent API (`/api/v1/agents`)

```
GET    /api/v1/agents             # 列出 Agent
POST   /api/v1/agents/execute     # 执行 Agent (SSE)
```

### Observer API (`/api/v1/observer`)

```
GET    /api/v1/observer/status    # 观察者状态
GET    /api/v1/observer/gear      # 当前档位
POST   /api/v1/observer/gear      # 切换档位
GET    /api/v1/observer/consensus # 共识快照
```

### WebSocket API (`/ws`)

```
ws://localhost:4402/ws

# Client → Server
{ type: "agent_request", session_id, agent, message }
{ type: "agent_cancel", id }
{ type: "tool_request", tool, params }

# Server → Client
{ type: "agent_text", content }
{ type: "agent_tool_call", tool, arguments }
{ type: "agent_complete", reason, usage }
```

---

## 配置位置

```
~/.codecoder/
├── config.json         # 核心配置
├── secrets.json        # 凭证 (gitignored, 600 权限)
├── trading.json        # 交易配置
├── channels.json       # IM 渠道配置
└── providers.json      # LLM 提供商配置
```

---

## 开发命令

```bash
# 安装依赖
bun install

# 开发模式 (D 档)
bun dev

# 指定档位
bun dev --gear S    # Sport 模式 (高自主)
bun dev --gear N    # Neutral 模式 (纯观察)
bun dev --gear M --observe 80 --decide 30 --act 10  # 手动模式

# 类型检查
bun turbo typecheck

# 测试 (需在具体包内运行)
cd packages/ccode && bun test

# 构建 Rust 服务
cd services && cargo build --release

# 启动服务
./ops.sh start all
```

---

## 相关文档

- [CLAUDE.md](../CLAUDE.md) - 项目指南 (LLM 主要参考)
- [docs/architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) - 架构详解
- [docs/FEATURES.md](FEATURES.md) - 功能清单
- [docs/DEBT.md](DEBT.md) - 技术债务
- [docs/RUNBOOK.md](RUNBOOK.md) - 运维手册
- [docs/progress/REMAINING_TASKS.md](progress/REMAINING_TASKS.md) - 剩余任务

---

## 更新记录

| 日期 | 变更 |
|------|------|
| 2026-03-11 | 初始版本，整合项目状态信息 |
