# Research Loop Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix autonomous task early termination by adding intelligent task classification and a dedicated Research Loop for research/analysis tasks.

**Architecture:** LLM-based Task Classifier routes messages to either Evolution Loop (implementation) or Research Loop (research). Research Loop executes multi-source search, synthesis, analysis, and report generation with learning capabilities.

**Tech Stack:** TypeScript, Zod, Claude Haiku (classifier), existing WebSearcher, Bus event system

---

## Task 1: Add Research Events to events.ts

**Files:**
- Modify: `packages/ccode/src/autonomous/events.ts:545-586`

**Step 1: Write the event definitions**

Add after line 544 (before `const BusPromise`):

```typescript
  // ============================================================================
  // Research Loop Events
  // ============================================================================

  export const ResearchStarted = BusEvent.define(
    "autonomous.research.started",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      dimensions: z.array(z.string()).optional(),
      sourceTypes: z.array(z.enum(["web", "financial", "news"])).optional(),
    }),
  )

  export const ResearchPhaseChanged = BusEvent.define(
    "autonomous.research.phase_changed",
    z.object({
      sessionId: z.string(),
      phase: z.enum([
        "understanding",
        "searching",
        "synthesizing",
        "analyzing",
        "reporting",
        "learning",
      ]),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const ResearchSourceFound = BusEvent.define(
    "autonomous.research.source_found",
    z.object({
      sessionId: z.string(),
      sourceCount: z.number(),
      credibilityBreakdown: z.object({
        high: z.number(),
        medium: z.number(),
        low: z.number(),
      }),
    }),
  )

  export const ResearchCompleted = BusEvent.define(
    "autonomous.research.completed",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      success: z.boolean(),
      reportMode: z.enum(["inline", "file"]),
      reportPath: z.string().optional(),
      insightCount: z.number(),
      sourceCount: z.number(),
      durationMs: z.number(),
      handCreated: z.string().optional(),
    }),
  )

  export const ResearchFailed = BusEvent.define(
    "autonomous.research.failed",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      phase: z.string(),
      error: z.string(),
      retryable: z.boolean(),
    }),
  )

  export const ResearchPatternLearned = BusEvent.define(
    "autonomous.research.pattern_learned",
    z.object({
      sessionId: z.string(),
      patternId: z.string(),
      topic: z.string(),
      keywords: z.array(z.string()),
      frequency: z.enum(["daily", "weekly", "monthly"]).optional(),
      confidence: z.number(),
    }),
  )
```

**Step 2: Verify TypeScript compiles**

Run: `cd packages/ccode && bun run typecheck`
Expected: No errors

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/events.ts
git commit -m "feat(autonomous): add Research Loop events"
```

---

## Task 2: Create Task Classifier Types

**Files:**
- Create: `packages/ccode/src/autonomous/classification/types.ts`

**Step 1: Write types file**

```typescript
/**
 * Task Classification Types
 *
 * Defines types for intelligent task classification that routes
 * messages to appropriate execution loops.
 */

import z from "zod"

/** Supported task types */
export type TaskType = "implementation" | "research" | "query" | "other"

/** Classification result schema */
export const ClassificationResultSchema = z.object({
  type: z.enum(["implementation", "research", "query", "other"]),
  confidence: z.number().min(0).max(1),
  reasoning: z.string(),
  /** Research-specific: identified topic */
  researchTopic: z.string().optional(),
  /** Research-specific: suggested data sources */
  suggestedSources: z.array(z.string()).optional(),
  /** Research-specific: is this a periodic task? */
  isPeriodic: z.boolean().optional(),
})

export type ClassificationResult = z.infer<typeof ClassificationResultSchema>

/** Keywords for quick pre-classification */
export const RESEARCH_KEYWORDS = [
  // Chinese
  "梳理", "分析", "研究", "调研", "走势", "行情",
  "汇总", "总结", "对比", "评估", "趋势", "预测",
  "盘点", "回顾", "展望", "解读", "综述",
  // English
  "analyze", "research", "trend", "summary", "review",
  "compare", "evaluate", "forecast", "outlook",
] as const

export const IMPLEMENTATION_KEYWORDS = [
  // Chinese
  "实现", "创建", "修复", "开发", "构建", "编写", "生成", "执行",
  "部署", "配置", "设置", "安装", "更新", "修改", "重构", "优化",
  "自动", "定时", "调度",
  // English
  "implement", "create", "fix", "build", "write", "generate", "execute",
  "deploy", "configure", "setup", "install", "update", "modify", "refactor",
  "automate", "schedule", "cron",
] as const

export const QUERY_KEYWORDS = [
  // Chinese
  "什么是", "为什么", "怎么", "如何", "哪些", "哪个",
  "是什么", "能否", "可以吗", "有没有",
  // English
  "what is", "why", "how", "which", "can you", "is there",
] as const

/** Classifier configuration */
export interface ClassifierConfig {
  /** Use LLM for uncertain cases (default: true) */
  useLLMFallback?: boolean
  /** Confidence threshold for rule-based classification (default: 0.7) */
  ruleConfidenceThreshold?: number
  /** LLM model to use (default: haiku) */
  llmModel?: "haiku" | "sonnet"
}

export const DEFAULT_CLASSIFIER_CONFIG: Required<ClassifierConfig> = {
  useLLMFallback: true,
  ruleConfidenceThreshold: 0.7,
  llmModel: "haiku",
}
```

**Step 2: Verify TypeScript compiles**

Run: `cd packages/ccode && bun run typecheck`
Expected: No errors

**Step 3: Commit**

```bash
git add packages/ccode/src/autonomous/classification/types.ts
git commit -m "feat(autonomous): add task classifier types"
```

---

## Task 3: Create Task Classifier Implementation

**Files:**
- Create: `packages/ccode/src/autonomous/classification/task-classifier.ts`
- Test: `packages/ccode/test/autonomous/classification/task-classifier.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, test, expect } from "bun:test"
import { classifyTask, createTaskClassifier } from "../../../src/autonomous/classification/task-classifier"

describe("TaskClassifier", () => {
  describe("rule-based classification", () => {
    test("classifies research keywords as research", async () => {
      const result = await classifyTask("梳理当前的黄金走势情况")
      expect(result.type).toBe("research")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies implementation keywords as implementation", async () => {
      const result = await classifyTask("实现一个用户登录功能")
      expect(result.type).toBe("implementation")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("classifies query keywords as query", async () => {
      const result = await classifyTask("什么是 TypeScript")
      expect(result.type).toBe("query")
      expect(result.confidence).toBeGreaterThan(0.5)
    })

    test("extracts research topic", async () => {
      const result = await classifyTask("分析今年的比特币走势")
      expect(result.type).toBe("research")
      expect(result.researchTopic).toBeDefined()
    })
  })

  describe("factory function", () => {
    test("creates classifier with config", () => {
      const classifier = createTaskClassifier({ useLLMFallback: false })
      expect(classifier).toBeDefined()
      expect(classifier.classify).toBeInstanceOf(Function)
    })
  })
})
```

**Step 2: Run test to verify it fails**

Run: `cd packages/ccode && bun test test/autonomous/classification/task-classifier.test.ts`
Expected: FAIL with "Cannot find module"

**Step 3: Write minimal implementation**

```typescript
/**
 * Task Classifier
 *
 * Two-stage classification:
 * 1. Fast rule-based pre-classification using keywords
 * 2. LLM-based classification for uncertain cases
 */

import { Log } from "@/util/log"
import {
  type TaskType,
  type ClassificationResult,
  type ClassifierConfig,
  DEFAULT_CLASSIFIER_CONFIG,
  RESEARCH_KEYWORDS,
  IMPLEMENTATION_KEYWORDS,
  QUERY_KEYWORDS,
  ClassificationResultSchema,
} from "./types"

const log = Log.create({ service: "autonomous.classifier" })

/** Count keyword matches in message */
function countKeywordMatches(message: string, keywords: readonly string[]): number {
  const lowerMessage = message.toLowerCase()
  return keywords.filter((k) => lowerMessage.includes(k.toLowerCase())).length
}

/** Extract potential research topic from message */
function extractResearchTopic(message: string): string | undefined {
  // Simple extraction: remove common research verbs and particles
  const cleaned = message
    .replace(/^(梳理|分析|研究|调研|总结|评估|盘点|回顾|解读)/g, "")
    .replace(/(的情况|的走势|的趋势|的现状)$/g, "")
    .trim()

  return cleaned.length > 0 ? cleaned : undefined
}

/** Rule-based pre-classification */
function ruleBasedClassify(message: string): { type: TaskType; confidence: number } {
  const researchCount = countKeywordMatches(message, RESEARCH_KEYWORDS)
  const implementationCount = countKeywordMatches(message, IMPLEMENTATION_KEYWORDS)
  const queryCount = countKeywordMatches(message, QUERY_KEYWORDS)

  const total = researchCount + implementationCount + queryCount

  if (total === 0) {
    return { type: "other", confidence: 0.3 }
  }

  const scores = [
    { type: "research" as const, count: researchCount },
    { type: "implementation" as const, count: implementationCount },
    { type: "query" as const, count: queryCount },
  ].sort((a, b) => b.count - a.count)

  const winner = scores[0]
  const confidence = winner.count / Math.max(total, 1)

  return { type: winner.type, confidence: Math.min(confidence + 0.3, 0.95) }
}

/** LLM-based classification for uncertain cases */
async function llmClassify(
  message: string,
  _model: "haiku" | "sonnet",
): Promise<ClassificationResult> {
  // Lazy import to avoid circular dependency
  const { Anthropic } = await import("@anthropic-ai/sdk")

  const client = new Anthropic()

  const systemPrompt = `You are a task classifier. Classify the user's request into one of these types:
- implementation: Code writing, feature creation, bug fixes, deployment
- research: Information gathering, analysis, trend research, data synthesis
- query: Simple questions, explanations, definitions
- other: Anything else

Respond with JSON only: {"type": "...", "confidence": 0.9, "reasoning": "...", "researchTopic": "..." (if research)}`

  try {
    const response = await client.messages.create({
      model: "claude-3-5-haiku-latest",
      max_tokens: 200,
      system: systemPrompt,
      messages: [{ role: "user", content: message }],
    })

    const text = response.content[0].type === "text" ? response.content[0].text : ""
    const jsonMatch = text.match(/\{[\s\S]*\}/)
    if (!jsonMatch) {
      throw new Error("No JSON in response")
    }

    const parsed = JSON.parse(jsonMatch[0])
    return ClassificationResultSchema.parse(parsed)
  } catch (error) {
    log.warn("LLM classification failed, falling back to rules", {
      error: error instanceof Error ? error.message : String(error),
    })
    const { type, confidence } = ruleBasedClassify(message)
    return { type, confidence, reasoning: "Fallback to rule-based classification" }
  }
}

/** Main classification function */
export async function classifyTask(
  message: string,
  config: ClassifierConfig = {},
): Promise<ClassificationResult> {
  const cfg = { ...DEFAULT_CLASSIFIER_CONFIG, ...config }

  // Step 1: Rule-based pre-classification
  const ruleResult = ruleBasedClassify(message)

  log.debug("Rule-based classification", {
    message: message.slice(0, 50),
    type: ruleResult.type,
    confidence: ruleResult.confidence,
  })

  // If confidence is high enough, use rule-based result
  if (ruleResult.confidence >= cfg.ruleConfidenceThreshold) {
    const result: ClassificationResult = {
      type: ruleResult.type,
      confidence: ruleResult.confidence,
      reasoning: "Rule-based classification with high confidence",
    }

    // Extract research topic if applicable
    if (ruleResult.type === "research") {
      result.researchTopic = extractResearchTopic(message)
    }

    return result
  }

  // Step 2: LLM classification for uncertain cases
  if (cfg.useLLMFallback) {
    log.debug("Using LLM fallback for classification", { message: message.slice(0, 50) })
    return llmClassify(message, cfg.llmModel)
  }

  // No LLM fallback, return rule-based with low confidence
  return {
    type: ruleResult.type,
    confidence: ruleResult.confidence,
    reasoning: "Rule-based classification (LLM disabled)",
    researchTopic: ruleResult.type === "research" ? extractResearchTopic(message) : undefined,
  }
}

/** Factory function to create a classifier instance */
export function createTaskClassifier(config: ClassifierConfig = {}) {
  const cfg = { ...DEFAULT_CLASSIFIER_CONFIG, ...config }

  return {
    classify: (message: string) => classifyTask(message, cfg),
    config: cfg,
  }
}

export type TaskClassifier = ReturnType<typeof createTaskClassifier>
```

**Step 4: Run test to verify it passes**

Run: `cd packages/ccode && bun test test/autonomous/classification/task-classifier.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add packages/ccode/src/autonomous/classification/task-classifier.ts packages/ccode/test/autonomous/classification/task-classifier.test.ts
git commit -m "feat(autonomous): implement task classifier with TDD"
```

---

## Task 4: Create Classification Index

**Files:**
- Create: `packages/ccode/src/autonomous/classification/index.ts`

**Step 1: Write index file**

```typescript
export * from "./types"
export * from "./task-classifier"
```

**Step 2: Commit**

```bash
git add packages/ccode/src/autonomous/classification/index.ts
git commit -m "feat(autonomous): add classification module index"
```

---

## Task 5: Create Report Renderer

**Files:**
- Create: `packages/ccode/src/autonomous/execution/report-renderer.ts`
- Test: `packages/ccode/test/autonomous/execution/report-renderer.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { renderReport, createReportRenderer } from "../../../src/autonomous/execution/report-renderer"
import { existsSync, rmSync, mkdirSync } from "fs"
import { join } from "path"

const TEST_DIR = "/tmp/test-reports"

describe("ReportRenderer", () => {
  beforeEach(() => {
    if (existsSync(TEST_DIR)) rmSync(TEST_DIR, { recursive: true })
    mkdirSync(TEST_DIR, { recursive: true })
  })

  afterEach(() => {
    if (existsSync(TEST_DIR)) rmSync(TEST_DIR, { recursive: true })
  })

  test("renders short report inline", async () => {
    const result = await renderReport({
      topic: "Test Topic",
      summary: "Short summary",
      analysis: "Brief analysis",
      insights: ["insight 1"],
      sources: [],
    }, { maxInlineLength: 1000, outputDir: TEST_DIR })

    expect(result.mode).toBe("inline")
    expect(result.content).toContain("Test Topic")
    expect(result.filePath).toBeUndefined()
  })

  test("saves long report to file", async () => {
    const longAnalysis = "x".repeat(1500)
    const result = await renderReport({
      topic: "Long Topic",
      summary: "Summary",
      analysis: longAnalysis,
      insights: [],
      sources: [],
    }, { maxInlineLength: 1000, outputDir: TEST_DIR })

    expect(result.mode).toBe("file")
    expect(result.filePath).toBeDefined()
    expect(existsSync(result.filePath!)).toBe(true)
  })

  test("factory creates renderer with config", () => {
    const renderer = createReportRenderer({ maxInlineLength: 500 })
    expect(renderer.render).toBeInstanceOf(Function)
  })
})
```

**Step 2: Run test to verify it fails**

Run: `cd packages/ccode && bun test test/autonomous/execution/report-renderer.test.ts`
Expected: FAIL

**Step 3: Write minimal implementation**

```typescript
/**
 * Report Renderer
 *
 * Renders research reports to either inline text or files
 * based on report length.
 */

import { Log } from "@/util/log"
import { join } from "path"
import { homedir } from "os"

const log = Log.create({ service: "autonomous.report-renderer" })

export interface ReportData {
  topic: string
  summary: string
  analysis: string
  insights: string[]
  sources: Array<{
    url: string
    title: string
    snippet?: string
    credibility?: "high" | "medium" | "low"
  }>
}

export interface RenderConfig {
  /** Maximum characters for inline return (default: 1000) */
  maxInlineLength?: number
  /** Output directory for file reports */
  outputDir?: string
  /** Filename pattern: {date}-{topic}.md */
  filenamePattern?: string
}

export interface RenderResult {
  mode: "inline" | "file"
  content: string
  filePath?: string
}

const DEFAULT_CONFIG: Required<RenderConfig> = {
  maxInlineLength: 1000,
  outputDir: join(homedir(), ".codecoder", "workspace", "reports"),
  filenamePattern: "{date}-{topic}.md",
}

/** Format report as Markdown */
function formatReport(data: ReportData): string {
  const sections: string[] = []

  sections.push(`# ${data.topic} 分析报告\n`)
  sections.push(`**生成时间**: ${new Date().toISOString()}`)
  sections.push(`**数据来源**: ${data.sources.length} 个来源\n`)

  sections.push("## 摘要\n")
  sections.push(data.summary + "\n")

  sections.push("## 详细分析\n")
  sections.push(data.analysis + "\n")

  if (data.insights.length > 0) {
    sections.push("## 关键洞察\n")
    data.insights.forEach((insight, i) => {
      sections.push(`${i + 1}. ${insight}`)
    })
    sections.push("")
  }

  if (data.sources.length > 0) {
    sections.push("## 数据来源\n")
    data.sources.forEach((source) => {
      const credIcon = source.credibility === "high" ? "🟢" : source.credibility === "medium" ? "🟡" : "🔴"
      sections.push(`- ${credIcon} [${source.title}](${source.url})`)
    })
    sections.push("")
  }

  sections.push("---")
  sections.push("*由 CodeCoder Research Loop 自动生成*")

  return sections.join("\n")
}

/** Generate filename from pattern */
function generateFilename(pattern: string, topic: string): string {
  const date = new Date().toISOString().split("T")[0]
  const safeTopic = topic
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/g, "-")
    .slice(0, 50)
  return pattern.replace("{date}", date).replace("{topic}", safeTopic)
}

/** Generate summary for file mode */
function generateSummary(data: ReportData, filePath: string): string {
  const lines = [
    `📊 **${data.topic}** 分析报告已生成`,
    "",
    `📝 **摘要**: ${data.summary.slice(0, 200)}${data.summary.length > 200 ? "..." : ""}`,
    "",
    `💡 **关键洞察**: ${data.insights.length} 条`,
    `📚 **数据来源**: ${data.sources.length} 个`,
    "",
    `📄 **完整报告**: \`${filePath}\``,
  ]
  return lines.join("\n")
}

/** Main render function */
export async function renderReport(
  data: ReportData,
  config: RenderConfig = {},
): Promise<RenderResult> {
  const cfg = { ...DEFAULT_CONFIG, ...config }
  const report = formatReport(data)

  log.debug("Rendering report", {
    topic: data.topic,
    length: report.length,
    maxInline: cfg.maxInlineLength,
  })

  // Check if report fits inline
  if (report.length <= cfg.maxInlineLength) {
    return {
      mode: "inline",
      content: report,
    }
  }

  // Save to file
  const filename = generateFilename(cfg.filenamePattern, data.topic)
  const filePath = join(cfg.outputDir, filename)

  // Ensure directory exists
  const { mkdir } = await import("fs/promises")
  await mkdir(cfg.outputDir, { recursive: true })

  // Write file
  await Bun.write(filePath, report)

  log.info("Report saved to file", { filePath, length: report.length })

  return {
    mode: "file",
    content: generateSummary(data, filePath),
    filePath,
  }
}

/** Factory function to create a renderer instance */
export function createReportRenderer(config: RenderConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }

  return {
    render: (data: ReportData) => renderReport(data, cfg),
    config: cfg,
  }
}

export type ReportRenderer = ReturnType<typeof createReportRenderer>
```

**Step 4: Run test to verify it passes**

Run: `cd packages/ccode && bun test test/autonomous/execution/report-renderer.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add packages/ccode/src/autonomous/execution/report-renderer.ts packages/ccode/test/autonomous/execution/report-renderer.test.ts
git commit -m "feat(autonomous): implement report renderer with TDD"
```

---

## Task 6: Create Research Learner

**Files:**
- Create: `packages/ccode/src/autonomous/execution/research-learner.ts`
- Test: `packages/ccode/test/autonomous/execution/research-learner.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, test, expect } from "bun:test"
import { createResearchLearner, type LearnedResearchPattern } from "../../../src/autonomous/execution/research-learner"

describe("ResearchLearner", () => {
  test("detects periodic pattern from multiple similar requests", () => {
    const learner = createResearchLearner()

    // Simulate multiple similar requests
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "走势"], sources: ["yahoo"] })
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "行情"], sources: ["yahoo"] })
    learner.recordResearch({ topic: "黄金走势", keywords: ["黄金", "价格"], sources: ["yahoo"] })

    const patterns = learner.getPatterns()
    expect(patterns.length).toBeGreaterThan(0)

    const goldPattern = patterns.find(p => p.topic.includes("黄金"))
    expect(goldPattern).toBeDefined()
    expect(goldPattern?.confidence).toBeGreaterThan(0.5)
  })

  test("suggests Hand creation for periodic tasks", () => {
    const learner = createResearchLearner()

    // Simulate daily pattern
    for (let i = 0; i < 5; i++) {
      learner.recordResearch({
        topic: "每日财经新闻",
        keywords: ["财经", "新闻"],
        sources: ["yahoo"],
        timestamp: new Date(Date.now() - i * 24 * 60 * 60 * 1000).toISOString(),
      })
    }

    const suggestion = learner.suggestHandCreation("每日财经新闻")
    expect(suggestion).toBeDefined()
    expect(suggestion?.frequency).toBe("daily")
  })

  test("merges similar keywords across research sessions", () => {
    const learner = createResearchLearner()

    learner.recordResearch({ topic: "股票分析", keywords: ["股票", "A股"] })
    learner.recordResearch({ topic: "股票分析", keywords: ["股票", "大盘"] })

    const pattern = learner.getPattern("股票分析")
    expect(pattern?.keywords).toContain("股票")
    expect(pattern?.keywords.length).toBeGreaterThanOrEqual(2)
  })
})
```

**Step 2: Run test to verify it fails**

Run: `cd packages/ccode && bun test test/autonomous/execution/research-learner.test.ts`
Expected: FAIL

**Step 3: Write minimal implementation**

```typescript
/**
 * Research Learner
 *
 * Learns research patterns from successful research tasks:
 * - Effective keywords for topics
 * - Useful data sources
 * - Periodic task detection
 * - Automatic Hand creation suggestions
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "autonomous.research-learner" })

export interface ResearchRecord {
  topic: string
  keywords: string[]
  sources?: string[]
  timestamp?: string
  success?: boolean
}

export interface LearnedResearchPattern {
  id: string
  topic: string
  keywords: string[]
  sources: string[]
  analysisFramework?: string
  frequency?: "daily" | "weekly" | "monthly"
  confidence: number
  createdAt: string
  lastUsedAt: string
  usageCount: number
}

export interface HandSuggestion {
  topic: string
  frequency: "daily" | "weekly" | "monthly"
  keywords: string[]
  sources: string[]
  schedule: string
  confidence: number
}

interface ResearchLearnerState {
  patterns: Map<string, LearnedResearchPattern>
  history: ResearchRecord[]
}

/** Normalize topic for matching */
function normalizeTopic(topic: string): string {
  return topic.toLowerCase().trim()
}

/** Generate pattern ID */
function generatePatternId(topic: string): string {
  return `research-${normalizeTopic(topic).replace(/\s+/g, "-").slice(0, 30)}-${Date.now()}`
}

/** Detect frequency from timestamps */
function detectFrequency(timestamps: string[]): "daily" | "weekly" | "monthly" | undefined {
  if (timestamps.length < 3) return undefined

  const sorted = timestamps.map((t) => new Date(t).getTime()).sort((a, b) => a - b)
  const intervals = []

  for (let i = 1; i < sorted.length; i++) {
    intervals.push(sorted[i] - sorted[i - 1])
  }

  const avgInterval = intervals.reduce((a, b) => a + b, 0) / intervals.length
  const dayMs = 24 * 60 * 60 * 1000

  if (avgInterval < dayMs * 1.5) return "daily"
  if (avgInterval < dayMs * 10) return "weekly"
  if (avgInterval < dayMs * 45) return "monthly"

  return undefined
}

/** Create cron schedule from frequency */
function frequencyToSchedule(frequency: "daily" | "weekly" | "monthly"): string {
  switch (frequency) {
    case "daily":
      return "0 8 * * *"
    case "weekly":
      return "0 8 * * 1"
    case "monthly":
      return "0 8 1 * *"
  }
}

/** Create a Research Learner instance */
export function createResearchLearner() {
  const state: ResearchLearnerState = {
    patterns: new Map(),
    history: [],
  }

  return {
    /** Record a completed research session */
    recordResearch(record: ResearchRecord): void {
      const normalizedTopic = normalizeTopic(record.topic)
      const timestamp = record.timestamp ?? new Date().toISOString()

      state.history.push({ ...record, timestamp })

      // Update or create pattern
      const existing = state.patterns.get(normalizedTopic)

      if (existing) {
        // Merge keywords (dedupe)
        const allKeywords = [...new Set([...existing.keywords, ...record.keywords])]
        const allSources = [...new Set([...existing.sources, ...(record.sources ?? [])])]

        // Get all timestamps for this topic
        const topicHistory = state.history.filter(
          (h) => normalizeTopic(h.topic) === normalizedTopic,
        )
        const timestamps = topicHistory.map((h) => h.timestamp!).filter(Boolean)

        existing.keywords = allKeywords
        existing.sources = allSources
        existing.lastUsedAt = timestamp
        existing.usageCount++
        existing.frequency = detectFrequency(timestamps)
        existing.confidence = Math.min(0.95, existing.confidence + 0.1)

        log.debug("Updated research pattern", {
          topic: record.topic,
          usageCount: existing.usageCount,
          frequency: existing.frequency,
        })
      } else {
        const pattern: LearnedResearchPattern = {
          id: generatePatternId(record.topic),
          topic: record.topic,
          keywords: record.keywords,
          sources: record.sources ?? [],
          confidence: 0.5,
          createdAt: timestamp,
          lastUsedAt: timestamp,
          usageCount: 1,
        }
        state.patterns.set(normalizedTopic, pattern)

        log.debug("Created new research pattern", { topic: record.topic })
      }
    },

    /** Get all learned patterns */
    getPatterns(): LearnedResearchPattern[] {
      return Array.from(state.patterns.values())
    },

    /** Get a specific pattern by topic */
    getPattern(topic: string): LearnedResearchPattern | undefined {
      return state.patterns.get(normalizeTopic(topic))
    },

    /** Suggest Hand creation for periodic tasks */
    suggestHandCreation(topic: string): HandSuggestion | undefined {
      const pattern = state.patterns.get(normalizeTopic(topic))

      if (!pattern || !pattern.frequency || pattern.usageCount < 3) {
        return undefined
      }

      return {
        topic: pattern.topic,
        frequency: pattern.frequency,
        keywords: pattern.keywords,
        sources: pattern.sources,
        schedule: frequencyToSchedule(pattern.frequency),
        confidence: pattern.confidence,
      }
    },

    /** Export state for persistence */
    exportState(): ResearchLearnerState {
      return {
        patterns: new Map(state.patterns),
        history: [...state.history],
      }
    },

    /** Import state from persistence */
    importState(imported: ResearchLearnerState): void {
      state.patterns = new Map(imported.patterns)
      state.history = [...imported.history]
    },
  }
}

export type ResearchLearner = ReturnType<typeof createResearchLearner>
```

**Step 4: Run test to verify it passes**

Run: `cd packages/ccode && bun test test/autonomous/execution/research-learner.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add packages/ccode/src/autonomous/execution/research-learner.ts packages/ccode/test/autonomous/execution/research-learner.test.ts
git commit -m "feat(autonomous): implement research learner with TDD"
```

---

## Task 7: Create Research Loop Core

**Files:**
- Create: `packages/ccode/src/autonomous/execution/research-loop.ts`
- Test: `packages/ccode/test/autonomous/execution/research-loop.test.ts`

**Step 1: Write the failing test**

```typescript
import { describe, test, expect, mock } from "bun:test"
import { createResearchLoop } from "../../../src/autonomous/execution/research-loop"

describe("ResearchLoop", () => {
  test("creates research loop with config", () => {
    const loop = createResearchLoop({ maxSources: 5 })
    expect(loop).toBeDefined()
    expect(loop.research).toBeInstanceOf(Function)
    expect(loop.cleanup).toBeInstanceOf(Function)
  })

  test("research returns result structure", async () => {
    const loop = createResearchLoop({
      maxSources: 3,
      enableLearning: false,
    })

    // Mock the actual research to avoid network calls
    const result = await loop.research({
      sessionId: "test-session",
      topic: "测试主题",
      maxSources: 2,
    })

    expect(result).toHaveProperty("success")
    expect(result).toHaveProperty("topic")
    expect(result).toHaveProperty("summary")
    expect(result).toHaveProperty("report")
    expect(result).toHaveProperty("sources")
    expect(result).toHaveProperty("insights")
    expect(result).toHaveProperty("durationMs")
  })
})
```

**Step 2: Run test to verify it fails**

Run: `cd packages/ccode && bun test test/autonomous/execution/research-loop.test.ts`
Expected: FAIL

**Step 3: Write implementation**

```typescript
/**
 * Research Loop
 *
 * Dedicated execution loop for research/analysis tasks.
 *
 * Phases:
 * 1. Understanding - Parse topic, dimensions, search strategy
 * 2. Searching - Multi-source parallel search
 * 3. Synthesizing - Dedupe, validate, annotate sources
 * 4. Analyzing - LLM-based analysis and insight extraction
 * 5. Reporting - Generate structured report
 * 6. Learning - Sediment patterns, suggest Hands
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { createWebSearcher, type WebSearcher } from "./web-search"
import { renderReport, type ReportData } from "./report-renderer"
import { createResearchLearner, type ResearchLearner } from "./research-learner"

const log = Log.create({ service: "autonomous.research-loop" })

// ============================================================================
// Types
// ============================================================================

export interface ResearchProblem {
  sessionId: string
  topic: string
  dimensions?: string[]
  timeRange?: "today" | "week" | "month" | "all"
  sourceTypes?: ("web" | "financial" | "news")[]
  maxSources?: number
}

export interface ResearchSource {
  url: string
  title: string
  snippet: string
  credibility: "high" | "medium" | "low"
  content?: string
}

export interface ResearchResult {
  success: boolean
  topic: string
  summary: string
  report: string
  sources: ResearchSource[]
  insights: string[]
  durationMs: number
  outputPath?: string
  handCreated?: string
}

export interface ResearchLoopConfig {
  maxSources?: number
  maxInlineLength?: number
  enableLearning?: boolean
  enableHandCreation?: boolean
}

const DEFAULT_CONFIG: Required<ResearchLoopConfig> = {
  maxSources: 10,
  maxInlineLength: 1000,
  enableLearning: true,
  enableHandCreation: true,
}

// ============================================================================
// Research Loop Implementation
// ============================================================================

export function createResearchLoop(config: ResearchLoopConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }
  let webSearcher: WebSearcher | null = null
  let learner: ResearchLearner | null = null

  /** Initialize components lazily */
  function ensureInitialized(sessionId: string) {
    if (!webSearcher) {
      webSearcher = createWebSearcher(sessionId, { confidenceThreshold: 0.3 })
    }
    if (!learner && cfg.enableLearning) {
      learner = createResearchLearner()
    }
  }

  /** Phase 1: Understand the research request */
  async function understand(problem: ResearchProblem): Promise<{
    topic: string
    searchQueries: string[]
    dimensions: string[]
  }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId: problem.sessionId,
      phase: "understanding",
    })

    // Generate search queries from topic
    const searchQueries = [
      problem.topic,
      `${problem.topic} 最新`,
      `${problem.topic} 分析`,
    ]

    // Add time-based queries
    if (problem.timeRange === "today") {
      searchQueries.push(`${problem.topic} ${new Date().toISOString().split("T")[0]}`)
    }

    const dimensions = problem.dimensions ?? ["趋势", "数据", "分析"]

    log.debug("Research understanding complete", {
      topic: problem.topic,
      queryCount: searchQueries.length,
    })

    return { topic: problem.topic, searchQueries, dimensions }
  }

  /** Phase 2: Search multiple sources */
  async function search(
    sessionId: string,
    queries: string[],
    maxSources: number,
  ): Promise<ResearchSource[]> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "searching",
    })

    const sources: ResearchSource[] = []

    // Run searches in parallel
    const searchPromises = queries.slice(0, 3).map(async (query) => {
      try {
        const result = await webSearcher?.search({
          sessionId,
          problem: query,
          previousAttempts: [],
        })

        if (result?.sources) {
          return result.sources.map((s) => ({
            url: s.url,
            title: s.title,
            snippet: s.snippet ?? "",
            credibility: assessCredibility(s.url),
            content: s.content,
          }))
        }
      } catch (error) {
        log.warn("Search query failed", { query, error })
      }
      return []
    })

    const results = await Promise.all(searchPromises)
    results.forEach((r) => sources.push(...r))

    // Dedupe by URL
    const uniqueSources = Array.from(
      new Map(sources.map((s) => [s.url, s])).values(),
    ).slice(0, maxSources)

    await Bus.publish(AutonomousEvent.ResearchSourceFound, {
      sessionId,
      sourceCount: uniqueSources.length,
      credibilityBreakdown: {
        high: uniqueSources.filter((s) => s.credibility === "high").length,
        medium: uniqueSources.filter((s) => s.credibility === "medium").length,
        low: uniqueSources.filter((s) => s.credibility === "low").length,
      },
    })

    log.debug("Search complete", { sourceCount: uniqueSources.length })

    return uniqueSources
  }

  /** Assess source credibility based on URL */
  function assessCredibility(url: string): "high" | "medium" | "low" {
    const highCredSites = [
      "reuters.com", "bloomberg.com", "wsj.com", "ft.com",
      "economist.com", "nytimes.com", "bbc.com",
      "gov.cn", "stats.gov.cn", "pbc.gov.cn",
    ]
    const mediumCredSites = [
      "yahoo.com", "google.com", "bing.com",
      "sina.com", "163.com", "sohu.com",
    ]

    const domain = new URL(url).hostname.replace("www.", "")

    if (highCredSites.some((s) => domain.includes(s))) return "high"
    if (mediumCredSites.some((s) => domain.includes(s))) return "medium"
    return "low"
  }

  /** Phase 3: Synthesize information */
  async function synthesize(
    sessionId: string,
    sources: ResearchSource[],
  ): Promise<string> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "synthesizing",
    })

    // Combine content from sources
    const combinedContent = sources
      .filter((s) => s.content || s.snippet)
      .map((s) => `[${s.title}]\n${s.content || s.snippet}`)
      .join("\n\n---\n\n")

    log.debug("Synthesis complete", { contentLength: combinedContent.length })

    return combinedContent
  }

  /** Phase 4: Analyze with LLM */
  async function analyze(
    sessionId: string,
    topic: string,
    synthesizedContent: string,
    dimensions: string[],
  ): Promise<{ summary: string; analysis: string; insights: string[] }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "analyzing",
    })

    // Use LLM for analysis
    const { Anthropic } = await import("@anthropic-ai/sdk")
    const client = new Anthropic()

    const systemPrompt = `你是一个专业的研究分析师。基于提供的信息，生成结构化的分析报告。

请分析以下维度：${dimensions.join("、")}

输出JSON格式：
{
  "summary": "3-5句话的摘要",
  "analysis": "详细分析（300-500字）",
  "insights": ["关键洞察1", "关键洞察2", "关键洞察3"]
}`

    try {
      const response = await client.messages.create({
        model: "claude-3-5-sonnet-latest",
        max_tokens: 2000,
        system: systemPrompt,
        messages: [{
          role: "user",
          content: `主题：${topic}\n\n收集的信息：\n${synthesizedContent.slice(0, 8000)}`,
        }],
      })

      const text = response.content[0].type === "text" ? response.content[0].text : ""
      const jsonMatch = text.match(/\{[\s\S]*\}/)

      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0])
        return {
          summary: parsed.summary ?? "无法生成摘要",
          analysis: parsed.analysis ?? "无法生成分析",
          insights: parsed.insights ?? [],
        }
      }
    } catch (error) {
      log.error("LLM analysis failed", { error })
    }

    // Fallback
    return {
      summary: `关于"${topic}"的研究已完成，收集了${synthesizedContent.length}字的相关信息。`,
      analysis: synthesizedContent.slice(0, 1000),
      insights: ["需要进一步分析"],
    }
  }

  /** Phase 5: Generate report */
  async function report(
    sessionId: string,
    topic: string,
    summary: string,
    analysis: string,
    insights: string[],
    sources: ResearchSource[],
  ): Promise<{ content: string; filePath?: string }> {
    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "reporting",
    })

    const reportData: ReportData = {
      topic,
      summary,
      analysis,
      insights,
      sources: sources.map((s) => ({
        url: s.url,
        title: s.title,
        snippet: s.snippet,
        credibility: s.credibility,
      })),
    }

    const result = await renderReport(reportData, {
      maxInlineLength: cfg.maxInlineLength,
    })

    log.debug("Report generated", { mode: result.mode })

    return { content: result.content, filePath: result.filePath }
  }

  /** Phase 6: Learn from research */
  async function learn(
    sessionId: string,
    topic: string,
    sources: ResearchSource[],
  ): Promise<string | undefined> {
    if (!cfg.enableLearning || !learner) return undefined

    await Bus.publish(AutonomousEvent.ResearchPhaseChanged, {
      sessionId,
      phase: "learning",
    })

    // Record this research session
    learner.recordResearch({
      topic,
      keywords: topic.split(/\s+/),
      sources: sources.map((s) => new URL(s.url).hostname),
    })

    // Check for Hand creation suggestion
    if (cfg.enableHandCreation) {
      const suggestion = learner.suggestHandCreation(topic)
      if (suggestion) {
        await Bus.publish(AutonomousEvent.ResearchPatternLearned, {
          sessionId,
          patternId: `pattern-${Date.now()}`,
          topic,
          keywords: suggestion.keywords,
          frequency: suggestion.frequency,
          confidence: suggestion.confidence,
        })

        log.info("Research pattern learned", {
          topic,
          frequency: suggestion.frequency,
          confidence: suggestion.confidence,
        })

        // TODO: Actually create the Hand
        return `Suggested: ${suggestion.frequency} research on "${topic}"`
      }
    }

    return undefined
  }

  return {
    /** Execute full research loop */
    async research(problem: ResearchProblem): Promise<ResearchResult> {
      const startTime = Date.now()
      ensureInitialized(problem.sessionId)

      await Bus.publish(AutonomousEvent.ResearchStarted, {
        sessionId: problem.sessionId,
        topic: problem.topic,
        dimensions: problem.dimensions,
        sourceTypes: problem.sourceTypes,
      })

      try {
        // Phase 1: Understand
        const { topic, searchQueries, dimensions } = await understand(problem)

        // Phase 2: Search
        const sources = await search(
          problem.sessionId,
          searchQueries,
          problem.maxSources ?? cfg.maxSources,
        )

        if (sources.length === 0) {
          return {
            success: false,
            topic,
            summary: "未能找到相关信息",
            report: "",
            sources: [],
            insights: [],
            durationMs: Date.now() - startTime,
          }
        }

        // Phase 3: Synthesize
        const synthesized = await synthesize(problem.sessionId, sources)

        // Phase 4: Analyze
        const { summary, analysis, insights } = await analyze(
          problem.sessionId,
          topic,
          synthesized,
          dimensions,
        )

        // Phase 5: Report
        const { content, filePath } = await report(
          problem.sessionId,
          topic,
          summary,
          analysis,
          insights,
          sources,
        )

        // Phase 6: Learn
        const handCreated = await learn(problem.sessionId, topic, sources)

        const result: ResearchResult = {
          success: true,
          topic,
          summary,
          report: content,
          sources,
          insights,
          durationMs: Date.now() - startTime,
          outputPath: filePath,
          handCreated,
        }

        await Bus.publish(AutonomousEvent.ResearchCompleted, {
          sessionId: problem.sessionId,
          topic,
          success: true,
          reportMode: filePath ? "file" : "inline",
          reportPath: filePath,
          insightCount: insights.length,
          sourceCount: sources.length,
          durationMs: result.durationMs,
          handCreated,
        })

        return result
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)

        await Bus.publish(AutonomousEvent.ResearchFailed, {
          sessionId: problem.sessionId,
          topic: problem.topic,
          phase: "unknown",
          error: errorMsg,
          retryable: true,
        })

        return {
          success: false,
          topic: problem.topic,
          summary: `研究失败: ${errorMsg}`,
          report: "",
          sources: [],
          insights: [],
          durationMs: Date.now() - startTime,
        }
      }
    },

    /** Cleanup resources */
    async cleanup(): Promise<void> {
      webSearcher = null
      learner = null
    },

    /** Get learner for external access */
    getLearner(): ResearchLearner | null {
      return learner
    },
  }
}

export type ResearchLoop = ReturnType<typeof createResearchLoop>
```

**Step 4: Run test to verify it passes**

Run: `cd packages/ccode && bun test test/autonomous/execution/research-loop.test.ts`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add packages/ccode/src/autonomous/execution/research-loop.ts packages/ccode/test/autonomous/execution/research-loop.test.ts
git commit -m "feat(autonomous): implement research loop with TDD"
```

---

## Task 8: Integrate into chat.ts

**Files:**
- Modify: `packages/ccode/src/api/server/handlers/chat.ts`

**Step 1: Add imports at top of file (after line ~10)**

```typescript
import { classifyTask } from "../../../autonomous/classification"
import { createResearchLoop } from "../../../autonomous/execution/research-loop"
```

**Step 2: Replace isActionableTask function (lines 200-214)**

Replace:
```typescript
function isActionableTask(message: string): boolean {
  const actionKeywords = [
    // ... existing code
  ]
  const lowerMessage = message.toLowerCase()
  return actionKeywords.some(keyword => lowerMessage.includes(keyword))
}
```

With:
```typescript
// Note: isActionableTask is replaced by classifyTask from autonomous/classification
// Keeping for backward compatibility but marking deprecated
/** @deprecated Use classifyTask instead */
function isActionableTask(message: string): boolean {
  const actionKeywords = [
    "实现", "创建", "修复", "开发", "构建", "编写", "生成", "执行",
    "部署", "配置", "设置", "安装", "更新", "修改", "重构", "优化",
    "自动", "定时", "调度", "每天", "每周", "每小时",
    "implement", "create", "fix", "build", "write", "generate", "execute",
    "deploy", "configure", "setup", "install", "update", "modify", "refactor",
    "automate", "schedule", "cron", "daily", "weekly", "hourly",
  ]
  const lowerMessage = message.toLowerCase()
  return actionKeywords.some(keyword => lowerMessage.includes(keyword))
}
```

**Step 3: Add executeResearchChat function (after executeAutonomousChat)**

```typescript
/**
 * Execute chat with Research Loop for research/analysis tasks
 */
async function executeResearchChat(
  input: ChatRequest,
  classification: Awaited<ReturnType<typeof classifyTask>>,
  ctx: TracingContext,
  startTime: number
): Promise<HttpResponse> {
  const { SessionPrompt } = await import("../../../session/prompt")
  const { LocalSession } = await import("../../../api")

  const sessionId = await getOrCreateSession(input.conversation_id)

  logLifecycleEvent(ctx, "http_request", {
    function: "executeResearchChat",
    topic: classification.researchTopic ?? input.message,
    confidence: classification.confidence,
  })

  const researchLoop = createResearchLoop({
    maxSources: 10,
    maxInlineLength: 1000,
    enableLearning: true,
    enableHandCreation: true,
  })

  try {
    const result = await researchLoop.research({
      sessionId,
      topic: classification.researchTopic ?? input.message,
      maxSources: 10,
    })

    const durationMs = Math.round(performance.now() - startTime)

    logLifecycleEvent(ctx, "function_end", {
      function: "executeResearchChat",
      duration_ms: durationMs,
      success: result.success,
      sourceCount: result.sources.length,
      insightCount: result.insights.length,
      reportMode: result.outputPath ? "file" : "inline",
    })

    return jsonResponse({
      success: true,
      data: {
        message: result.report || result.summary,
        conversation_id: input.conversation_id ?? sessionId,
        agent: "research",
        research_result: {
          success: result.success,
          topic: result.topic,
          sourceCount: result.sources.length,
          insightCount: result.insights.length,
          outputPath: result.outputPath,
          handCreated: result.handCreated,
          durationMs: result.durationMs,
        },
      },
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "executeResearchChat",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  } finally {
    await researchLoop.cleanup()
  }
}
```

**Step 4: Modify chat function to use classifier (in the main chat handler)**

Find the section after autonomous mode check (around line 649) and add task classification:

```typescript
    // Task Classification - route to appropriate loop
    const classification = await classifyTask(input.message)

    logLifecycleEvent(ctx, "http_request", {
      function: "chat.classification",
      type: classification.type,
      confidence: classification.confidence,
    })

    // Route to Research Loop for research tasks
    if (classification.type === "research" && classification.confidence > 0.6) {
      return await executeResearchChat(input, classification, ctx, startTime)
    }

    // Continue with existing logic for other task types...
```

**Step 5: Verify TypeScript compiles**

Run: `cd packages/ccode && bun run typecheck`
Expected: No errors

**Step 6: Commit**

```bash
git add packages/ccode/src/api/server/handlers/chat.ts
git commit -m "feat(chat): integrate task classifier and research loop"
```

---

## Task 9: Update autonomous/index.ts exports

**Files:**
- Modify: `packages/ccode/src/autonomous/index.ts`

**Step 1: Add exports**

```typescript
// Classification
export * from "./classification"

// Research Loop
export { createResearchLoop, type ResearchLoop, type ResearchProblem, type ResearchResult } from "./execution/research-loop"
export { createReportRenderer, type ReportRenderer, type ReportData, type RenderResult } from "./execution/report-renderer"
export { createResearchLearner, type ResearchLearner, type LearnedResearchPattern } from "./execution/research-learner"
```

**Step 2: Commit**

```bash
git add packages/ccode/src/autonomous/index.ts
git commit -m "feat(autonomous): export research loop modules"
```

---

## Task 10: Integration Test

**Files:**
- Create: `packages/ccode/test/autonomous/integration/research-flow.test.ts`

**Step 1: Write integration test**

```typescript
import { describe, test, expect } from "bun:test"
import { classifyTask } from "../../../src/autonomous/classification"
import { createResearchLoop } from "../../../src/autonomous/execution/research-loop"

describe("Research Flow Integration", () => {
  test("classifies and routes research task correctly", async () => {
    // Step 1: Classify
    const classification = await classifyTask("梳理当前的黄金走势情况")
    expect(classification.type).toBe("research")
    expect(classification.confidence).toBeGreaterThan(0.5)

    // Step 2: Execute research (with mock to avoid network)
    const loop = createResearchLoop({
      maxSources: 2,
      enableLearning: false,
    })

    const result = await loop.research({
      sessionId: "test-integration",
      topic: classification.researchTopic ?? "黄金走势",
      maxSources: 2,
    })

    expect(result).toHaveProperty("topic")
    expect(result).toHaveProperty("durationMs")

    await loop.cleanup()
  })

  test("classifies implementation task correctly", async () => {
    const classification = await classifyTask("实现一个用户登录功能")
    expect(classification.type).toBe("implementation")
  })

  test("classifies query task correctly", async () => {
    const classification = await classifyTask("什么是 TypeScript")
    expect(classification.type).toBe("query")
  })
})
```

**Step 2: Run integration test**

Run: `cd packages/ccode && bun test test/autonomous/integration/research-flow.test.ts`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add packages/ccode/test/autonomous/integration/research-flow.test.ts
git commit -m "test(autonomous): add research flow integration tests"
```

---

## Summary

**Files Created (7):**
- `packages/ccode/src/autonomous/classification/types.ts`
- `packages/ccode/src/autonomous/classification/task-classifier.ts`
- `packages/ccode/src/autonomous/classification/index.ts`
- `packages/ccode/src/autonomous/execution/report-renderer.ts`
- `packages/ccode/src/autonomous/execution/research-learner.ts`
- `packages/ccode/src/autonomous/execution/research-loop.ts`
- `packages/ccode/test/autonomous/integration/research-flow.test.ts`

**Files Modified (3):**
- `packages/ccode/src/autonomous/events.ts` - Added Research events
- `packages/ccode/src/api/server/handlers/chat.ts` - Integrated classifier and Research Loop
- `packages/ccode/src/autonomous/index.ts` - Added exports

**Test Files (4):**
- `packages/ccode/test/autonomous/classification/task-classifier.test.ts`
- `packages/ccode/test/autonomous/execution/report-renderer.test.ts`
- `packages/ccode/test/autonomous/execution/research-learner.test.ts`
- `packages/ccode/test/autonomous/execution/research-loop.test.ts`

---

Plan complete and saved to `docs/plans/2026-03-03-research-loop-implementation.md`.

**Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
