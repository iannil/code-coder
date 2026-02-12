import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import type { DecisionRecord as CriteriaDecisionRecord, DecisionType } from "./criteria"
import { Decision as MemoryDecision } from "@/memory/history/decisions"
import z from "zod"

// Re-export DecisionRecord type for convenience
export type DecisionRecord = CriteriaDecisionRecord

const log = Log.create({ service: "autonomous.decision.history" })

/**
 * Stored decision record format
 */
const StoredDecisionRecord = z.object({
  id: z.string(),
  sessionId: z.string(),
  type: z.enum(["architecture", "implementation", "refactor", "bugfix", "feature", "test", "rollback", "checkpoint", "resource", "other"]),
  description: z.string(),
  context: z.string(),
  score: z.object({
    convergence: z.number(),
    leverage: z.number(),
    optionality: z.number(),
    surplus: z.number(),
    evolution: z.number(),
    total: z.number(),
  }),
  result: z.enum(["proceed", "proceed_with_caution", "pause", "block", "skip"]),
  reasoning: z.string(),
  timestamp: z.number(),
  criteria: z.record(z.string(), z.any()),
})

type StoredDecisionRecord = z.infer<typeof StoredDecisionRecord>

/**
 * Decision history manager
 *
 * Persists and retrieves decision records
 */
export namespace DecisionHistory {
  const STORAGE_PREFIX = ["autonomous", "decisions"]

  /**
   * Save a decision record
   */
  export async function save(record: CriteriaDecisionRecord): Promise<void> {
    const projectID = Instance.project.id

    const stored: StoredDecisionRecord = {
      id: record.id,
      sessionId: record.sessionId,
      type: record.type,
      description: record.description,
      context: record.context,
      score: record.score,
      result: record.result,
      reasoning: record.reasoning,
      timestamp: record.timestamp,
      criteria: record.criteria as unknown as Record<string, unknown>,
    }

    await Storage.write([...STORAGE_PREFIX, projectID, record.id], stored)
    log.info("Decision saved", { id: record.id, type: record.type })
  }

  /**
   * Get a decision by ID
   */
  export async function get(id: string): Promise<DecisionRecord | undefined> {
    const projectID = Instance.project.id

    try {
      const stored = await Storage.read<StoredDecisionRecord>([...STORAGE_PREFIX, projectID, id])
      return fromStored(stored)
    } catch {
      return undefined
    }
  }

  /**
   * Get all decisions for a session
   */
  export async function getBySession(sessionId: string): Promise<DecisionRecord[]> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])
      const records: DecisionRecord[] = []

      for (const key of keys) {
        try {
          const stored = await Storage.read<StoredDecisionRecord>(key)
          if (stored.sessionId === sessionId) {
            records.push(fromStored(stored))
          }
        } catch {
          // Skip invalid records
        }
      }

      return records.sort((a, b) => b.timestamp - a.timestamp)
    } catch {
      return []
    }
  }

  /**
   * Get decisions by type
   */
  export async function getByType(type: DecisionType, limit = 50): Promise<DecisionRecord[]> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])
      const records: DecisionRecord[] = []

      for (const key of keys) {
        try {
          const stored = await Storage.read<StoredDecisionRecord>(key)
          if (stored.type === type) {
            records.push(fromStored(stored))
          }
        } catch {
          // Skip invalid records
        }
      }

      return records.sort((a, b) => b.timestamp - a.timestamp).slice(0, limit)
    } catch {
      return []
    }
  }

  /**
   * Get recent decisions
   */
  export async function getRecent(limit = 20): Promise<DecisionRecord[]> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])
      const records: DecisionRecord[] = []

      for (const key of keys) {
        try {
          const stored = await Storage.read<StoredDecisionRecord>(key)
          records.push(fromStored(stored))
        } catch {
          // Skip invalid records
        }
      }

      return records.sort((a, b) => b.timestamp - a.timestamp).slice(0, limit)
    } catch {
      return []
    }
  }

  /**
   * Search decisions
   */
  export async function search(query: string): Promise<DecisionRecord[]> {
    const all = await getRecent(100)
    const lowerQuery = query.toLowerCase()

    return all.filter(
      (d) =>
        d.description.toLowerCase().includes(lowerQuery) ||
        d.reasoning.toLowerCase().includes(lowerQuery) ||
        d.type.toLowerCase().includes(lowerQuery),
    )
  }

  /**
   * Delete a decision
   */
  export async function remove(id: string): Promise<boolean> {
    const projectID = Instance.project.id

    try {
      await Storage.remove([...STORAGE_PREFIX, projectID, id])
      log.info("Decision removed", { id })
      return true
    } catch {
      return false
    }
  }

  /**
   * Clear all decisions for a session
   */
  export async function clearSession(sessionId: string): Promise<number> {
    const records = await getBySession(sessionId)
    let count = 0

    for (const record of records) {
      if (await remove(record.id)) {
        count++
      }
    }

    log.info("Session decisions cleared", { sessionId, count })
    return count
  }

  /**
   * Get decision statistics
   */
  export async function getStats(): Promise<{
    total: number
    byType: Record<DecisionType, number>
    byResult: Record<string, number>
    averageScore: number
  }> {
    const records = await getRecent(1000)

    const byType: Record<string, number> = {}
    const byResult: Record<string, number> = {}
    let totalScore = 0

    for (const record of records) {
      byType[record.type] = (byType[record.type] ?? 0) + 1
      byResult[record.result] = (byResult[record.result] ?? 0) + 1
      totalScore += record.score.total
    }

    return {
      total: records.length,
      byType: byType as Record<DecisionType, number>,
      byResult,
      averageScore: records.length > 0 ? totalScore / records.length : 0,
    }
  }

  /**
   * Convert stored format to DecisionRecord
   */
  function fromStored(stored: StoredDecisionRecord): DecisionRecord {
    return {
      id: stored.id,
      sessionId: stored.sessionId,
      type: stored.type,
      description: stored.description,
      context: stored.context,
      score: stored.score,
      result: stored.result,
      reasoning: stored.reasoning,
      timestamp: stored.timestamp,
      criteria: stored.criteria as DecisionRecord["criteria"],
    }
  }

  /**
   * Export decisions as markdown
   */
  export async function exportToMarkdown(sessionId?: string): Promise<string> {
    const records = sessionId ? await getBySession(sessionId) : await getRecent(50)

    const lines: string[] = []

    lines.push("# Decision History")
    lines.push("")
    lines.push(`Generated: ${new Date().toISOString()}`)
    lines.push(`Total decisions: ${records.length}`)
    lines.push("")

    for (const record of records) {
      lines.push(`## ${record.type.toUpperCase()}: ${record.description}`)
      lines.push("")
      lines.push(`**ID:** ${record.id}`)
      lines.push(`**Time:** ${new Date(record.timestamp).toISOString()}`)
      lines.push(`**Result:** ${record.result}`)
      lines.push("")
      lines.push("### CLOSE Scores")
      lines.push(`- Convergence: ${record.score.convergence.toFixed(1)}/10`)
      lines.push(`- Leverage: ${record.score.leverage.toFixed(1)}/10`)
      lines.push(`- Optionality: ${record.score.optionality.toFixed(1)}/10`)
      lines.push(`- Surplus: ${record.score.surplus.toFixed(1)}/10`)
      lines.push(`- Evolution: ${record.score.evolution.toFixed(1)}/10`)
      lines.push(`- **Total: ${record.score.total.toFixed(2)}/10**`)
      lines.push("")
      lines.push("### Reasoning")
      lines.push(record.reasoning)
      lines.push("")
      lines.push("---")
      lines.push("")
    }

    return lines.join("\n")
  }

  /**
   * Sync a Autonomous Mode decision to the main memory system
   * This bridges the Autonomous Mode decision system with the existing Decision memory
   */
  export async function syncToMemory(record: DecisionRecord): Promise<void> {
    try {
      // Map Autonomous Mode decision types to memory decision types
      const typeMap: Record<string, MemoryDecision.DecisionRecord["type"]> = {
        architecture: "architecture",
        implementation: "implementation",
        refactor: "refactor",
        bugfix: "bugfix",
        feature: "feature",
        test: "other",
        rollback: "other",
        checkpoint: "other",
        resource: "other",
        other: "other",
      }

      await MemoryDecision.create({
        type: typeMap[record.type] ?? "other",
        title: record.description.slice(0, 100),
        description: `${record.context}\n\nReasoning: ${record.reasoning}`,
        rationale: `CLOSE Score: ${record.score.total.toFixed(2)}/10\n` +
          `Convergence: ${record.score.convergence}, ` +
          `Leverage: ${record.score.leverage}, ` +
          `Optionality: ${record.score.optionality}, ` +
          `Surplus: ${record.score.surplus}, ` +
          `Evolution: ${record.score.evolution}`,
        outcome: record.result,
        sessionID: record.sessionId,
        tags: ["autonomous-mode", record.type, record.result],
      })

      log.info("Decision synced to memory", { id: record.id })
    } catch (error) {
      log.error("Failed to sync decision to memory", {
        id: record.id,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Import relevant decisions from memory for context
   */
  export async function importFromMemory(limit = 10): Promise<DecisionRecord[]> {
    try {
      const memoryDecisions = await MemoryDecision.getRecent(limit)

      const validTypes: DecisionType[] = [
        "architecture",
        "implementation",
        "refactor",
        "bugfix",
        "feature",
        "test",
        "rollback",
        "checkpoint",
        "resource",
        "other",
      ]

      return memoryDecisions
        .filter((d) => d.tags?.includes("autonomous-mode"))
        .map((md) => {
          const foundType = md.tags?.find((t) => validTypes.includes(t as DecisionType))
          const type = (foundType ?? "other") as DecisionType
          return {
            id: md.id,
            sessionId: md.sessionID ?? "imported",
            type,
            description: md.title,
            context: md.description.slice(0, 500),
            score: {
              convergence: 5,
              leverage: 5,
              optionality: 5,
              surplus: 5,
              evolution: 5,
              total: 5,
            },
            result: (md.outcome ?? "other") as DecisionRecord["result"],
            reasoning: md.rationale ?? md.description,
            timestamp: md.timestamp,
            criteria: {
              type,
              description: md.title,
              riskLevel: "medium",
              convergence: 5,
              leverage: 5,
              optionality: 5,
              surplus: 5,
              evolution: 5,
            },
          } satisfies DecisionRecord
        })
    } catch (error) {
      log.error("Failed to import decisions from memory", {
        error: error instanceof Error ? error.message : String(error),
      })
      return []
    }
  }

  /**
   * Create ADR from a Autonomous Mode decision
   */
  export async function createADR(record: DecisionRecord): Promise<void> {
    try {
      await MemoryDecision.createADR({
        title: record.description,
        status: record.result === "proceed" || record.result === "proceed_with_caution"
          ? "accepted"
          : record.result === "block"
            ? "rejected"
            : "proposed",
        context: record.context,
        decision: record.reasoning,
        consequences: [
          `CLOSE Score: ${record.score.total.toFixed(2)}/10`,
          `Convergence: ${record.score.convergence}/10`,
          `Leverage: ${record.score.leverage}/10`,
          `Optionality: ${record.score.optionality}/10`,
          `Surplus: ${record.score.surplus}/10`,
          `Evolution: ${record.score.evolution}/10`,
        ],
        tags: ["autonomous-mode", record.type, ...Object.keys(record.criteria)],
      })

      log.info("ADR created from decision", { id: record.id })
    } catch (error) {
      log.error("Failed to create ADR", {
        id: record.id,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }
}
