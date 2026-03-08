export { Preferences } from "./preferences"
export { Style } from "./style"
export { Knowledge } from "./knowledge/index"
export { CodeIndex } from "./knowledge/code-index"
export { SemanticGraph, CallGraph, CausalGraph, GraphEngine } from "./knowledge/graph"
export { Patterns } from "./knowledge/patterns"
export { KnowledgeStorage } from "./knowledge/storage"
export * as History from "./history/index"
export { Decision } from "./history/decisions"
export { EditHistory } from "./history/edits"
export { Vector } from "./vector"
export { LocalStorage } from "./storage/local"
export { DatabaseStorage } from "./storage/database"
export { Sync } from "./storage/sync"

// Dynamic Tool Registry (Phase 12)
export { DynamicToolRegistry, ToolTypes, ToolRegistry, ToolSearch, ToolLearner } from "./tools/index"
export { searchTools, learnTool, getToolForExecution } from "./tools/index"

// Embedding Provider (Phase 2)
export {
  EmbeddingProvider,
  getEmbeddingProvider,
  createEmbeddingProvider,
  resetEmbeddingProvider,
} from "./embedding-provider"
export type { EmbeddingResult, EmbeddingProviderConfig } from "./embedding-provider"

// Markdown Chunker (Phase 2)
export { MarkdownChunker, getChunker, createChunker, chunkMarkdown } from "./chunker"
export type { Chunk, ChunkMetadata, ChunkerConfig } from "./chunker"

// Global Context Hub (Phase 2)
export { GlobalContextHub, getContextHub, createContextHub, retrieveContext } from "./context-hub"
export type { ContextItem, ContextSource, ContextResult, RetrievalOptions } from "./context-hub"

// Note: Types are available as Preferences.CodeStyle, Style.EditChoice, etc.

import { Preferences } from "./preferences"
import { Style } from "./style"
import { Knowledge } from "./knowledge/index"
import { CodeIndex } from "./knowledge/code-index"
import { SemanticGraph, CallGraph } from "./knowledge/graph"
import { Patterns } from "./knowledge/patterns"

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Global } from "@/util/global"
import path from "path"
import { EditHistory as _EditHistory } from "./history/edits"
import { Vector as _Vector } from "./vector"
import {
  createMemorySystem,
  isNative,
  type MemorySystemHandleType,
  type NapiMemoryStats,
} from "@codecoder-ai/core"

const EditHistory = _EditHistory
const Vector = _Vector

const log = Log.create({ service: "memory" })

// ============================================================================
// Native Memory System Handle (singleton)
// ============================================================================

let memorySystemHandle: MemorySystemHandleType | null = null

/**
 * Get or create the native memory system handle
 * Returns null if native bindings are not available
 */
function getMemorySystem(): MemorySystemHandleType | null {
  if (memorySystemHandle) return memorySystemHandle
  if (!isNative || !createMemorySystem) return null

  try {
    // Use Global.Path.data/memory for memory system storage
    const dataDir = path.join(Global.Path.data, "memory")
    memorySystemHandle = createMemorySystem(dataDir, Instance.project.id)
    return memorySystemHandle
  } catch (error) {
    log.warn("Failed to create native memory system", { error })
    return null
  }
}

// ============================================================================
// Memory Module API
// ============================================================================

export async function initialize(options?: { watch?: boolean; sync?: boolean }): Promise<void> {
  log.info("initializing memory module")

  try {
    // Initialize native memory system first (if available)
    getMemorySystem()

    await Preferences.get()
    await Style.get()
    await Knowledge.get()
    await Patterns.detectCommonPatterns()
    await CodeIndex.load()

    if (options?.sync) {
      const { Sync } = await import("./storage/sync")
      const config = await Sync.getConfig()
      if (config.enabled) {
        await Sync.syncNow()
      }
    }

    log.info("memory module initialized")
  } catch (error) {
    log.error("failed to initialize memory module", { error })
  }
}

export async function invalidate(): Promise<void> {
  log.info("invalidating memory cache")

  // Use native invalidation for history (single NAPI call)
  const nativeSystem = getMemorySystem()
  if (nativeSystem) {
    nativeSystem.invalidate()
  }

  // Invalidate TypeScript-managed modules
  await Preferences.invalidate()
  await Style.invalidate()
  await Knowledge.invalidate()
  await Patterns.invalidate()
  await CodeIndex.invalidate()
  await SemanticGraph.invalidate()
  await CallGraph.invalidate()
  await Vector.invalidate()
  const { History: MemHistory } = await import("./history/index")
  await MemHistory.invalidate()
  const { DynamicToolRegistry } = await import("./tools/index")
  await DynamicToolRegistry.invalidate()
}

export async function exportMemory(): Promise<{
  projectID: string
  timestamp: number
  data: any
}> {
  const nativeSystem = getMemorySystem()

  // Use native export for history data
  const nativeSnapshot = nativeSystem?.export()

  // Use Sync module for other data
  const { Sync } = await import("./storage/sync")
  const syncExport = await Sync.exportMemory()

  // Merge native and sync exports
  return {
    ...syncExport,
    data: {
      ...syncExport.data,
      _native: nativeSnapshot ? {
        version: nativeSnapshot.version,
        history: nativeSnapshot.history,
        metadata: nativeSnapshot.metadata,
      } : undefined,
    },
  }
}

export async function importMemory(
  data: any,
  options?: { merge?: boolean; overwrite?: boolean },
): Promise<{ imported: number; skipped: number; conflicts: number }> {
  const nativeSystem = getMemorySystem()

  // Import native data if present
  let nativeResult = { imported: 0, skipped: 0, conflicts: 0 }
  if (nativeSystem && data?._native) {
    const result = nativeSystem.importSnapshot(
      {
        version: data._native.version || 1,
        timestamp: Date.now(),
        projectId: Instance.project.id,
        history: data._native.history,
        metadata: data._native.metadata || "{}",
      },
      {
        merge: options?.merge || false,
        overwriteConflicts: options?.overwrite || false,
      }
    )
    nativeResult = { imported: result.imported, skipped: result.skipped, conflicts: result.conflicts }
  }

  // Import sync data
  const { Sync } = await import("./storage/sync")
  const syncResult = await Sync.importMemory(data, options)

  return {
    imported: syncResult.imported + nativeResult.imported,
    skipped: syncResult.skipped + nativeResult.skipped,
    conflicts: syncResult.conflicts + nativeResult.conflicts,
  }
}

/**
 * Get native memory stats (sync operation)
 * Returns null if native bindings are not available
 */
export function getNativeStats(): NapiMemoryStats | null {
  const nativeSystem = getMemorySystem()
  return nativeSystem?.stats() ?? null
}

export async function getMemoryStats(): Promise<{
  preferences: any
  knowledge: any
  history: any
  vector: any
  codeIndex: any
  callGraph: any
  toolRegistry: any
  native?: NapiMemoryStats
}> {
  const { History: MemHistory } = await import("./history/index")
  const { DynamicToolRegistry } = await import("./tools/index")

  // Get native stats (sync, fast)
  const nativeStats = getNativeStats()

  const [prefs, knowledge, history, vectorStats, codeIndex, callGraphStats, toolStats] = await Promise.all([
    Preferences.get(),
    Knowledge.get(),
    MemHistory.getSummary(),
    Vector.getStats(),
    CodeIndex.get(),
    CallGraph.getStats().catch(() => ({ totalNodes: 0, totalEdges: 0, averageIncoming: 0, averageOutgoing: 0, maxIncoming: 0, maxOutgoing: 0 })),
    DynamicToolRegistry.getStats().catch(() => ({ totalTools: 0, byLanguage: {}, byTag: {}, mostUsed: [], recentlyAdded: [], lastUpdated: 0 })),
  ])

  return {
    preferences: {
      codeStyle: prefs.codeStyle,
      learnedPatterns: prefs.learnedPatterns.length,
    },
    knowledge: {
      apiEndpoints: knowledge?.apiEndpoints.length || 0,
      dataModels: knowledge?.dataModels.length || 0,
      components: knowledge?.components.length || 0,
      notes: knowledge?.notes.length || 0,
    },
    history: {
      totalDecisions: history.totalDecisions,
      totalEdits: history.totalEdits,
      totalSessions: history.totalSessions,
    },
    vector: {
      totalEmbeddings: vectorStats.totalEmbeddings,
      dimension: vectorStats.dimension,
    },
    codeIndex: {
      functions: codeIndex?.functions.length || 0,
      classes: codeIndex?.classes.length || 0,
      interfaces: codeIndex?.interfaces.length || 0,
    },
    callGraph: callGraphStats,
    toolRegistry: {
      totalTools: toolStats.totalTools,
      byLanguage: toolStats.byLanguage,
      byTag: toolStats.byTag,
    },
    native: nativeStats ?? undefined,
  }
}

export async function cleanup(options?: { maxAge?: number; maxEntries?: number }): Promise<{ cleaned: number }> {
  let cleaned = 0

  const maxAge = options?.maxAge || 30 * 24 * 60 * 60 * 1000 // 30 days
  const cutoff = Date.now() - maxAge
  const maxAgeDays = Math.floor(maxAge / (24 * 60 * 60 * 1000))

  // Use native cleanup for history (single NAPI call)
  const nativeSystem = getMemorySystem()
  if (nativeSystem) {
    const result = nativeSystem.cleanup(maxAgeDays)
    cleaned += result.removed
  }

  cleaned += await Vector.cleanup(cutoff)
  cleaned += await EditHistory.cleanup(cutoff)

  // Cleanup unused tools
  const { DynamicToolRegistry } = await import("./tools/index")
  cleaned += await DynamicToolRegistry.cleanup(maxAgeDays)

  return { cleaned }
}
