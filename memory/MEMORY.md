# CodeCoder 长期记忆

本文件存储项目的关键上下文、用户偏好和重要决策。

## 用户偏好

- **语言**: 交流与文档使用中文，代码使用英文
- **发布方式**: 优先使用 Docker 部署
- **网络隔离**: 项目配置独立网络，避免与其他项目冲突
- **文档风格**: LLM 友好，清晰可读，模式统一

## 项目上下文

### 版本信息

- **当前版本**: 0.0.1 (开发中，功能完成度 99%+)
- **发布时间**: 待定
- **Agent 数量**: 31 个
- **测试覆盖**: TypeScript 74.93% / Rust 364+ tests
- **最后更新**: 2026-03-11

### 核心架构

- **运行时**: Bun 1.3+
- **构建系统**: Turborepo monorepo
- **前端**: Solid.js + OpenTUI
- **后端**: Hono + Cloudflare Workers
- **AI**: 多提供商支持 (20+)

### 目录结构约定

- `/release` - 发布物固定存放位置
  - `/release/rust` - Rust 服务发布
  - `/release/docker` - Docker 配置
- `/memory` - 双层记忆架构
  - `/memory/daily/{YYYY-MM-DD}.md` - 每日笔记
  - `/memory/MEMORY.md` - 本文件，长期记忆

## 关键决策

### 2026-02-04: 架构简化

- **决策**: 移除 ACP (Agent Client Protocol)，采用直接 API 架构
- **理由**: 简化架构，减少维护成本
- **影响**: 合并为核心 ccode 包，移除 app/desktop/plugin/function 包

### 2026-02-04: 域名更新

- **决策**: 域名从 `code-coder.com` 更新为 `code-coder.com`
- **影响范围**: Schema URL、文档 URL、所有外部引用

### 2026-02-05: SDK 生成移除

- **决策**: 移除 `script/generate.ts` 和 `packages/sdk/`
- **理由**: SDK 已被本地类型定义替代
- **影响**: AGENTS.md 中不再包含 SDK 生成说明

### 2026-02-09: Agent 命名简化

- **决策**: 移除祝融说系列 (ZRS) agent 的 `zrs-` 前缀
- **理由**: 简化调用方式，更直观的 agent 名称
- **变更**: observer, decision, macro, trader, picker, miniproduct, ai-engineer
- **影响**: 所有测试通过，文档已同步更新

### 2026-02-12: Crazy Mode CLOSE 决策框架集成

- **决策**: 在 Crazy Agent 的工具调用中集成 CLOSE 决策评估
- **理由**: 实现可控的自主决策，每个工具调用前评估风险
- **影响**: DecisionEngine.evaluate() 在 tool/tool.ts 中触发

### 2026-02-12: Verifier Agent 实现

- **决策**: 添加 verifier agent 用于形式化验证
- **理由**: 提供数学化、属性测试和契约验证能力
- **影响**: 新增 packages/ccode/src/verifier/ 模块

### 2026-02-13: Write 工具大参数截断修复

- **问题**: Agent 生成大文件时，Write 工具 JSON 参数被截断，导致 "Unterminated string" 错误
- **原因**: 工具调用参数有大小限制，超过限制时 JSON 解析失败
- **解决**: 在 write.txt、edit.txt、multiedit.txt prompt 中添加大文件处理指导
- **策略**: 引导 agent 使用分批写入或 Bash 工具处理超大内容

### 2026-02-13: Autonomous Agent Thinking 模式修复

- **问题**: Thinking 模式与 maxOutputTokens 冲突导致截断
- **解决**: 默认禁用 thinking，改进 token 计算逻辑
- **影响**: agent.ts 配置修改，transform.ts 逻辑优化

### 2026-02-14: Writer + Expander 集成

- **决策**: Writer Agent 可以调用 Expander 子 Agent
- **理由**: 支持系统化的长篇内容创作
- **影响**: writer.txt 添加 expander 调用指令

### 2026-02-16: Storage 数据完整性增强

- **决策**: 添加原子写入、备份机制、损坏文件隔离
- **理由**: 确保 session/message/part 数据可靠性
- **影响**: storage.ts 添加 backup/restore/healthCheck 功能

### 2026-02-16: CodeCoder + ZeroBot 整合

- **决策**: 合并两个项目，实现双向集成
- **理由**: ZeroBot 需要调用 CodeCoder Agent，共享记忆
- **影响**: 新增 services/zero-bot/、memory-zerobot/、Agent HTTP API

### 2026-02-16: 存储路径迁移

- **决策**: 将存储路径从 `~/.zero-bot` 迁移到 `~/.codecoder`
- **理由**: 统一品牌标识，避免与 ZeroBot 服务混淆
- **影响**: storage.ts 路径常量更新，需要手动迁移现有数据
- **迁移命令**: `mv ~/.zero-bot/* ~/.codecoder/`

### 2026-02-17: Telegram 交互增强

- **决策**: 添加 Telegram 交互式确认和语音消息支持
- **理由**: 提升用户体验，支持语音输入
- **影响**: Telegram channel 添加 interactive confirmation、voice message support

### 2026-02-18: Playwright 集成 + 本地 Whisper STT

- **决策**: 集成 Playwright 浏览器自动化，使用本地 Faster Whisper 替代云端 STT
- **理由**: 支持网页交互任务，降低语音转文本延迟和成本
- **影响**: 新增 Playwright tools、Faster Whisper Server (端口 4403)

### 2026-02-19: MCP 集成 + SkillHub 实现

- **决策**: 实现 MCP (Model Context Protocol) 集成和 SkillHub 技能市场
- **理由**: 支持与外部 MCP 服务器交互，实现技能发现与安装
- **影响**: 新增 MCP client 支持、Skill API 端点

### 2026-02-20: CodeCoder MCP Server 实现

- **决策**: 将 CodeCoder 作为 MCP Server 暴露工具、提示和资源
- **理由**: 允许 ZeroBot 和其他 MCP 客户端使用 CodeCoder 能力
- **影响**: 新增 `ccode mcp serve` 命令，支持 stdio/HTTP 传输

### 2026-02-21: Zero-CLI 重构

- **决策**: 将 zero-bot 单体重构拆分为 zero-cli + zero-* 服务
- **理由**: 降低耦合，提高可维护性和模块化
- **影响**: 移除 zero-bot，新增 zero-cli、zero-gateway、zero-channels、zero-workflow、zero-memory、zero-tools、zero-agent、zero-common

### 2026-02-21: 智擎工作舱 (Omni-Nexus) v3 核心功能

- **决策**: 完成平台无业务感知的核心能力层
- **理由**: 实现可配置驱动的差异化场景支持
- **影响**: 资源级 RBAC、敏感数据路由、统一配置中心、Agent 注册发现、Skill 打包安装、Prompt 模板引擎、Workflow DSL 引擎

### 2026-02-21: Agent 自举飞轮系统

- **决策**: 实现 Agent 从经验中学习并固化为可复用技能的能力
- **理由**: 实现真正的自主学习和能力积累
- **影响**: 新增 bootstrap/ 模块（候选存储、自省、生成、验证、置信度、压缩、成本追踪）、/crystallize 技能

### 2026-02-22: 智擎工作舱 v4 Phase 1 交互层

- **决策**: 打通 ZeroBot → CodeCoder API 的完整交互闭环
- **理由**: 实现 Web Portal 与核心服务的完整连接
- **影响**: Chat API、Metering API、Registry API、Admin/Chat 页面 API 连接

### 2026-02-23: CodeCoder + ZeroBot 深度集成 (Phase 1-10)

- **决策**: 实现 IM 到 Agent 的完整调用链
- **理由**: 让用户通过 Telegram/飞书/钉钉 直接调用 23 个 Agent
- **影响**:
  - IM Agent 命令解析（@agent_name prompt 模式）
  - Budget API、DLP API、Token Gateway API
  - 自主求解闭环、全局上下文枢纽
  - Docker 沙箱、WASM 沙箱、Redis Event Bus
  - Call Graph 代码依赖图

### 2026-02-23: 端口重新分配

- **决策**: 调整 Rust 服务端口避免冲突
- **理由**: 统一端口规划，便于运维管理
- **新端口分配**:
  - Zero Gateway: 4404 → 4410
  - Zero Channels: 4405 → 4411
  - Zero Workflow: 4406 → 4412
  - MCP Server: 4405 → 4420

### 2026-02-24: 智能工具系统 + 因果链 (Phase 12-18)

- **决策**: 构建自主学习和因果追溯能力
- **理由**: 实现真正的 AI Agent 自我进化
- **实现内容**:
  - Phase 12: Dynamic Tool Registry - 从执行中学习工具
  - Phase 13: Sandbox-Tool Integration - 5 步进化循环
  - Phase 14: Intelligent LLM Router - RBAC + 任务分类
  - Phase 15: Scheduled Task API - Agent 定时任务
  - Phase 16: Causal Graph - 因果链图数据库
  - Phase 17: LLM-Enhanced Abstraction - 智能代码泛化
  - Phase 18: Agent Causal Integration - Agent 因果链集成
- **里程碑**: 架构覆盖率达到 100%（中枢调度层、自主保底层、全局记忆层）

### 2026-02-25: Goals 对齐开发完成 (Phase 1-6)

- **决策**: 完成智擎工作舱 6 个 Phase 的核心功能对齐
- **理由**: 实现产品愿景中的差异化能力
- **实现内容**:
  - Phase 1: 自主求解闭环 - LLMSolver + EvolutionLoop + MemoryWriter
  - Phase 2: 全局上下文枢纽 - 6 种数据源融合 (vector, knowledge, markdown, tool, sedimentation, pattern)
  - Phase 3: 中国企业 IM - 飞书/钉钉/企业微信完整集成
  - Phase 4: 产品运营 - prd-generator + feasibility-assess agents + TicketBridge
  - Phase 5: 投研量化 - Mermaid 因果链 + RiskMonitor
  - Phase 6: 管理合规 - Admin UI + Metering + Quota
- **里程碑**: 功能完成度达到 90%+，Agent 数量增至 25 个

### 2026-02-25: Phase 7 性能验证套件实现

- **决策**: 实现 NFR-04 性能验证基准测试套件
- **理由**: 验证系统满足 goals.md 中定义的性能要求
- **实现内容**:
  - packages/ccode/bench/ 性能基准测试套件
    - startup.bench.ts - 启动时间测试 (目标 ≤0.5s)
    - plan-scan.bench.ts - Plan 模式扫描测试 (目标 ≤15s/100k LOC)
    - api-latency.bench.ts - API 延迟测试 (P50/P95/P99)
  - services/zero-*/src/main.rs - Rust 服务启动计时日志
  - script/benchmark.sh - 自动化测试脚本
  - docs/reports/templates/performance-report.md - 报告模板
- **验证结果**:
  - TS Core Startup: 168ms (通过，目标 ≤500ms)
  - Plan Scan 100k LOC: ~148ms 外推 (通过，目标 ≤15s)
  - 所有 7 项测试通过

### 2026-02-25: Chrome DevTools MCP 性能基准测试

- **决策**: 添加 MCP 工具实际执行性能测试
- **理由**: 验证 chrome-devtools-mcp 在实际网页操作中的性能表现
- **测试工具数量**: 28 个
- **成功率**: 100% (38/38 调用)
- **性能分布**:
  - ⚡ 极快 (<50ms): 17 个工具 (61%)
  - 🔶 中等 (50-200ms): 11 个工具 (39%)
  - 🐢 较慢 (>200ms): 0 个工具 (0%)
- **延迟统计**:
  - 平均: 41ms
  - P50: 1ms
  - P95: 104ms
  - P99: 176ms
- **关键发现**:
  - DOM 操作极快 (0-2ms): click, hover, fill, drag
  - 数据获取中等 (~100ms): get_network, list_pages
  - MCP 冷启动 (~2.8s) 是主要瓶颈，连接后操作极快

### 2026-02-25: Goals 对齐实现验证完成

- **决策**: 执行全面的实现验证，确认计划中所有功能已完成
- **验证方法**: TypeScript typecheck + 单元测试 + Rust 编译 + 组件加载测试
- **验证结果**:
  - TypeScript: 4/4 packages passed
  - 单元测试: 150 tests passed (autonomous 91 + GitHub Scout 37 + WASM 22)
  - Rust: 8 services compiled successfully
  - Agent System: 30 agents loaded (超过文档记录的 25 个)
- **Agent 数量更正**: 实际 30 个，包含:
  - 主模式 (4): build, plan, autonomous, writer
  - 工程类 (6): general, explore, code-reviewer, security-reviewer, tdd-guide, architect
  - 逆向工程 (2): code-reverse, jar-code-reverse
  - 内容创作 (4): expander, expander-fiction, expander-nonfiction, proofreader
  - 祝融说系列 (7): observer, decision, macro, trader, picker, miniproduct, ai-engineer
  - 产品运营 (3): verifier, prd-generator, feasibility-assess
  - 辅助 (1): synton-assistant
  - 系统隐藏 (3): compaction, title, summary
- **结论**: 功能完成度确认 90%+，所有核心能力就绪

### 2026-02-26: Zero Trading 信号生成模式确立

- **决策**: 移除 Broker 实盘交易模块，定位为"信号生成器 + IM 推送"
- **理由**: 风控、合规、灵活性考量；实盘交易涉及复杂的监管要求
- **影响**: 删除 broker/futu.rs、broker/mod.rs，保留 Paper Trading 用于策略验证
- **后续**: 用户可根据信号自行在券商平台执行交易

### 2026-02-26: 数据源架构升级

- **决策**: 采用 iTick (主) + Lixin (备用) 双数据源架构
- **理由**: 替代不稳定的 Ashare/Tushare 数据源，提高数据可靠性
- **技术细节**:
  - 新增 `itick.rs` - iTick API 集成（实时行情 + 历史数据）
  - 新增 `rate_limiter.rs` - 令牌桶限流算法
  - 新增 `lixin.rs` - 备用数据源
  - 删除 `ashare.rs`、`tushare.rs`
- **配置变更**: 数据源配置移至 `secrets.external.itick` 和 `secrets.external.lixin`

### 2026-02-27: IM Agent 自动路由实现

- **决策**: 在 zero-channels bridge.rs 中实现自动路由功能
- **理由**: 根据 prompt 内容智能推荐 agent，提升用户体验
- **实现**: RecommendRequest/Response 类型 + 200ms 超时保护
- **优先级链**: metadata (@agent) > recommended (API) > default

### 2026-02-28: Hands + HITL 系统完成 (OpenFang 学习点整合)

- **决策**: 完成从 OpenFang 学习的四大特性实现
- **实现内容**:
  - Phase 1: Auto-Approve 基础实现 (风险评估、HAND.md 清单扩展)
  - Phase 2: Auto-Approve 生态扩展 (Config、Zod Schema、Agent 级别配置)
  - Phase 3: 深度增强 (环境变量、会话检查点、DOOM_LOOP 检测、审计日志)
  - Phase 4: 安全加固 (Prompt 注入扫描、Agent 签名验证、Hands 桥接、自适应风险、沙箱集成)
- **测试覆盖**: 143 个新增测试通过
- **新增模块**:
  - `packages/ccode/src/permission/auto-approve.ts`
  - `packages/ccode/src/security/prompt-injection.ts`
  - `packages/ccode/src/agent/signature.ts`
  - `packages/ccode/src/autonomous/hands/bridge.ts`
  - `packages/ccode/src/autonomous/execution/session-checkpoint.ts`
  - `packages/ccode/src/audit/audit-log.ts`
  - `packages/ccode/src/hitl/` (HITL 审批队列)

### 2026-02-28: 文档整理完成

- **决策**: 系统性整理项目文档，归档已完成工作
- **归档数量**: 14 个 progress 文档移至 completed/
- **修正问题**:
  - 时间戳错误: 2025-02-27 → 2026-02-27
  - 文件命名规范化: 为无日期文件添加日期前缀
- **结果**: docs/progress/ 目录已清空，所有已完成工作可追溯

### 2026-03-01: 技术债务 P0-P1 完结

- **决策**: 完成所有 P0-P1 技术债务修复
- **P0 完成项**:
  - 覆盖率阈值启用 (line 80%, branch 75%, function 85%)
  - 10 个原子提交 (TypeScript/Rust 代码质量改进)
  - GitHub Actions CI 工作流 (.github/workflows/test.yml)
- **P1 完成项**:
  - document.ts 拆分 (2858 → 80 行入口 + 13 模块)
  - 结构化日志替换 (19 个 console 语句)
  - @ai-sdk/* 升级至 v3/v4 (17 个包)
  - Web 包测试 (295 新增，覆盖率 43.91% → 74.93%)
  - Rust 单元测试验证 (145 测试通过)
- **延迟项**: prompt.ts, config.ts, server.ts 因重构复杂度高暂缓

### 2026-03-01: Web 包测试基础设施建立

- **决策**: 为 packages/web 建立完整测试基础设施
- **覆盖范围**: 10 个 store 模块全覆盖
- **测试模式**: vi.mock() + beforeEach 重置 + 完整状态覆盖
- **覆盖阈值**: statements 60%, branches 60%, functions 55%, lines 60%
- **影响**: 从 0% 到 74.93% 的测试覆盖率飞跃

### 2026-03-01: TypeScript 类型安全增强

- **决策**: 启用 `noUncheckedIndexedAccess` 严格模式
- **理由**: 防止数组/对象索引访问时的潜在 undefined 错误
- **影响**: 需要显式处理可能为 undefined 的数组元素访问
- **配套**: 添加非空断言注释说明安全访问场景

### 2026-03-01: AI SDK 现代化升级

- **决策**: 升级 @ai-sdk/* 至最新 v3/v4 版本
- **理由**: 保持与上游 Vercel AI SDK 同步，获取最新功能
- **影响**: 17 个提供商包更新，修复 API 重命名
- **破坏性变更**: `createProviderDefinedToolFactoryWithOutputSchema` → `createProviderToolFactoryWithOutputSchema`

### 2026-03-01: IM 事件溯源架构实现

- **决策**: 使用 Redis Streams 实现 IM 任务的事件溯源
- **理由**: 支持任务恢复、断点续传、可靠消息传递
- **实现内容**:
  - Redis Streams 客户端 (Rust + TypeScript)
  - 14 种事件类型定义
  - 任务调度器和消费者
  - 心跳超时策略
  - 断点续传支持
  - ImProgressHandler 重构
- **配置**: taskQueue 块 (backend, consumerGroup, timeouts)

### 2026-03-02: IM 默认 Autonomous Agent

- **决策**: IM 渠道默认使用 `autonomous` agent 而非 `build`
- **理由**: autonomous agent 具有更广泛的任务处理能力
- **影响**: 更好的用户体验，减少手动指定 agent

### 2026-03-02: Trace Thinking 清理

- **决策**: Trace 完成后自动清理 thinking 信息
- **理由**: 减少 IM 消息中的噪音，保持消息简洁
- **实现**: 修改 `ImProgressHandler` 的 `onTraceComplete` 方法

### 2026-03-02: Phase 1.5 内部能力生成优先

- **决策**: Evolution Loop 优先使用内部能力生成概念
- **理由**: 减少对外部服务的依赖，提高自主学习循环稳定性
- **影响**: 自主 Agent 更可靠地学习新技能

### 2026-03-03: Question 工具 IM 显示

- **决策**: 实现 Question 工具在 IM 渠道的交互式显示
- **理由**: 让用户在 IM 中可以看到并响应 Agent 提出的问题
- **实现**: 修改 `ImProgressHandler` 支持 question 事件类型

### 2026-03-03: Agent 定时任务 IM 回调

- **决策**: 定时任务执行后自动推送结果到原始 IM 渠道
- **理由**: 用户通过 IM 创建的任务，结果应该回到 IM
- **实现**:
  - Rust `CronCommand::Agent` 添加 `callback_channel_type` 和 `callback_channel_id`
  - TypeScript `TaskCommandSchema` 添加回调字段
  - 自动从上下文注入回调渠道信息

### 2026-03-04 ~ 2026-03-06: TypeScript to Rust 迁移完成

- **状态**: Phase 1-8.1 完成，项目进入维护阶段
- **累计删除**: ~3,500+ 行 TypeScript 代码
- **性能提升**: 平均 5-10x
- **最终迁移的模块**:
  - Phase 1-2: Storage (KV), Security (Vault, Injection)
  - Phase 3-4: Context (Fingerprint, Relevance), Memory (Vector, Chunker)
  - Phase 5: Graph (Causal, Call, Semantic)
  - Phase 6: Trace System
  - Phase 7: Provider Transform
  - Phase 8: Tools (18 工具)
  - Phase 8.1: Shell Parser, Git Operations
- **不再迁移的模块**: Document (6k 行), Session (5k 行), Autonomous (30k 行)
- **不迁移原因**: LLM 编排主导，TypeScript AI SDK 生态优势
- **最终报告**: docs/reports/completed/2026-03-05-ts-to-rust-migration-final-assessment.md

### 2026-03-06: Phase 8.1 Git Operations 完成

- **决策**: 完成 git-ops.ts 的 Rust 迁移
- **实现**: 删除 TypeScript fallback 代码，统一使用 Rust 实现
- **影响**: Git 操作性能提升 ~8x
- **验证**: 单元测试通过，实际 Git 操作验证通过

### 2026-03-07: 架构优化研究完成

- **决策**: 完成 8 个开源项目对比分析
- **项目**: DeerFlow, STORM, MiroThinker, Paperclip, BettaFish, MiroFish, Agent Lightning, Goose
- **识别**: 渐进式技能加载、多视角问题生成、GraphRAG、会话分支等可借鉴模式
- **影响**: 制定 P0/P1/P2 优化计划
- **位置**: docs/research/

### 2026-03-07: NAPI 类型同步与编译修复

- **决策**: 统一 TypeScript/Rust 类型定义
- **完成项**:
  - NAPI Handle 类型自动生成
  - TypeScript/Rust 编译错误修复
  - 技能加载器原生 (Rust) 实现
  - 类型安全优化
- **影响**: 跨语言调用更可靠，编译速度提升

### 2026-03-08: P0/P2 架构优化实现

- **P0 完成**:
  - 技能预加载器 (packages/ccode/src/skill/preloader.ts)
  - 置信度机制 (packages/ccode/src/autonomous/confidence/)
  - 健康检查 (services/zero-cli/src/heartbeat/health.rs)
  - 错误恢复 (packages/ccode/src/tool/error-recovery.ts)
  - 事件发射器 (emit.ts + emitter.rs)
- **P2 完成**:
  - Document IR 层 (packages/ccode/src/document/ir/)
  - 工具宏系统 (packages/ccode/src/tool/macro/)
  - Forum 聚合器 (packages/ccode/src/agent/forum/)
  - HITL 升级 (services/zero-hub/src/gateway/hitl/)
- **P2 待办**: 会话分支、多视角问题生成、GraphRAG 增强

### 2026-03-08: 架构简化 v3

- **决策**: 清理冗余代码和过期文档
- **删除**: autonomous-agent.ts (已被 forum/aggregator 替代)
- **整理**: 移动 10 个已完成进度文件到 completed/
- **影响**: 代码库更简洁，文档结构更清晰

### 2026-03-11: CodeCoder v2 架构计划审查

- **决策**: 简化原 7 周重构计划至 1 周
- **理由**: 代码分析发现大部分工作已完成
  - LLM Providers: 95% (zero-hub/gateway/provider/)
  - Observer Network: 100% (zero-cli/observer/)
  - Gear System: 100% (zero-cli/gear/)
  - Agent API: 90% (zero-cli/unified_api/)
- **实际工作**:
  - NAPI 绑定扩展 (LLM/Observer/Gear/Agent)
  - ccode-ui 包创建 (纯 UI 层)
  - 集成验证
- **影响**: 避免重复劳动，聚焦真正缺失的组件

### 2026-03-11: 文档整理与 PROJECT_STATUS.md

- **决策**: 系统性整理项目文档，创建 LLM 友好的项目状态概览
- **归档内容**:
  - 5 个已完成计划 (plans/ → reports/completed/)
  - 9 个已完成报告 (reports/ → reports/completed/)
- **删除目录**:
  - `docs/guides/` (空目录)
  - `docs/plans/` (文件已归档)
- **新建文档**:
  - `docs/PROJECT_STATUS.md` - LLM 友好的项目状态概览
  - `docs/reports/2026-03-11-code-audit-redundancy.md` - 冗余代码审计报告
- **影响**: 文档结构更清晰，LLM 更容易理解项目全貌

## 经验教训

### 代码清理

- 删除废弃代码前，先验证是否有实际使用
- 依赖审查后，记得更新 package.json 和 DEBT.md

### 文档维护

- 每次修改都要更新对应文档，带上时间戳
- 未完成的工作放在 `/docs/progress/`
- 完成的工作移到 `/docs/reports/completed/`

### 发布准备

- 发布目录必须包含所有生产环境需要的文件
- 考虑全量发布和增量发布的差异
