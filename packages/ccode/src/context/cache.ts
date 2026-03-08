/**
 * Context Cache Module (Native-Only)
 *
 * Provides project structure caching using native Rust implementation.
 * Caches routes, components, configs, and test files for fast lookup.
 *
 * @package context
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/infrastructure/storage/storage"
import { Fingerprint } from "./fingerprint"
import path from "path"
import z from "zod"

const log = Log.create({ service: "context.cache" })

// ============================================================================
// Native Bindings
// ============================================================================

interface NapiProjectCache {
  projectId: string
  routes: NapiRouteCache[]
  components: NapiComponentCache[]
  configs: NapiCacheConfigEntry[]
  testFiles: string[]
  time: { created: number; updated: number }
}

interface NapiRouteCache {
  path: string
  type: string
  framework?: string
  methods?: string[]
  middleware?: string
}

interface NapiComponentCache {
  path: string
  name: string
  type: string
  props?: string[]
  imports?: string[]
}

interface NapiCacheConfigEntry {
  path: string
  name: string
  type: string
  content?: string
}

interface NativeCacheBindings {
  buildProjectCache: (worktree: string, projectId: string, framework?: string) => NapiProjectCache
}

let nativeBindings: NativeCacheBindings | null = null
let loadAttempted = false

/**
 * Load native cache bindings. Throws if unavailable.
 */
async function loadNativeBindings(): Promise<NativeCacheBindings> {
  if (loadAttempted && nativeBindings) return nativeBindings

  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>

    if (typeof bindings.buildProjectCache === "function") {
      nativeBindings = bindings as unknown as NativeCacheBindings
      log.debug("Loaded native cache bindings")
      loadAttempted = true
      return nativeBindings
    }
  } catch (e) {
    loadAttempted = true
    throw new Error(`Native bindings required: @codecoder-ai/core cache functions not available: ${e}`)
  }

  loadAttempted = true
  throw new Error("Native bindings required: @codecoder-ai/core cache functions not available")
}

// ============================================================================
// Types
// ============================================================================

export namespace Cache {
  export const CacheEntry = z.object({
    path: z.string(),
    type: z.enum(["file", "directory", "config", "route", "component", "test"]),
    lastModified: z.number(),
    size: z.number(),
    hash: z.string().optional(),
  })
  export type CacheEntry = z.infer<typeof CacheEntry>

  export const RouteCache = z.object({
    path: z.string(),
    type: z.enum(["file", "directory", "app", "pages", "api"]),
    framework: z.string().optional(),
    methods: z.array(z.string()).optional(),
    middleware: z.string().optional(),
  })
  export type RouteCache = z.infer<typeof RouteCache>

  export const ComponentCache = z.object({
    path: z.string(),
    name: z.string(),
    type: z.enum(["component", "hook", "util", "layout", "page"]),
    props: z.array(z.string()).optional(),
    imports: z.array(z.string()).optional(),
  })
  export type ComponentCache = z.infer<typeof ComponentCache>

  export const ConfigCache = z.object({
    path: z.string(),
    name: z.string(),
    type: z.string(),
    content: z.string().optional(),
    parsed: z.record(z.string(), z.any()).optional(),
  })
  export type ConfigCache = z.infer<typeof ConfigCache>

  export const Info = z.object({
    projectID: z.string(),
    routes: z.array(RouteCache),
    components: z.array(ComponentCache),
    configs: z.array(ConfigCache),
    testFiles: z.array(z.string()),
    entries: z.record(z.string(), CacheEntry),
    time: z.object({
      created: z.number(),
      updated: z.number(),
    }),
  })
  export type Info = z.infer<typeof Info>

  // ============================================================================
  // Native Cache Conversion
  // ============================================================================

  function convertNativeCache(native: NapiProjectCache): Info {
    const entries: Record<string, CacheEntry> = {}

    for (const route of native.routes) {
      entries[route.path] = {
        path: route.path,
        type: "route",
        lastModified: native.time.updated,
        size: 0,
      }
    }

    for (const component of native.components) {
      entries[component.path] = {
        path: component.path,
        type: "component",
        lastModified: native.time.updated,
        size: 0,
      }
    }

    for (const config of native.configs) {
      entries[config.path] = {
        path: config.path,
        type: "config",
        lastModified: native.time.updated,
        size: config.content?.length ?? 0,
      }
    }

    for (const test of native.testFiles) {
      entries[test] = {
        path: test,
        type: "test",
        lastModified: native.time.updated,
        size: 0,
      }
    }

    return {
      projectID: native.projectId,
      routes: native.routes.map((r) => ({
        path: r.path,
        type: r.type as RouteCache["type"],
        framework: r.framework,
        methods: r.methods,
        middleware: r.middleware,
      })),
      components: native.components.map((c) => ({
        path: c.path,
        name: c.name,
        type: c.type as ComponentCache["type"],
        props: c.props,
        imports: c.imports,
      })),
      configs: native.configs.map((c) => ({
        path: c.path,
        name: c.name,
        type: c.type,
        content: c.content,
      })),
      testFiles: native.testFiles,
      entries,
      time: {
        created: native.time.created,
        updated: native.time.updated,
      },
    }
  }

  // ============================================================================
  // Public API
  // ============================================================================

  /**
   * Build cache using native implementation.
   * @throws Error if native bindings unavailable
   */
  export async function build(): Promise<Info> {
    const projectID = Instance.project.id
    const worktree = Instance.worktree

    log.info("building cache (native)", { projectID })

    const fingerprint = await Fingerprint.load()
    const framework = fingerprint.frameworks[0]?.name

    const native = await loadNativeBindings()
    const nativeCache = native.buildProjectCache(worktree, projectID, framework)
    const result = convertNativeCache(nativeCache)

    log.info("cache built", {
      projectID,
      routesCount: result.routes.length,
      componentsCount: result.components.length,
      configsCount: result.configs.length,
      testFilesCount: result.testFiles.length,
    })

    return result
  }

  export async function get(): Promise<Info | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Info>(["context", "cache", projectID])
    } catch {
      return undefined
    }
  }

  export async function load(options?: { force?: boolean }): Promise<Info> {
    let existing = await get()

    if (!existing || options?.force) {
      existing = await build()
      await save(existing)
    }

    return existing
  }

  export async function save(cache: Info): Promise<void> {
    const projectID = Instance.project.id
    const updated = { ...cache, time: { ...cache.time, updated: Date.now() } }
    await Storage.write(["context", "cache", projectID], updated)
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["context", "cache", projectID])
  }

  export async function updateEntry(entry: CacheEntry): Promise<void> {
    const cache = await load()
    const updated = {
      ...cache,
      entries: { ...cache.entries, [entry.path]: entry },
      time: { ...cache.time, updated: Date.now() },
    }
    await save(updated)
  }

  export async function removeEntry(entryPath: string): Promise<void> {
    const cache = await get()
    if (!cache) return

    const { [entryPath]: _, ...remainingEntries } = cache.entries
    const updated = {
      ...cache,
      entries: remainingEntries,
      routes: cache.routes.filter((r) => r.path !== entryPath),
      components: cache.components.filter((c) => c.path !== entryPath),
      configs: cache.configs.filter((c) => c.path !== entryPath),
      testFiles: cache.testFiles.filter((t) => t !== entryPath),
    }

    await save(updated)
  }

  export async function getRoute(routePath: string): Promise<RouteCache | undefined> {
    const cache = await get()
    return cache?.routes.find((r) => r.path === routePath)
  }

  export async function getRoutesByPattern(pattern: string): Promise<RouteCache[]> {
    const cache = await get()
    if (!cache) return []

    const regex = new RegExp(pattern.replace(/\*/g, ".*"))
    return cache.routes.filter((r) => regex.test(r.path))
  }

  export async function getComponent(name: string): Promise<ComponentCache | undefined> {
    const cache = await get()
    return cache?.components.find((c) => c.name === name)
  }

  export async function getComponentsByType(type: ComponentCache["type"]): Promise<ComponentCache[]> {
    const cache = await get()
    if (!cache) return []
    return cache.components.filter((c) => c.type === type)
  }

  export async function getConfig(name: string): Promise<ConfigCache | undefined> {
    const cache = await get()
    return cache?.configs.find((c) => c.name === name)
  }

  export async function getTestForFile(filePath: string): Promise<string | undefined> {
    const cache = await get()
    if (!cache) return undefined

    const baseName = path.basename(filePath).replace(/\.(ts|tsx|js|jsx)$/, "")

    return cache.testFiles.find((testFile) => {
      const testBaseName = path.basename(testFile).replace(/\.(test|spec)\.(ts|tsx|js|jsx)$/, "$1")
      return testBaseName === baseName
    })
  }

  /**
   * Check if native cache implementation is being used
   */
  export function isUsingNative(): boolean {
    return nativeBindings !== null
  }
}
