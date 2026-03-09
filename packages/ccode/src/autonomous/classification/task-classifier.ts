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
  DECISION_KEYWORDS,
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
  const decisionCount = countKeywordMatches(message, DECISION_KEYWORDS)
  const queryCount = countKeywordMatches(message, QUERY_KEYWORDS)

  const total = researchCount + implementationCount + decisionCount + queryCount

  if (total === 0) {
    return { type: "other", confidence: 0.3 }
  }

  const scores = [
    { type: "research" as const, count: researchCount },
    { type: "implementation" as const, count: implementationCount },
    { type: "decision" as const, count: decisionCount },
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
  const { generateObject } = await import("ai")
  const { Provider } = await import("@/provider/provider")
  const z = await import("zod").then((m) => m.default)

  const systemPrompt = `You are a task classifier. Classify the user's request into one of these types:
- implementation: Code writing, feature creation, bug fixes, deployment
- research: Information gathering, analysis, trend research, data synthesis
- decision: Choices, trade-offs, evaluations, career/investment decisions, CLOSE framework
- query: Simple questions, explanations, definitions
- other: Anything else`

  try {
    const defaultModel = await Provider.defaultModel()
    const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
    const language = await Provider.getLanguage(model)

    const result = await generateObject({
      temperature: 0.1,
      messages: [
        { role: "system", content: systemPrompt },
        { role: "user", content: message },
      ],
      model: language,
      schema: z.object({
        type: z.enum(["implementation", "research", "decision", "query", "other"]),
        confidence: z.number(),
        reasoning: z.string(),
        researchTopic: z.string().optional(),
      }),
    })

    return ClassificationResultSchema.parse(result.object)
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
