# Phase 1.1: Lightweight Emit API - 可观测性系统

> 日期: 2026-03-07
> 状态: 已完成

## 概述

实现了基于 Agent Lightning 启发的轻量级可观测性系统，用于零侵入的执行追踪。

## 职责划分

| 层 | 职责 | 语言 |
|---|---|---|
| Storage | 事件存储、聚合、查询 | **Rust** (确定性) |
| Metrics | 百分位数计算、成本汇总 | **Rust** (确定性) |
| Interface | 接口封装、上下文管理 | TypeScript (薄包装) |

## 创建的文件

### Rust 核心 (services/zero-core/src/observability/)

1. **mod.rs** - 模块定义和重导出
2. **event.rs** - 事件类型定义
   - `LlmCallEvent` - LLM 调用事件（提供商、模型、tokens、延迟、成本）
   - `ToolExecutionEvent` - 工具执行事件（名称、状态、耗时）
   - `AgentLifecycleEvent` - Agent 生命周期事件（启动、完成、错误、分叉）
   - `SpanEvent` - 嵌套跟踪 span
3. **metrics.rs** - 指标聚合
   - `MetricsSummary` - 综合指标摘要
   - `LlmMetrics` - LLM 指标（p50/p95/p99 延迟、缓存命中率、成功率）
   - `ToolMetrics` - 工具指标
   - `AgentMetrics` - Agent 指标
   - `MetricsAggregator` - 确定性聚合器
4. **store.rs** - SQLite 存储
   - `ObservabilityStore` - 主存储类
   - 支持 LLM 调用、工具执行、Agent 生命周期的专用表
   - 高性能索引（trace_id, session_id, timestamp）
   - 自动清理旧事件

### Rust NAPI 绑定 (services/zero-core/src/napi/)

1. **observability.rs** - NAPI 绑定
   - `ObservabilityStoreHandle` - JS 可用的存储句柄
   - `NapiLlmCallEvent`, `NapiToolExecutionEvent`, etc. - JS 对象类型
   - `NapiMetricsSummary` - 指标返回类型

### TypeScript 接口 (packages/ccode/src/observability/)

1. **tracer.ts** - 主接口
   - `Tracer` 类 - 封装原生调用
   - `emitLlmCall()`, `emitToolExecution()`, `emitAgentLifecycle()`, `emitSpan()`
   - `getMetrics()`, `getTotalCost()`, `getTotalTokens()`
   - `getGlobalTracer()` - 全局单例
2. **index.ts** - 模块导出

## 测试结果

```
running 13 tests
test observability::event::tests::test_event_type_display ... ok
test observability::event::tests::test_llm_call_event_default ... ok
test observability::event::tests::test_tool_execution_event_default ... ok
test observability::event::tests::test_event_enum_serialization ... ok
test observability::metrics::tests::test_empty_aggregation ... ok
test observability::metrics::tests::test_llm_aggregation ... ok
test observability::metrics::tests::test_tool_aggregation ... ok
test observability::metrics::tests::test_percentile_calculation ... ok
test observability::store::tests::test_emit_llm_call ... ok
test observability::store::tests::test_emit_tool_execution ... ok
test observability::store::tests::test_query_by_trace_id ... ok
test observability::store::tests::test_aggregate_metrics ... ok
test observability::store::tests::test_cost_by_model ... ok

test result: ok. 13 passed; 0 failed
```

## 使用示例

### TypeScript

```typescript
import { getGlobalTracer } from './observability'

const tracer = getGlobalTracer()

// 记录 LLM 调用
tracer.emitLlmCall({
  provider: 'anthropic',
  model: 'claude-opus-4-5',
  inputTokens: 1500,
  outputTokens: 500,
  latencyMs: 2500,
  costUsd: 0.03,
  success: true
})

// 获取过去 24 小时的指标
const metrics = tracer.getMetrics({ hours: 24 })
console.log(`总成本: $${metrics.llm.totalCostUsd.toFixed(4)}`)
console.log(`P95 延迟: ${metrics.llm.p95LatencyMs}ms`)
```

### Rust

```rust
use zero_core::observability::{ObservabilityStore, LlmCallEvent};

let store = ObservabilityStore::open("~/.codecoder/observability.db")?;

store.emit_llm_call(LlmCallEvent {
    provider: "anthropic".into(),
    model: "claude-opus-4-5".into(),
    input_tokens: 1500,
    output_tokens: 500,
    latency_ms: 2500,
    cost_usd: 0.03,
    ..Default::default()
})?;

let metrics = store.aggregate_metrics(from, to)?;
println!("Total cost: ${:.4}", metrics.llm.total_cost_usd);
```

## 后续工作

- [ ] Phase 1.2: 成本追踪与预算系统 - 基于此模块添加预算检查
- [ ] Phase 1.3: 审批流程 - 集成到 Hooks 系统
- [ ] 集成到 Provider 层自动记录 LLM 调用
- [ ] 集成到 Tool 层自动记录工具执行

## 修改的现有文件

- `services/zero-core/src/lib.rs` - 添加 observability 模块
- `services/zero-core/src/napi/mod.rs` - 添加 observability NAPI
- `services/zero-core/Cargo.toml` - 添加 hex 依赖
- `packages/core/src/index.ts` - 添加 observability 导出
- `services/zero-core/src/foundation/watcher.rs` - 修复测试编译错误（无关修改）
