# OpenFang vs CodeCoder 对比分析

> 生成时间: 2026-02-28
> 版本: 1.0.0

## 上下文

本文档对比分析两个 Agent 框架/系统：
- **OpenFang**: RightNow-AI 开发的开源 Agent 操作系统
- **CodeCoder**: 本项目，融合工程能力与决策智慧的个人工作台

## 执行摘要

| 维度 | OpenFang | CodeCoder |
|------|----------|-----------|
| **定位** | Agent Operating System | 个人 AI 工作台 |
| **语言** | 纯 Rust (137K LOC, 14 crates) | TypeScript + Rust 混合 |
| **核心理念** | 自主运行的 "Hands" | 交互式 Agent 协作 |
| **UI 模式** | Web Dashboard | TUI 优先 (Solid.js) |
| **代码量** | ~137K LOC Rust | ~50K TS + ~30K Rust |
| **二进制大小** | ~32MB 单文件 | 多服务架构 |

---

## 1. 架构对比

### 1.1 OpenFang 架构

```
┌─────────────────────────────────────────────────────────┐
│                 OpenFang (单一 Rust 二进制)              │
├─────────────────────────────────────────────────────────┤
│  openfang-cli      ─── CLI 交互 + daemon 启动           │
│  openfang-kernel   ─── 核心调度器                       │
│  openfang-runtime  ─── Agent 执行引擎                   │
│  openfang-hands    ─── 7 个内置 Hands                   │
│  openfang-channels ─── 40+ 渠道适配器                   │
│  openfang-memory   ─── SQLite + 向量存储                │
│  openfang-api      ─── HTTP API + Dashboard             │
│  openfang-wire     ─── 网络协议 (OFP)                   │
│  openfang-skills   ─── 技能系统                         │
│  openfang-types    ─── 类型定义                         │
└─────────────────────────────────────────────────────────┘
```

**特点：**
- **单体架构**：所有功能编译到一个 ~32MB 二进制
- **WASM 沙箱**：双计量器执行环境
- **OFP 协议**：自定义 P2P 网络协议
- **Dashboard UI**：Alpine.js SPA

### 1.2 CodeCoder 架构

```
┌─────────────────────────────────────────────────────────┐
│                CodeCoder (分布式微服务)                   │
├─────────────────────────────────────────────────────────┤
│  packages/ccode    ─── 核心 CLI (TypeScript/Bun)        │
│  packages/web      ─── Web 前端 (React)                 │
│  services/zero-*   ─── Rust 微服务集群                  │
│    ├── zero-cli    ─── Daemon + 进程编排               │
│    ├── zero-gateway─── 认证/路由/配额                   │
│    ├── zero-channels── IM 渠道适配                     │
│    └── zero-workflow── Webhook/Cron                    │
└─────────────────────────────────────────────────────────┘
```

**特点：**
- **混合架构**：TypeScript 做高层抽象，Rust 做高性能服务
- **TUI 优先**：Solid.js + OpenTUI 终端界面
- **MCP 协议**：标准化的模型上下文协议
- **Markdown 记忆**：透明可读的双层存储

### 1.3 架构设计哲学对比

| 维度 | OpenFang | CodeCoder |
|------|----------|-----------|
| **构建策略** | 单二进制，零依赖部署 | 微服务，按需启动 |
| **复杂度管理** | 编译时验证，Rust 强类型 | 运行时灵活，TypeScript 快速迭代 |
| **扩展性** | WASM 插件 + OFP 网络 | MCP 协议 + HTTP API |
| **运维负担** | 极低（单文件） | 中等（多服务编排） |

---

## 2. Agent 设计理念对比

### 2.1 OpenFang: Hands (自主执行包)

OpenFang 的核心创新是 **Hands** — 预构建的自主能力包：

| Hand | 功能 |
|------|------|
| **Clip** | YouTube 视频剪辑、字幕生成、发布到 Telegram/WhatsApp |
| **Lead** | 每日运行，发现/评分/去重潜在客户 |
| **Collector** | OSINT 级情报收集，变化检测，知识图谱构建 |
| **Predictor** | 超级预测引擎，Brier 评分跟踪 |
| **Researcher** | 深度研究，CRAAP 标准评估，APA 引用 |
| **Twitter** | 自主 Twitter 账号管理，发布审批队列 |
| **Browser** | 网页自动化，购买强制审批 |

**Hand 结构示例：**
```toml
# HAND.toml
[hand]
name = "researcher"
version = "0.1.0"
tools = ["web_search", "file_read", "file_write"]

[settings]
depth = 3
languages = ["en", "zh"]
```

**执行模式：**
- Hands 可配置 Cron 调度（如 Lead 每日运行）
- 敏感操作进入审批队列
- 支持 FangHub 发布和共享

### 2.2 CodeCoder: 23 个专业 Agent

CodeCoder 采用更细粒度的 **Agent 矩阵**：

| 类别 | Agent | 特色 |
|------|-------|------|
| **主模式** | build, plan | 开发和规划 |
| **工程** | code-reviewer, security-reviewer, tdd-guide, architect | 代码质量 |
| **逆向** | code-reverse, jar-code-reverse | 代码逆向分析 |
| **内容** | writer, proofreader | 长文写作与校对 |
| **祝融说** | observer, decision, macro, trader, picker, miniproduct, ai-engineer | 哲学 + 领域 |
| **系统** | autonomous, general | 自主执行与辅助 |

**Agent 定义示例：**
```typescript
// packages/ccode/src/agent/agent.ts
{
  name: "decision",
  description: "基于可持续决策理论的决策智慧师，使用CLOSE五维评估框架",
  prompt: PROMPT_DECISION,
  temperature: 0.6,
  permission: PermissionNext.merge(defaults, user),
}
```

**触发机制：**
- 关键词触发（如 `@decision`）
- 上下文自动推荐
- Fuse.js 模糊搜索匹配

### 2.3 关键差异

| 维度 | OpenFang Hands | CodeCoder Agents |
|------|----------------|------------------|
| **执行模式** | 自主调度运行 (24/7) | 交互式调用 |
| **触发方式** | Cron/事件驱动 | 用户请求/上下文匹配 |
| **状态管理** | 内置 SQLite 持久化 | Markdown 双层记忆 |
| **审批机制** | 强制审批队列 | 权限规则 (ask/allow/deny) |
| **可扩展性** | FangHub 发布 | prompt 文件 + 配置 |
| **哲学框架** | 无 | 祝融说 + CLOSE 决策 |

---

## 3. 技术栈对比

### 3.1 运行时与构建

| 组件 | OpenFang | CodeCoder |
|------|----------|-----------|
| **主语言** | Rust 100% | TypeScript 60% + Rust 40% |
| **运行时** | Native Binary | Bun (TS) + Native (Rust) |
| **构建** | Cargo | Turborepo + Cargo Workspace |
| **桌面应用** | Tauri 2.0 | 无 (TUI 优先) |
| **测试** | 1,767+ 单元测试 | Vitest + Rust tests |

### 3.2 性能基准

**OpenFang 官方基准 (来自 README):**

```
Cold Start Time:
  ZeroClaw    10 ms     (对照项目)
  OpenFang   180 ms     ★
  LangGraph  2.5 sec
  CrewAI     3.0 sec

Idle Memory:
  ZeroClaw     5 MB
  OpenFang    40 MB     ★
  LangGraph  180 MB
  CrewAI     200 MB
```

**CodeCoder 估算值（混合架构）：**
- TUI 启动: ~500ms (Bun 冷启动)
- 完整服务启动: ~3-5s (所有微服务)
- 单 Agent 执行: ~100-200ms (不含 LLM 调用)

### 3.3 渠道支持对比

| 渠道 | OpenFang | CodeCoder |
|------|:--------:|:---------:|
| Telegram | ✅ | ✅ |
| Discord | ✅ | ✅ |
| Slack | ✅ | ✅ |
| 飞书 | ✅ | ✅ |
| 企业微信 | ✅ | ✅ |
| WhatsApp | ✅ | ✅ |
| Email | ✅ | ✅ |
| iMessage | ❓ | ✅ |
| Matrix | ❓ | ✅ |
| 钉钉 | ❓ | ✅ |
| **总计** | 40+ | 12+ |

---

## 4. 安全模型对比

### 4.1 OpenFang: 16 层安全

OpenFang 采用深度防御策略：

1. **WASM 双计量沙箱** - 指令和资源双重限制
2. **工具白名单** - 每个 Hand 声明可用工具
3. **资源配额** - CPU/内存/网络限制
4. **审批队列** - 敏感操作人工审批
5. **Merkle 哈希链审计** - 不可篡改的操作日志
6. **配对码认证** - 设备绑定
7. **JWT 会话管理** - 短期令牌
8. **... 等共 16 层**

### 4.2 CodeCoder: 权限规则系统

```typescript
// 细粒度权限控制示例
permission: {
  "*": "allow",
  doom_loop: "ask",
  external_directory: {
    "*": "ask",
    [Truncate.DIR]: "allow"
  },
  read: {
    "*.env": "ask",
    "*.env.*": "ask"
  },
}
```

**安全特性：**
- **Hook 系统** - PreToolUse/PostToolUse/Stop 钩子
- **自动审批配置** - 细粒度 allowedTools
- **RBAC** - Gateway 层角色权限
- **Secrets 隔离** - secrets.json 单独存储 (600 权限)
- **审计日志** - zero-common 提供

### 4.3 安全模型对比表

| 安全维度 | OpenFang | CodeCoder |
|----------|----------|-----------|
| 沙箱执行 | WASM 双计量器 | Docker 容器 (可选) |
| 审批机制 | 审批队列 UI | Hook + ask 模式 |
| 审计追踪 | Merkle 哈希链 | 日志 + SQLite |
| 权限粒度 | 工具级白名单 | 路径 + 操作级 |
| 密钥管理 | 内置 + 加密存储 | secrets.json 分离 |

---

## 5. 记忆系统对比

### 5.1 OpenFang 记忆架构

```
┌─────────────────────────────────────────────────┐
│              OpenFang Memory Layer              │
├─────────────────────────────────────────────────┤
│  SQLite Database                                │
│  ├── agents (Agent 状态和配置)                  │
│  ├── messages (对话历史)                        │
│  ├── artifacts (生成的文件)                     │
│  └── knowledge (向量嵌入，可选)                 │
├─────────────────────────────────────────────────┤
│  向量存储 (可选)                                │
│  └── 嵌入模型支持语义搜索                       │
├─────────────────────────────────────────────────┤
│  知识图谱 (Collector Hand)                      │
│  └── 实体关系建模                               │
└─────────────────────────────────────────────────┘
```

### 5.2 CodeCoder 双层记忆架构

```
┌─────────────────────────────────────────────────┐
│          CodeCoder 双层记忆 (流 + 沉积)          │
├─────────────────────────────────────────────────┤
│                                                  │
│  第一层: 每日笔记 (流/Stream)                    │
│  ─────────────────────────────                   │
│  memory/daily/2026-02-28.md                      │
│  ┌───────────────────────────┐                   │
│  │ ## 10:30 - 对比分析任务    │                  │
│  │ - 创建了 OpenFang 对比文档 │                  │
│  │ - 学习了 Hands 概念        │                  │
│  │                           │      整合        │
│  │ ## 14:00 - 架构改进       │    ────────►     │
│  │ - 考虑借鉴审批队列设计    │                  │
│  └───────────────────────────┘                   │
│                                                  │
│  特点: 仅追加 | 按时间线 | 类比：河流           │
│                                                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  第二层: 长期记忆 (沉积/Sediment)                │
│  ─────────────────────────────                   │
│  memory/MEMORY.md                                │
│  ┌───────────────────────────┐                   │
│  │ ## 用户偏好               │                   │
│  │ - 代码风格: 函数式        │                   │
│  │ - 审批: 自动批准测试命令  │                   │
│  │                           │                   │
│  │ ## 关键决策               │                   │
│  │ - 2026-02-28: 借鉴        │                   │
│  │   OpenFang 审批队列设计   │                   │
│  └───────────────────────────┘                   │
│                                                  │
│  特点: 可编辑 | 结构化 | 类比：沉积岩           │
│                                                  │
└─────────────────────────────────────────────────┘
```

### 5.3 记忆系统对比表

| 维度 | OpenFang | CodeCoder |
|------|----------|-----------|
| **存储格式** | SQLite + 向量 | Markdown 文件 |
| **人类可读** | 需工具查询 | 直接编辑 |
| **Git 友好** | 二进制数据库 | 纯文本，可 diff |
| **语义搜索** | 向量嵌入 | 无 (禁止复杂检索) |
| **知识图谱** | Collector Hand 构建 | 无 |
| **调试方式** | SQL 查询 | 直接编辑 .md 文件 |

---

## 6. 适用场景分析

### 6.1 OpenFang 最佳场景

| 场景 | 适合度 | 原因 |
|------|:------:|------|
| **24/7 自主任务** | ⭐⭐⭐⭐⭐ | 核心设计目标 |
| **Lead 生成/销售** | ⭐⭐⭐⭐⭐ | Lead Hand 专门优化 |
| **社交媒体管理** | ⭐⭐⭐⭐⭐ | Twitter Hand + 审批队列 |
| **单一部署需求** | ⭐⭐⭐⭐⭐ | 单二进制，零依赖 |
| **桌面应用** | ⭐⭐⭐⭐ | Tauri Dashboard |
| **性能敏感** | ⭐⭐⭐⭐ | 纯 Rust，低内存 |

### 6.2 CodeCoder 最佳场景

| 场景 | 适合度 | 原因 |
|------|:------:|------|
| **交互式开发** | ⭐⭐⭐⭐⭐ | TUI + Agent 矩阵 |
| **代码审查/TDD** | ⭐⭐⭐⭐⭐ | 工程类 Agent |
| **长文写作** | ⭐⭐⭐⭐⭐ | writer + proofreader |
| **宏观经济分析** | ⭐⭐⭐⭐⭐ | ZRS 系列 Agent |
| **决策支持** | ⭐⭐⭐⭐⭐ | CLOSE 框架 + decision Agent |
| **终端工作流** | ⭐⭐⭐⭐⭐ | TUI 优先设计 |
| **定制化需求** | ⭐⭐⭐⭐ | prompt 文件可编辑 |

### 6.3 场景选择决策树

```
你的需求是什么？
│
├── 需要 Agent 自主运行 (无人值守)?
│   └── ✅ OpenFang (Hands + Cron 调度)
│
├── 需要实时交互式协作?
│   └── ✅ CodeCoder (TUI + 对话式)
│
├── 需要领域决策支持 (宏观/交易/选品)?
│   └── ✅ CodeCoder (祝融说系列)
│
├── 需要单文件部署?
│   └── ✅ OpenFang (~32MB 二进制)
│
├── 需要透明可审计的记忆?
│   └── ✅ CodeCoder (Markdown 记忆)
│
└── 需要极致性能?
    └── ✅ OpenFang (纯 Rust)
```

---

## 7. 可学习的设计模式

### 7.1 从 OpenFang 可借鉴

#### 1. Hands 概念 → 组合 Agent

将 CodeCoder 的某些 Agent 组合包装成可调度的 "Hands"：

```
lead-gen Hand = explore + researcher + writer
```

**实现建议：**
```typescript
// packages/ccode/src/agent/hands/lead-gen.ts
export const leadGenHand: Hand = {
  name: "lead-gen",
  schedule: "0 9 * * *",  // 每日 9:00
  agents: ["explore", "researcher", "writer"],
  pipeline: "sequential",
  approvalRequired: ["email_send"],
}
```

#### 2. HAND.toml 清单 → 声明式 Agent 配置

为 Agent 添加声明式配置文件：

```toml
# agents/decision/AGENT.toml
[agent]
name = "decision"
version = "1.0.0"
framework = "close"

[tools]
allowed = ["web_search", "file_read"]
denied = ["file_write", "bash"]

[metrics]
dashboard = true
track_decisions = true
```

#### 3. 审批队列 → 统一审批 UI

敏感操作统一进入审批队列，而非当前的 ask/allow 二选一：

```
┌─────────────────────────────────────────────────┐
│              CodeCoder 审批队列                   │
├─────────────────────────────────────────────────┤
│  ⏳ [email_send] 发送外链邮件到 user@example.com │
│     └── 来源: lead-gen Hand                      │
│     └── 时间: 10:30                              │
│     └── [批准] [拒绝] [稍后]                     │
│                                                  │
│  ⏳ [purchase] 购买 $50 API Credits              │
│     └── 来源: autonomous Agent                   │
│     └── 时间: 11:00                              │
│     └── [批准] [拒绝] [稍后]                     │
└─────────────────────────────────────────────────┘
```

#### 4. OFP 网络协议 → Agent 间通信标准化

考虑为 zero-* 服务间通信定义更正式的协议：

```rust
// services/zero-common/src/protocol.rs
pub enum AgentMessage {
    TaskAssign { agent_id: String, task: Task },
    StatusUpdate { progress: f32, status: Status },
    ResultComplete { result: TaskResult },
    ApprovalRequest { action: PendingAction },
}
```

### 7.2 CodeCoder 的独特优势

#### 1. 祝融说哲学集成

OpenFang 是纯工具，CodeCoder 融入了决策哲学：

- **CLOSE 框架** - 五维决策评估
- **观察者理论** - 观察即创造
- **可持续决策 > 最优决策** - 保持余量

这是 CodeCoder 的核心差异化。

#### 2. 双层记忆的透明性

```bash
# 直接查看今天做了什么
cat memory/daily/2026-02-28.md

# 直接编辑长期记忆
vim memory/MEMORY.md

# Git diff 查看记忆变化
git diff memory/
```

无需任何工具，人类和 Git 都能直接理解。

#### 3. 混合架构的灵活性

```
高不确定性任务 → ccode (TypeScript + LLM)
  - 意图理解
  - 代码生成
  - 决策建议

高确定性任务 → zero-* (Rust)
  - 协议解析
  - 签名验证
  - 调度执行
```

这种划分让每种语言发挥各自优势。

#### 4. TUI 优先的效率

开发者在终端工作，TUI 消除了上下文切换：

```
┌───────────────────────────────────────────────┐
│  CodeCoder                    CPU: 2% MEM: 40M │
├───────────────────────────────────────────────┤
│  > @decision 用CLOSE框架分析这个职业选择       │
│                                               │
│  [decision] 正在分析...                        │
│                                               │
│  CLOSE 五维评估:                               │
│  • Clarity (清晰度): 7/10                     │
│  • Leverage (杠杆): 6/10                      │
│  • Optionality (可选性): 8/10                 │
│  • Sustainability (可持续): 5/10              │
│  • Edge (优势): 7/10                          │
│                                               │
│  综合建议: ...                                 │
└───────────────────────────────────────────────┘
```

---

## 8. 结论

### 8.1 核心定位差异

| | OpenFang | CodeCoder |
|---|----------|-----------|
| **核心问题** | "如何让 Agent 自主工作" | "如何让 Agent 辅助人工作" |
| **用户画像** | 需要自动化运营的团队 | 需要 AI 辅助开发的工程师 |
| **交互模式** | 配置 → 运行 → 查看报告 | 对话 → 协作 → 迭代 |
| **设计哲学** | 效率优先，自动化优先 | 可持续决策，人机协作 |

### 8.2 互补可能性

两者可以互补：

1. 使用 **CodeCoder** 进行开发、决策、写作
2. 将成熟的重复性流程部署为 **OpenFang Hands** 自主执行

```
开发阶段 (CodeCoder)          运营阶段 (OpenFang)
────────────────────          ────────────────────
  code-reviewer                   Lead Hand
  tdd-guide          ──►          Collector Hand
  writer                          Twitter Hand
```

### 8.3 选择建议

| 如果你... | 选择 |
|-----------|------|
| 需要 24/7 自主运行的 Agent | OpenFang |
| 需要交互式开发辅助 | CodeCoder |
| 需要决策支持和哲学框架 | CodeCoder |
| 需要单二进制部署 | OpenFang |
| 需要透明可编辑的记忆 | CodeCoder |
| 需要极致性能 | OpenFang |
| 需要领域专家 Agent (宏观/交易/选品) | CodeCoder |

---

## 附录

### A. 关键文件参考

**OpenFang:**
- GitHub: https://github.com/RightNow-AI/openfang
- 文档: https://openfang.sh/docs

**CodeCoder:**
- 架构: `docs/architecture/ARCHITECTURE.md`
- Agent 定义: `packages/ccode/src/agent/agent.ts`
- 渠道服务: `services/zero-channels/src/lib.rs`
- 设计哲学: `docs/architecture/DESIGN_PHILOSOPHY.md`

### B. 版本信息

- OpenFang: 基于 2026-02 公开信息分析
- CodeCoder: 当前版本 (2026-02-28)

### C. 更新历史

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-02-28 | 1.0.0 | 初始版本 |
