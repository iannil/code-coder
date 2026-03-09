# Evolution Loop Capability Priority Optimization

**完成时间**: 2026-03-02
**状态**: 已完成

## 概述

重构 Evolution Loop 的执行顺序，使其按照 **内置能力 → 已学习资源 → 外部资源** 的优先级执行，而不是直接跳到 Web Search 和代码生成。

## 问题

当开启 `/enable_autonomous` 后，Evolution Loop 的执行顺序不合理：
- 直接跳到 Web Search 和代码生成
- 没有优先使用项目已有的内置能力（Agent、Skill、Hand、Tool、Knowledge、Memory）

## 解决方案

### 新执行顺序

```
Phase 1: 内置能力 (Priority: Highest)
  1.1 Agent Discovery    - 找合适的专用 Agent
  1.2 Skill Discovery    - 找匹配的 Skill
  1.3 Hand Discovery     - 找匹配的自主代理（支持定时任务等）
  1.4 Tool Discovery     - 内置工具 + 动态工具

Phase 2: 已学习资源 (Priority: High)
  2.1 Knowledge Search   - 已沉淀的解决方案
  2.2 Memory Search      - MEMORY.md + 日常笔记

Phase 3: 外部资源 (Priority: Low - 最后手段)
  3.1 Web Search         - 文档、StackOverflow
  3.2 GitHub Scout       - 开源库搜索
  3.3 Code Generation    - LLM 生成脚本（Python/Shell/Node.js）

Phase 4: 自我改进 (Post-execution)
  4.1 Self-Reflection
  4.2 Knowledge Sedimentation
  4.3 Tool Learning
  4.4 Auto-Builder (Gap Detection)
```

## 修改文件

| 文件 | 修改内容 |
|------|---------|
| `packages/ccode/src/autonomous/execution/evolution-loop.ts` | 主要修改 |

## 代码变更详情

### 1. 新增配置选项

```typescript
export interface EvolutionConfig {
  // ... 现有配置 ...

  // Phase 1: Internal Capability Discovery
  enableAgentDiscovery: boolean       // default: true
  enableSkillDiscovery: boolean       // default: true
  enableHandDiscovery: boolean        // default: true
  enableMemorySearch: boolean         // default: true

  // Match thresholds
  agentMatchThreshold: number         // default: 0.7
  skillMatchThreshold: number         // default: 0.6
  handMatchThreshold: number          // default: 0.7

  // Early exit control
  skipExternalIfInternalMatch: boolean // default: true
}
```

### 2. 新增类型定义

- `AgentMatchResult` - Agent 匹配结果
- `SkillMatchResult` - Skill 匹配结果
- `HandMatchResult` - Hand 匹配结果（支持 cron/webhook/git 触发）
- `MemorySearchResult` - Memory 搜索结果
- `CapabilitySearchSummary` - 能力搜索统计

### 3. 新增发现方法

- `tryAgentMatch()` - 使用 Agent Registry 查找匹配的 Agent
- `trySkillMatch()` - 查找匹配的 Skill
- `tryHandMatch()` - 查找匹配的 Hand（支持定时任务场景）
- `searchMemorySystem()` - 搜索 MEMORY.md 和日常笔记
- `recordCapabilitySearch()` - 记录能力搜索统计

### 4. 增强 EvolutionResult

```typescript
export interface EvolutionResult {
  // ... 现有字段 ...

  // 新增：能力匹配信息
  matchedCapability?: {
    type: 'agent' | 'skill' | 'hand' | 'tool' | 'knowledge' | 'memory'
    identifier: string
    score: number
  }
  capabilitiesSearched?: Array<{
    type: string
    searched: boolean
    matchCount: number
    topMatchScore?: number
  }>
}
```

## 验证

- [x] TypeScript 类型检查通过 (`bun turbo typecheck --filter=ccode`)
- [x] 现有测试通过

## 使用示例

### Agent Discovery 触发

```
用户输入: "使用 macro agent 分析今天的 PMI 数据"
→ 命中 Agent Discovery → 返回 "@macro agent" 推荐
```

### Hand Discovery 触发

```
用户输入: "每天早上8点执行财经摘要"
→ 命中 Hand Discovery（cron 触发）→ 返回相关 Hand 推荐
```

### 回退到外部资源

```
用户输入: "实现一个数据处理脚本"
→ Phase 1: 内置能力未匹配
→ Phase 2: 已学习资源未匹配
→ Phase 3: Web Search → GitHub Scout → Code Generation
```

## 风险缓解

| 风险 | 缓解措施 |
|------|---------|
| Agent/Skill 误匹配 | 使用阈值控制（0.6-0.7），低于阈值不算匹配 |
| Hands 服务不可用 | try-catch 包裹，服务不可用时跳过 |
| 性能下降 | 每个发现步骤都是 O(1) 或 O(n) 小规模搜索，影响可忽略 |
