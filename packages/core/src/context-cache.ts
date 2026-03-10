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
  private handle: any

  private constructor(handle: any) {
    this.handle = handle
  }

  /**
   * Open or create a context cache store
   */
  static async open(dbPath: string): Promise<ContextCacheStore> {
    if (!(await checkNativeAvailable())) {
      throw new Error('Native binding required: ContextCacheStore requires @codecoder-ai/core NAPI bindings')
    }
    const bindings = (await import('./binding.js')) as any
    const handle = new bindings.ContextCacheStoreHandle(dbPath)
    return new ContextCacheStore(handle)
  }

  /**
   * Create an in-memory context cache store
   */
  static async memory(): Promise<ContextCacheStore> {
    if (!(await checkNativeAvailable())) {
      throw new Error('Native binding required: ContextCacheStore requires @codecoder-ai/core NAPI bindings')
    }
    const bindings = (await import('./binding.js')) as any
    const handle = bindings.createMemoryContextCacheStore()
    return new ContextCacheStore(handle)
  }

  /**
   * Save a project cache
   */
  async save(cache: ProjectCache): Promise<void> {
    this.handle.save(cache)
  }

  /**
   * Load a project cache
   */
  async load(projectPath: string): Promise<ProjectCache | null> {
    return this.handle.load(projectPath)
  }

  /**
   * Check if a project cache exists and is fresh
   */
  async isFresh(projectPath: string, maxAgeSeconds: number): Promise<boolean> {
    return this.handle.isFresh(projectPath, maxAgeSeconds)
  }

  /**
   * Get routes for a project
   */
  async getRoutes(projectPath: string): Promise<RouteCache[]> {
    return this.handle.getRoutes(projectPath)
  }

  /**
   * Get components for a project
   */
  async getComponents(projectPath: string): Promise<ComponentCache[]> {
    return this.handle.getComponents(projectPath)
  }

  /**
   * Get configs for a project
   */
  async getConfigs(projectPath: string): Promise<ConfigCache[]> {
    return this.handle.getConfigs(projectPath)
  }

  /**
   * Delete a project cache
   */
  async delete(projectPath: string): Promise<boolean> {
    return this.handle.delete(projectPath)
  }

  /**
   * List all cached project paths
   */
  async listProjects(): Promise<string[]> {
    return this.handle.listProjects()
  }

  /**
   * Get cache statistics
   */
  async getStats(): Promise<{ projectCount: number; totalEntries: number }> {
    return this.handle.getStats()
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
  if (!(await checkNativeAvailable())) {
    throw new Error('Native binding required: buildProjectCache requires @codecoder-ai/core NAPI bindings')
  }
  const bindings = (await import('./binding.js')) as any
  return bindings.buildProjectCache(projectPath, options)
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
