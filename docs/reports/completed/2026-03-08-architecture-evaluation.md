# 系统业务架构评估报告

> **用途**: 内部技术审视
> **评估时间**: 2026-03-08
> **评估范围**: 架构设计、代码结构、业务能力、改进方向
> **评估状态**: 已完成

## Context

本报告为内部技术审视，旨在帮助团队了解架构现状并识别改进方向。基于对架构文档、代码结构和实现细节的深入分析。

---

## 一、架构总览

### 1.1 系统定位

CodeCoder 定位为**"既有自动档、也有手动档的 AI 代理观察系统"**，核心特点：

- 融合工程能力（代码审查、安全分析、TDD）与决策智慧（宏观经济、交易分析）
- 基于"祝融说"哲学框架，强调可持续决策优于最优决策
- 双语言架构：TypeScript (智能) + Rust (安全边界)

### 1.2 技术栈

| 层级 | 技术选型 |
|------|----------|
| 运行时 | Bun 1.3+ (TS), Rust 1.75+ |
| 构建 | Turborepo + Cargo Workspace |
| 前端 | React (Web), Solid.js + OpenTUI (TUI) |
| 后端 | Hono (TS), Axum (Rust) |
| AI | Vercel AI SDK + 多提供商 |

### 1.3 核心架构图

```
╔══════════════════════════════════════════════════════════════════════════════════════╗
║                              CodeCoder 系统架构                                        ║
╠══════════════════════════════════════════════════════════════════════════════════════╣
║                                                                                       ║
║  ┌─────────────────────────────────────────────────────────────────────────────────┐ ║
║  │                              用户接入层                                          │ ║
║  │   TUI (:4400)  │  Web (:4401)  │  CLI  │  Telegram  │  Discord  │  10+ Channels │ ║
║  └─────────────────────────────────────────────────────────────────────────────────┘ ║
║                                        │                                             ║
║                                        ▼                                             ║
║  ┌─────────────────────────────────────────────────────────────────────────────────┐ ║
║  │                     核心服务层 (TypeScript/Bun)                                  │ ║
║  │                                                                                  │ ║
║  │   API Server ◄─► Agent Engine (31 Agents) ◄─► Observer Network ◄─► Memory      │ ║
║  │                                                                                  │ ║
║  └─────────────────────────────────────────────────────────────────────────────────┘ ║
║                                        │                                             ║
║                                        ▼                                             ║
║  ┌─────────────────────────────────────────────────────────────────────────────────┐ ║
║  │                     Rust 服务层 (5 Crates)                                       │ ║
║  │                                                                                  │ ║
║  │   zero-cli ──► zero-core (工具库) + zero-hub (服务中枢) + zero-trading (交易)   │ ║
║  │                     │                                                            │ ║
║  │                     └──► zero-common (共享基础库)                                │ ║
║  │                                                                                  │ ║
║  └─────────────────────────────────────────────────────────────────────────────────┘ ║
║                                                                                       ║
╚══════════════════════════════════════════════════════════════════════════════════════╝
```

---

## 二、架构评估

### 2.1 优势

#### 职责划分清晰

- **ccode (TypeScript)**: AI 智能核心，31 个 Agent，记忆系统，Observer Network
- **zero-* (Rust)**: 安全边界，外部接入，协议适配，高性能工具
- 遵循"确定性 vs 不确定性"划分原则，让每种技术用在最擅长的地方

#### Observer Network 设计先进

Observer Network 是系统的核心创新，将 CodeCoder 从执行中心系统转变为观察中心系统：

```
packages/ccode/src/observer/
├── watchers/           # 四大观察者 (CodeWatch/WorldWatch/SelfWatch/MetaWatch)
├── consensus/          # 共识引擎 (attention/patterns/anomaly/opportunity/world-model)
├── controller/         # 模式控制 (mode/thresholds/close-evaluator/escalation)
├── responders/         # 响应组件 (notifier/analyzer/executor/historian)
├── integration/        # 集成客户端 (channels/memory/agent)
├── tower/              # 观察层级抽象 (raw/pattern/meta)
├── panel/              # 控制面板
├── dial.ts             # 档位系统 (P/N/D/S/M)
└── api.ts              # 统一 API
```

**档位系统 (Gear System)** 提供了直观的自主性控制机制：

| 档位 | 名称 | Observe | Decide | Act | 说明 |
|------|------|---------|--------|-----|------|
| **P** | Park | 0% | 0% | 0% | 系统停止，无资源消耗 |
| **N** | Neutral | 50% | 0% | 0% | 仅观察记录，不干预 |
| **D** | Drive | 70% | 60% | 40% | 日常平衡自主模式 |
| **S** | Sport | 90% | 80% | 70% | 高自主性，激进模式 |
| **M** | Manual | 自定义 | 自定义 | 自定义 | 完全手动控制 |

#### 模块化程度高

Rust 服务整合为 5 个 crate，职责清晰：

| Crate | 代码量 | 职责 |
|-------|--------|------|
| `zero-cli` | ~32k 行 | CLI + Daemon (统一入口 :4402) |
| `zero-core` | ~74k 行 | 核心工具库 (grep/glob/edit, NAPI 绑定, MCP/LSP 协议) |
| `zero-hub` | ~62k 行 | gateway + channels (10+平台) + workflow |
| `zero-trading` | ~47k 行 | PO3+SMT 交易策略系统 |
| `zero-common` | ~21k 行 | 共享配置、日志、事件总线 |

#### 记忆系统简洁透明

双层架构（流 + 沉积），纯 Markdown，Git 友好：

```
memory/
├── daily/              # 每日笔记 (流) - 仅追加，按时间线记录
│   └── YYYY-MM-DD.md
└── MEMORY.md           # 长期记忆 (沉积) - 可编辑，结构化
```

拒绝向量数据库复杂性，保持对人类和 Git 的友好性。

### 2.2 潜在问题

#### 问题 1: 代码量分布不均

| Crate | 实际代码量 | 评估 |
|-------|------------|------|
| zero-core | ~74k 行 | **过于庞大**，可能违反单一职责 |
| zero-hub | ~62k 行 | 合理，但需监控增长 |
| zero-trading | ~47k 行 | 合理 |
| zero-cli | ~32k 行 | 合理 |
| zero-common | ~21k 行 | 合理 |

**建议**: 审视 `zero-core` 是否需要进一步拆分，其中 tools、session、security、graph、protocol、napi 模块可能可以独立。

#### 问题 2: 通信机制复杂度

当前存在三种通信方式：

1. HTTP API（主要）
2. Redis Event Bus（事件驱动）
3. SSE 流式响应

可能导致调试和维护复杂度增加。

**建议**: 明确文档化各场景应使用的通信方式。

#### 问题 3: Agent 数量众多

31 个 Agent 可能存在：

- 功能重叠
- 选择困难
- 维护负担

**当前 Agent 分类**:

| 分类 | 数量 | Agent 列表 |
|------|------|-----------|
| 主模式 | 4 | build, plan, writer, autonomous |
| 逆向工程 | 2 | code-reverse, jar-code-reverse |
| 工程质量 | 6 | general, explore, code-reviewer, security-reviewer, tdd-guide, architect |
| 内容创作 | 5 | expander, expander-fiction, expander-nonfiction, proofreader, verifier |
| 祝融说系列 | 8 | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst |
| 产品运营 | 2 | prd-generator, feasibility-assess |
| 辅助 | 1 | synton-assistant |
| 系统隐藏 | 3 | compaction, title, summary |

**建议**:

1. 建立 Agent 能力矩阵
2. 考虑合并相似功能的 Agent
3. 强化 Agent 自动选择机制

#### 问题 4: Observer Network 与 Agent 集成

Observer Network 架构完整，但与现有 Agent 系统的集成情况如下：

**已定义 observerCapability 的 Agent**:

| Agent | canWatch | contributeToConsensus |
|-------|----------|----------------------|
| explore | code | Yes |
| code-reviewer | self | Yes |
| security-reviewer | self | Yes |
| observer | meta | Yes (不报告给自己) |
| decision | self | Yes |
| macro | world | Yes |
| trader | world | Yes |

**测试覆盖**: 16 个测试文件覆盖 observer 模块各组件

---

## 三、业务能力分析

### 3.1 三层能力架构

| 层级 | 已实现 Agent | 成熟度 |
|------|-------------|--------|
| **工程层** | build, plan, code-reviewer, security-reviewer, tdd-guide, architect, explore | 高 |
| **领域层** | macro, trader, picker, miniproduct, ai-engineer, value-analyst | 中 |
| **思维层** | observer, decision | 中 |

### 3.2 外部渠道支持

`zero-hub/src/channels/` 支持 10+ 平台：

**IM 渠道**:

- Telegram (telegram/mod.rs, telegram/format.rs)
- Discord (discord/mod.rs, discord/format.rs)
- Slack (slack/mod.rs, slack/format.rs)
- 飞书 Feishu (feishu.rs)
- 钉钉 DingTalk (dingtalk.rs)
- 企业微信 WeCom (wecom.rs)
- Email (email.rs)
- iMessage (imessage.rs)
- WhatsApp (whatsapp.rs)
- Matrix (matrix.rs)

**语音处理**:

- TTS: ElevenLabs, OpenAI (tts/)
- STT: OpenAI, Compatible (stt/)

**其他**:

- SSE 流式 (sse.rs)
- CLI 调试 (cli.rs)
- Debug (debug.rs)

### 3.3 工作流能力

`zero-hub/src/workflow/` 包含：

| 模块 | 说明 |
|------|------|
| Scheduler | 定时调度 (Cron) |
| Hands | 自主执行系统，6 级自治 (Lunatic → Timid) |
| GitHub/GitLab | Git 集成 |
| Webhook | HTTP 回调处理 |
| CLOSE | 五维决策框架评估 |

### 3.4 Hands 自治级别

| 级别 | 描述 | 自动批准范围 |
|------|------|-------------|
| Lunatic | 完全自主，无需审批 | 所有操作 |
| Insane | 高度自主，仅关键操作审批 | Safe + Low + Medium |
| Crazy | 自主运行，大部分操作自动批准 | Safe + Low |
| Wild | 中等自主，部分操作需审批 | Safe |
| Bold | 保守自主，多数操作需审批 | 极少 Safe |
| Timid | 最谨慎，几乎所有操作需审批 | 几乎无 |

---

## 四、Observer Network 深度分析

### 4.1 四大观察者节点

| 观察者 | 职责 | 关联 Agent | 实现文件 |
|--------|------|-----------|----------|
| **CodeWatch** | 代码库扫描、Git 变更、构建状态 | explore | watchers/code-watch.ts |
| **WorldWatch** | 市场数据、新闻舆情、API 变化 | macro, trader | watchers/world-watch.ts |
| **SelfWatch** | Agent 行为、决策日志、错误模式 | code-reviewer, security-reviewer, decision | watchers/self-watch.ts |
| **MetaWatch** | 观察质量、系统健康、观察盲点 | observer | watchers/meta-watch.ts |

### 4.2 共识层组件

| 组件 | 职责 | 实现文件 |
|------|------|----------|
| AttentionCalculator | 计算观察权重 | consensus/attention.ts |
| PatternDetector | 模式检测 | consensus/patterns.ts |
| AnomalyDetector | 异常检测 | consensus/anomaly.ts |
| OpportunityIdentifier | 机会识别 | consensus/opportunity.ts |
| WorldModelBuilder | 世界模型构建 | consensus/world-model.ts |
| ConsensusEngine | 统一引擎 | consensus/engine.ts |

### 4.3 控制器层

| 组件 | 职责 | 实现文件 |
|------|------|----------|
| ThresholdManager | 阈值管理 | controller/thresholds.ts |
| CLOSEEvaluator | CLOSE 五维评估 | controller/close-evaluator.ts |
| EscalationManager | 升级管理 | controller/escalation.ts |
| ModeController | 模式控制 | controller/mode.ts |

### 4.4 CLOSE 五维评估框架

| 维度 | 英文 | 含义 |
|------|------|------|
| **C** | Convergence | 收敛度 - 观察到确定性的转化程度 |
| **L** | Leverage | 杠杆 - 投入产出比 |
| **O** | Optionality | 可选性 - 未来选择权的保留程度 |
| **S** | Surplus | 余量 - 可用资源的富余程度 |
| **E** | Evolution | 演化 - 系统的进化潜力 |

---

## 五、改进建议

### 5.1 短期 (1-2 周)

1. **文档补充**
   - [ ] 创建 Agent 能力矩阵文档
   - [ ] 补充通信方式选择指南
   - [ ] 更新 Observer Network 使用说明

2. **代码审计**
   - [ ] 审计 `zero-core` 模块职责
   - [ ] 检查 31 个 Agent 的功能重叠度

3. **测试修复**
   - [ ] 修复 observer 模块测试中的初始化问题
   - [ ] 确保测试覆盖率达到 80%+

### 5.2 中期 (1-2 月)

1. **Observer Network 深度集成**
   - [ ] 激活四大观察者与 Agent 的实时联动
   - [ ] 实现观察事件到 Agent 的自动路由
   - [ ] 添加观察质量指标仪表盘

2. **Agent 系统优化**
   - [ ] 评估合并相似 Agent 的可行性
   - [ ] 强化 Agent 自动推荐算法
   - [ ] 添加 Agent 使用统计和分析

### 5.3 长期 (3-6 月)

1. **架构演进**
   - [ ] 评估 `zero-core` 拆分方案
   - [ ] 优化 Rust ↔ TypeScript 通信效率
   - [ ] 探索 WASM 在更多场景的应用

2. **可观测性增强**
   - [ ] 全链路追踪集成
   - [ ] Agent 决策日志可视化
   - [ ] Observer Network 仪表盘

---

## 六、总体评价

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计 | 8.5/10 | 清晰的职责划分，创新的 Observer Network |
| 代码质量 | 8/10 | 类型严格，模块化好，但 zero-core 过大 |
| 可扩展性 | 9/10 | 良好的插件架构，Agent 系统灵活 |
| 可维护性 | 7.5/10 | 文档需补充，Agent 数量需控制 |
| 创新性 | 9/10 | 档位系统、CLOSE 框架、祝融说哲学融合独特 |

**综合评分: 8.4/10**

---

## 七、关键文件索引

### 架构文档

- `docs/architecture/ARCHITECTURE.md` - 系统架构概览
- `docs/architecture/DESIGN_PHILOSOPHY.md` - 设计哲学
- `docs/architecture/CCODE_VS_ZERO.md` - ccode vs zero-* 职责划分

### Agent 系统

- `packages/ccode/src/agent/agent.ts` - Agent 定义 (31 个)
- `packages/ccode/src/agent/prompt/*.txt` - Prompt 文件

### Observer Network

- `packages/ccode/src/observer/index.ts` - 模块入口
- `packages/ccode/src/observer/dial.ts` - 档位系统
- `packages/ccode/src/observer/watchers/` - 四大观察者
- `packages/ccode/src/observer/consensus/` - 共识引擎
- `packages/ccode/src/observer/controller/` - 模式控制器
- `packages/ccode/src/observer/responders/` - 响应组件

### Rust 服务

- `services/zero-cli/src/daemon/mod.rs` - Daemon 入口
- `services/zero-hub/src/channels/` - IM 渠道 (10+)
- `services/zero-hub/src/workflow/hands/` - Hands 自主执行
- `services/zero-trading/` - 交易系统

---

## 八、验证方式

### 8.1 服务验证

```bash
# 启动 TUI，验证 Agent 系统
bun dev

# 启动完整服务栈
./ops.sh start all

# 健康检查
./ops.sh health

# 查看服务状态
./ops.sh status
```

### 8.2 Agent 验证

```bash
# 在 TUI 中查看 Agent 列表
@help

# 查看特定 Agent
@build --info
@observer --info
```

### 8.3 Observer Network 验证

```bash
# 运行 observer 测试
cd packages/ccode && bun test test/observer/

# 查看测试文件
ls -la packages/ccode/test/observer/
```

### 8.4 Rust 服务验证

```bash
# 构建 Rust 服务
./ops.sh build rust

# 启动 daemon
./ops.sh start zero-daemon

# 查看日志
./ops.sh logs zero-daemon
```

---

## 附录: 数据来源

- Agent 数量: 通过分析 `packages/ccode/src/agent/agent.ts` 中的 `result` 对象
- Rust 代码量: 通过 `find services/zero-* -name "*.rs" | xargs wc -l` 统计
- Observer 测试: 通过 `ls packages/ccode/test/observer/**/*.test.ts` 统计
- IM 渠道: 通过 `ls services/zero-hub/src/channels/` 统计

---

**评估完成时间**: 2026-03-08
**评估人**: AI 架构评估代理
