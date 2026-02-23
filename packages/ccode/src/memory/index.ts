export { Preferences } from "./preferences"
export { Style } from "./style"
export { Knowledge } from "./knowledge/index"
export { CodeIndex } from "./knowledge/code-index"
export { SemanticGraph } from "./knowledge/semantic-graph"
export { CallGraph } from "./knowledge/call-graph"
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

// Note: Types are available as Preferences.CodeStyle, Style.EditChoice, etc.

import { Preferences } from "./preferences"
import { Style } from "./style"
import { Knowledge } from "./knowledge/index"
import { CodeIndex } from "./knowledge/code-index"
import { SemanticGraph } from "./knowledge/semantic-graph"
import { CallGraph } from "./knowledge/call-graph"
import { Patterns } from "./knowledge/patterns"

import { Log } from "@/util/log"
import { EditHistory as _EditHistory } from "./history/edits"
import { Vector as _Vector } from "./vector"

const EditHistory = _EditHistory
const Vector = _Vector

const log = Log.create({ service: "memory" })

export async function initialize(options?: { watch?: boolean; sync?: boolean }): Promise<void> {
  log.info("initializing memory module")

  try {
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
  const { Sync } = await import("./storage/sync")
  return Sync.exportMemory()
}

export async function importMemory(
  data: any,
  options?: { merge?: boolean; overwrite?: boolean },
): Promise<{ imported: number; skipped: number; conflicts: number }> {
  const { Sync } = await import("./storage/sync")
  return Sync.importMemory(data, options)
}

export async function getMemoryStats(): Promise<{
  preferences: any
  knowledge: any
  history: any
  vector: any
  codeIndex: any
  callGraph: any
  toolRegistry: any
}> {
  const { History: MemHistory } = await import("./history/index")
  const { DynamicToolRegistry } = await import("./tools/index")
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
  }
}

export async function cleanup(options?: { maxAge?: number; maxEntries?: number }): Promise<{ cleaned: number }> {
  let cleaned = 0

  const maxAge = options?.maxAge || 30 * 24 * 60 * 60 * 1000 // 30 days
  const cutoff = Date.now() - maxAge
  const maxAgeDays = Math.floor(maxAge / (24 * 60 * 60 * 1000))

  cleaned += await Vector.cleanup(cutoff)
  cleaned += await EditHistory.cleanup(cutoff)

  // Cleanup unused tools
  const { DynamicToolRegistry } = await import("./tools/index")
  cleaned += await DynamicToolRegistry.cleanup(maxAgeDays)

  return { cleaned }
}
