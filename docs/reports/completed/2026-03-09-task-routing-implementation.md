# Task Routing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Route autonomous mode requests to appropriate execution loops (code/research/decision) based on task classification.

**Architecture:** Extend existing `classification/task-classifier.ts` to add "decision" type, then use it in `Orchestrator.process()` to route to `processCodeTask()`, `processResearchTask()`, or `processDecisionTask()`.

**Tech Stack:** TypeScript, Zod schemas, ai-sdk generateObject

---

## Task 1: Add "decision" TaskType

**Files:**
- Modify: `packages/ccode/src/autonomous/classification/types.ts:11`
- Modify: `packages/ccode/src/autonomous/classification/types.ts:14-15`

**Step 1: Update TaskType union**

In `types.ts`, change line 11:

```typescript
// FROM:
export type TaskType = "implementation" | "research" | "query" | "acceptance" | "fix" | "other"

// TO:
export type TaskType = "implementation" | "research" | "decision" | "query" | "acceptance" | "fix" | "other"
```

**Step 2: Update ClassificationResultSchema**

In `types.ts`, change line 15:

```typescript
// FROM:
type: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),

// TO:
type: z.enum(["implementation", "research", "decision", "query", "acceptance", "fix", "other"]),
```

**Step 3: Add DECISION_KEYWORDS**

Add after line 74 in `types.ts`:

```typescript
export const DECISION_KEYWORDS = [
  // Chinese
  "决定", "选择", "评估", "比较", "权衡", "抉择", "取舍",
  "CLOSE", "决策", "方案", "利弊",
  // English
  "decide", "choose", "evaluate", "compare", "trade-off", "weigh",
  "decision", "option", "pros cons", "close framework",
] as const
```

**Step 4: Commit**

```bash
git add packages/ccode/src/autonomous/classification/types.ts
git commit -m "feat(autonomous): add decision TaskType for CLOSE evaluation"
```

---

## Task 2: Update task-classifier.ts

**Files:**
- Modify: `packages/ccode/src/autonomous/classification/task-classifier.ts:10-18`
- Modify: `packages/ccode/src/autonomous/classification/task-classifier.ts:41-55`

**Step 1: Import DECISION_KEYWORDS**

Change line 17 to add DECISION_KEYWORDS import:

```typescript
import {
  type TaskType,
  type ClassificationResult,
  type ClassifierConfig,
  DEFAULT_CLASSIFIER_CONFIG,
  RESEARCH_KEYWORDS,
  IMPLEMENTATION_KEYWORDS,
  DECISION_KEYWORDS,  // ADD
  QUERY_KEYWORDS,
  ClassificationResultSchema,
} from "./types"
```

**Step 2: Update ruleBasedClassify function**

In function `ruleBasedClassify`, add decision count (around line 42-55):

```typescript
function ruleBasedClassify(message: string): { type: TaskType; confidence: number } {
  const researchCount = countKeywordMatches(message, RESEARCH_KEYWORDS)
  const implementationCount = countKeywordMatches(message, IMPLEMENTATION_KEYWORDS)
  const decisionCount = countKeywordMatches(message, DECISION_KEYWORDS)  // ADD
  const queryCount = countKeywordMatches(message, QUERY_KEYWORDS)

  const total = researchCount + implementationCount + decisionCount + queryCount  // UPDATE

  if (total === 0) {
    return { type: "other", confidence: 0.3 }
  }

  const scores = [
    { type: "research" as const, count: researchCount },
    { type: "implementation" as const, count: implementationCount },
    { type: "decision" as const, count: decisionCount },  // ADD
    { type: "query" as const, count: queryCount },
  ].sort((a, b) => b.count - a.count)

  const winner = scores[0]
  const confidence = winner.count / Math.max(total, 1)

  return { type: winner.type, confidence: Math.min(confidence + 0.3, 0.95) }
}
```

**Step 3: Update LLM classification prompt**

In function `llmClassify`, update the system prompt (around line 74-78):

```typescript
const systemPrompt = `You are a task classifier. Classify the user's request into one of these types:
- implementation: Code writing, feature creation, bug fixes, deployment
- research: Information gathering, analysis, trend research, data synthesis
- decision: Choices, trade-offs, evaluations, career/investment decisions, CLOSE framework
- query: Simple questions, explanations, definitions
- other: Anything else`
```

**Step 4: Update schema in llmClassify**

In function `llmClassify`, update the zod schema (around line 92):

```typescript
schema: z.object({
  type: z.enum(["implementation", "research", "decision", "query", "other"]),
  confidence: z.number(),
  reasoning: z.string(),
  researchTopic: z.string().optional(),
}),
```

**Step 5: Commit**

```bash
git add packages/ccode/src/autonomous/classification/task-classifier.ts
git commit -m "feat(autonomous): add decision classification with CLOSE keywords"
```

---

## Task 3: Add --mode flag to CLI

**Files:**
- Modify: `packages/ccode/src/cli/cmd/autonomous.ts:25-34`
- Modify: `packages/ccode/src/cli/cmd/autonomous.ts:74-106`

**Step 1: Add TaskMode type**

After line 25, add:

```typescript
type TaskMode = "code" | "research" | "decision" | "auto"
```

**Step 2: Update AutonomousArgs interface**

Update the interface around line 27-34:

```typescript
interface AutonomousArgs {
  request: string
  mode?: TaskMode  // ADD
  "autonomy-level"?: AutonomyLevel
  budget?: string
  unattended?: boolean
  "max-tokens"?: number
  "max-cost"?: number
}
```

**Step 3: Add --mode option to builder**

In the builder function (after line 86), add:

```typescript
.option("mode", {
  type: "string",
  choices: ["code", "research", "decision", "auto"] as const,
  default: "auto" as TaskMode,
  describe: "Task mode: code (TDD), research (web search), decision (CLOSE), auto (classify)",
})
```

**Step 4: Pass mode to config**

In function `runAutonomous`, update the config creation (around line 163-167):

```typescript
const config: OrchestratorConfig = {
  mode: args.mode ?? "auto",  // ADD
  autonomyLevel,
  resourceBudget,
  unattended: args.unattended ?? false,
}
```

**Step 5: Commit**

```bash
git add packages/ccode/src/cli/cmd/autonomous.ts
git commit -m "feat(cli): add --mode flag to autonomous command"
```

---

## Task 4: Add mode to OrchestratorConfig

**Files:**
- Modify: `packages/ccode/src/autonomous/orchestration/orchestrator.ts:24-31`

**Step 1: Import TaskMode type**

Add to imports at top of file:

```typescript
import { classifyTask, type TaskType } from "../classification"
```

**Step 2: Add mode to OrchestratorConfig**

Update interface around line 24-31:

```typescript
export interface OrchestratorConfig {
  mode?: "code" | "research" | "decision" | "auto"  // ADD
  autonomyLevel: "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
  resourceBudget: ResourceBudget
  executionConfig?: ExecutionConfig
  unattended: boolean
  enableEvolutionLoop?: boolean
}
```

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): add mode field to OrchestratorConfig"
```

---

## Task 5: Add resolveTaskMode method

**Files:**
- Modify: `packages/ccode/src/autonomous/orchestration/orchestrator.ts`

**Step 1: Add resolveTaskMode method**

Add this method to Orchestrator class (after constructor, around line 128):

```typescript
/**
 * Resolve the task mode from config or classification
 */
private async resolveTaskMode(request: string): Promise<"code" | "research" | "decision"> {
  // If explicit mode provided (not "auto"), use it
  if (this.config.mode && this.config.mode !== "auto") {
    log.info("Using explicit task mode", { mode: this.config.mode })
    return this.config.mode
  }

  // Otherwise, classify with LLM
  log.info("Classifying task type", { requestPreview: request.slice(0, 50) })
  const classification = await classifyTask(request)

  log.info("Task classified", {
    type: classification.type,
    confidence: classification.confidence,
    reasoning: classification.reasoning,
  })

  // Map TaskType to our modes
  switch (classification.type) {
    case "implementation":
    case "fix":
      return "code"
    case "research":
    case "query":
      return "research"
    case "decision":
      return "decision"
    case "acceptance":
      return "code"  // Acceptance is part of code flow
    default:
      return "code"  // Default to code mode
  }
}
```

**Step 2: Commit**

```bash
git add packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): add resolveTaskMode method for task classification"
```

---

## Task 6: Add processResearchTask method

**Files:**
- Modify: `packages/ccode/src/autonomous/orchestration/orchestrator.ts`

**Step 1: Import createResearchLoop**

Add to imports:

```typescript
import { createResearchLoop, type ResearchResult } from "../execution/research-loop"
```

**Step 2: Add processResearchTask method**

Add this method to Orchestrator class:

```typescript
/**
 * Process a research task using the research loop
 */
private async processResearchTask(request: string): Promise<{
  success: boolean
  result: {
    success: boolean
    mode: "research"
    topic: string
    report: string
    sources: Array<{ url: string; title: string; relevance: number }>
    insights: string[]
    duration: number
    tokensUsed: number
    costUSD: number
  } | null
}> {
  log.info("Processing research task", {
    sessionId: this.context.sessionId,
    topic: request.slice(0, 100),
  })

  await this.stateMachine.transition(AutonomousState.EXECUTING, {
    reason: "Starting research loop",
  })

  const researchLoop = createResearchLoop({
    maxSources: 10,
    enableLearning: true,
  })

  try {
    const result = await researchLoop.research({
      sessionId: this.context.sessionId,
      topic: request,
    })

    const usage = this.safetyGuard.getCurrentUsage()

    await this.stateMachine.transition(AutonomousState.COMPLETED, {
      reason: "Research completed",
    })

    return {
      success: result.success,
      result: {
        success: result.success,
        mode: "research",
        topic: request,
        report: result.report ?? "",
        sources: result.sources ?? [],
        insights: result.insights ?? [],
        duration: Date.now() - this.context.startTime,
        tokensUsed: usage.tokensUsed,
        costUSD: usage.costUSD,
      },
    }
  } catch (error) {
    log.error("Research task failed", {
      sessionId: this.context.sessionId,
      error: error instanceof Error ? error.message : String(error),
    })

    await this.stateMachine.transition(AutonomousState.FAILED, {
      reason: error instanceof Error ? error.message : String(error),
    })

    return { success: false, result: null }
  }
}
```

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): add processResearchTask method"
```

---

## Task 7: Add processDecisionTask method

**Files:**
- Modify: `packages/ccode/src/autonomous/orchestration/orchestrator.ts`

**Step 1: Add processDecisionTask method**

Add this method to Orchestrator class:

```typescript
/**
 * Process a decision task using research + CLOSE evaluation
 */
private async processDecisionTask(request: string): Promise<{
  success: boolean
  result: {
    success: boolean
    mode: "decision"
    topic: string
    research: string
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
    tokensUsed: number
    costUSD: number
  } | null
}> {
  log.info("Processing decision task", {
    sessionId: this.context.sessionId,
    topic: request.slice(0, 100),
  })

  await this.stateMachine.transition(AutonomousState.EXECUTING, {
    reason: "Starting decision analysis",
  })

  // Phase 1: Gather context via research
  const researchLoop = createResearchLoop({ maxSources: 8 })

  try {
    const research = await researchLoop.research({
      sessionId: this.context.sessionId,
      topic: request,
    })

    // Phase 2: Run CLOSE evaluation
    await this.stateMachine.transition(AutonomousState.EVALUATING, {
      reason: "Running CLOSE evaluation",
    })

    const closeResult = await this.evaluateWithCLOSE(request, research)
    const usage = this.safetyGuard.getCurrentUsage()

    await this.stateMachine.transition(AutonomousState.COMPLETED, {
      reason: "Decision analysis completed",
    })

    return {
      success: true,
      result: {
        success: true,
        mode: "decision",
        topic: request,
        research: research.report ?? "",
        closeScore: closeResult.score,
        recommendation: closeResult.recommendation,
        alternatives: closeResult.alternatives,
        duration: Date.now() - this.context.startTime,
        tokensUsed: usage.tokensUsed,
        costUSD: usage.costUSD,
      },
    }
  } catch (error) {
    log.error("Decision task failed", {
      sessionId: this.context.sessionId,
      error: error instanceof Error ? error.message : String(error),
    })

    await this.stateMachine.transition(AutonomousState.FAILED, {
      reason: error instanceof Error ? error.message : String(error),
    })

    return { success: false, result: null }
  }
}

/**
 * Evaluate a decision using CLOSE framework
 */
private async evaluateWithCLOSE(
  request: string,
  research: ResearchResult,
): Promise<{
  score: {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
    overall: number
  }
  recommendation: string
  alternatives: string[]
}> {
  const { generateObject } = await import("ai")
  const { Provider } = await import("@/provider/provider")
  const z = (await import("zod")).default

  const defaultModel = await Provider.defaultModel()
  const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
  const language = await Provider.getLanguage(model)

  const result = await generateObject({
    model: language,
    schema: z.object({
      convergence: z.number().min(0).max(10).describe("How certain/convergent is the outcome? 0=chaotic, 10=certain"),
      leverage: z.number().min(0).max(10).describe("How much impact per unit effort? 0=none, 10=massive"),
      optionality: z.number().min(0).max(10).describe("How reversible? How many future options preserved? 0=irreversible, 10=fully reversible"),
      surplus: z.number().min(0).max(10).describe("How much buffer/margin for error? 0=none, 10=abundant"),
      evolution: z.number().min(0).max(10).describe("Does this enable growth/learning? 0=stagnation, 10=high growth"),
      recommendation: z.string().describe("Primary recommendation based on CLOSE analysis"),
      alternatives: z.array(z.string()).describe("2-3 alternative options"),
    }),
    prompt: `Analyze this decision using the CLOSE framework (祝融说):

Decision: ${request}

Research Context:
${research.report?.slice(0, 2000) ?? "No research available"}

Evaluate each CLOSE dimension (0-10):
- C (Convergence): How certain/predictable is the outcome?
- L (Leverage): How much impact per unit of effort?
- O (Optionality): How reversible? Future options preserved?
- S (Surplus): How much buffer/margin for error?
- E (Evolution): Does this enable growth and learning?

Provide a clear recommendation and 2-3 alternatives.`,
  })

  const obj = result.object
  const overall = (obj.convergence + obj.leverage + obj.optionality + obj.surplus + obj.evolution) / 5

  return {
    score: {
      convergence: obj.convergence,
      leverage: obj.leverage,
      optionality: obj.optionality,
      surplus: obj.surplus,
      evolution: obj.evolution,
      overall,
    },
    recommendation: obj.recommendation,
    alternatives: obj.alternatives,
  }
}
```

**Step 2: Commit**

```bash
git add packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): add processDecisionTask with CLOSE evaluation"
```

---

## Task 8: Update process() method to route

**Files:**
- Modify: `packages/ccode/src/autonomous/orchestration/orchestrator.ts:163-175`

**Step 1: Add routing at start of process()**

Replace the beginning of process() method (around line 163-182):

```typescript
async process(request: string): Promise<{
  success: boolean
  result: {
    success: boolean
    qualityScore?: number
    crazinessScore?: number
    duration: number
    tokensUsed: number
    costUSD: number
    iterationsCompleted?: number
    // Research mode fields
    mode?: "code" | "research" | "decision"
    topic?: string
    report?: string
    sources?: Array<{ url: string; title: string; relevance: number }>
    insights?: string[]
    // Decision mode fields
    closeScore?: {
      convergence: number
      leverage: number
      optionality: number
      surplus: number
      evolution: number
      overall: number
    }
    recommendation?: string
    alternatives?: string[]
  } | null
}> {
  try {
    // Step 1: Determine task mode
    const mode = await this.resolveTaskMode(request)

    log.info("Task mode resolved", {
      sessionId: this.context.sessionId,
      mode,
    })

    // Step 2: Route to appropriate execution path
    switch (mode) {
      case "research":
        return this.processResearchTask(request)
      case "decision":
        return this.processDecisionTask(request)
      case "code":
      default:
        return this.processCodeTask(request)
    }
  } catch (error) {
    // ... existing error handling
  }
}
```

**Step 2: Rename existing implementation to processCodeTask**

Find the existing process() implementation (the main loop with TDD) and extract it into a new private method:

```typescript
/**
 * Process a code task through the full TDD pipeline
 */
private async processCodeTask(request: string): Promise<{
  success: boolean
  result: {
    success: boolean
    mode: "code"
    qualityScore: number
    crazinessScore: number
    duration: number
    tokensUsed: number
    costUSD: number
    iterationsCompleted: number
  } | null
}> {
  // ... move existing TDD loop code here
  // Add mode: "code" to result
}
```

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): add task routing in process() method"
```

---

## Task 9: Update displayResults for all modes

**Files:**
- Modify: `packages/ccode/src/cli/cmd/autonomous.ts:302-341`

**Step 1: Update displayResults function**

Replace the displayResults function:

```typescript
function displayResults(result: {
  success: boolean
  mode?: "code" | "research" | "decision"
  qualityScore?: number
  crazinessScore?: number
  duration: number
  tokensUsed: number
  costUSD: number
  iterationsCompleted?: number
  // Research
  topic?: string
  report?: string
  sources?: Array<{ url: string; title: string; relevance: number }>
  insights?: string[]
  // Decision
  closeScore?: {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
    overall: number
  }
  recommendation?: string
  alternatives?: string[]
}): void {
  prompts.log.success("Autonomous execution completed!")

  const minutes = Math.floor(result.duration / 60000)
  const seconds = Math.floor((result.duration % 60000) / 1000)
  const mode = result.mode ?? "code"

  const lines = [
    "",
    "═══════════════════════════════════════════════════",
    `           AUTONOMOUS MODE REPORT [${mode.toUpperCase()}]`,
    "═══════════════════════════════════════════════════",
    "",
  ]

  if (mode === "code") {
    lines.push(`  Quality Score:    ${(result.qualityScore ?? 0).toFixed(1)}/100`)
    lines.push(`  Craziness Score:  ${(result.crazinessScore ?? 0).toFixed(1)}/100`)
    if (result.iterationsCompleted !== undefined) {
      lines.push(`  Iterations:       ${result.iterationsCompleted}`)
    }
  } else if (mode === "research") {
    lines.push(`  Topic:            ${result.topic?.slice(0, 40) ?? "N/A"}`)
    lines.push(`  Sources Found:    ${result.sources?.length ?? 0}`)
    lines.push(`  Insights:         ${result.insights?.length ?? 0}`)
    if (result.report) {
      lines.push("")
      lines.push("  Report Preview:")
      lines.push(`  ${result.report.slice(0, 200)}...`)
    }
  } else if (mode === "decision") {
    lines.push(`  Topic:            ${result.topic?.slice(0, 40) ?? "N/A"}`)
    if (result.closeScore) {
      lines.push("")
      lines.push("  CLOSE Score:")
      lines.push(`    Convergence:    ${result.closeScore.convergence.toFixed(1)}/10`)
      lines.push(`    Leverage:       ${result.closeScore.leverage.toFixed(1)}/10`)
      lines.push(`    Optionality:    ${result.closeScore.optionality.toFixed(1)}/10`)
      lines.push(`    Surplus:        ${result.closeScore.surplus.toFixed(1)}/10`)
      lines.push(`    Evolution:      ${result.closeScore.evolution.toFixed(1)}/10`)
      lines.push(`    ─────────────────────`)
      lines.push(`    Overall:        ${result.closeScore.overall.toFixed(1)}/10`)
    }
    if (result.recommendation) {
      lines.push("")
      lines.push(`  Recommendation:   ${result.recommendation.slice(0, 60)}`)
    }
  }

  lines.push("")
  lines.push(`  Duration:         ${minutes}m ${seconds}s`)
  lines.push(`  Tokens Used:      ${result.tokensUsed.toLocaleString()}`)
  lines.push(`  Cost:             $${result.costUSD.toFixed(4)}`)
  lines.push("")
  lines.push("═══════════════════════════════════════════════════")

  for (const line of lines) {
    prompts.log.message(line)
  }
}
```

**Step 2: Commit**

```bash
git add packages/ccode/src/cli/cmd/autonomous.ts
git commit -m "feat(cli): update displayResults for all task modes"
```

---

## Task 10: Export new types from index.ts

**Files:**
- Modify: `packages/ccode/src/autonomous/index.ts`

**Step 1: Add TaskMode export**

Near line 60, update the orchestrator exports:

```typescript
export { Orchestrator, createOrchestrator } from "./orchestration/orchestrator"
export type { OrchestratorConfig, SessionContext, TaskMode } from "./orchestration/orchestrator"
```

**Step 2: Add TaskMode type to orchestrator.ts**

In orchestrator.ts, add the type export:

```typescript
export type TaskMode = "code" | "research" | "decision" | "auto"
```

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/index.ts packages/ccode/src/autonomous/orchestration/orchestrator.ts
git commit -m "feat(autonomous): export TaskMode type"
```

---

## Task 11: Integration test

**Files:**
- Run tests in: `packages/ccode/`

**Step 1: Run type check**

```bash
cd packages/ccode && bun run typecheck
```

Expected: No errors

**Step 2: Test auto classification**

```bash
cd /Users/iannil/Code/zproducts/code-coder && timeout 30 bun run --cwd packages/ccode src/index.ts autonomous "分析最近的AI发展趋势" --autonomy-level timid --max-tokens 5000 --unattended --print-logs 2>&1 | head -50
```

Expected: Should show "Task mode resolved: research"

**Step 3: Test explicit mode**

```bash
cd /Users/iannil/Code/zproducts/code-coder && timeout 30 bun run --cwd packages/ccode src/index.ts autonomous --mode research "测试研究模式" --autonomy-level timid --max-tokens 5000 --unattended --print-logs 2>&1 | head -50
```

Expected: Should show "Using explicit task mode: research"

**Step 4: Test decision mode**

```bash
cd /Users/iannil/Code/zproducts/code-coder && timeout 30 bun run --cwd packages/ccode src/index.ts autonomous --mode decision "评估是否应该学习Rust" --autonomy-level timid --max-tokens 5000 --unattended --print-logs 2>&1 | head -50
```

Expected: Should show CLOSE evaluation running

**Step 5: Commit final changes**

```bash
git add -A
git commit -m "feat(autonomous): complete task routing implementation

- Add decision TaskType to classification
- Add --mode CLI flag (code/research/decision/auto)
- Route to processResearchTask for research mode
- Route to processDecisionTask with CLOSE evaluation
- Update CLI displayResults for all modes"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add decision TaskType | classification/types.ts |
| 2 | Update task-classifier | classification/task-classifier.ts |
| 3 | Add --mode CLI flag | cli/cmd/autonomous.ts |
| 4 | Add mode to OrchestratorConfig | orchestration/orchestrator.ts |
| 5 | Add resolveTaskMode method | orchestration/orchestrator.ts |
| 6 | Add processResearchTask | orchestration/orchestrator.ts |
| 7 | Add processDecisionTask + CLOSE | orchestration/orchestrator.ts |
| 8 | Update process() routing | orchestration/orchestrator.ts |
| 9 | Update displayResults | cli/cmd/autonomous.ts |
| 10 | Export new types | autonomous/index.ts |
| 11 | Integration test | - |

**Total commits:** 10 (one per task)
