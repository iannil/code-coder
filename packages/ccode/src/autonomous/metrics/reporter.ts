import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import type { SessionMetrics } from "./metrics"
import type { QualityScoreBreakdown, CrazinessScoreBreakdown, CrazinessLevel } from "./scorer"
import { AutonomousState } from "../state/states"
import { DecisionHistory } from "../decision/history"

const log = Log.create({ service: "autonomous.reporter" })

/**
 * Report types
 */
export type ReportType = "summary" | "detailed" | "metrics" | "decisions" | "full"

/**
 * Report data
 */
export interface Report {
  sessionId: string
  type: ReportType
  generatedAt: number
  duration: number
  summary: string
  metrics?: SessionMetrics
  qualityScore?: QualityScoreBreakdown
  crazinessScore?: CrazinessScoreBreakdown
  details?: Record<string, unknown>
}

/**
 * Report options
 */
export interface ReportOptions {
  includeMetrics: boolean
  includeDecisions: boolean
  includeTimeline: boolean
  format: "text" | "markdown" | "json"
}

/**
 * Reporter for generating execution reports
 *
 * Creates comprehensive reports of Autonomous Mode sessions
 */
export class Reporter {
  private sessionId: string
  private storageKey: string[]

  constructor(sessionId: string) {
    this.sessionId = sessionId
    const projectID = Instance.project.id
    this.storageKey = ["autonomous", "reports", projectID, sessionId]
  }

  /**
   * Generate a report
   */
  async generate(
    type: ReportType = "summary",
    options?: Partial<ReportOptions>,
  ): Promise<Report> {
    const opts: ReportOptions = {
      includeMetrics: type !== "summary",
      includeDecisions: type === "detailed" || type === "decisions" || type === "full",
      includeTimeline: type === "detailed" || type === "full",
      format: "markdown",
      ...options,
    }

    // Handle full report type
    const reportType: ReportType = type === "full" ? "detailed" : type

    const generatedAt = Date.now()

    // Load metrics
    let metrics: SessionMetrics | undefined
    if (opts.includeMetrics) {
      const { getSessionMetrics } = await import("./metrics")
      metrics = await getSessionMetrics(this.sessionId)
    }

    // Calculate duration
    const duration = metrics?.duration ?? 0

    // Generate summary
    const summary = this.generateSummary(metrics, type)

    // Publish event
    await Bus.publish(AutonomousEvent.ReportGenerated, {
      sessionId: this.sessionId,
      reportType: reportType,
    })

    return {
      sessionId: this.sessionId,
      type: reportType,
      generatedAt,
      duration,
      summary,
      metrics,
    }
  }

  /**
   * Generate text report
   */
  async generateText(type: ReportType = "summary"): Promise<string> {
    const report = await this.generate(type, { format: "text" })
    return this.formatText(report)
  }

  /**
   * Generate markdown report
   */
  async generateMarkdown(type: ReportType = "summary"): Promise<string> {
    const report = await this.generate(type, { format: "markdown" })
    return this.formatMarkdown(report)
  }

  /**
   * Generate JSON report
   */
  async generateJSON(type: ReportType = "full"): Promise<string> {
    const report = await this.generate(type, { format: "json" })
    return JSON.stringify(report, null, 2)
  }

  /**
   * Save report to file
   */
  async saveToFile(type: ReportType = "summary", filePath?: string): Promise<string> {
    const content = await this.generateMarkdown(type)

    const defaultPath = [
      Instance.worktree,
      ".ccode",
      "autonomous-reports",
      `${this.sessionId}-${type}.md`,
    ].join("/")

    const targetPath = filePath ?? defaultPath

    await Storage.write([targetPath], content)

    log.info("Report saved", { type, path: targetPath })

    return targetPath
  }

  /**
   * Generate summary text
   */
  private generateSummary(metrics?: SessionMetrics, type: ReportType = "summary"): string {
    const parts: string[] = []

    parts.push(`# Autonomous Mode Session Report`)
    parts.push("")
    parts.push(`**Session ID:** ${this.sessionId}`)
    parts.push(`**Report Type:** ${type}`)
    parts.push(`**Generated:** ${new Date().toISOString()}`)
    parts.push("")

    if (metrics) {
      parts.push(`**Duration:** ${this.formatDuration(metrics.duration)}`)
      parts.push("")

      // Task summary
      parts.push("## Task Summary")
      parts.push("")
      parts.push(`- Total: ${metrics.tasks.total}`)
      parts.push(`- Completed: ${metrics.tasks.completed}`)
      parts.push(`- Failed: ${metrics.tasks.failed}`)
      parts.push(`- Skipped: ${metrics.tasks.skipped}`)
      parts.push("")

      // Test summary
      if (metrics.tests.run > 0) {
        parts.push("## Test Results")
        parts.push("")
        parts.push(`- Run: ${metrics.tests.run}`)
        parts.push(`- Passed: ${metrics.tests.passed}`)
        parts.push(`- Failed: ${metrics.tests.failed}`)
        parts.push(`- Pass Rate: ${(metrics.tests.passRate * 100).toFixed(1)}%`)
        parts.push("")
      }

      // Decision summary
      if (metrics.decisions.total > 0) {
        parts.push("## Decisions")
        parts.push("")
        parts.push(`- Total: ${metrics.decisions.total}`)
        parts.push(`- Approved: ${metrics.decisions.approved}`)
        parts.push(`- Paused: ${metrics.decisions.paused}`)
        parts.push(`- Blocked: ${metrics.decisions.blocked}`)
        parts.push(`- Avg Score: ${metrics.decisions.averageScore.toFixed(2)}/10`)
        parts.push("")
      }

      // Resource summary
      parts.push("## Resources")
      parts.push("")
      parts.push(`- Tokens Used: ${metrics.resources.tokensUsed.toLocaleString()}`)
      parts.push(`- Cost: $${metrics.resources.costUSD.toFixed(4)}`)
      parts.push(`- Files Changed: ${metrics.resources.filesChanged}`)
      parts.push("")

      // Safety summary
      if (metrics.safety.rollbacks > 0 || metrics.safety.loopsDetected > 0) {
        parts.push("## Safety Events")
        parts.push("")
        parts.push(`- Rollbacks: ${metrics.safety.rollbacks}`)
        parts.push(`- Loops Detected: ${metrics.safety.loopsDetected}`)
        parts.push(`- Warnings: ${metrics.safety.warnings}`)
        parts.push("")
      }

      // TDD summary
      if (metrics.tdd.cycles > 0) {
        parts.push("## TDD Cycles")
        parts.push("")
        parts.push(`- Total Cycles: ${metrics.tdd.cycles}`)
        parts.push(`- Red Phase: ${metrics.tdd.redPassed}/${metrics.tdd.cycles} passed`)
        parts.push(`- Green Phase: ${metrics.tdd.greenPassed}/${metrics.tdd.cycles} passed`)
        parts.push(`- Refactor Phase: ${metrics.tdd.refactorPassed}/${metrics.tdd.cycles} passed`)
        parts.push("")
      }
    }

    return parts.join("\n")
  }

  /**
   * Format as text
   */
  private formatText(report: Report): string {
    return report.summary
  }

  /**
   * Format as markdown
   */
  private formatMarkdown(report: Report): string {
    let content = report.summary

    // Add scores if available
    if (report.qualityScore || report.crazinessScore) {
      content += "\n## Scores\n\n"

      if (report.qualityScore) {
        content += `### Quality Score: ${report.qualityScore.overall}/100\n\n`
        content += `- Test Coverage: ${report.qualityScore.testCoverage}/100\n`
        content += `- Code Quality: ${report.qualityScore.codeQuality}/100\n`
        content += `- Decision Quality: ${report.qualityScore.decisionQuality}/100\n`
        content += `- Efficiency: ${report.qualityScore.efficiency}/100\n`
        content += `- Safety: ${report.qualityScore.safety}/100\n\n`
      }

      if (report.crazinessScore) {
        content += `### Craziness Score: ${report.crazinessScore.overall}/100 (${report.crazinessScore.level})\n\n`
        content += `- Autonomy: ${report.crazinessScore.autonomy}/100\n`
        content += `- Self-Correction: ${report.crazinessScore.selfCorrection}/100\n`
        content += `- Speed: ${report.crazinessScore.speed}/100\n`
        content += `- Risk-Taking: ${report.crazinessScore.riskTaking}/100\n\n`
      }
    }

    return content
  }

  /**
   * Format duration
   */
  private formatDuration(ms: number): string {
    const seconds = Math.floor(ms / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)

    if (hours > 0) {
      return `${hours}h ${minutes % 60}m`
    }
    if (minutes > 0) {
      return `${minutes}m ${seconds % 60}s`
    }
    return `${seconds}s`
  }

  /**
   * Get all reports for a session
   */
  async getReports(): Promise<Report[]> {
    try {
      const data = await Storage.read<Record<string, Report>>(this.storageKey)
      return Object.values(data ?? {}).sort((a, b) => b.generatedAt - a.generatedAt)
    } catch {
      return []
    }
  }

  /**
   * Save a report
   */
  async saveReport(report: Report): Promise<void> {
    try {
      const existing = await Storage.read<Record<string, Report>>(this.storageKey) ?? {}
      existing[report.type] = report
      await Storage.write(this.storageKey, existing)

      log.info("Report saved", { type: report.type })
    } catch (error) {
      log.error("Failed to save report", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }
}

/**
 * Create a reporter
 */
export function createReporter(sessionId: string): Reporter {
  return new Reporter(sessionId)
}

/**
 * Generate a quick summary report (convenience function)
 */
export async function generateSummaryReport(sessionId: string): Promise<string> {
  const reporter = new Reporter(sessionId)
  return reporter.generateMarkdown("summary")
}

/**
 * Generate a full report with decisions (convenience function)
 */
export async function generateFullReport(sessionId: string): Promise<string> {
  const reporter = new Reporter(sessionId)

  let content = await reporter.generateMarkdown("full")

  // Add decision history
  const decisions = await DecisionHistory.getBySession(sessionId)
  if (decisions.length > 0) {
    content += "\n## Decision History\n\n"
    for (const decision of decisions) {
      content += `### ${decision.type}: ${decision.description}\n`
      content += `- Score: ${decision.score.total.toFixed(2)}/10\n`
      content += `- Result: ${decision.result}\n`
      content += `- Time: ${new Date(decision.timestamp).toISOString()}\n\n`
    }
  }

  return content
}

/**
 * Get craziness level description
 */
export function getCrazinessDescription(level: CrazinessLevel): string {
  const descriptions: Record<CrazinessLevel, string> = {
    lunatic: "完全自主 - Operates without any human intervention",
    insane: "高度自主 - Almost never needs intervention",
    crazy: "显著自主 - Can work independently with periodic check-ins",
    wild: "部分自主 - Requires regular human guidance",
    bold: "谨慎自主 - Asks for permission frequently",
    timid: "几乎无法自主 - Requires constant supervision",
  }
  return descriptions[level]
}
