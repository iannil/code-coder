/**
 * Memory Bridge
 *
 * Bridges existing memory system with the new Markdown layer.
 * Each system serves different purposes:
 *
 * - Existing memory (@/memory): Vector search, code indexing, pattern learning
 * - Markdown layer (@/memory-markdown): User preferences, decisions, lessons learned
 */

import { Log } from "@/util/log"
import { getAgentContext } from "./context"
import { loadMarkdownMemoryContext, loadRecentContext } from "@/memory-markdown"

const log = Log.create({ service: "agent.memory-bridge" })

/**
 * Combined memory context for agents
 */
export interface BridgedMemoryContext {
  technical: Awaited<ReturnType<typeof getAgentContext>>
  markdown: Awaited<ReturnType<typeof loadMarkdownMemoryContext>>
  formatted: string
}

/**
 * Build complete memory context combining both systems
 */
export async function buildMemoryContext(options?: {
  task?: string
  filePaths?: string[]
  includeMarkdownDays?: number
}): Promise<BridgedMemoryContext> {
  const { task, filePaths, includeMarkdownDays = 3 } = options ?? {}

  try {
    const [technical, markdown] = await Promise.all([
      getAgentContext(task ?? "", filePaths),
      loadMarkdownMemoryContext({ includeDays: includeMarkdownDays }),
    ])

    const formatted = formatBridgedContext(technical, markdown)

    log.debug("built bridged memory context", {
      hasTechnical: !!technical,
      markdownDailyCount: markdown.daily.length,
      combinedLength: formatted.length,
    })

    return { technical, markdown, formatted }
  } catch (error) {
    log.warn("failed to build complete context, using partial", { error })

    const technical = await getAgentContext(task ?? "", filePaths).catch(() => null)

    return {
      technical: technical ?? getDefaultTechnicalContext(),
      markdown: {
        longTerm: "",
        daily: [],
        combined: "",
      },
      formatted: formatTechnicalOnly(technical),
    }
  }
}

/**
 * Get recent markdown memory for prompt injection
 */
export async function getRecentMarkdownMemory(days = 1): Promise<string> {
  try {
    return await loadRecentContext(days)
  } catch (error) {
    log.warn("failed to load recent markdown memory", { error })
    return ""
  }
}

/**
 * Format combined context for agent consumption
 */
function formatBridgedContext(
  technical: Awaited<ReturnType<typeof getAgentContext>>,
  markdown: Awaited<ReturnType<typeof loadMarkdownMemoryContext>>,
): string {
  const parts: string[] = []

  parts.push("# Complete Memory Context")
  parts.push("")

  // Technical context from existing memory system
  if (technical) {
    parts.push("## Technical Context")
    parts.push("")
    parts.push(formatTechnicalContext(technical))
    parts.push("")
  }

  // Markdown memory layer
  if (markdown.combined) {
    parts.push("## Markdown Memory Layer")
    parts.push("")
    parts.push(markdown.combined)
  }

  return parts.join("\n")
}

/**
 * Format technical context section
 */
function formatTechnicalContext(context: Awaited<ReturnType<typeof getAgentContext>>): string {
  const lines: string[] = []

  if (context.projectFingerprint) {
    lines.push(`**Project:** ${context.projectFingerprint}`)
  }

  if (context.codeStyle) {
    lines.push(`**Code Style:** ${context.codeStyle}`)
  }

  if (context.learnedPatterns.length > 0) {
    lines.push(`**Patterns:** ${context.learnedPatterns.join(", ")}`)
  }

  return lines.join("\n")
}

/**
 * Get default technical context when loading fails
 */
function getDefaultTechnicalContext(): Awaited<ReturnType<typeof getAgentContext>> {
  return {
    projectFingerprint: "unknown",
    codeStyle: "",
    learnedPatterns: [],
    projectKnowledge: { apiEndpoints: 0, components: 0, dataModels: 0 },
    relevantFiles: [],
    recentEdits: [],
    decisions: [],
  }
}

/**
 * Format technical-only context
 */
function formatTechnicalOnly(technical: Awaited<ReturnType<typeof getAgentContext>> | null): string {
  if (!technical) return "# Technical Context\n\n_No technical context available._\n"

  return `# Technical Context\n\n${formatTechnicalContext(technical)}\n`
}
