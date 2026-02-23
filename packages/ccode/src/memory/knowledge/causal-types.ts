/**
 * Causal Graph Types
 *
 * Zod schemas and types for the Causal Graph system.
 * Tracks Decision → Action → Outcome chains for agent decision analysis.
 *
 * Part of Phase 16: 因果链图数据库 (Causal Graph)
 */

import z from "zod"

// ============================================================================
// Node Types
// ============================================================================

/**
 * Decision context - what information was available when making the decision
 */
export const DecisionContextSchema = z.object({
  files: z.array(z.string()).optional(),
  tools: z.array(z.string()).optional(),
  externalSources: z.array(z.string()).optional(),
  codebaseState: z.string().optional(),
})

export type DecisionContext = z.infer<typeof DecisionContextSchema>

/**
 * Decision node - records an agent's decision
 */
export const DecisionNodeSchema = z.object({
  id: z.string(),
  type: z.literal("decision"),
  sessionId: z.string(),
  agentId: z.string(),
  prompt: z.string(),
  reasoning: z.string(),
  confidence: z.number().min(0).max(1),
  timestamp: z.string(),
  context: DecisionContextSchema.optional(),
})

export type DecisionNode = z.infer<typeof DecisionNodeSchema>

/**
 * Action types
 */
export const ActionTypeSchema = z.enum([
  "code_change",
  "tool_execution",
  "api_call",
  "file_operation",
  "search",
  "other",
])

export type ActionType = z.infer<typeof ActionTypeSchema>

/**
 * Action node - records an action taken as a result of a decision
 */
export const ActionNodeSchema = z.object({
  id: z.string(),
  type: z.literal("action"),
  decisionId: z.string(),
  actionType: ActionTypeSchema,
  description: z.string(),
  input: z.record(z.string(), z.unknown()),
  output: z.record(z.string(), z.unknown()).optional(),
  timestamp: z.string(),
  duration: z.number().optional(),
})

export type ActionNode = z.infer<typeof ActionNodeSchema>

/**
 * Outcome status
 */
export const OutcomeStatusSchema = z.enum(["success", "failure", "partial"])

export type OutcomeStatus = z.infer<typeof OutcomeStatusSchema>

/**
 * Outcome metrics - quantifiable results
 */
export const OutcomeMetricsSchema = z.object({
  errorCount: z.number().optional(),
  testsPass: z.number().optional(),
  testsFail: z.number().optional(),
  coverageChange: z.number().optional(),
  linesAdded: z.number().optional(),
  linesRemoved: z.number().optional(),
  filesModified: z.number().optional(),
})

export type OutcomeMetrics = z.infer<typeof OutcomeMetricsSchema>

/**
 * Outcome node - records the result of an action
 */
export const OutcomeNodeSchema = z.object({
  id: z.string(),
  type: z.literal("outcome"),
  actionId: z.string(),
  status: OutcomeStatusSchema,
  description: z.string(),
  metrics: OutcomeMetricsSchema.optional(),
  feedback: z.string().optional(),
  timestamp: z.string(),
})

export type OutcomeNode = z.infer<typeof OutcomeNodeSchema>

// ============================================================================
// Edge Types
// ============================================================================

/**
 * Causal relationship types
 */
export const CausalRelationshipSchema = z.enum([
  "causes",
  "leads_to",
  "results_in",
  "influences",
])

export type CausalRelationship = z.infer<typeof CausalRelationshipSchema>

/**
 * Causal edge - connects nodes in the causal graph
 */
export const CausalEdgeSchema = z.object({
  id: z.string(),
  source: z.string(),
  target: z.string(),
  relationship: CausalRelationshipSchema,
  weight: z.number().min(0).max(1),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export type CausalEdge = z.infer<typeof CausalEdgeSchema>

// ============================================================================
// Graph Types
// ============================================================================

/**
 * Complete causal chain from decision to outcome
 */
export const CausalChainSchema = z.object({
  decision: DecisionNodeSchema,
  actions: z.array(ActionNodeSchema),
  outcomes: z.array(OutcomeNodeSchema),
  edges: z.array(CausalEdgeSchema),
})

export type CausalChain = z.infer<typeof CausalChainSchema>

/**
 * Full causal graph structure
 */
export const CausalGraphSchema = z.object({
  projectId: z.string(),
  nodes: z.object({
    decisions: z.array(DecisionNodeSchema),
    actions: z.array(ActionNodeSchema),
    outcomes: z.array(OutcomeNodeSchema),
  }),
  edges: z.array(CausalEdgeSchema),
  adjacencyMap: z.object({
    outgoing: z.record(z.string(), z.array(z.string())),
    incoming: z.record(z.string(), z.array(z.string())),
  }),
  time: z.object({
    created: z.number(),
    updated: z.number(),
  }),
})

export type CausalGraphData = z.infer<typeof CausalGraphSchema>

// ============================================================================
// API Request/Response Types
// ============================================================================

/**
 * Record decision request
 */
export const RecordDecisionRequestSchema = z.object({
  sessionId: z.string(),
  agentId: z.string(),
  prompt: z.string(),
  reasoning: z.string(),
  confidence: z.number().min(0).max(1),
  context: DecisionContextSchema.optional(),
})

export type RecordDecisionRequest = z.infer<typeof RecordDecisionRequestSchema>

/**
 * Record action request
 */
export const RecordActionRequestSchema = z.object({
  decisionId: z.string(),
  actionType: ActionTypeSchema,
  description: z.string(),
  input: z.record(z.string(), z.unknown()),
  output: z.record(z.string(), z.unknown()).optional(),
  duration: z.number().optional(),
})

export type RecordActionRequest = z.infer<typeof RecordActionRequestSchema>

/**
 * Record outcome request
 */
export const RecordOutcomeRequestSchema = z.object({
  actionId: z.string(),
  status: OutcomeStatusSchema,
  description: z.string(),
  metrics: OutcomeMetricsSchema.optional(),
  feedback: z.string().optional(),
})

export type RecordOutcomeRequest = z.infer<typeof RecordOutcomeRequestSchema>

/**
 * Causal pattern - recurring decision-outcome patterns
 */
export const CausalPatternSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  agentId: z.string(),
  actionType: ActionTypeSchema,
  occurrences: z.number(),
  successRate: z.number().min(0).max(1),
  avgConfidence: z.number().min(0).max(1),
  examples: z.array(z.string()),
})

export type CausalPattern = z.infer<typeof CausalPatternSchema>

/**
 * Suggestion based on historical causal data
 */
export const CausalSuggestionSchema = z.object({
  id: z.string(),
  type: z.enum(["similar_decision", "avoid_pattern", "recommended_action"]),
  confidence: z.number().min(0).max(1),
  reasoning: z.string(),
  basedOn: z.array(z.string()),
  suggestedAction: z.string().optional(),
})

export type CausalSuggestion = z.infer<typeof CausalSuggestionSchema>

/**
 * Statistics for causal graph
 */
export const CausalStatsSchema = z.object({
  totalDecisions: z.number(),
  totalActions: z.number(),
  totalOutcomes: z.number(),
  totalEdges: z.number(),
  successRate: z.number(),
  avgConfidence: z.number(),
  topAgents: z.array(
    z.object({
      agentId: z.string(),
      decisionCount: z.number(),
      successRate: z.number(),
    }),
  ),
  actionTypeDistribution: z.record(z.string(), z.number()),
})

export type CausalStats = z.infer<typeof CausalStatsSchema>

/**
 * Query for complex causal graph searches
 */
export const CausalQuerySchema = z.object({
  agentId: z.string().optional(),
  sessionId: z.string().optional(),
  actionType: ActionTypeSchema.optional(),
  status: OutcomeStatusSchema.optional(),
  dateFrom: z.string().optional(),
  dateTo: z.string().optional(),
  minConfidence: z.number().min(0).max(1).optional(),
  limit: z.number().int().min(1).max(1000).optional(),
})

export type CausalQuery = z.infer<typeof CausalQuerySchema>
