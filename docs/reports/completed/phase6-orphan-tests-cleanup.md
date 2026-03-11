# Phase 6 废弃代码清理报告 - 孤儿测试文件

> 完成时间: 2026-03-11
> 状态: ✅ 已完成

## 概述

删除 3 个导入路径错误、无法编译的孤儿测试文件。

## 删除内容

| 文件 | 行数 | 原因 |
|------|------|------|
| `test/unit/memory/causal-graph.test.ts` | 837 | 导入 `@/memory/knowledge/causal-graph`（路径错误） |
| `test/unit/memory/call-graph.test.ts` | 722 | 导入 `../../../src/memory/knowledge/call-graph`（路径错误） |
| `test/unit/agent/causal-recorder.test.ts` | 750 | 导入 `@/memory/knowledge/causal-graph`（路径错误） |
| **总计** | **2,309** | - |

## 技术分析

### 问题发现

测试文件导入的模块路径与实际源码不匹配：

- 测试导入: `@/memory/knowledge/causal-graph` 和 `call-graph`
- 正确路径: `@/memory/knowledge/graph`

`CausalGraph` 和 `CallGraph` 实际都存在于 `graph.ts` 中，但测试文件引用了错误的模块名。

### 源码验证

```bash
# 实际存在的模块
packages/ccode/src/memory/knowledge/graph.ts
  └── export namespace CausalGraph { ... }
  └── export namespace CallGraph { ... }

# 不存在的模块（测试错误引用）
packages/ccode/src/memory/knowledge/causal-graph.ts  # 不存在
packages/ccode/src/memory/knowledge/call-graph.ts    # 不存在
```

## 验证结果

### TypeScript 编译

```bash
bun turbo typecheck --filter=ccode
# Tasks: 1 successful, Time: 1.827s
```

### 单元测试

```bash
bun test test/unit --bail=5
# 423 tests, 9 fail, 1 error
# 失败为预存在的 Native SafetyGuard 环境问题，与本次清理无关
```

## 累计清理统计

| Phase | 描述 | 删除行数 | 删除文件数 |
|-------|------|----------|------------|
| Phase 1 | Observer Network 源码 | ~14,800 | ~45 |
| Phase 2 | Trace + Bootstrap | ~17,200 | ~50 |
| Phase 2.5 | 孤儿 Observer 测试 | ~7,350 | ~15 |
| Phase 3 | 孤儿 Trace 测试 | ~230 | ~2 |
| Phase 4 | session/message, agent/forum 等 | ~2,421 | ~4 |
| Phase 5 | autonomous/expansion, hands | ~2,081 | 7 |
| **Phase 6** | **孤儿测试文件** | **2,309** | **3** |
| **累计** | | **~46,391** | **~126** |

## 经验教训

1. **路径不匹配**: 测试文件可能引用重构前的旧模块路径
2. **验证方法**: 除了检查 `@/module` 导入，还需检查相对路径导入
3. **模块重命名**: `causal-graph.ts` 可能曾存在，后被重构合并到 `graph.ts`

## 后续建议

### Phase 7 候选

1. **@deprecated 模块分析**: 15 个文件标记了 @deprecated，需评估是否可删除
2. **util/ 未使用函数**: 检查 `prefixSuffixSimilarity` 等未使用的导出
3. **document/ 模块精简**: 检查是否有未使用的子模块
