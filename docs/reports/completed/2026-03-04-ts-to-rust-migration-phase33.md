# TypeScript to Rust Migration: Phase 33 - Context 模块 TypeScript 集成

## 完成状态

**状态**: ✅ 已完成
**日期**: 2026-03-04T13:20:00Z

## 完成内容

### 1. 创建 Native 包装器

**文件**: `packages/ccode/src/context/relevance-native.ts` (~185 行)

提供以下 NAPI 函数的 TypeScript 包装：

| 函数 | 功能 | 返回类型 |
|------|------|----------|
| `scoreRelevanceNative(query, content)` | 内容相关性评分 | `NapiRelevanceScore \| null` |
| `scoreRelevanceWithConfigNative(query, content, config)` | 自定义配置评分 | `NapiRelevanceScore \| null` |
| `scoreFilesNative(query, files)` | 批量文件评分 | `NapiScoredFile[] \| null` |
| `contentHashNative(content)` | 内容哈希 | `string \| null` |
| `isNativeAvailable()` | 检查 native 可用性 | `boolean` |

### 2. 集成到 Relevance 模块

**修改文件**: `packages/ccode/src/context/relevance.ts`

- 添加了 `calculateRelevanceScoreAsync()` - 异步评分函数，优先使用 native
- 添加了 `calculateRelevanceScoreSync()` - 同步 TypeScript 回退
- 保持了 `calculateRelevanceScore()` 向后兼容
- 添加了 `isUsingNative()` 导出
- 添加了 `scoreRelevance()` 公共 API
- 日志中添加 `nativeAvailable` 字段

### 3. 更新模块导出

**修改文件**: `packages/ccode/src/context/index.ts`

- 添加了 `RelevanceNative` 导出

### 4. 添加测试

**创建文件**: `packages/ccode/test/unit/context/relevance.test.ts` (~95 行)

- 10 个测试用例
- 覆盖 native 和 TypeScript 回退场景

## 验证结果

### TypeScript 类型检查
```
✓ bun turbo typecheck --filter=ccode
Tasks: 1 successful, 1 total
Time: 1.658s
```

### Rust 测试
```
✓ cargo test -p zero-core context::relevance
test result: ok. 9 passed; 0 failed
```

### TypeScript 测试
```
✓ bun test test/unit/context/relevance.test.ts
10 pass, 0 fail
```

## Native 函数映射

| Rust NAPI 函数 | TypeScript 包装 |
|----------------|-----------------|
| `score_relevance()` | `scoreRelevanceNative()` |
| `score_relevance_with_config()` | `scoreRelevanceWithConfigNative()` |
| `score_files()` | `scoreFilesNative()` |
| `content_hash()` | `contentHashNative()` |

## 设计决策

### 1. 懒加载模式
- Native bindings 只在首次调用时加载
- 避免启动时的性能开销
- 加载结果缓存，后续调用无需重复导入

### 2. 优雅回退
- 所有 native 函数返回 `T | null`
- 调用方可根据返回值决定是否使用 TypeScript 回退
- 不会因 native 不可用而中断程序

### 3. 类型同步
- TypeScript 接口严格匹配 Rust NAPI 类型
- 使用 camelCase 命名风格（与 NAPI 自动转换一致）

## 性能收益

| 操作 | 预估提升 | 原因 |
|------|----------|------|
| 单次评分 | 2-4x | 无 GC 开销 |
| 批量文件评分 | 3-5x | Rust 并行处理 |
| 内容哈希 | 5-10x | xxh3 SIMD |

## 后续优化建议

1. **批量操作优化**: 在 `findRelevantFiles()` 中使用 `scoreFilesNative()` 进行批量评分
2. **去重优化**: 在 `deduplicateAndPrioritize()` 中使用 `contentHashNative()`

## 文件变更清单

**新增**:
- `packages/ccode/src/context/relevance-native.ts`
- `packages/ccode/test/unit/context/relevance.test.ts`

**修改**:
- `packages/ccode/src/context/relevance.ts`
- `packages/ccode/src/context/index.ts`

**无需修改** (已存在):
- `services/zero-core/src/napi/context.rs`
- `services/zero-core/src/context/relevance.rs`
