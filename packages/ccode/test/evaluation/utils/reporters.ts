/**
 * Evaluation result reporters
 *
 * Provides functions to format and output evaluation results
 * in various formats (console, markdown, JSON).
 */

import type { EvaluationSummary, MetricResult } from "./metrics"

/**
 * Full evaluation report
 */
export interface EvaluationReport {
  timestamp: number
  duration: number
  dimensions: EvaluationSummary[]
  overallScore: number
  overallPassRate: number
  recommendations: string[]
}

/**
 * Generate a full evaluation report
 */
export function generateReport(
  dimensions: EvaluationSummary[],
  duration: number,
): EvaluationReport {
  const overallScore =
    dimensions.length > 0
      ? dimensions.reduce((sum, d) => sum + d.overallScore, 0) / dimensions.length
      : 0

  const overallPassRate =
    dimensions.length > 0
      ? dimensions.reduce((sum, d) => sum + d.passRate, 0) / dimensions.length
      : 0

  const recommendations = generateRecommendations(dimensions)

  return {
    timestamp: Date.now(),
    duration,
    dimensions,
    overallScore,
    overallPassRate,
    recommendations,
  }
}

/**
 * Generate recommendations based on evaluation results
 */
function generateRecommendations(dimensions: EvaluationSummary[]): string[] {
  const recommendations: string[] = []

  for (const dimension of dimensions) {
    const failedMetrics = dimension.metrics.filter((m) => !m.passed)

    for (const metric of failedMetrics) {
      recommendations.push(
        `[${dimension.dimension}] ${metric.name}: Current ${metric.value.toFixed(3)}, ` +
          `Target ${metric.target.toFixed(3)}${metric.details ? ` - ${metric.details}` : ""}`,
      )
    }
  }

  return recommendations
}

/**
 * Format report as markdown
 */
export function formatReportAsMarkdown(report: EvaluationReport): string {
  const lines: string[] = []

  lines.push("# Bootstrap Flywheel Evaluation Report")
  lines.push("")
  lines.push(`**Generated:** ${new Date(report.timestamp).toISOString()}`)
  lines.push(`**Duration:** ${(report.duration / 1000).toFixed(2)}s`)
  lines.push("")

  // Overall summary
  lines.push("## Overall Summary")
  lines.push("")
  lines.push(`| Metric | Value |`)
  lines.push(`|--------|-------|`)
  lines.push(`| Overall Score | ${(report.overallScore * 100).toFixed(1)}% |`)
  lines.push(`| Pass Rate | ${(report.overallPassRate * 100).toFixed(1)}% |`)
  lines.push(`| Dimensions Evaluated | ${report.dimensions.length} |`)
  lines.push("")

  // Dimension details
  lines.push("## Dimension Results")
  lines.push("")

  for (const dimension of report.dimensions) {
    lines.push(`### ${dimension.dimension}`)
    lines.push("")
    lines.push(`**Score:** ${(dimension.overallScore * 100).toFixed(1)}% | ` +
      `**Pass Rate:** ${(dimension.passRate * 100).toFixed(1)}%`)
    lines.push("")
    lines.push("| Metric | Value | Target | Status |")
    lines.push("|--------|-------|--------|--------|")

    for (const metric of dimension.metrics) {
      const status = metric.passed ? "✅" : "❌"
      lines.push(
        `| ${metric.name} | ${metric.value.toFixed(3)} | ${metric.target.toFixed(3)} | ${status} |`,
      )
    }
    lines.push("")
  }

  // Recommendations
  if (report.recommendations.length > 0) {
    lines.push("## Recommendations")
    lines.push("")
    for (const rec of report.recommendations) {
      lines.push(`- ${rec}`)
    }
    lines.push("")
  }

  // Summary assessment
  lines.push("## Assessment")
  lines.push("")
  if (report.overallPassRate >= 0.8) {
    lines.push("✅ **Excellent:** The Bootstrap Flywheel system is performing well.")
  } else if (report.overallPassRate >= 0.6) {
    lines.push("⚠️ **Good:** The system is functional but has areas for improvement.")
  } else {
    lines.push("❌ **Needs Work:** Significant improvements needed in the identified areas.")
  }
  lines.push("")

  return lines.join("\n")
}

/**
 * Format report as JSON
 */
export function formatReportAsJson(report: EvaluationReport): string {
  return JSON.stringify(report, null, 2)
}

/**
 * Format report for console output
 */
export function formatReportForConsole(report: EvaluationReport): string {
  const lines: string[] = []
  const width = 60

  lines.push("=".repeat(width))
  lines.push("  BOOTSTRAP FLYWHEEL EVALUATION REPORT")
  lines.push("=".repeat(width))
  lines.push("")

  // Overall summary
  lines.push(`Overall Score:    ${formatBar(report.overallScore, 30)} ${(report.overallScore * 100).toFixed(1)}%`)
  lines.push(`Pass Rate:        ${formatBar(report.overallPassRate, 30)} ${(report.overallPassRate * 100).toFixed(1)}%`)
  lines.push(`Duration:         ${(report.duration / 1000).toFixed(2)}s`)
  lines.push("")
  lines.push("-".repeat(width))

  // Dimensions
  for (const dimension of report.dimensions) {
    lines.push("")
    lines.push(`[${dimension.dimension}]`)
    lines.push(`  Score: ${formatBar(dimension.overallScore, 20)} ${(dimension.overallScore * 100).toFixed(1)}%`)

    for (const metric of dimension.metrics) {
      const status = metric.passed ? "✓" : "✗"
      const padding = 25 - metric.name.length
      lines.push(
        `  ${status} ${metric.name}${" ".repeat(Math.max(0, padding))} ${metric.value.toFixed(3)} / ${metric.target.toFixed(3)}`,
      )
    }
  }

  lines.push("")
  lines.push("-".repeat(width))

  // Recommendations
  if (report.recommendations.length > 0) {
    lines.push("")
    lines.push("Recommendations:")
    for (const rec of report.recommendations.slice(0, 5)) {
      lines.push(`  • ${rec}`)
    }
    if (report.recommendations.length > 5) {
      lines.push(`  ... and ${report.recommendations.length - 5} more`)
    }
  }

  lines.push("")
  lines.push("=".repeat(width))

  return lines.join("\n")
}

/**
 * Format a progress bar
 */
function formatBar(value: number, width: number): string {
  const filled = Math.round(value * width)
  const empty = width - filled
  return `[${"█".repeat(filled)}${"░".repeat(empty)}]`
}

/**
 * Format a single metric result for quick display
 */
export function formatMetricResult(metric: MetricResult): string {
  const status = metric.passed ? "PASS" : "FAIL"
  return `[${status}] ${metric.name}: ${metric.value.toFixed(3)} (target: ${metric.target.toFixed(3)})`
}

/**
 * Create a dimension summary for test output
 */
export function createDimensionSummary(summary: EvaluationSummary): string {
  const passed = summary.metrics.filter((m) => m.passed).length
  const total = summary.metrics.length
  return `${summary.dimension}: ${passed}/${total} metrics passed (${(summary.passRate * 100).toFixed(0)}%)`
}
