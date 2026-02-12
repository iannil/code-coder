import { Identifier } from "../id/id"
import z from "zod"

export namespace DocumentSchema {
  export const Status = z.enum(["planning", "writing", "reviewing", "completed"])
  export type Status = z.infer<typeof Status>

  export const ChapterStatus = z.enum(["pending", "outlining", "drafting", "reviewing", "completed"])
  export type ChapterStatus = z.infer<typeof ChapterStatus>

  export const ChapterOutline = z.object({
    id: z.string(),
    title: z.string(),
    description: z.string(),
    estimatedWords: z.number().int().positive(),
    subsections: z.array(z.string()).optional(),
  })
  export type ChapterOutline = z.infer<typeof ChapterOutline>

  export const Outline = z.object({
    title: z.string(),
    description: z.string().optional(),
    chapters: z.array(ChapterOutline),
  })
  export type Outline = z.infer<typeof Outline>

  export const StyleGuide = z.object({
    tone: z.string().optional(),
    audience: z.string().optional(),
    format: z.enum(["markdown", "html", "plain"]).optional(),
    requirements: z.array(z.string()).optional(),
    sampleText: z.string().optional(),
  })
  export type StyleGuide = z.infer<typeof StyleGuide>

  // ============================================================================
  // New Types for Long Document Support
  // ============================================================================

  // Entity types for tracking characters, locations, concepts, etc.
  export const EntityType = z.enum(["character", "location", "concept", "item", "event"])
  export type EntityType = z.infer<typeof EntityType>

  export const Entity = z.object({
    id: z.string(),
    type: EntityType,
    name: z.string(),
    aliases: z.array(z.string()).default([]),
    description: z.string(),
    firstAppearedChapterID: z.string(),
    attributes: z.record(z.string(), z.string()).default({}),
    relationships: z
      .array(
        z.object({
          targetEntityID: z.string(),
          type: z.string(),
          description: z.string(),
        }),
      )
      .default([]),
    createdAt: z.number().int().nonnegative(),
    updatedAt: z.number().int().nonnegative(),
  })
  export type Entity = z.infer<typeof Entity>

  // Volume/Part structure for organizing long documents
  export const Volume = z.object({
    id: z.string(),
    documentID: z.string(),
    title: z.string(),
    description: z.string().optional(),
    summary: z.string().optional(),
    startChapterID: z.string(),
    endChapterID: z.string(),
    order: z.number().int().nonnegative(),
    createdAt: z.number().int().nonnegative(),
    updatedAt: z.number().int().nonnegative(),
  })
  export type Volume = z.infer<typeof Volume>

  // Global summary for long document context
  export const KeyArc = z.object({
    name: z.string(),
    description: z.string(),
    status: z.enum(["setup", "developing", "resolved", "abandoned"]),
  })
  export type KeyArc = z.infer<typeof KeyArc>

  export const GlobalSummary = z.object({
    overallPlot: z.string().max(2000),
    mainThemes: z.array(z.string()).default([]),
    writingStyle: z.string().max(1000),
    keyArcs: z.array(KeyArc).default([]),
    lastUpdated: z.number().int().nonnegative(),
  })
  export type GlobalSummary = z.infer<typeof GlobalSummary>

  // Snapshot for version management (incremental)
  export const ChapterDelta = z.object({
    chapterID: z.string(),
    action: z.enum(["created", "updated", "deleted"]),
    content: z.string().optional(),
    summary: z.string().optional(),
    wordCount: z.number().int().nonnegative().optional(),
  })
  export type ChapterDelta = z.infer<typeof ChapterDelta>

  export const EntityDelta = z.object({
    entityID: z.string(),
    action: z.enum(["created", "updated", "deleted"]),
    data: Entity.optional(),
  })
  export type EntityDelta = z.infer<typeof EntityDelta>

  export const Snapshot = z.object({
    id: z.string(),
    documentID: z.string(),
    message: z.string(),
    timestamp: z.number().int().nonnegative(),
    baselineSnapshotID: z.string().optional(),
    chapterDeltas: z.array(ChapterDelta).default([]),
    globalSummary: GlobalSummary.optional(),
    entityDeltas: z.array(EntityDelta).default([]),
    chapterCount: z.number().int().nonnegative(),
    totalWords: z.number().int().nonnegative(),
  })
  export type Snapshot = z.infer<typeof Snapshot>

  // Context budget for intelligent context selection
  export const ContextBudget = z.object({
    totalTokens: z.number().int().positive(),
    systemPromptTokens: z.number().int().nonnegative(),
    globalSummaryTokens: z.number().int().nonnegative(),
    entityTokens: z.number().int().nonnegative(),
    volumeSummaryTokens: z.number().int().nonnegative(),
    chapterSummaryTokens: z.number().int().nonnegative(),
    recentChapterTokens: z.number().int().nonnegative(),
    currentChapterTokens: z.number().int().nonnegative(),
    reservedOutputTokens: z.number().int().nonnegative(),
  })
  export type ContextBudget = z.infer<typeof ContextBudget>

  // Selected context for chapter writing
  export const SelectedContext = z.object({
    globalSummary: z.string().optional(),
    relevantEntities: z.array(Entity).default([]),
    volumeSummaries: z.array(z.object({ volume: Volume, summary: z.string() })).default([]),
    chapterSummaries: z.array(z.object({ chapterID: z.string(), title: z.string(), summary: z.string() })).default([]),
    recentChapterContent: z.string().optional(),
    currentChapterOutline: ChapterOutline,
    styleGuide: StyleGuide.optional(),
  })
  export type SelectedContext = z.infer<typeof SelectedContext>

  // Consistency check results
  export const ConsistencyIssue = z.object({
    id: z.string(),
    type: z.enum(["entity", "plot", "style", "continuity"]),
    severity: z.enum(["low", "medium", "high", "critical"]),
    description: z.string(),
    location: z.string().optional(), // chapterID or volumeID
    suggestion: z.string().optional(),
    autoFixable: z.boolean().default(false),
  })
  export type ConsistencyIssue = z.infer<typeof ConsistencyIssue>

  export const ConsistencyReport = z.object({
    documentID: z.string(),
    timestamp: z.number().int().nonnegative(),
    issues: z.array(ConsistencyIssue).default([]),
    summary: z.object({
      critical: z.number().int().nonnegative(),
      high: z.number().int().nonnegative(),
      medium: z.number().int().nonnegative(),
      low: z.number().int().nonnegative(),
    }),
  })
  export type ConsistencyReport = z.infer<typeof ConsistencyReport>

  // ============================================================================
  // Proofreader Types
  // ============================================================================

  export const ProofreaderIssueType = z.enum([
    "grammar",
    "spelling",
    "punctuation",
    "terminology",
    "style",
    "flow",
    "readability",
    "structure",
  ])
  export type ProofreaderIssueType = z.infer<typeof ProofreaderIssueType>

  export const ProofreaderIssue = z.object({
    id: z.string(),
    type: ProofreaderIssueType,
    severity: z.enum(["low", "medium", "high", "critical"]),
    description: z.string(),
    location: z
      .object({
        chapterID: z.string(),
        chapterTitle: z.string(),
        lineReference: z.string().optional(),
        excerpt: z.string().optional(),
      })
      .optional(),
    suggestion: z.string().optional(),
    autoFixable: z.boolean().default(false),
    fixedContent: z.string().optional(),
  })
  export type ProofreaderIssue = z.infer<typeof ProofreaderIssue>

  export const ProofreaderReadabilityMetrics = z.object({
    avgSentenceLength: z.number(),
    avgWordLength: z.number(),
    complexWords: z.number().int().nonnegative(),
    totalSentences: z.number().int().nonnegative(),
    totalWords: z.number().int().nonnegative(),
  })
  export type ProofreaderReadabilityMetrics = z.infer<typeof ProofreaderReadabilityMetrics>

  export const ProofreaderReport = z.object({
    id: z.string(),
    documentID: z.string(),
    timestamp: z.number().int().nonnegative(),
    scope: z.enum(["chapter", "document", "selection"]),
    chapterID: z.string().optional(),
    issues: z.array(ProofreaderIssue).default([]),
    summary: z.object({
      byType: z.record(z.string(), z.number().int().nonnegative()),
      bySeverity: z.record(z.string(), z.number().int().nonnegative()),
      autoFixable: z.number().int().nonnegative(),
    }),
    readabilityScore: z.number().min(0).max(100).optional(),
    readabilityMetrics: ProofreaderReadabilityMetrics.optional(),
  })
  export type ProofreaderReport = z.infer<typeof ProofreaderReport>

  // Extended metadata with new fields
  export const Metadata = z.object({
    id: Identifier.schema("document"),
    projectID: z.string(),
    title: z.string(),
    description: z.string().optional(),
    status: Status,
    targetWords: z.number().int().positive(),
    currentWords: z.number().int().nonnegative(),
    createdAt: z.number().int().nonnegative(),
    updatedAt: z.number().int().nonnegative(),
    outline: Outline,
    styleGuide: StyleGuide.optional(),
    // New fields for long document support
    globalSummary: GlobalSummary.optional(),
    volumes: z.array(z.string()).default([]), // volume IDs
  })
  export type Metadata = z.infer<typeof Metadata>

  // Extended chapter with volume reference
  export const Chapter = z.object({
    id: Identifier.schema("chapter"),
    documentID: Identifier.schema("document"),
    outlineID: z.string(),
    title: z.string(),
    status: ChapterStatus,
    content: z.string(),
    summary: z.string().optional(),
    wordCount: z.number().int().nonnegative(),
    createdAt: z.number().int().nonnegative(),
    updatedAt: z.number().int().nonnegative(),
    // New fields
    volumeID: z.string().optional(),
    mentionedEntityIDs: z.array(z.string()).default([]),
  })
  export type Chapter = z.infer<typeof Chapter>
}
