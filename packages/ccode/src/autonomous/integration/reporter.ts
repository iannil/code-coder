/**
 * Decision Reporter for Autonomous Mode
 *
 * Generates human-readable reports of CLOSE decisions made during a session.
 *
 * @package crazy/integration
 */

import { Log } from "@/util/log"
import { type CLOSEScore, type DecisionRecord } from "../decision/criteria"

const log = Log.create({ service: "autonomous.integration.reporter" })

/**
 * Decision summary for reporting
 */
export interface DecisionSummary {
  total: number
  byAction: Record<string, number>
  approved: number
  blocked: number
  caution: number
  averageScore: number
  lowestScore: { score: number; description: string }
  highestScore: { score: number; description: string }
}

/**
 * Format a single CLOSE score as readable text
 */
export function formatCLOSEScore(score: CLOSEScore): string {
  const parts: string[] = []
  parts.push(`**Total: ${score.total.toFixed(2)}/10**`)

  const breakdown = [
    `C: ${score.convergence.toFixed(1)}`,
    `L: ${score.leverage.toFixed(1)}`,
    `O: ${score.optionality.toFixed(1)}`,
    `S: ${score.surplus.toFixed(1)}`,
    `E: ${score.evolution.toFixed(1)}`,
  ]
  parts.push(`(${breakdown.join(" | ")})`)

  return parts.join("\n")
}

/**
 * Format a decision record as readable text
 */
export function formatDecision(decision: DecisionRecord): string {
  const actionEmoji = {
    proceed: "✅",
    proceed_with_caution: "⚠️",
    pause: "⏸️",
    block: "🚫",
    skip: "⏭️",
  }

  const emoji = actionEmoji[decision.result] ?? "❓"

  return [
    `${emoji} **${decision.result.toUpperCase()}** (${decision.score.total.toFixed(2)}/10)`,
    `    ${decision.description.slice(0, 80)}${decision.description.length > 80 ? "..." : ""}`,
    `    CLOSE: C=${decision.score.convergence.toFixed(1)} L=${decision.score.leverage.toFixed(1)} O=${decision.score.optionality.toFixed(1)} S=${decision.score.surplus.toFixed(1)} E=${decision.score.evolution.toFixed(1)}`,
  ].join("\n")
}

/**
 * Generate a summary of decisions
 */
export function summarizeDecisions(decisions: DecisionRecord[]): DecisionSummary {
  if (decisions.length === 0) {
    return {
      total: 0,
      byAction: {},
      approved: 0,
      blocked: 0,
      caution: 0,
      averageScore: 0,
      lowestScore: { score: 10, description: "N/A" },
      highestScore: { score: 0, description: "N/A" },
    }
  }

  const byAction: Record<string, number> = {}
  let approved = 0
  let blocked = 0
  let caution = 0
  let totalScore = 0

  for (const decision of decisions) {
    byAction[decision.type] = (byAction[decision.type] ?? 0) + 1

    if (decision.result === "proceed") approved++
    if (decision.result === "proceed_with_caution") caution++
    if (decision.result === "block" || decision.result === "pause") blocked++

    totalScore += decision.score.total
  }

  const averageScore = totalScore / decisions.length
  const sortedByScore = [...decisions].sort((a, b) => a.score.total - b.score.total)

  return {
    total: decisions.length,
    byAction,
    approved,
    blocked,
    caution,
    averageScore: Math.round(averageScore * 100) / 100,
    lowestScore: {
      score: sortedByScore[0].score.total,
      description: sortedByScore[0].description.slice(0, 50),
    },
    highestScore: {
      score: sortedByScore[decisions.length - 1].score.total,
      description: sortedByScore[decisions.length - 1].description.slice(0, 50),
    },
  }
}

/**
 * Generate a markdown report of decisions
 */
export function generateMarkdownReport(decisions: DecisionRecord[], sessionId: string): string {
  const summary = summarizeDecisions(decisions)

  const lines: string[] = []
  lines.push(`# CLOSE Decision Report`)
  lines.push(``)
  lines.push(`**Session:** ${sessionId}`)
  lines.push(`**Total Decisions:** ${summary.total}`)
  lines.push(`**Approved:** ${summary.approved} | **Caution:** ${summary.caution} | **Blocked:** ${summary.blocked}`)
  lines.push(`**Average Score:** ${summary.averageScore.toFixed(2)}/10`)
  lines.push(``)

  if (summary.total > 0) {
    lines.push(`## Score Range`)
    lines.push(``)
    lines.push(`- **Highest:** ${summary.highestScore.score.toFixed(2)}/10`)
    lines.push(`  - ${summary.highestScore.description}`)
    lines.push(`- **Lowest:** ${summary.lowestScore.score.toFixed(2)}/10`)
    lines.push(`  - ${summary.lowestScore.description}`)
    lines.push(``)

    lines.push(`## Decision Breakdown by Type`)
    lines.push(``)
    for (const [type, count] of Object.entries(summary.byAction).sort((a, b) => b[1] - a[1])) {
      lines.push(`- **${type}:** ${count}`)
    }
    lines.push(``)

    lines.push(`## Recent Decisions`)
    lines.push(``)

    for (const decision of decisions.slice(0, 20)) {
      lines.push(formatDecision(decision))
      lines.push(``)
    }

    if (decisions.length > 20) {
      lines.push(`... and ${decisions.length - 20} more decisions`)
    }
  }

  return lines.join("\n")
}

/**
 * Generate a compact single-line report for inclusion in tool output
 */
export function generateCompactReport(decision: DecisionRecord): string {
  const actionShort = {
    proceed: "✓",
    proceed_with_caution: "!",
    pause: "P",
    block: "✗",
    skip: "s",
  }

  const short = actionShort[decision.result] ?? "?"

  return `[CLOSE ${short} ${decision.score.total.toFixed(1)}/10: ${decision.description.slice(0, 40)}]`
}

/**
 * Generate JSON output for programmatic consumption
 */
export function generateJSONReport(decisions: DecisionRecord[], sessionId: string): string {
  const summary = summarizeDecisions(decisions)

  return JSON.stringify(
    {
      sessionId,
      generatedAt: new Date().toISOString(),
      summary: {
        total: summary.total,
        approved: summary.approved,
        blocked: summary.blocked,
        caution: summary.caution,
        averageScore: summary.averageScore,
      },
      decisions: decisions.map((d) => ({
        id: d.id,
        type: d.type,
        description: d.description,
        result: d.result,
        score: d.score.total,
        closeScores: {
          convergence: d.score.convergence,
          leverage: d.score.leverage,
          optionality: d.score.optionality,
          surplus: d.score.surplus,
          evolution: d.score.evolution,
        },
        reasoning: d.reasoning.split("\n")[0],
        timestamp: d.timestamp,
      })),
    },
    null,
    2,
  )
}

/**
 * Report namespace
 */
export const DecisionReporter = {
  formatCLOSEScore,
  formatDecision,
  summarizeDecisions,
  generateMarkdownReport,
  generateCompactReport,
  generateJSONReport,
} as const
