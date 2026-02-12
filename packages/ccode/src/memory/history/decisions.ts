import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { History } from "./index"
import z from "zod"

const log = Log.create({ service: "memory.history.decisions" })

export namespace Decision {
  export const ArchitectureDecisionRecord = z.object({
    id: z.string(),
    title: z.string(),
    status: z.enum(["proposed", "accepted", "deprecated", "superseded", "rejected"]),
    context: z.string(),
    decision: z.string(),
    consequences: z.array(z.string()),
    alternatives: z
      .array(
        z.object({
          description: z.string(),
          rejected: z.boolean(),
          reason: z.string().optional(),
        }),
      )
      .optional(),
    supersededBy: z.string().optional(),
    created: z.number(),
    updated: z.number(),
    tags: z.array(z.string()).optional(),
  })
  export type ArchitectureDecisionRecord = z.infer<typeof ArchitectureDecisionRecord>

  export const DecisionRecord = z.object({
    id: z.string(),
    type: z.enum(["architecture", "implementation", "refactor", "bugfix", "feature", "other"]),
    title: z.string(),
    description: z.string(),
    rationale: z.string().optional(),
    alternatives: z.array(z.string()).optional(),
    outcome: z.string().optional(),
    sessionID: z.string().optional(),
    files: z.array(z.string()).optional(),
    tags: z.array(z.string()).optional(),
    timestamp: z.number(),
  })
  export type DecisionRecord = z.infer<typeof DecisionRecord>

  export async function getAll(): Promise<DecisionRecord[]> {
    const projectID = Instance.project.id
    try {
      const keys = await Storage.list(["memory", "history", "decisions", projectID])
      const decisions: DecisionRecord[] = []
      for (const key of keys) {
        const decision = await Storage.read<DecisionRecord>(key)
        decisions.push(decision)
      }
      return decisions.sort((a, b) => b.timestamp - a.timestamp)
    } catch {
      return []
    }
  }

  export async function get(id: string): Promise<DecisionRecord | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<DecisionRecord>(["memory", "history", "decisions", projectID, id])
    } catch {
      return undefined
    }
  }

  export async function create(input: Omit<DecisionRecord, "id" | "timestamp">): Promise<DecisionRecord> {
    const projectID = Instance.project.id
    const now = Date.now()

    const record: DecisionRecord = {
      id: `decision_${now}_${Math.random().toString(36).slice(2, 9)}`,
      ...input,
      timestamp: now,
    }

    await Storage.write(["memory", "history", "decisions", projectID, record.id], record)

    const summary = await History.getSummary()
    await History.updateSummary({
      totalDecisions: summary.totalDecisions + 1,
    })

    return record
  }

  export async function update(
    id: string,
    updates: Partial<Omit<DecisionRecord, "id" | "timestamp">>,
  ): Promise<DecisionRecord | undefined> {
    const existing = await get(id)
    if (!existing) return undefined

    const updated: DecisionRecord = {
      ...existing,
      ...updates,
    }

    const projectID = Instance.project.id
    await Storage.write(["memory", "history", "decisions", projectID, id], updated)

    return updated
  }

  export async function remove(id: string): Promise<boolean> {
    const projectID = Instance.project.id
    try {
      await Storage.remove(["memory", "history", "decisions", projectID, id])

      const summary = await History.getSummary()
      await History.updateSummary({
        totalDecisions: Math.max(0, summary.totalDecisions - 1),
      })

      return true
    } catch {
      return false
    }
  }

  export async function findByType(type: DecisionRecord["type"]): Promise<DecisionRecord[]> {
    const all = await getAll()
    return all.filter((d) => d.type === type)
  }

  export async function findBySession(sessionID: string): Promise<DecisionRecord[]> {
    const all = await getAll()
    return all.filter((d) => d.sessionID === sessionID)
  }

  export async function findByFile(filePath: string): Promise<DecisionRecord[]> {
    const all = await getAll()
    return all.filter((d) => d.files?.includes(filePath))
  }

  export async function search(query: string): Promise<DecisionRecord[]> {
    const all = await getAll()
    const lowerQuery = query.toLowerCase()

    return all.filter(
      (d) =>
        d.title.toLowerCase().includes(lowerQuery) ||
        d.description.toLowerCase().includes(lowerQuery) ||
        d.rationale?.toLowerCase().includes(lowerQuery) ||
        d.tags?.some((t: string) => t.toLowerCase().includes(lowerQuery)),
    )
  }

  export async function getRecent(limit = 10): Promise<DecisionRecord[]> {
    const all = await getAll()
    return all.slice(0, limit)
  }

  export async function createADR(
    input: Omit<ArchitectureDecisionRecord, "id" | "created" | "updated">,
  ): Promise<ArchitectureDecisionRecord> {
    const projectID = Instance.project.id
    const now = Date.now()

    const record: ArchitectureDecisionRecord = {
      id: `adr_${now}_${Math.random().toString(36).slice(2, 9)}`,
      ...input,
      created: now,
      updated: now,
    }

    await Storage.write(["memory", "history", "adr", projectID, record.id], record)

    const decisionRecord: DecisionRecord = {
      id: record.id,
      type: "architecture",
      title: record.title,
      description: `${record.context}\n\nDecision: ${record.decision}\n\nConsequences:\n${record.consequences.join("\n")}`,
      rationale: record.decision,
      alternatives: record.alternatives?.map((a) => a.description),
      outcome: record.status,
      timestamp: now,
    }

    await create(decisionRecord)

    return record
  }

  export async function getADR(id: string): Promise<ArchitectureDecisionRecord | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<ArchitectureDecisionRecord>(["memory", "history", "adr", projectID, id])
    } catch {
      return undefined
    }
  }

  export async function getAllADRs(): Promise<ArchitectureDecisionRecord[]> {
    const projectID = Instance.project.id
    try {
      const keys = await Storage.list(["memory", "history", "adr", projectID])
      const adrs: ArchitectureDecisionRecord[] = []
      for (const key of keys) {
        const adr = await Storage.read<ArchitectureDecisionRecord>(key)
        adrs.push(adr)
      }
      return adrs.sort((a, b) => b.created - a.created)
    } catch {
      return []
    }
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id

    const decisionKeys = await Storage.list(["memory", "history", "decisions", projectID])
    for (const key of decisionKeys) {
      await Storage.remove(key)
    }

    const adrKeys = await Storage.list(["memory", "history", "adr", projectID])
    for (const key of adrKeys) {
      await Storage.remove(key)
    }
  }

  export function formatADR(adr: ArchitectureDecisionRecord): string {
    const lines: string[] = []

    lines.push(`# ${adr.title}`)
    lines.push("")
    lines.push(`**Status:** ${adr.status}`)
    lines.push(`**Date:** ${new Date(adr.created).toISOString()}`)
    if (adr.tags && adr.tags.length > 0) {
      lines.push(`**Tags:** ${adr.tags.join(", ")}`)
    }
    lines.push("")

    lines.push("## Context")
    lines.push(adr.context)
    lines.push("")

    lines.push("## Decision")
    lines.push(adr.decision)
    lines.push("")

    if (adr.consequences.length > 0) {
      lines.push("## Consequences")
      for (const consequence of adr.consequences) {
        lines.push(`- ${consequence}`)
      }
      lines.push("")
    }

    if (adr.alternatives && adr.alternatives.length > 0) {
      lines.push("## Alternatives Considered")
      for (const alt of adr.alternatives) {
        lines.push(`- ${alt.description}${alt.rejected ? " (rejected)" : ""}`)
        if (alt.reason) {
          lines.push(`  - Reason: ${alt.reason}`)
        }
      }
      lines.push("")
    }

    return lines.join("\n")
  }
}
