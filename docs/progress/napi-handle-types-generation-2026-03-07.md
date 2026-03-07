# NAPI Handle Types Generation

**日期**: 2026-03-07
**状态**: 完成

---

## 任务概述

为缺失的 NAPI Handle 类型生成 TypeScript 类型定义。

## 问题分析

### 原始问题

TypeScript 代码 (`packages/core/src/index.ts`) 导出了多个 Handle 类型，但这些类型在 NAPI 生成的 `index.d.ts` 中不存在。

### 根本原因

1. **NAPI CLI 未安装**: 项目没有安装 `@napi-rs/cli`
2. **Build 命令不完整**: `ops.sh build rust` 只运行 `cargo build --release`，不包含 `--features napi-bindings`
3. **缺少 package.json**: NAPI CLI 需要 `package.json` 来运行，但 `services/zero-core/` 目录没有
4. **类型冲突**: 存在 ambiguous glob re-exports 导致类型生成失败

## 解决方案

由于 NAPI CLI 类型生成存在问题，采用手动添加类型定义的方案。

### 添加的 Handle 类型

| Handle 类型 | 位置 (index.d.ts) | 功能 |
|-------------|-------------------|------|
| `IgnoreEngineHandle` | 第 3791 行 | 文件忽略引擎 |
| `ContextLoaderHandle` | 第 3885 行 | 上下文加载器 |
| `EmbeddingIndexHandle` | 第 3940 行 | 嵌入向量索引 |
| `MemorySystemHandle` | 第 4110 行 | 统一内存系统 |

### 添加的接口类型

**Ignore Engine:**
- `NapiIgnoreConfig`
- `NapiIgnoreCheckResult`

**Context Loader:**
- `NapiFileEntry`
- `NapiDirectoryStructure`
- `NapiFileIndex`
- `NapiDependencyGraph`
- `NapiScanOptions`
- `NapiScanResult`

**Embedding Index:**
- `NapiEmbeddingSearchResult`
- `NapiEmbeddingItem`
- `NapiEmbeddingIndexStats`

**Memory System:**
- `NapiHistoryStats`
- `NapiVectorStats`
- `NapiTokenizerStats`
- `NapiMemoryStats`
- `NapiHistorySnapshot`
- `NapiVectorSnapshotData`
- `NapiMemorySnapshot`
- `NapiImportOptions`
- `NapiImportResult`
- `NapiCleanupResult`
- `NapiStoredEmbedding`
- `NapiKnnResult`
- `NapiToolDefinition`
- `NapiToolMatch`
- `NapiDecisionType` (enum)
- `NapiFileEditType` (enum)

**Hash Embedding:**
- `NapiHashEmbeddingResult`

### 添加的工厂函数

```typescript
// Ignore Engine
createIgnoreEngine(): IgnoreEngineHandle
createIgnoreEngineWithConfig(config: NapiIgnoreConfig): IgnoreEngineHandle
shouldIgnorePath(path: string): boolean
getIgnoreDefaultPatterns(): Array<string>
getIgnoreDefaultFolders(): Array<string>
filterIgnoredPaths(paths: Array<string>): Array<string>
filterPathsWithPatterns(paths: Array<string>, additionalPatterns: Array<string>): Array<string>

// Context Loader
createContextLoader(root: string, options?: NapiScanOptions): ContextLoaderHandle
scanDirectory(root: string, options?: NapiScanOptions): NapiScanResult
extractDirectoryDependencies(root: string, language: NapiProjectLanguage, options?: NapiScanOptions): NapiDependencyGraph

// Embedding Index
createEmbeddingIndex(dimension: number): EmbeddingIndexHandle

// Memory System
createMemorySystem(dataDir: string, projectId: string): MemorySystemHandle

// Hash Embedding
generateHashEmbedding(text: string, dimension?: number): Array<number>
generateHashEmbeddingWithInfo(text: string): NapiHashEmbeddingResult
generateHashEmbeddingsBatch(texts: Array<string>, dimension?: number): Array<Array<number>>
generateCombinedHashEmbedding(texts: Array<string>, dimension?: number): Array<number>
generatePositionalHashEmbedding(text: string, position: number, maxPosition: number, dimension?: number): Array<number>
hashEmbeddingSimilarity(a: Array<number>, b: Array<number>): number
```

## 执行步骤

1. 安装 NAPI CLI: `bun add -g @napi-rs/cli`
2. 创建 `services/zero-core/package.json` (NAPI CLI 需要)
3. 尝试运行 NAPI 构建 (失败 - 输出空文件)
4. 恢复原始 `index.d.ts`
5. 手动添加缺失的 Handle 类型定义
6. 运行同步脚本: `bun scripts/sync-napi-types.ts`
7. 验证类型检查

## 验证结果

```bash
# 类型已添加
grep "class IgnoreEngineHandle" services/zero-core/index.d.ts  # 第 3791 行
grep "class ContextLoaderHandle" services/zero-core/index.d.ts # 第 3885 行
grep "class EmbeddingIndexHandle" services/zero-core/index.d.ts # 第 3940 行
grep "class MemorySystemHandle" services/zero-core/index.d.ts  # 第 4110 行

# 同步结果
# NAPI types:     305
# New in NAPI:    48
# Total lines:    4515

# Handle 类型无错误
tsc --noEmit 2>&1 | grep "Handle" # 无输出 = 无错误
```

## 待处理问题

`packages/core` 存在其他预存的类型错误（与本任务无关）：

1. **Prune 相关类型缺失**: `NapiPruneConfig`, `NapiPartReference`, `NapiPrunePlan` 等
2. **Skill Parser 类型缺失**: `NapiSkillMetadata`, `NapiParsedSkill` 等
3. **类型命名不一致**: `NapiVectorStatsMemory` 应为 `NapiVectorStats`

这些问题需要单独修复。

## 文件变更

- `services/zero-core/index.d.ts`: 添加 ~380 行类型定义
- `services/zero-core/package.json`: 新建 (NAPI CLI 需要)
- `packages/core/src/binding.d.ts`: 同步更新
- `docs/progress/napi-handle-types-generation-2026-03-07.md`: 本文档

## 后续建议

1. 修复 NAPI 构建流程，使其能自动生成类型
2. 解决 `mod.rs` 中的 ambiguous glob re-exports 警告
3. 修复其他预存的类型错误
