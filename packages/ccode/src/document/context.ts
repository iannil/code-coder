import z from "zod"
import { Document } from "./index"
import { DocumentSchema } from "./schema"
import * as Knowledge from "./knowledge"
import { Entity } from "./entity"
import { Volume } from "./volume"

// Default token budget for 200k context window (Claude 3.6+)
const DEFAULT_BUDGET: DocumentSchema.ContextBudget = {
  totalTokens: 200000,
  systemPromptTokens: 5000,
  globalSummaryTokens: 3000,
  entityTokens: 10000,
  volumeSummaryTokens: 5000,
  chapterSummaryTokens: 15000,
  recentChapterTokens: 20000,
  currentChapterTokens: 2000,
  reservedOutputTokens: 128000,
}

// Approximate token count (1 token ≈ 3-4 characters for Chinese, ≈ 4 characters for English)
function estimateTokens(text: string): number {
  const chineseChars = (text.match(/[\u4e00-\u9fa5]/g) || []).length
  const otherChars = text.length - chineseChars
  return Math.ceil(chineseChars / 2 + otherChars / 4)
}

function truncateToTokens(text: string, maxTokens: number): string {
  if (estimateTokens(text) <= maxTokens) return text

  const targetChars = maxTokens * 3 // Rough approximation
  if (text.length <= targetChars) return text

  return text.slice(0, targetChars) + "\n\n[Content truncated due to token limit]"
}

export namespace Context {
  export interface SelectContextOptions {
    documentID: string
    chapterID: string
    budget?: Partial<DocumentSchema.ContextBudget>
    modelContextWindow?: number
  }

  /**
   * Intelligently select context for chapter writing based on token budget
   * Implements hierarchical summary mechanism:
   * - Global summary (overall plot, themes, style)
   * - Volume summaries (if applicable)
   * - Chapter summaries (recent chapters)
   * - Recent full chapter content
   * - Relevant entities
   */
  export async function selectContextForChapter(
    options: SelectContextOptions,
  ): Promise<DocumentSchema.SelectedContext> {
    const doc = await Document.get(options.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(options.documentID, options.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const chapters = await Document.Chapter.list(options.documentID)
    const entities = await Entity.list(options.documentID)

    // Calculate current chapter index
    const currentIndex = chapters.findIndex((c) => c.id === options.chapterID)
    if (currentIndex < 0) throw new Error("Chapter not found in list")

    const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
    if (!outline) throw new Error("Chapter outline not found")

    // Merge user-provided budget with defaults
    const budget: DocumentSchema.ContextBudget = {
      ...DEFAULT_BUDGET,
      ...options.budget,
      totalTokens: options.modelContextWindow || DEFAULT_BUDGET.totalTokens,
    }

    const result: DocumentSchema.SelectedContext = {
      currentChapterOutline: outline,
      styleGuide: doc.styleGuide,
      globalSummary: undefined,
      relevantEntities: [],
      volumeSummaries: [],
      chapterSummaries: [],
      recentChapterContent: undefined,
    }

    // 1. Global Summary (if available)
    if (doc.globalSummary) {
      const summaryText = formatGlobalSummary(doc.globalSummary)
      result.globalSummary = truncateToTokens(summaryText, budget.globalSummaryTokens)
    }

    // 2. Relevant Entities (find entities mentioned in previous chapters)
    const relevantEntities = selectRelevantEntities(entities, chapters, currentIndex)
    result.relevantEntities = limitEntitiesByTokens(relevantEntities, budget.entityTokens)

    // 3. Volume Summaries (if volumes exist)
    if (doc.volumes.length > 0) {
      result.volumeSummaries = await selectVolumeSummaries(
        options.documentID,
        doc.volumes,
        chapters,
        currentIndex,
        budget.volumeSummaryTokens,
      )
    }

    // 4. Chapter Summaries (recent completed chapters)
    const recentSummaries = selectRecentChapterSummaries(chapters, currentIndex, budget.chapterSummaryTokens)
    result.chapterSummaries = recentSummaries

    // 5. Recent Full Chapter Content
    const recentContent = selectRecentChapterContent(chapters, currentIndex, budget.recentChapterTokens)
    result.recentChapterContent = recentContent

    return result
  }

  /**
   * Format selected context into a prompt string
   */
  export function formatContextForPrompt(context: DocumentSchema.SelectedContext, docTitle: string): string {
    const lines: string[] = []

    lines.push("# Writing Context")
    lines.push("")
    lines.push(`**Document:** ${docTitle}`)
    lines.push("")

    // Global Summary
    if (context.globalSummary) {
      lines.push("## Global Summary")
      lines.push("")
      lines.push(context.globalSummary)
      lines.push("")
    }

    // Volume Summaries
    if (context.volumeSummaries.length > 0) {
      lines.push("## Volume/Part Summaries")
      lines.push("")
      for (const vol of context.volumeSummaries) {
        lines.push(`### ${vol.volume.title}`)
        if (vol.summary) lines.push(vol.summary)
        lines.push("")
      }
    }

    // Relevant Entities
    if (context.relevantEntities.length > 0) {
      lines.push("## Character & Entity Reference")
      lines.push("")
      for (const entity of context.relevantEntities) {
        const emoji = {
          character: "👤",
          location: "📍",
          concept: "💡",
          item: "🔮",
          event: "📅",
        }[entity.type]

        lines.push(`${emoji} **${entity.name}** (${entity.type})`)
        lines.push(`   ${entity.description}`)

        // List aliases
        if (entity.aliases.length > 0) {
          lines.push(`   Aliases: ${entity.aliases.join(", ")}`)
        }

        // List key attributes
        if (Object.keys(entity.attributes).length > 0) {
          const attrs = Object.entries(entity.attributes)
            .slice(0, 3) // Limit to 3 most important
            .map(([k, v]) => `${k}: ${v}`)
            .join(", ")
          lines.push(`   ${attrs}`)
        }

        // List relationships
        if (entity.relationships.length > 0) {
          lines.push(`   Relationships:`)
          for (const rel of entity.relationships.slice(0, 3)) {
            lines.push(`      - ${rel.type}: ${rel.description}`)
          }
        }

        lines.push("")
      }
    }

    // Chapter Summaries
    if (context.chapterSummaries.length > 0) {
      lines.push("## Previous Chapters Summary")
      lines.push("")
      for (const ch of context.chapterSummaries) {
        lines.push(`### ${ch.title}`)
        lines.push(ch.summary)
        lines.push("")
      }
    }

    // Recent Full Content
    if (context.recentChapterContent) {
      lines.push("---")
      lines.push("")
      lines.push("## Recent Chapter (Full Content)")
      lines.push("")
      lines.push(context.recentChapterContent)
      lines.push("")
    }

    // Style Guide
    if (context.styleGuide) {
      lines.push("---")
      lines.push("")
      lines.push("## Style Guide")
      lines.push("")
      if (context.styleGuide.tone) lines.push(`**Tone:** ${context.styleGuide.tone}`)
      if (context.styleGuide.audience) lines.push(`**Audience:** ${context.styleGuide.audience}`)
      if (context.styleGuide.requirements?.length) {
        lines.push("**Requirements:**")
        for (const req of context.styleGuide.requirements) {
          lines.push(`  - ${req}`)
        }
      }
      if (context.styleGuide.sampleText) {
        lines.push("")
        lines.push("**Sample Text:**")
        lines.push("```")
        lines.push(context.styleGuide.sampleText)
        lines.push("```")
      }
      lines.push("")
    }

    return lines.join("\n")
  }

  /**
   * Get context statistics for debugging/monitoring
   */
  export async function getContextStats(options: SelectContextOptions): Promise<{
    budget: DocumentSchema.ContextBudget
    estimatedTokens: {
      globalSummary: number
      entities: number
      volumes: number
      chapters: number
      recentContent: number
      total: number
    }
  }> {
    const context = await selectContextForChapter(options)

    return {
      budget: {
        ...DEFAULT_BUDGET,
        ...options.budget,
        totalTokens: options.modelContextWindow || DEFAULT_BUDGET.totalTokens,
      },
      estimatedTokens: {
        globalSummary: estimateTokens(context.globalSummary || ""),
        entities: estimateTokens(JSON.stringify(context.relevantEntities)),
        volumes: estimateTokens(JSON.stringify(context.volumeSummaries)),
        chapters: estimateTokens(JSON.stringify(context.chapterSummaries)),
        recentContent: estimateTokens(context.recentChapterContent || ""),
        total: 0, // Calculated below
      },
    }
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  function formatGlobalSummary(summary: DocumentSchema.GlobalSummary): string {
    const lines: string[] = []

    if (summary.overallPlot) {
      lines.push(`**Overall Plot:** ${summary.overallPlot}`)
    }

    if (summary.mainThemes.length > 0) {
      lines.push(`**Main Themes:** ${summary.mainThemes.join(", ")}`)
    }

    if (summary.writingStyle) {
      lines.push(`**Writing Style:** ${summary.writingStyle}`)
    }

    if (summary.keyArcs.length > 0) {
      lines.push("**Key Story Arcs:**")
      for (const arc of summary.keyArcs) {
        lines.push(`  - ${arc.name} (${arc.status}): ${arc.description}`)
      }
    }

    return lines.join("\n")
  }

  function selectRelevantEntities(
    entities: DocumentSchema.Entity[],
    chapters: DocumentSchema.Chapter[],
    currentIndex: number,
  ): DocumentSchema.Entity[] {
    // Get entities mentioned in previous chapters
    const mentionedIDs = new Set<string>()
    for (let i = 0; i < currentIndex; i++) {
      const ch = chapters[i]
      if (ch.mentionedEntityIDs) {
        for (const id of ch.mentionedEntityIDs) {
          mentionedIDs.add(id)
        }
      }
    }

    // Also include entities from their first appearance chapter
    for (const entity of entities) {
      const entityChapterIndex = chapters.findIndex((c) => c.id === entity.firstAppearedChapterID)
      if (entityChapterIndex >= 0 && entityChapterIndex < currentIndex) {
        mentionedIDs.add(entity.id)
      }
    }

    return entities.filter((e) => mentionedIDs.has(e.id))
  }

  function limitEntitiesByTokens(
    entities: DocumentSchema.Entity[],
    maxTokens: number,
  ): DocumentSchema.Entity[] {
    const result: DocumentSchema.Entity[] = []
    let usedTokens = 0

    // Sort entities by number of relationships (more important first)
    const sorted = [...entities].sort((a, b) => b.relationships.length - a.relationships.length)

    for (const entity of sorted) {
      const entityTokens = estimateTokens(
        JSON.stringify({
          name: entity.name,
          type: entity.type,
          description: entity.description.slice(0, 200),
          attributes: Object.fromEntries(Object.entries(entity.attributes).slice(0, 3)),
          relationships: entity.relationships.slice(0, 2),
        }),
      )

      if (usedTokens + entityTokens > maxTokens) break

      result.push(entity)
      usedTokens += entityTokens
    }

    return result
  }

  async function selectVolumeSummaries(
    documentID: string,
    volumeIDs: string[],
    chapters: DocumentSchema.Chapter[],
    currentChapterIndex: number,
    maxTokens: number,
  ): Promise<Array<{ volume: DocumentSchema.Volume; summary: string }>> {
    const result: Array<{ volume: DocumentSchema.Volume; summary: string }> = []
    let usedTokens = 0

    // Get current volume and previous volumes
    const currentChapter = chapters[currentChapterIndex]

    for (const volumeID of volumeIDs) {
      const volume = await Volume.get(documentID, volumeID)
      if (!volume) continue

      // Check if volume is relevant (contains current or recent chapters)
      const volumeStartIndex = chapters.findIndex((c) => c.id === volume.startChapterID)
      const volumeEndIndex = chapters.findIndex((c) => c.id === volume.endChapterID)

      // Include current volume and immediately preceding volume
      const isCurrentVolume =
        volumeStartIndex <= currentChapterIndex && volumeEndIndex >= currentChapterIndex
      const isPreviousVolume = volumeEndIndex < currentChapterIndex

      if (!isCurrentVolume && !isPreviousVolume) continue

      const summaryTokens = estimateTokens(volume.summary || "")

      if (usedTokens + summaryTokens > maxTokens && result.length > 0) continue

      result.push({ volume, summary: volume.summary || "" })
      usedTokens += summaryTokens
    }

    return result
  }

  function selectRecentChapterSummaries(
    chapters: DocumentSchema.Chapter[],
    currentIndex: number,
    maxTokens: number,
  ): Array<{ chapterID: string; title: string; summary: string }> {
    const result: Array<{ chapterID: string; title: string; summary: string }> = []
    let usedTokens = 0

    // Get summaries from previous completed chapters, most recent first
    for (let i = currentIndex - 1; i >= 0; i--) {
      const ch = chapters[i]
      if (!ch.summary || ch.status !== "completed") continue

      const summaryTokens = estimateTokens(ch.summary)

      // Always include at least the most recent 3 summaries if available
      const minRequired = result.length < 3

      if (!minRequired && usedTokens + summaryTokens > maxTokens) break

      result.unshift({ chapterID: ch.id, title: ch.title, summary: ch.summary })
      usedTokens += summaryTokens
    }

    return result
  }

  function selectRecentChapterContent(
    chapters: DocumentSchema.Chapter[],
    currentIndex: number,
    maxTokens: number,
  ): string | undefined {
    // Get the most recent completed chapter's full content
    for (let i = currentIndex - 1; i >= 0; i--) {
      const ch = chapters[i]
      if (ch.status === "completed" && ch.content) {
        const tokens = estimateTokens(ch.content)
        if (tokens <= maxTokens) {
          return `## ${ch.title}\n\n${ch.content}`
        }
        return `## ${ch.title}\n\n${truncateToTokens(ch.content, maxTokens)}`
      }
    }

    return undefined
  }

  // ============================================================================
  // Knowledge-Aware Context (BookExpander)
  // ============================================================================

  /**
   * Extended context budget for expansion operations.
   */
  export const ExpansionContextBudget = z.object({
    totalTokens: z.number().int().positive(),
    systemPromptTokens: z.number().int().nonnegative(),
    globalSummaryTokens: z.number().int().nonnegative(),
    entityTokens: z.number().int().nonnegative(),
    volumeSummaryTokens: z.number().int().nonnegative(),
    chapterSummaryTokens: z.number().int().nonnegative(),
    recentChapterTokens: z.number().int().nonnegative(),
    currentChapterTokens: z.number().int().nonnegative(),
    reservedOutputTokens: z.number().int().nonnegative(),
    // Knowledge-specific budgets
    knowledgeFrameworkTokens: z.number().int().nonnegative().default(15000),
    argumentChainsTokens: z.number().int().nonnegative().default(5000),
    establishedFactsTokens: z.number().int().nonnegative().default(3000),
    thematicContextTokens: z.number().int().nonnegative().default(5000),
  })
  export type ExpansionContextBudget = z.infer<typeof ExpansionContextBudget>

  const EXPANSION_BUDGET: ExpansionContextBudget = {
    ...DEFAULT_BUDGET,
    knowledgeFrameworkTokens: 15000,
    argumentChainsTokens: 5000,
    establishedFactsTokens: 3000,
    thematicContextTokens: 5000,
  }

  /**
   * Knowledge-aware context for expansion writing.
   */
  export const KnowledgeAwareContext = z.object({
    // Base context fields
    globalSummary: z.string().optional(),
    relevantEntities: z.array(DocumentSchema.Entity).default([]),
    volumeSummaries: z.array(z.object({ volume: DocumentSchema.Volume, summary: z.string() })).default([]),
    chapterSummaries: z.array(z.object({ chapterID: z.string(), title: z.string(), summary: z.string() })).default([]),
    recentChapterContent: z.string().optional(),
    currentChapterOutline: DocumentSchema.ChapterOutline,
    styleGuide: DocumentSchema.StyleGuide.optional(),

    // Knowledge-aware fields
    knowledgeFramework: Knowledge.KnowledgeSchema.ThematicFramework.optional(),
    relevantKnowledgeNodes: z.array(Knowledge.KnowledgeSchema.KnowledgeNode).default([]),
    argumentChains: z.array(Knowledge.KnowledgeSchema.ArgumentChain).default([]),
    storyArcs: z.array(Knowledge.KnowledgeSchema.StoryArc).default([]),
    worldFramework: Knowledge.KnowledgeSchema.WorldFramework.optional(),
    establishedFacts: z.array(z.string()).default([]),
    pendingConclusions: z.array(z.string()).default([]),
    thematicPrinciples: z.array(z.string()).default([]),
  })
  export type KnowledgeAwareContext = z.infer<typeof KnowledgeAwareContext>

  /**
   * Select knowledge-aware context for chapter writing/expansion.
   */
  export async function selectKnowledgeAwareContext(options: {
    documentID: string
    chapterID: string
    budget?: Partial<ExpansionContextBudget>
    modelContextWindow?: number
  }): Promise<KnowledgeAwareContext> {
    const doc = await Document.get(options.documentID)
    if (!doc) throw new Error("Document not found")

    const chapter = await Document.Chapter.get(options.documentID, options.chapterID)
    if (!chapter) throw new Error("Chapter not found")

    const chapters = await Document.Chapter.list(options.documentID)
    const entities = await Entity.list(options.documentID)

    // Calculate current chapter index
    const currentIndex = chapters.findIndex((c) => c.id === options.chapterID)
    if (currentIndex < 0) throw new Error("Chapter not found in list")

    const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
    if (!outline) throw new Error("Chapter outline not found")

    // Merge user-provided budget with defaults
    const budget: ExpansionContextBudget = {
      ...EXPANSION_BUDGET,
      ...options.budget,
      totalTokens: options.modelContextWindow ?? EXPANSION_BUDGET.totalTokens,
    }

    const result: KnowledgeAwareContext = {
      currentChapterOutline: outline,
      styleGuide: doc.styleGuide,
      globalSummary: undefined,
      relevantEntities: [],
      volumeSummaries: [],
      chapterSummaries: [],
      recentChapterContent: undefined,

      // Knowledge-aware fields (initially empty, filled below)
      knowledgeFramework: undefined,
      relevantKnowledgeNodes: [],
      argumentChains: [],
      storyArcs: [],
      worldFramework: undefined,
      establishedFacts: [],
      pendingConclusions: [],
      thematicPrinciples: [],
    }

    // 1. Global Summary (if available)
    if (doc.globalSummary) {
      const summaryText = formatGlobalSummary(doc.globalSummary)
      result.globalSummary = truncateToTokens(summaryText, budget.globalSummaryTokens)
    }

    // 2. Relevant Entities (find entities mentioned in previous chapters)
    const relevantEntities = selectRelevantEntities(entities, chapters, currentIndex)
    result.relevantEntities = limitEntitiesByTokens(relevantEntities, budget.entityTokens)

    // 3. Volume Summaries (if volumes exist)
    if (doc.volumes.length > 0) {
      result.volumeSummaries = await selectVolumeSummaries(
        options.documentID,
        doc.volumes,
        chapters,
        currentIndex,
        budget.volumeSummaryTokens,
      )
    }

    // 4. Chapter Summaries (recent completed chapters)
    const recentSummaries = selectRecentChapterSummaries(chapters, currentIndex, budget.chapterSummaryTokens)
    result.chapterSummaries = recentSummaries

    // 5. Recent Full Chapter Content
    const recentContent = selectRecentChapterContent(chapters, currentIndex, budget.recentChapterTokens)
    result.recentChapterContent = recentContent

    // 6. Knowledge Framework (if available)
    try {
      const { KnowledgeNode } = await import("./knowledge")
      const knowledgeNodes = await KnowledgeNode.list(options.documentID)
      result.relevantKnowledgeNodes = limitKnowledgeNodesByTokens(knowledgeNodes, budget.knowledgeFrameworkTokens)

      // Get thematic framework
      const { Storage } = await import("../storage/storage")
      const frameworkKeys = await Storage.list(["document_framework", options.documentID])
      if (frameworkKeys.length > 0) {
        const framework = await Storage.read<Knowledge.KnowledgeSchema.ThematicFramework>(frameworkKeys[0])
        if (framework) {
          result.knowledgeFramework = framework
          result.thematicPrinciples = framework.corePrinciples
        }
      }
    } catch {
      // Knowledge module may not be initialized
    }

    // 7. Argument Chains (for non-fiction)
    try {
      const { Storage } = await import("../storage/storage")
      const argumentKeys = await Storage.list(["document_argument", options.documentID])
      const limitedChains: Knowledge.KnowledgeSchema.ArgumentChain[] = []
      let usedTokens = 0

      for (const key of argumentKeys.slice(0, 10)) {
        const chain = await Storage.read<Knowledge.KnowledgeSchema.ArgumentChain>(key)
        if (chain) {
          const tokens = estimateTokens(JSON.stringify(chain))
          if (usedTokens + tokens > budget.argumentChainsTokens) break
          limitedChains.push(chain)
          usedTokens += tokens
        }
      }
      result.argumentChains = limitedChains
    } catch {
      // Ignore if storage read fails
    }

    // 8. Story Arcs (for fiction)
    try {
      const { Storage } = await import("../storage/storage")
      const arcKeys = await Storage.list(["document_story", options.documentID, "arc"])
      const limitedArcs: Knowledge.KnowledgeSchema.StoryArc[] = []
      let usedTokens = 0

      for (const key of arcKeys.slice(0, 10)) {
        const arc = await Storage.read<Knowledge.KnowledgeSchema.StoryArc>(key)
        if (arc) {
          const tokens = estimateTokens(JSON.stringify(arc))
          if (usedTokens + tokens > budget.thematicContextTokens) break
          limitedArcs.push(arc)
          usedTokens += tokens
        }
      }
      result.storyArcs = limitedArcs
    } catch {
      // Ignore if storage read fails
    }

    // 9. World Framework (for fiction)
    try {
      const { Storage } = await import("../storage/storage")
      const worldKeys = await Storage.list(["document_world", options.documentID])
      if (worldKeys.length > 0) {
        const world = await Storage.read<Knowledge.KnowledgeSchema.WorldFramework>(worldKeys[0])
        if (world) {
          result.worldFramework = world
        }
      }
    } catch {
      // Ignore if storage read fails
    }

    // 10. Established Facts (derived from knowledge nodes with high confidence)
    const highConfidenceNodes = result.relevantKnowledgeNodes.filter((n) => n.confidence >= 0.9)
    result.establishedFacts = highConfidenceNodes.map((n) => n.content).slice(0, 20)

    // 11. Pending Conclusions (arguments needing support)
    result.pendingConclusions = result.argumentChains
      .filter((c) => c.status === "pending")
      .map((c) => c.conclusion)

    return result
  }

  /**
   * Format knowledge-aware context into a prompt string.
   */
  export function formatKnowledgeForPrompt(context: KnowledgeAwareContext): string {
    const lines: string[] = []

    lines.push("# Knowledge-Aware Writing Context")
    lines.push("")

    // Thematic Framework
    if (context.knowledgeFramework) {
      lines.push("## Thematic Framework")
      lines.push("")
      lines.push(`**Thesis:** ${context.knowledgeFramework.thesis}`)
      if (context.thematicPrinciples.length > 0) {
        lines.push("**Core Principles:**")
        for (const principle of context.thematicPrinciples) {
          lines.push(`  - ${principle}`)
        }
      }
      if (context.knowledgeFramework.mainThemes.length > 0) {
        lines.push(`**Main Themes:** ${context.knowledgeFramework.mainThemes.join(", ")}`)
      }
      lines.push("")
    }

    // Knowledge Nodes
    if (context.relevantKnowledgeNodes.length > 0) {
      lines.push("## Knowledge Graph")
      lines.push("")
      for (const node of context.relevantKnowledgeNodes.slice(0, 15)) {
        const emoji: Record<Knowledge.KnowledgeSchema.KnowledgeNodeType, string> = {
          principle: "📜",
          concept: "💡",
          argument: "⚖️",
          evidence: "📊",
          conclusion: "✅",
          character: "👤",
          location: "📍",
          world_rule: "🌍",
        }
        lines.push(`${emoji[node.type]} **${node.type.toUpperCase()}:** ${node.content.slice(0, 100)}${node.content.length > 100 ? "..." : ""}`)
        if (node.confidence < 1) {
          lines.push(`   Confidence: ${(node.confidence * 100).toFixed(0)}%`)
        }
      }
      lines.push("")
    }

    // Argument Chains
    if (context.argumentChains.length > 0) {
      lines.push("## Argument Structure")
      lines.push("")
      for (const chain of context.argumentChains.slice(0, 5)) {
        lines.push(`**Argument:** ${chain.premise.slice(0, 80)}...`)
        lines.push(`**Status:** ${chain.status}`)
        lines.push(`**Conclusion:** ${chain.conclusion.slice(0, 80)}...`)
        lines.push("")
      }
    }

    // Story Arcs
    if (context.storyArcs.length > 0) {
      lines.push("## Story Arcs")
      lines.push("")
      for (const arc of context.storyArcs) {
        lines.push(`**${arc.name}** (${arc.type})`)
        if (arc.description) {
          lines.push(`   ${arc.description}`)
        }
        lines.push(`   Status: ${arc.status}`)
      }
      lines.push("")
    }

    // World Framework
    if (context.worldFramework) {
      lines.push("## World Framework")
      lines.push("")
      lines.push(`**${context.worldFramework.name}**`)
      if (context.worldFramework.rules.length > 0) {
        lines.push("**Rules:**")
        for (const rule of context.worldFramework.rules.slice(0, 5)) {
          lines.push(`  - ${rule}`)
        }
      }
      if (context.worldFramework.magicSystem) {
        lines.push(`**Magic System:** ${context.worldFramework.magicSystem}`)
      }
      if (context.worldFramework.technology) {
        lines.push(`**Technology:** ${context.worldFramework.technology}`)
      }
      lines.push("")
    }

    // Established Facts
    if (context.establishedFacts.length > 0) {
      lines.push("## Established Facts")
      lines.push("")
      for (const fact of context.establishedFacts.slice(0, 10)) {
        lines.push(`- ${fact}`)
      }
      lines.push("")
    }

    // Pending Conclusions
    if (context.pendingConclusions.length > 0) {
      lines.push("## Pending Support")
      lines.push("")
      lines.push("The following conclusions need supporting arguments:")
      for (const conclusion of context.pendingConclusions) {
        lines.push(`- ${conclusion.slice(0, 100)}...`)
      }
      lines.push("")
    }

    return lines.join("\n")
  }

  /**
   * Get established facts from knowledge nodes.
   */
  export async function getEstablishedFacts(
    documentID: string,
    options: {
      minConfidence?: number
      maxCount?: number
    } = {},
  ): Promise<string[]> {
    const { minConfidence = 0.9, maxCount = 50 } = options

    const { KnowledgeNode } = await import("./knowledge")
    const nodes = await KnowledgeNode.list(documentID)
    return nodes
      .filter((n) => n.confidence >= minConfidence)
      .map((n) => n.content)
      .slice(0, maxCount)
  }

  /**
   * Validate content against knowledge framework.
   */
  export async function validateAgainstKnowledge(
    content: string,
    framework: Knowledge.KnowledgeSchema.ThematicFramework,
  ): Promise<Knowledge.KnowledgeSchema.ExtendedConsistencyIssue[]> {
    const issues: Knowledge.KnowledgeSchema.ExtendedConsistencyIssue[] = []

    // Check for thematic alignment
    const thesisLower = framework.thesis.toLowerCase()
    const contentLower = content.toLowerCase()

    // Check if core principles are followed
    for (const principle of framework.corePrinciples) {
      const principleLower = principle.toLowerCase()
      // Simple check for potential contradictions
      if (contentLower.includes(`not ${principleLower}`) || contentLower.includes(`never ${principleLower}`)) {
        issues.push({
          id: `issue_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
          type: "thematic",
          severity: "medium",
          description: `Content may contradict principle: "${principle}"`,
          suggestion: `Review content for alignment with principle`,
          autoFixable: false,
          relatedNodeIDs: [],
        })
      }
    }

    return issues
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  function limitKnowledgeNodesByTokens(
    nodes: Knowledge.KnowledgeSchema.KnowledgeNode[],
    maxTokens: number,
  ): Knowledge.KnowledgeSchema.KnowledgeNode[] {
    const result: Knowledge.KnowledgeSchema.KnowledgeNode[] = []
    let usedTokens = 0

    for (const node of nodes) {
      const nodeTokens = estimateTokens(JSON.stringify(node))
      if (usedTokens + nodeTokens > maxTokens) break
      result.push(node)
      usedTokens += nodeTokens
    }

    return result
  }
}
