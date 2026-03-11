# CodeCoder 剩余任务清单

更新时间: 2026-03-11

> **状态**: 所有任务均已完成，项目整体完成度 99%+
>
> **最新完成**: 文档整理与归档 — 2026-03-11
> - 归档 5 个已完成计划 (plans/ → reports/completed/)
> - 归档 9 个已完成报告 (reports/ → reports/completed/)
> - 删除空目录 (docs/guides/, docs/plans/)
> - 创建 LLM 友好的项目状态文档 (PROJECT_STATUS.md)
>
> **之前完成**: 废弃代码清理 Phase 3 — 2026-03-11
> - 删除 test/unit/trace/ 目录 (孤儿测试, 230 行)
> - Phase 3 总计删除: 230 行
> - 累计删除 (Phase 1 + 2 + 2.5 + 3): ~39,580 行
>
> **之前完成**: 废弃代码清理 Phase 1 — 2026-03-11
> - 删除 14,831 行已迁移到 Rust 的 Observer 实现代码
> - Observer 目录从 ~17,000 行精简到 2,158 行 (-87%)
> - 将 TUI 需要的类型迁移到 SDK types.ts
> - 详见每日记忆: memory/daily/2026-03-11.md
>
> **之前完成**: Session API 扩展字段完成 — 2026-03-11
> - 添加 summary, permission, revert 三个扩展字段到 Rust session API
> - 实现与 TypeScript Session.Info 的完全兼容
> - SDK 适配器已更新支持新字段
>
> **架构迁移**: Rust-First 架构迁移 (Phase 2-8) — 2026-03-10
> - Observer Network 迁移到 Rust
> - Gear System (P/N/D/S/M + CLOSE) 迁移到 Rust
> - Agent 定义 (29 个) 迁移到 Rust
> - 四大 Watcher 迁移到 Rust
> - 详见: [完成报告](../reports/completed/RUST_FIRST_ARCHITECTURE_MIGRATION_20260310.md)

---

## 项目概览

| 指标 | 状态 |
|------|------|
| **整体完成度** | 99%+ |
| **Agent 数量** | 31 个 |
| **TypeScript 测试覆盖率** | 74.93% |
| **Rust 代码量** | 131,593 行 (services/zero-*) |
| **核心服务端口** | 4400-4439 |
| **累计删除 TS 代码** | ~39,580 行 (Phase 1: 14.8k, Phase 2: 17.2k, Phase 2.5: 7.3k, Phase 3: 0.2k) |

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

## 1.5 TUI SDK 迁移 🟢 完成

> **状态**: SDK 适配器完成，worker.ts 全部 session 方法迁移完成，功能测试通过，扩展字段已添加
> **更新日期**: 2026-03-11

### 已完成

| 组件 | 状态 | 说明 |
|------|------|------|
| SDK types 扩展 | ✅ | +400 行类型定义, SessionTime 兼容 |
| SDK client 扩展 | ✅ | +80 行新方法 (fork, compact, config) |
| Rust Session API | ✅ | 添加 time 对象, parent_id, directory |
| **Rust 扩展字段** | ✅ | **summary, permission, revert 完整支持** |
| SDK 适配器 | ✅ | adapter.ts - 转换 SDK 类型到 Session.Info |
| worker.ts session.list | ✅ | 支持 SDK 模式 (CODECODER_SDK_MODE=1) |
| worker.ts session.get | ✅ | 支持 SDK 模式 + adaptSessionInfo |
| worker.ts session.create | ✅ | 支持 SDK 模式 + adaptSessionInfo |
| worker.ts session.fork | ✅ | 支持 SDK 模式 + adaptSessionInfo |
| worker.ts session.remove/delete | ✅ | 支持 SDK 模式 |
| worker.ts session.compact | ✅ | 支持 SDK 模式 |
| app.tsx | ✅ | 使用 SDK parseModel, isDefaultTitle |
| context/local.tsx | ✅ | 使用 SDK parseModel |
| **功能测试** | ✅ | API 端点全部验证通过 |

### Session.Info 完整兼容性

| 字段 | 状态 | 说明 |
|------|------|------|
| `id` | ✅ | 会话 ID |
| `slug` | ✅ | 适配器生成 (前 8 字符) |
| `projectID` | ✅ | 项目 ID |
| `directory` | ✅ | 会话目录 |
| `parentID` | ✅ | 父会话 ID |
| `title` | ✅ | 会话标题 |
| `version` | ✅ | 适配器生成 ("v1.0.0") |
| `time` | ✅ | 时间对象 (毫秒) |
| `summary` | ✅ | 文件变更摘要 |
| `permission` | ✅ | 权限规则 |
| `revert` | ✅ | 回滚信息 |

### 功能测试结果 (Step 6)

| 测试项 | 结果 |
|--------|------|
| Health check | ✅ |
| Create session | ✅ |
| Get session | ✅ |
| Adapter conversion | ✅ |
| List sessions | ✅ |
| Compact session | ✅ |
| Fork session | ✅ |
| Delete session | ✅ |

### 待完成 (可选优化)

| 组件 | 状态 | 说明 |
|------|------|------|
| TUI 完整 E2E 测试 | 🔴 | 在 SDK 模式下运行完整 TUI |

### 保留原导入的组件

以下文件因深度集成需保留原导入：

| 文件 | 原因 |
|------|------|
| app.tsx | SessionApi.Event.* Bus 事件订阅 |
| dialog-session-list.tsx | Bus 事件订阅 |
| autonomous-status.tsx | 字段名不同 |
| routes/session/*.tsx | MessageV2 复杂类型 |

### 使用方式

```bash
# 默认模式 (SDK, 使用 Rust daemon)
bun dev

# 禁用 SDK 模式 (回退到 TypeScript API)
CODECODER_SDK_MODE=0 bun dev
```

---

## 1.6 废弃代码清理 🟢 完成

> **状态**: Observer Network + Trace + Bootstrap 废弃代码清理完成
> **更新日期**: 2026-03-11

### Phase 1: Observer Network (2026-03-11)

| 目录 | 文件数 | 行数 | 状态 |
|------|--------|------|------|
| `observer/watchers/` | 6 | ~2,500 | ✅ 已删除 |
| `observer/consensus/` | 7 | ~2,800 | ✅ 已删除 |
| `observer/controller/` | 5 | ~2,000 | ✅ 已删除 |
| `observer/tower/` | 4 | ~1,500 | ✅ 已删除 |
| `observer/panel/` | 2 | ~500 | ✅ 已删除 |
| `observer/responders/` | 5 | ~2,500 | ✅ 已删除 |
| `observer/integration/` | 5 | ~1,500 | ✅ 已删除 |
| `observer/event-stream.ts` | 1 | ~300 | ✅ 已删除 |
| `observer/api.ts` | 1 | ~200 | ✅ 已删除 |
| **Phase 1 总计** | **36** | **~14,800** | **✅** |

### Phase 2: Trace + Bootstrap (2026-03-11)

| 目录 | 文件数 | 行数 | 状态 |
|------|--------|------|------|
| `src/trace/` | 7 | ~1,500 | ✅ 已删除 |
| `src/bootstrap/` | 12 | ~3,300 | ✅ 已删除 |
| `cli/cmd/trace.ts` | 1 | ~600 | ✅ 已删除 |
| `test/bootstrap/` | 3 | ~300 | ✅ 已删除 |
| `test/evaluation/` | 26 | ~11,500 | ✅ 已删除 |
| **Phase 2 总计** | **49** | **~17,200** | **✅** |

### Phase 2.5: 孤儿测试清理 (2026-03-11)

| 目录 | 文件数 | 行数 | 状态 |
|------|--------|------|------|
| `test/observer/watchers/` | 4 | ~1,200 | ✅ 已删除 |
| `test/observer/consensus/` | 1 | ~600 | ✅ 已删除 |
| `test/observer/controller/` | 3 | ~1,500 | ✅ 已删除 |
| `test/observer/responders/` | 4 | ~1,800 | ✅ 已删除 |
| `test/observer/integration/` | 3 | ~900 | ✅ 已删除 |
| `test/observer/e2e/` | 2 | ~800 | ✅ 已删除 |
| `test/observer/fixtures/` | 1 | ~300 | ✅ 已删除 |
| `test/observer/helpers/` | 1 | ~100 | ✅ 已删除 |
| `test/observer/setup.ts` | 1 | ~150 | ✅ 已删除 |
| **Phase 2.5 总计** | **20** | **~7,350** | **✅** |

### Phase 3: 孤儿 Trace 测试清理 (2026-03-11)

| 目录 | 文件数 | 行数 | 状态 |
|------|--------|------|------|
| `test/unit/trace/` | 1 | ~230 | ✅ 已删除 |
| **Phase 3 总计** | **1** | **~230** | **✅** |

**清理原因**: 测试文件导入了已删除的 `@/trace/native` 模块

**Rust 替代**:
- `trace/` → `services/zero-core/src/trace/`
- `bootstrap/` → `services/zero-cli/src/skills/loader.rs`

### 类型迁移

将 TUI 需要的类型迁移到 `sdk/types.ts`:

| 类型 | 来源 | 目标 |
|------|------|------|
| `WatcherType`, `WatcherStatus` | observer/types | sdk/types.ts |
| `Observation` | observer/types | sdk/types.ts |
| `CLOSEEvaluation`, `CLOSEDimension` | observer/controller | sdk/types.ts |
| `Escalation`, `HumanDecision` | observer/controller | sdk/types.ts |
| `ConsensusSnapshot` | observer/consensus | sdk/types.ts |
| `ModeDecision`, `ModeControllerStats` | observer/controller | sdk/types.ts |
| `GEAR_INFO`, `GEAR_PRESETS` | observer/dial | sdk/types.ts |

### Observer 目录现状

```
packages/ccode/src/observer/
├── agent-registry.ts   (120 行) - 观察者 Agent 注册
├── client.ts           (374 行) - Rust API 客户端
├── dial.ts             (424 行) - 档位控制类
├── events.ts           (475 行) - 事件定义
├── index.ts            (147 行) - 导出入口
└── types.ts            (618 行) - 类型定义
                        ─────────
                        2,158 行 (原 ~17,000 行, -87%)
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
