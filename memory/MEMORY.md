# CodeCoder 长期记忆

本文件存储项目的关键上下文、用户偏好和重要决策。

## 用户偏好

- **语言**: 交流与文档使用中文，代码使用英文
- **发布方式**: 优先使用 Docker 部署
- **网络隔离**: 项目配置独立网络，避免与其他项目冲突
- **文档风格**: LLM 友好，清晰可读，模式统一

## 项目上下文

### 版本信息
- **当前版本**: 0.0.1 (开发中)
- **发布时间**: 待定

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
