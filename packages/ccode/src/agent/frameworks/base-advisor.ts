/**
 * BasePhilosophyAdvisor Framework
 *
 * Shared 祝融说 (Zhurong Philosophy) concepts for decision/observer agents.
 * Provides CLOSE framework scoring, decision analysis, and philosophical concepts.
 *
 * Core Philosophy:
 * - 可能性基底 (Possibility Substrate): Universe as infinite potential field
 * - 观察即收敛 (Observation as Convergence): Observation collapses possibilities
 * - 可用余量 (Available Margin): Unfixed potential space, source of free will
 * - 可持续决策 > 最优决策: Sustainable > Optimal ("再来一次" capability)
 *
 * @module agent/frameworks/base-advisor
 */

import z from "zod"

// ─────────────────────────────────────────────────────────────────────────────
// CLOSE Framework Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * CLOSE dimension with score and context.
 *
 * Each dimension is scored 0-10 with supporting factors.
 */
export const CLOSEDimension = z.object({
  /** Score from 0-10 */
  score: z.number().min(0).max(10),
  /** Confidence in this score (0-1) */
  confidence: z.number().min(0).max(1),
  /** Factors that contributed to this score */
  factors: z.array(z.string()),
  /** Human-readable assessment */
  assessment: z.string().optional(),
})
export type CLOSEDimension = z.infer<typeof CLOSEDimension>

/**
 * Complete CLOSE score with all five dimensions.
 *
 * CLOSE stands for:
 * - Convergence: How much does this narrow possibilities?
 * - Leverage: Asymmetric upside potential?
 * - Optionality: Can you reverse/adjust? (Most weighted - key to "再来一次")
 * - Surplus: How much buffer remains?
 * - Evolution: Does this enable future growth?
 */
export const CLOSEScore = z.object({
  /** 收敛度 - Convergence: How much does this narrow possibilities? */
  convergence: CLOSEDimension,
  /** 杠杆率 - Leverage: Asymmetric upside potential? */
  leverage: CLOSEDimension,
  /** 选择权 - Optionality: Can you reverse/adjust? (Key dimension) */
  optionality: CLOSEDimension,
  /** 余量 - Surplus: How much buffer remains? */
  surplus: CLOSEDimension,
  /** 进化 - Evolution: Does this enable future growth? */
  evolution: CLOSEDimension,
  /** Weighted total score (0-10) */
  total: z.number().min(0).max(10),
  /** Computed risk score (0-10, higher = riskier) */
  risk: z.number().min(0).max(10),
  /** Overall confidence (0-1) */
  confidence: z.number().min(0).max(1),
})
export type CLOSEScore = z.infer<typeof CLOSEScore>

/**
 * CLOSE dimension weights for scoring.
 */
export const CLOSEWeights = z.object({
  convergence: z.number().default(1.0),
  leverage: z.number().default(1.2),
  optionality: z.number().default(1.5), // Highest weight: "再来一次" is key
  surplus: z.number().default(1.3),
  evolution: z.number().default(0.8),
})
export type CLOSEWeights = z.infer<typeof CLOSEWeights>

export const DEFAULT_CLOSE_WEIGHTS: CLOSEWeights = {
  convergence: 1.0,
  leverage: 1.2,
  optionality: 1.5,
  surplus: 1.3,
  evolution: 0.8,
}

// ─────────────────────────────────────────────────────────────────────────────
// Decision Analysis Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Scenario analysis for a decision.
 */
export const ScenarioAnalysis = z.object({
  /** Best case outcome */
  best: z.object({
    description: z.string(),
    probability: z.number().min(0).max(1),
    impact: z.string(),
  }),
  /** Worst case outcome */
  worst: z.object({
    description: z.string(),
    probability: z.number().min(0).max(1),
    impact: z.string(),
  }),
  /** Most likely outcome */
  likely: z.object({
    description: z.string(),
    probability: z.number().min(0).max(1),
    impact: z.string(),
  }),
})
export type ScenarioAnalysis = z.infer<typeof ScenarioAnalysis>

/**
 * Option in a decision.
 */
export const DecisionOption = z.object({
  /** Option identifier */
  id: z.string(),
  /** Option description */
  description: z.string(),
  /** CLOSE score for this option */
  closeScore: CLOSEScore,
  /** Scenario analysis */
  scenarios: ScenarioAnalysis,
  /** Pros of this option */
  pros: z.array(z.string()),
  /** Cons of this option */
  cons: z.array(z.string()),
  /** Prerequisites required */
  prerequisites: z.array(z.string()).default([]),
  /** Time sensitivity */
  timeSensitivity: z.enum(["immediate", "short", "medium", "long", "flexible"]),
  /** Reversibility */
  reversibility: z.enum(["irreversible", "partially-reversible", "reversible"]),
})
export type DecisionOption = z.infer<typeof DecisionOption>

/**
 * Surplus protection strategies.
 */
export const SurplusProtection = z.object({
  /** Protection strategy description */
  strategy: z.string(),
  /** How this protects surplus */
  mechanism: z.string(),
  /** Cost of implementing */
  cost: z.enum(["low", "medium", "high"]),
  /** Effectiveness rating (0-1) */
  effectiveness: z.number().min(0).max(1),
})
export type SurplusProtection = z.infer<typeof SurplusProtection>

/**
 * Complete decision analysis.
 */
export const DecisionAnalysis = z.object({
  /** Analysis ID */
  id: z.string(),
  /** Analysis timestamp */
  timestamp: z.date(),
  /** Decision title/question */
  title: z.string(),
  /** Executive summary */
  summary: z.string(),
  /** Context and background */
  context: z.string(),
  /** Available options */
  options: z.array(DecisionOption),
  /** Recommended option ID */
  recommendedOption: z.string().optional(),
  /** Recommendation rationale */
  rationale: z.string().optional(),
  /** Surplus protection strategies */
  surplusProtection: z.array(SurplusProtection).default([]),
  /** Key uncertainties */
  uncertainties: z.array(z.string()).default([]),
  /** Information needed for better decision */
  informationNeeded: z.array(z.string()).default([]),
  /** Overall confidence in analysis (0-1) */
  confidence: z.number().min(0).max(1),
})
export type DecisionAnalysis = z.infer<typeof DecisionAnalysis>

// ─────────────────────────────────────────────────────────────────────────────
// Philosophy Concepts Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 可能性空间 (Possibility Space) - The state of potential before observation.
 */
export const PossibilitySpace = z.object({
  /** Description of the possibility space */
  description: z.string(),
  /** Estimated entropy (0-1, higher = more possibilities) */
  entropy: z.number().min(0).max(1),
  /** Known constraints that reduce possibilities */
  constraints: z.array(z.string()).default([]),
  /** Potential paths identified */
  potentialPaths: z.array(z.string()).default([]),
})
export type PossibilitySpace = z.infer<typeof PossibilitySpace>

/**
 * 观察记录 (Observation Record) - What was observed and its effect.
 */
export const ObservationRecord = z.object({
  /** What was observed */
  observation: z.string(),
  /** When observed */
  timestamp: z.date(),
  /** How it narrowed possibilities */
  convergenceEffect: z.string(),
  /** Possibilities eliminated */
  eliminated: z.array(z.string()).default([]),
  /** Possibilities revealed */
  revealed: z.array(z.string()).default([]),
  /** Was this observation irreversible? */
  irreversible: z.boolean(),
})
export type ObservationRecord = z.infer<typeof ObservationRecord>

/**
 * 余量评估 (Margin Assessment) - Current available margin.
 */
export const MarginAssessment = z.object({
  /** Type of margin */
  type: z.enum(["time", "resources", "options", "energy", "attention", "relationships"]),
  /** Current level (0-10) */
  level: z.number().min(0).max(10),
  /** Burn rate (how fast is it depleting) */
  burnRate: z.enum(["fast", "moderate", "slow", "stable", "recovering"]),
  /** What's consuming this margin */
  drains: z.array(z.string()).default([]),
  /** What's replenishing this margin */
  sources: z.array(z.string()).default([]),
  /** Recommendations for this margin type */
  recommendations: z.array(z.string()).default([]),
})
export type MarginAssessment = z.infer<typeof MarginAssessment>

// ─────────────────────────────────────────────────────────────────────────────
// Advisor Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Advisor focus mode.
 */
export const AdvisorFocusMode = z.enum([
  "theoretical", // Observer: philosophical analysis
  "practical", // Decision: actionable advice
  "reflective", // Both: introspection and review
])
export type AdvisorFocusMode = z.infer<typeof AdvisorFocusMode>

/**
 * Output style.
 */
export const AdvisorOutputStyle = z.enum([
  "reflective", // Contemplative, open-ended
  "actionable", // Concrete steps and recommendations
  "analytical", // Data-driven, structured
  "narrative", // Story-form, contextual
])
export type AdvisorOutputStyle = z.infer<typeof AdvisorOutputStyle>

/**
 * Base advisor configuration.
 */
export const BaseAdvisorConfig = z.object({
  /** Focus mode */
  focusMode: AdvisorFocusMode,
  /** Output style */
  outputStyle: AdvisorOutputStyle,
  /** CLOSE dimension weights */
  closeWeights: CLOSEWeights.default(DEFAULT_CLOSE_WEIGHTS),
  /** Minimum score to recommend an option */
  recommendationThreshold: z.number().min(0).max(10).default(6),
  /** Include philosophical context */
  includePhilosophy: z.boolean().default(true),
  /** Language for output (zh/en) */
  language: z.enum(["zh", "en"]).default("zh"),
})
export type BaseAdvisorConfig = z.infer<typeof BaseAdvisorConfig>

// ─────────────────────────────────────────────────────────────────────────────
// Utility Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Calculate weighted CLOSE total from dimensions.
 */
export function calculateCLOSETotal(
  dimensions: Omit<CLOSEScore, "total" | "risk" | "confidence">,
  weights: CLOSEWeights = DEFAULT_CLOSE_WEIGHTS,
): number {
  const weightSum = weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution

  const rawTotal =
    dimensions.convergence.score * weights.convergence +
    dimensions.leverage.score * weights.leverage +
    dimensions.optionality.score * weights.optionality +
    dimensions.surplus.score * weights.surplus +
    dimensions.evolution.score * weights.evolution

  return Math.round((rawTotal / weightSum) * 100) / 100
}

/**
 * Calculate risk from CLOSE dimensions.
 *
 * Risk is high when:
 * - Convergence is low (disagreement/uncertainty)
 * - Optionality is low (few choices, can't "再来一次")
 * - Surplus is low (no margin for error)
 */
export function calculateCLOSERisk(
  dimensions: Omit<CLOSEScore, "total" | "risk" | "confidence">,
): number {
  let risk = 5 // Neutral starting point

  // Low convergence = high uncertainty risk
  risk += (10 - dimensions.convergence.score) * 0.3

  // Low optionality = highest risk factor (can't "再来一次")
  risk += (10 - dimensions.optionality.score) * 0.4

  // Low surplus = high exposure risk
  risk += (10 - dimensions.surplus.score) * 0.3

  return Math.max(0, Math.min(10, Math.round(risk * 100) / 100))
}

/**
 * Calculate overall confidence from dimension confidences.
 */
export function calculateCLOSEConfidence(
  dimensions: Omit<CLOSEScore, "total" | "risk" | "confidence">,
): number {
  const confidences = [
    dimensions.convergence.confidence,
    dimensions.leverage.confidence,
    dimensions.optionality.confidence,
    dimensions.surplus.confidence,
    dimensions.evolution.confidence,
  ]
  return Math.round((confidences.reduce((sum, c) => sum + c, 0) / confidences.length) * 100) / 100
}

/**
 * Build complete CLOSE score from dimension scores.
 */
export function buildCLOSEScore(
  dimensions: Omit<CLOSEScore, "total" | "risk" | "confidence">,
  weights: CLOSEWeights = DEFAULT_CLOSE_WEIGHTS,
): CLOSEScore {
  return {
    ...dimensions,
    total: calculateCLOSETotal(dimensions, weights),
    risk: calculateCLOSERisk(dimensions),
    confidence: calculateCLOSEConfidence(dimensions),
  }
}

/**
 * Quick CLOSE score from simple numeric inputs.
 */
export function quickCLOSEScore(
  scores: {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
  },
  confidence: number = 0.8,
): CLOSEScore {
  const makeDimension = (score: number, name: string): CLOSEDimension => ({
    score,
    confidence,
    factors: [],
    assessment: `${name}: ${score}/10`,
  })

  return buildCLOSEScore({
    convergence: makeDimension(scores.convergence, "Convergence"),
    leverage: makeDimension(scores.leverage, "Leverage"),
    optionality: makeDimension(scores.optionality, "Optionality"),
    surplus: makeDimension(scores.surplus, "Surplus"),
    evolution: makeDimension(scores.evolution, "Evolution"),
  })
}

/**
 * Format CLOSE score as markdown.
 */
export function formatCLOSEMarkdown(score: CLOSEScore, language: "zh" | "en" = "zh"): string {
  const labels = language === "zh"
    ? {
        convergence: "收敛度",
        leverage: "杠杆率",
        optionality: "选择权",
        surplus: "余量",
        evolution: "进化",
        total: "总分",
        risk: "风险",
        confidence: "置信度",
      }
    : {
        convergence: "Convergence",
        leverage: "Leverage",
        optionality: "Optionality",
        surplus: "Surplus",
        evolution: "Evolution",
        total: "Total",
        risk: "Risk",
        confidence: "Confidence",
      }

  const bar = (val: number, max: number = 10) => {
    const filled = Math.round((val / max) * 10)
    return "[" + "=".repeat(filled) + " ".repeat(10 - filled) + "]"
  }

  return `### CLOSE ${language === "zh" ? "评估" : "Assessment"}

| ${language === "zh" ? "维度" : "Dimension"} | ${language === "zh" ? "分数" : "Score"} | ${language === "zh" ? "图示" : "Visual"} |
|--------|-------|--------|
| **${labels.convergence}** | ${score.convergence.score}/10 | ${bar(score.convergence.score)} |
| **${labels.leverage}** | ${score.leverage.score}/10 | ${bar(score.leverage.score)} |
| **${labels.optionality}** | ${score.optionality.score}/10 | ${bar(score.optionality.score)} |
| **${labels.surplus}** | ${score.surplus.score}/10 | ${bar(score.surplus.score)} |
| **${labels.evolution}** | ${score.evolution.score}/10 | ${bar(score.evolution.score)} |

**${labels.total}**: ${score.total}/10 | **${labels.risk}**: ${score.risk}/10 | **${labels.confidence}**: ${Math.round(score.confidence * 100)}%
`
}

/**
 * Format decision analysis as markdown.
 */
export function formatDecisionMarkdown(analysis: DecisionAnalysis, language: "zh" | "en" = "zh"): string {
  const header = language === "zh" ? "决策分析" : "Decision Analysis"

  const optionsSections = analysis.options
    .map((opt, i) => {
      const scoreEmoji = opt.closeScore.total >= 7 ? "[HIGH]" : opt.closeScore.total >= 5 ? "[MEDIUM]" : "[LOW]"
      const reversibilityEmoji =
        opt.reversibility === "irreversible"
          ? "[IRREVERSIBLE]"
          : opt.reversibility === "partially-reversible"
            ? "[PARTIAL]"
            : "[REVERSIBLE]"

      return `#### ${language === "zh" ? "选项" : "Option"} ${i + 1}: ${opt.description}

${scoreEmoji} CLOSE: ${opt.closeScore.total}/10 | ${reversibilityEmoji}

**${language === "zh" ? "优势" : "Pros"}**:
${opt.pros.map((p) => `- ${p}`).join("\n")}

**${language === "zh" ? "劣势" : "Cons"}**:
${opt.cons.map((c) => `- ${c}`).join("\n")}
`
    })
    .join("\n")

  const recommendation =
    analysis.recommendedOption && analysis.rationale
      ? `### ${language === "zh" ? "建议" : "Recommendation"}

${language === "zh" ? "推荐选项" : "Recommended"}: **${analysis.options.find((o) => o.id === analysis.recommendedOption)?.description ?? analysis.recommendedOption}**

${analysis.rationale}
`
      : ""

  return `## ${header}: ${analysis.title}

### ${language === "zh" ? "背景" : "Context"}
${analysis.context}

### ${language === "zh" ? "摘要" : "Summary"}
${analysis.summary}

${optionsSections}

${recommendation}

${
    analysis.surplusProtection.length > 0
      ? `### ${language === "zh" ? "余量保护策略" : "Surplus Protection"}
${analysis.surplusProtection.map((s) => `- **${s.strategy}**: ${s.mechanism}`).join("\n")}
`
      : ""
  }

${
    analysis.uncertainties.length > 0
      ? `### ${language === "zh" ? "不确定性" : "Uncertainties"}
${analysis.uncertainties.map((u) => `- ${u}`).join("\n")}
`
      : ""
  }

---
_${language === "zh" ? "置信度" : "Confidence"}: ${Math.round(analysis.confidence * 100)}%_
`
}

/**
 * Generate analysis ID.
 */
export function generateAnalysisId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 6)
  return `decision_${timestamp}_${random}`
}

/**
 * Core philosophical principles for reference.
 */
export const PHILOSOPHY_PRINCIPLES = {
  zh: {
    possibilitySubstrate: "可能性基底：宇宙的终极实在是包含一切潜能的无限场域",
    observationConvergence: "观察即收敛：观察是创造性行为，导致可能性「坍缩」为确定性",
    availableMargin: "可用余量：尚未被固化的潜能空间，是自由意志和创造力的来源",
    sustainableDecision: "可持续决策 > 最优决策：保持「再来一次」的能力比追求「最优解」更重要",
  },
  en: {
    possibilitySubstrate: "Possibility Substrate: The ultimate reality is an infinite field containing all potentials",
    observationConvergence: "Observation as Convergence: Observation is a creative act that collapses possibilities into certainty",
    availableMargin: "Available Margin: Unfixed potential space, the source of free will and creativity",
    sustainableDecision: "Sustainable > Optimal: Maintaining the ability to \"try again\" is more important than pursuing the \"optimal solution\"",
  },
}
