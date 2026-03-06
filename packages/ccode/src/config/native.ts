/**
 * Native bindings for configuration loading (Fail-Fast Mode)
 *
 * This module provides high-performance config loading via Rust NAPI bindings.
 * Throws error if native bindings are unavailable - no fallback.
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "config.native" })

// ============================================================================
// Native Type Definitions
// ============================================================================

interface NapiConfig {
  theme?: string
  logLevel?: string
  model?: string
  smallModel?: string
  defaultAgent?: string
  username?: string
  disabledProviders: string[]
  enabledProviders: string[]
  instructions: string[]
}

interface NapiProviderConfig {
  apiKey?: string
  baseUrl?: string
  organization?: string
  whitelist: string[]
  blacklist: string[]
}

interface NapiAgentConfig {
  model?: string
  temperature?: number
  topP?: number
  prompt?: string
  description?: string
  disable?: boolean
  mode?: string
  hidden?: boolean
  color?: string
  steps?: number
}

interface NapiCommandConfig {
  template: string
  description?: string
  agent?: string
  model?: string
  subtask?: boolean
}

interface NapiSecretsConfig {
  llm: Record<string, string | null>
  channels: Record<string, string | null>
  external: Record<string, string | null>
}

interface NapiConfigLoaderHandle {
  configDir(): string
  homeDir(): string
  addPath(path: string): void
  loadFile(path: string): string
  parseJsonc(content: string): string
  loadMerged(): string
  getConfig(): NapiConfig
  getProviders(): Record<string, NapiProviderConfig>
  getAgents(): Record<string, NapiAgentConfig>
  getCommands(): Record<string, NapiCommandConfig>
  getSecrets(): NapiSecretsConfig | null
  scanDirectory(start: string, stop?: string): string[]
  findConfigFiles(dir: string): string[]
  save(configJson: string): void
  loadSecrets(): Record<string, string>
  mergeConfigs(baseJson: string, sourceJson: string): string
}

interface NativeBindings {
  createConfigLoader: (paths?: string[]) => NapiConfigLoaderHandle
}

// ============================================================================
// Native Bindings Loader (Fail-Fast)
// ============================================================================

let nativeBindings: NativeBindings | null = null
let nativeBindingsLoaded = false

/**
 * Load native config bindings. Throws if unavailable.
 * @throws Error if native bindings cannot be loaded
 */
export async function loadNativeBindings(): Promise<NativeBindings> {
  if (nativeBindingsLoaded && nativeBindings) return nativeBindings

  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>
    if (typeof bindings.createConfigLoader === "function") {
      nativeBindings = bindings as unknown as NativeBindings
      log.debug("Using native config implementation (Rust)")
      nativeBindingsLoaded = true
      return nativeBindings
    }
  } catch (e) {
    nativeBindingsLoaded = true
    throw new Error(`Native bindings required: @codecoder-ai/core config functions not available: ${e}`)
  }

  nativeBindingsLoaded = true
  throw new Error("Native bindings required: @codecoder-ai/core config functions not available")
}

// Handle cache
let configLoader: NapiConfigLoaderHandle | null = null

/**
 * Get the config loader. Creates one if needed.
 * @throws Error if native bindings unavailable
 */
export async function getConfigLoader(paths?: string[]): Promise<NapiConfigLoaderHandle> {
  const native = await loadNativeBindings()

  if (configLoader && !paths) return configLoader

  configLoader = native.createConfigLoader(paths)
  return configLoader
}

// ============================================================================
// Native Config API (Fail-Fast)
// ============================================================================

/**
 * Parse JSONC content using native Rust parser (4x faster than JS).
 * @throws Error if native bindings unavailable
 */
export async function parseJsoncNative(content: string): Promise<unknown> {
  const loader = await getConfigLoader()
  const json = loader.parseJsonc(content)
  return JSON.parse(json)
}

/**
 * Load configuration from a single file using native parser.
 * @throws Error if native bindings unavailable
 */
export async function loadFileNative(path: string): Promise<unknown> {
  const loader = await getConfigLoader()
  const json = loader.loadFile(path)
  return JSON.parse(json)
}

/**
 * Load merged configuration from all paths.
 * @throws Error if native bindings unavailable
 */
export async function loadMergedNative(paths?: string[]): Promise<unknown> {
  const loader = await getConfigLoader(paths)
  const json = loader.loadMerged()
  return JSON.parse(json)
}

/**
 * Scan directory for .codecoder config directories.
 * @throws Error if native bindings unavailable
 */
export async function scanDirectoryNative(start: string, stop?: string): Promise<string[]> {
  const loader = await getConfigLoader()
  return loader.scanDirectory(start, stop)
}

/**
 * Find config files in a directory.
 * @throws Error if native bindings unavailable
 */
export async function findConfigFilesNative(dir: string): Promise<string[]> {
  const loader = await getConfigLoader()
  return loader.findConfigFiles(dir)
}

/**
 * Merge two config objects.
 * @throws Error if native bindings unavailable
 */
export async function mergeConfigsNative(base: unknown, source: unknown): Promise<unknown> {
  const loader = await getConfigLoader()
  const baseJson = JSON.stringify(base)
  const sourceJson = JSON.stringify(source)
  const mergedJson = loader.mergeConfigs(baseJson, sourceJson)
  return JSON.parse(mergedJson)
}

/**
 * Check if native bindings are available
 */
export async function hasNativeBindings(): Promise<boolean> {
  try {
    await loadNativeBindings()
    return true
  } catch {
    return false
  }
}

/**
 * Get the config directory path from native loader.
 * @throws Error if native bindings unavailable
 */
export async function getConfigDirNative(): Promise<string> {
  const loader = await getConfigLoader()
  return loader.configDir()
}

/**
 * Get the home directory path from native loader.
 * @throws Error if native bindings unavailable
 */
export async function getHomeDirNative(): Promise<string> {
  const loader = await getConfigLoader()
  return loader.homeDir()
}

export type {
  NapiConfig,
  NapiProviderConfig,
  NapiAgentConfig,
  NapiCommandConfig,
  NapiSecretsConfig,
  NapiConfigLoaderHandle,
}
