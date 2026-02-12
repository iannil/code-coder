import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { History } from "./index"
import path from "path"
import z from "zod"

const log = Log.create({ service: "memory.history.edits" })

export namespace EditHistory {
  export const FileEdit = z.object({
    path: z.string(),
    type: z.enum(["create", "update", "delete", "move"]),
    additions: z.number(),
    deletions: z.number(),
    preHash: z.string().optional(),
    postHash: z.string().optional(),
  })
  export type FileEdit = z.infer<typeof FileEdit>

  export const EditRecord = z.object({
    id: z.string(),
    sessionID: z.string().optional(),
    timestamp: z.number(),
    description: z.string().optional(),
    edits: z.array(FileEdit),
    agent: z.string().optional(),
    model: z.string().optional(),
    tokensUsed: z.number().optional(),
    duration: z.number().optional(),
  })
  export type EditRecord = z.infer<typeof EditRecord>

  export const EditSession = z.object({
    id: z.string(),
    projectID: z.string(),
    startTime: z.number(),
    endTime: z.number().optional(),
    edits: z.array(z.string()),
    totalTokens: z.number(),
    totalDuration: z.number(),
    description: z.string().optional(),
  })
  export type EditSession = z.infer<typeof EditSession>

  export async function createRecord(input: Omit<EditRecord, "id" | "timestamp">): Promise<EditRecord> {
    const projectID = Instance.project.id
    const now = Date.now()

    const record: EditRecord = {
      id: `edit_${now}_${Math.random().toString(36).slice(2, 9)}`,
      ...input,
      timestamp: now,
    }

    await Storage.write(["memory", "history", "edits", projectID, record.id], record)

    const summary = await History.getSummary()
    await History.updateSummary({
      totalEdits: summary.totalEdits + 1,
    })

    if (input.sessionID) {
      await updateSession(input.sessionID, record.id, input.tokensUsed, input.duration)
    }

    return record
  }

  export async function getRecord(id: string): Promise<EditRecord | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<EditRecord>(["memory", "history", "edits", projectID, id])
    } catch {
      return undefined
    }
  }

  export async function getRecordsBySession(sessionID: string): Promise<EditRecord[]> {
    const projectID = Instance.project.id
    try {
      const keys = await Storage.list(["memory", "history", "edits", projectID])
      const records: EditRecord[] = []
      for (const key of keys) {
        const record = await Storage.read<EditRecord>(key)
        if (record.sessionID === sessionID) {
          records.push(record)
        }
      }
      return records.sort((a, b) => a.timestamp - b.timestamp)
    } catch {
      return []
    }
  }

  export async function getRecordsByFile(filePath: string): Promise<EditRecord[]> {
    const projectID = Instance.project.id
    const relativePath = path.relative(Instance.worktree, filePath).replace(/\\/g, "/")

    try {
      const keys = await Storage.list(["memory", "history", "edits", projectID])
      const records: EditRecord[] = []
      for (const key of keys) {
        const record = await Storage.read<EditRecord>(key)
        if (record.edits.some((e) => e.path === relativePath)) {
          records.push(record)
        }
      }
      return records.sort((a, b) => b.timestamp - a.timestamp)
    } catch {
      return []
    }
  }

  export async function getRecentRecords(limit = 20): Promise<EditRecord[]> {
    const projectID = Instance.project.id
    try {
      const keys = await Storage.list(["memory", "history", "edits", projectID])
      const records: EditRecord[] = []
      for (const key of keys) {
        const record = await Storage.read<EditRecord>(key)
        records.push(record)
      }
      return records.sort((a, b) => b.timestamp - a.timestamp).slice(0, limit)
    } catch {
      return []
    }
  }

  export async function startSession(description?: string): Promise<EditSession> {
    const projectID = Instance.project.id
    const now = Date.now()

    const session: EditSession = {
      id: `session_${now}_${Math.random().toString(36).slice(2, 9)}`,
      projectID,
      startTime: now,
      edits: [],
      totalTokens: 0,
      totalDuration: 0,
      description,
    }

    await Storage.write(["memory", "history", "sessions", projectID, session.id], session)

    const summary = await History.getSummary()
    await History.updateSummary({
      totalSessions: summary.totalSessions + 1,
    })

    return session
  }

  export async function getSession(id: string): Promise<EditSession | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<EditSession>(["memory", "history", "sessions", projectID, id])
    } catch {
      return undefined
    }
  }

  export async function updateSession(
    id: string,
    editId: string,
    tokens?: number,
    duration?: number,
  ): Promise<EditSession | undefined> {
    const session = await getSession(id)
    if (!session) return undefined

    session.edits.push(editId)
    if (tokens) session.totalTokens += tokens
    if (duration) session.totalDuration += duration

    const projectID = Instance.project.id
    await Storage.write(["memory", "history", "sessions", projectID, id], session)

    return session
  }

  export async function endSession(id: string): Promise<EditSession | undefined> {
    const session = await getSession(id)
    if (!session) return undefined

    session.endTime = Date.now()

    const projectID = Instance.project.id
    await Storage.write(["memory", "history", "sessions", projectID, id], session)

    return session
  }

  export async function getAllSessions(): Promise<EditSession[]> {
    const projectID = Instance.project.id
    try {
      const keys = await Storage.list(["memory", "history", "sessions", projectID])
      const sessions: EditSession[] = []
      for (const key of keys) {
        const session = await Storage.read<EditSession>(key)
        sessions.push(session)
      }
      return sessions.sort((a, b) => b.startTime - a.startTime)
    } catch {
      return []
    }
  }

  export async function getActiveSessions(): Promise<EditSession[]> {
    const all = await getAllSessions()
    return all.filter((s) => !s.endTime)
  }

  export async function getFileHistory(
    filePath: string,
    limit = 10,
  ): Promise<{
    file: string
    edits: Array<{
      timestamp: number
      description?: string
      additions: number
      deletions: number
      agent?: string
    }>
  }> {
    const records = await getRecordsByFile(filePath)
    const relativePath = path.relative(Instance.worktree, filePath).replace(/\\/g, "/")

    const edits = records
      .filter((r) => r.edits.some((e) => e.path === relativePath))
      .slice(0, limit)
      .map((r) => {
        const fileEdit = r.edits.find((e) => e.path === relativePath)
        return {
          timestamp: r.timestamp,
          description: r.description,
          additions: fileEdit?.additions || 0,
          deletions: fileEdit?.deletions || 0,
          agent: r.agent,
        }
      })

    return {
      file: relativePath,
      edits,
    }
  }

  export async function getStats(): Promise<{
    totalEdits: number
    totalAdditions: number
    totalDeletions: number
    totalFiles: number
    topFiles: Array<{ path: string; editCount: number }>
    agentStats: Array<{ agent: string; editCount: number; tokenCount: number }>
  }> {
    const projectID = Instance.project.id
    const fileEditCounts = new Map<string, number>()
    const agentStats = new Map<string, { count: number; tokens: number }>()

    let totalAdditions = 0
    let totalDeletions = 0

    try {
      const keys = await Storage.list(["memory", "history", "edits", projectID])
      for (const key of keys) {
        const record = await Storage.read<EditRecord>(key)

        for (const edit of record.edits) {
          const count = fileEditCounts.get(edit.path) || 0
          fileEditCounts.set(edit.path, count + 1)

          totalAdditions += edit.additions
          totalDeletions += edit.deletions
        }

        if (record.agent) {
          const stats = agentStats.get(record.agent) || { count: 0, tokens: 0 }
          stats.count++
          stats.tokens += record.tokensUsed || 0
          agentStats.set(record.agent, stats)
        }
      }
    } catch {}

    const topFiles = Array.from(fileEditCounts.entries())
      .map(([path, count]) => ({ path, editCount: count }))
      .sort((a, b) => b.editCount - a.editCount)
      .slice(0, 10)

    const agentStatsArray = Array.from(agentStats.entries())
      .map(([agent, stats]) => ({ agent, editCount: stats.count, tokenCount: stats.tokens }))
      .sort((a, b) => b.editCount - a.editCount)

    return {
      totalEdits: fileEditCounts.size,
      totalAdditions,
      totalDeletions,
      totalFiles: fileEditCounts.size,
      topFiles,
      agentStats: agentStatsArray,
    }
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id

    const editKeys = await Storage.list(["memory", "history", "edits", projectID])
    for (const key of editKeys) {
      await Storage.remove(key)
    }

    const sessionKeys = await Storage.list(["memory", "history", "sessions", projectID])
    for (const key of sessionKeys) {
      await Storage.remove(key)
    }
  }

  export async function cleanup(beforeDate: number): Promise<number> {
    const projectID = Instance.project.id
    let removed = 0

    try {
      const keys = await Storage.list(["memory", "history", "edits", projectID])
      for (const key of keys) {
        const record = await Storage.read<EditRecord>(key)
        if (record.timestamp < beforeDate) {
          await Storage.remove(key)
          removed++
        }
      }

      const sessionKeys = await Storage.list(["memory", "history", "sessions", projectID])
      for (const key of sessionKeys) {
        const session = await Storage.read<EditSession>(key)
        if (session.startTime < beforeDate) {
          await Storage.remove(key)
          removed++
        }
      }
    } catch {}

    return removed
  }
}
