/**
 * Context Cache - Project structure caching with NAPI bindings
 *
 * Provides high-performance caching for project routes, components, and configs
 * with automatic framework detection.
 */

// ============================================================================
// Type Definitions
// ============================================================================

export type CacheEntryType = 'route' | 'component' | 'config' | 'api' | 'middleware' | 'layout' | 'page'

export type RouteType =
  | 'page'
  | 'api'
  | 'layout'
  | 'error'
  | 'loading'
  | 'middleware'
  | 'not_found'
  | 'template'
  | 'default'

export type ComponentType = 'react' | 'vue' | 'svelte' | 'solid' | 'preact' | 'unknown'

export interface CacheTime {
  secs: number
  nanos: number
}

export interface CacheEntry {
  entryType: CacheEntryType
  path: string
  name: string
  metadata: Record<string, string>
  lastModified: CacheTime
}

export interface RouteCache {
  path: string
  routeType: RouteType
  filePath: string
  params: string[]
  methods: string[]
}

export interface ComponentCache {
  name: string
  componentType: ComponentType
  filePath: string
  props: string[]
  isDefault: boolean
}

export interface ConfigCache {
  name: string
  filePath: string
  configType: string
}

export interface ProjectCache {
  projectPath: string
  framework: string | null
  routes: RouteCache[]
  components: ComponentCache[]
  configs: ConfigCache[]
  entries: CacheEntry[]
  createdAt: CacheTime
  updatedAt: CacheTime
}

// ============================================================================
// Native Binding Check
// ============================================================================

let _nativeAvailable: boolean | null = null

async function checkNativeAvailable(): Promise<boolean> {
  if (_nativeAvailable !== null) return _nativeAvailable

  try {
    const bindings = (await import('./binding.js')) as any
    _nativeAvailable = typeof bindings.ContextCacheStoreHandle === 'function'
  } catch {
    _nativeAvailable = false
  }

  return _nativeAvailable
}

export const isContextCacheNative = checkNativeAvailable

// ============================================================================
// Context Cache Store
// ============================================================================

export class ContextCacheStore {
  private handle: any = null
  private fallbackCache = new Map<string, ProjectCache>()

  private constructor() {}

  /**
   * Open or create a context cache store
   */
  static async open(dbPath: string): Promise<ContextCacheStore> {
    const store = new ContextCacheStore()

    if (await checkNativeAvailable()) {
      const bindings = (await import('./binding.js')) as any
      store.handle = new bindings.ContextCacheStoreHandle(dbPath)
    }

    return store
  }

  /**
   * Create an in-memory context cache store
   */
  static async memory(): Promise<ContextCacheStore> {
    const store = new ContextCacheStore()

    if (await checkNativeAvailable()) {
      const bindings = (await import('./binding.js')) as any
      store.handle = bindings.createMemoryContextCacheStore()
    }

    return store
  }

  /**
   * Save a project cache
   */
  async save(cache: ProjectCache): Promise<void> {
    if (this.handle) {
      this.handle.save(cache)
    } else {
      this.fallbackCache.set(cache.projectPath, cache)
    }
  }

  /**
   * Load a project cache
   */
  async load(projectPath: string): Promise<ProjectCache | null> {
    if (this.handle) {
      return this.handle.load(projectPath)
    }
    return this.fallbackCache.get(projectPath) ?? null
  }

  /**
   * Check if a project cache exists and is fresh
   */
  async isFresh(projectPath: string, maxAgeSeconds: number): Promise<boolean> {
    if (this.handle) {
      return this.handle.isFresh(projectPath, maxAgeSeconds)
    }

    const cache = this.fallbackCache.get(projectPath)
    if (!cache) return false

    const now = Date.now() / 1000
    const cacheAge = now - cache.updatedAt.secs
    return cacheAge < maxAgeSeconds
  }

  /**
   * Get routes for a project
   */
  async getRoutes(projectPath: string): Promise<RouteCache[]> {
    if (this.handle) {
      return this.handle.getRoutes(projectPath)
    }
    return this.fallbackCache.get(projectPath)?.routes ?? []
  }

  /**
   * Get components for a project
   */
  async getComponents(projectPath: string): Promise<ComponentCache[]> {
    if (this.handle) {
      return this.handle.getComponents(projectPath)
    }
    return this.fallbackCache.get(projectPath)?.components ?? []
  }

  /**
   * Get configs for a project
   */
  async getConfigs(projectPath: string): Promise<ConfigCache[]> {
    if (this.handle) {
      return this.handle.getConfigs(projectPath)
    }
    return this.fallbackCache.get(projectPath)?.configs ?? []
  }

  /**
   * Delete a project cache
   */
  async delete(projectPath: string): Promise<boolean> {
    if (this.handle) {
      return this.handle.delete(projectPath)
    }
    return this.fallbackCache.delete(projectPath)
  }

  /**
   * List all cached project paths
   */
  async listProjects(): Promise<string[]> {
    if (this.handle) {
      return this.handle.listProjects()
    }
    return Array.from(this.fallbackCache.keys())
  }

  /**
   * Get cache statistics
   */
  async getStats(): Promise<{ projectCount: number; totalEntries: number }> {
    if (this.handle) {
      return this.handle.getStats()
    }

    let totalEntries = 0
    for (const cache of this.fallbackCache.values()) {
      totalEntries += cache.entries.length
    }

    return {
      projectCount: this.fallbackCache.size,
      totalEntries,
    }
  }
}

// ============================================================================
// Cache Builder
// ============================================================================

export interface BuildCacheOptions {
  includeNodeModules?: boolean
  maxDepth?: number
  patterns?: string[]
}

/**
 * Build a project cache by scanning the project directory
 */
export async function buildProjectCache(
  projectPath: string,
  options: BuildCacheOptions = {}
): Promise<ProjectCache> {
  if (await checkNativeAvailable()) {
    const bindings = (await import('./binding.js')) as any
    return bindings.buildProjectCache(projectPath, options)
  }

  // Fallback: return empty cache
  const now = Date.now() / 1000
  const cacheTime: CacheTime = { secs: Math.floor(now), nanos: 0 }

  return {
    projectPath,
    framework: null,
    routes: [],
    components: [],
    configs: [],
    entries: [],
    createdAt: cacheTime,
    updatedAt: cacheTime,
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Get or build a project cache
 *
 * Checks if a fresh cache exists, otherwise builds a new one
 */
export async function getOrBuildCache(
  store: ContextCacheStore,
  projectPath: string,
  maxAgeSeconds = 3600,
  buildOptions: BuildCacheOptions = {}
): Promise<ProjectCache> {
  if (await store.isFresh(projectPath, maxAgeSeconds)) {
    const existing = await store.load(projectPath)
    if (existing) return existing
  }

  const cache = await buildProjectCache(projectPath, buildOptions)
  await store.save(cache)
  return cache
}
