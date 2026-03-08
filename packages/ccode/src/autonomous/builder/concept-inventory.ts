/**
 * Concept Inventory
 *
 * Unified discovery and search for all concept types in the system.
 * Integrates Agent.list(), Skill.all(), DynamicToolRegistry, and
 * scans for HAND, MEMORY, and WORKFLOW definitions.
 *
 * @package autonomous/builder
 */

import { Agent } from "@/agent/agent"
import { Skill } from "@/skill/skill"
import { DynamicToolRegistry, type ToolTypes } from "@/memory/tools"
import { Instance } from "@/project/instance"
import { Global } from "@/util/global"
import { Filesystem } from "@/util/filesystem"
import { ConfigMarkdown } from "@/config/markdown"
import { Log } from "@/util/log"
import path from "path"
import z from "zod"

import type { ConceptType } from "./types"

const log = Log.create({ service: "autonomous.builder.inventory" })

// ============================================================================
// Types
// ============================================================================

/**
 * Unified concept entry
 */
export const ConceptEntrySchema = z.object({
  /** Concept type */
  type: z.enum(["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"]),
  /** Unique identifier */
  identifier: z.string(),
  /** Human-readable name */
  displayName: z.string(),
  /** Description */
  description: z.string().optional(),
  /** File location */
  location: z.string().optional(),
  /** Whether this is a builtin/native concept */
  native: z.boolean(),
  /** Tags for filtering */
  tags: z.array(z.string()).optional(),
  /** Last updated timestamp */
  updatedAt: z.number().optional(),
})
export type ConceptEntry = z.infer<typeof ConceptEntrySchema>

/**
 * Search options
 */
export interface SearchOptions {
  /** Filter by concept type */
  types?: ConceptType[]
  /** Filter by tags */
  tags?: string[]
  /** Include only native/builtin concepts */
  nativeOnly?: boolean
  /** Maximum results */
  limit?: number
  /** Minimum similarity score (0-1) for semantic search */
  minScore?: number
}

/**
 * Search result with relevance score
 */
export interface SearchResult {
  concept: ConceptEntry
  score: number
  matchType: "exact" | "fuzzy" | "semantic"
}

// ============================================================================
// Glob Patterns
// ============================================================================

const HAND_GLOB = new Bun.Glob("{hands,hand}/**/HAND.md")
const MEMORY_GLOB = new Bun.Glob("{memory,memories}/**/*.schema.json")
const WORKFLOW_GLOB = new Bun.Glob("{workflows,workflow}/**/WORKFLOW.md")
const PROMPT_GLOB = new Bun.Glob("{prompts,prompt}/**/*.{txt,md}")

// ============================================================================
// Concept Inventory
// ============================================================================

export class ConceptInventory {
  private cachedConcepts: ConceptEntry[] | null = null
  private cacheTimestamp: number = 0
  private readonly cacheTTL = 60_000 // 1 minute cache

  /**
   * Get all concepts in the system
   */
  async all(): Promise<ConceptEntry[]> {
    // Check cache
    if (this.cachedConcepts && Date.now() - this.cacheTimestamp < this.cacheTTL) {
      return this.cachedConcepts
    }

    const concepts: ConceptEntry[] = []

    // Collect concepts in parallel
    const [agents, skills, tools, hands, memories, workflows, prompts] = await Promise.all([
      this.collectAgents(),
      this.collectSkills(),
      this.collectTools(),
      this.collectHands(),
      this.collectMemories(),
      this.collectWorkflows(),
      this.collectPrompts(),
    ])

    concepts.push(...agents, ...skills, ...tools, ...hands, ...memories, ...workflows, ...prompts)

    // Update cache
    this.cachedConcepts = concepts
    this.cacheTimestamp = Date.now()

    log.info("Concept inventory refreshed", {
      agents: agents.length,
      skills: skills.length,
      tools: tools.length,
      hands: hands.length,
      memories: memories.length,
      workflows: workflows.length,
      prompts: prompts.length,
      total: concepts.length,
    })

    return concepts
  }

  /**
   * Search for concepts by query
   */
  async search(query: string, options: SearchOptions = {}): Promise<SearchResult[]> {
    const allConcepts = await this.all()
    const results: SearchResult[] = []

    const queryLower = query.toLowerCase()
    const queryTerms = queryLower.split(/\s+/)

    for (const concept of allConcepts) {
      // Apply type filter
      if (options.types && !options.types.includes(concept.type)) {
        continue
      }

      // Apply native filter
      if (options.nativeOnly && !concept.native) {
        continue
      }

      // Apply tag filter
      if (options.tags && options.tags.length > 0) {
        const conceptTags = concept.tags ?? []
        const hasMatchingTag = options.tags.some((t) => conceptTags.includes(t))
        if (!hasMatchingTag) continue
      }

      // Calculate relevance score
      const score = this.calculateScore(concept, queryLower, queryTerms)

      // Apply minimum score filter
      if (options.minScore && score < options.minScore) {
        continue
      }

      if (score > 0) {
        results.push({
          concept,
          score,
          matchType: score === 1 ? "exact" : score > 0.5 ? "fuzzy" : "semantic",
        })
      }
    }

    // Sort by score descending
    results.sort((a, b) => b.score - a.score)

    // Apply limit
    if (options.limit) {
      return results.slice(0, options.limit)
    }

    return results
  }

  /**
   * Get a specific concept by identifier
   */
  async get(identifier: string, type?: ConceptType): Promise<ConceptEntry | null> {
    const allConcepts = await this.all()

    return (
      allConcepts.find((c) => c.identifier === identifier && (!type || c.type === type)) ?? null
    )
  }

  /**
   * Check if a concept identifier already exists
   */
  async exists(identifier: string, type?: ConceptType): Promise<boolean> {
    const concept = await this.get(identifier, type)
    return concept !== null
  }

  /**
   * Get concepts by type
   */
  async byType(type: ConceptType): Promise<ConceptEntry[]> {
    const allConcepts = await this.all()
    return allConcepts.filter((c) => c.type === type)
  }

  /**
   * Invalidate the cache
   */
  invalidateCache(): void {
    this.cachedConcepts = null
    this.cacheTimestamp = 0
  }

  // ============================================================================
  // Private Collectors
  // ============================================================================

  private async collectAgents(): Promise<ConceptEntry[]> {
    try {
      const agents = await Agent.list()
      return agents.map((agent) => ({
        type: "AGENT" as const,
        identifier: agent.name,
        displayName: agent.name,
        description: agent.description,
        native: agent.native ?? false,
        tags: this.extractAgentTags(agent),
      }))
    } catch (error) {
      log.error("Failed to collect agents", { error })
      return []
    }
  }

  private async collectSkills(): Promise<ConceptEntry[]> {
    try {
      const skills = await Skill.all()
      return skills.map((skill) => ({
        type: "SKILL" as const,
        identifier: skill.name,
        displayName: skill.name,
        description: skill.description,
        location: skill.location,
        native: skill.location.includes("builtin"),
      }))
    } catch (error) {
      log.error("Failed to collect skills", { error })
      return []
    }
  }

  private async collectTools(): Promise<ConceptEntry[]> {
    try {
      const tools = await DynamicToolRegistry.list()
      return tools.map((tool: ToolTypes.DynamicTool) => ({
        type: "TOOL" as const,
        identifier: tool.id,
        displayName: tool.name,
        description: tool.description,
        tags: tool.tags,
        native: false,
        updatedAt: tool.metadata.updatedAt,
      }))
    } catch (error) {
      log.error("Failed to collect tools", { error })
      return []
    }
  }

  private async collectHands(): Promise<ConceptEntry[]> {
    const hands: ConceptEntry[] = []

    for (const dir of await this.getConfigDirectories()) {
      try {
        for await (const match of HAND_GLOB.scan({
          cwd: dir,
          absolute: true,
          onlyFiles: true,
        })) {
          const hand = await this.parseMarkdownConcept(match, "HAND")
          if (hand) hands.push(hand)
        }
      } catch (error) {
        log.debug("Failed to scan for hands in directory", { dir, error })
      }
    }

    return hands
  }

  private async collectMemories(): Promise<ConceptEntry[]> {
    const memories: ConceptEntry[] = []

    for (const dir of await this.getConfigDirectories()) {
      try {
        for await (const match of MEMORY_GLOB.scan({
          cwd: dir,
          absolute: true,
          onlyFiles: true,
        })) {
          const memory = await this.parseJsonConcept(match, "MEMORY")
          if (memory) memories.push(memory)
        }
      } catch (error) {
        log.debug("Failed to scan for memories in directory", { dir, error })
      }
    }

    return memories
  }

  private async collectWorkflows(): Promise<ConceptEntry[]> {
    const workflows: ConceptEntry[] = []

    for (const dir of await this.getConfigDirectories()) {
      try {
        for await (const match of WORKFLOW_GLOB.scan({
          cwd: dir,
          absolute: true,
          onlyFiles: true,
        })) {
          const workflow = await this.parseMarkdownConcept(match, "WORKFLOW")
          if (workflow) workflows.push(workflow)
        }
      } catch (error) {
        log.debug("Failed to scan for workflows in directory", { dir, error })
      }
    }

    return workflows
  }

  private async collectPrompts(): Promise<ConceptEntry[]> {
    const prompts: ConceptEntry[] = []

    // Scan builtin prompts
    const builtinDir = path.join(import.meta.dirname, "../../agent/prompt")
    if (await Filesystem.isDir(builtinDir)) {
      try {
        for await (const match of PROMPT_GLOB.scan({
          cwd: builtinDir,
          absolute: true,
          onlyFiles: true,
        })) {
          prompts.push({
            type: "PROMPT",
            identifier: path.basename(match, path.extname(match)),
            displayName: path.basename(match, path.extname(match)),
            location: match,
            native: true,
          })
        }
      } catch (error) {
        log.debug("Failed to scan builtin prompts", { error })
      }
    }

    // Scan user prompts
    for (const dir of await this.getConfigDirectories()) {
      try {
        for await (const match of PROMPT_GLOB.scan({
          cwd: dir,
          absolute: true,
          onlyFiles: true,
        })) {
          prompts.push({
            type: "PROMPT",
            identifier: path.basename(match, path.extname(match)),
            displayName: path.basename(match, path.extname(match)),
            location: match,
            native: false,
          })
        }
      } catch (error) {
        log.debug("Failed to scan prompts in directory", { dir, error })
      }
    }

    return prompts
  }

  // ============================================================================
  // Helpers
  // ============================================================================

  private async getConfigDirectories(): Promise<string[]> {
    const dirs: string[] = []

    // Project-level directories
    const projectDirs = await Array.fromAsync(
      Filesystem.up({
        targets: [".claude", ".codecoder"],
        start: Instance.directory,
        stop: Instance.worktree,
      }),
    )
    dirs.push(...projectDirs)

    // Global directories
    const globalClaude = `${Global.Path.home}/.claude`
    const globalCodeCoder = `${Global.Path.home}/.codecoder`

    if (await Filesystem.isDir(globalClaude)) {
      dirs.push(globalClaude)
    }
    if (await Filesystem.isDir(globalCodeCoder)) {
      dirs.push(globalCodeCoder)
    }

    return dirs
  }

  private async parseMarkdownConcept(
    filePath: string,
    type: ConceptType,
  ): Promise<ConceptEntry | null> {
    try {
      const md = await ConfigMarkdown.parse(filePath)
      const data = md.data as Record<string, unknown>

      return {
        type,
        identifier: (data.id as string) ?? (data.name as string) ?? path.basename(path.dirname(filePath)),
        displayName: (data.name as string) ?? path.basename(path.dirname(filePath)),
        description: data.description as string | undefined,
        location: filePath,
        native: filePath.includes("builtin"),
        tags: Array.isArray(data.tags) ? data.tags : undefined,
      }
    } catch (error) {
      log.debug("Failed to parse markdown concept", { filePath, error })
      return null
    }
  }

  private async parseJsonConcept(
    filePath: string,
    type: ConceptType,
  ): Promise<ConceptEntry | null> {
    try {
      const content = await Bun.file(filePath).text()
      const data = JSON.parse(content) as Record<string, unknown>

      return {
        type,
        identifier: (data.$id as string) ?? (data.name as string) ?? path.basename(filePath, ".schema.json"),
        displayName: (data.title as string) ?? path.basename(filePath, ".schema.json"),
        description: data.description as string | undefined,
        location: filePath,
        native: false,
      }
    } catch (error) {
      log.debug("Failed to parse JSON concept", { filePath, error })
      return null
    }
  }

  private extractAgentTags(agent: Agent.Info): string[] {
    const tags: string[] = []

    if (agent.mode) tags.push(agent.mode)
    if (agent.native) tags.push("native")
    if (agent.hidden) tags.push("hidden")

    return tags
  }

  private calculateScore(concept: ConceptEntry, queryLower: string, queryTerms: string[]): number {
    const nameLower = concept.identifier.toLowerCase()
    const displayLower = concept.displayName.toLowerCase()
    const descLower = (concept.description ?? "").toLowerCase()

    // Exact match on identifier
    if (nameLower === queryLower) return 1.0

    // Exact match on display name
    if (displayLower === queryLower) return 0.95

    // Starts with query
    if (nameLower.startsWith(queryLower) || displayLower.startsWith(queryLower)) return 0.9

    // Contains query as substring
    if (nameLower.includes(queryLower) || displayLower.includes(queryLower)) return 0.8

    // All query terms found in description
    const descTermMatches = queryTerms.filter((term) => descLower.includes(term)).length
    if (descTermMatches === queryTerms.length && queryTerms.length > 0) return 0.7

    // Partial term matches
    const allText = `${nameLower} ${displayLower} ${descLower}`
    const partialMatches = queryTerms.filter((term) => allText.includes(term)).length
    if (partialMatches > 0) {
      return 0.3 + (partialMatches / queryTerms.length) * 0.3
    }

    // Check tags
    const tagText = (concept.tags ?? []).join(" ").toLowerCase()
    if (tagText.includes(queryLower)) return 0.5

    return 0
  }
}

// ============================================================================
// Singleton Instance
// ============================================================================

let inventoryInstance: ConceptInventory | null = null

/**
 * Get the global concept inventory instance
 */
export function getConceptInventory(): ConceptInventory {
  if (!inventoryInstance) {
    inventoryInstance = new ConceptInventory()
  }
  return inventoryInstance
}

/**
 * Create a new concept inventory instance (for testing)
 */
export function createConceptInventory(): ConceptInventory {
  return new ConceptInventory()
}
