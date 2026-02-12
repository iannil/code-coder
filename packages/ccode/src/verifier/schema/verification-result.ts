/**
 * Verification Result Schema
 *
 * Defines the structure of verification results produced by the verifier agent.
 */

import { z } from "zod"

/**
 * Status of a single verification item
 */
export const VerificationStatusSchema = z.enum([
  "pass",
  "fail",
  "skip",
  "warn",
  "blocked",
])

export type VerificationStatus = z.infer<typeof VerificationStatusSchema>

/**
 * Evidence for a verification result
 */
export const EvidenceSchema = z.object({
  type: z.enum(["test", "proof", "inspection", "property_test"]),
  source: z.string().describe("File path or reference"),
  excerpt: z.string().optional().describe("Relevant code or output excerpt"),
  timestamp: z.string().datetime().optional(),
})

export type Evidence = z.infer<typeof EvidenceSchema>

/**
 * Result of verifying a predicate (pre/post condition)
 */
export const PredicateResultSchema = z.object({
  id: z.string(),
  statement: z.string(),
  status: VerificationStatusSchema,
  evidence: z.array(EvidenceSchema),
  notes: z.string().optional(),
})

export type PredicateResult = z.infer<typeof PredicateResultSchema>

/**
 * Result of verifying an invariant
 */
export const InvariantResultSchema = z.object({
  id: z.string(),
  name: z.string(),
  status: VerificationStatusSchema,
  scope: z.enum(["function", "module", "system"]),
  violations: z.array(z.string()).describe("Observed violations, if any"),
  evidence: z.array(EvidenceSchema),
})

export type InvariantResult = z.infer<typeof InvariantResultSchema>

/**
 * Result of verifying a property
 */
export const PropertyResultSchema = z.object({
  id: z.string(),
  name: z.string(),
  category: z.enum(["algebraic", "relational", "temporal"]),
  status: VerificationStatusSchema,
  formal: z.string(),
  proofMethod: z.enum(["property_test", "formal_proof", "counterexample"]),
  counterexample: z.string().optional().describe("If disproved, the counterexample"),
  evidence: z.array(EvidenceSchema),
})

export type PropertyResult = z.infer<typeof PropertyResultSchema>

/**
 * Result of verifying acceptance criteria
 */
export const AcceptanceResultSchema = z.object({
  id: z.string(),
  criterion: z.string(),
  threshold: z.string(),
  measured: z.string().optional().describe("Actual measured value"),
  status: VerificationStatusSchema,
  evidence: z.array(EvidenceSchema),
})

export type AcceptanceResult = z.infer<typeof AcceptanceResultSchema>

/**
 * Coverage analysis result
 */
export const CoverageAnalysisSchema = z.object({
  requirementCoverage: z.number().describe("Percentage of requirements covered by tests"),
  testCoverage: z.number().describe("Code coverage percentage"),
  propertyCoverage: z.number().describe("Percentage of properties verified"),
  uncoveredRequirements: z.array(z.string()),
  partiallyCoveredRequirements: z.array(z.string()),
})

export type CoverageAnalysis = z.infer<typeof CoverageAnalysisSchema>

/**
 * Requirement-test matrix entry
 */
export const MatrixEntrySchema = z.object({
  requirementId: z.string(),
  testCases: z.array(z.string()).describe("Test case IDs that cover this requirement"),
  status: VerificationStatusSchema,
  coverage: z.enum(["none", "partial", "full"]),
})

export type MatrixEntry = z.infer<typeof MatrixEntrySchema>

/**
 * Issue discovered during verification
 */
export const IssueSchema = z.object({
  id: z.string(),
  severity: z.enum(["critical", "high", "medium", "low"]),
  category: z.enum([
    "precondition_violation",
    "postcondition_violation",
    "invariant_violation",
    "property_disproven",
    "acceptance_not_met",
    "missing_test",
    "code_quality",
    "security",
  ]),
  title: z.string(),
  description: z.string(),
  location: z.string().optional().describe("File:line reference"),
  recommendation: z.string().optional(),
})

export type Issue = z.infer<typeof IssueSchema>

/**
 * Final verdict for verification
 */
export const VerdictSchema = z.enum([
  "pass",
  "pass_with_warnings",
  "fail",
  "blocked",
])

export type Verdict = z.infer<typeof VerdictSchema>

/**
 * Complete verification result for a functional goal
 */
export const VerificationResultSchema = z.object({
  goalId: z.string(),
  goalTitle: z.string(),

  // Timestamps
  verifiedAt: z.string().datetime(),
  sessionId: z.string().optional(),
  duration: z.number().describe("Verification duration in milliseconds"),

  // Component results
  preconditions: z.array(PredicateResultSchema),
  postconditions: z.array(PredicateResultSchema),
  invariants: z.array(InvariantResultSchema),
  properties: z.array(PropertyResultSchema),
  acceptance: z.array(AcceptanceResultSchema),

  // Coverage
  coverage: CoverageAnalysisSchema,
  matrix: z.array(MatrixEntrySchema),

  // Issues discovered
  issues: z.array(IssueSchema),

  // Generated tests (via tdd-guide)
  generatedTests: z.array(
    z.object({
      testId: z.string(),
      filePath: z.string(),
      target: z.string().describe("What requirement/property this test covers"),
      status: z.enum(["pending", "pass", "fail"]),
    }),
  ),

  // Final verdict
  verdict: VerdictSchema,
  summary: z.string().describe("Human-readable summary"),
})

export type VerificationResult = z.infer<typeof VerificationResultSchema>

/**
 * Create a pass verdict result
 */
export function createPassResult(goal: { id: string; title: string }): Partial<VerificationResult> {
  return {
    goalId: goal.id,
    goalTitle: goal.title,
    verifiedAt: new Date().toISOString(),
    duration: 0,
    preconditions: [],
    postconditions: [],
    invariants: [],
    properties: [],
    acceptance: [],
    coverage: {
      requirementCoverage: 0,
      testCoverage: 0,
      propertyCoverage: 0,
      uncoveredRequirements: [],
      partiallyCoveredRequirements: [],
    },
    matrix: [],
    issues: [],
    generatedTests: [],
    verdict: "pass",
    summary: "",
  }
}

/**
 * Determine verdict from component results
 */
export function determineVerdict(results: {
  preconditions: PredicateResult[]
  postconditions: PredicateResult[]
  invariants: InvariantResult[]
  properties: PropertyResult[]
  acceptance: AcceptanceResult[]
  issues: Issue[]
}): Verdict {
  const hasFail = [
    ...results.preconditions,
    ...results.postconditions,
    ...results.invariants,
    ...results.properties,
    ...results.acceptance,
  ].some((r) => r.status === "fail")

  const hasBlocked = results.issues.some((i) => i.severity === "critical")

  const hasWarnings = [
    ...results.preconditions,
    ...results.postconditions,
    ...results.invariants,
    ...results.properties,
    ...results.acceptance,
  ].some((r) => r.status === "warn")

  if (hasBlocked) return "blocked"
  if (hasFail) return "fail"
  if (hasWarnings) return "pass_with_warnings"
  return "pass"
}
