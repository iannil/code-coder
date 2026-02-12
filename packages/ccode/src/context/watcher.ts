import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Filesystem } from "@/util/filesystem"
import { BusEvent } from "@/bus/bus-event"
import { Bus } from "@/bus"
import { Fingerprint } from "./fingerprint"
import { Loader } from "./loader"
import { Cache } from "./cache"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.watcher" })

export namespace Watcher {
  export const FileChange = z.object({
    path: z.string(),
    type: z.enum(["created", "updated", "deleted"]),
    timestamp: z.number(),
  })
  export type FileChange = z.infer<typeof FileChange>

  export const WatchState = z.object({
    projectID: z.string(),
    isWatching: z.boolean(),
    watchedPaths: z.array(z.string()),
    changes: z.array(FileChange),
    lastUpdate: z.number(),
  })
  export type WatchState = z.infer<typeof WatchState>

  export const Event = {
    FileChanged: BusEvent.define(
      "context.file.changed",
      z.object({
        projectID: z.string(),
        change: FileChange,
      }),
    ),
    ContextUpdated: BusEvent.define(
      "context.updated",
      z.object({
        projectID: z.string(),
        timestamp: z.number(),
      }),
    ),
  }

  let watcherInstance: FileSystemWatcher | undefined
  let debounceTimer: ReturnType<typeof setTimeout> | undefined
  let pendingChanges: FileChange[] = []

  class FileSystemWatcher {
    private watchedPaths = new Set<string>()
    private watchers = new Map<string, ReturnType<typeof Bun.file>>()
    private pollInterval: ReturnType<typeof setInterval> | undefined
    private previousState = new Map<string, { mtime: number; size: number }>()
    public isRunning = false

    async watch(paths: string[]): Promise<void> {
      log.info("starting file watcher", { paths })

      for (const watchPath of paths) {
        if (this.watchedPaths.has(watchPath)) continue

        this.watchedPaths.add(watchPath)

        try {
          await this.initializeWatch(watchPath)
        } catch (error) {
          log.warn("failed to watch path", { path: watchPath, error })
        }
      }

      if (!this.isRunning) {
        this.isRunning = true
        this.startPolling()
      }
    }

    async unwatch(paths: string[]): Promise<void> {
      for (const watchPath of paths) {
        this.watchedPaths.delete(watchPath)
        this.previousState.delete(watchPath)
      }

      if (this.watchedPaths.size === 0) {
        this.stop()
      }
    }

    private async initializeWatch(watchPath: string): Promise<void> {
      try {
        const stat = await Bun.file(watchPath).stat()
        this.previousState.set(watchPath, {
          mtime: stat.mtime?.getTime() ?? Date.now(),
          size: stat.size,
        })
      } catch {}
    }

    private startPolling(): void {
      if (this.pollInterval) return

      this.pollInterval = setInterval(async () => {
        await this.checkChanges()
      }, 2000)
    }

    private stop(): void {
      if (this.pollInterval) {
        clearInterval(this.pollInterval)
        this.pollInterval = undefined
      }
      this.isRunning = false
    }

    private async checkChanges(): Promise<void> {
      const changes: FileChange[] = []

      for (const watchPath of this.watchedPaths) {
        try {
          const exists = await Filesystem.exists(watchPath)

          if (!exists) {
            const previous = this.previousState.get(watchPath)
            if (previous) {
              changes.push({
                path: watchPath,
                type: "deleted",
                timestamp: Date.now(),
              })
              this.previousState.delete(watchPath)
            }
            continue
          }

          const stat = await Bun.file(watchPath).stat()
          const current = {
            mtime: stat.mtime?.getTime() ?? Date.now(),
            size: stat.size,
          }

          const previous = this.previousState.get(watchPath)

          if (!previous) {
            changes.push({
              path: watchPath,
              type: "created",
              timestamp: Date.now(),
            })
          } else if (current.mtime > previous.mtime || current.size !== previous.size) {
            changes.push({
              path: watchPath,
              type: "updated",
              timestamp: Date.now(),
            })
          }

          this.previousState.set(watchPath, current)
        } catch (error) {
          log.warn("error checking file change", { path: watchPath, error })
        }
      }

      if (changes.length > 0) {
        this.handleChanges(changes)
      }
    }

    private handleChanges(changes: FileChange[]): void {
      log.debug("detected file changes", { count: changes.length })

      pendingChanges.push(...changes)

      if (debounceTimer) clearTimeout(debounceTimer)

      debounceTimer = setTimeout(() => {
        this.processPendingChanges()
      }, 1000)
    }

    private async processPendingChanges(): Promise<void> {
      if (pendingChanges.length === 0) return

      const changesToProcess = [...pendingChanges]
      pendingChanges = []

      log.info("processing file changes", { count: changesToProcess.length })

      for (const change of changesToProcess) {
        Bus.publish(Event.FileChanged, {
          projectID: Instance.project.id,
          change,
        })

        await this.updateContextForChange(change)
      }

      Bus.publish(Event.ContextUpdated, {
        projectID: Instance.project.id,
        timestamp: Date.now(),
      })
    }

    private async updateContextForChange(change: FileChange): Promise<void> {
      const relativePath = path.relative(Instance.worktree, change.path).replace(/\\/g, "/")

      if (change.type === "deleted") {
        await Cache.removeEntry(relativePath)
      } else if (change.type === "created" || change.type === "updated") {
        const stat = await Bun.file(change.path).stat()
        await Cache.updateEntry({
          path: relativePath,
          type: "file",
          lastModified: stat.mtime?.getTime() ?? Date.now(),
          size: stat.size,
        })
      }
    }

    dispose(): void {
      this.stop()
      this.watchedPaths.clear()
      this.previousState.clear()
      if (debounceTimer) clearTimeout(debounceTimer)
    }
  }

  export async function start(paths?: string[]): Promise<void> {
    if (watcherInstance) {
      log.warn("watcher already running")
      return
    }

    const watchPaths = paths ?? [Instance.worktree]

    watcherInstance = new FileSystemWatcher()
    await watcherInstance.watch(watchPaths)

    log.info("context watcher started", { paths: watchPaths })
  }

  export async function stop(): Promise<void> {
    if (!watcherInstance) return

    watcherInstance.dispose()
    watcherInstance = undefined

    log.info("context watcher stopped")
  }

  export async function isWatching(): Promise<boolean> {
    return watcherInstance?.isRunning ?? false
  }

  export async function addWatch(path: string): Promise<void> {
    if (!watcherInstance) {
      await start([path])
    } else {
      await watcherInstance.watch([path])
    }
  }

  export async function removeWatch(path: string): Promise<void> {
    if (!watcherInstance) return

    await watcherInstance.unwatch([path])
  }

  export async function getRecentChanges(since?: number): Promise<FileChange[]> {
    const state = await getState()
    if (!since) return state.changes

    return state.changes.filter((c) => c.timestamp >= since)
  }

  async function getState(): Promise<WatchState> {
    const projectID = Instance.project.id
    const { Storage } = await import("@/storage/storage")

    try {
      return await Storage.read<WatchState>(["context", "watcher", projectID])
    } catch {
      return {
        projectID,
        isWatching: false,
        watchedPaths: [],
        changes: [],
        lastUpdate: Date.now(),
      }
    }
  }
}
