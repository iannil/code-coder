# Task Routing for Autonomous Mode

**Date:** 2026-03-09
**Status:** Approved
**Author:** Claude Code

## Overview

Add task classification and routing to autonomous mode so that research and decision tasks are handled appropriately instead of defaulting to TDD code execution.

## Problem Statement

Currently, the autonomous orchestrator treats all requests as code implementation tasks, routing them through the TDD cycle. Research questions (e.g., "分析美以和伊朗战争发展") incorrectly trigger test generation instead of the research loop.

## Solution

Add LLM-based task classification with explicit `--mode` override, routing to three execution paths:

- **code** → TDD cycle (existing behavior)
- **research** → 6-phase research loop
- **decision** → Research + CLOSE evaluation

## Design

### Task Types

```typescript
export type TaskMode = "code" | "research" | "decision" | "auto"

export interface TaskClassification {
  mode: "code" | "research" | "decision"
  confidence: number
  reasoning: string
}
```

### Classification Criteria

| Mode | Trigger Patterns |
|------|------------------|
| code | 实现, 修复, 添加, 创建, implement, fix, add, create, build |
| research | 分析, 研究, 调研, 趋势, analyze, research, investigate, market |
| decision | 选择, 决定, 评估, 比较, choose, decide, evaluate, compare, CLOSE |

### CLI Interface

```bash
# Auto-detect (default)
ccode autonomous "分析美以和伊朗战争发展"

# Explicit mode
ccode autonomous --mode research "分析市场趋势"
ccode autonomous --mode code "实现用户认证"
ccode autonomous --mode decision "评估这个职业选择"
```

### Orchestrator Changes

```typescript
// In OrchestratorConfig
export interface OrchestratorConfig {
  mode?: TaskMode  // NEW
  autonomyLevel: "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
  resourceBudget: ResourceBudget
  executionConfig?: ExecutionConfig
  unattended: boolean
  enableEvolutionLoop?: boolean
}

// In Orchestrator class
async process(request: string): Promise<ProcessResult> {
  const mode = await this.resolveTaskMode(request)

  switch (mode) {
    case "code":
      return this.processCodeTask(request)
    case "research":
      return this.processResearchTask(request)
    case "decision":
      return this.processDecisionTask(request)
  }
}
```

### Task Classifier

```typescript
private async classifyTask(request: string): Promise<TaskClassification> {
  const result = await generateObject({
    model: await Provider.getLanguageModel(),
    schema: z.object({
      mode: z.enum(["code", "research", "decision"]),
      confidence: z.number().min(0).max(1),
      reasoning: z.string(),
    }),
    prompt: `Classify this request into one of three modes:
- code: Implementation, bug fixes, refactoring, creating features
- research: Analysis, investigation, market research, news gathering
- decision: Choices, trade-offs, career decisions, investment analysis

Request: "${request}"`,
  })

  return result.object
}
```

### Research Mode Execution

Uses existing `createResearchLoop()`:

```typescript
private async processResearchTask(request: string): Promise<ProcessResult> {
  const researchLoop = createResearchLoop({
    maxSources: 10,
    enableLearning: true,
    enableHandCreation: this.config.unattended,
  })

  const result = await researchLoop.research({
    sessionId: this.context.sessionId,
    topic: request,
    sourceTypes: ["web", "news", "financial"],
  })

  return {
    success: result.success,
    mode: "research",
    result: { /* mapped fields */ },
  }
}
```

### Decision Mode Execution

Research first, then CLOSE evaluation:

```typescript
private async processDecisionTask(request: string): Promise<ProcessResult> {
  // Phase 1: Gather context via research
  const researchLoop = createResearchLoop({ maxSources: 8 })
  const research = await researchLoop.research({
    sessionId: this.context.sessionId,
    topic: request,
  })

  // Phase 2: Run CLOSE evaluation via decision agent
  const decisionResult = await this.invokeDecisionAgent(request, research)

  return {
    success: true,
    mode: "decision",
    result: { research, closeScore, recommendation },
  }
}
```

### Result Types

```typescript
export interface ProcessResult {
  success: boolean
  mode: "code" | "research" | "decision"
  result: CodeResult | ResearchResult | DecisionResult | null
}

interface ResearchResult {
  topic: string
  summary: string
  report: string
  sources: ResearchSource[]
  insights: string[]
  duration: number
  outputPath?: string
}

interface DecisionResult {
  topic: string
  research: ResearchResult
  closeScore: {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
    overall: number
  }
  recommendation: string
  alternatives: string[]
  duration: number
}
```

## Files to Modify

1. `packages/ccode/src/cli/cmd/autonomous.ts` - Add `--mode` flag
2. `packages/ccode/src/autonomous/orchestration/orchestrator.ts` - Add routing logic
3. `packages/ccode/src/autonomous/index.ts` - Export new types

## Testing

1. Verify auto-classification correctly identifies task types
2. Verify `--mode` flag overrides auto-detection
3. Verify research mode produces reports with sources
4. Verify decision mode produces CLOSE scores
5. Verify code mode still works (regression)

## Success Criteria

- Research questions trigger research loop, not TDD
- Decision questions get CLOSE evaluation
- Code tasks continue to work as before
- `--mode` flag allows explicit override
