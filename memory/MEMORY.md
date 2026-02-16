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
