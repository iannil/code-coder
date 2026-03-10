# 跨语言共享类型文档

本文档记录 Rust (`services/zero-*`) 和 TypeScript (`packages/ccode`) 之间共享的类型定义，确保两端类型保持同步。

## 1. 概述

### 1.1 当前状态

| 方向 | 来源 | 状态 |
|------|------|------|
| Rust → TS | ts-rs 自动生成 | ✅ 已自动化 |
| TS → Rust | 手动同步 | 🚧 需要时处理 |

### 1.2 自动化方案

使用 [ts-rs](https://github.com/Aleph-Alpha/ts-rs) 从 Rust 类型自动生成 TypeScript 类型定义。

**生成命令**:

```bash
./script/generate-ts-bindings.sh
```

**输出目录**: `packages/ccode/src/generated/`

**使用方式**:

```typescript
import {
  RiskLevel,
  TaskEvent,
  ApprovalStatus,
} from "@/generated"
```

---

## 2. 已自动化的类型

### 2.1 Guardrails 审批系统

| Rust 类型 | TypeScript 文件 | 源文件 |
|-----------|-----------------|--------|
| `RiskLevel` | `generated/guardrails/RiskLevel.ts` | `services/zero-core/src/common/guardrails.rs` |
| `ActionCategory` | `generated/guardrails/ActionCategory.ts` | `services/zero-core/src/common/guardrails.rs` |
| `Action` | `generated/guardrails/Action.ts` | `services/zero-core/src/common/guardrails.rs` |
| `Decision` | `generated/guardrails/Decision.ts` | `services/zero-core/src/common/guardrails.rs` |
| `ApprovalRequest` | `generated/guardrails/ApprovalRequest.ts` | `services/zero-core/src/common/guardrails.rs` |
| `ApprovalStatus` | `generated/guardrails/ApprovalStatus.ts` | `services/zero-core/src/common/guardrails.rs` |

### 2.2 HitL (Human-in-the-Loop) 系统

| Rust 类型 | TypeScript 文件 | 源文件 |
|-----------|-----------------|--------|
| `RiskLevel` | `generated/hitl/RiskLevel.ts` | `services/zero-core/src/common/hitl_client.rs` |
| `ApprovalType` | `generated/hitl/ApprovalType.ts` | `services/zero-core/src/common/hitl_client.rs` |
| `ApprovalStatus` | `generated/hitl/ApprovalStatus.ts` | `services/zero-core/src/common/hitl_client.rs` |
| `ApprovalRequest` | `generated/hitl/ApprovalRequest.ts` | `services/zero-core/src/common/hitl_client.rs` |
| `CreateApprovalRequest` | `generated/hitl/CreateApprovalRequest.ts` | `services/zero-core/src/common/hitl_client.rs` |
| `ApprovalResponse` | `generated/hitl/ApprovalResponse.ts` | `services/zero-core/src/common/hitl_client.rs` |

### 2.3 Task 事件系统

| Rust 类型 | TypeScript 文件 | 源文件 |
|-----------|-----------------|--------|
| `TaskEvent` | `generated/events/TaskEvent.ts` | `services/zero-core/src/common/events.rs` |
| `TaskStatus` | `generated/events/TaskStatus.ts` | `services/zero-core/src/common/events.rs` |
| `TaskState` | `generated/events/TaskState.ts` | `services/zero-core/src/common/events.rs` |
| `StreamEvent` | `generated/events/StreamEvent.ts` | `services/zero-core/src/common/events.rs` |
| `TaskCreatedData` | `generated/events/TaskCreatedData.ts` | `services/zero-core/src/common/events.rs` |
| `TaskStartedData` | `generated/events/TaskStartedData.ts` | `services/zero-core/src/common/events.rs` |
| `ThoughtData` | `generated/events/ThoughtData.ts` | `services/zero-core/src/common/events.rs` |
| `ToolUseData` | `generated/events/ToolUseData.ts` | `services/zero-core/src/common/events.rs` |
| `ProgressData` | `generated/events/ProgressData.ts` | `services/zero-core/src/common/events.rs` |
| `OutputData` | `generated/events/OutputData.ts` | `services/zero-core/src/common/events.rs` |
| `ConfirmationData` | `generated/events/ConfirmationData.ts` | `services/zero-core/src/common/events.rs` |
| `AgentSwitchData` | `generated/events/AgentSwitchData.ts` | `services/zero-core/src/common/events.rs` |
| `HeartbeatData` | `generated/events/HeartbeatData.ts` | `services/zero-core/src/common/events.rs` |
| `DebugInfoData` | `generated/events/DebugInfoData.ts` | `services/zero-core/src/common/events.rs` |
| `AgentInfoData` | `generated/events/AgentInfoData.ts` | `services/zero-core/src/common/events.rs` |
| `SkillUseData` | `generated/events/SkillUseData.ts` | `services/zero-core/src/common/events.rs` |
| `TaskCompletedData` | `generated/events/TaskCompletedData.ts` | `services/zero-core/src/common/events.rs` |
| `TaskFailedData` | `generated/events/TaskFailedData.ts` | `services/zero-core/src/common/events.rs` |
| `TaskUsage` | `generated/events/TaskUsage.ts` | `services/zero-core/src/common/events.rs` |

---

## 3. 添加新的共享类型

### 3.1 在 Rust 中添加 ts-rs 支持

```rust
// 在文件顶部添加条件导入
#[cfg(feature = "ts-bindings")]
use ts_rs::TS;

// 为类型添加 derive 宏
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-bindings", derive(TS))]
#[cfg_attr(feature = "ts-bindings", ts(export, export_to = "module_name/"))]
pub struct MyType {
    // ...
}
```

### 3.2 在导出测试中注册

编辑 `services/zero-core/tests/export_bindings.rs`:

```rust
use zero_common::my_module::MyType;

#[test]
fn export_all_bindings() {
    // ...existing exports...
    MyType::export_all().expect("Failed to export MyType");
}
```

### 3.3 更新 TypeScript 入口文件

编辑 `packages/ccode/src/generated/index.ts`:

```typescript
export * from "./module_name/MyType"
```

### 3.4 重新生成

```bash
./script/generate-ts-bindings.sh
```

---

## 4. 手动同步的类型 (未自动化)

### 4.1 Memory 系统

| Rust 类型 | TypeScript 类型 | 文件位置 |
|-----------|-----------------|----------|
| `MemoryCategory` | `MemoryCategory` | `packages/ccode/src/memory-zerobot/types.ts` |
| `MemoryEntry` | `MemoryEntry` | `packages/ccode/src/memory-zerobot/types.ts` |

这些类型目前手动同步，位于 `services/zero-bot/` 而非 `zero-core::common`。

---

## 5. 序列化约定

### 5.1 Enum 序列化

| Rust 属性 | JSON 输出 | 示例 |
|-----------|-----------|------|
| `#[serde(rename_all = "lowercase")]` | 小写 | `RiskLevel::High` → `"high"` |
| `#[serde(rename_all = "snake_case")]` | 下划线 | `FileSystem` → `"file_system"` |
| `#[serde(tag = "type")]` | 内嵌标签 | `{ "type": "progress", ... }` |

### 5.2 日期时间

- Rust: `DateTime<Utc>` (chrono)
- TypeScript: `string` (ISO 8601 格式)
- 示例: `"2026-03-03T12:34:56.789Z"`

### 5.3 可选字段

- Rust: `Option<T>` + `#[serde(skip_serializing_if = "Option::is_none")]`
- TypeScript: `field?: T`
- JSON: 字段不存在或为 `null`

---

## 6. 验证命令

```bash
# 1. 生成 TypeScript 绑定
./script/generate-ts-bindings.sh

# 2. TypeScript 类型检查
bun turbo typecheck

# 3. Rust 测试
cd services && cargo test --features ts-bindings
```

---

## 7. 相关文档

- [DEBT.md § 8.2](../DEBT.md#82-codecoder--zero--类型共享) - 技术债务追踪
- [ARCHITECTURE.md](./ARCHITECTURE.md) - 整体架构
- [CCODE_VS_ZERO.md](./CCODE_VS_ZERO.md) - ccode 与 zero-* 职责划分

---

## 更新记录

- 2026-03-03: 初始版本，记录 Guardrails、Memory、TaskEvent 共享类型
- 2026-03-03: 实现 ts-rs 自动化，更新文档反映自动化状态
