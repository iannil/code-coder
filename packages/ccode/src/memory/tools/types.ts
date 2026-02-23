/**
 * Dynamic Tool Registry Types
 *
 * Type definitions for the dynamic tool registry that stores
 * tools learned from successful code executions.
 *
 * Part of Phase 12: Dynamic Tool Library
 */

import z from "zod"

export namespace ToolTypes {
  // ============================================================================
  // Tool Parameter Schema
  // ============================================================================

  export const ToolParameter = z.object({
    /** Parameter name */
    name: z.string(),
    /** Parameter type (string, number, boolean, array, object) */
    type: z.enum(["string", "number", "boolean", "array", "object"]),
    /** Description of what this parameter does */
    description: z.string(),
    /** Whether this parameter is required */
    required: z.boolean().default(true),
    /** Default value if not provided */
    default: z.any().optional(),
    /** Enum values if this is a restricted set */
    enum: z.array(z.string()).optional(),
  })
  export type ToolParameter = z.infer<typeof ToolParameter>

  // ============================================================================
  // Tool Example Schema
  // ============================================================================

  export const ToolExample = z.object({
    /** Example description */
    description: z.string(),
    /** Input parameters for this example */
    input: z.record(z.string(), z.any()),
    /** Expected or actual output */
    output: z.string().optional(),
  })
  export type ToolExample = z.infer<typeof ToolExample>

  // ============================================================================
  // Tool Metadata Schema
  // ============================================================================

  export const ToolMetadata = z.object({
    /** When the tool was created */
    createdAt: z.number(),
    /** When the tool was last updated */
    updatedAt: z.number(),
    /** Who created the tool (agent/user) */
    createdBy: z.enum(["agent", "user"]),
    /** Source task ID that generated this tool */
    sourceTask: z.string().optional(),
    /** Version number for updates */
    version: z.number().default(1),
  })
  export type ToolMetadata = z.infer<typeof ToolMetadata>

  // ============================================================================
  // Tool Statistics Schema
  // ============================================================================

  export const ToolStats = z.object({
    /** Total number of times this tool has been used */
    usageCount: z.number().default(0),
    /** Number of successful executions */
    successCount: z.number().default(0),
    /** Number of failed executions */
    failureCount: z.number().default(0),
    /** When the tool was last used */
    lastUsedAt: z.number().nullable().default(null),
    /** Average execution time in milliseconds */
    averageExecutionTime: z.number().default(0),
  })
  export type ToolStats = z.infer<typeof ToolStats>

  // ============================================================================
  // Dynamic Tool Schema
  // ============================================================================

  export const DynamicTool = z.object({
    /** Unique tool identifier */
    id: z.string(),
    /** Human-readable tool name */
    name: z.string(),
    /** Description of what the tool does (used for semantic search) */
    description: z.string(),
    /** Category tags for filtering */
    tags: z.array(z.string()).default([]),
    /** The actual code/script content */
    code: z.string(),
    /** Programming language of the code */
    language: z.enum(["python", "nodejs", "bash"]),
    /** Tool parameters */
    parameters: z.array(ToolParameter).default([]),
    /** Usage examples */
    examples: z.array(ToolExample).default([]),
    /** Tool metadata */
    metadata: ToolMetadata,
    /** Usage statistics */
    stats: ToolStats,
    /** Vector embedding for semantic search (optional, generated on demand) */
    embedding: z.array(z.number()).optional(),
  })
  export type DynamicTool = z.infer<typeof DynamicTool>

  // ============================================================================
  // Input Types
  // ============================================================================

  export const CreateToolInput = z.object({
    /** Tool name */
    name: z.string().min(1).max(100),
    /** Tool description */
    description: z.string().min(1).max(2000),
    /** Tags for categorization */
    tags: z.array(z.string()).default([]),
    /** Tool code */
    code: z.string().min(1),
    /** Language */
    language: z.enum(["python", "nodejs", "bash"]),
    /** Parameters */
    parameters: z.array(ToolParameter).default([]),
    /** Examples */
    examples: z.array(ToolExample).default([]),
    /** Who is creating this */
    createdBy: z.enum(["agent", "user"]).default("agent"),
    /** Source task ID */
    sourceTask: z.string().optional(),
  })
  export type CreateToolInput = z.infer<typeof CreateToolInput>

  export const UpdateToolInput = z.object({
    /** Tool name */
    name: z.string().min(1).max(100).optional(),
    /** Tool description */
    description: z.string().min(1).max(2000).optional(),
    /** Tags */
    tags: z.array(z.string()).optional(),
    /** Tool code */
    code: z.string().min(1).optional(),
    /** Parameters */
    parameters: z.array(ToolParameter).optional(),
    /** Examples */
    examples: z.array(ToolExample).optional(),
  })
  export type UpdateToolInput = z.infer<typeof UpdateToolInput>

  // ============================================================================
  // Search Types
  // ============================================================================

  export const SearchOptions = z.object({
    /** Maximum number of results */
    limit: z.number().int().positive().default(10),
    /** Minimum similarity score (0-1) */
    minScore: z.number().min(0).max(1).default(0.3),
    /** Filter by tags */
    tags: z.array(z.string()).optional(),
    /** Filter by language */
    language: z.enum(["python", "nodejs", "bash"]).optional(),
  })
  export type SearchOptions = z.infer<typeof SearchOptions>

  export const ScoredTool = z.object({
    /** The tool */
    tool: DynamicTool,
    /** Similarity score (0-1) */
    score: z.number(),
  })
  export type ScoredTool = z.infer<typeof ScoredTool>

  // ============================================================================
  // Execution Types (for learning)
  // ============================================================================

  export const ExecutionRecord = z.object({
    /** The executed code */
    code: z.string(),
    /** Programming language */
    language: z.enum(["python", "nodejs", "bash"]),
    /** Task description that led to this execution */
    task: z.string(),
    /** Execution output (stdout) */
    output: z.string(),
    /** Exit code (0 = success) */
    exitCode: z.number(),
    /** Execution duration in ms */
    durationMs: z.number().optional(),
  })
  export type ExecutionRecord = z.infer<typeof ExecutionRecord>

  // ============================================================================
  // LLM Abstraction Types (Phase 17)
  // ============================================================================

  /**
   * Parameter extracted by LLM analysis
   */
  export const LLMExtractedParameter = z.object({
    /** Parameter name (snake_case) */
    name: z.string(),
    /** Parameter type */
    type: z.enum(["string", "number", "boolean", "array", "object"]),
    /** Description of what this parameter does */
    description: z.string(),
    /** Whether this parameter is required */
    required: z.boolean(),
    /** Default value if any */
    defaultValue: z.unknown().optional(),
    /** Where this was extracted from (line number or 'hardcoded') */
    extractedFrom: z.string(),
  })
  export type LLMExtractedParameter = z.infer<typeof LLMExtractedParameter>

  /**
   * Hardcoded value detected by LLM that should potentially be parameterized
   */
  export const HardcodedValue = z.object({
    /** The actual hardcoded value */
    value: z.string(),
    /** Line number where it appears */
    line: z.number(),
    /** Whether this should be turned into a parameter */
    shouldParameterize: z.boolean(),
    /** Suggested parameter name */
    suggestedParamName: z.string(),
  })
  export type HardcodedValue = z.infer<typeof HardcodedValue>

  /**
   * Usage example generated by LLM
   */
  export const LLMGeneratedExample = z.object({
    /** Description of this example use case */
    description: z.string(),
    /** Input parameters for this example */
    input: z.record(z.string(), z.unknown()),
    /** Expected output or behavior */
    expectedOutput: z.string(),
  })
  export type LLMGeneratedExample = z.infer<typeof LLMGeneratedExample>

  /**
   * Complete LLM analysis result for code abstraction
   */
  export const LLMAnalysisResult = z.object({
    /** One-line description of what the code does */
    purpose: z.string(),
    /** Suggested tool name (snake_case) */
    toolName: z.string(),
    /** Extracted parameters */
    parameters: z.array(LLMExtractedParameter),
    /** Detected hardcoded values */
    hardcodedValues: z.array(HardcodedValue),
    /** Generated usage examples */
    examples: z.array(LLMGeneratedExample),
    /** Generalized code with hardcoded values replaced by parameters */
    generalizedCode: z.string().optional(),
  })
  export type LLMAnalysisResult = z.infer<typeof LLMAnalysisResult>

  // ============================================================================
  // Registry Statistics
  // ============================================================================

  export const RegistryStats = z.object({
    /** Total number of tools */
    totalTools: z.number(),
    /** Tools by language */
    byLanguage: z.record(z.string(), z.number()),
    /** Tools by tag */
    byTag: z.record(z.string(), z.number()),
    /** Most used tools */
    mostUsed: z.array(
      z.object({
        id: z.string(),
        name: z.string(),
        usageCount: z.number(),
      }),
    ),
    /** Recently added tools */
    recentlyAdded: z.array(
      z.object({
        id: z.string(),
        name: z.string(),
        createdAt: z.number(),
      }),
    ),
    /** Last updated timestamp */
    lastUpdated: z.number(),
  })
  export type RegistryStats = z.infer<typeof RegistryStats>
}
