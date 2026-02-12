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
- **决策**: 域名从 `codecoder.ai` 更新为 `code-coder.com`
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
