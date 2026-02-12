export { Decision } from "./decisions"
export { EditHistory } from "./edits"

// Note: Types are available as Decision.ArchitectureDecisionRecord, Decision.DecisionRecord, EditHistory.EditRecord, etc.

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"

const log = Log.create({ service: "memory.history" })

export namespace History {
  export const Summary = z.object({
    projectID: z.string(),
    totalDecisions: z.number(),
    totalEdits: z.number(),
    totalSessions: z.number(),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Summary = z.infer<typeof Summary>

  export async function getSummary(): Promise<Summary> {
    const projectID = Instance.project.id
    const now = Date.now()

    try {
      const stored = await Storage.read<Summary>(["memory", "history", "summary", projectID])
      return stored
    } catch {
      return {
        projectID,
        totalDecisions: 0,
        totalEdits: 0,
        totalSessions: 0,
        time: {
          created: now,
          updated: now,
        },
      }
    }
  }

  export async function updateSummary(updates: Partial<Summary>): Promise<void> {
    const summary = await getSummary()
    Object.assign(summary, updates, { time: { ...summary.time, updated: Date.now() } })
    const projectID = Instance.project.id
    await Storage.write(["memory", "history", "summary", projectID], summary)
  }

  export async function invalidate(): Promise<void> {
    const { Decision } = await import("./decisions")
    const { EditHistory } = await import("./edits")
    const projectID = Instance.project.id

    await Storage.remove(["memory", "history", "summary", projectID])
    await Decision.invalidate()
    await EditHistory.invalidate()
  }
}
