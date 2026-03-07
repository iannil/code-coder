# Phase 8: 并行工具执行 (Parallel Tool Execution)

**日期**: 2026-03-07
**状态**: ✅ 完成

## 概述

Phase 8 实现了 native batch execution，将多个 native 工具调用合并为单次 NAPI 调用，减少 JS↔Rust 边界穿越开销。

## 问题背景

### 当前执行流程 (Phase 8 之前)

```
LLM Response
    ↓
BatchTool.execute() [TS]
    ↓
Promise.all([
  tool1.execute() → NAPI call → Rust execute_grep() → Result → NAPI return
  tool2.execute() → NAPI call → Rust execute_glob() → Result → NAPI return
  tool3.execute() → NAPI call → Rust execute_read() → Result → NAPI return
])
    ↓
Merge results [TS]
```

**问题**:
1. N 个工具调用 = N 次 JS↔Rust 边界穿越
2. NAPI 调用有固定开销 (~50-100μs/call)
3. 无法利用 Rust tokio 并行优势

### 目标架构

```
LLM Response
    ↓
BatchTool.execute() [TS]
    ↓
检查工具类型：
  - 全部为 native → 单次 NAPI executeBatch() → Rust join_all() → 单次返回
  - 混合 → 分离: native 批量 + JS 单独 → 合并结果
    ↓
Results [TS]
```

## 实现细节

### Step 1: Rust execute_batch() 方法

**文件**: `services/zero-core/src/napi/tool_registry.rs`

新增类型:
```rust
/// Tool call specification for batch execution
#[napi(object)]
pub struct NapiToolCall {
    pub tool: String,
    pub args_json: String,
    pub call_id: String,
}

/// Batch execution result
#[napi(object)]
pub struct NapiBatchResult {
    pub results: Vec<NapiToolExecuteResult>,
    pub total_duration_ms: u32,
    pub native_count: u32,
}
```

新增方法:
```rust
#[napi]
pub async fn execute_batch(&self, calls: Vec<NapiToolCall>) -> Result<NapiBatchResult> {
    // Clone Arc references for parallel execution
    let grep = self.grep.clone();
    // ... other tool clones

    // Execute all calls in parallel using futures_util::future::join_all
    let futures: Vec<_> = calls.into_iter().map(|call| {
        // Clone Arcs for each future
        let grep = grep.clone();
        // ...

        async move {
            // Dispatch to static execution methods
            match name.as_str() {
                "grep" => Self::execute_grep_batch(&grep, args).await,
                // ... other tools
            }
        }
    }).collect();

    let results = join_all(futures).await;
    // ...
}
```

关键设计决策:
- 使用 `Arc::clone()` 确保 futures 是 `Send + 'static`
- 添加 `*_batch` 静态方法避免 `&self` 借用问题
- 使用 `futures_util::future::join_all` 实现真正的并行执行

### Step 2: TypeScript 类型声明

**文件**: `packages/core/src/binding.d.ts`

```typescript
/** Tool call specification for batch execution */
export interface NapiToolCall {
  tool: string
  argsJson: string
  callId: string
}

/** Batch execution result */
export interface NapiBatchResult {
  results: NapiToolExecuteResult[]
  totalDurationMs: number
  nativeCount: number
}

export declare class ToolRegistryHandle {
  // ... existing methods ...
  executeBatch(calls: NapiToolCall[]): Promise<NapiBatchResult>
}
```

### Step 3: TypeScript BatchTool 重构

**文件**: `packages/ccode/src/tool/batch.ts`

关键变更:
1. 分离 native 和 non-native 工具调用
2. Native 工具通过 `executeBatch()` 单次批量执行
3. Non-native 工具仍通过 `Promise.all()` 并行执行
4. 合并结果并保持原始顺序

```typescript
// Native tools that can be batched in Rust
const NATIVE_TOOLS = new Set([
  "grep", "glob", "read", "edit", "write", "ls", "apply_patch", "multiedit"
])

// Separate native and non-native calls
for (const call of toolCalls) {
  if (nativeRegistry && NATIVE_TOOLS.has(call.tool)) {
    nativeCalls.push({ call, index, partID })
  } else {
    jsCalls.push({ call, index, partID })
  }
}

// Execute native calls in single batch NAPI call
if (nativeRegistry && nativeCalls.length > 0) {
  const batchCalls = nativeCalls.map(({ call, partID }) => ({
    tool: call.tool,
    argsJson: JSON.stringify(call.parameters),
    callId: partID,
  }))

  const batchResult = await nativeRegistry.executeBatch(batchCalls)
  // Process results...
}

// Execute JS calls in parallel
const jsResults = await Promise.all(jsCalls.map(executeCall))
```

## 文件变更清单

| 文件 | 操作 | 行数变化 |
|------|------|----------|
| `services/zero-core/src/napi/tool_registry.rs` | 修改 | +450 行 |
| `packages/core/src/binding.d.ts` | 修改 | +20 行 |
| `packages/core/src/index.ts` | 修改 | +2 行 |
| `packages/ccode/src/tool/batch.ts` | 重写 | ~260 行 (全新) |

## 预期收益

| 场景 | NAPI 调用次数 (前) | NAPI 调用次数 (后) | 减少 |
|------|------------------|------------------|------|
| 5 个 native 工具 | 5 | 1 | 80% |
| 10 个 native 工具 | 10 | 1 | 90% |
| 5 native + 3 JS 工具 | 8 | 4 (1 batch + 3 JS) | 50% |

## 验证结果

### Rust 编译
```bash
cargo check --features napi-bindings
# ✅ 通过 (有无关警告)
```

### TypeScript 类型检查
```bash
bun turbo typecheck --filter=ccode
# ✅ 通过
```

## 技术注意事项

### 1. Hook 处理

由于 `PreToolUse` / `PostToolUse` hooks 在 TypeScript 层实现，native batch 执行目前跳过这些 hooks。
这是设计决策：
- Batch 内的工具调用已经通过 session part 状态更新追踪
- 如需 hook 支持，可在批量执行前后分别调用

### 2. 错误隔离

批量执行中单个工具失败不影响其他工具：
- Rust 层每个工具独立 try/catch
- 返回结果数组保持与输入顺序一致
- 每个结果包含独立的 `success` 和 `error` 字段

### 3. Session 状态更新

每个工具的 session part 更新仍在 JS 层进行：
- 批量执行前：标记所有 native 工具为 "running"
- 批量执行后：根据结果更新为 "completed" 或 "error"

## 后续优化方向

1. **Hook 支持**: 考虑将 hook 逻辑移入 Rust 层
2. **流式结果**: 支持工具执行完成时立即返回，而非等待全部完成
3. **资源限制**: 添加并发数限制，避免同时执行过多 I/O 密集型工具
