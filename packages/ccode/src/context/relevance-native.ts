/**
 * Native Relevance Bindings (Fail-Fast Mode)
 *
 * Provides native Rust implementations for content relevance scoring.
 * Throws error if native bindings are unavailable - no fallback.
 *
 * @package context
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "context.relevance-native" })

// ============================================================================
// Type Definitions (must match NAPI types in context.rs)
// ============================================================================

/**
 * Relevance score result from native scorer
 */
export interface NapiRelevanceScore {
  score: number
  keywordScore: number
  structuralScore: number
  recencyScore: number
  matchedKeywords: string[]
}

/**
 * Configuration for relevance scoring
 */
export interface NapiRelevanceScorerConfig {
  keywordWeight: number
  structuralWeight: number
  recencyWeight: number
  minScore: number
  caseInsensitive: boolean
}

/**
 * File metadata for batch scoring
 */
export interface NapiFileMetadata {
  path: string
  content: string
  modified?: number
  extension?: string
}

/**
 * Scored file result
 */
export interface NapiScoredFile {
  path: string
  score: NapiRelevanceScore
}

// ============================================================================
// Native Bindings Interface
// ============================================================================

interface NativeRelevanceBindings {
  scoreRelevance: (query: string, content: string) => NapiRelevanceScore
  scoreRelevanceWithConfig: (
    query: string,
    content: string,
    config: NapiRelevanceScorerConfig,
  ) => NapiRelevanceScore
  scoreFiles: (query: string, files: NapiFileMetadata[]) => NapiScoredFile[]
  contentHash: (content: string) => string
}

// ============================================================================
// Native Bindings Loader (Fail-Fast)
// ============================================================================

let nativeBindings: NativeRelevanceBindings | null = null
let loadAttempted = false

/**
 * Load native bindings. Throws if unavailable.
 * @throws Error if native bindings cannot be loaded
 */
async function loadNativeBindings(): Promise<NativeRelevanceBindings> {
  if (loadAttempted && nativeBindings) return nativeBindings

  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>

    if (
      typeof bindings.scoreRelevance === "function" &&
      typeof bindings.scoreFiles === "function" &&
      typeof bindings.contentHash === "function"
    ) {
      nativeBindings = bindings as unknown as NativeRelevanceBindings
      log.debug("Loaded native relevance bindings")
      loadAttempted = true
      return nativeBindings
    }
  } catch (e) {
    loadAttempted = true
    throw new Error(`Native bindings required: @codecoder-ai/core relevance functions not available: ${e}`)
  }

  loadAttempted = true
  throw new Error("Native bindings required: @codecoder-ai/core relevance functions not available")
}

// ============================================================================
// Public API (Fail-Fast)
// ============================================================================

/**
 * Check if native relevance bindings are available
 */
export async function isNativeAvailable(): Promise<boolean> {
  try {
    await loadNativeBindings()
    return true
  } catch {
    return false
  }
}

/**
 * Score content relevance using native implementation
 * @throws Error if native bindings unavailable
 */
export async function scoreRelevanceNative(query: string, content: string): Promise<NapiRelevanceScore> {
  const bindings = await loadNativeBindings()
  return bindings.scoreRelevance(query, content)
}

/**
 * Score content relevance with custom configuration
 * @throws Error if native bindings unavailable
 */
export async function scoreRelevanceWithConfigNative(
  query: string,
  content: string,
  config: NapiRelevanceScorerConfig,
): Promise<NapiRelevanceScore> {
  const bindings = await loadNativeBindings()
  return bindings.scoreRelevanceWithConfig(query, content, config)
}

/**
 * Score multiple files and return sorted by relevance
 * @throws Error if native bindings unavailable
 */
export async function scoreFilesNative(query: string, files: NapiFileMetadata[]): Promise<NapiScoredFile[]> {
  const bindings = await loadNativeBindings()
  return bindings.scoreFiles(query, files)
}

/**
 * Compute content hash for deduplication
 * @throws Error if native bindings unavailable
 */
export async function contentHashNative(content: string): Promise<string> {
  const bindings = await loadNativeBindings()
  return bindings.contentHash(content)
}

/**
 * Default relevance scorer configuration
 */
export const DEFAULT_SCORER_CONFIG: NapiRelevanceScorerConfig = {
  keywordWeight: 0.5,
  structuralWeight: 0.3,
  recencyWeight: 0.2,
  minScore: 0.1,
  caseInsensitive: true,
}

/**
 * Check if native implementation is being used
 */
export function isUsingNative(): boolean {
  return nativeBindings !== null
}
