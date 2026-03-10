# Rust 迁移进度文档 - Phase 1

> 日期: 2026-03-10
> 状态: Phase 1 完成

---

## 概述

根据 CLAUDE.md 中的"确定性 vs 不确定性"原则，对 TS/Rust 边界进行审计和微调。

## 审计结果摘要

### 已迁移 (薄包装) ✅

| 模块 | TS 文件 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 注入扫描 | `security/prompt-injection.ts` | `zero-core/src/security/injection.rs` + NAPI | ✅ 已完成 (此前) |
| 状态机 | `autonomous/state/state-machine.ts` | `zero-core/src/autonomous/state.rs` + NAPI | ✅ 已完成 (此前) |
| 任务队列 | - | `zero-core/src/autonomous/queue.rs` + NAPI | ✅ Rust 原生 |
| **远程策略** | `security/remote-policy.ts` | `zero-core/src/security/remote_policy.rs` + NAPI | ✅ **今日完成** |

### 需要迁移 🔄

| 模块 | TS 文件 | 迁移目标 | 优先级 | 状态 |
|------|---------|----------|--------|------|
| 远程策略 | `security/remote-policy.ts` | `zero-core/src/security/remote_policy.rs` | P0 | ✅ 完成 |
| 会话管理 | `session/index.ts` | 调用 Rust HTTP API | P0 | ⏸️ 暂缓 (架构差异) |
| 安全护栏 | `autonomous/safety/guardrails.ts` | `zero-cli/src/autonomous/safety/` | P1 | ⏳ 待实施 |
| 决策引擎 | `autonomous/decision/engine.ts` | 使用 Rust `close.rs` API | P1 | ⏳ 待实施 |

### 架构差异

**Session 模块:**
- TS: 使用 KV `Storage` 抽象 (键值存储，复合键 `["session", projectID, id]`)
- Rust: 使用 SQLite `SessionStore` (关系存储)
- 结论: 需要更大重构，暂缓到后续阶段

---

## Phase 1 实施详情

### 1.1 远程策略迁移 ✅ (2026-03-10 完成)

**实施时间:** 2026-03-10 19:30 - 20:15

**创建文件:**
- `services/zero-core/src/security/remote_policy.rs` (499 行)
  - `RemoteRiskLevel` 枚举 (Safe/Moderate/Dangerous)
  - `RemoteTaskContext` 结构 (source, user_id, session_id)
  - `RemotePolicy` 实现:
    - `should_require_approval()` - 远程调用审批检查
    - `risk_level()` - 风险等级评估
    - `is_dangerous()` / `is_safe()` - 操作分类
    - 用户白名单管理 (allow/revoke/get/clear)
    - `describe_approval_reason()` - 人类可读描述
  - 完整测试覆盖

- `services/zero-core/src/napi/security.rs` (新增 ~200 行)
  - `RemotePolicyHandle` NAPI 类
  - 便捷函数: `getRemoteRiskLevel`, `isRemoteDangerous`, `isRemoteSafe`, `shouldRequireRemoteApproval`

**修改文件:**
- `services/zero-core/src/security/mod.rs` - 导出新模块
- `packages/core/src/index.ts` - 导出 NAPI 绑定
- `packages/ccode/src/security/remote-policy.ts` - 改为薄包装调用 NAPI

**验证:**
```bash
# Rust 编译通过
cd services/zero-core && cargo check --features napi-bindings
# ✅ warning: `zero-core` (lib) generated 11 warnings
# Finished `dev` profile in 16.68s

# NAPI 构建
cd services/zero-core && bunx @napi-rs/cli build --platform --release --features napi-bindings
# ✅ Finished `release` profile in 1m 27s

# TypeScript 类型检查 (remote-policy 相关错误已修复)
bun turbo typecheck --force
# ✅ @codecoder-ai/core 通过
# ⚠️ ccode 有预存在的 pty/index.ts 错误 (与本次迁移无关)
```

**额外修改 (2026-03-10 22:45):**
- `packages/core/src/binding.js` - 添加 RemotePolicy 导出
- `packages/core/src/binding.d.ts` - 添加 RemotePolicy 类型声明
- `packages/core/src/security.ts` - 添加 RemotePolicy 类包装器
- 复制 `.node` 文件到 `packages/core/`

**完成状态:** ✅ Phase 1 远程策略迁移完成

### 1.2 Session 模块 (暂缓)

**暂缓原因:**
- TS 使用 KV Storage 抽象，键格式: `["session", projectID, id]`, `["message", sessionID, messageID]`, `["part", messageID, partID]`
- Rust 使用 SQLite 关系模型
- 需要设计键值到关系的映射层
- `api/client/session.ts` HTTP 客户端已存在，后续可渐进迁移

---

## 时间线

| 时间 | 任务 | 状态 |
|------|------|------|
| 2026-03-10 10:00 | 代码审计完成 | ✅ |
| 2026-03-10 10:30 | 创建进度文档 | ✅ |
| 2026-03-10 19:30 | remote-policy Rust 实现 | ✅ |
| 2026-03-10 20:00 | remote-policy NAPI 绑定 | ✅ |
| 2026-03-10 20:15 | remote-policy TS 薄包装 | ✅ |
| 待定 | Session HTTP 客户端集成 | ⏸️ 暂缓 |

---

## 关键发现

1. **已迁移模块多**: `prompt-injection.ts` 和 `state-machine.ts` 在此前迁移中已完成
2. **NAPI 覆盖率高**: `autonomous.rs` 已有 815 行绑定代码
3. **REST API 完整**: `sessions.rs` 提供 565 行完整的会话管理 API
4. **remote-policy 迁移顺利**: 纯确定性逻辑，无外部依赖

---

## 下一步 (Phase 2)

1. 迁移 `guardrails.ts` 到 Rust
2. 为 Rust `CLOSEEvaluator` 添加 NAPI 绑定
3. 替换 `decision/engine.ts` 为 NAPI 调用
