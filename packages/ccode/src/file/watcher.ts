import { BusEvent } from "@/bus/bus-event"
import { Bus } from "@/bus"
import z from "zod"
import { Instance } from "../project/instance"
import { Log } from "@/util/log"
import { FileIgnore } from "./ignore"
import { Config } from "../config/config"
import path from "path"
import { $ } from "bun"
import { Flag } from "@/flag/flag"
import { readdir } from "fs/promises"

// Native binding imports - NAPI only, no fallback
import {
  createFileWatcherWithConfig,
  type FileWatcherHandleType,
  type FileWatcherConfig,
} from "@codecoder-ai/core"

// Verify native bindings are available at import time
if (typeof createFileWatcherWithConfig !== "function") {
  throw new Error(
    "@codecoder-ai/core native bindings required: FileWatcher not available. " +
    "Run: cd services/zero-core && cargo build --features napi-bindings"
  )
}

// Non-null reference after verification
const createWatcher = createFileWatcherWithConfig

export namespace FileWatcher {
  const log = Log.create({ service: "file.watcher" })

  export const Event = {
    Updated: BusEvent.define(
      "file.watcher.updated",
      z.object({
        file: z.string(),
        event: z.union([z.literal("add"), z.literal("change"), z.literal("unlink")]),
      }),
    ),
  }

  // Watcher state - native handles only
  interface WatcherState {
    handles: FileWatcherHandleType[]
  }

  // Empty state for non-git projects
  interface EmptyState {
    empty: true
  }

  type State = WatcherState | EmptyState

  const state = Instance.state(
    async (): Promise<State> => {
      if (Instance.project.vcs !== "git") return { empty: true }

      log.info("init", { native: true })

      const cfg = await Config.get()
      const cfgIgnores = cfg.watcher?.ignore ?? []

      return initNativeWatcher(cfgIgnores)
    },
    async (state) => {
      // Cleanup native handles
      if ("handles" in state && state.handles) {
        for (const handle of state.handles) {
          try {
            handle.unsubscribeAll()
          } catch {
            // Ignore cleanup errors
          }
        }
      }
    },
  )

  async function initNativeWatcher(cfgIgnores: string[]): Promise<WatcherState> {
    const handles: FileWatcherHandleType[] = []

    const config: FileWatcherConfig = {
      debounceMs: 100,
      recursive: true,
      ignore: [...FileIgnore.PATTERNS, ...cfgIgnores],
    }

    const callback = (filePath: string, event: string) => {
      const eventType = event === "add" ? "add" : event === "unlink" ? "unlink" : "change"
      Bus.publish(Event.Updated, { file: filePath, event: eventType })
    }

    // Watch project directory
    if (Flag.CCODE_EXPERIMENTAL_FILEWATCHER) {
      try {
        const handle = createWatcher(config)
        handle.subscribe(Instance.directory, callback)
        handles.push(handle)
        log.info("native watcher subscribed", { path: Instance.directory })
      } catch (err) {
        log.error("failed to subscribe native watcher to Instance.directory", { error: err })
      }
    }

    // Watch .git directory for HEAD changes
    const vcsDir = await $`git rev-parse --git-dir`
      .quiet()
      .nothrow()
      .cwd(Instance.worktree)
      .text()
      .then((x) => path.resolve(Instance.worktree, x.trim()))
      .catch(() => undefined)

    if (vcsDir && !cfgIgnores.includes(".git") && !cfgIgnores.includes(vcsDir)) {
      try {
        const gitDirContents = await readdir(vcsDir).catch(() => [])
        const ignoreList = gitDirContents.filter((entry) => entry !== "HEAD")

        const gitConfig: FileWatcherConfig = {
          debounceMs: 100,
          recursive: false, // Only watch top-level .git files
          ignore: ignoreList,
        }

        const handle = createWatcher(gitConfig)
        handle.subscribe(vcsDir, callback)
        handles.push(handle)
        log.info("native watcher subscribed to vcs", { path: vcsDir })
      } catch (err) {
        log.error("failed to subscribe native watcher to vcsDir", { error: err })
      }
    }

    return { handles }
  }

  export function init() {
    if (Flag.CCODE_EXPERIMENTAL_DISABLE_FILEWATCHER) {
      return
    }
    state()
  }

  /** Check if using native watcher (always true now) */
  export function isNative(): boolean {
    return true
  }
}
