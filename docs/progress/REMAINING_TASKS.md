# CodeCoder 剩余任务清单

更新时间: 2026-03-08

> **状态**: 所有任务均已完成，项目整体完成度 98%+

---

## 项目概览

| 指标 | 状态 |
|------|------|
| **整体完成度** | 98%+ |
| **Agent 数量** | 31 个 |
| **TypeScript 测试覆盖率** | 74.93% |
| **Rust 代码量** | 131,593 行 (services/zero-*) |
| **核心服务端口** | 4400-4439 |
| **累计删除 TS 代码** | ~3,500+ 行 |

**架构定位**: 高确定性任务用 zero-* (Rust)，高不确定性任务用 ccode (LLM)

---

## 一、TypeScript to Rust 迁移 ✅ 项目完成

> **状态**: 迁移项目于 2026-03-06 正式结束 (Phase 8.1 完成)，进入维护阶段。
>
> **最终报告**: [docs/reports/completed/2026-03-05-ts-to-rust-migration-final-assessment.md](../reports/completed/2026-03-05-ts-to-rust-migration-final-assessment.md)

### 已完成的迁移 (Phase 1-8.1)

| 模块 | Rust 实现 | 性能提升 |
|------|-----------|----------|
| Storage (KV) | `storage.rs` | ~5x |
| Security (Vault, Injection) | `security/` | ~5x |
| Context (Fingerprint, Relevance) | `context/` | ~5x |
| Memory (Vector, Chunker) | `memory/` | ~8x |
| Graph (Causal, Call, Semantic) | `graph/` | ~3x |
| Trace System | `trace/` | ~10x |
| Provider Transform | `provider/` | ~5x |
| Tool Execution (18 工具) | `tools/` | ~10x |
| **Shell Parser, Git Operations** | `shell.rs`, `git.rs` | **~8x** |

### 不再迁移的模块 (Phase 9+)

| 模块 | TypeScript 行数 | 不迁移原因 |
|------|-----------------|-----------|
| Document | 6,093 | LLM prompt 生成，计算密集度低 |
| Session | 4,995 | AI SDK 调用编排，TypeScript 生态优势 |
| Autonomous | 30,587 | LLM 编排主导，状态机已在 Rust |

### 混合架构最终形态

```
TypeScript 层 (高不确定性): Session, Autonomous, Document, Agent 协调
      │
      ▼ NAPI-RS
Rust 层 (高确定性): tools, trace, provider, security, context, memory, graph
```

---

## 二、Bug 修复 (优先级: 高) ✅ 全部验证通过

> **验证报告**: [docs/reports/completed/2026-03-06-bug-verification-phase17.md](../reports/completed/2026-03-06-bug-verification-phase17.md)

### 2.1 Autonomous Agent WebSearch 修复

- **状态**: 🟢 已验证
- **验证日期**: 2026-03-06
- **问题**: Autonomous agent 无法获取实时数据
- **修复内容**:
  - [x] 修复 web-search.ts performSearch 方法（改为调用 Exa MCP API）
  - [x] 为 autonomous agent 添加 websearch/webfetch 权限
  - [x] 修复 registry.ts 工具过滤逻辑
  - [x] 运行时日志验证通过

详见: [2026-03-02-autonomous-websearch-fix.md](../reports/completed/2026-03-02-autonomous-websearch-fix.md)

### 2.2 延迟任务渠道消息修复

- **状态**: 🟢 已验证
- **验证日期**: 2026-03-06
- **问题**: 延迟任务执行成功但消息未发送到 Telegram
- **修复内容**:
  - [x] 添加 channel_message 命令类型
  - [x] Rust 端实现 execute_channel_message_command
  - [x] TypeScript 端更新 scheduler API
  - [x] TaskContextRegistry 扩展
  - [x] Tool 上下文注入
  - [x] 自动渠道检测
  - [x] API 数据验证通过 (lastStatus: "ok")

详见: [2026-03-02-delay-task-channel-message-fix.md](../reports/completed/2026-03-02-delay-task-channel-message-fix.md)

### 2.3 Agent 任务 IM 回调机制

- **状态**: 🟢 已验证
- **验证日期**: 2026-03-06
- **问题**: Agent 定时任务执行后结果未推送回 IM
- **修复内容**:
  - [x] Rust CronCommand::Agent 新增回调字段
  - [x] execute_agent_command 支持回调
  - [x] TypeScript API Handler 更新
  - [x] 运行时日志验证通过 (has_callback=true, content_len=2546)

详见: [2026-03-03-agent-task-im-callback.md](../reports/completed/2026-03-03-agent-task-im-callback.md)

### 2.4 Question 工具 IM 显示

- **状态**: 🟢 已完成
- **日期**: 2026-03-03
- **功能**: 实现 Question 工具在 IM 渠道的交互式显示
- **实现**:
  - [x] 修改 ImProgressHandler 支持 question 事件类型
  - [x] 支持选项展示和用户交互
  - [x] 在 Telegram/Discord 等渠道正确渲染

---

## 三、技术债务 (优先级: 中)

### 3.1 AI SDK 版本升级

- **状态**: 🟢 已完成
- **完成日期**: 2026-03-01
- **提交**: `b894377` - feat: upgrade @ai-sdk/* packages to v3/v4
- **已完成**:
  - [x] 升级 `ai` 到 v6.0.105
  - [x] 升级所有 `@ai-sdk/*` 包到 v3/v4
  - [x] 修复 Breaking Changes (tool factory API)
  - [x] 全部 typecheck 通过

**最终版本**:
```json
{
  "@ai-sdk/provider": "3.0.8",
  "@ai-sdk/provider-utils": "4.0.16",
  "@ai-sdk/anthropic": "3.0.50",
  "@ai-sdk/openai": "3.0.37",
  "ai": "6.0.105"
}
```

详见: [2026-03-01-ai-sdk-migration.md](../reports/completed/2026-03-01-ai-sdk-migration.md)

### 3.2 硬编码风险审计

- **状态**: 🟢 全部完成
- **验证日期**: 2026-03-06
- **已完成**:
  - [x] Phase 1: 交易风险参数配置化 (T1RiskConfig)
  - [x] Phase 2: 服务端点配置化 (GATEWAY_URL)
  - [x] Phase 3: MEDIUM 级别项目（超时配置、限流参数、E2E 测试路径）

详见: [2026-03-02-hardcoded-risk-audit-fix.md](../reports/completed/2026-03-02-hardcoded-risk-audit-fix.md)

---

## 四、文件夹结构整改

### 4.1 清理违规目录

- [x] 删除 packages/ccode/memory/ (仅含测试日志，根目录 /memory/ 已是权威来源)
- [x] 删除 packages/ccode/docs/ (健康检查报告重复)
- [x] 清理 services/memory/hands/ 嵌套结构 (执行历史移至 /example/hands/*/executions/)

### 4.2 整合任务文档

- [x] 创建本文档 (REMAINING_TASKS.md)
- [x] 将分散的进度文档统一引用

---

## 五、状态图例

| 图标 | 含义 |
|------|------|
| 🟢 | 已完成 |
| 🟡 | 代码完成，待验证 |
| 🔄 | 进行中 |
| 🔴 | 待处理 |

---

## 六、验证命令

```bash
# 检查文件夹合规性（应无输出）
find packages services -type d \( -name "memory" -o -name "docs" -o -name "example" \) 2>/dev/null

# 验证 memory 目录
ls -la memory/
ls -la memory/daily/

# 检查 Rust 构建
cd services && cargo check -p zero-core

# 检查 TypeScript 类型
bun turbo typecheck
```
