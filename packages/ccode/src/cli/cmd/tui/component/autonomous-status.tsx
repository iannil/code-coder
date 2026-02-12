import type { SafetyStatus } from "@/autonomous/safety/integration"
import type { SessionMetrics } from "@/autonomous/metrics/metrics"
import type { QualityScoreBreakdown, CrazinessScoreBreakdown } from "@/autonomous/metrics/scorer"

/**
 * Autonomous Mode status display component
 *
 * Shows real-time Autonomous Mode execution status including:
 * - Safety status (resources, loops, rollbacks)
 * - Quality score
 * - Craziness score
 * - Session metrics
 */

interface DisplayProps {
  sessionId: string
  safetyStatus?: SafetyStatus
  qualityScore?: QualityScoreBreakdown
  crazinessScore?: CrazinessScoreBreakdown
  metrics?: SessionMetrics
  compact?: boolean
}

/**
 * Render safety status box
 */
function renderSafetyStatus(safety: SafetyStatus, compact = false): string {
  if (compact) {
    return `S: ${safety.safe ? "✓" : "✗"}`
  }

  const { resources, loops, rollbacks } = safety
  const lines: string[] = []

  // Resources
  lines.push(
    `Tokens: ${resources.usage.tokensUsed.toLocaleString()}/${resources.usage.tokensUsed + (resources.remaining.maxTokens ?? 0)}`,
  )
  lines.push(
    `Files: ${resources.usage.filesChanged}/${resources.usage.filesChanged + (resources.remaining.maxFilesChanged ?? 0)}`,
  )
  lines.push(`Surplus: ${(safety.resources.surplusRatio * 100).toFixed(0)}%`)

  // Loops
  if (loops.loopsBroken > 0) {
    lines.push(`Loops broken: ${loops.loopsBroken}`)
  }

  // Rollbacks
  lines.push(`Rollbacks: ${rollbacks.count}`)

  return `[Safety Status]\n${lines.join("\n")}`
}

/**
 * Render quality score box
 */
function renderQualityScore(quality: QualityScoreBreakdown, compact = false): string {
  if (compact) {
    return `Q: ${quality.overall}/100`
  }

  const lines: string[] = []

  // Overall score
  const scoreColor = quality.overall >= 80 ? "green" : quality.overall >= 60 ? "yellow" : "red"
  lines.push(`Overall: ${quality.overall}/100`)

  // Breakdown
  lines.push("─".repeat(20))
  lines.push(`Coverage: ${quality.testCoverage}/100`)
  lines.push(`Code Quality: ${quality.codeQuality}/100`)
  lines.push(`Decision Quality: ${quality.decisionQuality}/100`)
  lines.push(`Efficiency: ${quality.efficiency}/100`)
  lines.push(`Safety: ${quality.safety}/100`)

  return `[Quality Score]\n${lines.join("\n")}`
}

/**
 * Render craziness score box
 */
function renderCrazinessScore(craziness: CrazinessScoreBreakdown, compact = false): string {
  if (compact) {
    return `${craziness.level.toUpperCase()}: ${craziness.overall}/100`
  }

  const lines: string[] = []

  // Level
  lines.push(`Level: ${craziness.level.toUpperCase()}`)
  lines.push(`Overall: ${craziness.overall}/100`)

  // Breakdown
  lines.push("─".repeat(18))
  lines.push(`Autonomy: ${craziness.autonomy}/100`)
  lines.push(`Self-Correction: ${craziness.selfCorrection}/100`)
  lines.push(`Speed: ${craziness.speed}/100`)
  lines.push(`Risk-Taking: ${craziness.riskTaking}/100`)

  return `[Craziness Score]\n${lines.join("\n")}`
}

/**
 * Render session metrics box
 */
function renderSessionMetrics(metrics: SessionMetrics, compact = false): string {
  if (compact) {
    return `T: ${metrics.tasks.completed}/${metrics.tasks.total}`
  }

  const lines: string[] = []

  // Tasks
  lines.push(`Tasks: ${metrics.tasks.completed}/${metrics.tasks.total}`)
  if (metrics.tasks.failed > 0) {
    lines.push(`  Failed: ${metrics.tasks.failed}`)
  }

  // Tests
  lines.push(`Tests: ${metrics.tests.passed}/${metrics.tests.run} (${metrics.tests.passRate.toFixed(1)}%)`)

  // Decisions
  if (metrics.decisions.total > 0) {
    lines.push(`Decisions: ${metrics.decisions.approved}/${metrics.decisions.total}`)
  }

  // Duration
  const minutes = Math.floor(metrics.duration / 60000)
  const seconds = Math.floor((metrics.duration % 60000) / 1000)
  lines.push(`Duration: ${minutes}m ${seconds}s`)

  return `[Session Metrics]\n${lines.join("\n")}`
}

/**
 * Render TDD cycle status
 */
function renderTDDStatus(metrics: SessionMetrics, compact = false): string {
  if (compact) {
    return `TDD: ${metrics.tdd.cycles}`
  }

  const tdd = metrics.tdd
  const lines: string[] = []

  lines.push(`Cycles: ${tdd.cycles}`)

  // Phase indicators
  const redPhase = tdd.redPassed > 0 ? "✓" : "○"
  const greenPhase = tdd.greenPassed > 0 ? "✓" : "○"
  const refactorPhase = tdd.refactorPassed > 0 ? "✓" : "○"

  lines.push(`RED: ${redPhase} (${tdd.redPassed})`)
  lines.push(`GREEN: ${greenPhase} (${tdd.greenPassed})`)
  lines.push(`REFACTOR: ${refactorPhase} (${tdd.refactorPassed})`)

  return `[TDD Cycles]\n${lines.join("\n")}`
}

/**
 * Render Autonomous Mode status
 */
export function renderCrazyStatus(props: DisplayProps): string {
  const boxes: string[] = []

  // Safety status
  if (props.safetyStatus) {
    boxes.push(renderSafetyStatus(props.safetyStatus, props.compact))
  }

  // Quality score
  if (props.qualityScore) {
    boxes.push(renderQualityScore(props.qualityScore, props.compact))
  }

  // Craziness score
  if (props.crazinessScore) {
    boxes.push(renderCrazinessScore(props.crazinessScore, props.compact))
  }

  // Session metrics
  if (props.metrics) {
    boxes.push(renderSessionMetrics(props.metrics, props.compact))
  }

  if (props.metrics && props.metrics.tdd) {
    boxes.push(renderTDDStatus(props.metrics, props.compact))
  }

  if (boxes.length === 0) {
    return "No Autonomous Mode status available"
  }

  // Layout boxes horizontally
  if (props.compact) {
    return boxes.join(" ")
  }

  // In full mode, render boxes in a grid
  const columns = 2
  const rows: string[] = []

  for (let i = 0; i < boxes.length; i += columns) {
    const rowBoxes = boxes.slice(i, i + columns)
    rows.push(rowBoxes.join("\n\n"))
  }

  return rows.join("\n\n")
}

/**
 * Render inline status bar (for status line)
 */
export function renderCrazyStatusBar(props: DisplayProps): string {
  if (!props.safetyStatus) {
    return ""
  }

  if (!props.crazinessScore) {
    return ""
  }

  const safetyColor = props.safetyStatus.safe ? "green" : "red"
  const safetyIcon = props.safetyStatus.safe ? "✓" : "!"

  // Craziness level indicator
  const craziness = props.crazinessScore
  const crazinessColor =
    craziness.level === "lunatic"
      ? "magenta"
      : craziness.level === "insane"
        ? "red"
        : craziness.level === "crazy"
          ? "yellow"
          : craziness.level === "wild"
            ? "blue"
            : "gray"

  return `[${safetyColor}${safetyIcon} SAFE] ` + craziness.level.toUpperCase() + ` ${(craziness.overall / 100).toFixed(0)}`
}

/**
 * Render detailed report (for fullscreen view)
 */
export function renderCrazyReport(props: DisplayProps): string {
  const lines: string[] = []

  lines.push("═".repeat(50))
  lines.push("        CRAZY MODE SESSION REPORT")
  lines.push("═".repeat(50))
  lines.push("")

  lines.push(`Session ID: ${props.sessionId}`)
  lines.push(`Time: ${new Date().toLocaleString()}`)
  lines.push("")

  // Safety Status
  if (props.safetyStatus) {
    const safety = props.safetyStatus
    lines.push("── SAFETY STATUS ──")
    lines.push(`Safe: ${safety.safe ? "YES" : "NO"}`)
    lines.push(`Resources: ${(safety.resources.surplusRatio * 100).toFixed(1)}% surplus`)
    lines.push(`Loops Broken: ${safety.loops.loopsBroken}`)
    lines.push(`Rollbacks: ${safety.rollbacks.count}`)
    lines.push("")
  }

  // Quality Score
  if (props.qualityScore) {
    const q = props.qualityScore
    lines.push("── QUALITY SCORE ──")
    lines.push(`Overall: ${q.overall}/100`)
    lines.push(`  Coverage: ${q.testCoverage}/100`)
    lines.push(`  Code Quality: ${q.codeQuality}/100`)
    lines.push(`  Decision Quality: ${q.decisionQuality}/100`)
    lines.push(`  Efficiency: ${q.efficiency}/100`)
    lines.push(`  Safety: ${q.safety}/100`)
    lines.push("")
  }

  // Craziness Score
  if (props.crazinessScore) {
    const c = props.crazinessScore
    lines.push("── CRAZINESS SCORE ──")
    lines.push(`Level: ${c.level.toUpperCase()}`)
    lines.push(`Overall: ${c.overall}/100`)
    lines.push(`  Autonomy: ${c.autonomy}/100`)
    lines.push(`  Self-Correction: ${c.selfCorrection}/100`)
    lines.push(`  Speed: ${c.speed}/100`)
    lines.push(`  Risk-Taking: ${c.riskTaking}/100`)
    lines.push("")
  }

  // Session Metrics
  if (props.metrics) {
    const m = props.metrics
    lines.push("── SESSION METRICS ──")
    lines.push(`Tasks: ${m.tasks.completed}/${m.tasks.total}`)
    lines.push(`  Completed: ${m.tasks.completed}`)
    lines.push(`  Failed: ${m.tasks.failed}`)
    lines.push(`Tests: ${m.tests.passed}/${m.tests.run} (${m.tests.passRate.toFixed(1)}%)`)
    if (m.decisions.total > 0) {
      lines.push(`Decisions: ${m.decisions.approved}/${m.decisions.total}`)
    }
    const minutes = Math.floor(m.duration / 60000)
    const seconds = Math.floor((m.duration % 60000) / 1000)
    lines.push(`Duration: ${minutes}m ${seconds}s`)
    lines.push("")
  }

  lines.push("═".repeat(50))

  return lines.join("\n")
}

/**
 * Get craziness level description
 */
export function getCrazinessDescription(level: string): string {
  const descriptions: Record<string, string> = {
    lunatic: "完全自主，疯狂到令人担忧 - 在无任何人工干预的情况下运行",
    insane: "高度自主，几乎不需要干预 - 处理大多数复杂任务",
    crazy: "显著自主，偶需帮助 - 可以独立工作但需要定期检查",
    wild: "部分自主，需定期确认 - 需要频繁的人工指导",
    bold: "谨慎自主，频繁暂停 - 经常请求许可",
    timid: "几乎无法自主 - 需要持续监督",
  }
  return descriptions[level] ?? "Unknown level"
}
