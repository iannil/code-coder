import { Identifier } from "../../id/id"
import z from "zod"

export namespace KnowledgeSchema {
  // ============================================================================
  // Knowledge Node Types
  // ============================================================================

  /**
   * Types of knowledge nodes in the knowledge graph.
   * - principle: Foundational truths or axioms (non-fiction)
   * - concept: Abstract ideas or defined terms
   * - argument: Logical arguments or claims
   * - evidence: Supporting facts or data
   * - conclusion: Derived conclusions or findings
   * - character: Fictional character (fiction)
   * - location: Fictional place or setting (fiction)
   * - world_rule: Rules governing the fictional world (fiction)
   */
  export const KnowledgeNodeType = z.enum([
    "principle",
    "concept",
    "argument",
    "evidence",
    "conclusion",
    "character",
    "location",
    "world_rule",
  ])
  export type KnowledgeNodeType = z.infer<typeof KnowledgeNodeType>

  /**
   * A knowledge node represents a single piece of information in the knowledge graph.
   * Nodes can be connected via derivedFrom to form knowledge chains.
   */
  export const KnowledgeNode = z.object({
    id: Identifier.schema("knowledge"),
    type: KnowledgeNodeType,
    content: z.string().min(1),
    derivedFrom: z.array(z.string()), // Source node IDs
    confidence: z.number().min(0).max(1).default(1), // 0-1 confidence level
    attributes: z.record(z.string(), z.string()).default({}),
    chapterID: z.string().optional(), // First appearance chapter
    createdAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type KnowledgeNode = z.infer<typeof KnowledgeNode>

  // ============================================================================
  // Argument Chain (Non-fiction)
  // ============================================================================

  /**
   * A single reasoning step in an argument chain.
   */
  export const ReasoningStep = z.object({
    step: z.string().min(1),
    supportsPremise: z.boolean().optional(),
    evidence: z.array(z.string()), // References to evidence node IDs
  })
  export type ReasoningStep = z.infer<typeof ReasoningStep>

  /**
   * An argument chain represents a logical progression from premise to conclusion.
   * Used primarily for non-fiction content to track论证 coherence.
   */
  export const ArgumentChain = z.object({
    id: Identifier.schema("argument"),
    premise: z.string().min(1),
    reasoningSteps: z.array(ReasoningStep).default([]),
    conclusion: z.string().min(1),
    counterArguments: z.array(z.string()), // References to argument chain IDs
    status: z.enum(["valid", "weak", "refuted", "pending"]).default("pending"),
    chapterID: z.string().optional(), // Where this argument appears
    createdAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type ArgumentChain = z.infer<typeof ArgumentChain>

  // ============================================================================
  // Story Elements (Fiction)
  // ============================================================================

  /**
   * Types of story arcs following standard narrative structure.
   */
  export const StoryArcType = z.enum([
    "setup",      // Introduction of characters and setting
    "rising",     // Conflict development
    "climax",     // Peak of tension
    "falling",    // Aftermath of climax
    "resolution", // Conclusion
  ])
  export type StoryArcType = z.infer<typeof StoryArcType>

  /**
   * A story arc represents a narrative thread through multiple chapters.
   */
  export const StoryArc = z.object({
    id: Identifier.schema("arc"),
    name: z.string().min(1),
    type: StoryArcType,
    chapters: z.array(z.string()), // Chapter IDs
    characterIDs: z.array(z.string()), // Involved character IDs
    status: z.enum(["planned", "in_progress", "resolved", "abandoned"]).default("planned"),
    description: z.string().optional(),
    createdAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type StoryArc = z.infer<typeof StoryArc>

  /**
   * Timeline event for fictional world history.
   */
  export const TimelineEvent = z.object({
    id: Identifier.schema("timeline"),
    era: z.string().min(1), // Time period (e.g., "Age of Darkness", "2023-2030")
    event: z.string().min(1),
    order: z.number().int().nonnegative(), // For sorting within era
    relatedChapterIDs: z.array(z.string()),
  })
  export type TimelineEvent = z.infer<typeof TimelineEvent>

  /**
   * World framework defines the rules and setting of a fictional world.
   */
  export const WorldFramework = z.object({
    id: Identifier.schema("world"),
    name: z.string().min(1),
    description: z.string().optional(),
    rules: z.array(z.string()), // World rules (e.g., magic system rules)
    magicSystem: z.string().optional(), // Description of magic/special powers
    technology: z.string().optional(), // Technology level description
    timeline: z.array(TimelineEvent).default([]),
    geography: z.array(z.string()).default([]), // Key locations descriptions
    createdAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type WorldFramework = z.infer<typeof WorldFramework>

  // ============================================================================
  // Thematic Framework (Universal)
  // ============================================================================

  /**
   * Knowledge graph relationships between concepts.
   * Maps concept/node IDs to their related node IDs.
   */
  export type KnowledgeGraph = Record<string, string[]>

  /**
   * Thematic framework defines the core themes and knowledge structure.
   * Applicable to both fiction and non-fiction.
   */
  export const ThematicFramework = z.object({
    id: Identifier.schema("theme"),
    thesis: z.string().min(1), // Central thesis or theme
    corePrinciples: z.array(z.string()).default([]), // Foundational principles
    mainThemes: z.array(z.string()).default([]), // Key themes explored
    knowledgeGraph: z.record(z.string(), z.array(z.string())).default({}), // Concept relationships
    argumentChainIDs: z.array(z.string()).default([]), // For non-fiction
    storyArcIDs: z.array(z.string()).default([]), // For fiction
    worldFrameworkID: z.string().optional(), // For fiction
    createdAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type ThematicFramework = z.infer<typeof ThematicFramework>

  // ============================================================================
  // Core Idea Analysis (Input Processing)
  // ============================================================================

  /**
   * Result of analyzing a core idea/extract for expansion.
   */
  export const CoreIdeaAnalysis = z.object({
    contentType: z.enum(["fiction", "nonfiction", "mixed"]),
    coreThesis: z.string().min(1),
    mainThemes: z.array(z.string()).default([]),
    suggestedWordCount: z.number().int().positive(),
    suggestedChapterCount: z.number().int().positive(),
    keyConcepts: z.array(z.string()).default([]), // Concepts that become knowledge nodes
    potentialConflicts: z.array(z.string()).default([]), // For fiction
    potentialArguments: z.array(z.string()).default([]), // For non-fiction
  })
  export type CoreIdeaAnalysis = z.infer<typeof CoreIdeaAnalysis>

  // ============================================================================
  // Document Schema (Shared Types)
  // ============================================================================

  /**
   * Chapter outline for document structure.
   */
  export const ChapterOutline: z.ZodType<{
    id: string
    title: string
    description?: string
    estimatedWords: number
    subsections?: ChapterOutline[]
  }> = z.object({
    id: z.string(),
    title: z.string().min(1),
    description: z.string().optional(),
    estimatedWords: z.number().int().positive(),
    subsections: z.array(z.lazy(() => ChapterOutline)).optional(),
  })
  export type ChapterOutline = z.infer<typeof ChapterOutline>

  /**
   * Full document outline.
   */
  export const Outline = z.object({
    title: z.string().min(1),
    description: z.string().optional(),
    chapters: z.array(ChapterOutline),
  })
  export type Outline = z.infer<typeof Outline>

  /**
   * Document-related schemas grouped under DocumentSchema namespace.
   */
  export namespace DocumentSchema {
    export type ChapterOutline = KnowledgeSchema.ChapterOutline
    export type Outline = KnowledgeSchema.Outline
  }

  // ============================================================================
  // Expansion Progress Tracking
  // ============================================================================

  /**
   * Status of the book expansion process.
   */
  export const ExpansionStatus = z.enum([
    "analyzing",     // Phase 1: Analyzing core idea
    "framework",      // Phase 2: Building knowledge/framework
    "outlining",      // Phase 3: Generating outline
    "writing",        // Phase 4: Writing chapters
    "validating",     // Phase 5: Validating consistency
    "complete",       // Expansion complete
    "failed",         // Expansion failed
  ])
  export type ExpansionStatus = z.infer<typeof ExpansionStatus>

  /**
   * Progress tracking for book expansion.
   */
  export const ExpansionProgress = z.object({
    id: Identifier.schema("expansion"),
    documentID: Identifier.schema("document"),
    status: ExpansionStatus,
    currentPhase: z.enum([
      "idea_analysis",
      "framework_building",
      "outline_generation",
      "iterative_writing",
      "consistency_validation",
    ]).optional(),
    currentChapterIndex: z.number().int().nonnegative().default(0),
    totalChapters: z.number().int().nonnegative().default(0),
    wordsWritten: z.number().int().nonnegative().default(0),
    targetWords: z.number().int().positive(),
    consistencyScore: z.number().min(0).max(1).default(0),
    lastValidatedAt: z.number().int().nonnegative().optional(),
    error: z.string().optional(),
    startedAt: z.number().int().nonnegative().default(() => Date.now()),
    updatedAt: z.number().int().nonnegative().default(() => Date.now()),
  })
  export type ExpansionProgress = z.infer<typeof ExpansionProgress>

  // ============================================================================
  // Knowledge Context for Writing
  // ============================================================================

  /**
   * Context enriched with knowledge information for AI writing.
   */
  export const KnowledgeContext = z.object({
    framework: ThematicFramework.optional(),
    relevantNodes: z.array(KnowledgeNode).default([]),
    argumentChains: z.array(ArgumentChain).default([]),
    storyArcs: z.array(StoryArc).default([]),
    worldFramework: WorldFramework.optional(),
    establishedFacts: z.array(z.string()).default([]), // Non-refutable statements
    pendingConclusions: z.array(z.string()).default([]), // Conclusions needing support
  })
  export type KnowledgeContext = z.infer<typeof KnowledgeContext>

  // ============================================================================
  // Consistency Issue Extensions
  // ============================================================================

  /**
   * Extended consistency issue types including knowledge-related issues.
   */
  export const ExtendedConsistencyIssueType = z.enum([
    "entity",        // Entity consistency
    "plot",          // Plot consistency (fiction)
    "style",         // Style consistency
    "continuity",     // Continuity
    "logical",        // Logical consistency (non-fiction)
    "argumentative",  // Argument consistency (non-fiction)
    "worldview",      // Worldview consistency (fiction)
    "thematic",      // Thematic alignment
  ])
  export type ExtendedConsistencyIssueType = z.infer<typeof ExtendedConsistencyIssueType>

  /**
   * Extended consistency issue with knowledge-related information.
   */
  export const ExtendedConsistencyIssue = z.object({
    id: z.string(),
    type: ExtendedConsistencyIssueType,
    severity: z.enum(["low", "medium", "high", "critical"]),
    description: z.string(),
    location: z.string().optional(), // chapterID or volumeID
    suggestion: z.string().optional(),
    autoFixable: z.boolean().default(false),
    relatedNodeIDs: z.array(z.string()).default([]), // Related knowledge nodes
    confidence: z.number().min(0).max(1).optional(), // Detection confidence
  })
  export type ExtendedConsistencyIssue = z.infer<typeof ExtendedConsistencyIssue>
}
