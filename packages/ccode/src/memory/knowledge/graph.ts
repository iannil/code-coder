/**
 * Graph Module - Unified Graph Engine
 *
 * This module provides a thin TypeScript adapter over the Rust NAPI graph implementations.
 * It replaces the old TypeScript implementations of SemanticGraph, CallGraph, and CausalGraph.
 *
 * All graph operations are now performed in Rust for better performance.
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/infrastructure/storage/storage"
import z from "zod"

// Import native graph handles
import {
  GraphEngineHandle as NativeGraphEngineHandle,
  SemanticGraphHandle as NativeSemanticGraphHandle,
  CallGraphHandle as NativeCallGraphHandle,
  CausalGraphHandle as NativeCausalGraphHandle,
} from "@codecoder-ai/core"

// Re-export types
export type {
  NapiNodeData,
  NapiPathResult,
  NapiCycleResult,
  NapiDecisionNode,
  NapiActionNode,
  NapiOutcomeNode,
  NapiCausalChain,
  NapiCausalQuery,
  NapiCausalStats,
  NapiCallNode,
  NapiRecursionInfo,
  NapiSemanticNode,
  NapiSemanticStats,
} from "@codecoder-ai/core"

// Verify native bindings are available
if (
  typeof NativeGraphEngineHandle !== "function" ||
  typeof NativeSemanticGraphHandle !== "function" ||
  typeof NativeCallGraphHandle !== "function" ||
  typeof NativeCausalGraphHandle !== "function"
) {
  throw new Error(
    "@codecoder-ai/core native bindings required: Graph handles not available. " +
    "Run: cd services/zero-core && cargo build --features napi-bindings"
  )
}

// Non-null references after verification
const GraphEngineHandle = NativeGraphEngineHandle!
const SemanticGraphHandle = NativeSemanticGraphHandle!
const CallGraphHandle = NativeCallGraphHandle!
const CausalGraphHandle = NativeCausalGraphHandle!

const log = Log.create({ service: "memory.knowledge.graph" })

// ============================================================================
// Graph Engine (Generic)
// ============================================================================

export namespace GraphEngine {
  const STORAGE_PREFIX = ["memory", "knowledge", "graph-engine"]

  export type Handle = InstanceType<typeof GraphEngineHandle>

  /**
   * Create a new graph engine
   */
  export function create(): Handle {
    return new GraphEngineHandle()
  }

  /**
   * Load a graph engine from storage
   */
  export async function load(graphId: string): Promise<Handle | undefined> {
    const projectId = Instance.project.id
    try {
      const data = await Storage.read<string>([...STORAGE_PREFIX, projectId, graphId])
      return GraphEngineHandle.fromJson(data)
    } catch {
      return undefined
    }
  }

  /**
   * Save a graph engine to storage
   */
  export async function save(graphId: string, engine: Handle): Promise<void> {
    const projectId = Instance.project.id
    const json = engine.toJson()
    await Storage.write([...STORAGE_PREFIX, projectId, graphId], json)
  }
}

// ============================================================================
// Semantic Graph
// ============================================================================

// Import NAPI types for compatibility
import type { NapiSemanticNode, NapiSemanticStats } from "@codecoder-ai/core"

export namespace SemanticGraph {
  const STORAGE_KEY = "semantic-graph"

  export type Handle = InstanceType<typeof SemanticGraphHandle>

  // Re-export types for compatibility
  export type Node = NapiSemanticNode
  export type Stats = NapiSemanticStats

  // Graph type for compatibility with code that expects SemanticGraph.Graph
  export interface Graph {
    nodes: NapiSemanticNode[]
    edges: Array<{ source: string; target: string; type: string }>
  }

  // Zod schemas for compatibility with existing code
  export const NodeSchema = z.object({
    id: z.string(),
    node_type: z.string(),
    name: z.string(),
    file: z.string(),
    metadata: z.string().optional(),
  })

  /**
   * Create a new semantic graph handle
   */
  export function create(): Handle {
    const projectId = Instance.project.id
    return new SemanticGraphHandle(projectId)
  }

  /**
   * Load semantic graph from storage
   */
  export async function load(): Promise<Handle | undefined> {
    const projectId = Instance.project.id
    try {
      const data = await Storage.read<string>(["memory", "knowledge", STORAGE_KEY, projectId])
      return SemanticGraphHandle.fromJson(data)
    } catch {
      return undefined
    }
  }

  /**
   * Get existing graph or create new one
   */
  export async function get(): Promise<Handle> {
    const existing = await load()
    return existing ?? create()
  }

  /**
   * Save semantic graph to storage
   */
  export async function save(graph: Handle): Promise<void> {
    const projectId = Instance.project.id
    const json = graph.toJson()
    await Storage.write(["memory", "knowledge", STORAGE_KEY, projectId], json)
  }

  /**
   * Build semantic graph from code index
   * This is a high-level function that populates the graph from CodeIndex
   */
  export async function build(): Promise<Handle> {
    const { CodeIndex } = await import("./code-index")
    const projectId = Instance.project.id
    const graph = new SemanticGraphHandle(projectId)

    log.info("building semantic graph", { projectId })

    const codeIndex = await CodeIndex.load()

    // Add functions
    for (const func of codeIndex.functions) {
      graph.addFunction(func.name, func.file, func.signature ?? undefined, func.exported ?? false)
    }

    // Add classes
    for (const cls of codeIndex.classes) {
      graph.addClass(cls.name, cls.file, cls.extends ?? undefined, cls.methods ?? [])
    }

    // Add interfaces
    for (const iface of codeIndex.interfaces) {
      // addInterface expects a single extends (or undefined), take first if available
      const extendsInterface = iface.extends?.[0]
      graph.addInterface(iface.name, iface.file, extendsInterface)
    }

    // Add files
    for (const imp of codeIndex.imports) {
      graph.addFile(imp.file)
    }

    await save(graph)
    log.info("semantic graph built", { projectId, stats: graph.stats() })

    return graph
  }

  /**
   * Invalidate (delete) the semantic graph
   */
  export async function invalidate(): Promise<void> {
    const projectId = Instance.project.id
    await Storage.remove(["memory", "knowledge", STORAGE_KEY, projectId])
  }

  /**
   * Convert a handle to Graph interface for API compatibility
   */
  function handleToGraph(handle: Handle): Graph {
    const nodes = handle.getNodes()
    // Note: edges are not directly exposed in the Handle, so we return an empty array
    // The Rust implementation manages edges internally through relationship methods
    return {
      nodes,
      edges: [],
    }
  }

  /**
   * Load semantic graph as Graph interface (for API compatibility)
   * This is the function that should be used by API handlers
   */
  export async function loadAsGraph(): Promise<Graph | undefined> {
    const handle = await load()
    return handle ? handleToGraph(handle) : undefined
  }

  /**
   * Get semantic graph as Graph interface (for API compatibility)
   * This is the function that should be used by API handlers
   */
  export async function getAsGraph(): Promise<Graph> {
    const handle = await get()
    return handleToGraph(handle)
  }
}

// ============================================================================
// Call Graph
// ============================================================================

export namespace CallGraph {
  const STORAGE_KEY = "call-graph"

  export type Handle = InstanceType<typeof CallGraphHandle>

  /**
   * Create a new call graph handle
   */
  export function create(): Handle {
    const projectId = Instance.project.id
    return new CallGraphHandle(projectId)
  }

  /**
   * Load call graph from storage
   */
  export async function load(): Promise<Handle | undefined> {
    const projectId = Instance.project.id
    try {
      const data = await Storage.read<string>(["memory", "knowledge", STORAGE_KEY, projectId])
      return CallGraphHandle.fromJson(data)
    } catch {
      return undefined
    }
  }

  /**
   * Get existing graph or create new one
   */
  export async function get(): Promise<Handle> {
    const existing = await load()
    return existing ?? create()
  }

  /**
   * Save call graph to storage
   */
  export async function save(graph: Handle): Promise<void> {
    const projectId = Instance.project.id
    const json = graph.toJson()
    await Storage.write(["memory", "knowledge", STORAGE_KEY, projectId], json)
  }

  /**
   * Invalidate (delete) the call graph
   */
  export async function invalidate(): Promise<void> {
    const projectId = Instance.project.id
    await Storage.remove(["memory", "knowledge", STORAGE_KEY, projectId])
  }

  /**
   * Get call graph statistics
   */
  export async function getStats(): Promise<{
    totalNodes: number
    totalEdges: number
    averageIncoming: number
    averageOutgoing: number
    maxIncoming: number
    maxOutgoing: number
  }> {
    const graph = await get()
    const nodes = graph.getNodes()
    const totalNodes = nodes.length

    let totalEdges = 0
    let maxIncoming = 0
    let maxOutgoing = 0

    for (const node of nodes) {
      const callers = graph.getCallers(node.id)
      const callees = graph.getCallees(node.id)
      totalEdges += callees.length
      maxIncoming = Math.max(maxIncoming, callers.length)
      maxOutgoing = Math.max(maxOutgoing, callees.length)
    }

    return {
      totalNodes,
      totalEdges,
      averageIncoming: totalNodes > 0 ? totalEdges / totalNodes : 0,
      averageOutgoing: totalNodes > 0 ? totalEdges / totalNodes : 0,
      maxIncoming,
      maxOutgoing,
    }
  }
}

// ============================================================================
// Causal Graph
// ============================================================================

// Import types for compatibility with existing code
import type {
  NapiDecisionNode,
  NapiActionNode,
  NapiOutcomeNode,
  NapiCausalChain,
  NapiCausalQuery,
  NapiCausalStats,
  NapiCausalPattern,
  NapiSimilarDecision,
  NapiTrendAnalysis,
  NapiAgentInsights,
} from "@codecoder-ai/core"

export namespace CausalGraph {
  const STORAGE_KEY = "causal-graph"

  export type Handle = InstanceType<typeof CausalGraphHandle>

  // Re-export NAPI types for compatibility
  export type DecisionNode = NapiDecisionNode
  export type ActionNode = NapiActionNode
  export type OutcomeNode = NapiOutcomeNode
  export type CausalChain = NapiCausalChain
  export type CausalQuery = NapiCausalQuery
  export type CausalStats = NapiCausalStats
  export type CausalPattern = NapiCausalPattern
  export type SimilarDecision = NapiSimilarDecision
  export type TrendAnalysis = NapiTrendAnalysis
  export type AgentInsights = NapiAgentInsights

  /**
   * Create a new causal graph handle
   */
  export function create(): Handle {
    const projectId = Instance.project.id
    return new CausalGraphHandle(projectId)
  }

  /**
   * Load causal graph from storage
   */
  export async function load(): Promise<Handle | undefined> {
    const projectId = Instance.project.id
    try {
      const data = await Storage.read<string>(["memory", "knowledge", STORAGE_KEY, projectId])
      return CausalGraphHandle.fromJson(data)
    } catch {
      return undefined
    }
  }

  /**
   * Get existing graph or create new one
   */
  export async function get(): Promise<Handle> {
    const existing = await load()
    return existing ?? create()
  }

  /**
   * Save causal graph to storage
   */
  export async function save(graph: Handle): Promise<void> {
    const projectId = Instance.project.id
    const json = graph.toJson()
    await Storage.write(["memory", "knowledge", STORAGE_KEY, projectId], json)
  }

  /**
   * Invalidate (delete) the causal graph
   */
  export async function invalidate(): Promise<void> {
    const projectId = Instance.project.id
    await Storage.remove(["memory", "knowledge", STORAGE_KEY, projectId])
  }

  // ============================================================================
  // Convenience Functions (for compatibility with existing usage)
  // ============================================================================

  /**
   * Record a decision - accepts object-based input for API compatibility
   */
  export async function recordDecision(input: {
    sessionId: string
    agentId: string
    prompt: string
    reasoning: string
    confidence: number
    context?: Record<string, unknown>
  }): Promise<NapiDecisionNode> {
    const graph = await get()
    const contextJson = input.context ? JSON.stringify(input.context) : undefined
    const decision = graph.recordDecision(
      input.sessionId,
      input.agentId,
      input.prompt,
      input.reasoning,
      input.confidence,
      contextJson
    )
    await save(graph)
    return decision
  }

  /**
   * Record an action - accepts object-based input for API compatibility
   */
  export async function recordAction(input: {
    decisionId: string
    actionType: string
    description: string
    input?: Record<string, unknown>
    output?: Record<string, unknown>
    duration?: number
  }): Promise<NapiActionNode> {
    const graph = await get()
    const inputJson = input.input ? JSON.stringify(input.input) : undefined
    const outputJson = input.output ? JSON.stringify(input.output) : undefined
    const action = graph.recordAction(
      input.decisionId,
      input.actionType,
      input.description,
      inputJson,
      outputJson,
      input.duration
    )
    await save(graph)
    return action
  }

  /**
   * Record an outcome - accepts object-based input for API compatibility
   */
  export async function recordOutcome(input: {
    actionId: string
    status: "success" | "partial" | "failure"
    description: string
    metrics?: Record<string, unknown>
    feedback?: string
  }): Promise<NapiOutcomeNode> {
    const graph = await get()
    const metricsJson = input.metrics ? JSON.stringify(input.metrics) : undefined
    const outcome = graph.recordOutcome(
      input.actionId,
      input.status,
      input.description,
      metricsJson,
      input.feedback
    )
    await save(graph)
    return outcome
  }

  /**
   * Get a decision by ID
   */
  export async function getDecision(id: string): Promise<NapiDecisionNode | null> {
    const graph = await get()
    return graph.getDecision(id)
  }

  /**
   * Get an action by ID
   */
  export async function getAction(id: string): Promise<NapiActionNode | null> {
    const graph = await get()
    // Get the causal chain containing this action
    const chains = graph.query({})
    for (const chain of chains) {
      const action = chain.actions.find((a) => a.id === id)
      if (action) return action
    }
    return null
  }

  /**
   * Get an outcome by ID
   */
  export async function getOutcome(id: string): Promise<NapiOutcomeNode | null> {
    const graph = await get()
    // Get the causal chain containing this outcome
    const chains = graph.query({})
    for (const chain of chains) {
      const outcome = chain.outcomes.find((o) => o.id === id)
      if (outcome) return outcome
    }
    return null
  }

  /**
   * Get complete causal chain for a decision
   */
  export async function getCausalChain(decisionId: string): Promise<NapiCausalChain | null> {
    const graph = await get()
    return graph.getCausalChain(decisionId)
  }

  /**
   * Get causal chains for a session
   */
  export async function getCausalChainsForSession(sessionId: string): Promise<NapiCausalChain[]> {
    const graph = await get()
    return graph.query({ sessionId })
  }

  /**
   * Query causal chains with filters
   */
  export async function query(options: {
    agentId?: string
    sessionId?: string
    actionType?: string
    status?: string
    minConfidence?: number
    dateFrom?: string
    dateTo?: string
    limit?: number
  }): Promise<NapiCausalChain[]> {
    const graph = await get()
    // NAPI uses camelCase for query properties
    return graph.query({
      agentId: options.agentId,
      sessionId: options.sessionId,
      actionType: options.actionType,
      status: options.status,
      minConfidence: options.minConfidence,
      dateFrom: options.dateFrom,
      dateTo: options.dateTo,
      limit: options.limit,
    })
  }

  /**
   * Get success rate
   */
  export async function getSuccessRate(agentId?: string): Promise<number> {
    const graph = await get()
    return graph.getSuccessRate(agentId)
  }

  /**
   * Get statistics
   */
  export async function getStats(): Promise<NapiCausalStats> {
    const graph = await get()
    return graph.getStats()
  }

  // ============================================================================
  // Native Analysis Methods (Rust implementation)
  // ============================================================================

  /**
   * Find recurring decision-outcome patterns (native Rust implementation)
   */
  export async function findPatterns(options?: {
    agentId?: string
    minOccurrences?: number
    limit?: number
  }): Promise<NapiCausalPattern[]> {
    const graph = await get()
    return graph.findPatterns(
      options?.agentId ?? undefined,
      options?.minOccurrences ?? 2,
      options?.limit ?? 20
    )
  }

  /**
   * Find decisions similar to the given prompt (native Rust implementation)
   */
  export async function findSimilarDecisions(
    prompt: string,
    agentId: string,
    limit?: number
  ): Promise<NapiSimilarDecision[]> {
    const graph = await get()
    return graph.findSimilarDecisions(prompt, agentId, limit ?? 10)
  }

  /**
   * Analyze decision trends over time (native Rust implementation)
   */
  export async function analyzeTrends(options?: {
    agentId?: string
    periodDays?: number
  }): Promise<NapiTrendAnalysis> {
    const graph = await get()
    return graph.analyzeTrends(options?.agentId ?? undefined, options?.periodDays ?? 7)
  }

  /**
   * Get aggregated insights for an agent (native Rust implementation)
   */
  export async function getAgentInsights(agentId: string): Promise<NapiAgentInsights> {
    const graph = await get()
    return graph.getAgentInsights(agentId)
  }

  /**
   * Generate Mermaid diagram representation
   */
  export async function toMermaid(options?: {
    maxNodes?: number
    decisionId?: string
  }): Promise<string> {
    const graph = await get()
    const chains = options?.decisionId
      ? [graph.getCausalChain(options.decisionId)].filter((c): c is NapiCausalChain => c !== null)
      : graph.query({ limit: options?.maxNodes ?? 50 })

    const lines: string[] = ["graph LR"]
    const nodeIds = new Set<string>()

    for (const chain of chains) {
      // Add decision node
      if (!nodeIds.has(chain.decision.id)) {
        nodeIds.add(chain.decision.id)
        const label = chain.decision.prompt.slice(0, 30).replace(/"/g, "'")
        lines.push(`  ${chain.decision.id}["${label}..."]`)
      }

      // Add action nodes and edges
      for (const action of chain.actions) {
        if (!nodeIds.has(action.id)) {
          nodeIds.add(action.id)
          const label = `${action.actionType}: ${action.description.slice(0, 20)}`.replace(/"/g, "'")
          lines.push(`  ${action.id}["${label}"]`)
        }
        lines.push(`  ${chain.decision.id} --> ${action.id}`)

        // Add outcome nodes and edges
        for (const outcome of chain.outcomes.filter((o) => o.actionId === action.id)) {
          if (!nodeIds.has(outcome.id)) {
            nodeIds.add(outcome.id)
            const emoji = outcome.status === "success" ? "✅" : outcome.status === "failure" ? "❌" : "⚠️"
            const label = `${emoji} ${outcome.description.slice(0, 20)}`.replace(/"/g, "'")
            lines.push(`  ${outcome.id}["${label}"]`)
          }
          lines.push(`  ${action.id} --> ${outcome.id}`)
        }
      }
    }

    return lines.join("\n")
  }
}
