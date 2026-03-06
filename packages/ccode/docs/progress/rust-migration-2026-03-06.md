# Rust 迁移进度报告

**日期**: 2026-03-06
**策略**: 架构简化优先 + 无 Fallback (激进)

---

## 已完成阶段

### P0: 删除 Fallback ✅

移除所有 TypeScript fallback 代码，NAPI 为唯一路径。

- `file/watcher.ts` - 删除 @parcel/watcher fallback
- `hook/hook.ts` - 删除 JS regex fallback
- `storage/storage.ts` - 删除 JSON file fallback

### P1: 统一图引擎 ✅

使用 `GraphEngineHandle` 替代三个独立图实现。

- `memory/knowledge/semantic-graph.ts` - 使用 NAPI
- `memory/knowledge/call-graph.ts` - 使用 NAPI
- `memory/knowledge/causal-graph.ts` - 使用 NAPI

### P2: Session/Prompt 核心 ✅

**分析结论**: Session/Prompt 模块是"高不确定性"编排代码，应保留在 TypeScript。

**实际变更**:
- `util/token.ts` - 删除 fallback，添加 fail-fast 检查 (-9 行 fallback, +6 行检查)

```typescript
// 最终实现 - 无 fallback，失败快速
import { estimateTokens as napiEstimateTokens } from "@codecoder-ai/core"

if (typeof napiEstimateTokens !== "function") {
  throw new Error("NAPI binding 'estimateTokens' not available.")
}

const estimateTokens: (text: string) => number = napiEstimateTokens
```

---

## P3: 验证系统 + 工具整合 (分析完成)

### 3.1 Verifier 模块分析

**文件统计**:
- `verifier/index.ts` - 282 行 (协调器)
- `verifier/invariants/analyzer.ts` - 457 行 (不变量检测)
- `verifier/properties/checker.ts` - 468 行 (属性测试)
- 总计: ~3,654 行

**分析结论**: **不迁移**

理由:
1. 使用 `fast-check` 库进行属性测试 - 外部工具集成
2. 使用 Bun test runner 执行测试 - 运行时集成
3. 动态生成测试代码 - 高不确定性逻辑
4. 当前 NAPI 中无 `VerifierHandle` 实现
5. 符合"高不确定性任务用 TypeScript"原则

### 3.2 Scheduler 模块分析

**文件**: `tool/scheduler.ts` - 530 行

**分析结论**: **不迁移** (无需迁移)

理由:
1. 仅是 API 客户端，通过 `fetch()` 调用 `/api/v1/scheduler/tasks`
2. 实际调度逻辑在 `zero-workflow` Rust 服务中
3. 工具定义: scheduler_create_task, scheduler_list_tasks, scheduler_delete_task 等
4. TypeScript 是合适的 HTTP 客户端层

### 3.3 Formatter 模块分析

**文件**: `format/formatter.ts` - 358 行

**分析结论**: **不迁移**

理由:
1. 进程管理包装器，使用 `Bun.spawn` 调用外部格式化工具
2. 20+ 格式化器定义 (prettier, gofmt, rustfmt, biome 等)
3. Bun 的进程管理已足够高效
4. 迁移到 Rust 收益有限 (外部进程仍是瓶颈)

---

## P3 总结

| 模块 | 行数 | 迁移建议 | 理由 |
|------|------|----------|------|
| verifier/** | 3,654 | 不迁移 | 外部工具集成，高不确定性 |
| tool/scheduler.ts | 530 | 不迁移 | 仅 API 客户端，调度在 Rust |
| format/formatter.ts | 358 | 不迁移 | 进程管理，收益有限 |

**P3 实际代码变更**: 0 行

---

## 迁移总结

| 阶段 | 计划删除 | 实际变更 | 状态 |
|------|----------|----------|------|
| P0 | 1,237 行 | ~230 行 fallback 删除 | ✅ 完成 |
| P1 | 2,009 行 | 图模块使用 NAPI | ✅ 完成 |
| P2 | 2,793 行 | 3 行 (token.ts) | ✅ 完成 |
| P3 | 2,762 行 | 0 行 | ✅ 分析完成 |

### 关键发现

1. **高确定性 vs 高不确定性原则生效**
   - 图引擎、存储、文件监视 → 适合 Rust (高确定性)
   - Prompt 构建、验证、格式化 → 适合 TypeScript (高不确定性/外部集成)

2. **NAPI 绑定已覆盖核心路径**
   - `estimateTokens` - token 计数
   - `GraphEngineHandle` - 三种图类型
   - `KvStore` - 键值存储
   - `FileWatcher` - 文件监视
   - `matchesPattern` - 模式匹配

3. **剩余 TypeScript 代码的合理性**
   - Session/Prompt: LLM 交互编排，需要灵活性
   - Verifier: 外部测试框架集成
   - Scheduler: 仅 API 客户端层
   - Formatter: 外部进程调用

---

## 后续建议

1. **监控 NAPI 绑定稳定性** - 无 fallback 策略需要确保构建可靠
2. **考虑添加 `VerifierHandle`** - 如果验证性能成为瓶颈
3. **保持当前架构** - TypeScript 编排 + Rust 核心是合理分工
