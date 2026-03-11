/**
 * Autonomous Builder Types
 *
 * Core type definitions for the self-building capability system.
 * Enables CodeCoder to detect capability gaps and autonomously
 * construct new AGENT, PROMPT, SKILL, TOOL, HAND, MEMORY, WORKFLOW concepts.
 *
 * @package autonomous/builder
 */

import z from "zod"
import type { CLOSEScore, AutonomousDecisionCriteria } from "../decision/criteria"
import type { AutonomyLevel } from "../decision/engine"

// ============================================================================
// Concept Types
// ============================================================================

/**
 * The 7 concept types that can be autonomously built
 */
export const ConceptTypeSchema = z.enum([
  "AGENT",
  "PROMPT",
  "SKILL",
  "TOOL",
  "HAND",
  "MEMORY",
  "WORKFLOW",
])
export type ConceptType = z.infer<typeof ConceptTypeSchema>

/**
 * Risk level for concept types
 */
export const ConceptRiskLevelSchema = z.enum(["low", "medium", "high"])
export type ConceptRiskLevel = z.infer<typeof ConceptRiskLevelSchema>

/**
 * Concept type metadata with risk classification
 */
export const CONCEPT_METADATA: Record<ConceptType, {
  riskLevel: ConceptRiskLevel
  requiresApproval: boolean
  autoApprovable: boolean
  description: string
}> = {
  TOOL: {
    riskLevel: "low",
    requiresApproval: false,
    autoApprovable: true,
    description: "Reusable code scripts for task automation",
  },
  PROMPT: {
    riskLevel: "low",
    requiresApproval: false,
    autoApprovable: true,
    description: "Text templates for LLM interactions",
  },
  SKILL: {
    riskLevel: "low",
    requiresApproval: false,
    autoApprovable: true,
    description: "User-invocable capabilities with structured prompts",
  },
  AGENT: {
    riskLevel: "medium",
    requiresApproval: true,
    autoApprovable: false,
    description: "Specialized AI personas with unique behaviors",
  },
  MEMORY: {
    riskLevel: "medium",
    requiresApproval: false,
    autoApprovable: true,
    description: "Structured knowledge storage schemas",
  },
  HAND: {
    riskLevel: "high",
    requiresApproval: true,
    autoApprovable: false,
    description: "Autonomous scheduled agents with execution permissions",
  },
  WORKFLOW: {
    riskLevel: "high",
    requiresApproval: true,
    autoApprovable: false,
    description: "Multi-step orchestrated processes",
  },
}

// ============================================================================
// Gap Detection Types
// ============================================================================

/**
 * Evidence supporting a capability gap
 */
export const GapEvidenceSchema = z.object({
  /** Type of evidence */
  type: z.enum(["task_failure", "search_miss", "pattern_detection", "user_request"]),
  /** Description of the evidence */
  description: z.string(),
  /** Timestamp when detected */
  timestamp: z.number(),
  /** Source context (error message, query, etc.) */
  source: z.string().optional(),
  /** Additional metadata */
  metadata: z.record(z.string(), z.unknown()).optional(),
})
export type GapEvidence = z.infer<typeof GapEvidenceSchema>

/**
 * Result of capability gap detection
 */
export const GapDetectionResultSchema = z.object({
  /** ID for tracking */
  id: z.string(),
  /** Detected concept type to build */
  type: ConceptTypeSchema,
  /** Human-readable description of the gap */
  description: z.string(),
  /** Confidence score (0-1) */
  confidence: z.number().min(0).max(1),
  /** Evidence supporting the gap */
  evidence: z.array(GapEvidenceSchema),
  /** CLOSE framework evaluation */
  closeScore: z.custom<CLOSEScore>(),
  /** Suggested name for the new concept */
  suggestedName: z.string().optional(),
  /** Technology/domain context */
  technology: z.string().optional(),
  /** Detected timestamp */
  detectedAt: z.number(),
})
export type GapDetectionResult = z.infer<typeof GapDetectionResultSchema>

// ============================================================================
// Build Context Types
// ============================================================================

/**
 * Context for building a new concept
 */
export const BuildContextSchema = z.object({
  /** Session ID */
  sessionId: z.string(),
  /** Working directory */
  workingDir: z.string().optional(),
  /** User who triggered the build */
  triggeredBy: z.enum(["agent", "user", "system"]),
  /** Related task description */
  taskDescription: z.string().optional(),
  /** Error message if triggered by failure */
  errorMessage: z.string().optional(),
  /** Technology context */
  technology: z.string().optional(),
  /** Web sources consulted */
  webSources: z.array(z.object({
    url: z.string(),
    title: z.string().optional(),
    summary: z.string().optional(),
  })).optional(),
  /** Existing similar concepts found */
  similarConcepts: z.array(z.object({
    name: z.string(),
    type: ConceptTypeSchema,
    similarity: z.number(),
  })).optional(),
})
export type BuildContext = z.infer<typeof BuildContextSchema>

/**
 * Constraints for the build process
 */
export const BuildConstraintsSchema = z.object({
  /** Maximum tokens to spend */
  maxTokens: z.number().optional(),
  /** Maximum cost in USD */
  maxCostUsd: z.number().optional(),
  /** Timeout in milliseconds */
  timeoutMs: z.number().optional(),
  /** Autonomy level gate */
  autonomyLevel: z.custom<AutonomyLevel>().optional(),
  /** Skip approval for this build */
  skipApproval: z.boolean().optional(),
  /** Prefer minimal implementation */
  preferMinimal: z.boolean().optional(),
})
export type BuildConstraints = z.infer<typeof BuildConstraintsSchema>

/**
 * Complete build request
 */
export const BuildRequestSchema = z.object({
  /** Gap detection result */
  gap: GapDetectionResultSchema,
  /** Build context */
  context: BuildContextSchema,
  /** Build constraints */
  constraints: BuildConstraintsSchema.optional(),
})
export type BuildRequest = z.infer<typeof BuildRequestSchema>

// ============================================================================
// Generator Types
// ============================================================================

/**
 * Input for concept generators
 */
export const GeneratorInputSchema = z.object({
  /** Gap to address */
  gap: GapDetectionResultSchema,
  /** Build context */
  context: BuildContextSchema,
  /** Existing concepts to avoid conflicts */
  existingConcepts: z.array(z.string()),
  /** Additional hints or requirements */
  hints: z.record(z.string(), z.unknown()).optional(),
})
export type GeneratorInput = z.infer<typeof GeneratorInputSchema>

/**
 * Generated concept content
 */
export const GeneratedConceptSchema = z.object({
  /** Concept type */
  type: ConceptTypeSchema,
  /** Unique identifier/name */
  identifier: z.string(),
  /** Human-readable name */
  displayName: z.string(),
  /** Description */
  description: z.string(),
  /** Main content (code, prompt text, config JSON) */
  content: z.string(),
  /** File path where it should be stored */
  targetPath: z.string(),
  /** Additional files to create */
  additionalFiles: z.array(z.object({
    path: z.string(),
    content: z.string(),
  })).optional(),
  /** Metadata */
  metadata: z.object({
    generatedAt: z.number(),
    generatedBy: z.string(),
    version: z.string(),
    tags: z.array(z.string()).optional(),
    dependencies: z.array(z.string()).optional(),
  }),
})
export type GeneratedConcept = z.infer<typeof GeneratedConceptSchema>

/**
 * Generator interface
 */
export interface ConceptGenerator {
  /** Concept type this generator handles */
  conceptType: ConceptType
  /** Generate a new concept */
  generate(input: GeneratorInput): Promise<GeneratedConcept>
  /** Validate input before generation */
  validateInput?(input: GeneratorInput): Promise<{ valid: boolean; issues?: string[] }>
}

// ============================================================================
// Validation Types
// ============================================================================

/**
 * Validation result for generated concepts
 */
export const ValidationResultSchema = z.object({
  /** Whether validation passed */
  success: z.boolean(),
  /** Validation errors */
  errors: z.array(z.object({
    code: z.string(),
    message: z.string(),
    field: z.string().optional(),
  })).optional(),
  /** Validation warnings (non-blocking) */
  warnings: z.array(z.object({
    code: z.string(),
    message: z.string(),
    field: z.string().optional(),
  })).optional(),
  /** Quality score (0-100) */
  qualityScore: z.number().min(0).max(100).optional(),
})
export type ValidationResult = z.infer<typeof ValidationResultSchema>

/**
 * Validator interface
 */
export interface ConceptValidator {
  /** Validate a generated concept */
  validate(concept: GeneratedConcept): Promise<ValidationResult>
}

// ============================================================================
// Registration Types
// ============================================================================

/**
 * Registration result
 */
export const RegistrationResultSchema = z.object({
  /** Whether registration succeeded */
  success: z.boolean(),
  /** Registered concept ID */
  conceptId: z.string().optional(),
  /** Path where concept was stored */
  storagePath: z.string().optional(),
  /** Error message if failed */
  error: z.string().optional(),
  /** Whether a backup was created */
  backupCreated: z.boolean().optional(),
  /** Backup path if created */
  backupPath: z.string().optional(),
})
export type RegistrationResult = z.infer<typeof RegistrationResultSchema>

/**
 * Registrar interface
 */
export interface ConceptRegistrar {
  /** Register a validated concept */
  register(concept: GeneratedConcept): Promise<RegistrationResult>
  /** Unregister/remove a concept */
  unregister(conceptId: string): Promise<boolean>
}

// ============================================================================
// Build Result Types
// ============================================================================

/**
 * Build phase status
 */
export const BuildPhaseStatusSchema = z.enum([
  "pending",
  "in_progress",
  "completed",
  "failed",
  "skipped",
])
export type BuildPhaseStatus = z.infer<typeof BuildPhaseStatusSchema>

/**
 * Build phase result
 */
export const BuildPhaseResultSchema = z.object({
  /** Phase name */
  phase: z.enum(["detection", "evaluation", "generation", "validation", "approval", "registration"]),
  /** Phase status */
  status: BuildPhaseStatusSchema,
  /** Duration in ms */
  durationMs: z.number(),
  /** Phase output */
  output: z.unknown().optional(),
  /** Error if failed */
  error: z.string().optional(),
})
export type BuildPhaseResult = z.infer<typeof BuildPhaseResultSchema>

/**
 * Complete build result
 */
export const BuildResultSchema = z.object({
  /** Whether build succeeded */
  success: z.boolean(),
  /** Built concept (if successful) */
  concept: GeneratedConceptSchema.optional(),
  /** Gap that was addressed */
  gap: GapDetectionResultSchema,
  /** CLOSE score for the build decision */
  closeScore: z.custom<CLOSEScore>(),
  /** Phases executed */
  phases: z.array(BuildPhaseResultSchema),
  /** Total duration in ms */
  durationMs: z.number(),
  /** Summary message */
  summary: z.string(),
  /** Whether approval was required */
  approvalRequired: z.boolean(),
  /** Whether approval was granted */
  approvalGranted: z.boolean().optional(),
  /** Registration result */
  registration: RegistrationResultSchema.optional(),
})
export type BuildResult = z.infer<typeof BuildResultSchema>

// ============================================================================
// Autonomy Gate Types
// ============================================================================

/**
 * Mapping of autonomy levels to allowed concept types
 */
export const AUTONOMY_CONCEPT_GATES: Record<AutonomyLevel, ConceptType[]> = {
  lunatic: ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"],
  insane: ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY"],
  crazy: ["PROMPT", "SKILL", "TOOL", "HAND"],
  wild: ["PROMPT", "SKILL", "TOOL"],
  bold: ["PROMPT", "TOOL"],
  timid: ["TOOL"],
}

/**
 * Check if a concept type is allowed at the given autonomy level
 */
export function isConceptAllowed(conceptType: ConceptType, autonomyLevel: AutonomyLevel): boolean {
  return AUTONOMY_CONCEPT_GATES[autonomyLevel].includes(conceptType)
}

/**
 * Get the minimum autonomy level required for a concept type
 */
export function getMinimumAutonomyLevel(conceptType: ConceptType): AutonomyLevel {
  const levels: AutonomyLevel[] = ["timid", "bold", "wild", "crazy", "insane", "lunatic"]
  for (const level of levels) {
    if (AUTONOMY_CONCEPT_GATES[level].includes(conceptType)) {
      return level
    }
  }
  return "lunatic"
}

// ============================================================================
// Decision Template for Self-Building
// ============================================================================

/**
 * Create CLOSE criteria for self-building decision
 */
export function createSelfBuildingCriteria(
  gap: GapDetectionResult,
  context: BuildContext,
): Partial<AutonomousDecisionCriteria> {
  const conceptMeta = CONCEPT_METADATA[gap.type]

  // Base scores adjusted by concept risk level
  const riskAdjustment = conceptMeta.riskLevel === "low" ? 0 : conceptMeta.riskLevel === "medium" ? -1 : -2

  return {
    type: "feature",
    description: `Build new ${gap.type}: ${gap.description}`,
    riskLevel: conceptMeta.riskLevel === "low" ? "low" : conceptMeta.riskLevel === "medium" ? "medium" : "high",
    // Convergence: Lower risk concepts are more reversible
    convergence: Math.max(1, 7 + riskAdjustment),
    // Leverage: High because building once enables reuse
    leverage: 8,
    // Optionality: Building a concept doesn't lock us in
    optionality: Math.max(3, 8 + riskAdjustment),
    // Surplus: Depends on confidence and existing resources
    surplus: Math.round(gap.confidence * 8),
    // Evolution: Always learning value in building new concepts
    evolution: 7,
    metadata: {
      gapId: gap.id,
      conceptType: gap.type,
      confidence: gap.confidence,
      autoApprovable: conceptMeta.autoApprovable,
    },
  }
}
