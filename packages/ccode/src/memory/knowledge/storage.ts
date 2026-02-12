import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"

const log = Log.create({ service: "memory.knowledge.storage" })

export namespace KnowledgeStorage {
  export const StorageConfig = z.object({
    enabled: z.boolean(),
    location: z.enum(["local", "database", "hybrid"]),
    maxEntries: z.number().int().positive(),
    ttl: z.number().int().positive().optional(),
  })
  export type StorageConfig = z.infer<typeof StorageConfig>

  export const Entry = z.object({
    key: z.string(),
    value: z.any(),
    metadata: z.object({
      created: z.number(),
      updated: z.number(),
      accessed: z.number(),
      size: z.number(),
      tags: z.array(z.string()).optional(),
    }),
  })
  export type Entry = z.infer<typeof Entry>

  export const Stats = z.object({
    totalEntries: z.number(),
    totalSize: z.number(),
    byTag: z.record(z.string(), z.number()),
    lastCleanup: z.number().optional(),
  })
  export type Stats = z.infer<typeof Stats>

  const DEFAULT_CONFIG: StorageConfig = {
    enabled: true,
    location: "local",
    maxEntries: 10000,
    ttl: 30 * 24 * 60 * 60 * 1000, // 30 days
  }

  export async function getConfig(): Promise<StorageConfig> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<StorageConfig>(["memory", "knowledge", "storage-config", projectID])
      return { ...DEFAULT_CONFIG, ...stored }
    } catch {
      return DEFAULT_CONFIG
    }
  }

  export async function updateConfig(updates: Partial<StorageConfig>): Promise<StorageConfig> {
    const config = await getConfig()
    const updated = { ...config, ...updates }
    const projectID = Instance.project.id
    await Storage.write(["memory", "knowledge", "storage-config", projectID], updated)
    return updated
  }

  export async function set(key: string, value: any, tags?: string[]): Promise<void> {
    const config = await getConfig()
    if (!config.enabled) return

    const projectID = Instance.project.id
    const now = Date.now()

    const entry: Entry = {
      key,
      value,
      metadata: {
        created: now,
        updated: now,
        accessed: now,
        size: JSON.stringify(value).length,
        tags,
      },
    }

    await Storage.write(["memory", "knowledge", "entries", projectID, key], entry)
  }

  export async function get(key: string): Promise<any | undefined> {
    const config = await getConfig()
    if (!config.enabled) return undefined

    const projectID = Instance.project.id

    try {
      const entry = await Storage.read<Entry>(["memory", "knowledge", "entries", projectID, key])

      if (config.ttl) {
        const age = Date.now() - entry.metadata.accessed
        if (age > config.ttl) {
          await remove(key)
          return undefined
        }
      }

      entry.metadata.accessed = Date.now()
      await Storage.write(["memory", "knowledge", "entries", projectID, key], entry)

      return entry.value
    } catch {
      return undefined
    }
  }

  export async function remove(key: string): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "entries", projectID, key])
  }

  export async function clear(): Promise<void> {
    const config = await getConfig()
    if (!config.enabled) return

    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "knowledge", "entries", projectID])

    for (const key of keys) {
      await Storage.remove(key)
    }
  }

  export async function list(tag?: string): Promise<string[]> {
    const config = await getConfig()
    if (!config.enabled) return []

    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "knowledge", "entries", projectID])

    if (!tag) return keys.map((k) => k[k.length - 1])

    const matching: string[] = []
    for (const key of keys) {
      try {
        const entry = await Storage.read<Entry>(key)
        if (entry.metadata.tags?.includes(tag)) {
          matching.push(entry.key)
        }
      } catch {}
    }

    return matching
  }

  export async function getStats(): Promise<Stats> {
    const config = await getConfig()
    if (!config.enabled) {
      return {
        totalEntries: 0,
        totalSize: 0,
        byTag: {},
      }
    }

    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "knowledge", "entries", projectID])

    let totalEntries = 0
    let totalSize = 0
    const byTag: Record<string, number> = {}

    for (const key of keys) {
      try {
        const entry = await Storage.read<Entry>(key)
        totalEntries++
        totalSize += entry.metadata.size

        if (entry.metadata.tags) {
          for (const tag of entry.metadata.tags) {
            byTag[tag] = (byTag[tag] || 0) + 1
          }
        }
      } catch {}
    }

    return {
      totalEntries,
      totalSize,
      byTag,
    }
  }

  export async function cleanup(): Promise<number> {
    const config = await getConfig()
    if (!config.enabled) return 0

    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "knowledge", "entries", projectID])

    let removed = 0
    const now = Date.now()

    for (const key of keys) {
      try {
        const entry = await Storage.read<Entry>(key)

        if (config.ttl && now - entry.metadata.accessed > config.ttl) {
          await Storage.remove(key)
          removed++
        }
      } catch {
        await Storage.remove(key)
        removed++
      }
    }

    const maxEntries = config.maxEntries
    if (removed === 0 && maxEntries) {
      const allKeys = await Storage.list(["memory", "knowledge", "entries", projectID])
      if (allKeys.length > maxEntries) {
        const entries: Array<{ key: string; accessed: number }> = []

        for (const key of allKeys) {
          try {
            const entry = await Storage.read<Entry>(key)
            entries.push({ key: entry.key, accessed: entry.metadata.accessed })
          } catch {}
        }

        entries.sort((a, b) => a.accessed - b.accessed)

        const toRemove = entries.slice(0, entries.length - maxEntries)
        for (const entry of toRemove) {
          await Storage.remove(["memory", "knowledge", "entries", projectID, entry.key])
          removed++
        }
      }
    }

    if (removed > 0) {
      const stats = await getStats()
      log.info("cleanup completed", { removed, remaining: stats.totalEntries })
    }

    return removed
  }

  export async function exportData(): Promise<Record<string, any>> {
    const config = await getConfig()
    if (!config.enabled) return {}

    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "knowledge", "entries", projectID])

    const result: Record<string, any> = {}

    for (const key of keys) {
      try {
        const entry = await Storage.read<Entry>(key)
        result[entry.key] = entry.value
      } catch {}
    }

    return result
  }

  export async function importData(data: Record<string, any>): Promise<number> {
    const config = await getConfig()
    if (!config.enabled) return 0

    let imported = 0

    for (const [key, value] of Object.entries(data)) {
      try {
        await set(key, value)
        imported++
      } catch (error) {
        log.warn("failed to import key", { key, error })
      }
    }

    return imported
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "knowledge", "storage-config", projectID])
  }
}
