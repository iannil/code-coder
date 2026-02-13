import { Identifier } from "../../id/id"
import { Storage } from "../../storage/storage"
import { KnowledgeSchema } from "./schema"

export namespace StoryElements {
  const STORAGE_PREFIX = ["document_story"]
  const WORLD_STORAGE_PREFIX = ["document_world"]

  // ============================================================================
  // Story Arc Management
  // ============================================================================

  /**
   * Generate a unique story arc ID.
   */
  export function createArcID(): string {
    return Identifier.create("arc", false)
  }

  /**
   * Create a new story arc.
   */
  export async function createStoryArc(
    documentID: string,
    input: Omit<KnowledgeSchema.StoryArc, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeSchema.StoryArc> {
    const now = Date.now()
    const arc: KnowledgeSchema.StoryArc = {
      id: createArcID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    }

    await storage.write([...STORAGE_PREFIX, documentID, "arc", arc.id], arc)
    return arc
  }

  /**
   * Get a story arc by ID.
   */
  export async function getStoryArc(
    documentID: string,
    arcID: string,
  ): Promise<KnowledgeSchema.StoryArc | null> {
    try {
      return await storage.read<KnowledgeSchema.StoryArc>([...STORAGE_PREFIX, documentID, "arc", arcID])
    } catch {
      return null
    }
  }

  /**
   * Get all story arcs for a document.
   */
  export async function listStoryArcs(documentID: string): Promise<KnowledgeSchema.StoryArc[]> {
    const keys = await storage.list([...STORAGE_PREFIX, documentID, "arc"])
    const arcs: KnowledgeSchema.StoryArc[] = []

    for (const key of keys) {
      try {
        const arc = await storage.read<KnowledgeSchema.StoryArc>(key)
        if (arc) arcs.push(arc)
      } catch {
        // Skip invalid entries
      }
    }

    return arcs.sort((a, b) => b.createdAt - a.createdAt)
  }

  /**
   * Get story arcs by type.
   */
  export async function listStoryArcsByType(
    documentID: string,
    type: KnowledgeSchema.StoryArcType,
  ): Promise<KnowledgeSchema.StoryArc[]> {
    const arcs = await listStoryArcs(documentID)
    return arcs.filter((a) => a.type === type)
  }

  /**
   * Update a story arc.
   */
  export async function updateStoryArc(
    documentID: string,
    arcID: string,
    updates: Partial<Omit<KnowledgeSchema.StoryArc, "id" | "createdAt">>,
  ): Promise<KnowledgeSchema.StoryArc | null> {
    const existing = await getStoryArc(documentID, arcID)
    if (!existing) return null

    const updated: KnowledgeSchema.StoryArc = {
      ...existing,
      ...updates,
      id: existing.id,
      createdAt: existing.createdAt,
      updatedAt: Date.now(),
    }

    await storage.write([...STORAGE_PREFIX, documentID, "arc", arcID], updated)
    return updated
  }

  /**
   * Add a chapter to a story arc.
   */
  export async function addChapterToArc(
    documentID: string,
    arcID: string,
    chapterID: string,
  ): Promise<void> {
    const arc = await getStoryArc(documentID, arcID)
    if (!arc) throw new Error("Story arc not found")

    if (!arc.chapters.includes(chapterID)) {
      await updateStoryArc(documentID, arcID, {
        chapters: [...arc.chapters, chapterID],
      })
    }
  }

  /**
   * Complete a story arc.
   */
  export async function completeStoryArc(
    documentID: string,
    arcID: string,
  ): Promise<KnowledgeSchema.StoryArc | null> {
    return updateStoryArc(documentID, arcID, {
      status: "resolved",
    })
  }

  /**
   * Delete a story arc.
   */
  export async function removeStoryArc(documentID: string, arcID: string): Promise<boolean> {
    await storage.remove([...STORAGE_PREFIX, documentID, "arc", arcID])
    return true
  }

  // ============================================================================
  // World Framework Management
  // ============================================================================

  /**
   * Generate a unique world framework ID.
   */
  export function createWorldID(): string {
    return Identifier.create("world", false)
  }

  /**
   * Create a new world framework.
   */
  export async function createWorldFramework(
    documentID: string,
    input: Omit<KnowledgeSchema.WorldFramework, "id" | "createdAt" | "updatedAt">,
  ): Promise<KnowledgeSchema.WorldFramework> {
    const now = Date.now()
    const world: KnowledgeSchema.WorldFramework = {
      id: createWorldID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    }

    await storage.write([...WORLD_STORAGE_PREFIX, documentID, world.id], world)
    return world
  }

  /**
   * Get a world framework by ID.
   */
  export async function getWorldFramework(
    documentID: string,
    worldID: string,
  ): Promise<KnowledgeSchema.WorldFramework | null> {
    try {
      return await storage.read<KnowledgeSchema.WorldFramework>([...WORLD_STORAGE_PREFIX, documentID, worldID])
    } catch {
      return null
    }
  }

  /**
   * Get all world frameworks for a document.
   */
  export async function listWorldFrameworks(documentID: string): Promise<KnowledgeSchema.WorldFramework[]> {
    const keys = await storage.list([...WORLD_STORAGE_PREFIX, documentID])
    const worlds: KnowledgeSchema.WorldFramework[] = []

    for (const key of keys) {
      try {
        const world = await storage.read<KnowledgeSchema.WorldFramework>(key)
        if (world) worlds.push(world)
      } catch {
        // Skip invalid entries
      }
    }

    return worlds.sort((a, b) => b.createdAt - a.createdAt)
  }

  /**
   * Update a world framework.
   */
  export async function updateWorldFramework(
    documentID: string,
    worldID: string,
    updates: Partial<Omit<KnowledgeSchema.WorldFramework, "id" | "createdAt">>,
  ): Promise<KnowledgeSchema.WorldFramework | null> {
    const existing = await getWorldFramework(documentID, worldID)
    if (!existing) return null

    const updated: KnowledgeSchema.WorldFramework = {
      ...existing,
      ...updates,
      id: existing.id,
      createdAt: existing.createdAt,
      updatedAt: Date.now(),
    }

    await storage.write([...WORLD_STORAGE_PREFIX, documentID, worldID], updated)
    return updated
  }

  /**
   * Add a timeline event to a world framework.
   */
  export async function addTimelineEvent(
    documentID: string,
    worldID: string,
    event: Omit<KnowledgeSchema.TimelineEvent, "id">,
  ): Promise<KnowledgeSchema.WorldFramework | null> {
    const world = await getWorldFramework(documentID, worldID)
    if (!world) return null

    const newEvent: KnowledgeSchema.TimelineEvent = {
      id: Identifier.create("timeline", false),
      ...event,
    }

    return updateWorldFramework(documentID, worldID, {
      timeline: [...world.timeline, newEvent].sort((a, b) => a.order - b.order),
    })
  }

  // ============================================================================
  // Narrative Structure Analysis
  // ============================================================================

  /**
   * Analyze narrative structure across chapters.
   */
  export async function analyzeNarrativeStructure(input: {
    documentID: string
    chapters: Array<{ id: string; title: string; summary?: string; content: string }>
  }): Promise<{
    arcProgression: Array<{
      arcType: KnowledgeSchema.StoryArcType
      startChapter: number
      endChapter: number
      confidence: number
    }>
    recommendations: Array<{
      type: "add_conflict" | "develop_arc" | "resolve_arc" | "check_pacing"
      chapterIndex: number
      description: string
    }>
  }> {
    const { chapters } = input
    const arcProgression: Array<{
      arcType: KnowledgeSchema.StoryArcType
      startChapter: number
      endChapter: number
      confidence: number
    }> = []
    const recommendations: Array<{
      type: "add_conflict" | "develop_arc" | "resolve_arc" | "check_pacing"
      chapterIndex: number
      description: string
    }> = []

    // Analyze each chapter for arc type indicators
    const arcTypeScores: KnowledgeSchema.StoryArcType[][] = []
    for (let i = 0; i < chapters.length; i++) {
      const chapter = chapters[i]
      const scores = analyzeChapterForArcType(chapter)
      arcTypeScores.push(scores)
    }

    // Detect arc transitions
    let currentArc: { type: KnowledgeSchema.StoryArcType; start: number } | null = null
    for (let i = 0; i < arcTypeScores.length; i++) {
      const dominantArc = getDominantArcType(arcTypeScores[i])

      if (!currentArc || currentArc.type !== dominantArc) {
        if (currentArc) {
          arcProgression.push({
            arcType: currentArc.type,
            startChapter: currentArc.start,
            endChapter: i - 1,
            confidence: calculateArcConfidence(arcTypeScores, currentArc.start, i - 1, currentArc.type),
          })
        }
        currentArc = { type: dominantArc, start: i }
      }
    }

    if (currentArc) {
      arcProgression.push({
        arcType: currentArc.type,
        startChapter: currentArc.start,
        endChapter: chapters.length - 1,
        confidence: calculateArcConfidence(arcTypeScores, currentArc.start, chapters.length - 1, currentArc.type),
      })
    }

    // Generate recommendations
    const hasClimax = arcProgression.some((a) => a.arcType === "climax")
    if (!hasClimax && chapters.length > 5) {
      recommendations.push({
        type: "develop_arc",
        chapterIndex: Math.floor(chapters.length * 0.7),
        description: "Consider adding a climax chapter around 70% through the story",
      })
    }

    const hasResolution = arcProgression.some((a) => a.arcType === "resolution")
    if (!hasResolution && chapters.length > 3) {
      recommendations.push({
        type: "resolve_arc",
        chapterIndex: chapters.length - 1,
        description: "Story may benefit from a clear resolution chapter",
      })
    }

    // Check pacing
    const avgChapterLength = chapters.reduce((sum, ch) => sum + ch.content.length, 0) / chapters.length
    for (let i = 0; i < chapters.length; i++) {
      if (chapters[i].content.length > avgChapterLength * 2) {
        recommendations.push({
          type: "check_pacing",
          chapterIndex: i,
          description: `Chapter ${i + 1} is significantly longer than average - consider splitting`,
        })
      }
    }

    return { arcProgression, recommendations }
  }

  /**
   * Validate worldview consistency across chapters.
   */
  export async function checkWorldviewConsistency(input: {
    documentID: string
    worldFramework: KnowledgeSchema.WorldFramework
    chapters: Array<{ id: string; title: string; content: string }>
  }): Promise<{
    isConsistent: boolean
    issues: Array<{
      type: string
      chapterID: string
      description: string
      severity: "low" | "medium" | "high"
    }>
  }> {
    const { worldFramework, chapters } = input
    const issues: Array<{
      type: string
      chapterID: string
      description: string
      severity: "low" | "medium" | "high"
    }> = []

    // Check for violations of world rules
    const ruleViolations: Map<string, number[]> = new Map()
    for (const rule of worldFramework.rules) {
      const ruleKeywords = extractKeywords(rule)

      for (const chapter of chapters) {
        const content = chapter.content.toLowerCase()

        // Look for potential violations (e.g., "impossible", "cannot" followed by rule keywords)
        for (const keyword of ruleKeywords) {
          const violationPattern = new RegExp(`(?:impossible|cannot|can't|never)\\s+\\w*\\s*${keyword}`, "i")
          if (violationPattern.test(content)) {
            if (!ruleViolations.has(rule)) {
              ruleViolations.set(rule, [])
            }
            ruleViolations.get(rule)!.push(chapters.indexOf(chapter))
          }
        }
      }
    }

    for (const [rule, chapterIndices] of ruleViolations.entries()) {
      for (const idx of chapterIndices) {
        issues.push({
          type: "rule_violation",
          chapterID: chapters[idx].id,
          description: `Potential violation of world rule: "${rule}"`,
          severity: "medium",
        })
      }
    }

    // Check for inconsistent technology/magic references
    if (worldFramework.magicSystem) {
      const magicKeywords = extractKeywords(worldFramework.magicSystem)
      for (const chapter of chapters) {
        const content = chapter.content.toLowerCase()

        // Check for magic in non-magic context or vice versa
        const hasMagic = magicKeywords.some((kw) => content.includes(kw.toLowerCase()))
        const hasModernTech = /\b(?:computer|internet|phone|electricity|car|airplane)\b/i.test(chapter.content)

        if (hasMagic && hasModernTech) {
          issues.push({
            type: "tone_inconsistency",
            chapterID: chapter.id,
            description: "Chapter contains both magic and modern technology",
            severity: "low",
          })
        }
      }
    }

    return {
      isConsistent: issues.filter((i) => i.severity === "high").length === 0,
      issues,
    }
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  /**
   * Analyze a chapter to determine likely story arc type.
   */
  function analyzeChapterForArcType(chapter: {
    title: string
    summary?: string
    content: string
  }): KnowledgeSchema.StoryArcType[] {
    const text = (chapter.summary + " " + chapter.content).toLowerCase()
    const scores: KnowledgeSchema.StoryArcType[] = []

    // Setup indicators
    const setupKeywords = [
      "introduction", "beginning", "once upon", "first", "meet", "arrive",
      "establish", "setting", "background", "started", "began"
    ]
    const setupScore = setupKeywords.filter((kw) => text.includes(kw)).length
    if (setupScore >= 2) scores.push("setup")

    // Rising action indicators
    const risingKeywords = [
      "conflict", "problem", "challenge", "journey", "pursue", "develop",
      "build", "tension", "rising", "encounter", "obstacle"
    ]
    const risingScore = risingKeywords.filter((kw) => text.includes(kw)).length
    if (risingScore >= 2) scores.push("rising")

    // Climax indicators
    const climaxKeywords = [
      "peak", "highest", "final", "showdown", "confront", "battle",
      "face", "ultimate", "crisis", "turning point", "climax"
    ]
    const climaxScore = climaxKeywords.filter((kw) => text.includes(kw)).length
    if (climaxScore >= 1) scores.push("climax")

    // Falling action indicators
    const fallingKeywords = [
      "aftermath", "retreat", "escape", "consequence", "result",
      "falling", "following", "after the", "having"
    ]
    const fallingScore = fallingKeywords.filter((kw) => text.includes(kw)).length
    if (fallingScore >= 2) scores.push("falling")

    // Resolution indicators
    const resolutionKeywords = [
      "resolve", "conclusion", "ending", "peace", "understand",
      "accept", "resolution", "finally", "complete", "end"
    ]
    const resolutionScore = resolutionKeywords.filter((kw) => text.includes(kw)).length
    if (resolutionScore >= 2) scores.push("resolution")

    return scores.length > 0 ? scores : ["rising"]
  }

  /**
   * Get the dominant arc type from a list.
   */
  function getDominantArcType(types: KnowledgeSchema.StoryArcType[]): KnowledgeSchema.StoryArcType {
    const counts = new Map<KnowledgeSchema.StoryArcType, number>()
    for (const type of types) {
      counts.set(type, (counts.get(type) || 0) + 1)
    }

    let maxCount = 0
    let dominant: KnowledgeSchema.StoryArcType = "rising"
    for (const [type, count] of counts.entries()) {
      if (count > maxCount) {
        maxCount = count
        dominant = type
      }
    }

    return dominant
  }

  /**
   * Calculate confidence score for an arc detection.
   */
  function calculateArcConfidence(
    scores: KnowledgeSchema.StoryArcType[][],
    start: number,
    end: number,
    arcType: KnowledgeSchema.StoryArcType,
  ): number {
    let matches = 0
    for (let i = start; i <= end; i++) {
      if (scores[i]?.includes(arcType)) matches++
    }
    return matches / (end - start + 1)
  }

  /**
   * Extract keywords from a text.
   */
  function extractKeywords(text: string): string[] {
    return text
      .toLowerCase()
      .split(/\W+/)
      .filter((w) => w.length > 3)
      .filter((w, i, arr) => arr.indexOf(w) === i) // unique
  }

  /**
   * Import Storage dynamically to avoid circular dependencies.
   */
  async function getStorage(): Promise<
    typeof import("../../storage/storage").Storage
  > {
    return (await import("../../storage/storage")).Storage
  }

  // Local storage interface to avoid circular dependency
  const storage = {
    write: async (key: string[], content: unknown) =>
      (await getStorage()).write(key, content),
    read: async <T>(key: string[]) =>
      (await getStorage()).read<T>(key),
    list: async (prefix: string[]) =>
      (await getStorage()).list(prefix),
    remove: async (key: string[]) =>
      (await getStorage()).remove(key),
  }
}
