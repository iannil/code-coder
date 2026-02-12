/**
 * Verification Report Generator
 *
 * Generates structured verification reports in Markdown format.
 * Reports include all verification results, evidence, and recommendations.
 */

import { Log } from "@/util/log"
import type { VerificationResult, PropertyResult, InvariantResult, Issue } from "../schema/verification-result"
import type { Verdict } from "../schema/verification-result"

const log = Log.create({ service: "verifier.reporter.generator" })

/**
 * Report generation options
 */
export interface ReportOptions {
  includeFullEvidence?: boolean
  includeSourceCode?: boolean
  outputFile?: string
  format?: "markdown" | "json"
}

/**
 * Report generator state
 */
export class ReportGenerator {
  private sessionId: string

  constructor(sessionId: string) {
    this.sessionId = sessionId
  }

  /**
   * Generate a full verification report
   */
  generate(result: VerificationResult, options: ReportOptions = {}): string {
    const {
      includeFullEvidence = false,
      includeSourceCode = false,
      format = "markdown",
    } = options

    if (format === "json") {
      return this.generateJson(result)
    }

    return this.generateMarkdown(result, {
      includeFullEvidence,
      includeSourceCode,
    })
  }

  /**
   * Generate report in Markdown format
   */
  private generateMarkdown(result: VerificationResult, options: {
    includeFullEvidence: boolean
    includeSourceCode: boolean
  }): string {
    const verdictEmoji = this.getVerdictEmoji(result.verdict)
    const verdictColor = this.getVerdictColor(result.verdict)

    let md = `# ${result.goalTitle} 验证报告

> 验收日期: ${new Date(result.verifiedAt).toLocaleDateString("zh-CN")}
> Agent: verifier
> Session ID: ${this.sessionId}
> 相关需求: ${result.goalId}

## 执行摘要

| 维度 | 状态 | 详情 |
|------|------|------|
| 前置条件 | ${this.getStatusSummary(result.preconditions)} | ${this.getPassCount(result.preconditions)}/${result.preconditions.length} 已验证 |
| 后置条件 | ${this.getStatusSummary(result.postconditions)} | ${this.getPassCount(result.postconditions)}/${result.postconditions.length} 已验证 |
| 不变量 | ${this.getStatusSummary(result.invariants)} | ${this.getPassCount(result.invariants)}/${result.invariants.length} 保持 |
| 属性 | ${this.getStatusSummary(result.properties)} | ${this.getPassCount(result.properties)}/${result.properties.length} 已证明 |
| 测试覆盖率 | ${result.coverage.testCoverage.toFixed(1)}% | 目标: 80% |
| 验收标准 | ${this.getStatusSummary(result.acceptance)} | ${this.getPassCount(result.acceptance)}/${result.acceptance.length} 已满足 |

**最终判决**: ${verdictEmoji} **${this.getVerdictLabel(result.verdict)}**

${result.summary}

---

## 功能目标

### 目标: ${result.goalId}

**追溯**: ${result.goalId}

### 前置条件

| ID | 形式化陈述 | 状态 | 证据 |
|----|-----------|------|------|
${this.renderPredicateResults(result.preconditions, options)}

### 后置条件

| ID | 形式化陈述 | 状态 | 证据 |
|----|-----------|------|------|
${this.renderPredicateResults(result.postconditions, options)}

### 不变量

| ID | 不变量 | 作用域 | 状态 |
|----|--------|--------|------|
${this.renderInvariantResults(result.invariants, options)}

### 数学属性

| ID | 属性 | 形式化陈述 | 状态 | 证明方法 |
|----|------|-----------|------|----------|
${this.renderPropertyResults(result.properties, options)}

### 验收标准

| ID | 标准 (SMART) | 阈值 | 实测 | 状态 |
|----|--------------|------|------|------|
${this.renderAcceptanceResults(result.acceptance, options)}

---

## 覆盖率分析

### 需求-测试矩阵

| 需求ID | 测试用例 | 状态 | 覆盖度 |
|--------|----------|------|--------|
${this.renderMatrix(result.matrix)}

**未覆盖需求**: ${result.coverage.uncoveredRequirements.length > 0
  ? result.coverage.uncoveredRequirements.join(", ")
  : "无"}

**部分覆盖**: ${result.coverage.partiallyCoveredRequirements.length > 0
  ? result.coverage.partiallyCoveredRequirements.join(", ")
  : "无"}

---

## 发现

${this.renderIssues(result.issues)}

---

## 新生成测试

### 通过 tdd-guide 生成的测试

| 测试ID | 目标 | 文件路径 | 状态 |
|--------|------|----------|------|
${result.generatedTests.map((t: { testId: string; target: string; filePath: string; status: string }) =>
  `| ${t.testId} | ${t.target} | ${t.filePath} | ${t.status} |`
).join("\n") || "| - | - | - | - |"}

---

## 附录

### A. 验证元数据

| 字段 | 值 |
|------|-----|
| 验收日期 | ${new Date(result.verifiedAt).toLocaleString("zh-CN")} |
| 验收耗时 | ${result.duration}ms |
| Session ID | ${result.sessionId || "N/A"} |
| 验收人 | verifier agent |

### B. 形式化规范

\`\`\`
前置条件数量: ${result.preconditions.length}
后置条件数量: ${result.postconditions.length}
不变量数量: ${result.invariants.length}
数学属性数量: ${result.properties.length}
验收标准数量: ${result.acceptance.length}
\`\`\`

---
*本报告由 CodeCoder Verifier Agent 自动生成*
`

    return md
  }

  /**
   * Generate report in JSON format
   */
  private generateJson(result: VerificationResult): string {
    return JSON.stringify(result, null, 2)
  }

  /**
   * Render predicate results as table rows
   */
  private renderPredicateResults(
    predicates: Array<{ id: string; statement: string; status: string; evidence: any[] }>,
    options: { includeFullEvidence: boolean },
  ): string {
    if (predicates.length === 0) {
      return "| - | - | - | - |"
    }

    return predicates
      .map((p) => {
        const status = this.getStatusEmoji(p.status)
        const evidence = options.includeFullEvidence && p.evidence.length > 0
          ? p.evidence[0].source
          : p.evidence.length > 0
            ? `${p.evidence.length} 项证据`
            : "无"

        return `| ${p.id} | ${p.statement} | ${status} ${p.status} | ${evidence} |`
      })
      .join("\n")
  }

  /**
   * Render invariant results as table rows
   */
  private renderInvariantResults(
    invariants: Array<{ id: string; name: string; status: string; scope: string }>,
    _options: { includeFullEvidence: boolean },
  ): string {
    if (invariants.length === 0) {
      return "| - | - | - | - |"
    }

    return invariants
      .map((inv) => {
        const status = this.getStatusEmoji(inv.status)
        return `| ${inv.id} | ${inv.name} | ${inv.scope} | ${status} ${inv.status} |`
      })
      .join("\n")
  }

  /**
   * Render property results as table rows
   */
  private renderPropertyResults(
    properties: PropertyResult[],
    options: { includeFullEvidence: boolean },
  ): string {
    if (properties.length === 0) {
      return "| - | - | - | - | - |"
    }

    return properties
      .map((prop) => {
        const status = this.getStatusEmoji(prop.status)
        const method = prop.proofMethod === "formal_proof" ? "形式化证明" : "属性测试"

        return `| ${prop.id} | ${prop.name} | \`${prop.formal}\` | ${status} ${prop.status} | ${method} |`
      })
      .join("\n")
  }

  /**
   * Render acceptance results as table rows
   */
  private renderAcceptanceResults(
    acceptance: Array<{ id: string; criterion: string; threshold: string; measured?: string; status: string }>,
    _options: { includeFullEvidence: boolean },
  ): string {
    if (acceptance.length === 0) {
      return "| - | - | - | - | - |"
    }

    return acceptance
      .map((acc) => {
        const status = this.getStatusEmoji(acc.status)
        const measured = acc.measured || "未测量"

        return `| ${acc.id} | ${acc.criterion} | ${acc.threshold} | ${measured} | ${status} ${acc.status} |`
      })
      .join("\n")
  }

  /**
   * Render coverage matrix as table rows
   */
  private renderMatrix(matrix: Array<{
    requirementId: string
    testCases: string[]
    status: string
    coverage: string
  }>): string {
    if (matrix.length === 0) {
      return "| - | - | - | - |"
    }

    return matrix
      .map((entry) => {
        const status = this.getStatusEmoji(entry.status)
        const tests = entry.testCases.length > 0 ? entry.testCases.join(", ") : "无"
        const coverageBadge =
          entry.coverage === "full" ? "完整"
            : entry.coverage === "partial" ? "部分"
              : "无"

        return `| ${entry.requirementId} | ${tests} | ${status} ${entry.status} | ${coverageBadge} |`
      })
      .join("\n")
  }

  /**
   * Render issues section
   */
  private renderIssues(issues: Issue[]): string {
    if (issues.length === 0) {
      return "未发现问题。"
    }

    const critical = issues.filter((i) => i.severity === "critical")
    const high = issues.filter((i) => i.severity === "high")
    const medium = issues.filter((i) => i.severity === "medium")
    const low = issues.filter((i) => i.severity === "low")

    let md = ""

    if (critical.length > 0) {
      md += `### 严重（必须修复）\n\n`
      for (const issue of critical) {
        md += `#### ${issue.title}\n\n`
        md += `- **ID**: ${issue.id}\n`
        md += `- **类别**: ${issue.category}\n`
        md += `- **描述**: ${issue.description}\n`
        if (issue.location) md += `- **位置**: ${issue.location}\n`
        if (issue.recommendation) md += `- **建议**: ${issue.recommendation}\n`
        md += "\n"
      }
    }

    if (high.length > 0) {
      md += `### 重要（应当修复）\n\n`
      for (const issue of high) {
        md += `#### ${issue.title}\n\n`
        md += `- **ID**: ${issue.id}\n`
        md += `- **类别**: ${issue.category}\n`
        md += `- **描述**: ${issue.description}\n`
        if (issue.location) md += `- **位置**: ${issue.location}\n`
        if (issue.recommendation) md += `- **建议**: ${issue.recommendation}\n`
        md += "\n"
      }
    }

    if (medium.length > 0) {
      md += `### 建议（可以改进）\n\n`
      for (const issue of medium) {
        md += `- **${issue.title}**: ${issue.description}\n`
      }
    }

    if (low.length > 0) {
      md += `### 信息（仅供参考）\n\n`
      for (const issue of low) {
        md += `- **${issue.title}**: ${issue.description}\n`
      }
    }

    return md
  }

  /**
   * Get verdict emoji
   */
  private getVerdictEmoji(verdict: Verdict): string {
    switch (verdict) {
      case "pass":
        return "✅"
      case "pass_with_warnings":
        return "⚠️"
      case "fail":
        return "❌"
      case "blocked":
        return "🚫"
    }
  }

  /**
   * Get verdict color code
   */
  private getVerdictColor(verdict: Verdict): string {
    switch (verdict) {
      case "pass":
        return "green"
      case "pass_with_warnings":
        return "yellow"
      case "fail":
        return "red"
      case "blocked":
        return "gray"
    }
  }

  /**
   * Get verdict label in Chinese
   */
  private getVerdictLabel(verdict: Verdict): string {
    switch (verdict) {
      case "pass":
        return "通过"
      case "pass_with_warnings":
        return "通过（有警告）"
      case "fail":
        return "失败"
      case "blocked":
        return "阻塞"
    }
  }

  /**
   * Get status emoji
   */
  private getStatusEmoji(status: string): string {
    switch (status) {
      case "pass":
        return "✅"
      case "fail":
        return "❌"
      case "warn":
        return "⚠️"
      case "skip":
        return "⏭️"
      case "blocked":
        return "🚫"
      default:
        return "❓"
    }
  }

  /**
   * Get status summary for a list of results
   */
  private getStatusSummary(results: Array<{ status: string }>): string {
    const pass = results.filter((r) => r.status === "pass").length
    const fail = results.filter((r) => r.status === "fail").length
    const warn = results.filter((r) => r.status === "warn").length

    if (fail > 0) return "❌ FAIL"
    if (warn > 0) return "⚠️ WARN"
    if (pass === results.length && pass > 0) return "✅ PASS"
    return "⏭️ SKIP"
  }

  /**
   * Get count of passed results
   */
  private getPassCount(results: Array<{ status: string }>): number {
    return results.filter((r) => r.status === "pass").length
  }

  /**
   * Save report to file
   */
  async saveReport(
    result: VerificationResult,
    outputPath: string,
    options: ReportOptions = {},
  ): Promise<void> {
    const report = this.generate(result, options)

    const fs = require("fs")
    const path = require("path")

    // Ensure directory exists
    const dir = path.dirname(outputPath)
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true })
    }

    fs.writeFileSync(outputPath, report, "utf-8")

    log.info("Report saved", {
      sessionId: this.sessionId,
      path: outputPath,
      format: options.format ?? "markdown",
    })
  }

  /**
   * Generate summary statistics
   */
  generateSummary(result: VerificationResult): string {
    const stats = {
      preconditions: {
        total: result.preconditions.length,
        pass: result.preconditions.filter((p: { status: string }) => p.status === "pass").length,
      },
      postconditions: {
        total: result.postconditions.length,
        pass: result.postconditions.filter((p: { status: string }) => p.status === "pass").length,
      },
      invariants: {
        total: result.invariants.length,
        pass: result.invariants.filter((i: { status: string }) => i.status === "pass").length,
      },
      properties: {
        total: result.properties.length,
        pass: result.properties.filter((p: { status: string }) => p.status === "pass").length,
      },
      acceptance: {
        total: result.acceptance.length,
        pass: result.acceptance.filter((a: { status: string }) => a.status === "pass").length,
      },
    }

    return `
Verdict: ${result.verdict}
Duration: ${result.duration}ms
Coverage: ${result.coverage.testCoverage.toFixed(1)}%

Preconditions: ${stats.preconditions.pass}/${stats.preconditions.total}
Postconditions: ${stats.postconditions.pass}/${stats.postconditions.total}
Invariants: ${stats.invariants.pass}/${stats.invariants.total}
Properties: ${stats.properties.pass}/${stats.properties.total}
Acceptance: ${stats.acceptance.pass}/${stats.acceptance.total}

Issues: ${result.issues.length}
  Critical: ${result.issues.filter((i: { severity: string }) => i.severity === "critical").length}
  High: ${result.issues.filter((i: { severity: string }) => i.severity === "high").length}
  Medium: ${result.issues.filter((i: { severity: string }) => i.severity === "medium").length}
  Low: ${result.issues.filter((i: { severity: string }) => i.severity === "low").length}
`
  }
}

/**
 * Create a report generator
 */
export function createReportGenerator(sessionId: string): ReportGenerator {
  return new ReportGenerator(sessionId)
}

/**
 * Generate a default report file path
 */
export function generateReportPath(
  sessionId: string,
  goalId: string,
  extension = "md",
): string {
  const date = new Date().toISOString().split("T")[0]
  return `docs/reports/verification/${date}-${goalId}-${sessionId.slice(0, 8)}.${extension}`
}
