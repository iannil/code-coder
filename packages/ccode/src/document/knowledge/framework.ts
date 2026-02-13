import { Identifier } from "../../id/id"
import { KnowledgeSchema } from "./schema"
import { KnowledgeNode } from "./node"

/**
 * Dynamically import Storage to avoid circular dependencies.
 */
async function getStorage() {
  return (await import("../../storage/storage")).Storage
}

// Local storage interface to avoid circular dependency
const storage = {
  write: async (key: string[], content: unknown) =>
    (await getStorage()).write(key, content),
  read: async <T>(key: string[]) =>
    (await getStorage()).read<T>(key),
}

export namespace Framework {
  const STORAGE_PREFIX = ["document_framework"]

  /**
   * Generate a unique framework ID.
   */
  export function createID(): string {
    return Identifier.create("theme", false)
  }

  /**
   * Analyze a core idea and generate a framework for expansion.
   */
  export async function analyzeCoreIdea(input: {
    idea: string
    targetWords?: number
    contentType?: "fiction" | "nonfiction" | "auto"
  }): Promise<KnowledgeSchema.CoreIdeaAnalysis> {
    const { idea, targetWords = 50000, contentType = "auto" } = input

    // Auto-detect content type if not specified
    let detectedType: "fiction" | "nonfiction" = "nonfiction"
    if (contentType === "auto") {
      detectedType = detectContentType(idea)
    } else {
      detectedType = contentType
    }

    // Extract key concepts using simple NLP heuristics
    const keyConcepts = extractKeyConcepts(idea)

    // Generate suggested structure
    const wordsPerChapter = 3000 // Average chapter length
    const suggestedChapterCount = Math.max(5, Math.ceil(targetWords / wordsPerChapter))

    const analysis: KnowledgeSchema.CoreIdeaAnalysis = {
      contentType: detectedType,
      coreThesis: extractThesis(idea),
      mainThemes: extractThemes(idea),
      suggestedWordCount: targetWords,
      suggestedChapterCount,
      keyConcepts,
      potentialConflicts: detectedType === "fiction" ? extractConflicts(idea) : [],
      potentialArguments: detectedType === "nonfiction" ? extractArguments(idea) : [],
    }

    return analysis
  }

  /**
   * Create a thematic framework from core idea analysis.
   */
  export async function createThematicFramework(input: {
    documentID: string
    analysis: KnowledgeSchema.CoreIdeaAnalysis
  }): Promise<KnowledgeSchema.ThematicFramework> {
    const { documentID, analysis } = input

    // Create knowledge nodes from key concepts
    const conceptNodes: KnowledgeSchema.KnowledgeNode[] = []
    for (const concept of analysis.keyConcepts) {
      const node = await KnowledgeNode.create(documentID, {
        type: "concept",
        content: concept,
        derivedFrom: [],
        confidence: 1,
        attributes: { source: "core_idea" },
      })
      conceptNodes.push(node)
    }

    // Build knowledge graph from concepts
    const knowledgeGraph: Record<string, string[]> = {}
    for (let i = 0; i < conceptNodes.length; i++) {
      knowledgeGraph[conceptNodes[i].id] = []
      // Connect related concepts (adjacent in list)
      if (i > 0) {
        knowledgeGraph[conceptNodes[i].id].push(conceptNodes[i - 1].id)
      }
      if (i < conceptNodes.length - 1) {
        knowledgeGraph[conceptNodes[i].id].push(conceptNodes[i + 1].id)
      }
    }

    const framework: KnowledgeSchema.ThematicFramework = {
      id: createID(),
      thesis: analysis.coreThesis,
      corePrinciples: analysis.mainThemes,
      mainThemes: analysis.mainThemes,
      knowledgeGraph,
      argumentChainIDs: [],
      storyArcIDs: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }

    return framework
  }

  /**
   * Create a world framework for fiction content.
   */
  export async function createWorldFramework(input: {
    documentID: string
    analysis: KnowledgeSchema.CoreIdeaAnalysis
    worldName?: string
  }): Promise<KnowledgeSchema.WorldFramework> {
    const { documentID, analysis, worldName = "Untitled World" } = input

    // Extract world-building elements from the idea
    const rules = extractWorldRules(analysis.keyConcepts)

    // Create location nodes for key settings
    const locations = await extractLocations(documentID, analysis.keyConcepts)

    const framework: KnowledgeSchema.WorldFramework = {
      id: Identifier.create("world", false),
      name: worldName,
      description: `World framework for: ${analysis.coreThesis}`,
      rules,
      magicSystem: analysis.contentType === "fiction" ? extractMagicSystem(analysis.keyConcepts) : undefined,
      technology: extractTechLevel(analysis.keyConcepts),
      timeline: [],
      geography: locations,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }

    return framework
  }

  /**
   * Get a framework by ID.
   */
  export async function get(
    documentID: string,
    frameworkID: string,
  ): Promise<KnowledgeSchema.ThematicFramework | null> {
    try {
      return await storage.read<KnowledgeSchema.ThematicFramework>([...STORAGE_PREFIX, documentID, frameworkID])
    } catch {
      return null
    }
  }

  /**
   * Save a framework.
   */
  export async function save(
    documentID: string,
    framework: KnowledgeSchema.ThematicFramework,
  ): Promise<void> {
    await storage.write([...STORAGE_PREFIX, documentID, framework.id], framework)
  }

  /**
   * Expand framework with new knowledge.
   */
  export async function expandFramework(input: {
    documentID: string
    framework: KnowledgeSchema.ThematicFramework
    newContent: string
    chapterID: string
  }): Promise<KnowledgeSchema.ThematicFramework> {
    const { documentID, framework, newContent, chapterID } = input

    // Extract new concepts from content
    const newConcepts = extractKeyConcepts(newContent, 10)

    // Create knowledge nodes for new concepts
    const newNodeIDs: string[] = []
    for (const concept of newConcepts) {
      // Check if similar concept exists
      const existing = await KnowledgeNode.search(documentID, concept)
      const isDuplicate = existing.some((n) => n.content.toLowerCase() === concept.toLowerCase())

      if (!isDuplicate) {
        const node = await KnowledgeNode.create(documentID, {
          type: "concept",
          content: concept,
          derivedFrom: [],
          confidence: 0.8,
          chapterID,
          attributes: { source: "writing", addedAt: Date.now().toString() },
        })
        newNodeIDs.push(node.id)
      }
    }

    // Update knowledge graph
    const updatedGraph = { ...framework.knowledgeGraph }
    for (const nodeID of newNodeIDs) {
      updatedGraph[nodeID] = []
      // Connect to existing related concepts
      for (const [existingID, related] of Object.entries(framework.knowledgeGraph)) {
        if (related.length < 5) { // Limit connections
          updatedGraph[existingID] = [...related, nodeID]
          updatedGraph[nodeID] = [...(updatedGraph[nodeID] || []), existingID]
        }
      }
    }

    const updated: KnowledgeSchema.ThematicFramework = {
      ...framework,
      knowledgeGraph: updatedGraph,
      updatedAt: Date.now(),
    }

    await save(documentID, updated)
    return updated
  }

  /**
   * Validate framework consistency.
   */
  export async function validateFramework(input: {
    documentID: string
    framework: KnowledgeSchema.ThematicFramework
  }): Promise<{
    isValid: boolean
    issues: Array<{ type: string; description: string; severity: "low" | "medium" | "high" }>
  }> {
    const { documentID, framework } = input
    const issues: Array<{ type: string; description: string; severity: "low" | "medium" | "high" }> = []

    // Check for orphaned nodes in knowledge graph
    const allNodeIDs = new Set(Object.keys(framework.knowledgeGraph))
    const connectedNodes = new Set<string>()
    for (const [id, related] of Object.entries(framework.knowledgeGraph)) {
      for (const relatedID of related) {
        connectedNodes.add(id)
        connectedNodes.add(relatedID)
      }
    }

    const orphaned = [...allNodeIDs].filter((id) => !connectedNodes.has(id))
    if (orphaned.length > 0) {
      issues.push({
        type: "orphaned_concepts",
        description: `${orphaned.length} knowledge node(s) without connections`,
        severity: "low",
      })
    }

    // Check for circular references
    const cycles = await KnowledgeNode.detectCycles(documentID)
    if (cycles.length > 0) {
      issues.push({
        type: "circular_references",
        description: `${cycles.length} circular reference(s) detected in knowledge graph`,
        severity: "medium",
      })
    }

    // Check if thesis is supported by principles
    if (framework.corePrinciples.length === 0) {
      issues.push({
        type: "missing_principles",
        description: "Thematic framework lacks core principles",
        severity: "high",
      })
    }

    // Check if main themes are defined
    if (framework.mainThemes.length === 0) {
      issues.push({
        type: "missing_themes",
        description: "Thematic framework lacks defined themes",
        severity: "high",
      })
    }

    return {
      isValid: issues.filter((i) => i.severity === "high").length === 0,
      issues,
    }
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  function detectContentType(text: string): "fiction" | "nonfiction" {
    const fictionIndicators = [
      /\b(character|protagonist|antagonist|plot|story|narrative|tale|fantasy|sci-fi|magic|dragon)\b/i,
      /\b(once upon a time|in a galaxy far away|long ago|imaginary)\b/i,
    ]

    const nonfictionIndicators = [
      /\b(theorem|proof|argument|evidence|research|study|analysis|thesis|conclusion)\b/i,
      /\b(however|therefore|furthermore|moreover|consequently)\b/i,
    ]

    const fictionScore = fictionIndicators.reduce((sum, pattern) => sum + (pattern.test(text) ? 1 : 0), 0)
    const nonfictionScore = nonfictionIndicators.reduce((sum, pattern) => sum + (pattern.test(text) ? 1 : 0), 0)

    return fictionScore > nonfictionScore ? "fiction" : "nonfiction"
  }

  function extractThesis(text: string): string {
    // Try to find the main claim or premise
    const sentences = text.split(/[.!?]+/).filter((s) => s.trim().length > 10)

    // Look for thesis indicators
    const thesisPatterns = [
      /^(this paper|this book|this story|the thesis|the argument|the central claim)/i,
      /^(i argue|i believe|i will show|i propose)/i,
    ]

    for (const sentence of sentences) {
      for (const pattern of thesisPatterns) {
        if (pattern.test(sentence.trim())) {
          return sentence.trim()
        }
      }
    }

    // Default to first substantial sentence
    return sentences[0]?.trim() || text.slice(0, 200)
  }

  function extractThemes(text: string): string[] {
    const themes: string[] = []

    // Common theme keywords
    const themeKeywords: Record<string, string[]> = {
      "Love and Relationships": ["love", "relationship", "romance", "friendship", "family"],
      "Good vs Evil": ["good", "evil", "moral", "ethical", "right", "wrong"],
      "Coming of Age": ["growth", "maturity", "coming of age", "adolescence", "journey"],
      "Identity": ["identity", "self", "who am i", "belonging", "purpose"],
      "Power and Corruption": ["power", "corruption", "control", "authority", "freedom"],
      "Survival": ["survival", "survive", "struggle", "endurance", "overcoming"],
      "Knowledge and Truth": ["truth", "knowledge", "wisdom", "discovery", "understanding"],
      "Justice": ["justice", "fairness", "law", "equality", "rights"],
    }

    const lowerText = text.toLowerCase()

    for (const [theme, keywords] of Object.entries(themeKeywords)) {
      const matchCount = keywords.filter((kw) => lowerText.includes(kw)).length
      if (matchCount >= 2) {
        themes.push(theme)
      }
    }

    return themes.length > 0 ? themes : ["General Theme"]
  }

  function extractKeyConcepts(text: string, maxCount: number = 20): string[] {
    // Simple extraction based on capitalized phrases and key terms
    const concepts: string[] = []

    // Extract quoted terms
    const quotedMatches = text.matchAll(/"([^"]{2,30})"/g)
    for (const match of quotedMatches) {
      concepts.push(match[1])
    }

    // Extract capitalized phrases (likely proper nouns/concepts)
    const capitalizedMatches = text.matchAll(/\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,2})\b/g)
    for (const match of capitalizedMatches) {
      const phrase = match[1]
      if (phrase.length > 2 && !concepts.includes(phrase)) {
        concepts.push(phrase)
      }
    }

    // Extract domain-specific terms
    const domainPatterns = [
      /(?:the concept of|the idea of|the principle of) ([a-z]{3,30})/gi,
      /(?:define|refers to|means) ([a-z]{3,30})/gi,
    ]

    for (const pattern of domainPatterns) {
      const matches = text.matchAll(pattern)
      for (const match of matches) {
        if (!concepts.includes(match[1])) {
          concepts.push(match[1])
        }
      }
    }

    return concepts.slice(0, maxCount)
  }

  function extractConflicts(text: string): string[] {
    const conflicts: string[] = []

    // Conflict indicators
    const conflictPatterns = [
      /(?:conflict|struggle|tension) (?:between|with|against) ([^.!?]+)/gi,
      /(?:must|should) (?:choose|decide) (?:between|among) ([^.!?]+)/gi,
    ]

    for (const pattern of conflictPatterns) {
      const matches = text.matchAll(pattern)
      for (const match of matches) {
        conflicts.push(match[1].trim())
      }
    }

    return conflicts
  }

  function extractArguments(text: string): string[] {
    const extractedArgs: string[] = []

    // Argument indicators
    const argPatterns = [
      /(?:the argument|the claim|the thesis) (?:is|that) ([^.!?]+)/gi,
      /(?:because|therefore|thus|consequently) ([^.!?]+)/gi,
    ]

    for (const pattern of argPatterns) {
      const matches = text.matchAll(pattern)
      for (const match of matches) {
        extractedArgs.push(match[1].trim())
      }
    }

    return extractedArgs
  }

  function extractWorldRules(concepts: string[]): string[] {
    // Look for rule-like concepts
    const rules: string[] = []

    for (const concept of concepts) {
      const lower = concept.toLowerCase()
      if (/^(?:rule|law|principle|can't|cannot|must|should|magic)/.test(lower)) {
        rules.push(concept)
      }
    }

    return rules
  }

  function extractMagicSystem(concepts: string[]): string | undefined {
    const magicConcepts = concepts.filter((c) =>
      /(?:magic|spell|power|ability|curse|enchantment)/i.test(c),
    )

    if (magicConcepts.length > 0) {
      return `Magic system based on: ${magicConcepts.join(", ")}`
    }

    return undefined
  }

  function extractTechLevel(concepts: string[]): string {
    const techKeywords = {
      primitive: ["stone age", "medieval", "sword", "horse", "castle"],
      modern: ["computer", "internet", "phone", "car", "electricity"],
      futuristic: ["spaceship", "laser", "robot", "ai", "cybernetics", "warp"],
    }

    for (const [level, keywords] of Object.entries(techKeywords)) {
      for (const concept of concepts) {
        if (keywords.some((kw) => concept.toLowerCase().includes(kw))) {
          return level
        }
      }
    }

    return "modern"
  }

  async function extractLocations(documentID: string, concepts: string[]): Promise<string[]> {
    const locations: string[] = []

    for (const concept of concepts) {
      // Look for location-like phrases
      if (/^(?:kingdom|city|castle|forest|mountain|river|world|realm|kingdom)/i.test(concept)) {
        locations.push(concept)

        // Create location knowledge nodes
        await KnowledgeNode.create(documentID, {
          type: "location",
          content: concept,
          derivedFrom: [],
          confidence: 0.9,
          attributes: { source: "core_idea" },
        })
      }
    }

    return locations
  }
}
