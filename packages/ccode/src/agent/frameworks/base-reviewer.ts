/**
 * BaseReviewer Framework
 *
 * Shared structure for code analysis agents (code-reviewer, security-reviewer).
 * Provides consistent finding structures, severity levels, and report formats.
 *
 * @module agent/frameworks/base-reviewer
 */

import z from "zod"

// ─────────────────────────────────────────────────────────────────────────────
// Severity & Category Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Finding severity levels (consistent across all reviewers).
 */
export const Severity = z.enum(["critical", "high", "medium", "low", "info"])
export type Severity = z.infer<typeof Severity>

/**
 * Severity weight for scoring (higher = more severe).
 */
export const SEVERITY_WEIGHTS: Record<Severity, number> = {
  critical: 100,
  high: 50,
  medium: 20,
  low: 5,
  info: 1,
}

/**
 * Finding categories for code-reviewer.
 */
export const CodeReviewCategory = z.enum([
  "correctness",
  "performance",
  "maintainability",
  "readability",
  "testing",
  "documentation",
  "architecture",
  "best-practices",
])
export type CodeReviewCategory = z.infer<typeof CodeReviewCategory>

/**
 * Finding categories for security-reviewer (aligned with OWASP).
 */
export const SecurityCategory = z.enum([
  "injection",
  "broken-auth",
  "sensitive-data",
  "xxe",
  "broken-access",
  "misconfiguration",
  "xss",
  "insecure-deserialization",
  "vulnerable-components",
  "insufficient-logging",
  "secrets",
  "cryptography",
])
export type SecurityCategory = z.infer<typeof SecurityCategory>

// ─────────────────────────────────────────────────────────────────────────────
// Finding Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Location in source code.
 */
export const Location = z.object({
  /** File path relative to project root */
  file: z.string(),
  /** Line number (1-indexed) */
  line: z.number().int().positive().optional(),
  /** Column number (1-indexed) */
  column: z.number().int().positive().optional(),
  /** End line for multi-line findings */
  endLine: z.number().int().positive().optional(),
  /** End column */
  endColumn: z.number().int().positive().optional(),
})
export type Location = z.infer<typeof Location>

/**
 * Base finding structure shared by all reviewers.
 */
export const BaseFinding = z.object({
  /** Unique finding ID (e.g., "CR-001", "SEC-042") */
  id: z.string(),
  /** Finding severity */
  severity: Severity,
  /** Category string (interpreted by specific reviewer) */
  category: z.string(),
  /** Location in source code */
  location: Location,
  /** Short title describing the issue */
  title: z.string(),
  /** Detailed description of the issue */
  description: z.string(),
  /** Suggested fix or improvement */
  suggestion: z.string().optional(),
  /** Code snippet showing the issue */
  codeSnippet: z.string().optional(),
  /** Confidence in this finding (0-1) */
  confidence: z.number().min(0).max(1).default(0.9),
})
export type BaseFinding = z.infer<typeof BaseFinding>

/**
 * Code review finding.
 */
export const CodeFinding = BaseFinding.extend({
  category: CodeReviewCategory,
  /** Effort to fix (low = quick fix, high = significant refactor) */
  effort: z.enum(["low", "medium", "high"]).optional(),
})
export type CodeFinding = z.infer<typeof CodeFinding>

/**
 * Security finding with OWASP-aligned fields.
 */
export const SecurityFinding = BaseFinding.extend({
  category: SecurityCategory,
  /** CWE ID (e.g., "CWE-79" for XSS) */
  cweId: z.string().optional(),
  /** CVE ID if related to known vulnerability */
  cveId: z.string().optional(),
  /** CVSS score if available */
  cvssScore: z.number().min(0).max(10).optional(),
  /** Potential impact if exploited */
  impact: z.string().optional(),
  /** Attack vector */
  attackVector: z.string().optional(),
})
export type SecurityFinding = z.infer<typeof SecurityFinding>

// ─────────────────────────────────────────────────────────────────────────────
// Report Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Severity metrics for a report.
 */
export const SeverityMetrics = z.object({
  critical: z.number().int().nonnegative(),
  high: z.number().int().nonnegative(),
  medium: z.number().int().nonnegative(),
  low: z.number().int().nonnegative(),
  info: z.number().int().nonnegative(),
})
export type SeverityMetrics = z.infer<typeof SeverityMetrics>

/**
 * Review verdict (decision on whether code is acceptable).
 */
export const Verdict = z.enum([
  "approved",
  "approved_with_changes",
  "needs_changes",
  "not_approved",
])
export type Verdict = z.infer<typeof Verdict>

/**
 * Base review report structure.
 */
export const BaseReviewReport = z.object({
  /** Report ID */
  id: z.string(),
  /** Report timestamp */
  timestamp: z.date(),
  /** Executive summary */
  summary: z.string(),
  /** Files reviewed */
  filesReviewed: z.array(z.string()),
  /** Total findings */
  totalFindings: z.number().int().nonnegative(),
  /** Severity breakdown */
  metrics: SeverityMetrics,
  /** Overall verdict */
  verdict: Verdict,
  /** Overall score (0-100) */
  score: z.number().min(0).max(100),
  /** Duration of review in ms */
  durationMs: z.number().optional(),
})
export type BaseReviewReport = z.infer<typeof BaseReviewReport>

/**
 * Code review report.
 */
export const CodeReviewReport = BaseReviewReport.extend({
  type: z.literal("code"),
  findings: z.array(CodeFinding),
  /** Highlights (positive observations) */
  highlights: z.array(z.string()).default([]),
})
export type CodeReviewReport = z.infer<typeof CodeReviewReport>

/**
 * Security review report.
 */
export const SecurityReviewReport = BaseReviewReport.extend({
  type: z.literal("security"),
  findings: z.array(SecurityFinding),
  /** Overall risk level */
  riskLevel: z.enum(["low", "medium", "high", "critical"]),
  /** Compliance notes */
  complianceNotes: z.array(z.string()).default([]),
})
export type SecurityReviewReport = z.infer<typeof SecurityReviewReport>

// ─────────────────────────────────────────────────────────────────────────────
// Reviewer Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Base reviewer configuration.
 */
export const BaseReviewerConfig = z.object({
  /** Focus areas for this review */
  focus: z.array(z.string()).default([]),
  /** Minimum severity to report */
  minSeverity: Severity.default("low"),
  /** Auto-approve threshold (score above this = approved) */
  autoApproveThreshold: z.number().min(0).max(100).default(90),
  /** Maximum findings before failing */
  maxFindings: z
    .object({
      critical: z.number().default(0),
      high: z.number().default(3),
      medium: z.number().default(10),
      low: z.number().default(50),
    })
    .optional(),
  /** Patterns to ignore */
  ignorePatterns: z.array(z.string()).default([]),
})
export type BaseReviewerConfig = z.infer<typeof BaseReviewerConfig>

// ─────────────────────────────────────────────────────────────────────────────
// Utility Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Calculate severity metrics from findings.
 */
export function calculateMetrics(findings: BaseFinding[]): SeverityMetrics {
  return findings.reduce(
    (acc, f) => ({
      ...acc,
      [f.severity]: acc[f.severity as Severity] + 1,
    }),
    { critical: 0, high: 0, medium: 0, low: 0, info: 0 },
  )
}

/**
 * Calculate overall score from findings (100 = perfect, 0 = terrible).
 */
export function calculateScore(findings: BaseFinding[]): number {
  if (findings.length === 0) return 100

  const totalWeight = findings.reduce((sum, f) => sum + SEVERITY_WEIGHTS[f.severity as Severity], 0)

  // Score decreases logarithmically with total weight
  // 0 weight = 100, 1000 weight ~ 0
  const score = 100 - Math.min(100, Math.log10(totalWeight + 1) * 33.3)
  return Math.round(score * 100) / 100
}

/**
 * Calculate verdict from findings and config.
 */
export function calculateVerdict(findings: BaseFinding[], config: BaseReviewerConfig): Verdict {
  const metrics = calculateMetrics(findings)

  // Critical findings = not approved
  if (metrics.critical > (config.maxFindings?.critical ?? 0)) {
    return "not_approved"
  }

  // Too many high severity = needs changes
  if (metrics.high > (config.maxFindings?.high ?? 3)) {
    return "needs_changes"
  }

  // High + medium threshold check
  if (metrics.medium > (config.maxFindings?.medium ?? 10)) {
    return "needs_changes"
  }

  const score = calculateScore(findings)

  // Auto-approve if score is high enough
  if (score >= config.autoApproveThreshold) {
    return metrics.high > 0 || metrics.medium > 0 ? "approved_with_changes" : "approved"
  }

  // Default: needs some changes
  return metrics.high > 0 || metrics.medium > 0 ? "needs_changes" : "approved_with_changes"
}

/**
 * Format findings as markdown table.
 */
export function formatFindingsMarkdown(findings: BaseFinding[]): string {
  if (findings.length === 0) return "_No findings_"

  const sorted = [...findings].sort(
    (a, b) => SEVERITY_WEIGHTS[b.severity as Severity] - SEVERITY_WEIGHTS[a.severity as Severity],
  )

  const severityEmoji: Record<Severity, string> = {
    critical: "[CRITICAL]",
    high: "[HIGH]",
    medium: "[MEDIUM]",
    low: "[LOW]",
    info: "[INFO]",
  }

  return sorted
    .map((f) => {
      const loc = f.location.line ? `${f.location.file}:${f.location.line}` : f.location.file
      return `- ${severityEmoji[f.severity as Severity]} **${f.title}** (${loc})
  ${f.description}${f.suggestion ? `\n  > Suggestion: ${f.suggestion}` : ""}`
    })
    .join("\n\n")
}

/**
 * Format report summary as markdown.
 */
export function formatReportMarkdown(report: BaseReviewReport, findings: BaseFinding[]): string {
  const { metrics, verdict, score, summary } = report

  const verdictEmoji: Record<Verdict, string> = {
    approved: "[APPROVED]",
    approved_with_changes: "[APPROVED WITH CHANGES]",
    needs_changes: "[NEEDS CHANGES]",
    not_approved: "[NOT APPROVED]",
  }

  return `## Review Report ${verdictEmoji[verdict]}

**Score**: ${score}/100
**Total Findings**: ${report.totalFindings}

### Summary
${summary}

### Severity Breakdown
| Critical | High | Medium | Low | Info |
|----------|------|--------|-----|------|
| ${metrics.critical} | ${metrics.high} | ${metrics.medium} | ${metrics.low} | ${metrics.info} |

### Findings
${formatFindingsMarkdown(findings)}
`
}

/**
 * Generate a unique finding ID.
 */
export function generateFindingId(prefix: string): string {
  const num = Math.random().toString(36).slice(2, 6).toUpperCase()
  return `${prefix}-${num}`
}

/**
 * Generate a unique report ID.
 */
export function generateReportId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 6)
  return `report_${timestamp}_${random}`
}
