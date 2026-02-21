/**
 * Agent Registry - Dynamic agent registration and discovery.
 *
 * Extends the existing Agent system with:
 * - Rich metadata (capabilities, triggers, examples)
 * - Runtime registration API
 * - Agent search and recommendations
 * - Category-based organization
 */

import z from "zod"
import { Agent } from "./agent"
import Fuse from "fuse.js"

// ─────────────────────────────────────────────────────────────────────────────
// Schema Definitions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Agent capability declaration.
 */
export const AgentCapability = z.object({
  /** Unique capability identifier */
  id: z.string(),
  /** Human-readable name */
  name: z.string(),
  /** Description of what this capability does */
  description: z.string(),
  /** Whether this capability is primary or secondary */
  primary: z.boolean().default(false),
})
export type AgentCapability = z.infer<typeof AgentCapability>

/**
 * Trigger condition for automatic agent invocation.
 */
export const AgentTrigger = z.object({
  /** Trigger type */
  type: z.enum(["keyword", "pattern", "event", "context"]),
  /** Trigger value (keyword, regex pattern, event name, or context condition) */
  value: z.string(),
  /** Trigger priority (higher = more important) */
  priority: z.number().default(0),
  /** Optional description */
  description: z.string().optional(),
})
export type AgentTrigger = z.infer<typeof AgentTrigger>

/**
 * Usage example for an agent.
 */
export const AgentExample = z.object({
  /** Example title */
  title: z.string(),
  /** User input */
  input: z.string(),
  /** Expected agent behavior or output summary */
  output: z.string(),
  /** Tags for categorization */
  tags: z.array(z.string()).default([]),
})
export type AgentExample = z.infer<typeof AgentExample>

/**
 * Agent category for organization.
 */
export const AgentCategory = z.enum([
  "engineering",
  "content",
  "analysis",
  "philosophy",
  "system",
  "custom",
])
export type AgentCategory = z.infer<typeof AgentCategory>

/**
 * Extended agent metadata for registry.
 */
export const AgentMetadata = z.object({
  /** Agent name (must match Agent.Info.name) */
  name: z.string(),
  /** Display name for UI */
  displayName: z.string().optional(),
  /** Short description (one line) */
  shortDescription: z.string().optional(),
  /** Long description (markdown supported) */
  longDescription: z.string().optional(),
  /** Agent category */
  category: AgentCategory.default("custom"),
  /** Agent capabilities */
  capabilities: z.array(AgentCapability).default([]),
  /** Auto-invocation triggers */
  triggers: z.array(AgentTrigger).default([]),
  /** Usage examples */
  examples: z.array(AgentExample).default([]),
  /** Tags for search */
  tags: z.array(z.string()).default([]),
  /** Author information */
  author: z.string().optional(),
  /** Version string */
  version: z.string().default("1.0.0"),
  /** Whether this is a built-in agent */
  builtin: z.boolean().default(false),
  /** Icon name or emoji */
  icon: z.string().optional(),
  /** Recommended for first-time users */
  recommended: z.boolean().default(false),
})
export type AgentMetadata = z.infer<typeof AgentMetadata>

/**
 * Search result with relevance score.
 */
export interface SearchResult {
  agent: AgentMetadata
  score: number
  matches: Array<{
    key: string
    value: string
    indices: ReadonlyArray<[number, number]>
  }>
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Metadata for Built-in Agents
// ─────────────────────────────────────────────────────────────────────────────

const BUILTIN_METADATA: Record<string, Partial<AgentMetadata>> = {
  build: {
    displayName: "Build",
    shortDescription: "Primary development agent for building features and fixing bugs",
    category: "engineering",
    capabilities: [
      { id: "code-write", name: "Code Writing", description: "Write and modify code", primary: true },
      { id: "file-edit", name: "File Editing", description: "Edit files in the codebase", primary: true },
      { id: "planning", name: "Planning", description: "Create implementation plans", primary: false },
    ],
    triggers: [
      { type: "keyword", value: "build", priority: 10 },
      { type: "keyword", value: "implement", priority: 8 },
      { type: "keyword", value: "create", priority: 7 },
      { type: "keyword", value: "fix", priority: 6 },
    ],
    examples: [
      {
        title: "Implement a feature",
        input: "Add a dark mode toggle to the settings page",
        output: "Creates the toggle component, updates settings state, and adds theme switching logic",
        tags: ["feature", "ui"],
      },
    ],
    tags: ["development", "coding", "primary"],
    builtin: true,
    icon: "🔨",
    recommended: true,
  },
  plan: {
    displayName: "Plan",
    shortDescription: "Creates detailed implementation plans before coding",
    category: "engineering",
    capabilities: [
      { id: "planning", name: "Planning", description: "Create step-by-step plans", primary: true },
      { id: "analysis", name: "Analysis", description: "Analyze requirements and codebase", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "plan", priority: 10 },
      { type: "keyword", value: "design", priority: 8 },
      { type: "keyword", value: "architecture", priority: 7 },
    ],
    examples: [
      {
        title: "Plan a refactor",
        input: "Plan the migration from REST to GraphQL",
        output: "Detailed plan with phases, dependencies, and risk assessment",
        tags: ["planning", "architecture"],
      },
    ],
    tags: ["planning", "design", "primary"],
    builtin: true,
    icon: "📋",
  },
  "code-reviewer": {
    displayName: "Code Reviewer",
    shortDescription: "Comprehensive code quality reviews with actionable feedback",
    category: "engineering",
    capabilities: [
      { id: "review", name: "Code Review", description: "Review code for quality issues", primary: true },
      { id: "suggestions", name: "Suggestions", description: "Provide improvement suggestions", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "review", priority: 10 },
      { type: "keyword", value: "code review", priority: 10 },
      { type: "event", value: "pr.opened", priority: 8 },
    ],
    examples: [
      {
        title: "Review a PR",
        input: "Review the changes in src/auth/",
        output: "Detailed review with issues categorized by severity",
        tags: ["review", "quality"],
      },
    ],
    tags: ["review", "quality", "engineering"],
    builtin: true,
    icon: "🔍",
    recommended: true,
  },
  "security-reviewer": {
    displayName: "Security Reviewer",
    shortDescription: "Analyzes code for security vulnerabilities",
    category: "engineering",
    capabilities: [
      { id: "security-audit", name: "Security Audit", description: "Identify security issues", primary: true },
      { id: "owasp-check", name: "OWASP Check", description: "Check OWASP Top 10", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "security", priority: 10 },
      { type: "keyword", value: "vulnerability", priority: 9 },
      { type: "context", value: "auth|payment|credential", priority: 8 },
    ],
    tags: ["security", "audit", "engineering"],
    builtin: true,
    icon: "🔒",
  },
  "tdd-guide": {
    displayName: "TDD Guide",
    shortDescription: "Enforces test-driven development methodology",
    category: "engineering",
    capabilities: [
      { id: "tdd", name: "TDD Workflow", description: "Guide through TDD process", primary: true },
      { id: "test-writing", name: "Test Writing", description: "Write tests first", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "tdd", priority: 10 },
      { type: "keyword", value: "test first", priority: 9 },
      { type: "keyword", value: "write tests", priority: 8 },
    ],
    tags: ["testing", "tdd", "engineering"],
    builtin: true,
    icon: "🧪",
  },
  architect: {
    displayName: "Architect",
    shortDescription: "Designs system architecture and establishes patterns",
    category: "engineering",
    capabilities: [
      { id: "architecture", name: "Architecture Design", description: "Design system architecture", primary: true },
      { id: "patterns", name: "Design Patterns", description: "Apply design patterns", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "architect", priority: 10 },
      { type: "keyword", value: "design system", priority: 9 },
      { type: "keyword", value: "scalability", priority: 7 },
    ],
    tags: ["architecture", "design", "engineering"],
    builtin: true,
    icon: "🏗️",
  },
  writer: {
    displayName: "Writer",
    shortDescription: "Long-form content writing (20k+ words)",
    category: "content",
    capabilities: [
      { id: "long-form", name: "Long-form Writing", description: "Write books and long articles", primary: true },
      { id: "outlining", name: "Outlining", description: "Create content outlines", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "write book", priority: 10 },
      { type: "keyword", value: "write article", priority: 9 },
      { type: "keyword", value: "content", priority: 7 },
    ],
    tags: ["writing", "content", "books"],
    builtin: true,
    icon: "✍️",
  },
  proofreader: {
    displayName: "Proofreader",
    shortDescription: "Grammar, spelling, and style checking",
    category: "content",
    capabilities: [
      { id: "proofreading", name: "Proofreading", description: "Check grammar and spelling", primary: true },
      { id: "style-check", name: "Style Check", description: "Ensure style consistency", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "proofread", priority: 10 },
      { type: "keyword", value: "grammar", priority: 9 },
      { type: "keyword", value: "spelling", priority: 8 },
    ],
    tags: ["proofreading", "editing", "content"],
    builtin: true,
    icon: "📝",
  },
  observer: {
    displayName: "Observer (祝融说)",
    shortDescription: "Analysis through Zhu Rong philosophy",
    category: "philosophy",
    capabilities: [
      { id: "philosophy", name: "Philosophical Analysis", description: "Apply observer theory", primary: true },
      { id: "possibility", name: "Possibility Space", description: "Explore possibility space", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "祝融说", priority: 10 },
      { type: "keyword", value: "observer", priority: 9 },
      { type: "keyword", value: "可能性", priority: 8 },
    ],
    tags: ["philosophy", "zhurong", "analysis"],
    builtin: true,
    icon: "👁️",
  },
  decision: {
    displayName: "Decision (CLOSE)",
    shortDescription: "Sustainable decision-making with CLOSE framework",
    category: "philosophy",
    capabilities: [
      { id: "decision", name: "Decision Analysis", description: "Apply CLOSE framework", primary: true },
      { id: "sustainability", name: "Sustainability", description: "Evaluate decision sustainability", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "decision", priority: 10 },
      { type: "keyword", value: "CLOSE", priority: 10 },
      { type: "keyword", value: "选择", priority: 8 },
    ],
    tags: ["decision", "close", "philosophy"],
    builtin: true,
    icon: "🎯",
  },
  macro: {
    displayName: "Macro Economist",
    shortDescription: "Macroeconomic data analysis",
    category: "analysis",
    capabilities: [
      { id: "macro-analysis", name: "Macro Analysis", description: "Analyze economic data", primary: true },
      { id: "data-interpretation", name: "Data Interpretation", description: "Interpret economic indicators", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "macro", priority: 10 },
      { type: "keyword", value: "GDP", priority: 9 },
      { type: "keyword", value: "economy", priority: 8 },
    ],
    tags: ["economics", "macro", "analysis"],
    builtin: true,
    icon: "📊",
  },
  trader: {
    displayName: "Trader Guide",
    shortDescription: "Short-term trading analysis (educational)",
    category: "analysis",
    capabilities: [
      { id: "technical-analysis", name: "Technical Analysis", description: "Chart and pattern analysis", primary: true },
      { id: "position-sizing", name: "Position Sizing", description: "Risk management", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "trade", priority: 10 },
      { type: "keyword", value: "trading", priority: 9 },
      { type: "keyword", value: "技术分析", priority: 8 },
    ],
    tags: ["trading", "analysis", "technical"],
    builtin: true,
    icon: "📈",
  },
  explore: {
    displayName: "Explorer",
    shortDescription: "Fast codebase exploration and search",
    category: "engineering",
    capabilities: [
      { id: "search", name: "Code Search", description: "Search codebase", primary: true },
      { id: "explore", name: "Exploration", description: "Explore file structure", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "find", priority: 10 },
      { type: "keyword", value: "search", priority: 9 },
      { type: "keyword", value: "where", priority: 7 },
    ],
    tags: ["search", "exploration", "engineering"],
    builtin: true,
    icon: "🔭",
  },
  autonomous: {
    displayName: "Autonomous",
    shortDescription: "Fully autonomous execution with self-correction",
    category: "system",
    capabilities: [
      { id: "autonomous", name: "Autonomous Execution", description: "Self-directed task completion", primary: true },
      { id: "self-correction", name: "Self Correction", description: "Detect and fix own errors", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "autonomous", priority: 10 },
      { type: "keyword", value: "自主", priority: 9 },
    ],
    tags: ["autonomous", "system", "self-directed"],
    builtin: true,
    icon: "🤖",
  },
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry Implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Agent Registry for dynamic agent management.
 */
export class AgentRegistry {
  private metadata: Map<string, AgentMetadata> = new Map()
  private searchIndex: Fuse<AgentMetadata> | null = null

  constructor() {
    // Initialize will be called separately
  }

  /**
   * Initialize registry with built-in agents.
   */
  async initialize(): Promise<void> {
    const agents = await Agent.list()

    for (const agent of agents) {
      const builtinMeta = BUILTIN_METADATA[agent.name]
      const metadata: AgentMetadata = {
        name: agent.name,
        displayName: builtinMeta?.displayName ?? agent.name,
        shortDescription: builtinMeta?.shortDescription ?? agent.description,
        longDescription: builtinMeta?.longDescription,
        category: builtinMeta?.category ?? "custom",
        capabilities: builtinMeta?.capabilities ?? [],
        triggers: builtinMeta?.triggers ?? [],
        examples: builtinMeta?.examples ?? [],
        tags: builtinMeta?.tags ?? [],
        author: builtinMeta?.author,
        version: builtinMeta?.version ?? "1.0.0",
        builtin: agent.native ?? false,
        icon: builtinMeta?.icon,
        recommended: builtinMeta?.recommended ?? false,
      }

      this.metadata.set(agent.name, metadata)
    }

    this.rebuildSearchIndex()
  }

  /**
   * Register a new agent with metadata.
   */
  register(metadata: AgentMetadata): void {
    const validated = AgentMetadata.parse(metadata)
    this.metadata.set(validated.name, validated)
    this.rebuildSearchIndex()
  }

  /**
   * Unregister an agent.
   */
  unregister(name: string): boolean {
    const existed = this.metadata.delete(name)
    if (existed) {
      this.rebuildSearchIndex()
    }
    return existed
  }

  /**
   * Get metadata for an agent.
   */
  get(name: string): AgentMetadata | undefined {
    return this.metadata.get(name)
  }

  /**
   * List all registered agents.
   */
  list(): AgentMetadata[] {
    return Array.from(this.metadata.values())
  }

  /**
   * List agents by category.
   */
  listByCategory(category: AgentCategory): AgentMetadata[] {
    return this.list().filter((a) => a.category === category)
  }

  /**
   * List recommended agents.
   */
  listRecommended(): AgentMetadata[] {
    return this.list().filter((a) => a.recommended)
  }

  /**
   * Search agents by query.
   */
  search(query: string, options?: { limit?: number; threshold?: number }): SearchResult[] {
    if (!this.searchIndex || !query.trim()) {
      return []
    }

    const results = this.searchIndex.search(query, {
      limit: options?.limit ?? 10,
    })

    return results
      .filter((r) => r.score !== undefined && r.score <= (options?.threshold ?? 0.6))
      .map((r) => ({
        agent: r.item,
        score: r.score ?? 1,
        matches: (r.matches ?? []).map((m) => ({
          key: m.key ?? "",
          value: m.value ?? "",
          indices: m.indices ?? [],
        })),
      }))
  }

  /**
   * Find agent by trigger match.
   */
  findByTrigger(input: string): AgentMetadata[] {
    const lowercaseInput = input.toLowerCase()
    const matches: Array<{ agent: AgentMetadata; priority: number }> = []

    for (const agent of this.metadata.values()) {
      for (const trigger of agent.triggers) {
        let matched = false

        switch (trigger.type) {
          case "keyword":
            matched = lowercaseInput.includes(trigger.value.toLowerCase())
            break
          case "pattern":
            try {
              matched = new RegExp(trigger.value, "i").test(input)
            } catch {
              matched = false
            }
            break
          case "context":
            try {
              matched = new RegExp(trigger.value, "i").test(input)
            } catch {
              matched = false
            }
            break
          case "event":
            // Event triggers are handled separately
            break
        }

        if (matched) {
          matches.push({ agent, priority: trigger.priority })
          break // Only count first trigger match per agent
        }
      }
    }

    // Sort by priority descending
    return matches.sort((a, b) => b.priority - a.priority).map((m) => m.agent)
  }

  /**
   * Find agent by event trigger.
   */
  findByEvent(eventName: string): AgentMetadata[] {
    return this.list().filter((agent) =>
      agent.triggers.some((t) => t.type === "event" && t.value === eventName)
    )
  }

  /**
   * Get agents with a specific capability.
   */
  findByCapability(capabilityId: string): AgentMetadata[] {
    return this.list().filter((agent) => agent.capabilities.some((c) => c.id === capabilityId))
  }

  /**
   * Recommend an agent based on user intent.
   */
  recommend(intent: string): AgentMetadata | undefined {
    // First try trigger matching
    const triggerMatches = this.findByTrigger(intent)
    if (triggerMatches.length > 0) {
      return triggerMatches[0]
    }

    // Fall back to search
    const searchResults = this.search(intent, { limit: 1 })
    if (searchResults.length > 0) {
      return searchResults[0].agent
    }

    // Fall back to default recommended
    const recommended = this.listRecommended()
    return recommended[0]
  }

  /**
   * Rebuild the search index.
   */
  private rebuildSearchIndex(): void {
    const items = Array.from(this.metadata.values())

    this.searchIndex = new Fuse(items, {
      keys: [
        { name: "name", weight: 2 },
        { name: "displayName", weight: 2 },
        { name: "shortDescription", weight: 1.5 },
        { name: "longDescription", weight: 1 },
        { name: "tags", weight: 1.5 },
        { name: "capabilities.name", weight: 1 },
        { name: "capabilities.description", weight: 0.8 },
        { name: "examples.title", weight: 0.8 },
        { name: "examples.input", weight: 0.6 },
      ],
      includeScore: true,
      includeMatches: true,
      threshold: 0.4,
      ignoreLocation: true,
    })
  }

  /**
   * Export registry as JSON.
   */
  toJSON(): Record<string, AgentMetadata> {
    return Object.fromEntries(this.metadata.entries())
  }

  /**
   * Import registry from JSON.
   */
  fromJSON(data: Record<string, unknown>): void {
    for (const [name, meta] of Object.entries(data)) {
      try {
        const metaObj = typeof meta === "object" && meta !== null ? meta : {}
        const validated = AgentMetadata.parse({ ...metaObj, name })
        this.metadata.set(name, validated)
      } catch (e) {
        console.warn(`Failed to import agent ${name}:`, e)
      }
    }
    this.rebuildSearchIndex()
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let registryInstance: AgentRegistry | null = null

/**
 * Get the global agent registry instance.
 */
export async function getRegistry(): Promise<AgentRegistry> {
  if (!registryInstance) {
    registryInstance = new AgentRegistry()
    await registryInstance.initialize()
  }
  return registryInstance
}

/**
 * Reset the registry (for testing).
 */
export function resetRegistry(): void {
  registryInstance = null
}
