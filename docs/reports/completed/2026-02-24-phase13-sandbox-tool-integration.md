# Phase 13: Sandbox-Tool Registry 集成 实现报告

## 概述

本阶段将 Phase 12 实现的 DynamicToolRegistry 与 Evolution Loop (自主求解循环) 集成，实现以下闭环：

```
Evolution Loop
    ↓
[1] 检索相关工具 (DynamicToolRegistry.search)
    ↓
[2] 找到工具? → 使用工具执行 → 更新统计
    ↓ 没找到
[3] 生成新代码 → Sandbox 执行
    ↓ 成功
[4] 学习为新工具 (DynamicToolRegistry.learnFromExecution)
```

## 实现日期

2026-02-24 16:30

## 实现内容

### 修改文件

| 文件路径 | 修改内容 |
|---------|---------|
| `packages/ccode/src/autonomous/execution/evolution-loop.ts` | 集成 DynamicToolRegistry |
| `packages/ccode/test/unit/autonomous/tool-integration.test.ts` | 新增集成测试 (12 tests) |

### 具体修改

#### 1. 更新 Evolution Loop 文档头

从 4 步循环更新为 5 步循环：
1. Resource Retrieval - 搜索文档
2. Tool Discovery - 检索现有工具 (新增)
3. Dynamic Code Generation - 生成代码
4. Self-Reflection & Retry - 反思重试
5. Knowledge Sedimentation + Tool Learning - 知识沉淀 + 工具学习

#### 2. 扩展配置项

```typescript
interface EvolutionConfig {
  // ... 原有配置
  /** 启用从成功执行中学习工具 */
  enableToolLearning: boolean    // 默认: true
  /** 启用在生成代码前检索现有工具 */
  enableToolDiscovery: boolean   // 默认: true
  /** 工具匹配最低分数 (0-1) */
  toolMatchThreshold: number     // 默认: 0.7
}
```

#### 3. 扩展结果类型

```typescript
interface EvolutionResult {
  // ... 原有字段
  /** 新学习的工具 ID */
  learnedToolId?: string
  /** 使用的现有工具 ID */
  usedToolId?: string
}

interface SolutionAttempt {
  // ... 原有字段
  /** 使用的工具 ID */
  toolId?: string
  /** 使用的工具名称 */
  toolName?: string
}
```

#### 4. 新增方法

**tryExistingTool**: 搜索并执行现有工具
```typescript
private async tryExistingTool(problem): Promise<{
  toolId: string
  toolName: string
  code: string
} | null>
```

**learnToolFromExecution**: 从成功执行中学习新工具
```typescript
private async learnToolFromExecution(
  problem,
  code,
  result
): Promise<DynamicTool | null>
```

**mapLanguageToToolLanguage**: 语言映射辅助函数
```typescript
private mapLanguageToToolLanguage(
  language: "python" | "nodejs" | "shell"
): "python" | "nodejs" | "bash"
```

### 质量门控

工具学习包含以下质量门控，避免学习无用代码：

| 条件 | 说明 |
|------|------|
| 非自动生成 | 跳过含 "Auto-generated verification script" 的代码 |
| 最少行数 | 至少 5 行有效代码 |
| 最大行数 | 不超过 500 行 |
| 有实际逻辑 | 必须包含 function/def/const 等关键词 |

## 测试覆盖

新增 12 个集成测试：

### Tool Discovery
- ✅ should find and use existing tool when available
- ✅ should not use tool when score is below threshold
- ✅ should skip tool discovery when disabled

### Tool Learning
- ✅ should learn tool from successful execution
- ✅ should not learn trivial code
- ✅ should not learn code without output

### Usage Statistics
- ✅ should record usage when tool is executed

### EvolutionConfig
- ✅ should have correct default config values
- ✅ should allow overriding tool config

### SolutionAttempt
- ✅ should include tool information when using existing tool

### EvolutionResult
- ✅ should have learnedToolId and usedToolId fields in result
- ✅ should include usedToolId when existing tool is used

## 测试结果

```
 54 pass (autonomous tests)
 82 pass (tool registry tests)
  0 fail
```

## 架构覆盖率更新

| 层级 | 之前 | 之后 |
|------|------|------|
| 自主保底层 | 95% | 98% |
| 全局记忆层 | 95% | 98% |

### 覆盖率提升说明

- **自主保底层**: Evolution Loop 现在可以检索和复用学习到的工具，减少重复代码生成
- **全局记忆层**: DynamicToolRegistry 与执行层集成，成功的代码自动沉淀为可复用工具

## 使用示例

### 自动学习和复用

```typescript
import { createEvolutionLoop } from "@/autonomous/execution/evolution-loop"

const loop = createEvolutionLoop({
  enableToolLearning: true,
  enableToolDiscovery: true,
  toolMatchThreshold: 0.7,
})

// 第一次执行：学习为新工具
const result1 = await loop.evolve({
  sessionId: "session-1",
  description: "Calculate sum of array",
  technology: "nodejs",
})
console.log(result1.learnedToolId) // tool_xxx

// 第二次执行：复用已有工具
const result2 = await loop.evolve({
  sessionId: "session-2",
  description: "Sum numbers in array",
  technology: "nodejs",
})
console.log(result2.usedToolId) // tool_xxx (same tool)

await loop.cleanup()
```

### 禁用工具功能

```typescript
const loop = createEvolutionLoop({
  enableToolLearning: false,  // 不学习新工具
  enableToolDiscovery: false, // 不检索现有工具
})
```

## 后续优化建议

1. **异步学习**: 工具学习可以放入后台队列，不阻塞主执行流程
2. **工具质量评分**: 根据使用频率和成功率调整工具在检索中的权重
3. **工具版本管理**: 当发现更好的实现时自动更新工具代码
4. **跨会话共享**: 支持多用户共享高质量工具

## 备注

- 工具检索不会阻塞执行：如果没找到合适工具，会继续生成新代码
- 统计更新是原子的：即使执行失败也会记录使用情况
- 质量门控可配置：通过 `ToolLearner.LearnerConfig` 调整
