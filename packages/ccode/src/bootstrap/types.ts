import z from "zod"

/**
 * Bootstrap Flywheel Types
 *
 * Defines the core data structures for the Agent Self-Bootstrap system.
 * This system enables agents to learn from experience and crystallize
 * solutions into reusable skills.
 */
export namespace BootstrapTypes {
  /**
   * Types of skills that can be generated
   */
  export const SkillType = z.enum(["pattern", "workflow", "tool", "agent"])
  export type SkillType = z.infer<typeof SkillType>

  /**
   * Trigger types for skill generation
   */
  export const TriggerType = z.enum(["auto", "session_end", "manual", "scheduled"])
  export type TriggerType = z.infer<typeof TriggerType>

  /**
   * Verification status for skill candidates
   */
  export const VerificationStatus = z.enum(["pending", "passed", "failed"])
  export type VerificationStatus = z.infer<typeof VerificationStatus>

  /**
   * Confidence level classifications
   */
  export const ConfidenceLevel = z.enum(["experimental", "stable", "mature"])
  export type ConfidenceLevel = z.infer<typeof ConfidenceLevel>

  /**
   * Skill content depending on type
   */
  export const SkillContent = z.object({
    code: z.string().optional(),
    steps: z.array(z.string()).optional(),
    toolDefinition: z.string().optional(),
    agentPrompt: z.string().optional(),
  })
  export type SkillContent = z.infer<typeof SkillContent>

  /**
   * Source information for tracking where skill was learned
   */
  export const SkillSource = z.object({
    sessionId: z.string(),
    toolCalls: z.array(z.string()),
    problem: z.string(),
    solution: z.string(),
  })
  export type SkillSource = z.infer<typeof SkillSource>

  /**
   * Trigger information for skill generation
   */
  export const SkillTrigger = z.object({
    type: TriggerType,
    context: z.string(),
  })
  export type SkillTrigger = z.infer<typeof SkillTrigger>

  /**
   * Verification tracking
   */
  export const SkillVerification = z.object({
    status: VerificationStatus,
    attempts: z.number().int().min(0),
    lastResult: z.string().optional(),
    confidence: z.number().min(0).max(1),
    testScenarios: z.array(z.string()).optional(),
  })
  export type SkillVerification = z.infer<typeof SkillVerification>

  /**
   * Metadata for tracking skill lifecycle
   */
  export const SkillMetadata = z.object({
    created: z.number(),
    updated: z.number(),
    usageCount: z.number().int().min(0),
    successCount: z.number().int().min(0).optional(),
    failureCount: z.number().int().min(0).optional(),
    avgTokensSaved: z.number().optional(),
    avgStepsSaved: z.number().optional(),
  })
  export type SkillMetadata = z.infer<typeof SkillMetadata>

  /**
   * A skill candidate represents a potential skill extracted from
   * a session that needs verification before becoming a real skill.
   */
  export const SkillCandidate = z.object({
    id: z.string(),
    type: SkillType,
    name: z.string(),
    description: z.string(),
    trigger: SkillTrigger,
    content: SkillContent,
    source: SkillSource,
    verification: SkillVerification,
    metadata: SkillMetadata,
  })
  export type SkillCandidate = z.infer<typeof SkillCandidate>

  /**
   * Candidate store for persisting skill candidates
   */
  export const CandidateStore = z.object({
    version: z.number().int(),
    candidates: z.array(SkillCandidate),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type CandidateStore = z.infer<typeof CandidateStore>

  /**
   * Tool call record for tracking what tools were used
   */
  export const ToolCallRecord = z.object({
    id: z.string(),
    tool: z.string(),
    input: z.record(z.string(), z.any()),
    output: z.string().optional(),
    duration: z.number().optional(),
    timestamp: z.number(),
  })
  export type ToolCallRecord = z.infer<typeof ToolCallRecord>

  /**
   * Agent capabilities introspection result
   */
  export const AgentCapabilities = z.object({
    name: z.string(),
    description: z.string().optional(),
    tools: z.array(z.string()),
    skills: z.array(z.string()),
    mcpServers: z.array(z.string()),
    permissions: z.record(z.string(), z.boolean()),
    model: z
      .object({
        providerID: z.string(),
        modelID: z.string(),
      })
      .optional(),
  })
  export type AgentCapabilities = z.infer<typeof AgentCapabilities>

  /**
   * Result of checking if agent can handle a task
   */
  export const CanHandleResult = z.object({
    confident: z.boolean(),
    confidence: z.number().min(0).max(1),
    missingCapabilities: z.array(z.string()).optional(),
    suggestedResources: z.array(z.string()).optional(),
  })
  export type CanHandleResult = z.infer<typeof CanHandleResult>

  /**
   * Test scenario for verification
   */
  export const TestScenario = z.object({
    id: z.string(),
    name: z.string(),
    description: z.string(),
    input: z.string(),
    expectedBehavior: z.string(),
    result: z
      .object({
        passed: z.boolean(),
        actual: z.string().optional(),
        error: z.string().optional(),
      })
      .optional(),
  })
  export type TestScenario = z.infer<typeof TestScenario>

  /**
   * Verification result
   */
  export const VerificationResult = z.object({
    passed: z.boolean(),
    confidence: z.number().min(0).max(1),
    scenarios: z.array(TestScenario),
    error: z.string().optional(),
    corrections: z.string().optional(),
  })
  export type VerificationResult = z.infer<typeof VerificationResult>

  /**
   * Confidence calculation factors
   */
  export const ConfidenceFactors = z.object({
    verificationPassed: z.boolean(),
    usageCount: z.number(),
    successRate: z.number().min(0).max(1),
    scenarioCoverage: z.number().min(0).max(1),
    codeQuality: z.number().min(0).max(1).optional(),
    userFeedback: z.number().min(-1).max(1).optional(),
  })
  export type ConfidenceFactors = z.infer<typeof ConfidenceFactors>
}
