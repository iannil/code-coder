/**
 * Agent Registry - Dynamic agent registration and discovery.
 *
 * Extends the existing Agent system with:
 * - Rich metadata (capabilities, triggers, examples)
 * - Runtime registration API
 * - Agent search and recommendations
 * - Category-based organization
 *
 * Triggers are loaded from external configuration:
 * - Default: packages/ccode/src/agent/keywords.default.json
 * - User override: ~/.codecoder/keywords.json
 *
 * ## Rust Backend
 *
 * Set `CODECODER_RUST_REGISTRY=1` to use the Rust fuzzy search implementation
 * via NAPI bindings. This provides better performance for large agent indexes.
 */

import z from "zod"
import { getAgentBridge, toAgentInfo } from "@/sdk/agent-bridge"
import Fuse from "fuse.js"

// Feature flag: Use Rust NAPI bindings for fuzzy search
const USE_RUST_REGISTRY = process.env.CODECODER_RUST_REGISTRY === "1"

// Lazy-load Rust bindings only when feature flag is enabled
type RustIndexHandle = {
  search(query: string, options?: { limit?: number; threshold?: number }): Promise<RustSearchResult[]>
  findByTrigger(input: string): Promise<RustAgentMetadata[]>
  recommend(intent: string): Promise<RustAgentMetadata | null>
  listByMode(mode: string): Promise<RustAgentMetadata[]>
  listByCategory(category: string): Promise<RustAgentMetadata[]>
  getPrimaryForMode(mode: string): Promise<RustAgentMetadata | null>
  list(): Promise<RustAgentMetadata[]>
  listVisible(): Promise<RustAgentMetadata[]>
  listRecommended(): Promise<RustAgentMetadata[]>
  get(name: string): Promise<RustAgentMetadata | null>
  count(): Promise<number>
}

interface RustAgentMetadata {
  name: string
  display_name?: string
  short_description?: string
  long_description?: string
  category: string
  mode?: string
  role: string
  capabilities: Array<{ id: string; name: string; description: string; primary: boolean }>
  triggers: Array<{ type: string; value: string; priority: number; description?: string }>
  examples: Array<{ title: string; input: string; output: string; tags: string[] }>
  tags: string[]
  author?: string
  version: string
  builtin: boolean
  icon?: string
  recommended: boolean
}

interface RustSearchResult {
  agent: RustAgentMetadata
  score: number
  matches: Array<{ key: string; value: string; indices: number[][] }>
}

let rustIndexHandle: RustIndexHandle | null = null

async function getRustIndex(): Promise<RustIndexHandle | null> {
  if (!USE_RUST_REGISTRY) return null
  if (rustIndexHandle) return rustIndexHandle

  try {
    // Dynamic import to avoid loading Rust bindings when not needed
    // Use require() with type assertion since NAPI types may not be generated
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const core = await import("@codecoder-ai/core") as { createAgentMetadataIndexWithBuiltins?: () => RustIndexHandle }
    if (!core.createAgentMetadataIndexWithBuiltins) {
      console.warn("[AgentRegistry] Rust NAPI bindings not available (function not exported)")
      return null
    }
    rustIndexHandle = core.createAgentMetadataIndexWithBuiltins()
    return rustIndexHandle
  } catch (e) {
    console.warn("[AgentRegistry] Failed to load Rust NAPI bindings, falling back to TypeScript:", e)
    return null
  }
}

/**
 * Convert Rust metadata to TypeScript format
 */
function rustToTsMetadata(r: RustAgentMetadata): AgentMetadata {
  return {
    name: r.name,
    displayName: r.display_name,
    shortDescription: r.short_description,
    longDescription: r.long_description,
    category: r.category as AgentCategory,
    mode: r.mode,
    role: r.role as AgentRole,
    capabilities: r.capabilities.map((c) => ({
      id: c.id,
      name: c.name,
      description: c.description,
      primary: c.primary,
    })),
    triggers: r.triggers.map((t) => ({
      type: t.type as "keyword" | "pattern" | "event" | "context",
      value: t.value,
      priority: t.priority,
      description: t.description,
    })),
    examples: r.examples.map((e) => ({
      title: e.title,
      input: e.input,
      output: e.output,
      tags: e.tags,
    })),
    tags: r.tags,
    author: r.author,
    version: r.version,
    builtin: r.builtin,
    icon: r.icon,
    recommended: r.recommended,
  }
}

/**
 * Convert Rust search result to TypeScript format
 */
function rustToTsSearchResult(r: RustSearchResult): SearchResult {
  return {
    agent: rustToTsMetadata(r.agent),
    score: r.score,
    matches: r.matches.map((m) => ({
      key: m.key,
      value: m.value,
      indices: m.indices.map(([start, end]) => [start, end] as [number, number]),
    })),
  }
}
import {
  getKeywords,
  detectAlias,
  detectTrigger,
  type KeywordsConfig,
  type TriggerRule,
  type AgentKeywords,
} from "@/config/keywords"
import {
  MODES,
  DEFAULT_MODE,
  getMode,
  getDefaultMode,
  listModes,
  agentBelongsToMode,
  findModesForAgent,
  getAgentsInMode,
  parseModeCapability,
  type Mode,
} from "./mode"

// Re-export mode system for convenience
export {
  MODES,
  DEFAULT_MODE,
  getMode,
  getDefaultMode,
  listModes,
  agentBelongsToMode,
  findModesForAgent,
  getAgentsInMode,
  parseModeCapability,
  type Mode,
}

// ─────────────────────────────────────────────────────────────────────────────
// Zod Workaround
// ─────────────────────────────────────────────────────────────────────────────

// Helper to avoid Zod v4.1.8 + Bun escapeRegex issue with .default([])
const defaultArray = <T extends z.ZodTypeAny>(schema: T) =>
  z.array(schema).optional().transform((v) => v ?? [])

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
  tags: defaultArray(z.string()),
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
 * Agent role within a mode.
 */
export const AgentRole = z.enum(["primary", "alternative", "capability", "system", "hidden"])
export type AgentRole = z.infer<typeof AgentRole>

/**
 * Extended agent metadata for registry.
 */
export const AgentMetadata = z.object({
  /** Agent name (must match AgentInfo.name from agent-bridge) */
  name: z.string(),
  /** Display name for UI */
  displayName: z.string().optional(),
  /** Short description (one line) */
  shortDescription: z.string().optional(),
  /** Long description (markdown supported) */
  longDescription: z.string().optional(),
  /** Agent category */
  category: AgentCategory.default("custom"),
  /** Mode this agent belongs to (build, writer, decision, or undefined for system) */
  mode: z.string().optional(),
  /** Role within the mode (primary, alternative, capability, system, hidden) */
  role: AgentRole.default("capability"),
  /** Agent capabilities */
  capabilities: defaultArray(AgentCapability),
  /** Auto-invocation triggers */
  triggers: defaultArray(AgentTrigger),
  /** Usage examples */
  examples: defaultArray(AgentExample),
  /** Tags for search */
  tags: defaultArray(z.string()),
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
  // ─────────────────────────────────────────────────────────────────────────────
  // @build Mode - Software Development
  // ─────────────────────────────────────────────────────────────────────────────
  general: {
    displayName: "General Assistant",
    shortDescription: "General-purpose assistant for conversation and queries",
    category: "custom",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "conversation", name: "Conversation", description: "Natural conversation", primary: true },
      { id: "query", name: "Query Handling", description: "Answer questions and queries", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "help", priority: 5 },
      { type: "keyword", value: "帮助", priority: 5 },
      { type: "keyword", value: "问", priority: 4 },
    ],
    examples: [
      {
        title: "General query",
        input: "What is the weather today?",
        output: "Responds with helpful information",
        tags: ["general", "query"],
      },
    ],
    tags: ["general", "assistant", "conversation"],
    builtin: true,
    icon: "💬",
    recommended: true,
  },
  build: {
    displayName: "Build",
    shortDescription: "Primary development agent for building features and fixing bugs",
    category: "engineering",
    mode: "build",
    role: "primary",
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
  },
  plan: {
    displayName: "Plan",
    shortDescription: "Creates detailed implementation plans before coding",
    category: "engineering",
    mode: "build",
    role: "alternative",
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
  autonomous: {
    displayName: "Autonomous",
    shortDescription: "Fully autonomous execution with self-correction",
    category: "system",
    mode: "build",
    role: "alternative",
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
  "code-reviewer": {
    displayName: "Code Reviewer",
    shortDescription: "Comprehensive code quality reviews with actionable feedback",
    category: "engineering",
    mode: "build",
    role: "capability",
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
  },
  "security-reviewer": {
    displayName: "Security Reviewer",
    shortDescription: "Analyzes code for security vulnerabilities",
    category: "engineering",
    mode: "build",
    role: "capability",
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
    mode: "build",
    role: "capability",
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
    mode: "build",
    role: "capability",
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
  explore: {
    displayName: "Explorer",
    shortDescription: "Fast codebase exploration and search",
    category: "engineering",
    mode: "build",
    role: "capability",
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
  "code-reverse": {
    displayName: "Code Reverse Engineer",
    shortDescription: "Reverse engineering and code analysis",
    category: "engineering",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "reverse-engineering", name: "Reverse Engineering", description: "Analyze and understand code", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "reverse", priority: 10 },
      { type: "keyword", value: "decompile", priority: 9 },
    ],
    tags: ["reverse-engineering", "analysis", "engineering"],
    builtin: true,
    icon: "🔬",
  },
  "jar-code-reverse": {
    displayName: "JAR Reverse Engineer",
    shortDescription: "Reverse engineering for Java JAR files",
    category: "engineering",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "jar-analysis", name: "JAR Analysis", description: "Analyze Java JAR files", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "jar", priority: 10 },
      { type: "keyword", value: "java reverse", priority: 9 },
    ],
    tags: ["java", "jar", "reverse-engineering"],
    builtin: true,
    icon: "☕",
  },
  verifier: {
    displayName: "Verifier",
    shortDescription: "Verification and validation agent",
    category: "engineering",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "verification", name: "Verification", description: "Verify code and outputs", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "verify", priority: 10 },
      { type: "keyword", value: "validate", priority: 9 },
    ],
    tags: ["verification", "validation"],
    builtin: true,
    icon: "✅",
  },
  "prd-generator": {
    displayName: "PRD Generator",
    shortDescription: "Generate product requirement documents",
    category: "engineering",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "prd", name: "PRD Generation", description: "Generate product requirements", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "prd", priority: 10 },
      { type: "keyword", value: "requirements", priority: 8 },
    ],
    tags: ["product", "requirements", "documentation"],
    builtin: true,
    icon: "📄",
  },
  "feasibility-assess": {
    displayName: "Feasibility Assessor",
    shortDescription: "Assess technical and business feasibility",
    category: "engineering",
    mode: "build",
    role: "capability",
    capabilities: [
      { id: "feasibility", name: "Feasibility Assessment", description: "Assess project feasibility", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "feasibility", priority: 10 },
      { type: "keyword", value: "assess", priority: 7 },
    ],
    tags: ["feasibility", "assessment", "planning"],
    builtin: true,
    icon: "📊",
  },

  // ─────────────────────────────────────────────────────────────────────────────
  // @writer Mode - Content Creation
  // ─────────────────────────────────────────────────────────────────────────────
  writer: {
    displayName: "Writer",
    shortDescription: "Long-form content writing (20k+ words)",
    category: "content",
    mode: "writer",
    role: "primary",
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
  expander: {
    displayName: "Expander",
    shortDescription: "Unified content expansion (fiction/nonfiction auto-detection)",
    category: "content",
    mode: "writer",
    role: "capability",
    capabilities: [
      { id: "expansion", name: "Content Expansion", description: "Expand brief content into detailed writing", primary: true },
      { id: "fiction-expansion", name: "Fiction Expansion", description: "Expand fiction narratives with domain auto-detection or [DOMAIN:fiction] tag", primary: false },
      { id: "nonfiction-expansion", name: "Nonfiction Expansion", description: "Expand nonfiction content with domain auto-detection or [DOMAIN:nonfiction] tag", primary: false },
    ],
    triggers: [
      { type: "keyword", value: "expand", priority: 10 },
      { type: "keyword", value: "elaborate", priority: 9 },
      { type: "keyword", value: "fiction", priority: 8 },
      { type: "keyword", value: "story", priority: 7 },
      { type: "keyword", value: "nonfiction", priority: 8 },
      { type: "keyword", value: "technical writing", priority: 7 },
    ],
    tags: ["expansion", "elaboration", "content", "fiction", "narrative", "creative-writing", "nonfiction", "technical", "educational"],
    builtin: true,
    icon: "📖",
  },
  proofreader: {
    displayName: "Proofreader",
    shortDescription: "Grammar, spelling, and style checking",
    category: "content",
    mode: "writer",
    role: "capability",
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

  // ─────────────────────────────────────────────────────────────────────────────
  // @decision Mode - Decision Making & Philosophy (祝融说)
  // ─────────────────────────────────────────────────────────────────────────────
  decision: {
    displayName: "Decision (CLOSE)",
    shortDescription: "Sustainable decision-making with CLOSE framework",
    category: "philosophy",
    mode: "decision",
    role: "primary",
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
  observer: {
    displayName: "Observer (祝融说)",
    shortDescription: "Analysis through Zhu Rong philosophy",
    category: "philosophy",
    mode: "decision",
    role: "alternative",
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
  macro: {
    displayName: "Macro Economist",
    shortDescription: "Macroeconomic data analysis",
    category: "analysis",
    mode: "decision",
    role: "capability",
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
    mode: "decision",
    role: "capability",
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
  "value-analyst": {
    displayName: "Value Analyst",
    shortDescription: "Value investing and fundamental analysis",
    category: "analysis",
    mode: "decision",
    role: "capability",
    capabilities: [
      { id: "value-analysis", name: "Value Analysis", description: "Analyze intrinsic value", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "value", priority: 9 },
      { type: "keyword", value: "fundamental", priority: 8 },
    ],
    tags: ["value-investing", "fundamentals", "analysis"],
    builtin: true,
    icon: "💎",
  },
  picker: {
    displayName: "Product Picker",
    shortDescription: "Product selection and evaluation",
    category: "analysis",
    mode: "decision",
    role: "capability",
    capabilities: [
      { id: "selection", name: "Product Selection", description: "Evaluate and select products", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "pick", priority: 10 },
      { type: "keyword", value: "选品", priority: 9 },
    ],
    tags: ["selection", "product", "evaluation"],
    builtin: true,
    icon: "🛒",
  },
  miniproduct: {
    displayName: "Mini Product Designer",
    shortDescription: "Design minimal viable products",
    category: "analysis",
    mode: "decision",
    role: "capability",
    capabilities: [
      { id: "mvp", name: "MVP Design", description: "Design minimal viable products", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "miniproduct", priority: 10 },
      { type: "keyword", value: "mvp", priority: 9 },
      { type: "keyword", value: "极小产品", priority: 9 },
    ],
    tags: ["mvp", "product", "design"],
    builtin: true,
    icon: "🎨",
  },
  "ai-engineer": {
    displayName: "AI Engineer",
    shortDescription: "AI/ML engineering guidance",
    category: "analysis",
    mode: "decision",
    role: "capability",
    capabilities: [
      { id: "ai-guidance", name: "AI Guidance", description: "Guide AI/ML engineering decisions", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "ai engineer", priority: 10 },
      { type: "keyword", value: "ml", priority: 8 },
    ],
    tags: ["ai", "ml", "engineering"],
    builtin: true,
    icon: "🤖",
  },
  "synton-assistant": {
    displayName: "Synton Assistant",
    shortDescription: "Synton project specialized assistance",
    category: "custom",
    mode: "decision",
    role: "capability",
    capabilities: [
      { id: "synton", name: "Synton Support", description: "Synton project assistance", primary: true },
    ],
    triggers: [
      { type: "keyword", value: "synton", priority: 10 },
    ],
    tags: ["synton", "specialized"],
    builtin: true,
    icon: "🔧",
  },

  // ─────────────────────────────────────────────────────────────────────────────
  // System Agents (Hidden from users)
  // ─────────────────────────────────────────────────────────────────────────────
  compaction: {
    displayName: "Compaction",
    shortDescription: "Context window compaction",
    category: "system",
    role: "hidden",
    capabilities: [],
    triggers: [],
    tags: ["system", "internal"],
    builtin: true,
    icon: "📦",
  },
  title: {
    displayName: "Title Generator",
    shortDescription: "Generate conversation titles",
    category: "system",
    role: "hidden",
    capabilities: [],
    triggers: [],
    tags: ["system", "internal"],
    builtin: true,
    icon: "🏷️",
  },
  summary: {
    displayName: "Summary Generator",
    shortDescription: "Generate conversation summaries",
    category: "system",
    role: "hidden",
    capabilities: [],
    triggers: [],
    tags: ["system", "internal"],
    builtin: true,
    icon: "📋",
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
  private keywordsConfig: KeywordsConfig | null = null

  constructor() {
    // Initialize will be called separately
  }

  /**
   * Initialize registry with built-in agents.
   *
   * Loads agent metadata from built-in defaults and triggers from
   * the keywords configuration file.
   */
  async initialize(): Promise<void> {
    const bridge = await getAgentBridge()
    const [rawAgents, keywordsConfig] = await Promise.all([
      bridge.list(),
      getKeywords(),
    ])
    const agents = rawAgents.map(toAgentInfo)

    this.keywordsConfig = keywordsConfig

    for (const agent of agents) {
      const builtinMeta = BUILTIN_METADATA[agent.name]
      const keywordsMeta = keywordsConfig.agents[agent.name]

      // Build triggers from keywords config (with fallback to builtin)
      const triggers = this.buildTriggersFromKeywords(agent.name, keywordsMeta, builtinMeta?.triggers)

      const metadata: AgentMetadata = {
        name: agent.name,
        displayName: builtinMeta?.displayName ?? agent.name,
        shortDescription: builtinMeta?.shortDescription ?? agent.description,
        longDescription: builtinMeta?.longDescription,
        category: builtinMeta?.category ?? "custom",
        mode: builtinMeta?.mode,
        role: builtinMeta?.role ?? "capability",
        capabilities: builtinMeta?.capabilities ?? [],
        triggers,
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
   * Build triggers array from keywords config.
   */
  private buildTriggersFromKeywords(
    agentName: string,
    keywordsMeta: AgentKeywords | undefined,
    fallbackTriggers: AgentTrigger[] | undefined
  ): AgentTrigger[] {
    // If no keywords config, fall back to builtin
    if (!keywordsMeta) {
      return fallbackTriggers ?? []
    }

    const triggers: AgentTrigger[] = []

    // Convert keywords config triggers to AgentTrigger format
    for (const trigger of keywordsMeta.triggers) {
      if (typeof trigger === "string") {
        // Simple string trigger
        triggers.push({
          type: "keyword",
          value: trigger,
          priority: keywordsMeta.priority,
        })
      } else {
        // Advanced trigger rule
        triggers.push({
          type: trigger.type,
          value: trigger.value,
          priority: trigger.priority ?? keywordsMeta.priority,
          description: trigger.description,
        })
      }
    }

    // Also add aliases as keyword triggers (lower priority)
    for (const alias of keywordsMeta.aliases) {
      // Don't add if already exists as a trigger
      const alreadyExists = triggers.some((t) =>
        t.type === "keyword" && t.value.toLowerCase() === alias.toLowerCase()
      )
      if (!alreadyExists && alias.toLowerCase() !== agentName.toLowerCase()) {
        triggers.push({
          type: "keyword",
          value: alias,
          priority: keywordsMeta.priority - 1,
        })
      }
    }

    return triggers
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
   * List agents by mode.
   */
  listByMode(modeId: string): AgentMetadata[] {
    return this.list().filter((a) => a.mode === modeId)
  }

  /**
   * List agents by role within a mode.
   */
  listByRole(role: AgentRole): AgentMetadata[] {
    return this.list().filter((a) => a.role === role)
  }

  /**
   * Get primary agent for a mode.
   */
  getPrimaryForMode(modeId: string): AgentMetadata | undefined {
    return this.list().find((a) => a.mode === modeId && a.role === "primary")
  }

  /**
   * Get all capabilities (non-primary agents) for a mode.
   */
  getCapabilitiesForMode(modeId: string): AgentMetadata[] {
    return this.list().filter((a) => a.mode === modeId && a.role === "capability")
  }

  /**
   * List agents visible to users (excludes hidden/system).
   */
  listVisible(): AgentMetadata[] {
    return this.list().filter((a) => a.role !== "hidden")
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
   *
   * Priority:
   * 1. @mention alias match (via keywords config)
   * 2. Keyword trigger match (via loaded triggers)
   * 3. Fuzzy search match
   * 4. Default recommended agent
   */
  recommend(intent: string): AgentMetadata | undefined {
    const trimmed = intent.trim()

    // First try @mention alias detection (from keywords config)
    if (this.keywordsConfig && trimmed.startsWith("@")) {
      const aliasMatch = detectAlias(intent, this.keywordsConfig)
      if (aliasMatch) {
        const agentMeta = this.metadata.get(aliasMatch)
        if (agentMeta) {
          return agentMeta
        }
      }
    }

    // Then try implicit trigger matching (from keywords config)
    if (this.keywordsConfig) {
      const triggerMatch = detectTrigger(intent, this.keywordsConfig)
      if (triggerMatch) {
        const agentMeta = this.metadata.get(triggerMatch)
        if (agentMeta) {
          return agentMeta
        }
      }
    }

    // Fall back to legacy trigger matching
    const triggerMatches = this.findByTrigger(intent)
    if (triggerMatches.length > 0) {
      return triggerMatches[0]
    }

    // For short inputs (≤3 chars), skip fuzzy search to avoid false matches
    // E.g., "hi" should not match "architect"
    if (trimmed.length <= 3) {
      const recommended = this.listRecommended()
      return recommended[0]
    }

    // Fall back to search for longer inputs
    const searchResults = this.search(intent, { limit: 1 })
    if (searchResults.length > 0) {
      return searchResults[0].agent
    }

    // Fall back to default recommended
    const recommended = this.listRecommended()
    return recommended[0]
  }

  // =========================================================================
  // Async Methods with Rust Backend Support
  // =========================================================================

  /**
   * Search agents using Rust backend when available (async).
   *
   * When `CODECODER_RUST_REGISTRY=1` is set, uses the high-performance Rust
   * fuzzy search implementation via NAPI bindings. Falls back to Fuse.js.
   */
  async searchAsync(query: string, options?: { limit?: number; threshold?: number }): Promise<SearchResult[]> {
    const rustIndex = await getRustIndex()
    if (rustIndex) {
      const results = await rustIndex.search(query, options)
      return results.map(rustToTsSearchResult)
    }
    return this.search(query, options)
  }

  /**
   * Find agent by trigger match using Rust backend when available (async).
   */
  async findByTriggerAsync(input: string): Promise<AgentMetadata[]> {
    const rustIndex = await getRustIndex()
    if (rustIndex) {
      const results = await rustIndex.findByTrigger(input)
      return results.map(rustToTsMetadata)
    }
    return this.findByTrigger(input)
  }

  /**
   * Recommend an agent using Rust backend when available (async).
   */
  async recommendAsync(intent: string): Promise<AgentMetadata | undefined> {
    const rustIndex = await getRustIndex()
    if (rustIndex) {
      const result = await rustIndex.recommend(intent)
      return result ? rustToTsMetadata(result) : undefined
    }
    return this.recommend(intent)
  }

  /**
   * List agents by mode using Rust backend when available (async).
   */
  async listByModeAsync(modeId: string): Promise<AgentMetadata[]> {
    const rustIndex = await getRustIndex()
    if (rustIndex) {
      const results = await rustIndex.listByMode(modeId)
      return results.map(rustToTsMetadata)
    }
    return this.listByMode(modeId)
  }

  /**
   * Get primary agent for mode using Rust backend when available (async).
   */
  async getPrimaryForModeAsync(modeId: string): Promise<AgentMetadata | undefined> {
    const rustIndex = await getRustIndex()
    if (rustIndex) {
      const result = await rustIndex.getPrimaryForMode(modeId)
      return result ? rustToTsMetadata(result) : undefined
    }
    return this.getPrimaryForMode(modeId)
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

/**
 * Check if Rust backend is enabled via CODECODER_RUST_REGISTRY=1.
 */
export function isRustBackendEnabled(): boolean {
  return USE_RUST_REGISTRY
}

/**
 * Check if Rust backend is available (enabled and successfully loaded).
 */
export async function isRustBackendAvailable(): Promise<boolean> {
  if (!USE_RUST_REGISTRY) return false
  const index = await getRustIndex()
  return index !== null
}
