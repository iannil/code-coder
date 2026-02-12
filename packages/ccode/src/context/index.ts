// Context module - exports available as namespaces: Fingerprint, Loader, Cache, Watcher, Relevance
import * as FingerprintNS from "./fingerprint"
import * as LoaderNS from "./loader"
import * as CacheNS from "./cache"
import * as WatcherNS from "./watcher"
import * as RelevanceNS from "./relevance"

export const { Fingerprint } = FingerprintNS
export const { Loader } = LoaderNS
export const { Cache } = CacheNS
export const { Watcher } = WatcherNS
export const { Relevance } = RelevanceNS

// Types available as: WatcherNS.Watcher.FileChange, RelevanceNS.Relevance.ContextRequest, etc.


export async function initialize(options?: { watch?: boolean }): Promise<void> {
  await Fingerprint.load()
  await Loader.load()
  await Cache.load()

  if (options?.watch) {
    await Watcher.start()
  }
}

export async function invalidate(): Promise<void> {
  await Fingerprint.invalidate()
  await Loader.invalidate()
  await Cache.invalidate()
}

export async function getContextForTask(task: string, filePaths?: string[], maxTokens?: number) {
  const { Relevance } = await import("./relevance")
  return Relevance.getRelevantContext({
    task,
    filePaths,
    maxTokens,
    includeTests: true,
    includeConfigs: true,
    includeDependencies: true,
  })
}
