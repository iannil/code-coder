# Research Loop 设计文档

**日期**: 2026-03-03
**状态**: 已批准
**作者**: Claude + User

## 背景

### 问题描述

自主任务在执行"研究/分析"类请求时提前结束，没有生成完整的分析报告。

**根本原因**: `chat.ts` 中的 `isActionableTask()` 函数只包含"实现类"关键词，导致研究任务走了简化路径，只执行了 websearch 就结束了。

**示例**: 用户请求"梳理当前的黄金走势情况"
- 预期：多源搜索 → 信息整合 → 分析 → 报告生成
- 实际：2次 websearch → 直接结束

### 用户需求

1. **智能分类**: 使用 LLM 判断任务类型
2. **自动判断输出**: 短报告内联，长报告保存文件
3. **全面学习**: 创建 Hand + 沉淀研究模式

## 解决方案

### 整体架构

```
用户消息 (IM)
     │
     ▼
┌────────────────────────────────────────────────────────────────┐
│                    Task Classifier (新)                        │
│  使用 Haiku LLM 判断任务类型                                    │
│  → implementation | research | query | other                   │
└────────────────────────────────────────────────────────────────┘
     │
     ├─── research ───▶ Research Loop (新)
     │
     ├─── implementation ───▶ Evolution Loop (现有)
     │
     └─── query/other ───▶ Agent 直接响应 (现有)
```

### 新增模块

| 模块 | 文件路径 | 职责 |
|------|----------|------|
| Task Classifier | `autonomous/classification/task-classifier.ts` | LLM 判断任务类型 |
| Research Loop | `autonomous/execution/research-loop.ts` | 研究任务专用执行循环 |
| Report Renderer | `autonomous/execution/report-renderer.ts` | 报告输出（内联/文件） |
| Research Learner | `autonomous/execution/research-learner.ts` | 沉淀研究模式、创建 Hand |

### 修改文件

| 文件 | 改动 |
|------|------|
| `api/server/handlers/chat.ts` | 集成 Task Classifier，路由到 Research Loop |
| `autonomous/events.ts` | 添加 Research 相关事件 |
| `autonomous/index.ts` | 导出新模块 |

## 详细设计

### 1. Task Classifier

```typescript
// autonomous/classification/task-classifier.ts

export type TaskType = "implementation" | "research" | "query" | "other"

export interface ClassificationResult {
  type: TaskType
  confidence: number
  reasoning: string
  researchTopic?: string
  suggestedSources?: string[]
  isPeriodic?: boolean
}
```

**两阶段分类**:

1. **快速预分类** (规则 + 关键词)
   ```typescript
   const researchKeywords = [
     "梳理", "分析", "研究", "调研", "走势", "行情",
     "汇总", "总结", "对比", "评估", "趋势", "预测"
   ]
   ```

2. **LLM 精确分类** (仅在规则不确定时触发)
   - 使用 Claude Haiku
   - 延迟 ~300ms，成本 ~$0.0001

### 2. Research Loop

```typescript
// autonomous/execution/research-loop.ts

export interface ResearchProblem {
  sessionId: string
  topic: string
  dimensions?: string[]
  timeRange?: "today" | "week" | "month" | "all"
  sourceTypes?: ("web" | "financial" | "news")[]
  maxSources?: number
}

export interface ResearchResult {
  success: boolean
  topic: string
  summary: string
  report: string
  sources: Array<{
    url: string
    title: string
    snippet: string
    credibility: "high" | "medium" | "low"
  }>
  insights: string[]
  durationMs: number
  outputPath?: string
  handCreated?: string
}
```

**执行流程**:

1. **Phase 1: 需求理解** - 解析研究主题、确定信息维度、生成搜索策略
2. **Phase 2: 多源搜索** (并行) - Web Search、Financial APIs、专业数据源
3. **Phase 3: 信息整合** - 去重、验证、来源标注、矛盾信息标记
4. **Phase 4: 结构化分析** (LLM) - 主题分析、趋势识别、关键洞察提取
5. **Phase 5: 报告生成** - 结构化报告、Report Renderer 选择输出方式
6. **Phase 6: 学习沉淀** - daily notes、MEMORY.md、Research Learner 检测周期性任务

### 3. Report Renderer

```typescript
// autonomous/execution/report-renderer.ts

export interface RenderConfig {
  maxInlineLength: number     // 默认 1000 字
  outputDir: string           // ~/.codecoder/workspace/reports/
  filenamePattern: string     // {date}-{topic}.md
}

export interface RenderResult {
  mode: "inline" | "file"
  content: string
  filePath?: string
}
```

**自动判断逻辑**:
- 报告 < 1000 字 → 内联返回
- 报告 ≥ 1000 字 → 保存文件，返回摘要 + 链接

### 4. Research Learner

```typescript
// autonomous/execution/research-learner.ts

export interface LearnedResearchPattern {
  id: string
  topic: string
  keywords: string[]
  sources: string[]
  analysisFramework: string
  frequency?: string          // daily/weekly/monthly
  confidence: number
  createdAt: string
  lastUsedAt: string
}
```

**学习内容**:
- 搜索模式（有效关键词、数据源）
- 分析框架（维度、报告结构）
- 周期性检测 → 自动建议创建 Hand

## 与现有系统的关系

### 与 Evolution Loop 的区别

| 方面 | Evolution Loop | Research Loop |
|------|----------------|---------------|
| 目标 | 解决问题、生成代码 | 收集信息、生成报告 |
| 代码执行 | 是（沙箱） | 否 |
| 测试验证 | 是（TDD） | 否 |
| 主要输出 | 代码 + 工具 | 报告 + 洞察 |
| 学习沉淀 | 工具学习 | 研究模式学习 |

### 复用组件

- `WebSearcher` - 复用现有网络搜索能力
- `Bus` + `AutonomousEvent` - 复用事件系统
- `Log` - 复用日志系统
- `CLOSE Framework` - 复用决策框架（用于评估研究价值）

## 测试计划

1. **单元测试**
   - Task Classifier 分类准确性
   - Research Loop 各 Phase 独立测试
   - Report Renderer 输出逻辑

2. **集成测试**
   - 完整流程：分类 → Research Loop → 报告
   - 与 IM 渠道集成

3. **端到端测试**
   - 用户场景：从 Telegram 发送"梳理黄金走势"
   - 验证完整报告生成

## 里程碑

1. **M1**: Task Classifier 实现
2. **M2**: Research Loop 核心流程
3. **M3**: Report Renderer + Research Learner
4. **M4**: 集成到 chat.ts + 测试

## 附录

### 研究类关键词列表

```typescript
const researchKeywords = [
  // 中文
  "梳理", "分析", "研究", "调研", "走势", "行情",
  "汇总", "总结", "对比", "评估", "趋势", "预测",
  "盘点", "回顾", "展望", "解读", "综述",
  // 英文
  "analyze", "research", "trend", "summary", "review",
  "compare", "evaluate", "forecast", "outlook"
]
```

### 报告模板

```markdown
# {topic} 分析报告

**生成时间**: {timestamp}
**数据来源**: {sourceCount} 个来源

## 摘要
{summary}

## 详细分析
{analysis}

## 关键洞察
{insights}

## 数据来源
{sources}

---
*由 CodeCoder Research Loop 自动生成*
```
