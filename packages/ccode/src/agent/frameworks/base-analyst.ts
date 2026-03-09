/**
 * BaseMarketAnalyst Framework
 *
 * Shared structure for market/economic analysis agents (macro, trader, value-analyst).
 * Provides consistent data structures, cycle analysis, and report formats.
 *
 * @module agent/frameworks/base-analyst
 */

import z from "zod"

// ─────────────────────────────────────────────────────────────────────────────
// Time & Cycle Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Time horizon for analysis.
 */
export const TimeHorizon = z.enum([
  "intraday", // trader: minutes to hours
  "short", // trader: days to weeks
  "medium", // macro: weeks to months
  "long", // macro: months to years
])
export type TimeHorizon = z.infer<typeof TimeHorizon>

/**
 * Data frequency.
 */
export const DataFrequency = z.enum(["realtime", "minute", "hourly", "daily", "weekly", "monthly"])
export type DataFrequency = z.infer<typeof DataFrequency>

/**
 * Economic/market cycle phase.
 */
export const CyclePhase = z.enum(["expansion", "peak", "contraction", "trough", "recovery"])
export type CyclePhase = z.infer<typeof CyclePhase>

/**
 * Position within cycle phase.
 */
export const CyclePosition = z.enum(["early", "mid", "late"])
export type CyclePosition = z.infer<typeof CyclePosition>

/**
 * Trend direction.
 */
export const TrendDirection = z.enum(["up", "flat", "down"])
export type TrendDirection = z.infer<typeof TrendDirection>

/**
 * Current state assessment.
 */
export const CurrentState = z.object({
  /** Current cycle phase */
  cycle: CyclePhase,
  /** Position within cycle */
  position: CyclePosition,
  /** Overall trend */
  trend: TrendDirection,
  /** Confidence in this assessment (0-1) */
  confidence: z.number().min(0).max(1),
  /** Key supporting factors */
  factors: z.array(z.string()).default([]),
})
export type CurrentState = z.infer<typeof CurrentState>

// ─────────────────────────────────────────────────────────────────────────────
// Market Data Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Single data point/indicator.
 */
export const Indicator = z.object({
  /** Indicator name (e.g., "PMI", "RSI", "GDP Growth") */
  name: z.string(),
  /** Current value */
  value: z.union([z.number(), z.string()]),
  /** Previous value for comparison */
  previousValue: z.union([z.number(), z.string()]).optional(),
  /** Change from previous */
  change: z.number().optional(),
  /** Change percentage */
  changePercent: z.number().optional(),
  /** Unit of measurement */
  unit: z.string().optional(),
  /** Expected value (for economic releases) */
  expected: z.union([z.number(), z.string()]).optional(),
  /** Interpretation of the value */
  interpretation: z.enum(["bullish", "bearish", "neutral", "mixed"]).optional(),
})
export type Indicator = z.infer<typeof Indicator>

/**
 * Market data bundle.
 */
export const MarketData = z.object({
  /** Data source identifier */
  source: z.string(),
  /** Data timestamp */
  timestamp: z.date(),
  /** Asset or market being analyzed */
  asset: z.string().optional(),
  /** List of indicators */
  indicators: z.array(Indicator),
  /** Overall confidence in data quality (0-1) */
  confidence: z.number().min(0).max(1),
  /** Data freshness (how recent) */
  freshness: z.enum(["live", "recent", "delayed", "historical"]).default("recent"),
  /** Additional metadata */
  metadata: z.record(z.string(), z.any()).optional(),
})
export type MarketData = z.infer<typeof MarketData>

// ─────────────────────────────────────────────────────────────────────────────
// Causal Chain Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Single link in a causal chain.
 */
export const CausalLink = z.object({
  /** Source event/factor */
  from: z.string(),
  /** Target event/factor */
  to: z.string(),
  /** Relationship type */
  relationship: z.enum(["causes", "enables", "blocks", "influences", "correlates"]),
  /** Strength of relationship (0-1) */
  strength: z.number().min(0).max(1),
  /** Time lag (if applicable) */
  lagDescription: z.string().optional(),
})
export type CausalLink = z.infer<typeof CausalLink>

/**
 * Causal chain describing economic/market relationships.
 */
export const CausalChain = z.object({
  /** Chain title */
  title: z.string(),
  /** Chain description */
  description: z.string(),
  /** Links in the chain */
  links: z.array(CausalLink),
  /** Overall confidence */
  confidence: z.number().min(0).max(1),
})
export type CausalChain = z.infer<typeof CausalChain>

// ─────────────────────────────────────────────────────────────────────────────
// Risk & Recommendation Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Risk assessment.
 */
export const RiskAssessment = z.object({
  /** Risk description */
  description: z.string(),
  /** Probability (0-1) */
  probability: z.number().min(0).max(1),
  /** Impact severity */
  impact: z.enum(["low", "medium", "high", "severe"]),
  /** Time frame */
  timeFrame: TimeHorizon.optional(),
  /** Mitigation suggestions */
  mitigation: z.array(z.string()).default([]),
})
export type RiskAssessment = z.infer<typeof RiskAssessment>

/**
 * Action recommendation.
 */
export const Recommendation = z.object({
  /** Action to take */
  action: z.string(),
  /** Rationale for the recommendation */
  rationale: z.string(),
  /** Priority level */
  priority: z.enum(["low", "medium", "high", "urgent"]),
  /** Time sensitivity */
  timeSensitivity: z.enum(["immediate", "soon", "flexible", "long-term"]),
  /** Confidence in this recommendation (0-1) */
  confidence: z.number().min(0).max(1),
  /** Prerequisites */
  prerequisites: z.array(z.string()).default([]),
})
export type Recommendation = z.infer<typeof Recommendation>

// ─────────────────────────────────────────────────────────────────────────────
// Analysis Report Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Base analysis report structure.
 */
export const BaseAnalysisReport = z.object({
  /** Report ID */
  id: z.string(),
  /** Report timestamp */
  timestamp: z.date(),
  /** Report title */
  title: z.string(),
  /** Executive summary */
  summary: z.string(),
  /** Current state assessment */
  currentState: CurrentState,
  /** Data points used in analysis */
  dataPoints: z.array(MarketData),
  /** Identified risks */
  risks: z.array(RiskAssessment),
  /** Recommendations */
  recommendations: z.array(Recommendation),
  /** Standard risk disclaimer */
  disclaimer: z.string(),
  /** Analysis confidence (0-1) */
  confidence: z.number().min(0).max(1),
})
export type BaseAnalysisReport = z.infer<typeof BaseAnalysisReport>

/**
 * Macro economic analysis report.
 */
export const MacroAnalysisReport = BaseAnalysisReport.extend({
  type: z.literal("macro"),
  /** Time horizon */
  timeHorizon: z.enum(["medium", "long"]),
  /** Economic outlook */
  outlook: z.object({
    gdpForecast: z.string().optional(),
    inflationForecast: z.string().optional(),
    rateOutlook: z.string().optional(),
    employmentOutlook: z.string().optional(),
  }),
  /** Causal chains explaining relationships */
  causalChains: z.array(CausalChain).default([]),
  /** Cross-asset correlations */
  correlations: z.array(z.object({
    asset1: z.string(),
    asset2: z.string(),
    correlation: z.number().min(-1).max(1),
    interpretation: z.string(),
  })).default([]),
})
export type MacroAnalysisReport = z.infer<typeof MacroAnalysisReport>

/**
 * Trading/technical analysis report.
 */
export const TradingAnalysisReport = BaseAnalysisReport.extend({
  type: z.literal("trading"),
  /** Time horizon */
  timeHorizon: z.enum(["intraday", "short"]),
  /** Asset being analyzed */
  asset: z.string(),
  /** Technical levels */
  levels: z.object({
    support: z.array(z.number()).default([]),
    resistance: z.array(z.number()).default([]),
    pivot: z.number().optional(),
  }),
  /** Technical signals */
  signals: z.array(z.object({
    indicator: z.string(),
    signal: z.enum(["buy", "sell", "hold", "neutral"]),
    strength: z.number().min(0).max(1),
  })).default([]),
  /** Trading bias */
  bias: z.enum(["bullish", "bearish", "neutral"]),
  /** Entry/exit suggestions */
  tradePlan: z.object({
    entry: z.number().optional(),
    stopLoss: z.number().optional(),
    takeProfit: z.array(z.number()).default([]),
    positionSize: z.string().optional(),
  }).optional(),
})
export type TradingAnalysisReport = z.infer<typeof TradingAnalysisReport>

/**
 * Value analysis report.
 */
export const ValueAnalysisReport = BaseAnalysisReport.extend({
  type: z.literal("value"),
  /** Time horizon */
  timeHorizon: z.enum(["medium", "long"]),
  /** Asset/entity being analyzed */
  subject: z.string(),
  /** Valuation metrics */
  valuation: z.object({
    intrinsicValue: z.number().optional(),
    currentPrice: z.number().optional(),
    marginOfSafety: z.number().optional(),
    method: z.string(),
  }),
  /** Quality factors */
  qualityFactors: z.array(z.object({
    factor: z.string(),
    score: z.number().min(0).max(10),
    assessment: z.string(),
  })).default([]),
  /** Competitive position */
  competitivePosition: z.string().optional(),
})
export type ValueAnalysisReport = z.infer<typeof ValueAnalysisReport>

// ─────────────────────────────────────────────────────────────────────────────
// Analyst Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Base analyst configuration.
 */
export const BaseAnalystConfig = z.object({
  /** Analysis time horizon */
  timeHorizon: TimeHorizon,
  /** Data frequency preference */
  dataFrequency: DataFrequency,
  /** Risk disclaimer text */
  riskDisclaimer: z.string().default(
    "This analysis is for informational purposes only and should not be considered financial advice. Past performance does not guarantee future results.",
  ),
  /** Confidence threshold for recommendations */
  minConfidence: z.number().min(0).max(1).default(0.6),
  /** Focus areas */
  focusAreas: z.array(z.string()).default([]),
})
export type BaseAnalystConfig = z.infer<typeof BaseAnalystConfig>

// ─────────────────────────────────────────────────────────────────────────────
// Utility Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Calculate overall cycle state from indicators.
 */
export function calculateCycleState(indicators: Indicator[]): CurrentState {
  // Simple heuristic based on indicator interpretations
  const interpretations = indicators
    .filter((i) => i.interpretation)
    .map((i) => i.interpretation!)

  const bullishCount = interpretations.filter((i) => i === "bullish").length
  const bearishCount = interpretations.filter((i) => i === "bearish").length
  const total = interpretations.length

  const trend: TrendDirection = bullishCount > bearishCount * 1.5
    ? "up"
    : bearishCount > bullishCount * 1.5
      ? "down"
      : "flat"

  // Determine cycle phase based on trend and momentum
  const cycle: CyclePhase = trend === "up"
    ? bullishCount > total * 0.7 ? "expansion" : "recovery"
    : trend === "down"
      ? bearishCount > total * 0.7 ? "contraction" : "peak"
      : "trough"

  // Position is harder to determine without historical data
  const position: CyclePosition = "mid"

  const confidence = total > 0 ? Math.abs(bullishCount - bearishCount) / total : 0.5

  return {
    cycle,
    position,
    trend,
    confidence: Math.round(confidence * 100) / 100,
    factors: indicators.slice(0, 3).map((i) => `${i.name}: ${i.interpretation ?? "N/A"}`),
  }
}

/**
 * Format causal chain as Mermaid diagram.
 */
export function formatCausalChainMermaid(chain: CausalChain): string {
  const relationshipArrows: Record<CausalLink["relationship"], string> = {
    causes: "-->",
    enables: "-.->",
    blocks: "--x",
    influences: "~~~",
    correlates: "<-->",
  }

  const lines = [
    "```mermaid",
    "graph LR",
    `  subgraph ${chain.title.replace(/\s+/g, "_")}`,
  ]

  chain.links.forEach((link, i) => {
    const from = link.from.replace(/\s+/g, "_")
    const to = link.to.replace(/\s+/g, "_")
    const arrow = relationshipArrows[link.relationship]
    lines.push(`    ${from}${arrow}${to}`)
  })

  lines.push("  end")
  lines.push("```")

  return lines.join("\n")
}

/**
 * Format analysis report summary as markdown.
 */
export function formatAnalysisMarkdown(report: BaseAnalysisReport): string {
  const { currentState, summary, risks, recommendations, disclaimer } = report

  const cycleEmoji: Record<CyclePhase, string> = {
    expansion: "[EXPANSION]",
    peak: "[PEAK]",
    contraction: "[CONTRACTION]",
    trough: "[TROUGH]",
    recovery: "[RECOVERY]",
  }

  const priorityEmoji: Record<Recommendation["priority"], string> = {
    urgent: "[URGENT]",
    high: "[HIGH]",
    medium: "[MEDIUM]",
    low: "[LOW]",
  }

  const riskSections = risks.length > 0
    ? `### Risks\n${risks.map((r) => `- **${r.impact.toUpperCase()}** (${Math.round(r.probability * 100)}%): ${r.description}`).join("\n")}`
    : ""

  const recSections = recommendations.length > 0
    ? `### Recommendations\n${recommendations.map((r) => `- ${priorityEmoji[r.priority]} ${r.action}\n  _${r.rationale}_`).join("\n\n")}`
    : ""

  return `## ${report.title}

**Cycle**: ${cycleEmoji[currentState.cycle]} ${currentState.position}
**Trend**: ${currentState.trend.toUpperCase()}
**Confidence**: ${Math.round(currentState.confidence * 100)}%

### Summary
${summary}

${riskSections}

${recSections}

---
_${disclaimer}_
`
}

/**
 * Generate a unique report ID.
 */
export function generateAnalysisReportId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 6)
  return `analysis_${timestamp}_${random}`
}

/**
 * Default risk disclaimer by analyst type.
 */
export const DEFAULT_DISCLAIMERS: Record<string, string> = {
  macro:
    "This macroeconomic analysis is for informational purposes only. Economic forecasts are inherently uncertain. Consult qualified professionals before making financial decisions.",
  trading:
    "Trading involves substantial risk of loss. This analysis does not constitute a recommendation to trade. Past performance is not indicative of future results.",
  value:
    "Value analysis is based on assumptions that may not be realized. Intrinsic value estimates are subjective. Do your own research before investing.",
}
