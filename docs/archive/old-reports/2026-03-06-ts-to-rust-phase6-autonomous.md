# Phase 6: Autonomous 状态管理统一

**日期**: 2026-03-06
**状态**: ✅ 完成
**执行者**: Claude Code

## 概述

将 autonomous 模块的状态管理从 TypeScript fallback 模式迁移到 fail-fast native 模式。删除所有 TS 备用实现，直接使用 Rust NAPI 绑定。

## 变更摘要

### 6.1 TaskQueue 统一 (task-queue.ts)

| 指标 | 变更前 | 变更后 | 变化 |
|------|--------|--------|------|
| 行数 | 831 | 490 | -341 (41%) |
| fallback 逻辑 | 有 | 无 | 删除 |
| 原生依赖 | 可选 | 必需 | fail-fast |

**主要变更**:
1. 删除 `loadNativeBindings()` 异步加载逻辑
2. 删除所有方法中的 `if (this.native)` fallback 分支
3. 删除 TS TaskQueue 实现 (Map, Set, priority sort 等)
4. 添加本地 NAPI 类型定义 (解决 binding.d.ts 过时问题)
5. 添加模块加载时的 fail-fast 验证

**保留内容**:
- 类型定义 (TaskPriority, TaskStatus, Task, TaskQueueConfig)
- 类型转换函数 (toNativePriority, fromNativeTask)
- 事件发布逻辑 (Bus.publish)
- 工厂函数 (createTaskQueue)

### 6.2 StateMachine 统一 (state-machine.ts)

| 指标 | 变更前 | 变更后 | 变化 |
|------|--------|--------|------|
| 行数 | 357 | 337 | -20 (6%) |
| fallback 逻辑 | 有 | 无 | 删除 |
| 原生依赖 | 可选 | 必需 | fail-fast |

**主要变更**:
1. 删除 `initNative()` 异步加载逻辑
2. 删除所有方法中的 `try/catch` fallback 分支
3. 删除 TS state history 实现
4. 添加本地 NAPI 类型定义

**保留内容**:
- 回调接口 (onStateChange, onInvalidTransition)
- 事件发布逻辑 (Bus.publish for StateChanged, InvalidTransition)
- 状态转换验证 (isValidTransition from states.ts)
- 工厂函数 (createStateMachine)

### 总计

| 模块 | 删除行数 | 百分比 |
|------|---------|--------|
| task-queue.ts | 341 | 41% |
| state-machine.ts | 20 | 6% |
| **总计** | **361** | **27%** |

## 技术说明

### 为什么 StateMachine 删除较少?

1. **回调集成**: `onStateChange` 和 `onInvalidTransition` 是业务逻辑钩子，需要保留在 TS 层
2. **事件总线**: `Bus.publish` 是 TS 基础设施，状态变化事件需要通过它传播
3. **类型定义**: 由于 binding.d.ts 过时，需要添加 ~35 行本地类型定义
4. **验证逻辑**: `isValidTransition` 验证使用 TS 的 `states.ts` 定义 (与 Rust 保持一致)

### binding.d.ts 过时问题

发现 `packages/core/src/binding.d.ts` 中的以下类型定义与实际 Rust NAPI 实现不匹配:

```typescript
// binding.d.ts 声明 (错误)
export declare function createTaskQueue(): TaskQueueHandle
export declare function createStateMachine(config: any): StateMachineHandle

// 实际 Rust 实现
pub fn create_task_queue(session_id: String, config: Option<NapiTaskQueueConfig>) -> TaskQueueHandle
pub fn create_state_machine(config: Option<NapiStateMachineConfig>) -> StateMachineHandle
```

**临时解决方案**: 在消费文件中定义本地类型并使用类型断言
**永久解决方案**: 从 Rust 代码重新生成 binding.d.ts (TODO)

### 预存在的类型错误

以下文件存在预存在的类型错误 (非 Phase 6 引入):

| 文件 | 问题 |
|------|------|
| `src/audit/audit-log.ts` | AuditLogFallback 导出缺失 |
| `src/file/ripgrep.ts` | glob/grep API 签名不匹配 |
| `src/memory/chunker.ts` | chunkText API 签名不匹配 |
| `src/provider/transform.ts` | transformMessages 返回类型不匹配 |
| `src/tool/edit.ts` | replaceWithFuzzyMatch 参数数量不匹配 |
| `src/tool/grep.ts` | grep API 签名不匹配 |
| `src/util/jar-analyzer.ts` | analyzeJar 参数数量不匹配 |
| `src/util/java-fingerprints.ts` | FingerprintEngineHandle.create 缺失 |
| `src/util/tech-fingerprints.ts` | detectWebTechnologies 参数类型不匹配 |

这些需要单独的 binding.d.ts 修复任务。

## 验证结果

```bash
# autonomous 模块类型检查通过
bun turbo typecheck --filter=ccode 2>&1 | grep -E "autonomous|state-machine|task-queue"
# (无输出 = 无错误)
```

## 文件变更

```
packages/ccode/src/autonomous/orchestration/task-queue.ts  | -341 行
packages/ccode/src/autonomous/state/state-machine.ts       | -20 行
```

## 后续任务

- [ ] Phase 6.3: Events 总线迁移 (保留，events.ts 主要是类型定义)
- [ ] 修复 binding.d.ts 与 Rust NAPI 的类型不匹配
- [ ] Phase 7: Tool 模块迁移

## Rust 实现参考

- `services/zero-core/src/autonomous/queue.rs` (827 行) - TaskQueue 完整实现
- `services/zero-core/src/autonomous/state.rs` (553 行) - StateMachine 完整实现
- `services/zero-core/src/napi/autonomous.rs` (814 行) - NAPI 绑定
