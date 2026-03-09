# CodeCoder 项目全景文档

> 最后更新: 2026-03-08
> 文档类型: LLM 友好的项目参考

## 一、项目定位

**CodeCoder** 是一个以观察为中心的 AI 代理系统，核心特点：

| 层次 | 能力 |
|------|------|
| 工程层 | 代码审查、安全分析、TDD、架构设计、逆向工程 |
| 领域层 | 宏观经济、交易分析、选品策略、极小产品、AI 工程 |
| 思维层 | 祝融说哲学体系、CLOSE 决策框架、观察者理论 |

### 快速状态

| 指标 | 数值 |
|------|------|
| 整体完成度 | 98%+ |
| Agent 数量 | 31 个 |
| TypeScript 覆盖率 | 74.93% |
| Rust 代码量 | ~342,000 行 |
| 最近更新 | 2026-03-08 |

## 二、技术架构

### 2.1 核心原则

> 高确定性任务 → Rust (zero-*)
> 高不确定性任务 → TypeScript/LLM (ccode)

| 任务类型 | 最佳工具 | 原因 |
|---------|---------|------|
| 协议解析、签名验证、调度 | zero-* (Rust) | 规则明确，需要高性能和安全性 |
| 意图理解、代码生成、决策建议 | ccode (LLM) | 需要推理和领域知识 |

### 2.2 Monorepo 结构

```
/
├── packages/                      # TypeScript 包
│   ├── ccode/                     # 核心 CLI 工具 (主包)
│   │   ├── src/
│   │   │   ├── agent/             # Agent 定义与 Prompt
│   │   │   │   ├── aggregator/    # 结果聚合器
│   │   │   │   └── forum/         # 多视角讨论
│   │   │   ├── api/               # HTTP API 服务器
│   │   │   ├── autonomous/        # 自主执行框架
│   │   │   │   └── confidence/    # 置信度评估 (P0)
│   │   │   ├── cli/               # CLI 命令与 TUI
│   │   │   ├── config/            # 配置管理
│   │   │   ├── document/          # 文档处理
│   │   │   │   └── ir/            # Document IR 层 (P2)
│   │   │   ├── mcp/               # MCP 协议支持
│   │   │   ├── observability/     # 可观测性
│   │   │   │   └── emit.ts        # 事件发射器 (P0)
│   │   │   ├── provider/          # AI 提供商适配器 (20+)
│   │   │   ├── security/          # 安全策略
│   │   │   ├── skill/             # 技能系统
│   │   │   │   └── preloader.ts   # 技能预加载器 (P0)
│   │   │   ├── storage/           # 数据持久化
│   │   │   ├── tool/              # 工具定义
│   │   │   │   ├── error-recovery.ts  # 错误恢复 (P0)
│   │   │   │   └── macro/         # 工具宏系统 (P2)
│   │   │   └── verifier/          # 形式化验证
│   │   └── test/                  # 测试文件
│   ├── util/                      # 共享工具库
│   └── web/                       # Web 前端 (React + Vite)
│
├── services/                      # Rust 服务 (5 Crates)
│   ├── zero-cli/                  # CLI + Daemon (统一入口 :4402)
│   │   └── src/
│   │       ├── heartbeat/         # 健康检查 (P0)
│   │       │   ├── health.rs
│   │       │   └── monitor.rs
│   │       ├── observability/     # 可观测性
│   │       │   └── emitter.rs     # 事件发射器 (P0)
│   │       └── skills/            # 技能加载器
│   │           └── loader.rs
│   ├── zero-core/                 # 核心工具库 (NAPI 绑定)
│   │   └── src/tools/
│   │       └── error.rs           # 错误处理
│   ├── zero-hub/                  # 服务中枢
│   │   └── src/
│   │       ├── gateway/           # 认证、路由、配额
│   │       │   └── hitl/          # HITL 审批 (P2)
│   │       │       ├── escalation.rs
│   │       │       └── queue.rs
│   │       ├── channels/          # IM 渠道
│   │       └── workflow/          # 调度、Webhook
│   │           └── forum/         # 多视角讨论 (P2)
│   ├── zero-trading/              # PO3+SMT 交易系统
│   └── zero-common/               # 共享配置、日志
│
├── memory/                        # 双层记忆系统
│   ├── MEMORY.md                  # 长期记忆
│   └── daily/                     # 每日笔记
│
├── docs/                          # 文档
│   ├── progress/                  # 进行中 (2 文件)
│   ├── reports/completed/         # 完成报告 (117+ 文件)
│   ├── research/                  # 对比研究 (8 文件)
│   ├── architecture/              # 架构文档
│   └── guides/                    # 使用指南
│
└── script/                        # 构建脚本
```

### 2.3 端口配置

| 服务 | 端口 | 说明 |
|------|------|------|
| CodeCoder API Server | 4400 | Bun/TypeScript |
| Web Frontend (Vite) | 4401 | React |
| Zero CLI Daemon | 4402 | Rust, 统一服务入口 |
| Faster Whisper Server | 4403 | Docker, 语音转文字 |
| Redis Server | 4410 | Docker, 会话存储 |
| MCP Server (HTTP) | 4420 | Model Context Protocol |

### 2.4 技术栈

| 类别 | 技术选型 |
|------|----------|
| 运行时 | Bun 1.3+ (TypeScript), Rust 1.75+ (Services) |
| 构建系统 | Turborepo (TS), Cargo Workspace (Rust) |
| 前端框架 | React (Web), Solid.js + OpenTUI (终端) |
| HTTP 框架 | Hono (TS), Axum (Rust) |
| AI 集成 | 20+ 提供商 SDK、MCP 协议 |
| 验证库 | Zod (TS), Serde (Rust) |
| 存储路径 | `~/.codecoder/` |

## 三、Agent 系统

### 3.1 Agent 3-Mode 设计

| 模式 | 主 Agent | 备选 | 能力 |
|------|----------|------|------|
| **@build** (默认) | build | plan, autonomous | code-reviewer, security-reviewer, tdd-guide, architect, explore, general, code-reverse, jar-code-reverse, verifier, prd-generator, feasibility-assess |
| **@writer** | writer | - | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| **@decision** | decision | observer | macro, trader, value-analyst, picker, miniproduct, ai-engineer, synton-assistant |

### 3.2 全部 31 个 Agent

| 分类 | Agent | 用途 |
|------|-------|------|
| **主模式 (4)** | build, plan, writer, autonomous | 主要开发/创作模式 |
| **逆向工程 (2)** | code-reverse, jar-code-reverse | 代码逆向分析 |
| **工程质量 (6)** | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | 代码质量保障 |
| **内容创作 (5)** | expander, expander-fiction, expander-nonfiction, proofreader, verifier | 长文写作与校对 |
| **祝融说系列 (8)** | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | 决策与领域咨询 |
| **产品运营 (2)** | prd-generator, feasibility-assess | 需求与可行性 |
| **辅助 (1)** | synton-assistant | 句元助手 |
| **系统隐藏 (3)** | compaction, title, summary | 内部使用 |

### 3.3 Agent 调用示例

```bash
# 默认 @build 模式
bun dev

# 切换模式
bun dev -m writer    # 写作模式
bun dev -m decision  # 决策模式

# 访问特定能力
bun dev @build:security-review
bun dev @decision:macro
```

## 四、最近完成的功能

### 2026-03-07 ~ 2026-03-08 架构优化

| 类别 | 模块 | 文件位置 |
|------|------|----------|
| **P0** | 技能预加载器 | `packages/ccode/src/skill/preloader.ts` |
| **P0** | 置信度机制 | `packages/ccode/src/autonomous/confidence/` |
| **P0** | 健康检查 | `services/zero-cli/src/heartbeat/health.rs` |
| **P0** | 错误恢复 | `packages/ccode/src/tool/error-recovery.ts` |
| **P0** | 事件发射器 | `emit.ts` + `emitter.rs` |
| **P2** | Document IR 层 | `packages/ccode/src/document/ir/` |
| **P2** | 工具宏系统 | `packages/ccode/src/tool/macro/` |
| **P2** | Forum 聚合器 | `packages/ccode/src/agent/forum/` |
| **P2** | HITL 升级 | `services/zero-hub/src/gateway/hitl/` |

### 2026-03-07 开源项目对比研究

| 项目 | 核心发现 |
|------|----------|
| DeerFlow | 多 Agent 研究流程、GraphRAG |
| STORM | 多视角问题生成、大纲迭代 |
| MiroThinker | 扩散链推理、自适应抽象 |
| Paperclip | Mermaid 可视化、演化优化 |
| BettaFish | 工具感知规划、动态会话 |
| MiroFish | 多维度分析、结构化报告 |
| Agent Lightning | 技能自动发现、模块化设计 |
| Goose | 会话分支、轨迹回顾 |

### 2026-03-04 ~ 2026-03-06 TypeScript to Rust 迁移

- **状态**: Phase 1-8.1 完成，项目进入维护阶段
- **累计删除**: ~3,500+ 行 TypeScript 代码
- **性能提升**: 平均 5-10x
- **迁移模块**: Storage, Security, Context, Memory, Graph, Trace, Provider Transform, Tools (18个), Shell Parser, Git Operations

## 五、项目状态

所有主要任务均已完成 (整体完成度 98%+)。详见 [REMAINING_TASKS.md](./progress/REMAINING_TASKS.md)

### P2 待办项 (未来规划)

- [ ] 会话分支系统 (参考 Goose)
- [ ] 多视角问题生成 (参考 STORM)
- [ ] GraphRAG 增强 (参考 DeerFlow)

## 六、关键文件索引

| 模块 | 入口文件 |
|------|----------|
| Agent 定义 | `packages/ccode/src/agent/agent.ts` |
| Agent Prompt | `packages/ccode/src/agent/prompt/` |
| 配置加载 | `packages/ccode/src/config/config.ts` |
| 数据存储 | `packages/ccode/src/storage/storage.ts` |
| CLI 入口 | `packages/ccode/src/index.ts` |
| TUI 代码 | `packages/ccode/src/cli/cmd/tui/` |
| Rust Daemon | `services/zero-cli/src/main.rs` |
| 服务中枢 | `services/zero-hub/src/lib.rs` |

## 七、开发命令速查

```bash
# 安装依赖
bun install

# 运行 TUI
bun dev

# 启动 API 服务器
bun dev serve --port 4400

# 类型检查
bun turbo typecheck

# 运行测试 (必须在特定包内运行)
cd packages/ccode && bun test

# 构建可执行文件
bun run --cwd packages/ccode build

# Rust 服务
./ops.sh start          # 启动服务
./ops.sh status         # 查看状态
./ops.sh build rust     # 构建
```

## 八、记忆系统

采用双层 Markdown 记忆架构：

1. **流层（每日笔记）**: `memory/daily/{YYYY-MM-DD}.md`
   - 仅追加日志，记录当日所有工作

2. **沉积层（长期记忆）**: `memory/MEMORY.md`
   - 结构化知识：用户偏好、关键决策、经验教训

**操作原则**: 人类可读、Git 友好、无复杂嵌入检索

---

*本文档用于快速理解项目全貌，适合 LLM 和新开发者参考。*
