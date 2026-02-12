/**
 * Formal Functional Goal Schema for Verification
 *
 * Defines the structure for specifying functional goals that can be
 * formally verified through testing, property-based testing, and proofs.
 */

import { z } from "zod"

/**
 * Predicate - A logical statement that can be true or false
 */
export const PredicateSchema = z.object({
  id: z.string(),
  statement: z.string().describe("Natural language description of the predicate"),
  formal: z.string().optional().describe("Formal representation (e.g., logical notation)"),
  verification: z.enum(["test", "proof", "inspection"]),
})

export type Predicate = z.infer<typeof PredicateSchema>

/**
 * Invariant - A property that must always hold true
 */
export const InvariantSchema = z.object({
  id: z.string(),
  name: z.string(),
  statement: z.string().describe("Natural language description of the invariant"),
  formal: z.string().describe("Formal representation"),
  scope: z.enum(["function", "module", "system"]),
  violation: z.string().describe("Behavior when invariant is violated"),
})

export type Invariant = z.infer<typeof InvariantSchema>

/**
 * Property - Mathematical/algebraic property to verify
 */
export const PropertySchema = z.object({
  id: z.string(),
  name: z.string().describe("e.g., idempotency, commutativity, round-trip"),
  category: z.enum(["algebraic", "relational", "temporal"]),
  formal: z.string().describe("Formal representation"),
  verification: z.enum(["property_test", "formal_proof"]),
  priority: z.enum(["critical", "standard"]),
})

export type Property = z.infer<typeof PropertySchema>

/**
 * Acceptance Criterion - SMART criteria for feature acceptance
 */
export const AcceptanceCriterionSchema = z.object({
  id: z.string(),
  criterion: z.string().describe("Specific, Measurable, Achievable, Relevant, Time-bound"),
  threshold: z.string().describe("Threshold value for acceptance"),
  metric: z.string().describe("How to measure"),
})

export type AcceptanceCriterion = z.infer<typeof AcceptanceCriterionSchema>

/**
 * Functional Goal - Complete specification for verification
 */
export const FunctionalGoalSchema = z.object({
  id: z.string().regex(/^V-[A-Z0-9]+-[A-Z0-9]+-\d{3}$/).describe(
    "Format: V-PROJECT-FEATURE-NNN, e.g., V-CCODE-LOGIN-001"
  ),
  title: z.string(),
  description: z.string(),

  // Formal specification
  preconditions: z.array(PredicateSchema).describe("Must be true before operation"),
  postconditions: z.array(PredicateSchema).describe("Must be true after operation"),
  invariants: z.array(InvariantSchema).describe("Must always be true"),
  properties: z.array(PropertySchema).describe("Mathematical properties to verify"),

  // Acceptance criteria
  acceptance: z.array(AcceptanceCriterionSchema).describe("SMART acceptance criteria"),

  // Traceability
  requirementTrace: z.array(z.string()).describe("Links to requirements, e.g., REQ-001"),
  testTrace: z.array(z.string()).describe("Links to test cases, e.g., TC-001"),
})

export type FunctionalGoal = z.infer<typeof FunctionalGoalSchema>

/**
 * Create a functional goal ID
 */
export function createGoalId(project: string, feature: string, seq: number): string {
  const paddedSeq = seq.toString().padStart(3, "0")
  return `V-${project.toUpperCase()}-${feature.toUpperCase()}-${paddedSeq}`
}

/**
 * Default property templates for common patterns
 */
export const PropertyTemplates = {
  idempotency: (fn: string): Property => ({
    id: `PROP-IDEM-${Date.now()}`,
    name: "idempotency",
    category: "algebraic",
    formal: `forall x. ${fn}(${fn}(x)) == ${fn}(x)`,
    verification: "property_test",
    priority: "standard",
  }),

  commutative: (fn: string): Property => ({
    id: `PROP-COMM-${Date.now()}`,
    name: "commutativity",
    category: "algebraic",
    formal: `forall a,b. ${fn}(a,b) == ${fn}(b,a)`,
    verification: "property_test",
    priority: "standard",
  }),

  roundTrip: (encode: string, decode: string): Property => ({
    id: `PROP-RT-${Date.now()}`,
    name: "round_trip",
    category: "relational",
    formal: `forall x. ${decode}(${encode}(x)) == x`,
    verification: "property_test",
    priority: "critical",
  }),

  monotonic: (fn: string): Property => ({
    id: `PROP-MONO-${Date.now()}`,
    name: "monotonicity",
    category: "relational",
    formal: `x <= y implies ${fn}(x) <= ${fn}(y)`,
    verification: "property_test",
    priority: "standard",
  }),

  associativity: (fn: string): Property => ({
    id: `PROP-ASSOC-${Date.now()}`,
    name: "associativity",
    category: "algebraic",
    formal: `forall a,b,c. ${fn}(${fn}(a,b),c) == ${fn}(a,${fn}(b,c))`,
    verification: "property_test",
    priority: "standard",
  }),

  identity: (fn: string, identityVal: string): Property => ({
    id: `PROP-ID-${Date.now()}`,
    name: "identity",
    category: "algebraic",
    formal: `forall x. ${fn}(x,${identityVal}) == x && ${fn}(${identityVal},x) == x`,
    verification: "property_test",
    priority: "standard",
  }),
} as const
