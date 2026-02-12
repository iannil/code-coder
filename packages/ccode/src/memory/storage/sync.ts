import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import z from "zod"

const log = Log.create({ service: "memory.storage.sync" })

export namespace Sync {
  export const SyncConfig = z.object({
    enabled: z.boolean(),
    endpoint: z.string().optional(),
    apiKey: z.string().optional(),
    interval: z.number().int().positive().optional(),
    lastSync: z.number().optional(),
  })
  export type SyncConfig = z.infer<typeof SyncConfig>

  export const SyncStatus = z.object({
    inProgress: z.boolean(),
    lastSync: z.number().optional(),
    lastError: z.string().optional(),
    pendingUploads: z.number(),
    pendingDownloads: z.number(),
  })
  export type SyncStatus = z.infer<typeof SyncStatus>

  const DEFAULT_CONFIG: SyncConfig = {
    enabled: false,
    interval: 5 * 60 * 1000, // 5 minutes
  }

  let syncTimer: ReturnType<typeof setTimeout> | undefined
  let currentStatus: SyncStatus = {
    inProgress: false,
    pendingUploads: 0,
    pendingDownloads: 0,
  }

  export async function getConfig(): Promise<SyncConfig> {
    const projectID = Instance.project.id
    try {
      const stored = await Storage.read<SyncConfig>(["memory", "sync", "config", projectID])
      return { ...DEFAULT_CONFIG, ...stored }
    } catch {
      return DEFAULT_CONFIG
    }
  }

  export async function updateConfig(updates: Partial<SyncConfig>): Promise<SyncConfig> {
    const config = await getConfig()
    const updated = { ...config, ...updates }
    const projectID = Instance.project.id
    await Storage.write(["memory", "sync", "config", projectID], updated)

    if (updated.enabled && !syncTimer) {
      startSyncTimer(updated.interval)
    } else if (!updated.enabled && syncTimer) {
      stopSyncTimer()
    }

    return updated
  }

  export async function getStatus(): Promise<SyncStatus> {
    return { ...currentStatus }
  }

  export async function syncNow(): Promise<boolean> {
    const config = await getConfig()
    if (!config.enabled || !config.endpoint) {
      log.warn("sync not configured")
      return false
    }

    if (currentStatus.inProgress) {
      log.warn("sync already in progress")
      return false
    }

    currentStatus.inProgress = true

    try {
      const result = await performSync(config)
      currentStatus.lastSync = Date.now()
      currentStatus.pendingUploads = 0
      currentStatus.pendingDownloads = 0
      currentStatus.lastError = undefined

      await updateConfig({ lastSync: currentStatus.lastSync })

      log.info("sync completed", { result })
      return true
    } catch (error) {
      currentStatus.lastError = error instanceof Error ? error.message : String(error)
      log.error("sync failed", { error })
      return false
    } finally {
      currentStatus.inProgress = false
    }
  }

  async function performSync(config: SyncConfig): Promise<{
    uploaded: number
    downloaded: number
    conflicts: number
  }> {
    const projectID = Instance.project.id
    const lastSync = config.lastSync || 0

    const localChanges = await getLocalChangesSince(lastSync)
    const remoteChanges = config.endpoint ? await getRemoteChangesSince(config, lastSync) : []

    let uploaded = 0
    let downloaded = 0
    let conflicts = 0

    for (const change of localChanges) {
      try {
        await uploadChange(config, change)
        uploaded++
      } catch (error) {
        log.warn("failed to upload change", { change, error })
      }
    }

    for (const change of remoteChanges) {
      try {
        const localExists = await Storage.read(change.key)
        if (localExists) {
          if (change.timestamp > lastSync) {
            await applyRemoteChange(change)
            downloaded++
          } else {
            conflicts++
          }
        } else {
          await applyRemoteChange(change)
          downloaded++
        }
      } catch (error) {
        log.warn("failed to apply remote change", { change, error })
      }
    }

    return { uploaded, downloaded, conflicts }
  }

  async function getLocalChangesSince(timestamp: number): Promise<
    Array<{
      key: string[]
      timestamp: number
    }>
  > {
    const changes: Array<{ key: string[]; timestamp: number }> = []

    const memoryNamespaces = ["memory/preferences", "memory/style", "memory/knowledge", "memory/history"]

    for (const ns of memoryNamespaces) {
      try {
        const keys = await Storage.list([ns])
        for (const key of keys) {
          try {
            const data = await Storage.read<any>(key)
            if (data.time?.updated && data.time.updated > timestamp) {
              changes.push({ key, timestamp: data.time.updated })
            }
          } catch {}
        }
      } catch {}
    }

    return changes
  }

  async function getRemoteChangesSince(
    config: SyncConfig,
    timestamp: number,
  ): Promise<Array<{ key: string[]; timestamp: number; value: any }>> {
    if (!config.endpoint) return []

    try {
      const url = `${config.endpoint}/api/sync/changes?since=${timestamp}`
      const response = await fetch(url, {
        headers: {
          Authorization: `Bearer ${config.apiKey}`,
          "Content-Type": "application/json",
        },
      })

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`)
      }

      return await response.json()
    } catch (error) {
      log.warn("failed to fetch remote changes", { error })
      return []
    }
  }

  async function uploadChange(config: SyncConfig, change: { key: string[]; timestamp: number }): Promise<void> {
    if (!config.endpoint) return

    const value = await Storage.read<any>(change.key)

    const url = `${config.endpoint}/api/sync/upload`
    const response = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${config.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        key: change.key,
        value,
        timestamp: change.timestamp,
      }),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }
  }

  async function applyRemoteChange(change: { key: string[]; timestamp: number; value: any }): Promise<void> {
    await Storage.write(change.key, change.value)
  }

  function startSyncTimer(interval: number = 5 * 60 * 1000): void {
    stopSyncTimer()

    syncTimer = setTimeout(async () => {
      await syncNow()
      startSyncTimer(interval)
    }, interval)
  }

  function stopSyncTimer(): void {
    if (syncTimer) {
      clearTimeout(syncTimer)
      syncTimer = undefined
    }
  }

  export async function exportMemory(): Promise<{
    projectID: string
    timestamp: number
    data: Record<string, any>
  }> {
    const projectID = Instance.project.id
    const timestamp = Date.now()

    const data: Record<string, any> = {}

    const namespaces = [
      ["memory", "preferences", projectID],
      ["memory", "style", projectID],
      ["memory", "knowledge", projectID],
      ["memory", "knowledge", "code-index", projectID],
      ["memory", "knowledge", "semantic-graph", projectID],
      ["memory", "knowledge", "patterns", projectID],
    ]

    for (const ns of namespaces) {
      try {
        const value = await Storage.read<any>(ns)
        const key = ns.join("/")
        data[key] = value
      } catch {}
    }

    return { projectID, timestamp, data }
  }

  export async function importMemory(
    importedData: { projectID: string; timestamp: number; data: Record<string, any> },
    options: { merge?: boolean; overwrite?: boolean } = {},
  ): Promise<{ imported: number; skipped: number; conflicts: number }> {
    let imported = 0
    let skipped = 0
    let conflicts = 0

    for (const [key, value] of Object.entries(importedData.data)) {
      const keyParts = key.split("/")

      try {
        const existing = await Storage.read<any>(keyParts)

        if (existing) {
          if (options.overwrite) {
            await Storage.write(keyParts, value)
            imported++
          } else if (options.merge && typeof existing === "object" && typeof value === "object") {
            const merged = { ...existing, ...value }
            await Storage.write(keyParts, merged)
            imported++
          } else {
            conflicts++
          }
        } else {
          await Storage.write(keyParts, value)
          imported++
        }
      } catch {
        await Storage.write(keyParts, value)
        imported++
      }
    }

    return { imported, skipped, conflicts }
  }

  export async function resetSync(): Promise<void> {
    stopSyncTimer()

    const projectID = Instance.project.id
    await Storage.remove(["memory", "sync", "config", projectID])
    await Storage.remove(["memory", "sync", "status", projectID])

    currentStatus = {
      inProgress: false,
      pendingUploads: 0,
      pendingDownloads: 0,
    }
  }

  export function stop(): void {
    stopSyncTimer()
  }
}
