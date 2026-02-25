/**
 * Evolution Memory Writer
 *
 * Integrates the autonomous evolution loop with the dual-layer memory system:
 * 1. Daily notes (flow layer): Immediate chronological logging
 * 2. MEMORY.md (sediment layer): Long-term knowledge extraction
 *
 * Following the memory architecture defined in CLAUDE.md:
 * - ./memory/daily/{YYYY-MM-DD}.md: Session logs, decisions, solutions
 * - ./memory/MEMORY.md: User preferences, project context, lessons learned
 *
 * Part of Phase 1: Autonomous Problem-Solving Loop Enhancement
 */

import { Log } from "@/util/log"
import { appendDailyNote, createEntry } from "@/memory-markdown/daily"
import { mergeToCategory, addListItem } from "@/memory-markdown/long-term"
import type { DailyEntry, DailyEntryType, MemoryCategory } from "@/memory-markdown/types"
import type { EvolutionResult, AutonomousProblem, SolutionAttempt } from "./evolution-loop"

const log = Log.create({ service: "autonomous.memory-writer" })

// ============================================================================
// Types
// ============================================================================

/** Context for writing evolution result to memory */
export interface EvolutionMemoryContext {
  /** The original problem */
  problem: AutonomousProblem
  /** Evolution result */
  result: EvolutionResult
  /** Session ID for correlation */
  sessionId: string
}

/** Options for memory writing */
export interface MemoryWriteOptions {
  /** Write to daily notes (default: true) */
  writeDailyNote: boolean
  /** Write to long-term memory (default: true for solved problems) */
  writeLongTerm: boolean
  /** Minimum attempts before writing to long-term (avoid trivial solutions) */
  minAttemptsForLongTerm: number
  /** Include code in daily notes (default: false for brevity) */
  includeCodeInDaily: boolean
}

const DEFAULT_OPTIONS: MemoryWriteOptions = {
  writeDailyNote: true,
  writeLongTerm: true,
  minAttemptsForLongTerm: 1,
  includeCodeInDaily: false,
}

// ============================================================================
// Memory Writer
// ============================================================================

/**
 * Write evolution result to the dual-layer memory system
 *
 * This implements the "沉淀与进化" (Sedimentation) step of the evolution cycle,
 * ensuring successful solutions become part of the system's long-term knowledge.
 */
export async function writeEvolutionToMemory(
  context: EvolutionMemoryContext,
  options?: Partial<MemoryWriteOptions>,
): Promise<void> {
  const opts = { ...DEFAULT_OPTIONS, ...options }
  const { problem, result, sessionId } = context

  try {
    // Write to daily notes (immediate logging)
    if (opts.writeDailyNote) {
      await writeToDailyNotes(problem, result, sessionId, opts.includeCodeInDaily)
    }

    // Write to long-term memory (sediment layer) only for solved problems
    // with meaningful effort (avoid trivial solutions)
    if (
      opts.writeLongTerm &&
      result.solved &&
      result.attempts.length >= opts.minAttemptsForLongTerm
    ) {
      await writeToLongTermMemory(problem, result)
    }

    log.info("Evolution result written to memory", {
      sessionId,
      solved: result.solved,
      attempts: result.attempts.length,
      dailyWritten: opts.writeDailyNote,
      longTermWritten: opts.writeLongTerm && result.solved,
    })
  } catch (error) {
    // Don't fail the evolution loop if memory writing fails
    log.warn("Failed to write evolution result to memory", {
      error: error instanceof Error ? error.message : String(error),
      sessionId,
    })
  }
}

/**
 * Write to daily notes (flow layer)
 *
 * Creates an append-only log entry in ./memory/daily/{YYYY-MM-DD}.md
 */
async function writeToDailyNotes(
  problem: AutonomousProblem,
  result: EvolutionResult,
  sessionId: string,
  includeCode: boolean,
): Promise<void> {
  const content = formatDailyNoteContent(problem, result, includeCode)

  // Use "solution" type for successful evolution, "error" for failures
  const entryType: DailyEntryType = result.solved ? "solution" : "error"

  const entry: DailyEntry = createEntry(
    entryType,
    content,
    {
      sessionId,
      solved: result.solved,
      attempts: result.attempts.length,
      durationMs: result.durationMs,
      knowledgeId: result.knowledgeId,
      learnedToolId: result.learnedToolId,
      usedToolId: result.usedToolId,
      source: "autonomous_evolution",
    },
  )

  await appendDailyNote(entry)
  log.debug("Written to daily notes", { sessionId, type: entry.type })
}

/**
 * Format content for daily note entry
 */
function formatDailyNoteContent(
  problem: AutonomousProblem,
  result: EvolutionResult,
  includeCode: boolean,
): string {
  const lines: string[] = []

  // Header with status
  const status = result.solved ? "✅ Solved" : "❌ Not Solved"
  lines.push(`### Autonomous Evolution: ${status}`)
  lines.push("")

  // Problem description
  lines.push("**Problem:**")
  lines.push(problem.description.slice(0, 300) + (problem.description.length > 300 ? "..." : ""))
  lines.push("")

  // Error if present
  if (problem.errorMessage) {
    lines.push("**Error:**")
    lines.push("```")
    lines.push(problem.errorMessage.slice(0, 200))
    lines.push("```")
    lines.push("")
  }

  // Summary
  lines.push(`**Summary:** ${result.summary}`)
  lines.push("")

  // Stats
  lines.push(`- **Attempts:** ${result.attempts.length}`)
  lines.push(`- **Duration:** ${(result.durationMs / 1000).toFixed(1)}s`)
  if (result.knowledgeId) {
    lines.push(`- **Knowledge ID:** ${result.knowledgeId}`)
  }
  if (result.learnedToolId) {
    lines.push(`- **Learned Tool:** ${result.learnedToolId}`)
  }
  if (result.usedToolId) {
    lines.push(`- **Used Tool:** ${result.usedToolId}`)
  }

  // Include code if requested
  if (includeCode && result.solution) {
    lines.push("")
    lines.push("**Solution Code:**")
    lines.push("```")
    lines.push(result.solution.slice(0, 1000))
    lines.push("```")
  }

  return lines.join("\n")
}

/**
 * Write to long-term memory (sediment layer)
 *
 * Extracts lessons learned and adds to ./memory/MEMORY.md
 */
async function writeToLongTermMemory(
  problem: AutonomousProblem,
  result: EvolutionResult,
): Promise<void> {
  // Extract lesson from the evolution process
  const lesson = extractLesson(problem, result)

  // Add to 经验教训 (Lessons Learned) category
  await addListItem(
    "经验教训" as MemoryCategory,
    lesson.title,
    lesson.description,
  )

  log.debug("Written to long-term memory", { title: lesson.title })
}

/**
 * Extract a lesson from the evolution process
 */
function extractLesson(
  problem: AutonomousProblem,
  result: EvolutionResult,
): { title: string; description: string } {
  // Determine the type of lesson based on the problem
  let title: string
  let description: string

  // Check if it was an error resolution
  if (problem.errorMessage) {
    const errorType = extractErrorType(problem.errorMessage)
    title = `${errorType} Resolution`
    description = `Problem: "${problem.description.slice(0, 50)}..." → Solved in ${result.attempts.length} attempt(s)`
  } else {
    // General problem solving
    const category = problem.technology ?? "general"
    title = `${category} Solution Pattern`
    description = `"${problem.description.slice(0, 50)}..." → ${result.summary.slice(0, 100)}`
  }

  // Add reflection from last attempt if available
  const lastAttempt = result.attempts[result.attempts.length - 1]
  if (lastAttempt?.reflection?.analysis) {
    description += ` | Key insight: ${lastAttempt.reflection.analysis.slice(0, 80)}`
  }

  return { title, description }
}

/**
 * Extract error type from error message
 */
function extractErrorType(errorMessage: string): string {
  // Common error patterns
  const patterns = [
    { regex: /(\w+Error)/, group: 1 },
    { regex: /(\w+Exception)/, group: 1 },
    { regex: /Error:\s*(\w+)/, group: 1 },
    { regex: /(\d{3})\s*(Unauthorized|Forbidden|Not Found)/i, group: 0 },
  ]

  for (const { regex, group } of patterns) {
    const match = errorMessage.match(regex)
    if (match) {
      return match[group] ?? "Error"
    }
  }

  return "Error"
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Quick write for successful evolution results
 */
export async function sedimentEvolutionSuccess(
  problem: AutonomousProblem,
  result: EvolutionResult,
): Promise<void> {
  if (!result.solved) {
    log.debug("Skipping sedimentation for unsolved problem")
    return
  }

  await writeEvolutionToMemory({
    problem,
    result,
    sessionId: problem.sessionId,
  })
}

/**
 * Log evolution failure for debugging
 */
export async function logEvolutionFailure(
  problem: AutonomousProblem,
  result: EvolutionResult,
): Promise<void> {
  await writeEvolutionToMemory(
    {
      problem,
      result,
      sessionId: problem.sessionId,
    },
    {
      writeDailyNote: true,
      writeLongTerm: false, // Don't pollute long-term memory with failures
    },
  )
}
